// src/common/pool/pool.rs
//
// Generic object pool implementation

use std::sync::{Arc, Mutex};
use std::sync::atomic::Ordering;
use std::thread;
use std::time::{Duration, Instant};

use super::{
    PoolConfig, PoolError, PoolStatus, PoolStats,
    AllocStrategy, WaitStrategy, Clear, Pooled
};

/// Generic object pool
pub struct Pool<T> {
    idle: Mutex<Vec<T>>,
    config: PoolConfig,
    stats: Arc<PoolStats>,
    factory: Box<dyn Fn() -> T + Send + Sync>,
    resetter: Option<Box<dyn Fn(&mut T) + Send + Sync>>,
    shutdown: Arc<Mutex<bool>>,
}

impl<T> Pool<T>
where
    T: Send + 'static,
{
    /// Create new pool with factory
    pub fn new<F>(factory: F, config: PoolConfig) -> Self
    where
        F: Fn() -> T + Send + Sync + 'static,
    {
        Self::with_resetter(factory, |_: &mut T| {}, config)
    }

    /// Create pool with factory and resetter
    pub fn with_resetter<F, R>(
        factory: F,
        resetter: R,
        config: PoolConfig,
    ) -> Self
    where
        F: Fn() -> T + Send + Sync + 'static,
        R: Fn(&mut T) + Send + Sync + 'static,
    {
        if config.capacity == 0 {
            panic!("pool capacity must be greater than 0");
        }
        if config.initial_capacity > config.capacity {
            panic!("initial capacity cannot exceed pool capacity");
        }

        let stats = PoolStats::new();

        let mut idle = Vec::with_capacity(config.initial_capacity);
        for _ in 0..config.initial_capacity {
            idle.push(factory());
        }

        stats.idle_count.store(config.initial_capacity, Ordering::Relaxed);

        Self {
            idle: Mutex::new(idle),
            config,
            stats,
            factory: Box::new(factory),
            resetter: Some(Box::new(resetter)),
            shutdown: Arc::new(Mutex::new(false)),
        }
    }

    /// Acquire object (may wait)
    pub fn acquire(&self) -> Result<Pooled<T>, PoolError> {
        self.try_acquire_internal(true)
    }

    /// Try acquire object (non-blocking)
    pub fn try_acquire(&self) -> Result<Pooled<T>, PoolError> {
        self.try_acquire_internal(false)
    }

    fn try_acquire_internal(&self, can_wait: bool) -> Result<Pooled<T>, PoolError> {
        {
            let shutdown = self.shutdown.lock().unwrap();
            if *shutdown {
                return Err(PoolError::Shutdown);
            }
        }

        let start = Instant::now();

        loop {
            {
                let mut idle = self.idle.lock().unwrap();
                if let Some(item) = self.pop_from_idle(&mut idle) {
                    self.stats.idle_count.fetch_sub(1, Ordering::Relaxed);
                    self.stats.active_count.fetch_add(1, Ordering::Relaxed);
                    self.stats.total_allocations.fetch_add(1, Ordering::Relaxed);
                    self.stats.pooled_allocations.fetch_add(1, Ordering::Relaxed);
                    return Ok(Pooled::new(item, Arc::new(self.clone_handle())));
                }
            }

            if self.config.allow_overflow {
                let item = (self.factory)();
                self.stats.active_count.fetch_add(1, Ordering::Relaxed);
                self.stats.total_allocations.fetch_add(1, Ordering::Relaxed);
                self.stats.overflow_allocations.fetch_add(1, Ordering::Relaxed);
                return Ok(Pooled::new(item, Arc::new(self.clone_handle())));
            }

            if !can_wait {
                return Err(PoolError::Empty);
            }

            match self.config.wait_strategy {
                WaitStrategy::Immediate => return Err(PoolError::Empty),
                WaitStrategy::Spin(count) => {
                    self.stats.wait_count.fetch_add(1, Ordering::Relaxed);
                    let wait_ns = start.elapsed().as_nanos();
                    self.stats.total_wait_ns.fetch_add(wait_ns, Ordering::Relaxed);
                    for _ in 0..count {
                        thread::yield_now();
                    }
                }
                WaitStrategy::Yield => {
                    self.stats.wait_count.fetch_add(1, Ordering::Relaxed);
                    let wait_ns = start.elapsed().as_nanos();
                    self.stats.total_wait_ns.fetch_add(wait_ns, Ordering::Relaxed);
                    thread::yield_now();
                }
                WaitStrategy::Timeout(duration) => {
                    self.stats.wait_count.fetch_add(1, Ordering::Relaxed);
                    if start.elapsed() >= duration {
                        let wait_ns = start.elapsed().as_nanos();
                        self.stats.total_wait_ns.fetch_add(wait_ns, Ordering::Relaxed);
                        return Err(PoolError::Timeout(duration));
                    }
                    thread::sleep(Duration::from_millis(1));
                }
                WaitStrategy::Blocking => {
                    self.stats.wait_count.fetch_add(1, Ordering::Relaxed);
                    thread::sleep(Duration::from_millis(10));
                }
            }
        }
    }

    /// Acquire multiple objects
    pub fn acquire_many(&self, count: usize) -> Result<Vec<Pooled<T>>, PoolError> {
        let mut result = Vec::with_capacity(count);
        for _ in 0..count {
            result.push(self.acquire()?);
        }
        Ok(result)
    }

    /// Release object back to pool
    pub fn release(&self, item: T) {
        {
            let shutdown = self.shutdown.lock().unwrap();
            if *shutdown {
                return;
            }
        }

        let mut idle = self.idle.lock().unwrap();

        if idle.len() >= self.config.capacity {
            return;
        }

        idle.push(item);
        self.stats.idle_count.fetch_add(1, Ordering::Relaxed);
        self.stats.active_count.fetch_sub(1, Ordering::Relaxed);
        self.stats.release_count.fetch_add(1, Ordering::Relaxed);
    }

    /// Release and clear object
    pub fn release_and_clear(&self, mut item: T)
    where
        T: Clear,
    {
        item.clear();
        self.release(item);
    }

    /// Get pool statistics
    pub fn stats(&self) -> &PoolStats {
        &self.stats
    }

    /// Reset statistics
    pub fn reset_stats(&self) {
        self.stats.reset();
    }

    /// Warm up pool (pre-allocate objects)
    pub fn warm_up(&self, count: usize) -> Result<(), PoolError> {
        let mut idle = self.idle.lock().unwrap();

        let current = idle.len();
        if current >= self.config.capacity {
            return Err(PoolError::Full);
        }

        let to_add = count.min(self.config.capacity - current);
        for _ in 0..to_add {
            idle.push((self.factory)());
        }

        self.stats.idle_count.fetch_add(to_add, Ordering::Relaxed);
        Ok(())
    }

    /// Shrink pool (release excess idle objects)
    pub fn shrink(&self, target_count: usize) {
        let mut idle = self.idle.lock().unwrap();

        let current = idle.len();
        if current > target_count {
            let to_remove = current - target_count;
            for _ in 0..to_remove {
                idle.pop();
            }
            self.stats.idle_count.fetch_sub(to_remove, Ordering::Relaxed);
        }
    }

    /// Get current pool status
    pub fn status(&self) -> PoolStatus {
        let idle = self.stats.idle_count.load(Ordering::Relaxed);
        let active = self.stats.active_count.load(Ordering::Relaxed);
        PoolStatus {
            idle,
            active,
            utilization: self.stats.utilization_rate(),
        }
    }

    /// Shutdown pool
    pub fn shutdown(&self) {
        let mut shutdown = self.shutdown.lock().unwrap();
        *shutdown = true;
    }

    fn pop_from_idle(&self, idle: &mut Vec<T>) -> Option<T> {
        if idle.is_empty() {
            return None;
        }

        match self.config.alloc_strategy {
            AllocStrategy::Fifo => {
                if idle.len() > 1 {
                    Some(idle.remove(0))
                } else {
                    idle.pop()
                }
            }
            AllocStrategy::Lifo => idle.pop(),
            AllocStrategy::Random => idle.pop(),
            AllocStrategy::Recent => idle.pop(),
        }
    }

    fn clone_handle(&self) -> Self {
        Self {
            idle: Mutex::new(Vec::new()),
            config: self.config.clone(),
            stats: Arc::clone(&self.stats),
            factory: Box::new(|_| panic!("cannot create object through cloned handle")),
            resetter: None,
            shutdown: Arc::clone(&self.shutdown),
        }
    }
}

