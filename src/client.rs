//! OMNI-MESH Developer SDK
//!
//! `OmnimeshClient` is the high-level API that application developers use to
//! send and receive strongly-typed messages over the mesh network. It abstracts
//! away all cryptography, routing, and serialization details.
//!
//! # Quick Start
//!
//! ```rust,ignore
//! use omnimesh::client::{OmnimeshClient, ClientConfig};
//! use omnimesh::payload;
//!
//! // Build a client node
//! let client = OmnimeshClient::builder()
//!     .with_config(ClientConfig::default())
//!     .build()
//!     .expect("Failed to build client");
//!
//! // Send a MotionCommand to a remote robot
//! let cmd = payload::motion_command(1.0, 0.0, 0.0, 0.0, 0.0, 0.5, 100_000);
//! client.send(robot_did, cmd).unwrap();
//!
//! // Receive decoded messages
//! if let Some(msg) = client.try_receive() {
//!     println!("Got: {:?}", msg.payload);
//! }
//! ```

use std::sync::{Arc, Mutex, Condvar};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::collections::VecDeque;
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use ed25519_dalek::SigningKey;
use rand_core::OsRng;

use crate::config::OmnimeshMode;
use crate::envelope::{Did, EnvelopeHeader, MessageId, Priority, PayloadType, SignedEnvelope};
use crate::buffer::PayloadStorage;
use crate::payload::{EnvelopePayload, encode_payload, decode_payload};
use crate::runtime::transport::interface::DEFAULT_PAYLOAD_CAPACITY;
use crate::runtime::transport::TransportLayer;
use crate::runtime::security::SecurityLayer;
use crate::runtime::RoutingTable;

/// A message received by the SDK — fully verified and decoded.
#[derive(Debug, Clone)]
pub struct ReceivedMessage {
    /// The DID of the sender node
    pub sender: Did,
    /// The decoded payload (e.g. MotionCommand, Heartbeat, LlmResponse)
    pub payload: EnvelopePayload,
    /// Timestamp of when the message was received (microseconds since epoch)
    pub received_at_us: u64,
}

/// Configuration for the SDK client.
#[derive(Debug, Clone)]
pub struct ClientConfig {
    /// Mode the runtime will operate in
    pub mode: OmnimeshMode,
    /// How many received messages to buffer before dropping (back-pressure)
    pub receive_buffer_capacity: usize,
    /// Custom TCP listen port (None = use default 9000)
    pub listen_port: Option<u16>,
}

impl Default for ClientConfig {
    fn default() -> Self {
        Self {
            mode: OmnimeshMode::development(),
            receive_buffer_capacity: 1024,
            listen_port: None,
        }
    }
}

impl ClientConfig {
    pub fn development() -> Self {
        Self { mode: OmnimeshMode::development(), ..Default::default() }
    }

    pub fn lightweight() -> Self {
        Self { mode: OmnimeshMode::lightweight(), ..Default::default() }
    }

    pub fn lightweight_on_port(port: u16) -> Self {
        Self { mode: OmnimeshMode::lightweight(), listen_port: Some(port), ..Default::default() }
    }

    pub fn production() -> Self {
        Self { mode: OmnimeshMode::production(), ..Default::default() }
    }

    pub fn production_on_port(port: u16) -> Self {
        Self { mode: OmnimeshMode::production(), listen_port: Some(port), ..Default::default() }
    }
}

/// Builder for `OmnimeshClient`
#[derive(Default)]
pub struct ClientBuilder {
    config: Option<ClientConfig>,
    signing_key: Option<SigningKey>,
}

impl ClientBuilder {
    pub fn with_config(mut self, config: ClientConfig) -> Self {
        self.config = Some(config);
        self
    }

    pub fn with_signing_key(mut self, key: SigningKey) -> Self {
        self.signing_key = Some(key);
        self
    }

    pub fn build(self) -> Result<OmnimeshClient, String> {
        let config = self.config.unwrap_or_default();
        let signing_key = self.signing_key.unwrap_or_else(|| SigningKey::generate(&mut OsRng));
        OmnimeshClient::new(config, signing_key)
    }
}

