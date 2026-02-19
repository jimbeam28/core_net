// src/protocols/ip/error.rs
//
// IP 协议错误类型定义

use std::fmt;

/// IP 协议错误
#[derive(Debug, Clone, PartialEq)]
pub enum IpError {
    /// 数据包长度不足
    InsufficientPacketLength {
        expected: usize,
        actual: usize,
    },

    /// IP 头部长度无效
    InvalidHeaderLength { ihl: u8 },

    /// IP 版本不支持
    UnsupportedVersion { version: u8 },

    /// 校验和错误
    ChecksumError {
        expected: u16,
        actual: u16,
    },

    /// 解析错误
    ParseError(String),

    /// 封装错误
    EncapError(String),

    /// 协议字段无效
    InvalidProtocol { protocol: u8 },
}

impl fmt::Display for IpError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            IpError::InsufficientPacketLength { expected, actual } => {
                write!(f, "IP数据包长度不足: 期望 {} 实际 {}", expected, actual)
            }
            IpError::InvalidHeaderLength { ihl } => {
                write!(f, "IP头部长度无效: IHL={}", ihl)
            }
            IpError::UnsupportedVersion { version } => {
                write!(f, "不支持的IP版本: {}", version)
            }
            IpError::ChecksumError { expected, actual } => {
                write!(f, "IP校验和错误: 期望 0x{:04x} 实际 0x{:04x}", expected, actual)
            }
            IpError::ParseError(msg) => {
                write!(f, "IP解析错误: {}", msg)
            }
            IpError::EncapError(msg) => {
                write!(f, "IP封装错误: {}", msg)
            }
            IpError::InvalidProtocol { protocol } => {
                write!(f, "无效的IP协议字段: {}", protocol)
            }
        }
    }
}

impl std::error::Error for IpError {}
