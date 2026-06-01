//! Production edge-case tests for OMNI-MESH
//!
//! These tests verify behavior under adversarial, boundary, and failure conditions
//! that are critical for production deployments.

use omnimesh::client::{OmnimeshClient, ClientConfig};
use omnimesh::payload::{self, PayloadKind, EnvelopePayload};
use omnimesh::envelope::Did;
use omnimesh::buffer::PayloadStorage;
use omnimesh::config::OmnimeshMode;
use ed25519_dalek::SigningKey;
use rand_core::OsRng;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

// ═══════════════════════════════════════════════════════════════════════════════
// GRACEFUL SHUTDOWN TESTS
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_client_shutdown_stops_receiving() {
    let client = OmnimeshClient::builder()
        .with_config(ClientConfig::development())
        .build()
        .unwrap();

    // Shutdown the client
    client.shutdown();
    assert!(client.is_shutdown());

    // receive_timeout should return None immediately after shutdown
    let start = std::time::Instant::now();
    let result = client.receive_timeout(Duration::from_secs(5));
    let elapsed = start.elapsed();

    assert!(result.is_none());
    // Should return almost immediately, not wait 5 seconds
    assert!(elapsed < Duration::from_millis(100),
        "receive_timeout took {:?} after shutdown, expected < 100ms", elapsed);
}

#[test]
fn test_client_shutdown_rejects_sends() {
    let client = OmnimeshClient::builder()
        .with_config(ClientConfig::development())
        .build()
        .unwrap();

    let target = Did([0xAA; 32]);
    client.shutdown();

    let result = client.send(target, payload::agent_command("test", b"", b""));
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("shut down"));
}

#[test]
fn test_client_drain_after_shutdown() {
    let node_a = OmnimeshClient::builder()
        .with_config(ClientConfig::development())
        .build()
        .unwrap();
    let node_b = OmnimeshClient::builder()
        .with_config(ClientConfig::development())
        .build()
        .unwrap();

    // Send messages before shutdown
    for i in 0..5 {
        let msg = payload::agent_command(&format!("cmd-{}", i), b"", b"");
        node_a.send(node_b.did, msg).unwrap();
    }

    // Wait long enough for all messages to be processed by the poller
    thread::sleep(Duration::from_millis(200));

    // Shutdown node_b
    node_b.shutdown();

    // Should still be able to drain existing messages with try_receive
    let mut drained = 0;
    while node_b.try_receive().is_some() {
        drained += 1;
    }
    assert_eq!(drained, 5, "Should drain all 5 messages after shutdown");
}

// ═══════════════════════════════════════════════════════════════════════════════
// BACK-PRESSURE AND OVERFLOW TESTS
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_inbox_backpressure_drops_messages() {
    // Create a client with a tiny inbox
    let config = ClientConfig {
        mode: OmnimeshMode::development(),
        receive_buffer_capacity: 5,
    };
    let receiver = OmnimeshClient::builder()
        .with_config(config)
        .build()
        .unwrap();

    let sender = OmnimeshClient::builder()
        .with_config(ClientConfig::development())
        .build()
        .unwrap();

    // Send more messages than the inbox can hold
    for i in 0..20 {
        let msg = payload::agent_command(&format!("flood-{}", i), b"", b"");
        sender.send(receiver.did, msg).unwrap();
    }

    // Wait for poller to process
    thread::sleep(Duration::from_millis(100));

    // Should have at most 5 messages (capacity limit)
    assert!(receiver.inbox_len() <= 5,
        "Inbox should respect capacity limit, got {}", receiver.inbox_len());

    // Health check should show drops
    let health = receiver.health();
    assert!(health.messages_dropped > 0,
        "Should report dropped messages, got {}", health.messages_dropped);
}

