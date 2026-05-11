//! Tests for mode-based transport selection.

#![allow(unused_imports)]

use crate::config::OmnimeshMode;
use crate::runtime::transport::TransportLayer;
use super::helpers::{assert_transport_kind, create_and_initialize_transport};

#[test]
fn transport_development_mode_uses_mock() {
    assert_transport_kind(&OmnimeshMode::development(), "mock transport");
}

#[test]
fn transport_lightweight_mode_uses_tcp() {
    assert_transport_kind(&OmnimeshMode::lightweight(), "tcp transport");
}

#[test]
fn transport_production_mode_uses_tcp() {
    // V8: Production now uses TCP for stability (QUIC in v8.3)
    assert_transport_kind(&OmnimeshMode::production(), "tcp transport");
}

#[test]
fn mock_transport_sends_envelope() {
    let mode = OmnimeshMode::development();
    let transport = TransportLayer::new(&mode).expect("transport creation failed");

    let envelope = transport
        .receive()
        .expect("expected to receive mock envelope");

    transport
        .send(&envelope)
        .expect("send should succeed");

    assert_eq!(transport.kind(), "mock transport");
}

#[test]
fn tcp_transport_initializes_successfully() {
    assert_transport_kind(&OmnimeshMode::lightweight(), "tcp transport");
}

#[test]
fn all_modes_initialize_successfully() {
    let modes = vec![
        OmnimeshMode::development(),
        OmnimeshMode::lightweight(),
        OmnimeshMode::production(),
    ];

    for mode in modes {
        let _transport = create_and_initialize_transport(&mode)
            .expect(&format!("transport initialization failed for {:?}", mode));
    }
}
