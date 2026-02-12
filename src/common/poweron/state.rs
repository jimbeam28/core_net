// src/common/poweron/state.rs
//
// 系统状态定义

use std::fmt;
use crate::common::{PoolStatus, pool};

/// 系统运行状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SystemState {
    /// 运行中
    Running,

    /// 正在关闭
    ShuttingDown,

    /// 已关闭
    Shutdown,
}

impl fmt::Display for SystemState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SystemState::Running => write!(f, "运行中"),
            SystemState::ShuttingDown => write!(f, "正在关闭"),
            SystemState::Shutdown => write!(f, "已关闭"),
        }
    }
}

impl SystemState {
    /// 判断系统是否运行中
    pub fn is_running(&self) -> bool {
        matches!(self, SystemState::Running)
    }

    /// 判断系统是否已关闭
    pub fn is_shutdown(&self) -> bool {
        matches!(self, SystemState::Shutdown)
    }

    /// 判断系统是否正在关闭
    pub fn is_shutting_down(&self) -> bool {
        matches!(self, SystemState::ShuttingDown)
    }
}

/// 系统状态快照
#[derive(Debug, Clone)]
pub struct SystemStatus {
    /// 系统状态
    pub state: SystemState,

    /// 对象池状态
    pub pool_status: PoolStatus,

    /// 接收队列长度
    pub rxq_len: usize,

    /// 接收队列容量
    pub rxq_capacity: usize,

    /// 发送队列长度
    pub txq_len: usize,

    /// 发送队列容量
    pub txq_capacity: usize,
}

impl SystemStatus {
    /// 创建新的系统状态快照
    pub fn new(
        state: SystemState,
        pool_status: PoolStatus,
        rxq_len: usize,
        rxq_capacity: usize,
        txq_len: usize,
        txq_capacity: usize,
    ) -> Self {
        Self {
            state,
            pool_status,
            rxq_len,
            rxq_capacity,
            txq_len,
            txq_capacity,
        }
    }

    /// 获取接收队列使用率
    pub fn rxq_utilization(&self) -> f64 {
        if self.rxq_capacity == 0 {
            0.0
        } else {
            (self.rxq_len as f64) / (self.rxq_capacity as f64)
        }
    }

    /// 获取发送队列使用率
    pub fn txq_utilization(&self) -> f64 {
        if self.txq_capacity == 0 {
            0.0
        } else {
            (self.txq_len as f64) / (self.txq_capacity as f64)
        }
    }
}

impl fmt::Display for SystemStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "=== 系统状态 ===")?;
        writeln!(f, "状态: {}", self.state)?;
        writeln!(f, "对象池: {}空闲 / {}活跃 ({:.1}%)",
            self.pool_status.idle,
            self.pool_status.active,
            self.pool_status.utilization * 100.0)?;
        writeln!(f, "接收队列: {} / {} ({:.1}%)",
            self.rxq_len, self.rxq_capacity, self.rxq_utilization() * 100.0)?;
        writeln!(f, "发送队列: {} / {} ({:.1}%)",
            self.txq_len, self.txq_capacity, self.txq_utilization() * 100.0)?;
        Ok(())
    }
}

// ========== 单元测试 ==========

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_system_state() {
        assert!(SystemState::Running.is_running());
        assert!(!SystemState::Running.is_shutdown());
        assert!(!SystemState::Running.is_shutting_down());

        assert!(!SystemState::Shutdown.is_running());
        assert!(SystemState::Shutdown.is_shutdown());
        assert!(!SystemState::Shutdown.is_shutting_down());

        assert!(!SystemState::ShuttingDown.is_running());
        assert!(!SystemState::ShuttingDown.is_shutdown());
        assert!(SystemState::ShuttingDown.is_shutting_down());
    }

    #[test]
    fn test_system_status() {
        let pool_status = PoolStatus {
            idle: 50,
            active: 50,
            utilization: 0.5,
        };

        let status = SystemStatus::new(
            SystemState::Running,
            pool_status,
            256,
            1024,
            128,
            1024,
        );

        assert_eq!(status.rxq_len, 256);
        assert_eq!(status.txq_len, 128);
        assert!((status.rxq_utilization() - 0.25).abs() < 0.01);
        assert!((status.txq_utilization() - 0.125).abs() < 0.01);
    }
}