#[test]
fn test_health_check_reports_correctly() {
    let client = OmnimeshClient::builder()
        .with_config(ClientConfig::development())
        .build()
        .unwrap();

    let health = client.health();
    assert!(health.is_healthy());
    assert_eq!(health.inbox_len, 0);
    assert_eq!(health.messages_sent, 0);
    assert_eq!(health.messages_received, 0);
    assert_eq!(health.messages_dropped, 0);
    assert!(!health.is_shutdown);
    assert!(health.inbox_utilization() < 0.01);
}

#[test]
fn test_health_after_activity() {
    let node_a = OmnimeshClient::builder()
        .with_config(ClientConfig::development())
        .build()
        .unwrap();
    let node_b = OmnimeshClient::builder()
        .with_config(ClientConfig::development())
        .build()
        .unwrap();

    // Send some messages
    for _ in 0..3 {
        node_a.send(node_b.did, payload::heartbeat(&[0; 32], 1000, 50, 2048, 1)).unwrap();
    }

    // Wait for poller to process all messages — poll until received
    let deadline = std::time::Instant::now() + Duration::from_secs(5);
    while node_b.health().messages_received < 3 && std::time::Instant::now() < deadline {
        thread::sleep(Duration::from_millis(10));
    }

    let health_a = node_a.health();
    assert_eq!(health_a.messages_sent, 3);

    let health_b = node_b.health();
    assert_eq!(health_b.messages_received, 3);
}

// ═══════════════════════════════════════════════════════════════════════════════
// PAYLOAD BOUNDARY TESTS
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_empty_payload_rejected() {
    let client = OmnimeshClient::builder()
        .with_config(ClientConfig::development())
        .build()
        .unwrap();

    let target = Did([0xBB; 32]);
    // An EnvelopePayload with None payload encodes to empty bytes
    let empty = EnvelopePayload { payload: None };
    let result = client.send(target, empty);
    // Empty payload should be rejected
    assert!(result.is_err(), "Empty payload should be rejected");
}

#[test]
fn test_maximum_payload_size() {
    let client = OmnimeshClient::builder()
        .with_config(ClientConfig::development())
        .build()
        .unwrap();

    let target = Did([0xCC; 32]);

    // Create a payload that's just under the limit
    let large_data = vec![0xAB; 900]; // Should fit in DEFAULT_PAYLOAD_CAPACITY
    let msg = payload::agent_command("large", &[0; 32], &large_data);
    let result = client.send(target, msg);
    assert!(result.is_ok(), "Payload under limit should succeed: {:?}", result.err());
}

#[test]
fn test_payload_with_special_characters() {
    let node_a = OmnimeshClient::builder()
        .with_config(ClientConfig::development())
        .build()
        .unwrap();
    let node_b = OmnimeshClient::builder()
        .with_config(ClientConfig::development())
        .build()
        .unwrap();

    // Test with unicode, null bytes, and special characters
    let special = "Hello 🤖 \x00\x01\x02 世界 \n\r\t";
    let msg = payload::agent_command(special, b"\x00\xFF", b"\x00\x01\x02\x03");
    node_a.send(node_b.did, msg).unwrap();

    thread::sleep(Duration::from_millis(50));

    let received = node_b.try_receive().expect("Should receive message");
    if let Some(PayloadKind::AgentCommand(cmd)) = received.payload.payload {
        assert_eq!(cmd.command_type, special);
        assert_eq!(cmd.target_did, b"\x00\xFF");
        assert_eq!(cmd.payload, b"\x00\x01\x02\x03");
    } else {
        panic!("Expected AgentCommand");
    }
}

#[test]
fn test_all_payload_types_roundtrip() {
    let node_a = OmnimeshClient::builder()
        .with_config(ClientConfig::development())
        .build()
        .unwrap();
    let node_b = OmnimeshClient::builder()
        .with_config(ClientConfig::development())
        .build()
        .unwrap();

    // Test each payload type
    let payloads = vec![
        payload::agent_command("test", b"target", b"data"),
        payload::motion_command(1.0, 2.0, 3.0, 0.1, 0.2, 0.3, 100_000),
        payload::heartbeat(&[0xAA; 32], 5000, 75, 4096, 42),
        payload::llm_query("What is Rust?", "You are helpful", "llama3"),
        payload::llm_response("Rust is a systems language", 1500),
        payload::model_weights("resnet50", &[1, 2, 3, 4], "zstd", 1024),
    ];

    for p in &payloads {
        node_a.send(node_b.did, p.clone()).unwrap();
    }

    thread::sleep(Duration::from_millis(100));

    let mut received_count = 0;
    while node_b.try_receive().is_some() {
        received_count += 1;
    }
    assert_eq!(received_count, payloads.len(),
        "All payload types should roundtrip successfully");
}

