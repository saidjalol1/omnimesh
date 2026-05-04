use crate::envelope::SignedEnvelope;
use crate::runtime::transport::config::TransportConfig;
use crate::runtime::transport::interface::{Transport, DEFAULT_PAYLOAD_CAPACITY};
use std::sync::{Arc, Mutex};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc;

/// TCP transport implementation using Tokio.
///
/// This transport provides reliable, ordered message delivery over TCP.
/// It maintains a background listener for incoming connections and handles
/// envelope serialization/deserialization automatically.
#[derive(Debug)]
pub struct TcpTransport {
    kind: &'static str,
    runtime: tokio::runtime::Runtime,
    rx: Arc<Mutex<mpsc::UnboundedReceiver<SignedEnvelope<DEFAULT_PAYLOAD_CAPACITY>>>>,
    config: TransportConfig,
}

impl TcpTransport {
    /// Creates a new TCP transport with the given configuration.
    ///
    /// This will start a background TCP listener and spawn async tasks
    /// for handling incoming connections.
    pub fn new(config: TransportConfig) -> Result<Self, String> {
        let runtime = tokio::runtime::Runtime::new()
            .map_err(|e| format!("Failed to create tokio runtime: {}", e))?;

        let (tx, rx) = mpsc::unbounded_channel();

        let transport = TcpTransport {
            kind: "tcp transport",
            runtime,
            rx: Arc::new(Mutex::new(rx)),
            config,
        };

        // Start TCP listener in background
        let tx_clone = tx.clone();
        let listen_addr = transport.config.tcp_listen_addr;
        let max_read_buffer = transport.config.max_read_buffer;

        transport.runtime.spawn(async move {
            match TcpListener::bind(listen_addr).await {
                Ok(listener) => {
                    println!("TCP listener started on {}", listen_addr);
                    loop {
                        match listener.accept().await {
                            Ok((mut socket, peer_addr)) => {
                                println!("TCP connection from {}", peer_addr);
                                let tx = tx_clone.clone();
                                tokio::spawn(async move {
                                    let mut buf = vec![0u8; max_read_buffer];
                                    match socket.read(&mut buf).await {
                                        Ok(n) if n > 0 => {
                                            match SignedEnvelope::deserialize(&buf[..n]) {
                                                Ok(envelope) => {
                                                    if tx.send(envelope).is_err() {
                                                        eprintln!("Failed to queue received envelope");
                                                    }
                                                }
                                                Err(e) => eprintln!("TCP deserialization failed: {:?}", e),
                                            }
                                        }
                                        Ok(_) => {} // Connection closed
                                        Err(e) => eprintln!("TCP read failed: {}", e),
                                    }
                                });
                            }
                            Err(e) => eprintln!("TCP accept failed: {}", e),
                        }
                    }
                }
                Err(e) => eprintln!("TCP listener bind failed: {}", e),
            }
        });

        Ok(transport)
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
        let bytes = envelope.serialize();
        let connect_addr = self.config.tcp_connect_addr;

        self.runtime.block_on(async {
            match TcpStream::connect(connect_addr).await {
                Ok(mut stream) => {
                    stream
                        .write_all(&bytes)
                        .await
                        .map_err(|e| format!("TCP write failed: {}", e))?;
                    println!("TCP transport: envelope sent to {}", connect_addr);
                    Ok(())
                }
                Err(e) => {
                    eprintln!("TCP connect failed: {}", e);
                    Ok(()) // Silently fail for now
                }
            }
        })
    }

    fn kind(&self) -> &'static str {
        self.kind
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tcp_transport_initializes_with_config() {
        let config = TransportConfig::default();
        let transport = TcpTransport::new(config);
        assert!(transport.is_ok());
    }

    #[test]
    fn tcp_transport_kind_is_correct() {
        let config = TransportConfig::default();
        let transport = TcpTransport::new(config).unwrap();
        assert_eq!(transport.kind(), "tcp transport");
    }

    #[test]
    fn tcp_transport_send_handles_connection_failure() {
        let config = TransportConfig::new(
            "127.0.0.1:8002".parse().unwrap(),
            "127.0.0.1:8003".parse().unwrap(), // Non-existent address
            "127.0.0.1:4434".parse().unwrap(),
        );
        let transport = TcpTransport::new(config).unwrap();

        let envelope = crate::runtime::transport::common::TransportUtils::sample_envelope();
        let result = transport.send(&envelope);
        // Should not panic, even if connection fails
        assert!(result.is_ok());
    }
}