use omnimesh::buffer::PayloadStorage;
use omnimesh::config::OmnimeshMode;
use omnimesh::envelope::{EnvelopeHeader, MessageId, Did, PayloadType, Priority, SignedEnvelope};
use omnimesh::runtime::security::SecurityLayer;

#[test]
fn security_verify_accepts_signed_envelope() {
    let security = SecurityLayer::new(&OmnimeshMode::development());
    let header = EnvelopeHeader::new(
        1,
        MessageId::new([0x01; 16]),
        Did::new([0x02; 32]),
        Did::new([0x03; 32]),
        1,
        1_700_000_000_000_000,
        Priority::Normal,
        PayloadType::Raw,
    );
    let payload = PayloadStorage::<128>::new();
    let envelope = SignedEnvelope::new(header, payload, [1u8; 64]);

    assert!(security.verify(&envelope).is_ok());
}

#[test]
fn security_verify_rejects_unsigned_envelope() {
    let security = SecurityLayer::new(&OmnimeshMode::development());
    let header = EnvelopeHeader::new(
        1,
        MessageId::new([0x01; 16]),
        Did::new([0x02; 32]),
        Did::new([0x03; 32]),
        1,
        1_700_000_000_000_000,
        Priority::Normal,
        PayloadType::Raw,
    );
    let payload = PayloadStorage::<128>::new();
    let envelope = SignedEnvelope::new(header, payload, [0u8; 64]);

    assert!(security.verify(&envelope).is_err());
}