// src/socket/error.rs
//
// Socket API 错误类型定义

use std::fmt;

/// Socket 错误类型
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SocketError {
    /// 无效的 Socket 描述符
    InvalidFd,

    /// 不支持的协议
    InvalidProtocol,

    /// Socket 已绑定
    AlreadyBound,

    /// Socket 未绑定
    NotBound,

    /// 地址已被占用
    AddrInUse,

    /// 地址不可用
    AddrNotAvailable,

    /// Socket 不是流式套接字
    NotStream,

    /// Socket 状态无效
    InvalidState,

    /// Socket 未监听
    NotListening,

    /// Socket 已连接
    AlreadyConnected,

    /// Socket 未连接
    NotConnected,

    /// 连接被拒绝
    ConnRefused,

    /// 连接超时
    ConnTimedOut,

    /// 连接被重置
    ConnReset,

    /// 非阻塞模式下操作会阻塞
    WouldBlock,

    /// 操作正在进行中
    InProgress,

    /// 操作被中断
    Interrupted,

    /// 缓冲区空间不足
    NoBufferSpace,

    /// Socket 表已满
    TableFull,

    /// 其他错误
    Other(String),
}

impl fmt::Display for SocketError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidFd => write!(f, "Invalid socket file descriptor"),
            Self::InvalidProtocol => write!(f, "Invalid protocol"),
            Self::AlreadyBound => write!(f, "Socket already bound"),
            Self::NotBound => write!(f, "Socket not bound"),
            Self::AddrInUse => write!(f, "Address already in use"),
            Self::AddrNotAvailable => write!(f, "Address not available"),
            Self::NotStream => write!(f, "Socket is not stream type"),
            Self::InvalidState => write!(f, "Invalid socket state"),
            Self::NotListening => write!(f, "Socket not listening"),
            Self::AlreadyConnected => write!(f, "Socket already connected"),
            Self::NotConnected => write!(f, "Socket not connected"),
            Self::ConnRefused => write!(f, "Connection refused"),
            Self::ConnTimedOut => write!(f, "Connection timed out"),
            Self::ConnReset => write!(f, "Connection reset"),
            Self::WouldBlock => write!(f, "Operation would block"),
            Self::InProgress => write!(f, "Operation in progress"),
            Self::Interrupted => write!(f, "Operation interrupted"),
            Self::NoBufferSpace => write!(f, "No buffer space available"),
            Self::TableFull => write!(f, "Socket table full"),
            Self::Other(msg) => write!(f, "{}", msg),
        }
    }
}

impl std::error::Error for SocketError {}

/// Socket 结果类型
pub type Result<T> = std::result::Result<T, SocketError>;
