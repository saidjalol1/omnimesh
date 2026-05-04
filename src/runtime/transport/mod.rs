//! Transport layer for OMNI-MESH runtime.
//!
//! This module provides pluggable transport implementations for message delivery
//! over various network protocols. The transport layer is designed to be:
//!
//! - **Extensible**: Easy to add new transport protocols
//! - **Configurable**: Network settings can be customized per deployment
//! - **Testable**: Mock transport for development and testing
//! - **Async**: Built on Tokio for high-performance I/O
//!
//! ## Architecture
//!
//! The transport layer follows a trait-based design:
//!
//! - [`Transport`] trait defines the interface for all transport implementations
//! - [`TransportLayer`] provides a facade over concrete transport types
//! - Mode-based selection automatically chooses the appropriate transport
//!
//! ## Transport Types
//!
//! - **Mock**: For testing and development (no network I/O)
//! - **TCP**: Reliable, ordered delivery over TCP
//! - **QUIC**: Secure, multiplexed delivery over QUIC (simulated)
//!
//! ## Usage
//!
//! ```rust,ignore
//! use omnimesh::runtime::transport::TransportLayer;
//! use omnimesh::config::OmnimeshMode;
//!
//! let mode = OmnimeshMode::development();
//! let transport = TransportLayer::new(&mode)?;
//! transport.initialize()?;
//!
//! if let Some(envelope) = transport.receive() {
//!     // Process received envelope
//! }
//! ```

pub mod common;
pub mod config;
pub mod interface;
pub mod layer;
pub mod mock;
pub mod quic;
pub mod tcp;
pub mod tests;

// Re-export the main types for convenience
pub use config::TransportConfig;
pub use interface::{Transport, DEFAULT_PAYLOAD_CAPACITY};
pub use layer::TransportLayer;

// Re-export transport implementations for advanced usage
pub use mock::MockTransport;
pub use quic::QuicTransport;
pub use tcp::TcpTransport;