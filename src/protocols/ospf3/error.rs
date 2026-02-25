// src/protocols/ospf3/error.rs
//
// OSPFv3 错误类型定义

use std::fmt;

/// OSPFv3 错误类型
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Ospfv3Error {
    /// 数据包太短
    PacketTooShort { expected: usize, actual: usize },

    /// 无效的报文类型
    InvalidPacketType { packet_type: u8 },

    /// 版本不匹配
    VersionMismatch { expected: u8, actual: u8 },

    /// 解析错误
    ParseError { field: String, details: String },

    /// 无效的 LSA 类型
    InvalidLsaType { lsa_type: u32 },

    /// 校验和错误
    ChecksumError,

    /// Hello 间隔不匹配
    HelloMismatch { expected: u16, received: u16 },

    /// 死亡间隔不匹配
    DeadIntervalMismatch { expected: u32, received: u32 },

    /// 其他错误
    Other { reason: String },
}

impl fmt::Display for Ospfv3Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Ospfv3Error::PacketTooShort { expected, actual } => {
                write!(f, "数据包太短: 期望 {} 字节, 实际 {} 字节", expected, actual)
            }
            Ospfv3Error::InvalidPacketType { packet_type } => {
                write!(f, "无效的报文类型: {}", packet_type)
            }
            Ospfv3Error::VersionMismatch { expected, actual } => {
                write!(f, "版本不匹配: 期望 {}, 实际 {}", expected, actual)
            }
            Ospfv3Error::ParseError { field, details } => {
                write!(f, "解析字段 {} 失败: {}", field, details)
            }
            Ospfv3Error::InvalidLsaType { lsa_type } => {
                write!(f, "无效的 LSA 类型: 0x{:08x}", lsa_type)
            }
            Ospfv3Error::ChecksumError => {
                write!(f, "校验和错误")
            }
            Ospfv3Error::HelloMismatch { expected, received } => {
                write!(f, "Hello 间隔不匹配: 期望 {}, 收到 {}", expected, received)
            }
            Ospfv3Error::DeadIntervalMismatch { expected, received } => {
                write!(f, "死亡间隔不匹配: 期望 {}, 收到 {}", expected, received)
            }
            Ospfv3Error::Other { reason } => {
                write!(f, "其他错误: {}", reason)
            }
        }
    }
}

impl std::error::Error for Ospfv3Error {}

/// OSPFv3 结果类型
pub type Ospfv3Result<T> = Result<T, Ospfv3Error>;

impl Ospfv3Error {
    pub fn packet_too_short(expected: usize, actual: usize) -> Self {
        Ospfv3Error::PacketTooShort { expected, actual }
    }

    pub fn invalid_packet_type(packet_type: u8) -> Self {
        Ospfv3Error::InvalidPacketType { packet_type }
    }

    pub fn parse_error(field: &str, details: String) -> Self {
        Ospfv3Error::ParseError {
            field: field.to_string(),
            details,
        }
    }
}
