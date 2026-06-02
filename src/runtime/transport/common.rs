use crate::buffer::PayloadStorage;
use crate::envelope::{Did, EnvelopeHeader, MessageId, PayloadType, Priority, SignedEnvelope};
use crate::runtime::transport::interface::DEFAULT_PAYLOAD_CAPACITY;
use std::sync::Mutex;

/// Certificate constants
pub mod certificate {
    /// Default certificate common name (CN)
    pub const DEFAULT_CN: &str = "omnimesh.local";

    /// Default organization name
    pub const DEFAULT_ORG: &str = "OMNI-MESH";

    /// Default country code
    pub const DEFAULT_COUNTRY: &str = "US";

    /// Alternative names for the certificate
    pub const ALT_NAMES: &[&str] = &["omnimesh.local", "localhost", "127.0.0.1"];

    /// Default certificate validity in days
    pub const DEFAULT_VALIDITY_DAYS: u32 = 365;
}

/// Error messages
pub mod errors {
    /// Error creating tokio runtime
    pub fn runtime_creation_failed(err: &dyn std::fmt::Display) -> String {
        format!("Failed to create tokio runtime: {}", err)
    }

    /// Error parsing certificate PEM
    pub fn cert_parse_failed(err: &dyn std::fmt::Display) -> String {
        format!("Failed to parse certificate PEM: {}", err)
    }

    /// Error parsing private key PEM
    pub fn key_parse_failed(err: &dyn std::fmt::Display) -> String {
        format!("Failed to parse private key PEM: {}", err)
    }

    /// Error generating certificate
    pub fn cert_generation_failed(err: &dyn std::fmt::Display) -> String {
        format!("Failed to generate certificate: {}", err)
    }

    /// Error reading certificate file
    pub fn cert_file_read_failed(path: &str, err: &dyn std::fmt::Display) -> String {
        format!("Failed to read certificate file '{}': {}", path, err)
    }

    /// Error reading private key file
    pub fn key_file_read_failed(path: &str, err: &dyn std::fmt::Display) -> String {
        format!("Failed to read private key file '{}': {}", path, err)
    }

    /// Error acquiring mutex lock
    pub fn lock_acquire_failed(err: &dyn std::fmt::Display) -> String {
        format!("Failed to acquire lock: {}", err)
    }

    /// Error connecting to address
    pub fn connect_failed(addr: &str, err: &dyn std::fmt::Display) -> String {
        format!("Failed to connect to {}: {}", addr, err)
    }

    /// Error certificate validation
    pub const INVALID_CERTIFICATES: &str = "Generated certificates are invalid";

    /// Error during compression
    pub fn compression_write_failed(err: &dyn std::fmt::Display) -> String {
        format!("Compression write failed: {}", err)
    }

    /// Error finishing compression
    pub fn compression_finish_failed(err: &dyn std::fmt::Display) -> String {
        format!("Compression finish failed: {}", err)
    }

    /// Error during decompression
    pub fn decompression_failed(err: &dyn std::fmt::Display) -> String {
        format!("Decompression failed: {}", err)
    }
}

/// Common utilities shared across transport implementations.
pub struct TransportUtils;

impl TransportUtils {
    /// Creates a sample envelope for testing and demonstration purposes.
    ///
    /// This envelope contains a simple "hello omnimesh" payload and is used
    /// by the mock transport and for testing.
    pub fn sample_envelope() -> SignedEnvelope<DEFAULT_PAYLOAD_CAPACITY> {
        let mut payload = PayloadStorage::<DEFAULT_PAYLOAD_CAPACITY>::new();
        let _ = payload.push_bytes(b"hello omnimesh");

        let header = EnvelopeHeader::new(
            1,                          // version
            MessageId::new([0x01; 16]), // message id
            Did::new([0x02; 32]),       // sender
            Did::new([0x03; 32]),       // recipient
            1,                          // ttl
            1_700_000_000_000_000,      // timestamp
            Priority::Normal,
            PayloadType::Raw,
        );

        SignedEnvelope::new(header, payload, [1u8; 64]) // signature
    }

    /// Creates a tokio runtime with error handling
    pub fn create_runtime() -> Result<tokio::runtime::Runtime, String> {
        tokio::runtime::Runtime::new().map_err(|e| errors::runtime_creation_failed(&e))
    }

    /// Safely acquires a mutex lock with standard error handling
    pub fn acquire_lock<'a, T>(
        mutex: &'a Mutex<T>,
    ) -> Result<std::sync::MutexGuard<'a, T>, String> {
        mutex.lock().map_err(|e| errors::lock_acquire_failed(&e))
    }
}

/// Logging utilities for transport layer
pub mod logging {
    /// Log layer initialization
    pub fn layer_initialized(layer_name: &str, transport_kind: &str) {
        println!("Initializing {} layer: {}", layer_name, transport_kind);
    }

    /// Log TCP listener started
    pub fn tcp_listener_started(addr: impl std::fmt::Display) {
        println!("TCP listener started on {}", addr);
    }

    /// Log TCP connection received
    pub fn tcp_connection_received(addr: impl std::fmt::Display) {
        println!("TCP connection from {}", addr);
    }

    /// Log TCP envelope sent
    pub fn tcp_envelope_sent(addr: impl std::fmt::Display, active: usize, max: usize) {
        println!(
            "TCP transport: envelope sent to {} (pool: {}/{})",
            addr, active, max
        );
    }

    /// Log QUIC endpoint initialized
    pub fn quic_endpoint_initialized(addr: impl std::fmt::Display) {
        println!("QUIC endpoint initialized on {} with certificates", addr);
    }

    /// Log mock transport envelope sent
    pub fn mock_envelope_sent() {
        println!("Mock transport: envelope sent (simulated)");
    }

    /// Log error: failed to queue envelope
    pub fn error_queue_failed() {
        eprintln!("Failed to queue received envelope");
    }

    /// Log error: deserialization failed
    pub fn error_deserialization(err: impl std::fmt::Debug) {
        eprintln!("TCP deserialization failed: {:?}", err);
    }

    /// Log error: read failed
    pub fn error_read(err: impl std::fmt::Display) {
        eprintln!("TCP read failed: {}", err);
    }

    /// Log error: accept failed
    pub fn error_accept(err: impl std::fmt::Display) {
        eprintln!("TCP accept failed: {}", err);
    }

    /// Log error: listener bind failed
    pub fn error_listener_bind(err: impl std::fmt::Display) {
        eprintln!("TCP listener bind failed: {}", err);
    }

    /// Log error: write failed
    pub fn error_write(err: impl std::fmt::Display) {
        eprintln!("TCP write failed: {}", err);
    }

    /// Log error: connect failed
    pub fn error_connect(addr: impl std::fmt::Display, err: impl std::fmt::Display) {
        eprintln!("TCP connect failed to {}: {}", addr, err);
    }

    /// Log error: QUIC endpoint error
    pub fn error_quic_endpoint(err: impl std::fmt::Display) {
        eprintln!("QUIC endpoint error: {}", err);
    }
}
