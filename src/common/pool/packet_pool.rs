// src/common/pool/packet_pool.rs
// Packet object pool adapter

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;
use std::ops::{Deref, DerefMut};

use super::{Pool, PoolConfig, PoolError, PoolStats, Pooled, Clear, AllocStrategy, WaitStrategy};
use crate::common::{Packet, InterfaceId, CoreError};

#[derive(Debug, Clone)]
pub struct PacketPoolConfig {
    pub pool: PoolConfig,
    pub packet_size: usize,
    pub header_reserve: usize,
    pub trailer_reserve: usize,
    pub default_interface: Option<InterfaceId>,
}

impl Default for PacketPoolConfig {
    fn default() -> Self {
        Self {
            pool: PoolConfig::default(),
            packet_size: 1514,
            header_reserve: 128,
            trailer_reserve: 0,
            default_interface: None,
        }
    }
}

impl PacketPoolConfig {
    pub fn high_throughput() -> Self {
        Self {
            pool: PoolConfig {
                capacity: 1000,
                initial_capacity: 200,
                alloc_strategy: AllocStrategy::Recent,
                wait_strategy: WaitStrategy::Spin(100),
                allow_overflow: true,
            },
            packet_size: 9000,
            header_reserve: 256,
            trailer_reserve: 64,
            default_interface: None,
        }
    }

    pub fn low_latency() -> Self {
        Self {
            pool: PoolConfig {
                capacity: 500,
                initial_capacity: 100,
                alloc_strategy: AllocStrategy::Lifo,
                wait_strategy: WaitStrategy::Immediate,
                allow_overflow: false,
            },
            packet_size: 1514,
            header_reserve: 128,
            trailer_reserve: 0,
            default_interface: None,
        }
    }

    pub fn memory_constrained() -> Self {
        Self {
            pool: PoolConfig {
                capacity: 50,
                initial_capacity: 10,
                alloc_strategy: AllocStrategy::Fifo,
                wait_strategy: WaitStrategy::Timeout(std::time::Duration::from_millis(10)),
                allow_overflow: false,
            },
            packet_size: 1514,
            header_reserve: 64,
            trailer_reserve: 0,
            default_interface: None,
        }
    }
}

#[derive(Debug)]
pub struct PacketPoolStats {
    pub base: PoolStats,
    pub total_bytes: AtomicU64,
    pub pooled_capacity_bytes: AtomicU64,
    pub avg_packet_length: AtomicU64,
}

impl PacketPoolStats {
    fn new(base: &PoolStats, capacity: usize, packet_size: usize) -> Self {
        Self {
            base: base.clone(),
            total_bytes: AtomicU64::new(0),
            pooled_capacity_bytes: AtomicU64::new((capacity * packet_size) as u64),
            avg_packet_length: AtomicU64::new(0),
        }
    }

    pub fn memory_efficiency(&self) -> f64 {
        let total = self.total_bytes.load(Ordering::Relaxed) as f64;
        let capacity = self.pooled_capacity_bytes.load(Ordering::Relaxed) as f64;
        if capacity == 0.0 { 0.0 } else { total / capacity }
    }
}

pub struct PacketPool {
    inner: Pool<Packet>,
    config: PacketPoolConfig,
    stats: Arc<PacketPoolStats>,
}

impl Clone for PacketPool {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            config: self.config.clone(),
            stats: Arc::clone(&self.stats),
        }
    }
}

impl PacketPool {
    pub fn new(config: PacketPoolConfig) -> Result<Self, PoolError> {
        if config.packet_size == 0 {
            return Err(PoolError::InvalidConfig("packet_size must be greater than 0".to_string()));
        }

        let packet_size = config.packet_size + config.header_reserve + config.trailer_reserve;

        let inner = Pool::with_resetter(
            move || {
                let mut packet = Packet::new(packet_size);
                let _ = packet.reserve_header(config.header_reserve);
                if let Some(iface) = config.default_interface {
                    packet.set_interface(iface);
                }
                packet
            },
            move |packet: &mut Packet| {
                packet.clear();
                let _ = packet.reserve_header(config.header_reserve);
                if let Some(iface) = config.default_interface {
                    packet.set_interface(iface);
                }
            },
            config.pool.clone(),
        );

        let stats = Arc::new(PacketPoolStats::new(inner.stats(), config.pool.capacity, packet_size));
        Ok(Self { inner, config, stats })
    }

