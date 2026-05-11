//! Unit tests for QuicTransport

#![allow(unused_imports)]

use crate::runtime::transport::quic::QuicTransport;
use crate::runtime::transport::interface::Transport;
use crate::runtime::transport::config::TransportConfig;

#[test]
fn quic_transport_initializes_with_config() {
    // Use unique ports to avoid conflicts
    let config = TransportConfig::new(
        "127.0.0.1:0".parse().unwrap(), // Port 0 = OS assigns random port
        "127.0.0.1:9999".parse().unwrap(),
        "127.0.0.1:0".parse().unwrap(), // Port 0 = OS assigns random port
    );
    let transport = QuicTransport::new(config, std::sync::Arc::new(crate::runtime::RoutingTable::new()));
    assert!(transport.is_ok());
}

#[test]
fn quic_transport_kind_is_correct() {
    // Use unique ports to avoid conflicts
    let config = TransportConfig::new(
        "127.0.0.1:0".parse().unwrap(),
        "127.0.0.1:9998".parse().unwrap(),
        "127.0.0.1:0".parse().unwrap(),
    );
    let transport = QuicTransport::new(config, std::sync::Arc::new(crate::runtime::RoutingTable::new())).unwrap();
    assert_eq!(transport.kind(), "quic transport");
}

#[test]
fn quic_transport_send_succeeds() {
    // Use unique ports to avoid conflicts
    let config = TransportConfig::new(
        "127.0.0.1:0".parse().unwrap(),
        "127.0.0.1:9997".parse().unwrap(),
        "127.0.0.1:0".parse().unwrap(),
    );
    let transport = QuicTransport::new(config, std::sync::Arc::new(crate::runtime::RoutingTable::new())).unwrap();

    let envelope = crate::runtime::transport::common::TransportUtils::sample_envelope();
    let result = transport.send(&envelope);
    assert!(result.is_ok());
}
