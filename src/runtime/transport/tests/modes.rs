//! Tests for mode-based transport selection.

use crate::config::OmnimeshMode;
use crate::runtime::transport::TransportLayer;

#[test]
fn transport_development_mode_uses_mock() {
    let mode = OmnimeshMode::development();
    let transport = TransportLayer::new(&mode).expect("transport creation failed");
    assert_eq!(transport.kind(), "mock transport");
}

#[test]
fn transport_lightweight_mode_uses_tcp() {
    let mode = OmnimeshMode::lightweight();
    let transport = TransportLayer::new(&mode).expect("transport creation failed");
    assert_eq!(transport.kind(), "tcp transport");
}

#[test]
fn transport_production_mode_uses_quic() {
    let mode = OmnimeshMode::production();
    let transport = TransportLayer::new(&mode).expect("transport creation failed");
    assert_eq!(transport.kind(), "quic transport");
}

#[test]
fn transport_certified_mode_uses_quic() {
    let mode = OmnimeshMode::certified();
    let transport = TransportLayer::new(&mode).expect("transport creation failed");
    assert_eq!(transport.kind(), "quic transport");
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
    let mode = OmnimeshMode::lightweight();
    let transport = TransportLayer::new(&mode).expect("transport creation failed");
    assert_eq!(transport.kind(), "tcp transport");
}

#[test]
fn quic_transport_initializes_successfully() {
    let mode = OmnimeshMode::production();
    let transport = TransportLayer::new(&mode).expect("transport creation failed");
    assert_eq!(transport.kind(), "quic transport");
}

#[test]
fn all_modes_initialize_successfully() {
    let modes = vec![
        OmnimeshMode::development(),
        OmnimeshMode::lightweight(),
        OmnimeshMode::production(),
        OmnimeshMode::certified(),
    ];

    for mode in modes {
        let transport = TransportLayer::new(&mode)
            .expect(&format!("transport creation failed for {:?}", mode));
        transport.initialize().expect("transport initialization failed");
    }
}
