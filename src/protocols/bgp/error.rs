// src/protocols/bgp/error.rs
//
// BGP 错误类型定义

use std::fmt;

/// BGP 错误类型
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BgpError {
    /// 消息长度无效
    InvalidMessageLength(String),

    /// 消息类型无效
    InvalidMessageType(u8),

    /// Marker 不匹配
    InvalidMarker,

    /// 版本不支持
    UnsupportedVersion(u8),

    /// BGP Identifier 冲突
    BgpIdentifierConflict,

    /// Hold Time 无效
    InvalidHoldTime(u16),

    /// 缺少必须的路径属性
    MissingRequiredAttribute(String),

    /// 路径属性无效
    InvalidPathAttribute(String),

    /// AS_PATH 检测到环路
    AsPathLoop,

    /// NEXT_HOP 不可达
    UnreachableNextHop,

    /// 对等体状态错误
    InvalidPeerState(String),

    /// 连接已关闭
    ConnectionClosed,

    /// 连接超时
    ConnectionTimeout,

    /// 缓冲区空间不足
    BufferSpaceInsufficient,

    /// 表已满
    TableFull,

    /// 其他错误
    Other(String),
}

impl fmt::Display for BgpError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BgpError::InvalidMessageLength(msg) => write!(f, "Invalid message length: {}", msg),
            BgpError::InvalidMessageType(t) => write!(f, "Invalid message type: {}", t),
            BgpError::InvalidMarker => write!(f, "Invalid marker"),
            BgpError::UnsupportedVersion(v) => write!(f, "Unsupported BGP version: {}", v),
            BgpError::BgpIdentifierConflict => write!(f, "BGP identifier conflict"),
            BgpError::InvalidHoldTime(t) => write!(f, "Invalid hold time: {}", t),
            BgpError::MissingRequiredAttribute(attr) => write!(f, "Missing required attribute: {}", attr),
            BgpError::InvalidPathAttribute(attr) => write!(f, "Invalid path attribute: {}", attr),
            BgpError::AsPathLoop => write!(f, "AS_PATH loop detected"),
            BgpError::UnreachableNextHop => write!(f, "Unreachable next hop"),
            BgpError::InvalidPeerState(s) => write!(f, "Invalid peer state: {}", s),
            BgpError::ConnectionClosed => write!(f, "Connection closed"),
            BgpError::ConnectionTimeout => write!(f, "Connection timeout"),
            BgpError::BufferSpaceInsufficient => write!(f, "Insufficient buffer space"),
            BgpError::TableFull => write!(f, "Table full"),
            BgpError::Other(msg) => write!(f, "BGP error: {}", msg),
        }
    }
}

impl std::error::Error for BgpError {}

/// BGP Result 类型
pub type Result<T> = std::result::Result<T, BgpError>;
