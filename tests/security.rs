use omnimesh::buffer::PayloadStorage;
use omnimesh::config::OmnimeshMode;
use omnimesh::envelope::{EnvelopeHeader, MessageId, Did, PayloadType, Priority, SignedEnvelope};
use omnimesh::runtime::security::SecurityLayer;
use ed25519_dalek::SigningKey;
use rand_core::OsRng;

#[test]
fn security_verify_accepts_signed_envelope() {
    let security = SecurityLayer::new(&OmnimeshMode::production(), None);
    
    let mut csprng = OsRng;
    let keypair = SigningKey::generate(&mut csprng);
    
    let header = EnvelopeHeader::new(
        1,
        MessageId::new([0x01; 16]),
        Did::new(keypair.verifying_key().to_bytes()),
        Did::new([0x03; 32]),
        1,
        1_700_000_000_000_000,
        Priority::Normal,
        PayloadType::Raw,
    );
    let payload = PayloadStorage::<128>::new();
    
    let envelope = SignedEnvelope::sign(header, payload, &keypair);

    assert!(security.verify(&envelope).is_ok());
}

#[test]
fn security_verify_rejects_unsigned_envelope_in_production() {
    let security = SecurityLayer::new(&OmnimeshMode::production(), None);
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