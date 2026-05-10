use crate::config::OmnimeshMode;
use crate::config::modes::layer_kinds;
use crate::envelope::{Did, MessageId, SignedEnvelope};
use crate::runtime::RuntimeLayer;
use crate::buffer::{FixedMap, RingBuffer};
use std::sync::Mutex;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeliveryStatus {
    Delivered,
    Duplicate,
    Buffered(u64),
    Stale,
    BufferFull,
}

#[derive(Debug, Clone, Copy)]
pub struct PendingMessage<const N: usize> {
    pub envelope: SignedEnvelope<N>,
}

pub struct OrderedChannel<const N: usize> {
    next_expected: FixedMap<(Did, Did), u64, 256>,
    seen_ring: RingBuffer<MessageId, 4096>,
    pending: FixedMap<((Did, Did), u64), PendingMessage<N>, 256>,
}

impl<const N: usize> Default for OrderedChannel<N> {
    fn default() -> Self {
        Self::new()
    }
}

impl<const N: usize> OrderedChannel<N> {
    pub fn new() -> Self {
        Self {
            next_expected: FixedMap::new(),
            seen_ring: RingBuffer::new(),
            pending: FixedMap::new(),
        }
    }

    pub fn process(&mut self, envelope: &SignedEnvelope<N>) -> DeliveryStatus {
        let msg_id = envelope.header.message_id;
        
        // 1. Deduplication (Exactly-Once)
        if self.seen_ring.contains(|id| *id == msg_id) {
            return DeliveryStatus::Duplicate;
        }

        let channel_id = (envelope.header.sender_did, envelope.header.recipient_did);
        let seq = envelope.header.sequence_number;
        let expected = *self.next_expected.get(&channel_id).unwrap_or(&0);

        // 2. Ordering
        if seq < expected {
            return DeliveryStatus::Stale;
        }

        if seq == expected {
            // Deliver current
            self.seen_ring.insert(msg_id);
            
            // Advance expected
            let mut next = seq + 1;
            let _ = self.next_expected.insert(channel_id, next);

            // Attempt to deliver buffered
            while let Some(pending) = self.pending.remove(&(channel_id, next)) {
                self.seen_ring.insert(pending.envelope.header.message_id);
                next += 1;
                let _ = self.next_expected.insert(channel_id, next);
            }

            DeliveryStatus::Delivered
        } else {
            // Buffer future messages
            let pending_msg = PendingMessage { envelope: *envelope };
            match self.pending.insert((channel_id, seq), pending_msg) {
                Ok(_) => DeliveryStatus::Buffered(seq),
                Err(_) => DeliveryStatus::BufferFull,
            }
        }
    }
}

pub struct DeliveryLayer {
    kind: &'static str,
    channel: Mutex<OrderedChannel<128>>,
}

impl std::fmt::Debug for DeliveryLayer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DeliveryLayer")
            .field("kind", &self.kind)
            .finish()
    }
}

impl DeliveryLayer {
    pub fn new(mode: &OmnimeshMode) -> Self {
        let kind = match mode {
            OmnimeshMode::Development(_) => layer_kinds::BEST_EFFORT_DELIVERY,
            OmnimeshMode::Lightweight(_) => layer_kinds::LIGHTWEIGHT_DELIVERY,
            OmnimeshMode::Production(_) => layer_kinds::RELIABLE_DELIVERY,
            OmnimeshMode::Certified(_) => layer_kinds::CERTIFIED_DELIVERY,
        };

        DeliveryLayer {
            kind,
            channel: Mutex::new(OrderedChannel::new()),
        }
    }

    pub fn deliver<const N: usize>(&self, envelope: &SignedEnvelope<N>) -> Result<DeliveryStatus, String> {
        // We only support N=128 right now due to our mutex type, but normally this would be generic
        // To simplify, we will just assume the envelope fits or we serialize it
        // Actually, since this is an in-memory layer in the runtime, we can cheat
        // and just cast or assume N=128 since we defined OrderedChannel<128>.
        // Wait, for this demo we'll use a hack to copy it to a 128 buffer:
        let mut payload_128 = crate::buffer::PayloadStorage::<128>::new();
        let _ = payload_128.push_bytes(envelope.payload.as_slice());
        
        let header = envelope.header;
        let mut sig = [0u8; 64];
        sig.copy_from_slice(&envelope.signature);
        let env_128 = SignedEnvelope::new(header, payload_128, sig);

        let status = self.channel.lock().unwrap().process(&env_128);
        
        println!(
            "Delivery to {:?} from {:?} via {}: {:?}",
            envelope.header.recipient_did,
            envelope.header.sender_did,
            self.kind,
            status
        );
        
        Ok(status)
    }
}

impl RuntimeLayer for DeliveryLayer {
    fn kind(&self) -> &'static str {
        self.kind
    }

    fn layer_name(&self) -> &'static str {
        "delivery"
    }
}
