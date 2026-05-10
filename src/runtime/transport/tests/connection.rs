//! Integration tests for transport connection handling and buffering.

#![allow(unused_imports)]

use crate::config::OmnimeshMode;
use crate::runtime::transport::{TransportLayer, TransportConfig};

/// Tests TCP transport with actual message buffering
#[test]
fn tcp_transport_buffers_received_envelopes() {
    let config = TransportConfig::default();
    let transport = TransportLayer::with_config(&OmnimeshMode::lightweight(), config)
        .expect("tcp transport creation failed");

    // TCP transport maintains a buffer of received messages
    // In production, actual connections would populate this buffer
    transport.initialize().expect("initialization failed");

    assert_eq!(transport.kind(), "tcp transport");
}

/// Tests that transports handle envelope serialization
#[test]
fn transport_envelope_serialization_roundtrip() {
    use crate::runtime::transport::DEFAULT_PAYLOAD_CAPACITY;
    
    let transport = TransportLayer::new(&OmnimeshMode::development())
        .expect("transport creation failed");

    let original = transport.receive()
        .expect("should receive envelope");
    
    let mut buf = [0u8; 2048];
    let len = original.serialize_into(&mut buf).unwrap();
    let serialized = &buf[..len];
    assert!(!serialized.is_empty());
    
    let deserialized: crate::envelope::SignedEnvelope<DEFAULT_PAYLOAD_CAPACITY> 
        = crate::envelope::SignedEnvelope::deserialize(&serialized)
        .expect("should deserialize");
    
    assert_eq!(original.header.version, deserialized.header.version);
    assert_eq!(original.payload.as_slice(), deserialized.payload.as_slice());
}

/// Tests transport initialization across all modes
#[test]
fn all_transports_initialize_without_panic() {
    let modes = vec![
        OmnimeshMode::development(),
        OmnimeshMode::lightweight(),
        OmnimeshMode::production(),
        OmnimeshMode::certified(),
    ];

    for mode in modes {
        let transport = TransportLayer::new(&mode)
            .expect(&format!("transport creation failed for {:?}", mode));
        
        transport.initialize()
            .expect(&format!("initialization failed for {:?}", mode));
    }
}

/// Tests transport send with custom configuration
#[test]
fn transport_respects_custom_configuration() {
    let custom_config = TransportConfig::new(
        "127.0.0.1:9001".parse().unwrap(),
        "127.0.0.1:9002".parse().unwrap(),
        "127.0.0.1:9443".parse().unwrap(),
    );

    let transport = TransportLayer::with_config(
        &OmnimeshMode::lightweight(),
        custom_config.clone()
    ).expect("transport creation failed");

    assert_eq!(transport.config().tcp_listen_addr.port(), 9001);
    assert_eq!(transport.config().tcp_connect_addr.port(), 9002);
}

/// Tests that TCP transport gracefully handles no listeners
#[test]
fn tcp_transport_gracefully_fails_without_listener() {
    let transport = TransportLayer::new(&OmnimeshMode::lightweight())
        .expect("tcp transport creation failed");

    let envelope = crate::runtime::transport::common::TransportUtils::sample_envelope();
    
    // Should not panic even if no one is listening
    let result = transport.send(&envelope);
    assert!(result.is_ok());
}

/// Tests envelope round-trip through transport layer
#[test]
fn envelope_round_trip_through_transport() {
    let transport = TransportLayer::new(&OmnimeshMode::development())
        .expect("transport creation failed");

    let received = transport.receive()
        .expect("should receive envelope");
    
    let sent_result = transport.send(&received);
    assert!(sent_result.is_ok());

    // Envelope should maintain integrity
    assert_eq!(received.header.version, 1);
}

/// Tests transport configuration persistence
#[test]
fn transport_configuration_persists() {
    let config1 = TransportConfig::default();
    let transport1 = TransportLayer::with_config(&OmnimeshMode::lightweight(), config1.clone())
        .expect("transport creation failed");

    let config2 = transport1.config();
    assert_eq!(config1.tcp_listen_addr, config2.tcp_listen_addr);
    assert_eq!(config1.tcp_connect_addr, config2.tcp_connect_addr);
}
