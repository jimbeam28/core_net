// src/protocols/tcp/error.rs
//
// TCP 错误类型定义

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
