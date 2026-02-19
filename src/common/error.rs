// 错误类型定义
use std::fmt;

/// CoreNet核心错误类型
#[derive(Debug, Clone, PartialEq)]
pub enum CoreError {
    // Buffer相关错误
    BufferOverflow,
    BufferUnderflow,
    InvalidLength { expected: usize, actual: usize },

    // 解析相关错误
    ParseError(String),
    InvalidProtocol(String),

    // 队列相关错误
    QueueFull,
    QueueEmpty,

    // Packet相关错误
    InvalidPacket(String),
    UnsupportedProtocol(String),
    InvalidOffset { offset: usize, max: usize },

    // 通用错误
    Other(String),
}

impl CoreError {
    pub fn parse_error(msg: impl Into<String>) -> Self {
        CoreError::ParseError(msg.into())
    }

    pub fn invalid_protocol(protocol: impl Into<String>) -> Self {
        CoreError::InvalidProtocol(protocol.into())
    }

    pub fn invalid_packet(msg: impl Into<String>) -> Self {
        CoreError::InvalidPacket(msg.into())
    }

    pub fn unsupported_protocol(protocol: impl Into<String>) -> Self {
        CoreError::UnsupportedProtocol(protocol.into())
    }
}

impl fmt::Display for CoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CoreError::BufferOverflow => write!(f, "Buffer溢出：写入数据超过容量"),
            CoreError::BufferUnderflow => write!(f, "Buffer下溢：读取数据超过实际长度"),
            CoreError::InvalidLength { expected, actual } => write!(f, "无效长度：预期{}，实际{}", expected, actual),
            CoreError::ParseError(msg) => write!(f, "解析错误：{}", msg),
            CoreError::InvalidProtocol(proto) => write!(f, "无效协议：{}", proto),
            CoreError::QueueFull => write!(f, "队列已满：无法插入更多元素"),
            CoreError::QueueEmpty => write!(f, "队列为空：无法获取元素"),
            CoreError::InvalidPacket(msg) => write!(f, "无效报文：{}", msg),
            CoreError::UnsupportedProtocol(proto) => write!(f, "不支持的协议：{}", proto),
            CoreError::InvalidOffset { offset, max } => write!(f, "位置越界：offset={}，max={}", offset, max),
            CoreError::Other(msg) => write!(f, "其他错误：{}", msg),
        }
    }
}

impl std::error::Error for CoreError {}

/// Result类型别名：使用CoreError作为错误类型
pub type Result<T> = std::result::Result<T, CoreError>;
