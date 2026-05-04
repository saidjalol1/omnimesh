//! Tests for the transport layer facade.

use crate::config::OmnimeshMode;
use crate::runtime::transport::TransportLayer;

#[test]
fn transport_receive_returns_valid_envelope() {
    let transport = TransportLayer::new(&OmnimeshMode::development())
        .expect("transport creation failed");
    let envelope = transport.receive().expect("expected envelope");

    assert_eq!(envelope.header.version, 1);
    assert_eq!(envelope.payload.as_slice(), b"hello omnimesh");
    assert_eq!(envelope.signature, [1u8; 64]);
}

#[test]
fn transport_layer_kind_matches_mode() {
    let dev_transport = TransportLayer::new(&OmnimeshMode::development())
        .expect("transport creation failed");
    assert_eq!(dev_transport.kind(), "mock transport");

    let lw_transport = TransportLayer::new(&OmnimeshMode::lightweight())
        .expect("transport creation failed");
    assert_eq!(lw_transport.kind(), "tcp transport");

    let prod_transport = TransportLayer::new(&OmnimeshMode::production())
        .expect("transport creation failed");
    assert_eq!(prod_transport.kind(), "quic transport");
}

#[test]
fn transport_send_succeeds_with_mock() {
    let transport = TransportLayer::new(&OmnimeshMode::development())
        .expect("transport creation failed");
    let envelope = transport.receive().expect("expected envelope");

    let result = transport.send(&envelope);
    assert!(result.is_ok());
}

#[test]
fn transport_send_succeeds_with_tcp() {
    let transport = TransportLayer::new(&OmnimeshMode::lightweight())
        .expect("transport creation failed");
    let envelope = crate::runtime::transport::common::TransportUtils::sample_envelope();

    let result = transport.send(&envelope);
    // Should handle gracefully even if connection fails
    assert!(result.is_ok());
}

#[test]
fn transport_initialize_succeeds() {
    let transport = TransportLayer::new(&OmnimeshMode::development())
        .expect("transport creation failed");
    let result = transport.initialize();
    assert!(result.is_ok());
}
