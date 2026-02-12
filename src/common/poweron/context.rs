// src/common/poweron/context.rs
//
// 系统上下文实现

use std::sync::{Arc, RwLock};
use crate::common::{
    queue::{SpscQueue, QueueConfig, QueueError},
    pool::{PacketPool, PacketPoolConfig, PoolError},
    Result, CoreError,
};
use super::{
    config::SystemConfig,
    error::PowerOnError,
    state::{SystemState, SystemStatus},
};

/// 系统上下文，持有所有资源的所有权
pub struct SystemContext {
    /// Packet 对象池
    pub pool: Arc<PacketPool>,

    /// 接收队列（注入器 -> 处理线程）
    pub rxq: Arc<SpscQueue<crate::common::Packet>>,

    /// 发送队列（处理线程 -> 输出）
    pub txq: Arc<SpscQueue<crate::common::Packet>>,

    /// 系统状态
    state: Arc<RwLock<SystemState>>,
}

impl SystemContext {
    /// 创建新的系统上下文（内部使用）
    fn new(
        pool: Arc<PacketPool>,
        rxq: Arc<SpscQueue<crate::common::Packet>>,
        txq: Arc<SpscQueue<crate::common::Packet>>,
    ) -> Self {
        Self {
            pool,
            rxq,
            txq,
            state: Arc::new(RwLock::new(SystemState::Running)),
        }
    }

    /// 获取对象池引用
    pub fn pool(&self) -> &Arc<PacketPool> {
        &self.pool
    }

    /// 获取接收队列引用
    pub fn rxq(&self) -> &Arc<SpscQueue<crate::common::Packet>> {
        &self.rxq
    }

    /// 获取发送队列引用
    pub fn txq(&self) -> &Arc<SpscQueue<crate::common::Packet>> {
        &self.txq
    }

    /// 获取系统状态
    pub fn state(&self) -> SystemState {
        self.state.read().map(|s| *s).unwrap_or(SystemState::Shutdown)
    }

    /// 判断系统是否运行中
    pub fn is_running(&self) -> bool {
        self.state().is_running()
    }

    /// 判断系统是否已关闭
    pub fn is_shutdown(&self) -> bool {
        self.state().is_shutdown()
    }

    /// 获取系统状态快照
    pub fn status(&self) -> SystemStatus {
        let pool_status = self.pool.status();
        let state = self.state();
        let rxq_len = self.rxq.len();
        let rxq_capacity = self.rxq.capacity();
        let txq_len = self.txq.len();
        let txq_capacity = self.txq.capacity();

        SystemStatus::new(
            state,
            pool_status,
            rxq_len,
            rxq_capacity,
            txq_len,
            txq_capacity,
        )
    }

    /// 打印系统状态到控制台
    pub fn print_status(&self) {
        let status = self.status();
        println!("{}", status);
    }

    /// 设置系统状态（内部使用）
    fn set_state(&self, new_state: SystemState) {
        if let Ok(mut state) = self.state.write() {
            *state = new_state;
        }
    }
}

// ========== 单元测试 ==========

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::pool;

    #[test]
    fn test_context_creation() {
        // 创建测试配置
        let pool_config = PacketPoolConfig {
            pool: pool::PoolConfig {
                capacity: 10,
                initial_capacity: 2,
                alloc_strategy: pool::AllocStrategy::Fifo,
                wait_strategy: pool::WaitStrategy::Immediate,
                allow_overflow: false,
            },
            packet_size: 1500,
            header_reserve: 128,
            trailer_reserve: 0,
            default_interface: None,
        };

        let pool = Arc::new(PacketPool::new(pool_config).unwrap());
        let rxq = Arc::new(SpscQueue::with_config(QueueConfig {
            capacity: 16,
            blocking: false,
            wait_strategy: crate::common::queue::WaitStrategy::Immediate,
        }));
        let txq = Arc::new(SpscQueue::with_config(QueueConfig {
            capacity: 16,
            blocking: false,
            wait_strategy: crate::common::queue::WaitStrategy::Immediate,
        }));

        let context = SystemContext::new(pool, rxq, txq);

        assert!(context.is_running());
        assert!(!context.is_shutdown());
    }

    #[test]
    fn test_context_status() {
        let pool_config = PacketPoolConfig {
            pool: pool::PoolConfig {
                capacity: 10,
                initial_capacity: 2,
                alloc_strategy: pool::AllocStrategy::Fifo,
                wait_strategy: pool::WaitStrategy::Immediate,
                allow_overflow: false,
            },
            packet_size: 1500,
            header_reserve: 128,
            trailer_reserve: 0,
            default_interface: None,
        };

        let pool = Arc::new(PacketPool::new(pool_config).unwrap());
        let rxq = Arc::new(SpscQueue::with_config(QueueConfig {
            capacity: 16,
            blocking: false,
            wait_strategy: crate::common::queue::WaitStrategy::Immediate,
        }));
        let txq = Arc::new(SpscQueue::with_config(QueueConfig {
            capacity: 16,
            blocking: false,
            wait_strategy: crate::common::queue::WaitStrategy::Immediate,
        }));

        let context = SystemContext::new(pool, rxq, txq);

        let status = context.status();
        assert_eq!(status.rxq_capacity, 16);
        assert_eq!(status.txq_capacity, 16);
        assert!(status.state.is_running());
    }
}