/// The OMNI-MESH Developer SDK client.
///
/// Manages its own background polling thread, routing table, and
/// sequence counter. Applications interact purely with high-level types.
///
/// Call `shutdown()` to gracefully stop the background poller thread.
/// The client also shuts down automatically when all clones are dropped.
#[derive(Clone)]
pub struct OmnimeshClient {
    /// This node's identity
    pub did: Did,
    signing_key: Arc<SigningKey>,
    transport: Arc<TransportLayer>,
    security: Arc<SecurityLayer>,
    routing: Arc<RoutingTable>,
    /// Inbound message queue (filled by background poller)
    inbox: Arc<Mutex<VecDeque<ReceivedMessage>>>,
    /// Condvar to wake up receivers waiting for messages
    inbox_notify: Arc<Condvar>,
    receive_buffer_capacity: usize,
    /// Monotonically increasing sequence counter
    seq: Arc<Mutex<u64>>,
    /// Graceful shutdown flag
    shutdown: Arc<AtomicBool>,
    /// Messages dropped due to back-pressure (observable metric)
    pub(crate) messages_dropped: Arc<AtomicU64>,
    /// Messages successfully received
    pub(crate) messages_received: Arc<AtomicU64>,
    /// Messages sent
    pub(crate) messages_sent: Arc<AtomicU64>,
}

impl OmnimeshClient {
    /// Returns a `ClientBuilder` for fluent construction.
    pub fn builder() -> ClientBuilder {
        ClientBuilder::default()
    }

    fn new(config: ClientConfig, signing_key: SigningKey) -> Result<Self, String> {
        let did = Did::new(signing_key.verifying_key().to_bytes());
        let routing = Arc::new(RoutingTable::new());

        // Create transport with custom config if a listen_port is specified
        // IMPORTANT: share the routing table so register_peer() works
        let transport = if let Some(port) = config.listen_port {
            use crate::runtime::transport::TransportConfig;
            let listen_addr = format!("127.0.0.1:{}", port).parse().unwrap();
            let transport_config = TransportConfig::new(
                listen_addr,
                listen_addr, // connect_addr same as listen for single-machine
                format!("127.0.0.1:{}", port + 443).parse().unwrap(),
            );
            Arc::new(TransportLayer::with_config_and_routing(&config.mode, transport_config, routing.clone())?)
        } else {
            // Use new_with_did so mock transport only pulls messages for this DID
            Arc::new(TransportLayer::new_with_did(&config.mode, did)?)
        };

        let security = Arc::new(SecurityLayer::new(&config.mode, None));
        let inbox = Arc::new(Mutex::new(VecDeque::new()));
        let inbox_notify = Arc::new(Condvar::new());
        let shutdown = Arc::new(AtomicBool::new(false));

        let client = Self {
            did,
            signing_key: Arc::new(signing_key),
            transport,
            security,
            routing,
            inbox,
            inbox_notify,
            receive_buffer_capacity: config.receive_buffer_capacity,
            seq: Arc::new(Mutex::new(0)),
            shutdown,
            messages_dropped: Arc::new(AtomicU64::new(0)),
            messages_received: Arc::new(AtomicU64::new(0)),
            messages_sent: Arc::new(AtomicU64::new(0)),
        };

        // Spin up the background poller thread
        client.start_poller();

        Ok(client)
    }

    /// Register a known peer so messages can be routed to them.
    ///
    /// Call this with the DID and IP:port of each neighbor node you know about.
    /// In a full deployment, the gossip protocol discovers these automatically.
    pub fn register_peer(&self, did: Did, addr: &str) -> Result<(), String> {
        let socket_addr = addr.parse().map_err(|e| format!("Invalid address '{}': {}", addr, e))?;
        self.routing.update_route(did, socket_addr);
        Ok(())
    }

