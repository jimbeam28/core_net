// src/protocols/icmp/types.rs
//
// ICMP 消息类型和代码定义
// 包含 ICMPv4 和 ICMPv6 的类型

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

// ========== ICMPv6 类型常量 ==========

/// ICMPv6 类型：Destination Unreachable
pub const ICMPV6_TYPE_DEST_UNREACHABLE: u8 = 1;

/// ICMPv6 类型：Packet Too Big
pub const ICMPV6_TYPE_PACKET_TOO_BIG: u8 = 2;

/// ICMPv6 类型：Time Exceeded
pub const ICMPV6_TYPE_TIME_EXCEEDED: u8 = 3;

/// ICMPv6 类型：Parameter Problem
pub const ICMPV6_TYPE_PARAMETER_PROBLEM: u8 = 4;

/// ICMPv6 类型：Echo Request
pub const ICMPV6_TYPE_ECHO_REQUEST: u8 = 128;

/// ICMPv6 类型：Echo Reply
pub const ICMPV6_TYPE_ECHO_REPLY: u8 = 129;

// ========== ICMPv6 类型枚举 ==========

/// ICMPv6 消息类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IcmpV6Type {
    /// Destination Unreachable
    DestinationUnreachable = 1,

    /// Packet Too Big
    PacketTooBig = 2,

    /// Time Exceeded
    TimeExceeded = 3,

    /// Parameter Problem
    ParameterProblem = 4,

    /// Echo Request
    EchoRequest = 128,

    /// Echo Reply
    EchoReply = 129,
}

impl IcmpV6Type {
    /// 从 u8 解析 ICMPv6 类型
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            1 => Some(IcmpV6Type::DestinationUnreachable),
            2 => Some(IcmpV6Type::PacketTooBig),
            3 => Some(IcmpV6Type::TimeExceeded),
            4 => Some(IcmpV6Type::ParameterProblem),
            128 => Some(IcmpV6Type::EchoRequest),
            129 => Some(IcmpV6Type::EchoReply),
            _ => None,
        }
    }

    /// 获取类型的 u8 值
    pub fn as_u8(self) -> u8 {
        self as u8
    }
}

impl fmt::Display for IcmpV6Type {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            IcmpV6Type::DestinationUnreachable => write!(f, "Destination Unreachable"),
            IcmpV6Type::PacketTooBig => write!(f, "Packet Too Big"),
            IcmpV6Type::TimeExceeded => write!(f, "Time Exceeded"),
            IcmpV6Type::ParameterProblem => write!(f, "Parameter Problem"),
            IcmpV6Type::EchoRequest => write!(f, "Echo Request"),
            IcmpV6Type::EchoReply => write!(f, "Echo Reply"),
        }
    }
}

// ========== ICMPv6 Destination Unreachable 代码 ==========

/// ICMPv6 Destination Unreachable 代码
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IcmpV6DestUnreachableCode {
    /// 没有路由到达目标
    NoRouteToDestination = 0,

    /// 与目标的通信被管理策略禁止
    CommunicationProhibited = 1,

    /// 超出源地址范围
    BeyondScopeOfSourceAddress = 2,

    /// 地址不可达
    AddressUnreachable = 3,

    /// 端口不可达
    PortUnreachable = 4,

    /// 源地址失败入口策略
    SourceAddressFailedPolicy = 5,

    /// 拒绝路由到目标
    RejectRouteToDestination = 6,
}

impl IcmpV6DestUnreachableCode {
    /// 从 u8 解析代码
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0 => Some(IcmpV6DestUnreachableCode::NoRouteToDestination),
            1 => Some(IcmpV6DestUnreachableCode::CommunicationProhibited),
            2 => Some(IcmpV6DestUnreachableCode::BeyondScopeOfSourceAddress),
            3 => Some(IcmpV6DestUnreachableCode::AddressUnreachable),
            4 => Some(IcmpV6DestUnreachableCode::PortUnreachable),
            5 => Some(IcmpV6DestUnreachableCode::SourceAddressFailedPolicy),
            6 => Some(IcmpV6DestUnreachableCode::RejectRouteToDestination),
            _ => None,
        }
    }

    /// 获取代码的 u8 值
    pub fn as_u8(self) -> u8 {
        self as u8
    }
}

// ========== ICMPv6 Time Exceeded 代码 ==========

/// ICMPv6 Time Exceeded 代码
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IcmpV6TimeExceededCode {
    /// Hop Limit 超时
    HopLimitExceeded = 0,

    /// 分片重组超时
    FragmentReassemblyTimeout = 1,
}

impl IcmpV6TimeExceededCode {
    /// 从 u8 解析代码
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0 => Some(IcmpV6TimeExceededCode::HopLimitExceeded),
            1 => Some(IcmpV6TimeExceededCode::FragmentReassemblyTimeout),
            _ => None,
        }
    }

    /// 获取代码的 u8 值
    pub fn as_u8(self) -> u8 {
        self as u8
    }
}