// ═══════════════════════════════════════════════════════════════════════════════
// CONCURRENCY AND THREAD SAFETY TESTS
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_concurrent_sends_from_multiple_threads() {
    let sender = Arc::new(
        OmnimeshClient::builder()
            .with_config(ClientConfig::development())
            .build()
            .unwrap()
    );
    let receiver = OmnimeshClient::builder()
        .with_config(ClientConfig::development())
        .build()
        .unwrap();

    let receiver_did = receiver.did;
    let mut handles = vec![];

    // 8 threads each sending 50 messages
    for thread_id in 0..8 {
        let s = sender.clone();
        let handle = thread::spawn(move || {
            for i in 0..50 {
                let msg = payload::agent_command(
                    &format!("t{}-m{}", thread_id, i),
                    b"",
                    b"concurrent",
                );
                s.send(receiver_did, msg).unwrap();
            }
        });
        handles.push(handle);
    }

    for h in handles {
        h.join().unwrap();
    }

    // Poll until all 400 messages arrive or timeout after 10 seconds
    let deadline = std::time::Instant::now() + Duration::from_secs(10);
    let mut total = 0;
    while total < 400 && std::time::Instant::now() < deadline {
        if receiver.try_receive().is_some() {
            total += 1;
        } else {
            thread::sleep(Duration::from_millis(1));
        }
    }
    assert_eq!(total, 400, "All concurrent messages should be delivered, got {}", total);
}

#[test]
fn test_concurrent_send_and_receive() {
    let node_a = Arc::new(
        OmnimeshClient::builder()
            .with_config(ClientConfig::development())
            .build()
            .unwrap()
    );
    let node_b = Arc::new(
        OmnimeshClient::builder()
            .with_config(ClientConfig::development())
            .build()
            .unwrap()
    );

    let a_did = node_a.did;
    let b_did = node_b.did;

    // Thread 1: A sends to B
    let a_clone = node_a.clone();
    let send_handle = thread::spawn(move || {
        for i in 0..100 {
            let msg = payload::heartbeat(&[0; 32], i, 50, 1024, i);
            a_clone.send(b_did, msg).unwrap();
            thread::sleep(Duration::from_micros(100));
        }
    });

    // Thread 2: B receives and sends back to A
    let b_clone = node_b.clone();
    let recv_handle = thread::spawn(move || {
        let mut received = 0;
        let deadline = std::time::Instant::now() + Duration::from_secs(5);
        while received < 100 && std::time::Instant::now() < deadline {
            if let Some(_msg) = b_clone.receive_timeout(Duration::from_millis(100)) {
                received += 1;
                // Echo back
                let echo = payload::agent_command("echo", b"", b"");
                b_clone.send(a_did, echo).unwrap();
            }
        }
        received
    });

    send_handle.join().unwrap();
    let received = recv_handle.join().unwrap();
    assert_eq!(received, 100, "B should receive all 100 messages, got {}", received);

    // A should have received echoes
    thread::sleep(Duration::from_millis(100));
    let mut echoes = 0;
    while node_a.try_receive().is_some() {
        echoes += 1;
    }
    assert_eq!(echoes, 100, "A should receive 100 echoes, got {}", echoes);
}

