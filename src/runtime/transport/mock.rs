use crate::envelope::SignedEnvelope;
use crate::runtime::transport::common::{TransportUtils, logging};
use crate::runtime::transport::interface::{Transport, DEFAULT_PAYLOAD_CAPACITY};
use crate::config::modes::layer_kinds;

/// Mock transport implementation for testing and development.
///
/// This transport provides a simulated network interface that generates
/// sample envelopes for testing purposes. It does not perform any actual
/// network communication.
use std::sync::Arc;
use crate::runtime::RoutingTable;

#[derive(Debug)]
pub struct MockTransport {
    kind: &'static str,
    #[allow(dead_code)]
    routing: Arc<RoutingTable>,
}

impl MockTransport {
    /// Creates a new mock transport instance.
    pub fn new(routing: Arc<RoutingTable>) -> Self {
        MockTransport {
            kind: layer_kinds::MOCK_TRANSPORT,
            routing,
        }
    }
}

impl Transport for MockTransport {
    fn receive(&self) -> Option<SignedEnvelope<DEFAULT_PAYLOAD_CAPACITY>> {
        // Always return a sample envelope for testing
        Some(TransportUtils::sample_envelope())
    }

    fn send(&self, _envelope: &SignedEnvelope<DEFAULT_PAYLOAD_CAPACITY>) -> Result<(), String> {
        logging::mock_envelope_sent();
        Ok(())
    }

    fn kind(&self) -> &'static str {
        self.kind
    }
}