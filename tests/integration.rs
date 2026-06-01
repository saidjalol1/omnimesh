//! End-to-end integration test: sign → verify → deliver → store → encode payload
//!
//! This test exercises the full OMNI-MESH pipeline in a single flow.

use omnimesh::buffer::PayloadStorage;
use omnimesh::envelope::{Did, EnvelopeHeader, MessageId, SignedEnvelope};
use omnimesh::config::OmnimeshMode;
use omnimesh::runtime::delivery::{DeliveryLayer, DeliveryStatus};
use omnimesh::runtime::security::SecurityLayer;
use omnimesh::runtime::storage::StorageLayer;
use omnimesh::runtime::stats::RuntimeStats;
use omnimesh::runtime::wcet::{WcetGuard, WcetBudget};
use omnimesh::payload;

use ed25519_dalek::SigningKey;
use rand_core::{OsRng, RngCore};

#[test]
fn end_to_end_sign_verify_deliver_store() {
    let stats = RuntimeStats::new();
    let mode = OmnimeshMode::production();
    let budget = WcetBudget::default();

    // --- 1. Generate keypair and create signed envelope ---
    let signing_key = SigningKey::generate(&mut OsRng);
    let sender_did = Did::new(signing_key.verifying_key().to_bytes());
    let recipient_did = Did([0xBB; 32]);

    let mut id_bytes = [0u8; 16];
    OsRng.fill_bytes(&mut id_bytes);

    let header = EnvelopeHeader {
        version: 7,
        message_id: MessageId(id_bytes),
        sender_did,
        recipient_did,
        sequence_number: 0,
        timestamp_us: 1234567890,
        priority: omnimesh::envelope::Priority::High,
        payload_type: omnimesh::envelope::PayloadType::RobotCommand,
    };

    let mut payload_buf = PayloadStorage::<128>::new();
    let motion = payload::motion_command(1.0, 0.0, 0.0, 0.0, 0.0, 0.5, 100_000);
    let encoded = payload::encode_payload(&motion);
    payload_buf.push_bytes(&encoded).unwrap();

    let envelope = SignedEnvelope::sign(header, payload_buf, &signing_key);
    stats.record_received();

    // --- 2. Verify signature (SecurityLayer) ---
    let guard = WcetGuard::start("ed25519_verify", budget.ed25519_verify_us, omnimesh::config::WcetMode::Log);
    let security = SecurityLayer::new(&mode, None);
    let verify_result = security.verify(&envelope);
    let wcet_result = guard.finish();
    
    assert!(verify_result.is_ok(), "Signature verification must pass: {:?}", verify_result);
    assert!(wcet_result.is_ok(), "WCET must not hard-fail in Log mode");

    // --- 3. Deliver (DeliveryLayer — exactly-once) ---
    let delivery = DeliveryLayer::new(&mode);
    let status = delivery.deliver(&envelope).expect("delivery must succeed");
    assert_eq!(status, DeliveryStatus::Delivered);
    stats.record_delivered();

    // Duplicate must be rejected
    let dup_status = delivery.deliver(&envelope).expect("delivery must succeed");
    assert_eq!(dup_status, DeliveryStatus::Duplicate);
    stats.record_duplicate();

    // --- 4. Store (StorageLayer) ---
    let mut storage = StorageLayer::new(&mode);
    
    // Convert to DEFAULT_PAYLOAD_CAPACITY size for storage
    let mut payload_1024 = omnimesh::buffer::PayloadStorage::<1024>::new();
    payload_1024.push_bytes(envelope.payload.as_slice()).unwrap();
    let envelope_1024 = SignedEnvelope::new(envelope.header, payload_1024, envelope.signature);
    
    storage.store(envelope_1024.clone()).expect("storage must succeed");
    assert_eq!(storage.stored_count(), 1);

    // --- 5. Decode the payload back ---
    let stored = storage.last_stored().unwrap();
    let decoded = payload::decode_payload(stored.payload.as_slice()).unwrap();
    match decoded.payload {
        Some(payload::PayloadKind::MotionCommand(cmd)) => {
            assert_eq!(cmd.linear.as_ref().unwrap().x, 1.0);
            assert_eq!(cmd.angular.as_ref().unwrap().z, 0.5);
        }
        _ => panic!("Expected MotionCommand payload"),
    }

    // --- 6. Check stats ---
    let snap = stats.snapshot();
    assert_eq!(snap.total_messages_received, 1);
    assert_eq!(snap.total_messages_delivered, 1);
    assert_eq!(snap.total_duplicates, 1);

    println!("=== End-to-End Integration Test PASSED ===");
    println!("Stats: received={}, delivered={}, duplicates={}", 
        snap.total_messages_received, snap.total_messages_delivered, snap.total_duplicates);
}

