use crate::config::modes::layer_kinds;
use crate::envelope::{Did, SignedEnvelope};
use crate::runtime::RoutingTable;
use crate::runtime::transport::interface::{DEFAULT_PAYLOAD_CAPACITY, Transport};

use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};

/// A shared in-process message bus keyed by recipient DID.
///
/// Each `send()` pushes the envelope into the recipient's per-DID inbox.
/// Each `receive()` pops from its own DID's inbox only.
/// This allows SDK tests to reliably route messages between multiple clients
/// in the same process without any actual network I/O.
#[derive(Debug, Default)]
struct MockBus {
    inboxes: Mutex<HashMap<[u8; 32], VecDeque<SignedEnvelope<DEFAULT_PAYLOAD_CAPACITY>>>>,
}

impl MockBus {
    fn push(&self, env: SignedEnvelope<DEFAULT_PAYLOAD_CAPACITY>) {
        if let Ok(mut map) = self.inboxes.lock() {
            map.entry(env.header.recipient_did.0)
                .or_default()
                .push_back(env);
        }
    }

    fn pop_for(&self, did: &Did) -> Option<SignedEnvelope<DEFAULT_PAYLOAD_CAPACITY>> {
        self.inboxes.lock().ok()?.get_mut(&did.0)?.pop_front()
    }
}

/// Process-wide singleton bus, shared by all MockTransport instances.
fn global_bus() -> &'static MockBus {
    use std::sync::OnceLock;
    static BUS: OnceLock<MockBus> = OnceLock::new();
    BUS.get_or_init(MockBus::default)
}

/// Mock transport for testing and development.
///
/// Uses a per-DID in-memory inbox so messages reach the correct client
/// even when multiple clients share the same process and global bus.
#[derive(Debug, Clone)]
pub struct MockTransport {
    kind: &'static str,
    /// The DID this transport instance is pulling messages for
    local_did: Did,
    #[allow(dead_code)]
    routing: Arc<RoutingTable>,
}

impl MockTransport {
    /// Creates a new mock transport instance associated with `local_did`.
    pub fn new(routing: Arc<RoutingTable>) -> Self {
        // Temporarily use a zero DID; the SDK client overwrites it via `with_did`.
        MockTransport {
            kind: layer_kinds::MOCK_TRANSPORT,
            local_did: Did([0u8; 32]),
            routing,
        }
    }

    /// Bind this transport to a specific local DID so `receive()` only
    /// returns messages addressed to that node.
    pub fn with_did(mut self, did: Did) -> Self {
        self.local_did = did;
        self
    }
}

impl Transport for MockTransport {
    fn receive(&self) -> Option<SignedEnvelope<DEFAULT_PAYLOAD_CAPACITY>> {
        global_bus().pop_for(&self.local_did)
    }

    fn send(&self, envelope: &SignedEnvelope<DEFAULT_PAYLOAD_CAPACITY>) -> Result<(), String> {
        global_bus().push(*envelope);
        Ok(())
    }

    fn kind(&self) -> &'static str {
        self.kind
    }
}
