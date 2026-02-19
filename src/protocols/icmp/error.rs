// src/protocols/icmp/error.rs
//
// ICMP 协议错误类型定义

use std::fmt;

/// ICMP 协议错误
#[derive(Debug, Clone, PartialEq)]
pub enum IcmpError {
    /// 数据包长度不足
    InsufficientPacketLength {
        expected: usize,
        actual: usize,
    },

    /// 不支持的 ICMP 类型
    UnsupportedType { type_: u8 },

    /// 无效的 ICMP 代码
    InvalidCode { type_: u8, code: u8 },

    /// 校验和错误
    ChecksumError {
        expected: u16,
        actual: u16,
    },

    /// 解析错误
    ParseError(String),

    /// 封装错误
    EncapError(String),

    /// Echo 请求超时
    EchoTimeout {
        identifier: u16,
        sequence: u16,
    },

    /// Echo 管理器错误
    EchoManagerError(String),
}

impl fmt::Display for IcmpError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            IcmpError::InsufficientPacketLength { expected, actual } => {
                write!(f, "ICMP数据包长度不足: 期望 {} 实际 {}", expected, actual)
            }
            IcmpError::UnsupportedType { type_ } => {
                write!(f, "不支持的ICMP类型: {}", type_)
            }
            IcmpError::InvalidCode { type_, code } => {
                write!(f, "无效的ICMP代码: Type={} Code={}", type_, code)
            }
            IcmpError::ChecksumError { expected, actual } => {
                write!(f, "ICMP校验和错误: 期望 0x{:04x} 实际 0x{:04x}", expected, actual)
            }
            IcmpError::ParseError(msg) => {
                write!(f, "ICMP解析错误: {}", msg)
            }
            IcmpError::EncapError(msg) => {
                write!(f, "ICMP封装错误: {}", msg)
            }
            IcmpError::EchoTimeout { identifier, sequence } => {
                write!(f, "Echo请求超时: ID={} Seq={}", identifier, sequence)
            }
            IcmpError::EchoManagerError(msg) => {
                write!(f, "Echo管理器错误: {}", msg)
            }
        }
    }
}

impl std::error::Error for IcmpError {}
