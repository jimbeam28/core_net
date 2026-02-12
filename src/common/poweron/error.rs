// src/common/poweron/error.rs
//
// 上电启动模块错误类型定义

use std::fmt;

/// 上电启动模块错误类型
#[derive(Debug)]
pub enum PowerOnError {
    /// 配置无效
    InvalidConfig(String),

    /// 对象池创建失败
    PoolCreationFailed(String),

    /// 队列创建失败
    QueueCreationFailed(String),

    /// 系统已关闭
    SystemShutdown,

    /// 其他错误
    Other(String),
}

impl fmt::Display for PowerOnError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PowerOnError::InvalidConfig(msg) => {
                write!(f, "配置无效: {}", msg)
            }
            PowerOnError::PoolCreationFailed(msg) => {
                write!(f, "对象池创建失败: {}", msg)
            }
            PowerOnError::QueueCreationFailed(msg) => {
                write!(f, "队列创建失败: {}", msg)
            }
            PowerOnError::SystemShutdown => {
                write!(f, "系统已关闭")
            }
            PowerOnError::Other(msg) => {
                write!(f, "其他错误: {}", msg)
            }
        }
    }
}

impl std::error::Error for PowerOnError {}

// ========== 单元测试 ==========

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = PowerOnError::InvalidConfig("test".to_string());
        assert_eq!(err.to_string(), "配置无效: test");

        let err = PowerOnError::PoolCreationFailed("pool error".to_string());
        assert_eq!(err.to_string(), "对象池创建失败: pool error");

        let err = PowerOnError::QueueCreationFailed("queue error".to_string());
        assert_eq!(err.to_string(), "队列创建失败: queue error");

        let err = PowerOnError::SystemShutdown;
        assert_eq!(err.to_string(), "系统已关闭");

        let err = PowerOnError::Other("other error".to_string());
        assert_eq!(err.to_string(), "其他错误: other error");
    }
}
