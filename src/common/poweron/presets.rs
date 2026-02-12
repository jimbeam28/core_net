// src/common/poweron/presets.rs
//
// 预设配置

use std::time::Duration;
use crate::common::{queue, pool};
use super::config::SystemConfig;

/// 创建默认配置
///
/// 适用于大多数通用场景
pub fn default() -> SystemConfig {
    SystemConfig::default()
}

/// 创建高吞吐量配置
///
/// 适用于大流量传输场景，提供更大的对象池和队列容量
pub fn high_throughput() -> SystemConfig {
    SystemConfig {
        packet_pool: pool::PacketPoolConfig {
            pool: pool::PoolConfig {
                capacity: 1000,
                initial_capacity: 100,
                alloc_strategy: pool::AllocStrategy::Recent,
                wait_strategy: pool::WaitStrategy::Timeout(Duration::from_millis(50)),
                allow_overflow: true,
            },
            packet_size: 9000,  // 支持巨型帧
            header_reserve: 128,
            trailer_reserve: 0,
            default_interface: None,
        },
        rxq_capacity: 8192,
        txq_capacity: 8192,
        queue_blocking: true,
        queue_wait_strategy: queue::WaitStrategy::Yield,
        warmup_pool: true,
        warmup_count: 100,
    }
}

/// 创建低延迟配置
///
/// 适用于实时通信场景，使用 LIFO 策略提高缓存命中率
pub fn low_latency() -> SystemConfig {
    SystemConfig {
        packet_pool: pool::PacketPoolConfig {
            pool: pool::PoolConfig {
                capacity: 500,
                initial_capacity: 50,
                alloc_strategy: pool::AllocStrategy::Lifo,
                wait_strategy: pool::WaitStrategy::Spin(100),
                allow_overflow: false,
            },
            packet_size: 1514,
            header_reserve: 128,
            trailer_reserve: 0,
            default_interface: None,
        },
        rxq_capacity: 256,
        txq_capacity: 256,
        queue_blocking: true,
        queue_wait_strategy: queue::WaitStrategy::Spin,
        warmup_pool: true,
        warmup_count: 50,
    }
}

/// 创建内存受限配置
///
/// 适用于嵌入式设备或内存受限环境
pub fn memory_constrained() -> SystemConfig {
    SystemConfig {
        packet_pool: pool::PacketPoolConfig {
            pool: pool::PoolConfig {
                capacity: 50,
                initial_capacity: 5,
                alloc_strategy: pool::AllocStrategy::Fifo,
                wait_strategy: pool::WaitStrategy::Immediate,
                allow_overflow: false,
            },
            packet_size: 1514,
            header_reserve: 64,
            trailer_reserve: 0,
            default_interface: None,
        },
        rxq_capacity: 256,
        txq_capacity: 256,
        queue_blocking: false,
        queue_wait_strategy: queue::WaitStrategy::Immediate,
        warmup_pool: false,
        warmup_count: 0,
    }
}

/// 创建自定义容量配置
///
/// # 参数
/// - `pool_cap`: 对象池容量
/// - `rxq_cap`: 接收队列容量
/// - `txq_cap`: 发送队列容量
pub fn with_capacity(pool_cap: usize, rxq_cap: usize, txq_cap: usize) -> SystemConfig {
    let mut config = SystemConfig::default();
    config.packet_pool.pool.capacity = pool_cap;
    config.rxq_capacity = rxq_cap;
    config.txq_capacity = txq_cap;
    config
}

// ========== 单元测试 ==========

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_preset_default() {
        let config = default();
        assert_eq!(config.rxq_capacity, 1024);
        assert_eq!(config.txq_capacity, 1024);
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_preset_high_throughput() {
        let config = high_throughput();
        assert_eq!(config.packet_pool.pool.capacity, 1000);
        assert_eq!(config.rxq_capacity, 8192);
        assert_eq!(config.txq_capacity, 8192);
        assert_eq!(config.packet_pool.packet_size, 9000);
        assert!(config.packet_pool.pool.allow_overflow);
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_preset_low_latency() {
        let config = low_latency();
        assert_eq!(config.packet_pool.pool.capacity, 500);
        assert_eq!(config.rxq_capacity, 256);
        assert_eq!(config.txq_capacity, 256);
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_preset_memory_constrained() {
        let config = memory_constrained();
        assert_eq!(config.packet_pool.pool.capacity, 50);
        assert_eq!(config.rxq_capacity, 256);
        assert_eq!(config.txq_capacity, 256);
        assert!(!config.queue_blocking);
        assert!(!config.warmup_pool);
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_preset_with_capacity() {
        let config = with_capacity(200, 512, 512);
        assert_eq!(config.packet_pool.pool.capacity, 200);
        assert_eq!(config.rxq_capacity, 512);
        assert_eq!(config.txq_capacity, 512);
        assert!(config.validate().is_ok());
    }
}
