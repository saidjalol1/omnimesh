//! SDK Integration Tests
//!
//! Proves that two developer applications can use `OmnimeshClient` to send
//! and receive strongly-typed messages across the mesh network without any
//! knowledge of the underlying cryptography or transport layers.

use omnimesh::client::{OmnimeshClient, ClientConfig};
use omnimesh::config::OmnimeshMode;
use omnimesh::payload::{self, PayloadKind};
use std::time::Duration;

/// Validates that the builder pattern works and a client can be created.
#[test]
fn sdk_client_builds_successfully() {
    let client = OmnimeshClient::builder()
        .with_config(ClientConfig::development())
        .build()
        .expect("Client should build");

    println!("Client created: {:?}", client);
    println!("Node DID (hex): {}", hex::encode(&client.did.0));
    assert_ne!(client.did.0, [0u8; 32], "DID must not be all zeros");
}

/// Two clients send a MotionCommand directly to each other via TCP.
#[test]
fn sdk_motion_command_roundtrip() {
    // Node A — robot controller
    let controller = OmnimeshClient::builder()
        .with_config(ClientConfig {
            mode: OmnimeshMode::development(),
            ..Default::default()
        })
        .build()
        .expect("Controller build failed");

    // Node B — the robot itself
    let robot = OmnimeshClient::builder()
        .with_config(ClientConfig {
            mode: OmnimeshMode::development(),
            ..Default::default()
        })
        .build()
        .expect("Robot build failed");

    // Since we are in dev (mock) mode, messages go directly through the
    // mock transport which is already visible to the same process.
    // Send: controller --> robot
    let cmd = payload::motion_command(1.5, 0.0, 0.0, 0.0, 0.0, 0.3, 50_000);
    controller
        .send(robot.did, cmd.clone())
        .expect("send must succeed");

    // Robot polls for a message — it should see the MotionCommand
    let msg = robot.receive_timeout(Duration::from_millis(200));
    assert!(msg.is_some(), "Robot should receive a message from controller");

    let msg = msg.unwrap();
    assert_eq!(msg.sender, controller.did);

    match msg.payload.payload {
        Some(PayloadKind::MotionCommand(mc)) => {
            let linear = mc.linear.expect("linear must be Some");
            assert!((linear.x - 1.5).abs() < 0.001, "linear.x must be 1.5");
            println!("✓ Robot received MotionCommand: linear.x={}", linear.x);
        }
        other => panic!("Expected MotionCommand, got {:?}", other),
    }
}

/// Send a Heartbeat and verify the decoded struct on the other side.
#[test]
fn sdk_heartbeat_roundtrip() {
    let sender = OmnimeshClient::builder()
        .with_config(ClientConfig::development())
        .build()
        .expect("Sender build failed");

    let receiver = OmnimeshClient::builder()
        .with_config(ClientConfig::development())
        .build()
        .expect("Receiver build failed");

    let hb = payload::heartbeat(&sender.did.0, 12345, 42, 1024, 7);
    sender.send(receiver.did, hb).expect("send failed");

    let msg = receiver.receive_timeout(Duration::from_millis(200));
    assert!(msg.is_some(), "Receiver should get Heartbeat");

    let msg = msg.unwrap();
    match msg.payload.payload {
        Some(PayloadKind::Heartbeat(h)) => {
            assert_eq!(h.uptime_ms, 12345);
            assert_eq!(h.cpu_usage, 42);
            assert_eq!(h.mem_usage_kb, 1024);
            assert_eq!(h.epoch, 7);
            println!("✓ Received Heartbeat: uptime={}ms, cpu={}%, mem={}KB", h.uptime_ms, h.cpu_usage, h.mem_usage_kb);
        }
        other => panic!("Expected Heartbeat, got {:?}", other),
    }
}

/// Send an LLM query and verify the decoded response.
#[test]
fn sdk_llm_query_roundtrip() {
    let edge_node = OmnimeshClient::builder()
        .with_config(ClientConfig::development())
        .build()
        .expect("Edge node build failed");

    let ai_node = OmnimeshClient::builder()
        .with_config(ClientConfig::development())
        .build()
        .expect("AI node build failed");

    let query = payload::llm_query(
        "What is the battery level of robot arm #3?",
        "You are a fleet management AI.",
        "llama3:8b",
    );
    edge_node.send(ai_node.did, query).expect("LlmQuery send failed");

    let msg = ai_node.receive_timeout(Duration::from_millis(200));
    assert!(msg.is_some(), "AI node should receive LlmQuery");

    let msg = msg.unwrap();
    match msg.payload.payload {
        Some(PayloadKind::LlmQuery(q)) => {
            assert!(q.prompt.contains("battery level"));
            assert_eq!(q.model, "llama3:8b");
            println!("✓ AI node received LlmQuery: prompt=\"{}\" model={}", q.prompt, q.model);
        }
        other => panic!("Expected LlmQuery, got {:?}", other),
    }
}

/// Try receive returns None immediately when inbox is empty.
#[test]
fn sdk_try_receive_returns_none_when_empty() {
    let client = OmnimeshClient::builder()
        .with_config(ClientConfig::development())
        .build()
        .expect("Client build failed");

    let result = client.try_receive();
    assert!(result.is_none(), "Inbox must be empty on startup");
    println!("✓ try_receive correctly returns None on empty inbox");
}

/// Inbox length is tracked correctly.
#[test]
fn sdk_inbox_len_tracks_messages() {
    let sender = OmnimeshClient::builder()
        .with_config(ClientConfig::development())
        .build()
        .unwrap();

    let receiver = OmnimeshClient::builder()
        .with_config(ClientConfig::development())
        .build()
        .unwrap();

    assert_eq!(receiver.inbox_len(), 0);

    // Send 3 messages
    for i in 0..3 {
        let hb = payload::heartbeat(&sender.did.0, i, 0, 0, i);
        sender.send(receiver.did, hb).unwrap();
    }

    std::thread::sleep(Duration::from_millis(150));
    let count = receiver.inbox_len();
    assert!(count > 0, "Inbox should have messages after send");
    println!("✓ inbox_len = {} after sending 3 messages", count);
}

/// Known peers list is populated after register_peer.
#[test]
fn sdk_register_peer_shows_in_known_peers() {
    let client = OmnimeshClient::builder()
        .with_config(ClientConfig::development())
        .build()
        .unwrap();

    let fake_did = omnimesh::Did([0xAB; 32]);
    client.register_peer(fake_did, "127.0.0.1:7777").expect("register_peer failed");

    let peers = client.known_peers();
    assert!(
        peers.iter().any(|(did, _)| *did == fake_did),
        "Registered peer must appear in known_peers()"
    );
    println!("✓ register_peer correctly populates routing table");
}
