use std::net::SocketAddr;

/// Network configuration for transport implementations.
///
/// This struct centralizes network-related configuration that can be
/// customized based on deployment environment and requirements.
#[derive(Debug, Clone)]
pub struct TransportConfig {
    /// Address for TCP transport to listen on
    pub tcp_listen_addr: SocketAddr,
    /// Address for TCP transport to connect to when sending
    pub tcp_connect_addr: SocketAddr,
    /// Address for QUIC transport to listen on
    pub quic_listen_addr: SocketAddr,
    /// Maximum buffer size for reading incoming data
    pub max_read_buffer: usize,
    /// Connection timeout in seconds
    pub connection_timeout_secs: u64,
}

impl Default for TransportConfig {
    fn default() -> Self {
        Self {
            tcp_listen_addr: "127.0.0.1:8001".parse().unwrap(),
            tcp_connect_addr: "127.0.0.1:8001".parse().unwrap(),
            quic_listen_addr: "127.0.0.1:4433".parse().unwrap(),
            max_read_buffer: 512,
            connection_timeout_secs: 30,
        }
    }
}

impl TransportConfig {
    /// Creates a new transport configuration with custom addresses.
    pub fn new(
        tcp_listen_addr: SocketAddr,
        tcp_connect_addr: SocketAddr,
        quic_listen_addr: SocketAddr,
    ) -> Self {
        Self {
            tcp_listen_addr,
            tcp_connect_addr,
            quic_listen_addr,
            ..Default::default()
        }
    }
}