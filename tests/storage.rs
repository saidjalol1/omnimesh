use omnimesh::buffer::PayloadStorage;
use omnimesh::config::OmnimeshMode;
use omnimesh::envelope::{EnvelopeHeader, MessageId, Did, PayloadType, Priority, SignedEnvelope};
use omnimesh::runtime::storage::StorageLayer;

const STORAGE_PAYLOAD_SIZE: usize = 1024; // Match DEFAULT_PAYLOAD_CAPACITY

#[test]
fn storage_store_increases_count() {
    let mut storage = StorageLayer::new(&OmnimeshMode::development());
    assert_eq!(storage.stored_count(), 0);

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
    let payload = PayloadStorage::<STORAGE_PAYLOAD_SIZE>::new();
    let envelope = SignedEnvelope::new(header, payload, [1u8; 64]);

    storage.store(envelope).unwrap();
    assert_eq!(storage.stored_count(), 1);
}

#[test]
fn storage_last_stored_returns_most_recent() {
    let mut storage = StorageLayer::new(&OmnimeshMode::development());

    let header1 = EnvelopeHeader::new(
        1,
        MessageId::new([0x01; 16]),
        Did::new([0x02; 32]),
        Did::new([0x03; 32]),
        1,
        1_700_000_000_000_000,
        Priority::Normal,
        PayloadType::Raw,
    );
    let payload1 = PayloadStorage::<STORAGE_PAYLOAD_SIZE>::new();
    let envelope1 = SignedEnvelope::new(header1, payload1, [1u8; 64]);

    let header2 = EnvelopeHeader::new(
        2,
        MessageId::new([0x02; 16]),
        Did::new([0x03; 32]),
        Did::new([0x04; 32]),
        2,
        1_700_000_000_000_001,
        Priority::High,
        PayloadType::RobotCommand,
    );
    let payload2 = PayloadStorage::<STORAGE_PAYLOAD_SIZE>::new();
    let envelope2 = SignedEnvelope::new(header2, payload2, [2u8; 64]);

    storage.store(envelope1).unwrap();
    storage.store(envelope2).unwrap();

    let last = storage.last_stored().unwrap();
    assert_eq!(last.header.version, 2);
    assert_eq!(last.signature, [2u8; 64]);
}