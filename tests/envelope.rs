use omnimesh::{
    Did, EnvelopeHeader, MessageId, PayloadStorage, PayloadType, Priority, SignedEnvelope,
};

#[test]
fn header_roundtrip() {
    let header = EnvelopeHeader::new(
        7,
        MessageId::new([1u8; 16]),
        Did::new([2u8; 32]),
        Did::new([3u8; 32]),
        42,
        1_700_000_000_000_000,
        Priority::High,
        PayloadType::RobotCommand,
    );

    let bytes = header.to_bytes();
    let decoded = EnvelopeHeader::from_bytes(&bytes).unwrap();

    assert_eq!(decoded.version, header.version);
    assert_eq!(decoded.message_id.0, header.message_id.0);
    assert_eq!(decoded.sender_did.0, header.sender_did.0);
    assert_eq!(decoded.recipient_did.0, header.recipient_did.0);
    assert_eq!(decoded.sequence_number, header.sequence_number);
    assert_eq!(decoded.timestamp_us, header.timestamp_us);
    assert_eq!(decoded.priority, header.priority);
    assert_eq!(decoded.payload_type, header.payload_type);
}

#[test]
fn signed_envelope_roundtrip() {
    let mut payload = PayloadStorage::<128>::new();
    payload.push_bytes(b"hello world").unwrap();

    let envelope = SignedEnvelope::new(
        EnvelopeHeader::new(
            7,
            MessageId::new([1u8; 16]),
            Did::new([2u8; 32]),
            Did::new([3u8; 32]),
            42,
            1_700_000_000_000_000,
            Priority::Normal,
            PayloadType::Raw,
        ),
        payload,
        [7u8; 64],
    );

    let mut buf = [0u8; 2048];
    let len = envelope.serialize_into(&mut buf).unwrap();
    let bytes = &buf[..len];
    let decoded = SignedEnvelope::<128>::deserialize(&bytes).unwrap();

    assert_eq!(decoded.header.version, envelope.header.version);
    assert_eq!(decoded.payload.as_slice(), envelope.payload.as_slice());
    assert_eq!(decoded.signature, envelope.signature);
}
