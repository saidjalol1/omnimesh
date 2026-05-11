//! Tests for the transport layer facade.

#![allow(unused_imports)]

use crate::config::OmnimeshMode;
use crate::runtime::transport::TransportLayer;
use super::helpers::{assert_transport_kind, create_and_initialize_transport};

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
    assert_transport_kind(&OmnimeshMode::development(), "mock transport");
    assert_transport_kind(&OmnimeshMode::lightweight(), "tcp transport");
    // V8: Production uses TCP for stability (QUIC in v8.3)
    assert_transport_kind(&OmnimeshMode::production(), "tcp transport");
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

#[test]
fn transport_layer_development_uses_mock() {
    assert_transport_kind(&OmnimeshMode::development(), "mock transport");
}

#[test]
fn transport_layer_lightweight_uses_tcp() {
    assert_transport_kind(&OmnimeshMode::lightweight(), "tcp transport");
}

#[test]
fn transport_layer_production_uses_tcp() {
    // V8: Production uses TCP for stability (QUIC in v8.3)
    assert_transport_kind(&OmnimeshMode::production(), "tcp transport");
}

#[test]
fn transport_layer_with_custom_config() {
    use crate::runtime::transport::TransportConfig;
    
    let mode = OmnimeshMode::lightweight();
    let config = TransportConfig::new(
        "127.0.0.1:9001".parse().unwrap(),
        "127.0.0.1:9001".parse().unwrap(),
        "127.0.0.1:9443".parse().unwrap(),
    );
    let layer = TransportLayer::with_config(&mode, config).unwrap();
    assert_eq!(layer.kind(), "tcp transport");
}
