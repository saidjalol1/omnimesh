//! Warehouse robot fleet simulation
//!
//! Simulates a fleet of warehouse robots coordinating tasks using OMNI-MESH.
//!
//! Scenario:
//! - 1 Coordinator node assigns tasks
//! - 3 Robot nodes execute tasks and report status
//! - Demonstrates offline-first messaging and task coordination
//!
//! Usage:
//!   cargo run --example warehouse_fleet

use omnimesh::buffer::PayloadStorage;
use omnimesh::config::OmnimeshMode;
use omnimesh::envelope::{Did, EnvelopeHeader, MessageId, SignedEnvelope, Priority, PayloadType};
use omnimesh::runtime::delivery::DeliveryLayer;
use omnimesh::runtime::security::SecurityLayer;
use omnimesh::payload;
use ed25519_dalek::SigningKey;
use rand_core::OsRng;
use std::thread;
use std::time::Duration;

struct Robot {
    id: u8,
    did: Did,
    signing_key: SigningKey,
    security: SecurityLayer,
    delivery: DeliveryLayer,
}

impl Robot {
    fn new(id: u8, mode: &OmnimeshMode) -> Self {
        let signing_key = SigningKey::generate(&mut OsRng);
        let did = Did::new(signing_key.verifying_key().to_bytes());
        
        Robot {
            id,
            did,
            signing_key,
            security: SecurityLayer::new(mode, None),
            delivery: DeliveryLayer::new(mode),
        }
    }
    
    fn create_task_command(&self, task: &str, target_location: &str, seq: u64, coordinator_key: &SigningKey) -> SignedEnvelope<1024> {
        let coordinator_did = Did::new(coordinator_key.verifying_key().to_bytes());
        
        let header = EnvelopeHeader {
            version: 7,
            message_id: MessageId([self.id, seq as u8, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]),
            sender_did: coordinator_did,
            recipient_did: self.did,
            sequence_number: seq,
            timestamp_us: 1234567890 + seq,
            priority: Priority::High,
            payload_type: PayloadType::Raw,
        };
        
        let task_bytes = format!("{}:{}", task, target_location);
        let agent_cmd = payload::agent_command(task, &self.did.0, task_bytes.as_bytes());
        let encoded = payload::encode_payload(&agent_cmd);
        
        let mut payload_buf = PayloadStorage::<1024>::new();
        payload_buf.push_bytes(&encoded).unwrap();
        
        SignedEnvelope::sign(header, payload_buf, coordinator_key)
    }
    
    fn create_status_report(&self, _status: &str, seq: u64) -> SignedEnvelope<1024> {
        let coordinator_did = Did([0xCC; 32]);
        
        let header = EnvelopeHeader {
            version: 7,
            message_id: MessageId([self.id, seq as u8, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]),
            sender_did: self.did,
            recipient_did: coordinator_did,
            sequence_number: seq,
            timestamp_us: 1234567890 + seq + 1000,
            priority: Priority::Normal,
            payload_type: PayloadType::Raw,
        };
        
        let heartbeat = payload::heartbeat(&self.did.0, 60000, 100, 4096, seq);
        let encoded = payload::encode_payload(&heartbeat);
        
        let mut payload_buf = PayloadStorage::<1024>::new();
        payload_buf.push_bytes(&encoded).unwrap();
        
        SignedEnvelope::sign(header, payload_buf, &self.signing_key)
    }
    
    fn process_task(&self, envelope: &SignedEnvelope<1024>) -> Result<String, String> {
        // Verify signature
        self.security.verify(envelope)?;
        
        // Check for duplicates
        self.delivery.deliver(envelope)?;
        
        // Decode payload
        let decoded = payload::decode_payload(envelope.payload.as_slice())
            .map_err(|e| format!("Failed to decode: {:?}", e))?;
        
        match decoded.payload {
            Some(payload::PayloadKind::AgentCommand(cmd)) => {
                let task_str = String::from_utf8_lossy(&cmd.payload);
                Ok(format!("Executing: {}", task_str))
            }
            _ => Err("Unexpected payload type".to_string()),
        }
    }
}

fn main() {
    println!("=== OMNI-MESH Warehouse Fleet Simulation ===\n");
    
    let mode = OmnimeshMode::production();
    
    // Create coordinator
    let coordinator_key = SigningKey::generate(&mut OsRng);
    let coordinator_did = Did::new(coordinator_key.verifying_key().to_bytes());
    println!("Coordinator DID: {:?}\n", hex::encode(&coordinator_did.0[..8]));
    
    // Create robot fleet
    let mut robots = vec![
        Robot::new(1, &mode),
        Robot::new(2, &mode),
        Robot::new(3, &mode),
    ];
    
    println!("Fleet initialized:");
    for robot in &robots {
        println!("  Robot {} - DID: {:?}", robot.id, hex::encode(&robot.did.0[..8]));
    }
    println!();
    
    // Coordinator assigns tasks
    println!("=== Task Assignment Phase ===\n");
    
    let tasks = vec![
        ("pick", "A-12"),
        ("transport", "B-05"),
        ("place", "C-18"),
    ];
    
    let mut task_envelopes = Vec::new();
    
    for (i, robot) in robots.iter().enumerate() {
        let (task, location) = tasks[i];
        println!("[Coordinator] Assigning task to Robot {}: {} at {}", 
            robot.id, task, location);
        
        let envelope = robot.create_task_command(task, location, i as u64, &coordinator_key);
        task_envelopes.push(envelope);
    }
    
    println!();
    
    // Simulate message delivery delay
    thread::sleep(Duration::from_millis(50));
    
    // Robots process tasks
    println!("=== Task Execution Phase ===\n");
    
    for (i, robot) in robots.iter_mut().enumerate() {
        let envelope = &task_envelopes[i];
        
        match robot.process_task(envelope) {
            Ok(msg) => {
                println!("[Robot {}] ✓ {}", robot.id, msg);
                
                // Simulate task execution time
                thread::sleep(Duration::from_millis(100));
                
                // Send status report
                let status_envelope = robot.create_status_report("completed", i as u64 + 100);
                println!("[Robot {}] Sending status report", robot.id);
                
                // In real scenario, this would be sent via transport
                // For simulation, we just verify it can be created
                assert!(robot.security.verify(&status_envelope).is_ok());
            }
            Err(e) => {
                println!("[Robot {}] ✗ Error: {}", robot.id, e);
            }
        }
    }
    
    println!();
    
    // Summary
    println!("=== Simulation Complete ===\n");
    println!("This example demonstrated:");
    println!("  ✓ Multi-node coordination");
    println!("  ✓ Task assignment and execution");
    println!("  ✓ AgentCommand payload type");
    println!("  ✓ Heartbeat status reports");
    println!("  ✓ Production mode (persistent deduplication)");
    println!("  ✓ Signature verification for all messages");
    println!("\nIn a real deployment:");
    println!("  • Robots would run on separate hardware");
    println!("  • Messages would be sent via TCP/QUIC transport");
    println!("  • DTN store would handle offline scenarios");
    println!("  • Prometheus metrics would track fleet health");
}