impl<T> Clone for Pool<T> {
    fn clone(&self) -> Self {
        Self {
            idle: Mutex::new(Vec::new()),
            config: self.config.clone(),
            stats: Arc::clone(&self.stats),
            factory: Box::new(|_| panic!("cannot create object through cloned pool")),
            resetter: None,
            shutdown: Arc::clone(&self.shutdown),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::pool::Clear;

    #[derive(Debug, PartialEq)]
    struct TestItem {
        value: u32,
    }

    impl Clear for TestItem {
        fn clear(&mut self) {
            self.value = 0;
        }
    }

    #[test]
    fn test_pool_new() {
        let pool = Pool::new(
            || TestItem { value: 42 },
            PoolConfig {
                capacity: 10,
                initial_capacity: 5,
                ..Default::default()
            },
        );
        assert_eq!(pool.stats().idle_count.load(Ordering::Relaxed), 5);
    }

    #[test]
    fn test_pool_acquire_release() {
        let pool = Pool::with_resetter(
            || TestItem { value: 100 },
            |item: &mut TestItem| { item.value = 0; },
            PoolConfig::default(),
        );

        let pooled = pool.acquire().unwrap();
        assert_eq!(pooled.value, 100);
        assert_eq!(pool.stats().active_count.load(Ordering::Relaxed), 1);

        drop(pooled);
        assert_eq!(pool.stats().active_count.load(Ordering::Relaxed), 0);
        assert_eq!(pool.stats().idle_count.load(Ordering::Relaxed), 1);
    }
}
