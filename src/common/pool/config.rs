// src/common/pool/config.rs
//
// 池配置和策略定义

use std::time::Duration;

/// 分配策略
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AllocStrategy {
    /// 先进先出（从头部获取，归还时追加到尾部）
    Fifo,

    /// 后进先出（从尾部获取，归还时追加到尾部）
    Lifo,

    /// 随机获取（用于减少锁竞争热点）
    Random,

    /// 优先分配最近使用的（利用 CPU 缓存局部性）
    Recent,
}

/// 等待策略
#[derive(Debug, Clone, Copy)]
pub enum WaitStrategy {
    /// 立即返回失败
    Immediate,

    /// 自旋等待指定次数
    Spin(usize),

    /// 让出 CPU 时间片
    Yield,

    /// 等待指定超时时间
    Timeout(Duration),

    /// 无限等待
    Blocking,
}

/// 池配置
#[derive(Debug, Clone)]
pub struct PoolConfig {
    /// 池容量（最大对象数量）
    pub capacity: usize,

    /// 初始容量（池创建时预分配的对象数）
    pub initial_capacity: usize,

    /// 分配策略
    pub alloc_strategy: AllocStrategy,

    /// 等待策略（当池为空时）
    pub wait_strategy: WaitStrategy,

    /// 是否自动扩展（超出容量时是否允许临时分配）
    pub allow_overflow: bool,
}

impl Default for PoolConfig {
    fn default() -> Self {
        Self {
            capacity: 100,
            initial_capacity: 10,
            alloc_strategy: AllocStrategy::Fifo,
            wait_strategy: WaitStrategy::Timeout(Duration::from_millis(100)),
            allow_overflow: false,
        }
    }
}

// ========== 单元测试 ==========

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = PoolConfig::default();
        assert_eq!(config.capacity, 100);
        assert_eq!(config.initial_capacity, 10);
        assert_eq!(config.alloc_strategy, AllocStrategy::Fifo);
        assert!(!config.allow_overflow);
    }

    #[test]
    fn test_alloc_strategy_copy() {
        let s = AllocStrategy::Lifo;
        let s2 = s;
        assert_eq!(s, s2);
    }

    #[test]
    fn test_alloc_strategy_eq() {
        assert_eq!(AllocStrategy::Fifo, AllocStrategy::Fifo);
        assert_ne!(AllocStrategy::Fifo, AllocStrategy::Lifo);
    }
}