#[test]
fn test_many_clients_mesh() {
    // Create 10 clients and have each send to every other
    let clients: Vec<_> = (0..10)
        .map(|_| {
            OmnimeshClient::builder()
                .with_config(ClientConfig::development())
                .build()
                .unwrap()
        })
        .collect();

    let dids: Vec<Did> = clients.iter().map(|c| c.did).collect();

    // Each client sends one message to every other client
    for (i, client) in clients.iter().enumerate() {
        for (j, &target_did) in dids.iter().enumerate() {
            if i != j {
                let msg = payload::agent_command(
                    &format!("from-{}-to-{}", i, j),
                    b"",
                    b"mesh",
                );
                client.send(target_did, msg).unwrap();
            }
        }
    }

    thread::sleep(Duration::from_millis(200));

    // Each client should receive exactly 9 messages (one from each other)
    for (i, client) in clients.iter().enumerate() {
        let mut count = 0;
        while client.try_receive().is_some() {
            count += 1;
        }
        assert_eq!(count, 9,
            "Client {} should receive 9 messages, got {}", i, count);
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// CRYPTOGRAPHIC INTEGRITY TESTS
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_signature_verification_rejects_tampered_envelope() {
    use omnimesh::envelope::{EnvelopeHeader, MessageId, SignedEnvelope, Priority, PayloadType};
    use omnimesh::runtime::security::SecurityLayer;

    let mode = OmnimeshMode::production();
    let security = SecurityLayer::new(&mode, None);
    let signing_key = SigningKey::generate(&mut OsRng);
    let sender_did = Did::new(signing_key.verifying_key().to_bytes());

    let header = EnvelopeHeader {
        version: 7,
        message_id: MessageId([0x01; 16]),
        sender_did,
        recipient_did: Did([0xBB; 32]),
        sequence_number: 1,
        timestamp_us: 1_700_000_000_000_000,
        priority: Priority::Normal,
        payload_type: PayloadType::Raw,
    };

    let mut payload_buf = PayloadStorage::<1024>::new();
    payload_buf.push_bytes(b"original data").unwrap();

    let mut envelope = SignedEnvelope::sign(header, payload_buf, &signing_key);

    // Tamper with the payload
    let mut tampered_payload = PayloadStorage::<1024>::new();
    tampered_payload.push_bytes(b"TAMPERED data!").unwrap();
    envelope.payload = tampered_payload;

    // Verification should fail
    let result = security.verify(&envelope);
    assert!(result.is_err(), "Tampered envelope should fail verification");
}

#[test]
fn test_signature_verification_rejects_wrong_key() {
    use omnimesh::envelope::{EnvelopeHeader, MessageId, SignedEnvelope, Priority, PayloadType};
    use omnimesh::runtime::security::SecurityLayer;

    let mode = OmnimeshMode::production();
    let security = SecurityLayer::new(&mode, None);

    let real_key = SigningKey::generate(&mut OsRng);
    let fake_key = SigningKey::generate(&mut OsRng);

    // Sign with real_key but claim sender is fake_key's DID
    let header = EnvelopeHeader {
        version: 7,
        message_id: MessageId([0x02; 16]),
        sender_did: Did::new(fake_key.verifying_key().to_bytes()), // Wrong DID!
        recipient_did: Did([0xCC; 32]),
        sequence_number: 1,
        timestamp_us: 1_700_000_000_000_000,
        priority: Priority::Normal,
        payload_type: PayloadType::Raw,
    };

    let mut payload_buf = PayloadStorage::<1024>::new();
    payload_buf.push_bytes(b"signed by wrong key").unwrap();

    let envelope = SignedEnvelope::sign(header, payload_buf, &real_key);

    // Verification should fail because sender_did doesn't match signing key
    let result = security.verify(&envelope);
    assert!(result.is_err(), "Wrong-key envelope should fail verification");
}

#[test]
fn test_unsigned_envelope_rejected_in_production() {
    use omnimesh::envelope::{EnvelopeHeader, MessageId, SignedEnvelope, Priority, PayloadType};
    use omnimesh::runtime::security::SecurityLayer;

    let mode = OmnimeshMode::production();
    let security = SecurityLayer::new(&mode, None);

    let header = EnvelopeHeader {
        version: 7,
        message_id: MessageId([0x03; 16]),
        sender_did: Did([0xAA; 32]),
        recipient_did: Did([0xBB; 32]),
        sequence_number: 1,
        timestamp_us: 1_700_000_000_000_000,
        priority: Priority::Normal,
        payload_type: PayloadType::Raw,
    };

    let payload_buf = PayloadStorage::<1024>::new();
    // Zero signature = unsigned
    let envelope = SignedEnvelope::new(header, payload_buf, [0u8; 64]);

    let result = security.verify(&envelope);
    assert!(result.is_err(), "Unsigned envelope should be rejected in production mode");
}

// ═══════════════════════════════════════════════════════════════════════════════
// DELIVERY ORDERING AND DEDUPLICATION EDGE CASES
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_out_of_order_delivery_reordering() {
    use omnimesh::envelope::{EnvelopeHeader, MessageId, SignedEnvelope, Priority, PayloadType};
    use omnimesh::runtime::delivery::{DeliveryLayer, DeliveryStatus};

    let mode = OmnimeshMode::development();
    let delivery = DeliveryLayer::new(&mode);
    let signing_key = SigningKey::generate(&mut OsRng);
    let sender_did = Did::new(signing_key.verifying_key().to_bytes());

    // Send sequence 2 before sequence 1 (out of order)
    let make_env = |seq: u64| -> SignedEnvelope<128> {
        let header = EnvelopeHeader {
            version: 7,
            message_id: MessageId([seq as u8; 16]),
            sender_did,
            recipient_did: Did([0xBB; 32]),
            sequence_number: seq,
            timestamp_us: 1_700_000_000_000_000 + seq,
            priority: Priority::Normal,
            payload_type: PayloadType::Raw,
        };
        let payload = PayloadStorage::<128>::new();
        SignedEnvelope::sign(header, payload, &signing_key)
    };

    // Deliver seq 0 first (expected)
    let status = delivery.deliver(&make_env(0)).unwrap();
    assert_eq!(status, DeliveryStatus::Delivered);

    // Deliver seq 2 (out of order, should be buffered)
    let status = delivery.deliver(&make_env(2)).unwrap();
    assert_eq!(status, DeliveryStatus::Buffered(2));

    // Deliver seq 1 (fills the gap, should deliver and flush buffered seq 2)
    let status = delivery.deliver(&make_env(1)).unwrap();
    assert_eq!(status, DeliveryStatus::Delivered);
}

#[test]
fn test_duplicate_detection() {
    use omnimesh::envelope::{EnvelopeHeader, MessageId, SignedEnvelope, Priority, PayloadType};
    use omnimesh::runtime::delivery::{DeliveryLayer, DeliveryStatus};

    let mode = OmnimeshMode::development();
    let delivery = DeliveryLayer::new(&mode);
    let signing_key = SigningKey::generate(&mut OsRng);
    let sender_did = Did::new(signing_key.verifying_key().to_bytes());

    let header = EnvelopeHeader {
        version: 7,
        message_id: MessageId([0x42; 16]),
        sender_did,
        recipient_did: Did([0xBB; 32]),
        sequence_number: 0,
        timestamp_us: 1_700_000_000_000_000,
        priority: Priority::Normal,
        payload_type: PayloadType::Raw,
    };
    let payload = PayloadStorage::<128>::new();
    let envelope = SignedEnvelope::sign(header, payload, &signing_key);

    // First delivery
    let status = delivery.deliver(&envelope).unwrap();
    assert_eq!(status, DeliveryStatus::Delivered);

    // Second delivery of same message
    let status = delivery.deliver(&envelope).unwrap();
    assert_eq!(status, DeliveryStatus::Duplicate);

    // Third delivery
    let status = delivery.deliver(&envelope).unwrap();
    assert_eq!(status, DeliveryStatus::Duplicate);
}

#[test]
fn test_stale_message_rejected() {
    use omnimesh::envelope::{EnvelopeHeader, MessageId, SignedEnvelope, Priority, PayloadType};
    use omnimesh::runtime::delivery::{DeliveryLayer, DeliveryStatus};

    let mode = OmnimeshMode::development();
    let delivery = DeliveryLayer::new(&mode);
    let signing_key = SigningKey::generate(&mut OsRng);
    let sender_did = Did::new(signing_key.verifying_key().to_bytes());

    let make_env = |seq: u64, unique_id: u8| -> SignedEnvelope<128> {
        // Each call gets a truly unique message_id
        let mut msg_id = [0u8; 16];
        msg_id[0] = unique_id;
        msg_id[1..9].copy_from_slice(&seq.to_le_bytes());
        let header = EnvelopeHeader {
            version: 7,
            message_id: MessageId(msg_id),
            sender_did,
            recipient_did: Did([0xBB; 32]),
            sequence_number: seq,
            timestamp_us: 1_700_000_000_000_000 + seq,
            priority: Priority::Normal,
            payload_type: PayloadType::Raw,
        };
        let payload = PayloadStorage::<128>::new();
        SignedEnvelope::sign(header, payload, &signing_key)
    };

    // Deliver seq 0, 1, 2 with unique message IDs
    delivery.deliver(&make_env(0, 10)).unwrap();
    delivery.deliver(&make_env(1, 11)).unwrap();
    delivery.deliver(&make_env(2, 12)).unwrap();

    // Try to deliver a NEW message (unique msg_id=13) but with seq 0 (stale)
    let status = delivery.deliver(&make_env(0, 13)).unwrap();
    // Should be Stale because seq 0 < expected (3)
    assert_eq!(status, DeliveryStatus::Stale);
}

// ═══════════════════════════════════════════════════════════════════════════════
// BUFFER AND MEMORY SAFETY TESTS
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_payload_storage_overflow_protection() {
    let mut storage = PayloadStorage::<64>::new();

    // Should succeed
    let result = storage.push_bytes(&[0xAA; 60]);
    assert!(result.is_ok());

    // Should fail — would overflow
    let result = storage.push_bytes(&[0xBB; 10]);
    assert!(result.is_err(), "Should reject overflow");
}

#[test]
fn test_payload_storage_empty_operations() {
    let storage = PayloadStorage::<128>::new();
    assert_eq!(storage.len(), 0);
    assert_eq!(storage.as_slice(), &[] as &[u8]);
}

#[test]
fn test_payload_storage_exact_capacity() {
    let mut storage = PayloadStorage::<32>::new();
    let result = storage.push_bytes(&[0xFF; 32]);
    assert!(result.is_ok(), "Should accept exactly capacity bytes");
    assert_eq!(storage.len(), 32);
}

// ═══════════════════════════════════════════════════════════════════════════════
// SERIALIZATION ROUNDTRIP TESTS
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_envelope_serialize_deserialize_roundtrip() {
    use omnimesh::envelope::{EnvelopeHeader, MessageId, SignedEnvelope, Priority, PayloadType};

    let signing_key = SigningKey::generate(&mut OsRng);
    let sender_did = Did::new(signing_key.verifying_key().to_bytes());

    let header = EnvelopeHeader {
        version: 7,
        message_id: MessageId([0xDE; 16]),
        sender_did,
        recipient_did: Did([0xAD; 32]),
        sequence_number: 999,
        timestamp_us: 1_700_000_000_000_000,
        priority: Priority::High,
        payload_type: PayloadType::Raw,
    };

    let mut payload_buf = PayloadStorage::<256>::new();
    payload_buf.push_bytes(b"roundtrip test data").unwrap();

    let original = SignedEnvelope::sign(header, payload_buf, &signing_key);

    // Serialize
    let mut buf = [0u8; 2048];
    let len = original.serialize_into(&mut buf).unwrap();

    // Deserialize
    let restored: SignedEnvelope<256> = SignedEnvelope::deserialize(&buf[..len]).unwrap();

    assert_eq!(original.header.version, restored.header.version);
    assert_eq!(original.header.message_id.0, restored.header.message_id.0);
    assert_eq!(original.header.sender_did.0, restored.header.sender_did.0);
    assert_eq!(original.header.recipient_did.0, restored.header.recipient_did.0);
    assert_eq!(original.header.sequence_number, restored.header.sequence_number);
    assert_eq!(original.payload.as_slice(), restored.payload.as_slice());
    assert_eq!(original.signature, restored.signature);

    // Verify signature still valid after roundtrip
    let verifying_key = signing_key.verifying_key();
    assert!(restored.verify_signature(&verifying_key).is_ok());
}

// ═══════════════════════════════════════════════════════════════════════════════
// TIMING AND PERFORMANCE EDGE CASES
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_receive_timeout_accuracy() {
    let client = OmnimeshClient::builder()
        .with_config(ClientConfig::development())
        .build()
        .unwrap();

    // Test that receive_timeout returns in approximately the right time
    let start = std::time::Instant::now();
    let result = client.receive_timeout(Duration::from_millis(100));
    let elapsed = start.elapsed();

    assert!(result.is_none());
    // Should be within 50ms of the target (generous for CI)
    assert!(elapsed >= Duration::from_millis(90),
        "Timeout too short: {:?}", elapsed);
    assert!(elapsed < Duration::from_millis(200),
        "Timeout too long: {:?}", elapsed);
}

#[test]
fn test_rapid_send_receive_latency() {
    let node_a = OmnimeshClient::builder()
        .with_config(ClientConfig::development())
        .build()
        .unwrap();
    let node_b = OmnimeshClient::builder()
        .with_config(ClientConfig::development())
        .build()
        .unwrap();

    let start = std::time::Instant::now();
    let msg = payload::agent_command("latency-test", b"", b"");
    node_a.send(node_b.did, msg).unwrap();

    let received = node_b.receive_timeout(Duration::from_secs(1));
    let latency = start.elapsed();

    assert!(received.is_some(), "Message should arrive");
    // In-process mock transport should be sub-10ms
    assert!(latency < Duration::from_millis(50),
        "Latency too high for mock transport: {:?}", latency);
}

// ═══════════════════════════════════════════════════════════════════════════════
// IDENTITY AND DID EDGE CASES
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_send_to_self() {
    let client = OmnimeshClient::builder()
        .with_config(ClientConfig::development())
        .build()
        .unwrap();

    // Send a message to yourself
    let msg = payload::agent_command("self-msg", b"", b"hello self");
    client.send(client.did, msg).unwrap();

    thread::sleep(Duration::from_millis(50));

    let received = client.try_receive();
    assert!(received.is_some(), "Should be able to send to self");
    if let Some(PayloadKind::AgentCommand(cmd)) = received.unwrap().payload.payload {
        assert_eq!(cmd.command_type, "self-msg");
    }
}

#[test]
fn test_unique_dids_per_client() {
    let clients: Vec<_> = (0..100)
        .map(|_| {
            OmnimeshClient::builder()
                .with_config(ClientConfig::development())
                .build()
                .unwrap()
        })
        .collect();

    // All DIDs should be unique
    let mut dids: Vec<[u8; 32]> = clients.iter().map(|c| c.did.0).collect();
    dids.sort();
    dids.dedup();
    assert_eq!(dids.len(), 100, "All 100 clients should have unique DIDs");
}

#[test]
fn test_deterministic_did_from_key() {
    let key = SigningKey::generate(&mut OsRng);
    let client1 = OmnimeshClient::builder()
        .with_config(ClientConfig::development())
        .with_signing_key(key.clone())
        .build()
        .unwrap();
    let client2 = OmnimeshClient::builder()
        .with_config(ClientConfig::development())
        .with_signing_key(key)
        .build()
        .unwrap();

    assert_eq!(client1.did.0, client2.did.0,
        "Same signing key should produce same DID");
}
