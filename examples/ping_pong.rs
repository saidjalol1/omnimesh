//! Simple ping-pong example using the new Developer API SDK
//!
//! Demonstrates basic message sending and receiving between two nodes
//! without needing to worry about the underlying cryptography or networking.
//!
//! Usage:
//!   cargo run --example ping_pong

use omnimesh::client::{OmnimeshClient, ClientConfig};
use omnimesh::payload;
use omnimesh::payload::PayloadKind;
use std::time::Duration;

fn main() {
    println!("=== OMNI-MESH Ping-Pong (SDK Edition) ===\n");
    
    // Create Node A (Ping sender)
    let node_a = OmnimeshClient::builder()
        .with_config(ClientConfig::development())
        .build()
        .expect("Failed to build Node A");
        
    // Create Node B (Ping receiver / Pong sender)
    let node_b = OmnimeshClient::builder()
        .with_config(ClientConfig::development())
        .build()
        .expect("Failed to build Node B");
        
    println!("Node A DID: {:?}", hex::encode(&node_a.did.0[..8]));
    println!("Node B DID: {:?}\n", hex::encode(&node_b.did.0[..8]));
    
    println!("Nodes initialized. Starting ping-pong...\n");
    
    // Node A sends "PING" as an AgentCommand payload
    let ping_msg = payload::agent_command("PING", b"data", b"");
    println!("[Node A] Sending PING...");
    node_a.send(node_b.did, ping_msg).expect("Failed to send PING");
    
    // Node B waits for message
    if let Some(msg) = node_b.receive_timeout(Duration::from_millis(500)) {
        if let Some(PayloadKind::AgentCommand(cmd)) = msg.payload.payload {
            println!("[Node B] Received: {}", cmd.command_type);
            
            // Node B sends "PONG"
            let pong_msg = payload::agent_command("PONG", b"data", b"");
            println!("[Node B] Sending PONG...");
            node_b.send(node_a.did, pong_msg).expect("Failed to send PONG");
        }
    } else {
        println!("[Node B] Timed out waiting for PING");
    }
    
    // Node A waits for response
    if let Some(msg) = node_a.receive_timeout(Duration::from_millis(500)) {
        if let Some(PayloadKind::AgentCommand(cmd)) = msg.payload.payload {
            println!("[Node A] Received: {}", cmd.command_type);
        }
    } else {
        println!("[Node A] Timed out waiting for PONG");
    }
    
    println!("\n=== Ping-Pong Complete ===");
    println!("This example demonstrated:");
    println!("  ✓ The new Developer API SDK (OmnimeshClient)");
    println!("  ✓ Sending strictly-typed payloads");
    println!("  ✓ Automatic cryptographic signing & verification");
    println!("  ✓ Background async daemon loop");
}
