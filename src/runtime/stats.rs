//! Runtime statistics and monitoring for OMNI-MESH.
//!
//! Provides real-time counters for messages, errors, and performance metrics.
//! Thread-safe via atomic operations — no locks in the hot path.

use core::sync::atomic::{AtomicBool, AtomicU64, Ordering};

/// Runtime statistics counters. All fields are atomic for lock-free access.
#[derive(Debug)]
pub struct RuntimeStats {
    pub total_messages_received: AtomicU64,
    pub total_messages_delivered: AtomicU64,
    pub total_dropped_pool_full: AtomicU64,
    pub total_dropped_signature_fail: AtomicU64,
    pub total_duplicates: AtomicU64,
    pub total_out_of_order_buffered: AtomicU64,
    pub total_stale: AtomicU64,
    pub current_pending_messages: AtomicU64,
    pub last_wcet_violation_us: AtomicU64,
    pub crypto_enabled: AtomicBool,
}

impl Default for RuntimeStats {
    fn default() -> Self {
        Self::new()
    }
}

impl RuntimeStats {
    pub fn new() -> Self {
        Self {
            total_messages_received: AtomicU64::new(0),
            total_messages_delivered: AtomicU64::new(0),
            total_dropped_pool_full: AtomicU64::new(0),
            total_dropped_signature_fail: AtomicU64::new(0),
            total_duplicates: AtomicU64::new(0),
            total_out_of_order_buffered: AtomicU64::new(0),
            total_stale: AtomicU64::new(0),
            current_pending_messages: AtomicU64::new(0),
            last_wcet_violation_us: AtomicU64::new(0),
            crypto_enabled: AtomicBool::new(false),
        }
    }

    pub fn record_received(&self) {
        self.total_messages_received.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_delivered(&self) {
        self.total_messages_delivered
            .fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_duplicate(&self) {
        self.total_duplicates.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_buffered(&self) {
        self.total_out_of_order_buffered
            .fetch_add(1, Ordering::Relaxed);
        self.current_pending_messages
            .fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_stale(&self) {
        self.total_stale.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_pool_full(&self) {
        self.total_dropped_pool_full.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_signature_fail(&self) {
        self.total_dropped_signature_fail
            .fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_wcet_violation(&self, elapsed_us: u64) {
        self.last_wcet_violation_us
            .store(elapsed_us, Ordering::Relaxed);
    }

    pub fn record_pending_drained(&self, count: u64) {
        self.current_pending_messages
            .fetch_sub(count, Ordering::Relaxed);
    }

    /// Returns a snapshot of all stats for logging/monitoring.
    pub fn snapshot(&self) -> StatsSnapshot {
        StatsSnapshot {
            total_messages_received: self.total_messages_received.load(Ordering::Relaxed),
            total_messages_delivered: self.total_messages_delivered.load(Ordering::Relaxed),
            total_dropped_pool_full: self.total_dropped_pool_full.load(Ordering::Relaxed),
            total_dropped_signature_fail: self.total_dropped_signature_fail.load(Ordering::Relaxed),
            total_duplicates: self.total_duplicates.load(Ordering::Relaxed),
            total_out_of_order_buffered: self.total_out_of_order_buffered.load(Ordering::Relaxed),
            total_stale: self.total_stale.load(Ordering::Relaxed),
            current_pending_messages: self.current_pending_messages.load(Ordering::Relaxed),
            last_wcet_violation_us: self.last_wcet_violation_us.load(Ordering::Relaxed),
            crypto_enabled: self.crypto_enabled.load(Ordering::Relaxed),
        }
    }
}

/// Immutable snapshot of runtime statistics.
#[derive(Debug, Clone)]
pub struct StatsSnapshot {
    pub total_messages_received: u64,
    pub total_messages_delivered: u64,
    pub total_dropped_pool_full: u64,
    pub total_dropped_signature_fail: u64,
    pub total_duplicates: u64,
    pub total_out_of_order_buffered: u64,
    pub total_stale: u64,
    pub current_pending_messages: u64,
    pub last_wcet_violation_us: u64,
    pub crypto_enabled: bool,
}
