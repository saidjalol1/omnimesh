use crate::envelope::SignedEnvelope;

pub const DEFAULT_PAYLOAD_CAPACITY: usize = 128;

/// Core transport trait for OMNI-MESH message transport.
///
/// This trait defines the interface that all transport implementations must provide.
/// Transports handle the low-level details of sending and receiving signed envelopes
/// over various network protocols (TCP, QUIC, etc.).
///
/// # Thread Safety
/// Implementations should be thread-safe and handle concurrent access appropriately.
/// The trait methods may be called from multiple threads.
///
/// # Error Handling
/// All methods return `Result` types to handle network failures gracefully.
/// Implementations should provide meaningful error messages for debugging.
///
/// # Performance
/// The `receive()` method should be non-blocking and return immediately if no
/// envelope is available. The `send()` method may block for network I/O.
pub trait Transport: std::fmt::Debug {
    /// Attempts to receive a signed envelope from the transport.
    ///
    /// This method should be non-blocking and return `None` if no envelope
    /// is currently available. Implementations should buffer incoming envelopes
    /// and return them as they become available.
    ///
    /// # Returns
    /// - `Some(envelope)` if an envelope was received
    /// - `None` if no envelope is currently available
    fn receive(&self) -> Option<SignedEnvelope<DEFAULT_PAYLOAD_CAPACITY>>;

    /// Sends a signed envelope through the transport.
    ///
    /// This method may block while performing network I/O operations.
    /// Implementations should handle connection establishment, serialization,
    /// and transmission of the envelope.
    ///
    /// # Arguments
    /// * `envelope` - The signed envelope to send
    ///
    /// # Returns
    /// * `Ok(())` if the envelope was sent successfully
    /// * `Err(String)` if sending failed with a descriptive error message
    fn send(&self, envelope: &SignedEnvelope<DEFAULT_PAYLOAD_CAPACITY>) -> Result<(), String>;

    /// Returns a human-readable description of this transport type.
    ///
    /// This is used for logging and debugging purposes.
    ///
    /// # Returns
    /// A static string describing the transport (e.g., "tcp transport", "quic transport")
    fn kind(&self) -> &'static str;
}