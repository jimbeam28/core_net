// src/common/pool/error.rs
//
// 池错误类型定义

use std::fmt;
use std::time::Duration;

/// 池错误
#[derive(Debug)]
pub enum PoolError {
    /// 池已空（无可用对象）
    Empty,

    /// 池已满（归还时超出容量）
    Full,

    /// 池已关闭
    Shutdown,

    /// 超时
    Timeout(Duration),

    /// 配置错误
    InvalidConfig(String),

    /// 其他错误
    Other(String),
}

impl fmt::Display for PoolError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PoolError::Empty => write!(f, "池已空，无可用对象"),
            PoolError::Full => write!(f, "池已满，无法归还更多对象"),
            PoolError::Shutdown => write!(f, "池已关闭"),
            PoolError::Timeout(d) => write!(f, "获取对象超时: {:?}", d),
            PoolError::InvalidConfig(msg) => write!(f, "配置错误: {}", msg),
            PoolError::Other(msg) => write!(f, "其他错误: {}", msg),
        }
    }
}

impl std::error::Error for PoolError {}

// ========== 单元测试 ==========

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        assert_eq!(format!("{}", PoolError::Empty), "池已空，无可用对象");
        assert_eq!(format!("{}", PoolError::Full), "池已满，无法归还更多对象");
        assert_eq!(format!("{}", PoolError::Shutdown), "池已关闭");
    }
}
