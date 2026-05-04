use omnimesh::{PayloadStorage, SafetyBufferPool};

#[test]
fn payload_storage_overflow() {
    let mut storage = PayloadStorage::<4>::new();
    assert!(storage.push_bytes(&[1, 2, 3, 4]).is_ok());
    assert_eq!(storage.len(), 4);
    assert!(storage.push_bytes(&[5]).is_err());
}

#[test]
fn safety_buffer_pool_acquire_release() {
    let mut pool = SafetyBufferPool::<16, 2>::new();

    let (index1, slot1) = pool.acquire().expect("first buffer");
    slot1.push_bytes(b"abc").unwrap();
    assert_eq!(pool.utilization(), 0.5);

    let (index2, slot2) = pool.acquire().expect("second buffer");
    slot2.push_bytes(b"xyz").unwrap();
    assert_eq!(pool.utilization(), 1.0);

    assert!(pool.acquire().is_none());

    pool.release(index1);
    assert_eq!(pool.utilization(), 0.5);

    pool.release(index2);
    assert_eq!(pool.utilization(), 0.0);
}
