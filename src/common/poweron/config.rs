// src/common/poweron/config.rs
//
// 系统配置定义

use std::time::Duration;
use crate::common::{queue, pool};

/// 系统配置结构
#[derive(Debug, Clone)]
pub struct SystemConfig {
    /// Packet 对象池配置
    pub packet_pool: pool::PacketPoolConfig,

    /// 接收队列容量
    pub rxq_capacity: usize,

    /// 发送队列容量
    pub txq_capacity: usize,

    /// 是否阻塞模式
    pub queue_blocking: bool,

    /// 队列等待策略
    pub queue_wait_strategy: queue::WaitStrategy,

    /// 是否预热对象池
    pub warmup_pool: bool,

    /// 预热对象数量
    pub warmup_count: usize,
}

impl Default for SystemConfig {
    fn default() -> Self {
        Self {
            packet_pool: pool::PacketPoolConfig::default(),
            rxq_capacity: 1024,
            txq_capacity: 1024,
            queue_blocking: true,
            queue_wait_strategy: queue::WaitStrategy::Yield,
            warmup_pool: true,
            warmup_count: 10,
        }
    }
}

impl SystemConfig {
    /// 创建新的系统配置
    pub fn new() -> Self {
        Self::default()
    }

    /// 设置接收队列容量
    pub fn with_rxq_capacity(mut self, capacity: usize) -> Self {
        self.rxq_capacity = capacity;
        self
    }

    /// 设置发送队列容量
    pub fn with_txq_capacity(mut self, capacity: usize) -> Self {
        self.txq_capacity = capacity;
        self
    }

    /// 设置队列容量（RxQ 和 TxQ 相同）
    pub fn with_queue_capacity(mut self, capacity: usize) -> Self {
        self.rxq_capacity = capacity;
        self.txq_capacity = capacity;
        self
    }

    /// 设置 Packet 对象池配置
    pub fn with_packet_pool(mut self, config: pool::PacketPoolConfig) -> Self {
        self.packet_pool = config;
        self
    }

    /// 设置队列阻塞模式
    pub fn with_blocking(mut self, blocking: bool) -> Self {
        self.queue_blocking = blocking;
        self
    }

    /// 设置队列等待策略
    pub fn with_wait_strategy(mut self, strategy: queue::WaitStrategy) -> Self {
        self.queue_wait_strategy = strategy;
        self
    }

    /// 启用或禁用池预热
    pub fn with_warmup(mut self, warmup: bool) -> Self {
        self.warmup_pool = warmup;
        self
    }

    /// 设置预热对象数量
    pub fn with_warmup_count(mut self, count: usize) -> Self {
        self.warmup_count = count;
        self
    }

    /// 验证配置是否有效
    pub fn validate(&self) -> Result<(), String> {
        // 验证队列容量
        if self.rxq_capacity < queue::MIN_QUEUE_CAPACITY {
            return Err(format!("接收队列容量小于最小值: {} < {}",
                self.rxq_capacity, queue::MIN_QUEUE_CAPACITY));
        }
        if self.rxq_capacity > queue::MAX_QUEUE_CAPACITY {
            return Err(format!("接收队列容量超过最大值: {} > {}",
                self.rxq_capacity, queue::MAX_QUEUE_CAPACITY));
        }

        if self.txq_capacity < queue::MIN_QUEUE_CAPACITY {
            return Err(format!("发送队列容量小于最小值: {} < {}",
                self.txq_capacity, queue::MIN_QUEUE_CAPACITY));
        }
        if self.txq_capacity > queue::MAX_QUEUE_CAPACITY {
            return Err(format!("发送队列容量超过最大值: {} > {}",
                self.txq_capacity, queue::MAX_QUEUE_CAPACITY));
        }

        // 验证预热数量
        if self.warmup_pool && self.warmup_count == 0 {
            return Err("启用预热但预热数量为0".to_string());
        }
        if self.warmup_count > self.packet_pool.pool.capacity {
            return Err(format!("预热数量超过池容量: {} > {}",
                self.warmup_count, self.packet_pool.pool.capacity));
        }

        Ok(())
    }
}

// ========== 单元测试 ==========

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_default_config() {
        let config = SystemConfig::default();
        assert_eq!(config.rxq_capacity, 1024);
        assert_eq!(config.txq_capacity, 1024);
        assert!(config.queue_blocking);
        assert!(config.warmup_pool);
        assert_eq!(config.warmup_count, 10);
    }

    #[test]
    fn test_config_builders() {
        let config = SystemConfig::new()
            .with_rxq_capacity(2048)
            .with_txq_capacity(512)
            .with_blocking(false)
            .with_warmup(false);

        assert_eq!(config.rxq_capacity, 2048);
        assert_eq!(config.txq_capacity, 512);
        assert!(!config.queue_blocking);
        assert!(!config.warmup_pool);
    }

    #[test]
    fn test_config_validate() {
        let config = SystemConfig::default();
        assert!(config.validate().is_ok());

        // 测试容量过小
        let mut bad_config = config.clone();
        bad_config.rxq_capacity = 1;
        assert!(bad_config.validate().is_err());

        // 测试预热数量超过容量
        let mut bad_config = config;
        bad_config.warmup_count = 1000;
        assert!(bad_config.validate().is_err());
    }
}
