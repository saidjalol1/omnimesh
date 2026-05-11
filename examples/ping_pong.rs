//! Simple ping-pong example
//!
//! Demonstrates basic message sending and receiving between two nodes.
//!
//! Usage:
//!   cargo run --example ping_pong

use omnimesh::buffer::PayloadStorage;
use omnimesh::config::OmnimeshMode;
use omnimesh::envelope::{Did, EnvelopeHeader, MessageId, SignedEnvelope, Priority, PayloadType};
use omnimesh::runtime::delivery::DeliveryLayer;
use omnimesh::runtime::security::SecurityLayer;
use omnimesh::runtime::transport::config::TransportConfig;
use omnimesh::runtime::transport::interface::Transport;
use omnimesh::runtime::transport::tcp::TcpTransport;
use omnimesh::runtime::RoutingTable;
use ed25519_dalek::SigningKey;
use rand_core::OsRng;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

fn main() {
    println!("=== OMNI-MESH Ping-Pong Example ===\n");
    
    // Create two nodes
    let node_a_key = SigningKey::generate(&mut OsRng);
    let node_b_key = SigningKey::generate(&mut OsRng);
    
    let node_a_did = Did::new(node_a_key.verifying_key().to_bytes());
    let node_b_did = Did::new(node_b_key.verifying_key().to_bytes());
    
    println!("Node A DID: {:?}", hex::encode(&node_a_did.0[..8]));
    println!("Node B DID: {:?}\n", hex::encode(&node_b_did.0[..8]));
    
    // Setup transport and runtime layers
    let mode = OmnimeshMode::development();
    let config_a = TransportConfig::default();
    let config_b = TransportConfig {
        tcp_listen_addr: "127.0.0.1:9001".parse().unwrap(),
        tcp_connect_addr: "127.0.0.1:9000".parse().unwrap(),
        ..Default::default()
    };
    
    let routing_a = Arc::new(RoutingTable::new());
    let routing_b = Arc::new(RoutingTable::new());
    
    // Register routes
    routing_a.update_route(node_b_did, "127.0.0.1:9001".parse().unwrap());
    routing_b.update_route(node_a_did, "127.0.0.1:9000".parse().unwrap());
    
    let transport_a = TcpTransport::new(config_a, routing_a).expect("Failed to create transport A");
    let transport_b = TcpTransport::new(config_b, routing_b).expect("Failed to create transport B");
    
    let security_a = SecurityLayer::new(&mode, None);
    let security_b = SecurityLayer::new(&mode, None);
    
    let delivery_a = DeliveryLayer::new(&mode);
    let delivery_b = DeliveryLayer::new(&mode);
    
    println!("Nodes initialized. Starting ping-pong...\n");
    
    // Node A sends PING
    let ping_header = EnvelopeHeader {
        version: 7,
        message_id: MessageId([1u8; 16]),
        sender_did: node_a_did,
        recipient_did: node_b_did,
        sequence_number: 1,
        timestamp_us: 1234567890,
        priority: Priority::Normal,
        payload_type: PayloadType::Raw,
    };
    
    let mut ping_payload = PayloadStorage::<1024>::new();
    ping_payload.push_bytes(b"PING").unwrap();
    
    let ping_envelope = SignedEnvelope::sign(ping_header, ping_payload, &node_a_key);
    
    println!("[Node A] Sending PING...");
    transport_a.send(&ping_envelope).expect("Failed to send PING");
    
    // Wait for message to arrive
    thread::sleep(Duration::from_millis(100));
    
    // Node B receives PING
    if let Some(received) = transport_b.receive() {
        println!("[Node B] Received message");
        
        // Verify signature
        if security_b.verify(&received).is_ok() {
            println!("[Node B] Signature verified ✓");
            
            // Check for duplicates
            if let Ok(status) = delivery_b.deliver(&received) {
                println!("[Node B] Delivery status: {:?}", status);
                
                let payload_str = String::from_utf8_lossy(received.payload.as_slice());
                println!("[Node B] Payload: {}\n", payload_str);
                
                // Node B sends PONG
                let pong_header = EnvelopeHeader {
                    version: 7,
                    message_id: MessageId([2u8; 16]),
                    sender_did: node_b_did,
                    recipient_did: node_a_did,
                    sequence_number: 1,
                    timestamp_us: 1234567891,
                    priority: Priority::Normal,
                    payload_type: PayloadType::Raw,
                };
                
                let mut pong_payload = PayloadStorage::<1024>::new();
                pong_payload.push_bytes(b"PONG").unwrap();
                
                let pong_envelope = SignedEnvelope::sign(pong_header, pong_payload, &node_b_key);
                
                println!("[Node B] Sending PONG...");
                transport_b.send(&pong_envelope).expect("Failed to send PONG");
            }
        }
    }
    
    // Wait for PONG to arrive
    thread::sleep(Duration::from_millis(100));
    
    // Node A receives PONG
    if let Some(received) = transport_a.receive() {
        println!("[Node A] Received message");
        
        if security_a.verify(&received).is_ok() {
            println!("[Node A] Signature verified ✓");
            
            if let Ok(status) = delivery_a.deliver(&received) {
                println!("[Node A] Delivery status: {:?}", status);
                
                let payload_str = String::from_utf8_lossy(received.payload.as_slice());
                println!("[Node A] Payload: {}\n", payload_str);
            }
        }
    }
    
    println!("=== Ping-Pong Complete ===");
    println!("\nThis example demonstrated:");
    println!("  ✓ Message signing with Ed25519");
    println!("  ✓ Signature verification");
    println!("  ✓ TCP transport");
    println!("  ✓ Exactly-once delivery");
    println!("  ✓ Bidirectional communication");
}
