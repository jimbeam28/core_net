// src/protocols/icmpv6/error.rs
//
// ICMPv6 模块错误类型

use std::fmt;

/// ICMPv6 模块错误类型
#[derive(Debug, Clone, PartialEq)]
pub enum Icmpv6Error {
    /// 解析错误
    ParseError(String),

    /// 无效的报文
    InvalidPacket(String),

    /// 校验和错误
    ChecksumError,

    /// 不支持的消息类型
    UnsupportedMessageType(u8),

    /// 邻居缓存错误
    NeighborCacheError(String),

    /// 配置错误
    ConfigError(String),

    /// 处理超时
    Timeout,
}

impl fmt::Display for Icmpv6Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Icmpv6Error::ParseError(msg) => write!(f, "解析错误: {}", msg),
            Icmpv6Error::InvalidPacket(msg) => write!(f, "无效报文: {}", msg),
            Icmpv6Error::ChecksumError => write!(f, "校验和错误"),
            Icmpv6Error::UnsupportedMessageType(ty) => write!(f, "不支持的消息类型: {}", ty),
            Icmpv6Error::NeighborCacheError(msg) => write!(f, "邻居缓存错误: {}", msg),
            Icmpv6Error::ConfigError(msg) => write!(f, "配置错误: {}", msg),
            Icmpv6Error::Timeout => write!(f, "处理超时"),
        }
    }
}

impl std::error::Error for Icmpv6Error {}

/// ICMPv6 Result 类型
pub type Icmpv6Result<T> = Result<T, Icmpv6Error>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = Icmpv6Error::ParseError("test error".to_string());
        assert_eq!(format!("{}", err), "解析错误: test error");

        let err = Icmpv6Error::ChecksumError;
        assert_eq!(format!("{}", err), "校验和错误");

        let err = Icmpv6Error::UnsupportedMessageType(255);
        assert_eq!(format!("{}", err), "不支持的消息类型: 255");
    }
}