    /// Send a strongly-typed payload to a recipient node identified by DID.
    ///
    /// Handles signing, serialization, sequence numbering, and routing automatically.
    /// Returns an error if the client has been shut down, the payload is too large,
    /// or the transport layer fails.
    pub fn send(&self, recipient: Did, payload: EnvelopePayload) -> Result<(), String> {
        if self.shutdown.load(Ordering::Relaxed) {
            return Err("Client has been shut down".to_string());
        }

        let encoded = encode_payload(&payload);
        if encoded.is_empty() {
            return Err("Cannot send empty payload".to_string());
        }
        if encoded.len() > DEFAULT_PAYLOAD_CAPACITY {
            return Err(format!(
                "Payload too large: {} bytes (max {})",
                encoded.len(), DEFAULT_PAYLOAD_CAPACITY
            ));
        }

        let seq = {
            let mut s = self.seq.lock().map_err(|e| e.to_string())?;
            *s += 1;
            *s
        };

        let now_us = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_micros() as u64;

        // Generate a unique MessageId from sequence + timestamp
        let mut id_bytes = [0u8; 16];
        id_bytes[..8].copy_from_slice(&seq.to_le_bytes());
        id_bytes[8..].copy_from_slice(&now_us.to_le_bytes());

        let header = EnvelopeHeader {
            version: 7,
            message_id: MessageId(id_bytes),
            sender_did: self.did,
            recipient_did: recipient,
            sequence_number: seq,
            timestamp_us: now_us,
            priority: Priority::Normal,
            payload_type: PayloadType::Raw,
        };

        let mut storage = PayloadStorage::<DEFAULT_PAYLOAD_CAPACITY>::new();
        storage.push_bytes(&encoded).map_err(|e| format!("{:?}", e))?;

        let envelope = SignedEnvelope::sign(header, storage, &self.signing_key);
        let result = self.transport.send(&envelope);
        if result.is_ok() {
            self.messages_sent.fetch_add(1, Ordering::Relaxed);
        }
        result
    }

    /// Try to receive a decoded message without blocking.
    ///
    /// Returns `None` immediately if no messages are available.
    pub fn try_receive(&self) -> Option<ReceivedMessage> {
        self.inbox.lock().ok()?.pop_front()
    }