#[test]
fn payload_roundtrip_all_types() {
    // AgentCommand
    let cmd = payload::agent_command("ship", &[0xAA; 32], b"order-123");
    let bytes = payload::encode_payload(&cmd);
    let decoded = payload::decode_payload(&bytes).unwrap();
    assert_eq!(cmd, decoded);

    // MotionCommand
    let motion = payload::motion_command(1.0, 2.0, 3.0, 0.1, 0.2, 0.3, 50000);
    let bytes = payload::encode_payload(&motion);
    let decoded = payload::decode_payload(&bytes).unwrap();
    assert_eq!(motion, decoded);

    // Heartbeat
    let hb = payload::heartbeat(&[0xCC; 32], 60000, 45, 2048, 999);
    let bytes = payload::encode_payload(&hb);
    let decoded = payload::decode_payload(&bytes).unwrap();
    assert_eq!(hb, decoded);

    // ModelWeights
    let mw = payload::model_weights("resnet50", &[0xFF; 100], "zstd", 1_000_000);
    let bytes = payload::encode_payload(&mw);
    let decoded = payload::decode_payload(&bytes).unwrap();
    assert_eq!(mw, decoded);

    // SensorFusion
    let sf = payload::sensor_fusion("frame-42", 123456, vec![], None);
    let bytes = payload::encode_payload(&sf);
    let decoded = payload::decode_payload(&bytes).unwrap();
    assert_eq!(sf, decoded);

    // LlmQuery
    let lq = payload::llm_query("What is 2+2?", "You are a helpful AI.", "llama3:8b");
    let bytes = payload::encode_payload(&lq);
    let decoded = payload::decode_payload(&bytes).unwrap();
    assert_eq!(lq, decoded);

    // LlmResponse
    let lr = payload::llm_response("It is 4.", 150000);
    let bytes = payload::encode_payload(&lr);
    let decoded = payload::decode_payload(&bytes).unwrap();
    assert_eq!(lr, decoded);

    println!("=== Payload Roundtrip Test PASSED (all 7 types) ===");
}

#[test]
fn tampered_envelope_rejected() {
    let signing_key = SigningKey::generate(&mut OsRng);
    let sender_did = Did::new(signing_key.verifying_key().to_bytes());

    let header = EnvelopeHeader {
        version: 7,
        message_id: MessageId([2u8; 16]),
        sender_did,
        recipient_did: Did([0xCC; 32]),
        sequence_number: 0,
        timestamp_us: 9999,
        priority: omnimesh::envelope::Priority::Critical,
        payload_type: omnimesh::envelope::PayloadType::Raw,
    };

    let mut payload_buf = PayloadStorage::<128>::new();
    payload_buf.push_bytes(b"original data").unwrap();

    let mut envelope = SignedEnvelope::sign(header, payload_buf, &signing_key);

    // Tamper with the payload
    let _ = envelope.payload.push_bytes(b"TAMPERED");

    // Verification must fail
    let mode = OmnimeshMode::production();
    let security = SecurityLayer::new(&mode, None);
    assert!(security.verify(&envelope).is_err(), "Tampered envelope must be rejected");

    println!("=== Tamper Detection Test PASSED ===");
}
