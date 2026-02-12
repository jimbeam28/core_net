// src/common/pool/stats.rs
//
// Pool statistics

use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;

/// Pool status snapshot
#[derive(Debug, Clone)]
pub struct PoolStatus {
    pub idle: usize,
    pub active: usize,
    pub utilization: f64,
}

/// Pool statistics (thread-safe with atomic operations)
#[derive(Debug)]
pub struct PoolStats {
    pub total_allocations: AtomicU64,
    pub pooled_allocations: AtomicU64,
    pub overflow_allocations: AtomicU64,
    pub idle_count: AtomicUsize,
    pub active_count: AtomicUsize,
    pub wait_count: AtomicUsize,
    pub total_wait_ns: AtomicU64,
    pub release_count: AtomicU64,
}

impl Clone for PoolStats {
    fn clone(&self) -> Self {
        Self {
            total_allocations: AtomicU64::new(self.total_allocations.load(Ordering::Relaxed)),
            pooled_allocations: AtomicU64::new(self.pooled_allocations.load(Ordering::Relaxed)),
            overflow_allocations: AtomicU64::new(self.overflow_allocations.load(Ordering::Relaxed)),
            idle_count: AtomicUsize::new(self.idle_count.load(Ordering::Relaxed)),
            active_count: AtomicUsize::new(self.active_count.load(Ordering::Relaxed)),
            wait_count: AtomicUsize::new(self.wait_count.load(Ordering::Relaxed)),
            total_wait_ns: AtomicU64::new(self.total_wait_ns.load(Ordering::Relaxed)),
            release_count: AtomicU64::new(self.release_count.load(Ordering::Relaxed)),
        }
    }
}

impl PoolStats {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            total_allocations: AtomicU64::new(0),
            pooled_allocations: AtomicU64::new(0),
            overflow_allocations: AtomicU64::new(0),
            idle_count: AtomicUsize::new(0),
            active_count: AtomicUsize::new(0),
            wait_count: AtomicUsize::new(0),
            total_wait_ns: AtomicU64::new(0),
            release_count: AtomicU64::new(0),
        })
    }

    /// Get pool utilization rate (active / (active + idle))
    pub fn utilization_rate(&self) -> f64 {
        let active = self.active_count.load(Ordering::Relaxed) as f64;
        let idle = self.idle_count.load(Ordering::Relaxed) as f64;
        if active + idle == 0.0 {
            0.0
        } else {
            active / (active + idle)
        }
    }

    /// Get average wait time in nanoseconds
    pub fn avg_wait_ns(&self) -> u64 {
        let waits = self.wait_count.load(Ordering::Relaxed);
        if waits == 0 {
            0
        } else {
            self.total_wait_ns.load(Ordering::Relaxed) / waits as u64
        }
    }

    /// Reset all statistics
    pub fn reset(&self) {
        self.total_allocations.store(0, Ordering::Relaxed);
        self.pooled_allocations.store(0, Ordering::Relaxed);
        self.overflow_allocations.store(0, Ordering::Relaxed);
        self.wait_count.store(0, Ordering::Relaxed);
        self.total_wait_ns.store(0, Ordering::Relaxed);
        self.release_count.store(0, Ordering::Relaxed);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pool_stats_new() {
        let stats = PoolStats::new();
        assert_eq!(stats.total_allocations.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn test_pool_stats_clone() {
        let stats = PoolStats::new();
        stats.total_allocations.store(100, Ordering::Relaxed);
        let cloned = stats.clone();
        assert_eq!(cloned.total_allocations.load(Ordering::Relaxed), 100);
    }
}
