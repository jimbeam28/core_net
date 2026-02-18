//! 测试框架错误类型定义

use std::fmt;

/// 测试框架错误
#[derive(Debug)]
pub enum HarnessError {
    /// 接口相关错误
    InterfaceError(String),

    /// 队列操作错误
    QueueError(String),

    /// 调度器错误
    SchedulerError(String),

    /// 全局状态错误
    GlobalStateError(String),

    /// Mutex 毒化错误（新增）
    MutexPoisonedError(String),
}

impl fmt::Display for HarnessError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HarnessError::InterfaceError(msg) => write!(f, "接口错误: {}", msg),
            HarnessError::QueueError(msg) => write!(f, "队列错误: {}", msg),
            HarnessError::SchedulerError(msg) => write!(f, "调度器错误: {}", msg),
            HarnessError::GlobalStateError(msg) => write!(f, "全局状态错误: {}", msg),
            HarnessError::MutexPoisonedError(msg) => write!(f, "Mutex 毒化错误: {}", msg),
        }
    }
}

impl std::error::Error for HarnessError {}

/// 测试框架结果类型
pub type HarnessResult<T> = Result<T, HarnessError>;

// ========== 错误转换 ==========

/// 从 InterfaceError 转换
impl From<crate::interface::InterfaceError> for HarnessError {
    fn from(err: crate::interface::InterfaceError) -> Self {
        HarnessError::InterfaceError(err.to_string())
    }
}

/// 从 ScheduleError 转换
impl From<crate::scheduler::ScheduleError> for HarnessError {
    fn from(err: crate::scheduler::ScheduleError) -> Self {
        HarnessError::SchedulerError(err.to_string())
    }
}

/// 从 QueueError 转换
impl From<crate::common::QueueError> for HarnessError {
    fn from(err: crate::common::QueueError) -> Self {
        HarnessError::QueueError(format!("{:?}", err))
    }
}
