use crate::envelope::SignedEnvelope;
use crate::runtime::transport::common::TransportUtils;
use crate::runtime::transport::interface::{Transport, DEFAULT_PAYLOAD_CAPACITY};

/// Mock transport implementation for testing and development.
///
/// This transport provides a simulated network interface that generates
/// sample envelopes for testing purposes. It does not perform any actual
/// network communication.
#[derive(Debug)]
pub struct MockTransport {
    kind: &'static str,
}

impl MockTransport {
    /// Creates a new mock transport instance.
    pub fn new() -> Self {
        MockTransport {
            kind: "mock transport",
        }
    }
}

impl Transport for MockTransport {
    fn receive(&self) -> Option<SignedEnvelope<DEFAULT_PAYLOAD_CAPACITY>> {
        // Always return a sample envelope for testing
        Some(TransportUtils::sample_envelope())
    }

    fn send(&self, _envelope: &SignedEnvelope<DEFAULT_PAYLOAD_CAPACITY>) -> Result<(), String> {
        println!("Mock transport: envelope sent (simulated)");
        Ok(())
    }

    fn kind(&self) -> &'static str {
        self.kind
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mock_transport_returns_sample_envelope() {
        let transport = MockTransport::new();
        let envelope = transport.receive().expect("should return envelope");

        assert_eq!(envelope.header.version, 1);
        assert_eq!(envelope.payload.as_slice(), b"hello omnimesh");
        assert_eq!(envelope.signature, [1u8; 64]);
    }

    #[test]
    fn mock_transport_send_succeeds() {
        let transport = MockTransport::new();
        let envelope = TransportUtils::sample_envelope();

        let result = transport.send(&envelope);
        assert!(result.is_ok());
    }

    #[test]
    fn mock_transport_kind_is_correct() {
        let transport = MockTransport::new();
        assert_eq!(transport.kind(), "mock transport");
    }
}