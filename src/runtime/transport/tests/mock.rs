//! Unit tests for MockTransport

#![allow(unused_imports)]

use crate::runtime::transport::mock::MockTransport;
use crate::runtime::transport::interface::Transport;
use crate::runtime::transport::common::TransportUtils;

#[test]
fn mock_transport_returns_sample_envelope() {
    let transport = MockTransport::new(std::sync::Arc::new(crate::runtime::RoutingTable::new()));
    let envelope = transport.receive().expect("should return envelope");

    assert_eq!(envelope.header.version, 1);
    assert_eq!(envelope.payload.as_slice(), b"hello omnimesh");
    assert_eq!(envelope.signature, [1u8; 64]);
}

#[test]
fn mock_transport_send_succeeds() {
    let transport = MockTransport::new(std::sync::Arc::new(crate::runtime::RoutingTable::new()));
    let envelope = TransportUtils::sample_envelope();

    let result = transport.send(&envelope);
    assert!(result.is_ok());
}

#[test]
fn mock_transport_kind_is_correct() {
    let transport = MockTransport::new(std::sync::Arc::new(crate::runtime::RoutingTable::new()));
    assert_eq!(transport.kind(), "mock transport");
}
