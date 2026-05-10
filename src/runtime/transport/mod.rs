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
//! - **QUIC**: Secure, multiplexed delivery over QUIC
//!
//! ## Usage Examples
//!
//! ### Basic Usage
//! ```rust,ignore
//! use omnimesh::runtime::transport::TransportLayer;
//! use omnimesh::config::OmnimeshMode;
//!
//! let mode = OmnimeshMode::development();
//! let transport = TransportLayer::new(&mode)?;
//! transport.initialize()?;
//!
//! if let Some(envelope) = transport.receive() {
//!     println!("Received envelope: {:?}", envelope.header);
//! }
//! ```
//!
//! ### Custom Configuration
//! ```rust,ignore
//! use omnimesh::runtime::transport::{TransportLayer, TransportConfig};
//! use omnimesh::config::OmnimeshMode;
//!
//! let config = TransportConfig::new(
//!     "0.0.0.0:8001".parse()?,
//!     "127.0.0.1:8001".parse()?,
//!     "0.0.0.0:4433".parse()?,
//! );
//!
//! let transport = TransportLayer::with_config(
//!     &OmnimeshMode::lightweight(),
//!     config
//! )?;
//! ```
//!
//! ### Mode-Based Selection
//! ```rust,ignore
//! use omnimesh::runtime::transport::TransportLayer;
//! use omnimesh::config::OmnimeshMode;
//!
//! // Automatically selects mock transport
//! let dev = TransportLayer::new(&OmnimeshMode::development())?;
//!
//! // Automatically selects TCP transport
//! let light = TransportLayer::new(&OmnimeshMode::lightweight())?;
//!
//! // Automatically selects QUIC transport
//! let prod = TransportLayer::new(&OmnimeshMode::production())?;
//! ```
//!
//! ## Module Organization
//!
//! - [`interface`] - Core `Transport` trait
//! - [`config`] - Network configuration
//! - [`mock`] - Mock transport for testing
//! - [`tcp`] - TCP transport using Tokio
//! - [`quic`] - QUIC transport using Quinn
//! - [`layer`] - Transport facade and factory
//! - [`common`] - Shared utilities
//! - [`cert`] - Certificate utilities
//! - [`compression`] - Optional message compression
//! - [`tests`] - Integration tests
//!
//! ## Thread Safety
//!
//! All transports are thread-safe and designed for concurrent access.
//! Uses `Arc<Mutex<>>` for channel receivers and `tokio` for async operations.

pub mod common;
pub mod compression;
pub mod config;
pub mod cert;
pub mod interface;
pub mod layer;
pub mod mock;
pub mod quic;
pub mod tcp;
pub mod tests;

// Re-export the main types for convenience
pub use config::TransportConfig;
pub use compression::{CompressionConfig, compress, decompress, compression_ratio};
pub use interface::{Transport, DEFAULT_PAYLOAD_CAPACITY};
pub use layer::TransportLayer;

// Re-export transport implementations for advanced usage
pub use mock::MockTransport;
pub use quic::QuicTransport;
pub use tcp::TcpTransport;