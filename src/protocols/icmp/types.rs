// src/protocols/icmp/types.rs
//
// ICMP 消息类型和代码定义

use std::fmt;

// ========== ICMP 类型常量 ==========

/// ICMP 类型：Echo Reply
pub const ICMP_TYPE_ECHO_REPLY: u8 = 0;

/// ICMP 类型：Destination Unreachable
pub const ICMP_TYPE_DEST_UNREACHABLE: u8 = 3;

/// ICMP 类型：Echo Request
pub const ICMP_TYPE_ECHO_REQUEST: u8 = 8;

/// ICMP 类型：Time Exceeded
pub const ICMP_TYPE_TIME_EXCEEDED: u8 = 11;

// ========== ICMP 类型枚举 ==========

/// ICMP 消息类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IcmpType {
    /// Echo Reply (ping 响应)
    EchoReply = 0,

    /// Destination Unreachable (目标不可达)
    DestinationUnreachable = 3,

    /// Echo Request (ping 请求)
    EchoRequest = 8,

    /// Time Exceeded (超时)
    TimeExceeded = 11,
}

impl IcmpType {
    /// 从 u8 解析 ICMP 类型
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0 => Some(IcmpType::EchoReply),
            3 => Some(IcmpType::DestinationUnreachable),
            8 => Some(IcmpType::EchoRequest),
            11 => Some(IcmpType::TimeExceeded),
            _ => None,
        }
    }

    /// 获取类型的 u8 值
    pub fn as_u8(self) -> u8 {
        self as u8
    }
}

impl fmt::Display for IcmpType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            IcmpType::EchoReply => write!(f, "Echo Reply"),
            IcmpType::DestinationUnreachable => write!(f, "Destination Unreachable"),
            IcmpType::EchoRequest => write!(f, "Echo Request"),
            IcmpType::TimeExceeded => write!(f, "Time Exceeded"),
        }
    }
}

// ========== Destination Unreachable 代码 ==========

/// Destination Unreachable 代码
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DestUnreachableCode {
    /// 网络不可达
    NetworkUnreachable = 0,

    /// 主机不可达
    HostUnreachable = 1,

    /// 协议不可达
    ProtocolUnreachable = 2,

    /// 端口不可达
    PortUnreachable = 3,

    /// 需要分片但 DF 设置
    FragmentationNeeded = 4,

    /// 源路由失败
    SourceRouteFailed = 5,
}

impl DestUnreachableCode {
    /// 从 u8 解析 Destination Unreachable 代码
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0 => Some(DestUnreachableCode::NetworkUnreachable),
            1 => Some(DestUnreachableCode::HostUnreachable),
            2 => Some(DestUnreachableCode::ProtocolUnreachable),
            3 => Some(DestUnreachableCode::PortUnreachable),
            4 => Some(DestUnreachableCode::FragmentationNeeded),
            5 => Some(DestUnreachableCode::SourceRouteFailed),
            _ => None,
        }
    }

    /// 获取代码的 u8 值
    pub fn as_u8(self) -> u8 {
        self as u8
    }
}

// ========== Time Exceeded 代码 ==========

/// Time Exceeded 代码
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimeExceededCode {
    /// TTL 过期
    TtlExpired = 0,

    /// 分片重组超时
    FragmentReassemblyTimeout = 1,
}

impl TimeExceededCode {
    /// 从 u8 解析 Time Exceeded 代码
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0 => Some(TimeExceededCode::TtlExpired),
            1 => Some(TimeExceededCode::FragmentReassemblyTimeout),
            _ => None,
        }
    }

    /// 获取代码的 u8 值
    pub fn as_u8(self) -> u8 {
        self as u8
    }
}
