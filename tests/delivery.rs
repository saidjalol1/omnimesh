use omnimesh::buffer::PayloadStorage;
use omnimesh::config::OmnimeshMode;
use omnimesh::envelope::{EnvelopeHeader, MessageId, Did, PayloadType, Priority, SignedEnvelope};
use omnimesh::runtime::delivery::DeliveryLayer;

#[test]
fn delivery_deliver_succeeds() {
    let delivery = DeliveryLayer::new(&OmnimeshMode::development());
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

    assert!(delivery.deliver(&envelope).is_ok());
}