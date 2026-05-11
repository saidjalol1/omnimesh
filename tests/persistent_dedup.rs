//! Tests for persistent deduplication across restarts

use omnimesh::buffer::PayloadStorage;
use omnimesh::envelope::{Did, EnvelopeHeader, MessageId, SignedEnvelope};
use omnimesh::config::OmnimeshMode;
use omnimesh::runtime::delivery::{DeliveryLayer, DeliveryStatus};
use omnimesh::runtime::DtnStore;
use ed25519_dalek::SigningKey;
use rand_core::OsRng;

#[test]
fn persistent_dedup_survives_restart() {
    let temp_dir = std::env::temp_dir().join(format!("omnimesh-test-{}", std::process::id()));
    let dtn_path = temp_dir.to_str().unwrap();
    
    // Clean up any existing test data
    let _ = std::fs::remove_dir_all(&temp_dir);
    
    let signing_key = SigningKey::generate(&mut OsRng);
    let sender_did = Did::new(signing_key.verifying_key().to_bytes());
    let recipient_did = Did([0xBB; 32]);

    let header = EnvelopeHeader {
        version: 7,
        message_id: MessageId([1u8; 16]),
        sender_did,
        recipient_did,
        sequence_number: 0,
        timestamp_us: 1234567890,
        priority: omnimesh::envelope::Priority::High,
        payload_type: omnimesh::envelope::PayloadType::RobotCommand,
    };

    let mut payload_buf = PayloadStorage::<128>::new();
    payload_buf.push_bytes(b"test message").unwrap();
    let envelope = SignedEnvelope::sign(header, payload_buf, &signing_key);

    // First delivery layer instance
    {
        let mode = OmnimeshMode::Production(omnimesh::config::modes::ProductionConfig {
            crypto_enabled: true,
            exactly_once_enabled: true,
            ordering_enabled: true,
            dtn_enabled: true,
            dtn_path: Some(dtn_path.into()),
        });
        
        let delivery = DeliveryLayer::new(&mode);
        
        // First delivery should succeed
        let status = delivery.deliver(&envelope).unwrap();
        assert_eq!(status, DeliveryStatus::Delivered);
        
        // Second delivery should be duplicate
        let status = delivery.deliver(&envelope).unwrap();
        assert_eq!(status, DeliveryStatus::Duplicate);
    }
    
    // Second delivery layer instance (simulating restart)
    {
        let mode = OmnimeshMode::Production(omnimesh::config::modes::ProductionConfig {
            crypto_enabled: true,
            exactly_once_enabled: true,
            ordering_enabled: true,
            dtn_enabled: true,
            dtn_path: Some(dtn_path.into()),
        });
        
        let delivery = DeliveryLayer::new(&mode);
        
        // After restart, duplicate should still be detected!
        let status = delivery.deliver(&envelope).unwrap();
        assert_eq!(status, DeliveryStatus::Duplicate, 
            "Persistent deduplication should survive restart");
    }
    
    // Cleanup
    let _ = std::fs::remove_dir_all(&temp_dir);
    
    println!("=== Persistent Deduplication Test PASSED ===");
}

#[test]
fn persistent_dedup_cleanup_works() {
    let temp_dir = std::env::temp_dir().join(format!("omnimesh-cleanup-{}", std::process::id()));
    let dtn_path = temp_dir.to_str().unwrap();
    
    // Clean up any existing test data
    let _ = std::fs::remove_dir_all(&temp_dir);
    
    let dtn_store = DtnStore::new(dtn_path).unwrap();
    
    // Mark some messages as seen
    for i in 0..10 {
        let msg_id = MessageId([i; 16]);
        dtn_store.mark_message_seen(&msg_id).unwrap();
    }
    
    // Verify they're all there
    for i in 0..10 {
        let msg_id = MessageId([i; 16]);
        assert!(dtn_store.has_seen_message(&msg_id), "Message {} should be marked as seen", i);
    }
    
    // Wait a bit
    std::thread::sleep(std::time::Duration::from_secs(2));
    
    // Clean up messages older than 1 second (should remove all)
    let removed = dtn_store.cleanup_old_seen_messages(1).unwrap();
    assert_eq!(removed, 10, "Should have removed all 10 messages");
    
    // Verify they're gone
    for i in 0..10 {
        let msg_id = MessageId([i; 16]);
        assert!(!dtn_store.has_seen_message(&msg_id), "Message {} should be cleaned up", i);
    }
    
    // Cleanup
    let _ = std::fs::remove_dir_all(&temp_dir);
    
    println!("=== Persistent Dedup Cleanup Test PASSED ===");
}

#[test]
fn ring_buffer_and_persistent_dedup_work_together() {
    let temp_dir = std::env::temp_dir().join(format!("omnimesh-combined-{}", std::process::id()));
    let dtn_path = temp_dir.to_str().unwrap();
    
    // Clean up any existing test data
    let _ = std::fs::remove_dir_all(&temp_dir);
    
    let mode = OmnimeshMode::Production(omnimesh::config::modes::ProductionConfig {
        crypto_enabled: true,
        exactly_once_enabled: true,
        ordering_enabled: true,
        dtn_enabled: true,
        dtn_path: Some(dtn_path.into()),
    });
    
    let delivery = DeliveryLayer::new(&mode);
    let signing_key = SigningKey::generate(&mut OsRng);
    let sender_did = Did::new(signing_key.verifying_key().to_bytes());
    let recipient_did = Did([0xCC; 32]);
    
    // Send 100 messages
    for i in 0..100 {
        let header = EnvelopeHeader {
            version: 7,
            message_id: MessageId([i; 16]),
            sender_did,
            recipient_did,
            sequence_number: i as u64,
            timestamp_us: 1234567890 + i as u64,
            priority: omnimesh::envelope::Priority::Normal,
            payload_type: omnimesh::envelope::PayloadType::RobotCommand,
        };
        
        let mut payload_buf = PayloadStorage::<128>::new();
        payload_buf.push_bytes(format!("message {}", i).as_bytes()).unwrap();
        let envelope = SignedEnvelope::sign(header, payload_buf, &signing_key);
        
        let status = delivery.deliver(&envelope).unwrap();
        assert_eq!(status, DeliveryStatus::Delivered, "Message {} should be delivered", i);
        
        // Try to deliver duplicate immediately
        let status = delivery.deliver(&envelope).unwrap();
        assert_eq!(status, DeliveryStatus::Duplicate, "Message {} duplicate should be detected", i);
    }
    
    // Now try to re-deliver all messages (should all be duplicates)
    for i in 0..100 {
        let header = EnvelopeHeader {
            version: 7,
            message_id: MessageId([i; 16]),
            sender_did,
            recipient_did,
            sequence_number: i as u64,
            timestamp_us: 1234567890 + i as u64,
            priority: omnimesh::envelope::Priority::Normal,
            payload_type: omnimesh::envelope::PayloadType::RobotCommand,
        };
        
        let mut payload_buf = PayloadStorage::<128>::new();
        payload_buf.push_bytes(format!("message {}", i).as_bytes()).unwrap();
        let envelope = SignedEnvelope::sign(header, payload_buf, &signing_key);
        
        let status = delivery.deliver(&envelope).unwrap();
        assert_eq!(status, DeliveryStatus::Duplicate, 
            "Message {} should still be detected as duplicate", i);
    }
    
    // Cleanup
    let _ = std::fs::remove_dir_all(&temp_dir);
    
    println!("=== Ring Buffer + Persistent Dedup Test PASSED ===");
}
