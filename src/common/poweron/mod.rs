// src/common/poweron/mod.rs
//
// 上电启动模块
// 负责系统资源的初始化和释放

mod config;
mod context;
mod error;
mod state;
mod presets;

pub use config::SystemConfig;
pub use context::SystemContext;
pub use error::PowerOnError;
pub use state::{SystemState, SystemStatus};

// 导出预设配置函数
pub use presets::{default, high_throughput, low_latency, memory_constrained, with_capacity};

use std::time::Duration;
use crate::common::{
    queue::{SpscQueue, QueueConfig},
    pool::{PacketPool, PacketPoolConfig, PoolConfig, AllocStrategy, WaitStrategy as PoolWaitStrategy},
    Result, CoreError,
};

/// 上电初始化
///
/// 使用指定配置初始化系统资源，创建 Packet 对象池和收发包队列。
///
/// # 参数
/// - `config`: 系统配置
///
/// # 返回
/// 包含所有资源的 SystemContext
///
/// # 错误
/// 资源创建失败时返回错误
pub fn boot(config: SystemConfig) -> Result<SystemContext> {
    // 验证配置
    config.validate()
        .map_err(|msg| CoreError::InvalidConfig(msg))?;

    // 1. 创建 Packet 对象池
    let pool = std::sync::Arc::new(
        PacketPool::new(config.packet_pool.clone())
            .map_err(|e| CoreError::Other(format!("对象池创建失败: {:?}", e)))?
    );

    // 2. 预热对象池（可选）
    if config.warmup_pool {
        pool.warm_up(config.warmup_count)
            .map_err(|e| CoreError::Other(format!("对象池预热失败: {:?}", e)))?;
    }

    // 3. 创建接收队列
    let rxq_config = QueueConfig {
        capacity: config.rxq_capacity,
        blocking: config.queue_blocking,
        wait_strategy: config.queue_wait_strategy,
    };
    let rxq = std::sync::Arc::new(SpscQueue::with_config(rxq_config));

    // 4. 创建发送队列
    let txq_config = QueueConfig {
        capacity: config.txq_capacity,
        blocking: config.queue_blocking,
        wait_strategy: config.queue_wait_strategy,
    };
    let txq = std::sync::Arc::new(SpscQueue::with_config(txq_config));

    // 5. 创建并返回系统上下文
    Ok(SystemContext::new(pool, rxq, txq))
}

/// 使用默认配置快速启动
pub fn boot_default() -> Result<SystemContext> {
    boot(default())
}

/// 使用指定容量快速启动
///
/// # 参数
/// - `pool_cap`: 对象池容量
/// - `rxq_cap`: 接收队列容量
/// - `txq_cap`: 发送队列容量
pub fn boot_with_capacity(pool_cap: usize, rxq_cap: usize, txq_cap: usize) -> Result<SystemContext> {
    boot(with_capacity(pool_cap, rxq_cap, txq_cap))
}

/// 下电释放
///
/// 优雅关闭系统，释放所有资源。
///
/// # 参数
/// - `context`: 系统上下文可变引用
///
/// # 流程
/// 1. 标记系统为 ShuttingDown 状态
/// 2. 关闭队列（停止接收新数据）
/// 3. 标记系统为 Shutdown 状态
///
/// 注意：实际的资源释放由 Drop trait 自动处理
pub fn shutdown(context: &mut SystemContext) -> Result<()> {
    // 1. 标记系统为正在关闭
    context.set_state(SystemState::ShuttingDown);

    // 2. 关闭队列
    context.txq().close();
    context.rxq().close();

    // 3. 标记系统为已关闭
    context.set_state(SystemState::Shutdown);

    Ok(())
}

// ========== 单元测试 ==========

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::queue::WaitStrategy as QueueWaitStrategy;

    #[test]
    fn test_boot_default() {
        let context = boot_default().unwrap();
        assert!(context.is_running());
        assert!(!context.is_shutdown());
    }

    #[test]
    fn test_boot_with_capacity() {
        let context = boot_with_capacity(50, 256, 256).unwrap();
        assert_eq!(context.pool().stats().capacity(), 50);
        assert_eq!(context.rxq().capacity(), 256);
        assert_eq!(context.txq().capacity(), 256);
    }

    #[test]
    fn test_boot_shutdown() {
        let mut context = boot_default().unwrap();
        assert!(context.is_running());

        shutdown(&mut context).unwrap();
        assert!(context.is_shutdown());
    }

    #[test]
    fn test_boot_invalid_config() {
        let config = SystemConfig {
            rxq_capacity: 1,  // 小于最小值
            ..Default::default()
        };

        let result = boot(config);
        assert!(result.is_err());
    }

    #[test]
    fn test_context_status() {
        let context = boot_default().unwrap();
        let status = context.status();

        assert!(status.state.is_running());
        assert_eq!(status.rxq_capacity, 1024);
        assert_eq!(status.txq_capacity, 1024);
    }

    #[test]
    fn test_preset_high_throughput() {
        let context = boot(high_throughput()).unwrap();
        assert_eq!(context.rxq().capacity(), 8192);
        assert_eq!(context.txq().capacity(), 8192);
    }

    #[test]
    fn test_preset_low_latency() {
        let context = boot(low_latency()).unwrap();
        assert_eq!(context.rxq().capacity(), 256);
        assert_eq!(context.txq().capacity(), 256);
    }

    #[test]
    fn test_preset_memory_constrained() {
        let context = boot(memory_constrained()).unwrap();
        assert_eq!(context.rxq().capacity(), 256);
        assert_eq!(context.txq().capacity(), 256);
    }
}
