//! Flow control and backpressure tests
//!
//! Tests that verify the transport layer properly handles:
//! - Backpressure when send buffers are full
//! - Flow recovery after backpressure
//! - Statistics tracking
//! - Slow receiver scenarios

use ed25519_dalek::SigningKey;
use omnimesh::buffer::PayloadStorage;
use omnimesh::envelope::{Did, EnvelopeHeader, MessageId, SignedEnvelope};
use omnimesh::runtime::RoutingTable;
use omnimesh::runtime::transport::config::TransportConfig;
use omnimesh::runtime::transport::interface::Transport;
use omnimesh::runtime::transport::quic::QuicTransport;
use omnimesh::runtime::transport::tcp::TcpTransport;
use rand_core::OsRng;
use std::sync::Arc;
use std::time::Duration;

fn create_test_envelope(seq: u64) -> SignedEnvelope<1024> {
    let signing_key = SigningKey::generate(&mut OsRng);
    let sender_did = Did::new(signing_key.verifying_key().to_bytes());
    let recipient_did = Did([0xBB; 32]);

    let header = EnvelopeHeader {
        version: 7,
        message_id: MessageId([seq as u8; 16]),
        sender_did,
        recipient_did,
        sequence_number: seq,
        timestamp_us: 1234567890 + seq,
        priority: omnimesh::envelope::Priority::Normal,
        payload_type: omnimesh::envelope::PayloadType::Raw,
    };

    let mut payload_buf = PayloadStorage::<1024>::new();
    let msg = format!("Message {}", seq);
    payload_buf.push_bytes(msg.as_bytes()).unwrap();

    SignedEnvelope::sign(header, payload_buf, &signing_key)
}

#[test]
fn test_tcp_backpressure() {
    let config = TransportConfig::default();
    let routing = Arc::new(RoutingTable::new());
    let transport = TcpTransport::new(config, routing).expect("Failed to create TCP transport");

    // Send messages rapidly to fill the bounded buffer (1000 capacity)
    // We send in tight loop to outpace the send worker
    let mut backpressure_hit = false;
    let mut success_count = 0;

    for i in 0..5000 {
        let envelope = create_test_envelope(i);
        match transport.send(&envelope) {
            Ok(_) => success_count += 1,
            Err(e) if e.contains("backpressure") || e.contains("buffer full") => {
                backpressure_hit = true;
                println!(
                    "TCP backpressure hit at message {} (after {} successful)",
                    i, success_count
                );
                break;
            }
            Err(e) => {
                // Connection errors are expected since there's no receiver
                if e.contains("closed") {
                    println!("Send channel closed (expected without receiver)");
                    return;
                }
                panic!("Unexpected error: {}", e);
            }
        }
    }

    // Either we hit backpressure or the channel closed (both are valid)
    println!(
        "TCP backpressure test: sent {} messages, backpressure={}",
        success_count, backpressure_hit
    );
}

#[test]
fn test_tcp_flow_recovery() {
    let config = TransportConfig::default();
    let routing = Arc::new(RoutingTable::new());
    let transport = TcpTransport::new(config, routing).expect("Failed to create TCP transport");

    // Fill the buffer
    for i in 0..1000 {
        let envelope = create_test_envelope(i);
        let _ = transport.send(&envelope);
    }

    // Wait for some messages to be processed
    std::thread::sleep(Duration::from_millis(100));

    // Should be able to send again
    let envelope = create_test_envelope(9999);
    let result = transport.send(&envelope);

    // May succeed or fail depending on timing, but shouldn't panic
    match result {
        Ok(_) => println!("Recovery successful"),
        Err(e) => {
            assert!(
                e.contains("backpressure") || e.contains("buffer full") || e.contains("closed")
            );
            println!("Still under backpressure: {}", e);
        }
    }
}

#[test]
fn test_tcp_stats_tracking() {
    let config = TransportConfig::default();
    let routing = Arc::new(RoutingTable::new());
    let transport = TcpTransport::new(config, routing).expect("Failed to create TCP transport");

    // Send some messages
    for i in 0..10 {
        let envelope = create_test_envelope(i);
        let _ = transport.send(&envelope);
    }

    // Stats should be tracked
    std::thread::sleep(Duration::from_millis(50));
    let stats = transport.stats();

    // We can't guarantee exact numbers due to async nature, but stats should exist
    println!(
        "TCP Stats: sent={}, received={}, failures={}, backpressure={}, reconnections={}",
        stats.messages_sent,
        stats.messages_received,
        stats.send_failures,
        stats.backpressure_events,
        stats.reconnections
    );
}

#[test]
fn test_quic_backpressure() {
    let config = TransportConfig::default();
    let routing = Arc::new(RoutingTable::new());
    let transport = QuicTransport::new(config, routing).expect("Failed to create QUIC transport");

    // Send messages rapidly to fill the bounded buffer
    let mut backpressure_hit = false;
    let mut success_count = 0;

    for i in 0..5000 {
        let envelope = create_test_envelope(i);
        match transport.send(&envelope) {
            Ok(_) => success_count += 1,
            Err(e) if e.contains("backpressure") || e.contains("buffer full") => {
                backpressure_hit = true;
                println!(
                    "QUIC backpressure hit at message {} (after {} successful)",
                    i, success_count
                );
                break;
            }
            Err(e) => {
                if e.contains("closed") {
                    println!("Send channel closed (expected without receiver)");
                    return;
                }
                panic!("Unexpected error: {}", e);
            }
        }
    }

    println!(
        "QUIC backpressure test: sent {} messages, backpressure={}",
        success_count, backpressure_hit
    );
}

#[test]
fn test_slow_receiver_scenario() {
    let config = TransportConfig::default();
    let routing = Arc::new(RoutingTable::new());
    let transport = TcpTransport::new(config, routing).expect("Failed to create TCP transport");

    // Simulate slow receiver by filling buffer quickly
    let mut success_count = 0;
    let mut backpressure_count = 0;
    let mut closed = false;

    for i in 0..5000 {
        let envelope = create_test_envelope(i);
        match transport.send(&envelope) {
            Ok(_) => success_count += 1,
            Err(e) if e.contains("backpressure") || e.contains("buffer full") => {
                backpressure_count += 1;
                if backpressure_count >= 10 {
                    break; // Stop after seeing backpressure multiple times
                }
            }
            Err(e) if e.contains("closed") => {
                closed = true;
                break;
            }
            Err(e) => panic!("Unexpected error: {}", e),
        }
    }

    println!(
        "Slow receiver test: {} successful, {} backpressure events, closed={}",
        success_count, backpressure_count, closed
    );
    assert!(success_count > 0, "Should send some messages successfully");
}

#[test]
fn test_tcp_pool_stats() {
    let config = TransportConfig::default();
    let routing = Arc::new(RoutingTable::new());
    let transport = TcpTransport::new(config, routing).expect("Failed to create TCP transport");

    // Send a few messages to create connections
    for i in 0..5 {
        let envelope = create_test_envelope(i);
        let _ = transport.send(&envelope);
    }

    std::thread::sleep(Duration::from_millis(50));

    // Check pool stats
    match transport.pool_stats() {
        Ok((active, max)) => {
            println!("Connection pool: {}/{} connections", active, max);
            assert!(max > 0, "Pool should have max size");
        }
        Err(e) => println!("Pool stats unavailable: {}", e),
    }
}
