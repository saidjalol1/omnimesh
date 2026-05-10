//! Unit tests for QuicTransport

#![allow(unused_imports)]

use crate::runtime::transport::quic::QuicTransport;
use crate::runtime::transport::interface::Transport;
use crate::runtime::transport::config::TransportConfig;

#[test]
fn quic_transport_initializes_with_config() {
    let config = TransportConfig::default();
    let transport = QuicTransport::new(config, std::sync::Arc::new(crate::runtime::RoutingTable::new()));
    assert!(transport.is_ok());
}

#[test]
fn quic_transport_kind_is_correct() {
    let config = TransportConfig::default();
    let transport = QuicTransport::new(config, std::sync::Arc::new(crate::runtime::RoutingTable::new())).unwrap();
    assert_eq!(transport.kind(), "quic transport");
}

#[test]
fn quic_transport_send_succeeds() {
    let config = TransportConfig::default();
    let transport = QuicTransport::new(config, std::sync::Arc::new(crate::runtime::RoutingTable::new())).unwrap();

    let envelope = crate::runtime::transport::common::TransportUtils::sample_envelope();
    let result = transport.send(&envelope);
    assert!(result.is_ok());
}
