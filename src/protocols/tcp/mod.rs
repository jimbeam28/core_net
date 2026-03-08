// src/protocols/tcp/mod.rs
//
// TCP 协议模块（精简版）

mod config;
mod header;
mod segment;
mod tcb;
mod connection;
mod process;
mod socket;
mod socket_manager;

pub use config::{TcpConfig, TCP_CONFIG_DEFAULT};
pub use header::TcpHeader;
pub use segment::TcpSegment;
pub use tcb::{Tcb, TcpConnectionId, TcpState, SentSegment};
pub use connection::{TcpConnectionManager, TcpOption};
pub use process::{
    TcpProcessResult,
    process_tcp_packet,
    encapsulate_tcp_segment,
    create_syn,
    create_ack,
    create_fin,
    create_rst,
};
pub use socket::{TcpSocket, TcpEvent, TcpCallback};
pub use socket_manager::{TcpSocketManager, ConnectionTuple};

// ==================== constant.rs 内容 ====================

/// TCP 协议号（在 IP 协议字段中的值）
pub const IP_PROTO_TCP: u8 = 6;

/// TCP 头部最小大小（20 字节）
pub const TCP_MIN_HEADER_LEN: usize = 20;

/// TCP 头部最大大小（60 字节）
pub const TCP_MAX_HEADER_LEN: usize = 60;

/// TCP 头部最小数据偏移值（5，表示 20 字节）
pub const TCP_MIN_DATA_OFFSET: u8 = 5;

/// 知名端口号
pub mod well_known_ports {
    /// FTP 数据
    pub const FTP_DATA: u16 = 20;
    /// FTP 控制
    pub const FTP_CONTROL: u16 = 21;
    /// SSH
    pub const SSH: u16 = 22;
    /// Telnet
    pub const TELNET: u16 = 23;
    /// SMTP
    pub const SMTP: u16 = 25;
    /// DNS
    pub const DNS: u16 = 53;
    /// HTTP
    pub const HTTP: u16 = 80;
    /// HTTPS
    pub const HTTPS: u16 = 443;
}

/// TCP 标志位常量
pub mod flags {
    /// FIN 标志位（结束连接）
    pub const FIN: u8 = 0x01;
    /// SYN 标志位（同步序列号）
    pub const SYN: u8 = 0x02;
    /// RST 标志位（重置连接）
    pub const RST: u8 = 0x04;
    /// PSH 标志位（推送数据）
    pub const PSH: u8 = 0x08;
    /// ACK 标志位（确认号有效）
    pub const ACK: u8 = 0x10;
    /// URG 标志位（紧急指针有效）
    pub const URG: u8 = 0x20;
    /// ECE 标志位（ECN-Echo）
    pub const ECE: u8 = 0x40;
    /// CWR 标志位（拥塞窗口减少）
    pub const CWR: u8 = 0x80;
}

/// TCP 选项类型
pub mod option_kind {
    /// 行尾（选项结束）
    pub const END: u8 = 0;
    /// 无操作（填充）
    pub const NOP: u8 = 1;
    /// 最大分段大小（MSS）
    pub const MSS: u8 = 2;
    /// 窗口缩放
    pub const WINDOW_SCALE: u8 = 3;
    /// SACK 允许
    pub const SACK_PERMITTED: u8 = 4;
    /// SACK 选项
    pub const SACK: u8 = 5;
    /// 时间戳
    pub const TIMESTAMPS: u8 = 8;
}

// ==================== error.rs 内容 ====================

/// TCP 错误类型
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TcpError {
    /// 校验和错误
    ChecksumError,

    /// 序列号不在窗口内
    SequenceOutOfWindow {
        /// 收到的序列号
        received: u32,
        /// 期望的序列号
        expected: u32,
        /// 接收窗口大小
        window: u32,
    },

    /// 连接不存在
    ConnectionNotExist,

    /// 连接已关闭
    ConnectionClosed,

    /// 无效状态转换
    InvalidState {
        /// 当前状态
        current: String,
        /// 尝试转换到的状态
        target: String,
    },

    /// 缓冲区已满
    BufferFull,

    /// 重传次数超限
    RetransmitExceeded {
        /// 最大重传次数
        max_attempts: u32,
    },

    /// 连接超时
    ConnectionTimeout,

    /// 连接被重置
    ConnectionReset,

    /// 无效选项
    InvalidOption {
        /// 选项类型
        kind: u8,
    },

    /// 解析错误
    ParseError(String),

    /// 其他错误
    Other(String),
}

impl std::fmt::Display for TcpError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TcpError::ChecksumError => write!(f, "TCP 校验和错误"),
            TcpError::SequenceOutOfWindow { received, expected, window } => {
                write!(f, "序列号 {} 不在窗口内（期望 {}，窗口 {}）", received, expected, window)
            }
            TcpError::ConnectionNotExist => write!(f, "TCP 连接不存在"),
            TcpError::ConnectionClosed => write!(f, "TCP 连接已关闭"),
            TcpError::InvalidState { current, target } => {
                write!(f, "无效状态转换: {} -> {}", current, target)
            }
            TcpError::BufferFull => write!(f, "TCP 缓冲区已满"),
            TcpError::RetransmitExceeded { max_attempts } => {
                write!(f, "重传次数超限: {}", max_attempts)
            }
            TcpError::ConnectionTimeout => write!(f, "TCP 连接超时"),
            TcpError::ConnectionReset => write!(f, "TCP 连接被重置"),
            TcpError::InvalidOption { kind } => {
                write!(f, "无效 TCP 选项: kind={}", kind)
            }
            TcpError::ParseError(msg) => write!(f, "TCP 解析错误: {}", msg),
            TcpError::Other(msg) => write!(f, "TCP 错误: {}", msg),
        }
    }
}

impl std::error::Error for TcpError {}

impl From<crate::common::CoreError> for TcpError {
    fn from(err: crate::common::CoreError) -> Self {
        match err {
            crate::common::CoreError::ParseError(msg) => TcpError::ParseError(msg),
            crate::common::CoreError::InvalidPacket(msg) => TcpError::Other(msg),
            _ => TcpError::Other(format!("{:?}", err)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = TcpError::ChecksumError;
        assert_eq!(format!("{}", err), "TCP 校验和错误");

        let err = TcpError::SequenceOutOfWindow {
            received: 1000,
            expected: 500,
            window: 200,
        };
        assert!(format!("{}", err).contains("序列号"));
    }

    #[test]
    fn test_sequence_out_of_window() {
        let err = TcpError::SequenceOutOfWindow {
            received: 1000,
            expected: 500,
            window: 200,
        };
        let err_str = format!("{}", err);
        assert!(err_str.contains("1000"));
        assert!(err_str.contains("500"));
    }

    #[test]
    fn test_invalid_state() {
        let err = TcpError::InvalidState {
            current: "CLOSED".to_string(),
            target: "ESTABLISHED".to_string(),
        };
        let err_str = format!("{}", err);
        assert!(err_str.contains("CLOSED"));
        assert!(err_str.contains("ESTABLISHED"));
    }
}
