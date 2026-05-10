//! Unit tests for Transport trait interface
//!
//! The Transport trait is tested primarily through its implementations
//! (MockTransport, TcpTransport, QuicTransport). See their respective
//! test modules for comprehensive interface verification.

#![allow(unused_imports)]

use crate::runtime::transport::interface::Transport;

#[test]
fn transport_trait_is_object_safe() {
    // This test verifies that Transport can be used as a trait object
    let _: &dyn Transport;
}
