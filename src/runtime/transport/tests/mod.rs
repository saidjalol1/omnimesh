//! Integration tests for the transport layer.
//!
//! These tests verify the behavior of the transport abstraction
//! and its various implementations.

// Unit tests for individual transport types
pub mod cert;
pub mod compression;
pub mod interface;
pub mod mock;
pub mod quic;
pub mod tcp;

// Integration tests across transport layer
pub mod connection;
pub mod layer;
pub mod modes;

/// Test helper utilities
pub mod helpers {
    use crate::config::OmnimeshMode;
    use crate::runtime::transport::TransportLayer;

    /// Helper to create and verify transport kind
    pub fn assert_transport_kind(mode: &OmnimeshMode, expected_kind: &str) {
        let transport = TransportLayer::new(mode).expect("transport creation failed");
        assert_eq!(transport.kind(), expected_kind);
    }

    /// Helper to create transport and initialize
    pub fn create_and_initialize_transport(mode: &OmnimeshMode) -> Result<TransportLayer, String> {
        let transport = TransportLayer::new(mode)?;
        transport.initialize()?;
        Ok(transport)
    }
}
