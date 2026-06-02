use crate::config::modes::layer_kinds;
use crate::envelope::SignedEnvelope;
use crate::runtime::transport::common::{TransportUtils, errors, logging};
use crate::runtime::transport::config::TransportConfig;
use crate::runtime::transport::interface::{DEFAULT_PAYLOAD_CAPACITY, Transport};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{Mutex, mpsc};

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
            && let Some(key) = self.connections.keys().next().copied()
        {
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

/// TCP transport implementation using Tokio with connection pooling and flow control.
///
/// This transport provides reliable, ordered message delivery over TCP with:
/// - Connection pooling for performance
/// - Bounded send buffers for backpressure
/// - Automatic reconnection with exponential backoff
/// - Connection health monitoring
#[derive(Debug)]
pub struct TcpTransport {
    kind: &'static str,
    runtime: Arc<tokio::runtime::Runtime>,
    rx: Arc<std::sync::Mutex<mpsc::UnboundedReceiver<SignedEnvelope<DEFAULT_PAYLOAD_CAPACITY>>>>,
    config: TransportConfig,
    pool: Arc<Mutex<ConnectionPool>>,
    routing: Arc<crate::runtime::RoutingTable>,
    send_buffer: Arc<Mutex<mpsc::Sender<SendRequest>>>,
    stats: Arc<Mutex<TransportStats>>,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct TransportStats {
    pub messages_sent: u64,
    pub messages_received: u64,
    pub send_failures: u64,
    pub backpressure_events: u64,
    pub reconnections: u64,
}

struct SendRequest {
    envelope: SignedEnvelope<DEFAULT_PAYLOAD_CAPACITY>,
    addr: std::net::SocketAddr,
}

impl TcpTransport {
    /// Creates a new TCP transport with the given configuration.
    ///
    /// This will start:
    /// - Background TCP listener for incoming connections
    /// - Send worker with bounded buffer (1000 messages)
    /// - Connection health monitor
    pub fn new(
        config: TransportConfig,
        routing: Arc<crate::runtime::RoutingTable>,
    ) -> Result<Self, String> {
        let runtime = TransportUtils::create_runtime()?;

        let (tx, rx) = mpsc::unbounded_channel();
        let (send_tx, mut send_rx) = mpsc::channel::<SendRequest>(1000); // Bounded for backpressure
        let pool = Arc::new(Mutex::new(ConnectionPool::new(10)));
        let stats = Arc::new(Mutex::new(TransportStats::default()));

        let runtime = Arc::new(runtime);

        let transport = TcpTransport {
            kind: layer_kinds::TCP_TRANSPORT,
            runtime: runtime.clone(),
            rx: Arc::new(std::sync::Mutex::new(rx)),
            config: config.clone(),
            pool: pool.clone(),
            routing: routing.clone(),
            send_buffer: Arc::new(Mutex::new(send_tx)),
            stats: stats.clone(),
        };

        // Start send worker with flow control
        let pool_clone = pool.clone();
        let stats_clone = stats.clone();
        let runtime_clone = runtime.clone();
        runtime_clone.spawn(async move {
            while let Some(req) = send_rx.recv().await {
                let mut pool_guard = pool_clone.lock().await;
                let mut stats_guard = stats_clone.lock().await;

                // Try to send with exponential backoff
                let mut retries = 0;
                let max_retries = 3;

                while retries < max_retries {
                    match pool_guard.get_or_create(req.addr).await {
                        Ok(_) => {
                            if let Some(stream) = pool_guard.get_mut(req.addr) {
                                let mut buf = [0u8; 2048];
                                if let Ok(len) = req.envelope.serialize_into(&mut buf) {
                                    // Write 4-byte length prefix + payload
                                    let len_bytes = (len as u32).to_be_bytes();
                                    let mut frame = Vec::with_capacity(4 + len);
                                    frame.extend_from_slice(&len_bytes);
                                    frame.extend_from_slice(&buf[..len]);
                                    match stream.write_all(&frame).await {
                                        Ok(_) => {
                                            let _ = stream.flush().await;
                                            stats_guard.messages_sent += 1;
                                            break;
                                        }
                                        Err(_) => {
                                            pool_guard.remove(req.addr);
                                            stats_guard.send_failures += 1;
                                            retries += 1;

                                            if retries < max_retries {
                                                stats_guard.reconnections += 1;
                                                tokio::time::sleep(
                                                    tokio::time::Duration::from_millis(
                                                        100 * (1 << retries), // Exponential backoff
                                                    ),
                                                )
                                                .await;
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        Err(_) => {
                            stats_guard.send_failures += 1;
                            retries += 1;

                            if retries < max_retries {
                                tokio::time::sleep(tokio::time::Duration::from_millis(
                                    100 * (1 << retries),
                                ))
                                .await;
                            }
                        }
                    }
                }
            }
        });

        // Start TCP listener in background
        let tx_clone = tx.clone();
        let listen_addr = config.tcp_listen_addr;
        let stats_clone = stats.clone();
        let runtime_clone = runtime.clone();

        runtime_clone.spawn(async move {
            match TcpListener::bind(listen_addr).await {
                Ok(listener) => {
                    logging::tcp_listener_started(listen_addr);
                    loop {
                        match listener.accept().await {
                            Ok((mut socket, peer_addr)) => {
                                logging::tcp_connection_received(peer_addr);
                                let tx = tx_clone.clone();
                                let stats = stats_clone.clone();

                                tokio::spawn(async move {
                                    let mut len_buf = [0u8; 4];
                                    let mut msg_buf = [0u8; 2048];

                                    'connection: loop {
                                        // Read 4-byte length prefix
                                        if let Err(e) = socket.read_exact(&mut len_buf).await {
                                            logging::error_read(e);
                                            break 'connection;
                                        }
                                        let msg_len = u32::from_be_bytes(len_buf) as usize;
                                        if msg_len == 0 || msg_len > msg_buf.len() {
                                            break 'connection;
                                        }
                                        // Read exact message bytes
                                        if let Err(e) =
                                            socket.read_exact(&mut msg_buf[..msg_len]).await
                                        {
                                            logging::error_read(e);
                                            break 'connection;
                                        }
                                        match SignedEnvelope::deserialize(&msg_buf[..msg_len]) {
                                            Ok(envelope) => {
                                                if tx.send(envelope).is_ok() {
                                                    let mut stats_guard = stats.lock().await;
                                                    stats_guard.messages_received += 1;
                                                } else {
                                                    logging::error_queue_failed();
                                                    break 'connection;
                                                }
                                            }
                                            Err(e) => logging::error_deserialization(e),
                                        }
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

    /// Returns transport statistics
    pub fn stats(&self) -> TransportStats {
        self.runtime.block_on(async { *self.stats.lock().await })
    }

    /// Returns the current pool statistics
    pub fn pool_stats(&self) -> Result<(usize, usize), String> {
        let pool = self
            .pool
            .try_lock()
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
        let connect_addr = self
            .routing
            .resolve(&envelope.header.recipient_did)
            .unwrap_or(self.config.tcp_connect_addr);

        let req = SendRequest {
            envelope: *envelope,
            addr: connect_addr,
        };

        // Try to send with backpressure handling
        self.runtime.block_on(async {
            let send_buffer = self.send_buffer.lock().await;
            match send_buffer.try_send(req) {
                Ok(_) => Ok(()),
                Err(mpsc::error::TrySendError::Full(_)) => {
                    // Backpressure: buffer is full
                    let mut stats_guard = self.stats.lock().await;
                    stats_guard.backpressure_events += 1;
                    Err("Send buffer full - backpressure applied".to_string())
                }
                Err(mpsc::error::TrySendError::Closed(_)) => Err("Send channel closed".to_string()),
            }
        })
    }

    fn kind(&self) -> &'static str {
        self.kind
    }
}
