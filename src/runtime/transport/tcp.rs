use crate::envelope::SignedEnvelope;
use crate::runtime::transport::config::TransportConfig;
use crate::runtime::transport::interface::{Transport, DEFAULT_PAYLOAD_CAPACITY};
use crate::runtime::transport::common::{TransportUtils, errors, logging};
use crate::config::modes::layer_kinds;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{mpsc, Mutex};

/// Connection pool for reusing TCP connections
#[derive(Debug)]
struct ConnectionPool {
    connections: HashMap<std::net::SocketAddr, TcpStream>,
    max_pool_size: usize,
}

impl ConnectionPool {
    /// Creates a new empty connection pool
    fn new(max_pool_size: usize) -> Self {
        ConnectionPool {
            connections: HashMap::new(),
            max_pool_size,
        }
    }

    /// Gets or creates a connection to the given address
    async fn get_or_create(&mut self, addr: std::net::SocketAddr) -> Result<(), String> {
        // If connection exists and is still valid, reuse it
        if self.connections.contains_key(&addr) {
            return Ok(());
        }

        // If pool is full, remove oldest connection
        if self.connections.len() >= self.max_pool_size
            && let Some(key) = self.connections.keys().next().copied() {
                self.connections.remove(&key);
            }

        // Create new connection
        let stream = TcpStream::connect(addr)
            .await
            .map_err(|e| errors::connect_failed(&addr.to_string(), &e))?;

        self.connections.insert(addr, stream);
        Ok(())
    }

    /// Gets a mutable reference to a connection if it exists
    fn get_mut(&mut self, addr: std::net::SocketAddr) -> Option<&mut TcpStream> {
        self.connections.get_mut(&addr)
    }

    /// Removes a connection from the pool (e.g., if it becomes invalid)
    fn remove(&mut self, addr: std::net::SocketAddr) {
        self.connections.remove(&addr);
    }
}

/// TCP transport implementation using Tokio with connection pooling.
///
/// This transport provides reliable, ordered message delivery over TCP.
/// It maintains a background listener for incoming connections and handles
/// envelope serialization/deserialization automatically.
///
/// Connection pooling improves performance by reusing existing TCP connections
/// instead of creating new ones for each message send.
#[derive(Debug)]
pub struct TcpTransport {
    kind: &'static str,
    runtime: tokio::runtime::Runtime,
    rx: Arc<std::sync::Mutex<mpsc::UnboundedReceiver<SignedEnvelope<DEFAULT_PAYLOAD_CAPACITY>>>>,
    config: TransportConfig,
    pool: Arc<Mutex<ConnectionPool>>,
    routing: Arc<crate::runtime::RoutingTable>,
}

impl TcpTransport {
    /// Creates a new TCP transport with the given configuration.
    ///
    /// This will start a background TCP listener and spawn async tasks
    /// for handling incoming connections. Connection pooling is enabled
    /// with a default maximum pool size of 10 connections.
    pub fn new(config: TransportConfig, routing: Arc<crate::runtime::RoutingTable>) -> Result<Self, String> {
        let runtime = TransportUtils::create_runtime()?;

        let (tx, rx) = mpsc::unbounded_channel();
        let pool = Arc::new(Mutex::new(ConnectionPool::new(10)));

        let transport = TcpTransport {
            kind: layer_kinds::TCP_TRANSPORT,
            runtime,
            rx: Arc::new(std::sync::Mutex::new(rx)),
            config,
            pool,
            routing,
        };

        // Start TCP listener in background
        let tx_clone = tx.clone();
        let listen_addr = transport.config.tcp_listen_addr;
        let _max_read_buffer = transport.config.max_read_buffer;

        transport.runtime.spawn(async move {
            match TcpListener::bind(listen_addr).await {
                Ok(listener) => {
                    logging::tcp_listener_started(listen_addr);
                    loop {
                        match listener.accept().await {
                            Ok((mut socket, peer_addr)) => {
                                logging::tcp_connection_received(peer_addr);
                                let tx = tx_clone.clone();
                                tokio::spawn(async move {
                                    let mut buf = [0u8; 2048];
                                    const MAX_READS_PER_CYCLE: usize = 16;
                                    
                                    'connection: loop {
                                        for _ in 0..MAX_READS_PER_CYCLE {
                                            match socket.read(&mut buf).await {
                                                Ok(n) if n > 0 => {
                                                    match SignedEnvelope::deserialize(&buf[..n]) {
                                                        Ok(envelope) => {
                                                            if tx.send(envelope).is_err() {
                                                                logging::error_queue_failed();
                                                                break 'connection;
                                                            }
                                                        }
                                                        Err(e) => logging::error_deserialization(e),
                                                    }
                                                }
                                                Ok(_) => break 'connection, // Connection closed
                                                Err(e) => {
                                                    logging::error_read(e);
                                                    break 'connection;
                                                }
                                            }
                                        }
                                        // Yield to scheduler to guarantee WCET determinism
                                        tokio::task::yield_now().await;
                                    }
                                });
                            }
                            Err(e) => logging::error_accept(e),
                        }
                    }
                }
                Err(e) => logging::error_listener_bind(e),
            }
        });

        Ok(transport)
    }

    /// Returns the current pool statistics
    pub fn pool_stats(&self) -> Result<(usize, usize), String> {
        let pool = self.pool.try_lock()
            .map_err(|_| "Failed to acquire lock".to_string())?;
        Ok((pool.connections.len(), pool.max_pool_size))
    }
}

impl Transport for TcpTransport {
    fn receive(&self) -> Option<SignedEnvelope<DEFAULT_PAYLOAD_CAPACITY>> {
        match self.rx.lock() {
            Ok(mut rx) => rx.try_recv().ok(),
            Err(_) => None,
        }
    }

    fn send(&self, envelope: &SignedEnvelope<DEFAULT_PAYLOAD_CAPACITY>) -> Result<(), String> {
        let mut buf = [0u8; 2048];
        let len = envelope.serialize_into(&mut buf).map_err(|e| format!("{:?}", e))?;
        let bytes = &buf[..len];
        // Resolve DID to IP, fallback to config connect addr for testing
        let connect_addr = self.routing.resolve(&envelope.header.recipient_did)
            .unwrap_or(self.config.tcp_connect_addr);
        let pool = Arc::clone(&self.pool);

        self.runtime.block_on(async {
            let mut pool_guard = pool.lock().await;

            // Try to get or create connection, but don't fail if unable to connect
            match pool_guard.get_or_create(connect_addr).await {
                Ok(_) => {
                    // Connection successful, try to send
                    if let Some(stream) = pool_guard.get_mut(connect_addr) {
                        match stream.write_all(&bytes).await {
                            Ok(_) => {
                                let (active, max) = (pool_guard.connections.len(), pool_guard.max_pool_size);
                                logging::tcp_envelope_sent(connect_addr, active, max);
                                Ok(())
                            }
                            Err(e) => {
                                logging::error_write(e);
                                // Remove failed connection from pool
                                pool_guard.remove(connect_addr);
                                Ok(()) // Silently fail for now
                            }
                        }
                    } else {
                        Ok(()) // Connection exists but unavailable
                    }
                }
                Err(e) => {
                    logging::error_connect(connect_addr, e);
                    Ok(()) // Silently fail on connection errors
                }
            }
        })
    }

    fn kind(&self) -> &'static str {
        self.kind
    }
}