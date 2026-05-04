use crate::buffer::PayloadStorage;
use crate::envelope::{Did, EnvelopeHeader, MessageId, PayloadType, Priority, SignedEnvelope};
use crate::runtime::transport::interface::DEFAULT_PAYLOAD_CAPACITY;

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
            1, // version
            MessageId::new([0x01; 16]), // message id
            Did::new([0x02; 32]), // sender
            Did::new([0x03; 32]), // recipient
            1, // ttl
            1_700_000_000_000_000, // timestamp
            Priority::Normal,
            PayloadType::Raw,
        );

        SignedEnvelope::new(header, payload, [1u8; 64]) // signature
    }
}