    pub fn with_capacity(capacity: usize) -> Result<Self, PoolError> {
        let config = PacketPoolConfig {
            pool: PoolConfig { capacity, ..Default::default() },
            ..Default::default()
        };
        Self::new(config)
    }

    pub fn acquire(&self) -> Result<PooledPacket, PoolError> {
        let inner = self.inner.acquire()?;
        Ok(PooledPacket::new(inner, Arc::new(self.clone())))
    }

    pub fn try_acquire(&self) -> Result<PooledPacket, PoolError> {
        let inner = self.inner.try_acquire()?;
        Ok(PooledPacket::new(inner, Arc::new(self.clone())))
    }

    pub fn release(&self, packet: Packet) {
        self.inner.release(packet);
    }

    pub fn release_and_clear(&self, packet: Packet) {
        self.inner.release_and_clear(packet);
    }

    pub fn stats(&self) -> &PacketPoolStats {
        &self.stats
    }

    pub fn warm_up(&self, count: usize) -> Result<(), PoolError> {
        self.inner.warm_up(count)
    }

    pub fn shrink(&self, target_count: usize) {
        self.inner.shrink(target_count);
    }

    pub fn shutdown(&self) {
        self.inner.shutdown();
    }
}

pub struct PooledPacket {
    inner: Pooled<Packet>,
    pool: Arc<PacketPool>,
    track_bytes: bool,
}

impl PooledPacket {
    fn new(inner: Pooled<Packet>, pool: Arc<PacketPool>) -> Self {
        Self { inner, pool, track_bytes: true }
    }

    pub fn release(self) {
        self.inner.release();
    }

    pub fn release_and_clear(self) {
        self.inner.release_and_clear();
    }

    pub fn detach(mut self) -> Packet {
        self.inner.detach()
    }

    pub fn pool(&self) -> &PacketPool {
        &self.pool
    }
}

impl Deref for PooledPacket {
    type Target = Packet;
    fn deref(&self) -> &Self::Target {
        self.inner.deref()
    }
}

impl DerefMut for PooledPacket {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.inner.deref_mut()
    }
}

impl Drop for PooledPacket {
    fn drop(&mut self) {
        if self.track_bytes {
            if let Some(packet) = self.inner.inner() {
                let len = packet.len();
                self.pool.stats.total_bytes.fetch_add(len as u64, Ordering::Relaxed);
            }
        }
    }
}

pub struct PacketBuilder {
    pool: Arc<PacketPool>,
    packet: Option<PooledPacket>,
}

impl PacketBuilder {
    pub fn new(pool: &PacketPool) -> Result<Self, PoolError> {
        Ok(Self { pool: Arc::new(pool.clone()), packet: None })
    }

    pub fn acquire(&mut self) -> Result<&mut PooledPacket, PoolError> {
        if self.packet.is_none() {
            self.packet = Some(self.pool.acquire()?);
        }
        Ok(self.packet.as_mut().unwrap())
    }

    pub fn with_timestamp(mut self, timestamp: Instant) -> Self {
        if let Some(pkt) = &mut self.packet {
            pkt.set_timestamp(timestamp);
        }
        self
    }

    pub fn with_interface(mut self, interface: InterfaceId) -> Self {
        if let Some(pkt) = &mut self.packet {
            pkt.set_interface(interface);
        }
        self
    }

    pub fn with_data(mut self, data: &[u8]) -> Result<Self, CoreError> {
        if let Some(pkt) = &mut self.packet {
            pkt.extend_from_slice(data)?;
        }
        Ok(self)
    }

    pub fn with_header_reserve(mut self, len: usize) -> Result<Self, CoreError> {
        if let Some(pkt) = &mut self.packet {
            pkt.reserve_header(len)?;
        }
        Ok(self)
    }

    pub fn build(mut self) -> Result<PooledPacket, PoolError> {
        self.packet.take().ok_or(PoolError::Empty)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_packet_pool_config_default() {
        let config = PacketPoolConfig::default();
        assert_eq!(config.packet_size, 1514);
        assert_eq!(config.header_reserve, 128);
    }

    #[test]
    fn test_packet_pool_with_capacity() {
        let pool = PacketPool::with_capacity(10).unwrap();
        let status = pool.inner.status();
        assert_eq!(status.idle, 10);
    }

    #[test]
    fn test_packet_pool_acquire() {
        let pool = PacketPool::with_capacity(10).unwrap();
        let _packet = pool.acquire().unwrap();
    }
}