    /// Block until a message is received or the timeout elapses.
    ///
    /// Uses a condition variable for efficient waiting — does not busy-poll.
    /// Returns `None` if the timeout expires or the client is shut down.
    pub fn receive_timeout(&self, timeout: Duration) -> Option<ReceivedMessage> {
        let deadline = Instant::now() + timeout;
        let mut guard = self.inbox.lock().ok()?;

        loop {
            if let Some(msg) = guard.pop_front() {
                return Some(msg);
            }
            if self.shutdown.load(Ordering::Relaxed) {
                return None;
            }
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                return None;
            }
            let (new_guard, timeout_result) = self.inbox_notify.wait_timeout(guard, remaining).ok()?;
            guard = new_guard;
            if timeout_result.timed_out() {
                return guard.pop_front();
            }
        }
    }

    /// Returns the number of messages currently queued in the inbox.
    pub fn inbox_len(&self) -> usize {
        self.inbox.lock().map(|q| q.len()).unwrap_or(0)
    }

    /// Returns a snapshot of all known routes (DID → SocketAddr pairs).
    pub fn known_peers(&self) -> Vec<(Did, std::net::SocketAddr)> {
        self.routing.gossip_routes()
    }

    /// Gracefully shut down the client, stopping the background poller thread.
    ///
    /// After calling this, `send()` and `receive_timeout()` will return errors/None.
    /// Any messages still in the inbox can be drained with `try_receive()`.
    pub fn shutdown(&self) {
        self.shutdown.store(true, Ordering::SeqCst);
        // Wake up any threads waiting on the condvar
        self.inbox_notify.notify_all();
    }

    /// Returns true if the client has been shut down.
    pub fn is_shutdown(&self) -> bool {
        self.shutdown.load(Ordering::Relaxed)
    }

    /// Returns a health check snapshot for monitoring.
    ///
    /// Useful for liveness probes, dashboards, and alerting.
    pub fn health(&self) -> ClientHealth {
        ClientHealth {
            did: self.did,
            is_shutdown: self.shutdown.load(Ordering::Relaxed),
            inbox_len: self.inbox_len(),
            inbox_capacity: self.receive_buffer_capacity,
            messages_sent: self.messages_sent.load(Ordering::Relaxed),
            messages_received: self.messages_received.load(Ordering::Relaxed),
            messages_dropped: self.messages_dropped.load(Ordering::Relaxed),
            known_peers: self.routing.gossip_routes().len(),
        }
    }

    // ── Internal ──────────────────────────────────────────────────────────────

    fn start_poller(&self) {
        let transport = self.transport.clone();
        let security = self.security.clone();
        let inbox = self.inbox.clone();
        let inbox_notify = self.inbox_notify.clone();
        let self_did = self.did;
        let capacity = self.receive_buffer_capacity;
        let shutdown = self.shutdown.clone();
        let messages_received = self.messages_received.clone();
        let messages_dropped = self.messages_dropped.clone();

        thread::Builder::new()
            .name(format!("omnimesh-poller-{}", hex::encode(&self_did.0[..4])))
            .spawn(move || {
                while !shutdown.load(Ordering::Relaxed) {
                    match transport.receive() {
                        Some(envelope) => {
                            // Only accept messages addressed to us
                            if envelope.header.recipient_did != self_did {
                                continue;
                            }

                            // Verify signature
                            if security.verify(&envelope).is_err() {
                                continue;
                            }

                            // Decode payload
                            let payload = match decode_payload(envelope.payload.as_slice()) {
                                Ok(p) => p,
                                Err(_) => continue,
                            };

                            let received_at_us = SystemTime::now()
                                .duration_since(UNIX_EPOCH)
                                .unwrap_or_default()
                                .as_micros() as u64;

                            let msg = ReceivedMessage {
                                sender: envelope.header.sender_did,
                                payload,
                                received_at_us,
                            };

                            if let Ok(mut q) = inbox.lock() {
                                if q.len() < capacity {
                                    q.push_back(msg);
                                    messages_received.fetch_add(1, Ordering::Relaxed);
                                    // Notify any waiting receivers
                                    inbox_notify.notify_one();
                                } else {
                                    messages_dropped.fetch_add(1, Ordering::Relaxed);
                                }
                            }
                        }
                        None => {
                            // Yield briefly — use a short sleep to avoid spinning
                            thread::sleep(Duration::from_micros(500));
                        }
                    }
                }
            })
            .expect("Failed to spawn poller thread");
    }
}

impl std::fmt::Debug for OmnimeshClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OmnimeshClient")
            .field("did", &hex::encode(&self.did.0))
            .field("inbox_len", &self.inbox_len())
            .field("is_shutdown", &self.is_shutdown())
            .field("messages_sent", &self.messages_sent.load(Ordering::Relaxed))
            .field("messages_received", &self.messages_received.load(Ordering::Relaxed))
            .finish()
    }
}

/// Health check snapshot for monitoring and liveness probes.
#[derive(Debug, Clone)]
pub struct ClientHealth {
    pub did: Did,
    pub is_shutdown: bool,
    pub inbox_len: usize,
    pub inbox_capacity: usize,
    pub messages_sent: u64,
    pub messages_received: u64,
    pub messages_dropped: u64,
    pub known_peers: usize,
}

impl ClientHealth {
    /// Returns true if the client is healthy (not shut down, inbox not full).
    pub fn is_healthy(&self) -> bool {
        !self.is_shutdown && self.inbox_len < self.inbox_capacity
    }

    /// Returns the inbox utilization as a percentage (0.0 to 1.0).
    pub fn inbox_utilization(&self) -> f64 {
        if self.inbox_capacity == 0 {
            return 1.0;
        }
        self.inbox_len as f64 / self.inbox_capacity as f64
    }
}
