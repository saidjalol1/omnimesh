//! Unit tests for TcpTransport

#![allow(unused_imports)]

use crate::runtime::transport::tcp::TcpTransport;
use crate::runtime::transport::interface::Transport;
use crate::runtime::transport::config::TransportConfig;

#[test]
fn tcp_transport_initializes_with_config() {
    let config = TransportConfig::default();
    let transport = TcpTransport::new(config, std::sync::Arc::new(crate::runtime::RoutingTable::new()));
    assert!(transport.is_ok());
}

#[test]
fn tcp_transport_kind_is_correct() {
    let config = TransportConfig::default();
    let transport = TcpTransport::new(config, std::sync::Arc::new(crate::runtime::RoutingTable::new())).unwrap();
    assert_eq!(transport.kind(), "tcp transport");
}

#[test]
fn tcp_transport_send_handles_connection_failure() {
    let config = TransportConfig::new(
        "127.0.0.1:8002".parse().unwrap(),
        "127.0.0.1:8003".parse().unwrap(), // Non-existent address
        "127.0.0.1:4434".parse().unwrap(),
    );
    let transport = TcpTransport::new(config, std::sync::Arc::new(crate::runtime::RoutingTable::new())).unwrap();

    let envelope = crate::runtime::transport::common::TransportUtils::sample_envelope();
    let result = transport.send(&envelope);
    // Should not panic, even if connection fails
    assert!(result.is_ok());
}

#[test]
fn tcp_transport_pool_initializes_empty() {
    let config = TransportConfig::default();
    let transport = TcpTransport::new(config, std::sync::Arc::new(crate::runtime::RoutingTable::new())).unwrap();

    let (active, max) = transport.pool_stats().expect("should get pool stats");
    assert_eq!(active, 0);
    assert_eq!(max, 10); // Default max pool size
}

#[test]
fn tcp_transport_pool_respects_max_size() {
    let config = TransportConfig::default();
    let transport = TcpTransport::new(config, std::sync::Arc::new(crate::runtime::RoutingTable::new())).unwrap();

    let (_, max) = transport.pool_stats().expect("should get pool stats");
    assert!(max >= 10, "Pool should allow at least 10 connections");
}
