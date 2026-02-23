// src/protocols/icmpv6/types.rs
//
// ICMPv6 消息类型和代码定义
// RFC 4443: ICMPv6 规范
// RFC 4861: 邻居发现规范

use std::fmt;

// ========== ICMPv6 协议常量 ==========

/// ICMPv6 协议号 (IPv6 Next Header)
pub const IPPROTO_ICMPV6: u8 = 58;

/// ICMPv6 报文最小长度
pub const ICMPV6_MIN_LEN: usize = 8;

/// ICMPv6 伪头部长度
pub const ICMPV6_PSEUDO_HEADER_LEN: usize = 40;

// ========== ICMPv6 消息类型 ==========

/// ICMPv6 消息类型枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Icmpv6Type {
    // ========== 错误消息 (0-127) ==========

    /// Destination Unreachable
    DestinationUnreachable = 1,

    /// Packet Too Big
    PacketTooBig = 2,

    /// Time Exceeded
    TimeExceeded = 3,

    /// Parameter Problem
    ParameterProblem = 4,

    // ========== 信息消息 (128-255) ==========

    /// Echo Request
    EchoRequest = 128,

    /// Echo Reply
    EchoReply = 129,

    /// Multicast Listener Query
    MldQuery = 130,

    /// Multicast Listener Report
    MldReport = 131,

    /// Multicast Listener Done
    MldDone = 132,

    /// Router Solicitation
    RouterSolicitation = 133,

    /// Router Advertisement
    RouterAdvertisement = 134,

    /// Neighbor Solicitation
    NeighborSolicitation = 135,

    /// Neighbor Advertisement
    NeighborAdvertisement = 136,

    /// Redirect
    Redirect = 137,
}

impl Icmpv6Type {
    /// 从 u8 解析 ICMPv6 类型
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            1 => Some(Icmpv6Type::DestinationUnreachable),
            2 => Some(Icmpv6Type::PacketTooBig),
            3 => Some(Icmpv6Type::TimeExceeded),
            4 => Some(Icmpv6Type::ParameterProblem),
            128 => Some(Icmpv6Type::EchoRequest),
            129 => Some(Icmpv6Type::EchoReply),
            130 => Some(Icmpv6Type::MldQuery),
            131 => Some(Icmpv6Type::MldReport),
            132 => Some(Icmpv6Type::MldDone),
            133 => Some(Icmpv6Type::RouterSolicitation),
            134 => Some(Icmpv6Type::RouterAdvertisement),
            135 => Some(Icmpv6Type::NeighborSolicitation),
            136 => Some(Icmpv6Type::NeighborAdvertisement),
            137 => Some(Icmpv6Type::Redirect),
            _ => None,
        }
    }

    /// 获取类型的 u8 值
    pub fn as_u8(self) -> u8 {
        self as u8
    }

    /// 判断是否为错误消息
    pub fn is_error_message(self) -> bool {
        (self as u8) < 128
    }

    /// 判断是否为信息消息
    pub fn is_informational(self) -> bool {
        (self as u8) >= 128
    }
}

impl fmt::Display for Icmpv6Type {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Icmpv6Type::DestinationUnreachable => write!(f, "Destination Unreachable"),
            Icmpv6Type::PacketTooBig => write!(f, "Packet Too Big"),
            Icmpv6Type::TimeExceeded => write!(f, "Time Exceeded"),
            Icmpv6Type::ParameterProblem => write!(f, "Parameter Problem"),
            Icmpv6Type::EchoRequest => write!(f, "Echo Request"),
            Icmpv6Type::EchoReply => write!(f, "Echo Reply"),
            Icmpv6Type::MldQuery => write!(f, "MLD Query"),
            Icmpv6Type::MldReport => write!(f, "MLD Report"),
            Icmpv6Type::MldDone => write!(f, "MLD Done"),
            Icmpv6Type::RouterSolicitation => write!(f, "Router Solicitation"),
            Icmpv6Type::RouterAdvertisement => write!(f, "Router Advertisement"),
            Icmpv6Type::NeighborSolicitation => write!(f, "Neighbor Solicitation"),
            Icmpv6Type::NeighborAdvertisement => write!(f, "Neighbor Advertisement"),
            Icmpv6Type::Redirect => write!(f, "Redirect"),
        }
    }
}

// ========== Destination Unreachable 代码 ==========

/// Destination Unreachable 代码
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum DestUnreachableCode {
    /// No route to destination
    NoRoute = 0,

    /// Communication administratively prohibited
    AdminProhibited = 1,

    /// Beyond scope of source address
    BeyondScope = 2,

    /// Address unreachable
    AddressUnreachable = 3,

    /// Port unreachable
    PortUnreachable = 4,

    /// Source address failed ingress/egress policy
    SourcePolicyFailed = 5,

    /// Reject route to destination
    RejectRoute = 6,
}

impl DestUnreachableCode {
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0 => Some(DestUnreachableCode::NoRoute),
            1 => Some(DestUnreachableCode::AdminProhibited),
            2 => Some(DestUnreachableCode::BeyondScope),
            3 => Some(DestUnreachableCode::AddressUnreachable),
            4 => Some(DestUnreachableCode::PortUnreachable),
            5 => Some(DestUnreachableCode::SourcePolicyFailed),
            6 => Some(DestUnreachableCode::RejectRoute),
            _ => None,
        }
    }

    pub fn as_u8(self) -> u8 {
        self as u8
    }
}

// ========== Time Exceeded 代码 ==========

/// Time Exceeded 代码
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum TimeExceededCode {
    /// Hop limit exceeded in transit
    HopLimitExceeded = 0,

    /// Fragment reassembly time exceeded
    ReassemblyTimeout = 1,
}

impl TimeExceededCode {
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0 => Some(TimeExceededCode::HopLimitExceeded),
            1 => Some(TimeExceededCode::ReassemblyTimeout),
            _ => None,
        }
    }

    pub fn as_u8(self) -> u8 {
        self as u8
    }
}

// ========== Parameter Problem 代码 ==========

/// Parameter Problem 代码
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ParameterProblemCode {
    /// Erroneous header field
    HeaderField = 0,

    /// Unrecognized Next Header type
    UnrecognizedNextHeader = 1,

    /// Unrecognized IPv6 option
    UnrecognizedOption = 2,
}

impl ParameterProblemCode {
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0 => Some(ParameterProblemCode::HeaderField),
            1 => Some(ParameterProblemCode::UnrecognizedNextHeader),
            2 => Some(ParameterProblemCode::UnrecognizedOption),
            _ => None,
        }
    }

    pub fn as_u8(self) -> u8 {
        self as u8
    }
}

// ========== ICMPv6 选项类型 ==========

/// ICMPv6 选项类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Icmpv6OptionType {
    /// Source Link-Layer Address
    SourceLinkLayerAddr = 1,

    /// Target Link-Layer Address
    TargetLinkLayerAddr = 2,

    /// Prefix Information
    PrefixInfo = 3,

    /// Redirected Header
    RedirectedHeader = 4,

    /// MTU
    Mtu = 5,

    /// Route Information (RFC 4191)
    RouteInfo = 24,

    /// Recursive DNS Server (RFC 8106)
    RecursiveDns = 25,
}

impl Icmpv6OptionType {
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            1 => Some(Icmpv6OptionType::SourceLinkLayerAddr),
            2 => Some(Icmpv6OptionType::TargetLinkLayerAddr),
            3 => Some(Icmpv6OptionType::PrefixInfo),
            4 => Some(Icmpv6OptionType::RedirectedHeader),
            5 => Some(Icmpv6OptionType::Mtu),
            24 => Some(Icmpv6OptionType::RouteInfo),
            25 => Some(Icmpv6OptionType::RecursiveDns),
            _ => None,
        }
    }

    pub fn as_u8(self) -> u8 {
        self as u8
    }
}

// ========== 邻居缓存状态 ==========

/// 邻居缓存条目状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum NeighborCacheState {
    /// 地址解析进行中
    Incomplete = 0,

    /// 邻居可达
    Reachable = 1,

    /// 邻居可能不可达
    Stale = 2,

    /// 延迟发送 NS
    Delay = 3,

    /// 正在探测
    Probe = 4,

    /// 永久条目
    Permanent = 5,
}

impl NeighborCacheState {
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0 => Some(NeighborCacheState::Incomplete),
            1 => Some(NeighborCacheState::Reachable),
            2 => Some(NeighborCacheState::Stale),
            3 => Some(NeighborCacheState::Delay),
            4 => Some(NeighborCacheState::Probe),
            5 => Some(NeighborCacheState::Permanent),
            _ => None,
        }
    }

    pub fn as_u8(self) -> u8 {
        self as u8
    }
}

impl fmt::Display for NeighborCacheState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NeighborCacheState::Incomplete => write!(f, "INCOMPLETE"),
            NeighborCacheState::Reachable => write!(f, "REACHABLE"),
            NeighborCacheState::Stale => write!(f, "STALE"),
            NeighborCacheState::Delay => write!(f, "DELAY"),
            NeighborCacheState::Probe => write!(f, "PROBE"),
            NeighborCacheState::Permanent => write!(f, "PERMANENT"),
        }
    }
}

// ========== 组播地址 ==========

/// 所有节点的组播地址 ff02::1
pub const ALL_NODES_MULTICAST: [u8; 16] = [
    0xff, 0x02, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x01,
];

/// 所有路由器的组播地址 ff02::2
pub const ALL_ROUTERS_MULTICAST: [u8; 16] = [
    0xff, 0x02, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x02,
];

/// 被请求节点组播地址前缀 (ff02::1:ff00:0/104)
pub const SOLICITED_NODE_MULTICAST_PREFIX: [u8; 13] = [
    0xff, 0x02, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x01,
    0xff,
];

/// 计算被请求节点组播地址
///
/// # 参数
/// - target_addr: 目标单播/任意播地址
///
/// # 返回
/// - 被请求节点组播地址
pub fn solicited_node_multicast(target_addr: &[u8; 16]) -> [u8; 16] {
    let mut addr = [0u8; 16];
    addr[..13].copy_from_slice(&SOLICITED_NODE_MULTICAST_PREFIX);
    addr[13..].copy_from_slice(&target_addr[13..]);
    addr
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_icmpv6_type_from_u8() {
        assert_eq!(Icmpv6Type::from_u8(1), Some(Icmpv6Type::DestinationUnreachable));
        assert_eq!(Icmpv6Type::from_u8(128), Some(Icmpv6Type::EchoRequest));
        assert_eq!(Icmpv6Type::from_u8(129), Some(Icmpv6Type::EchoReply));
        assert_eq!(Icmpv6Type::from_u8(135), Some(Icmpv6Type::NeighborSolicitation));
        assert_eq!(Icmpv6Type::from_u8(136), Some(Icmpv6Type::NeighborAdvertisement));
        assert_eq!(Icmpv6Type::from_u8(255), None);
    }

    #[test]
    fn test_icmpv6_type_properties() {
        assert!(Icmpv6Type::DestinationUnreachable.is_error_message());
        assert!(Icmpv6Type::PacketTooBig.is_error_message());
        assert!(Icmpv6Type::EchoRequest.is_informational());
        assert!(Icmpv6Type::EchoReply.is_informational());
    }

    #[test]
    fn test_dest_unreachable_code() {
        assert_eq!(DestUnreachableCode::from_u8(0), Some(DestUnreachableCode::NoRoute));
        assert_eq!(DestUnreachableCode::from_u8(4), Some(DestUnreachableCode::PortUnreachable));
        assert_eq!(DestUnreachableCode::from_u8(7), None);
    }

    #[test]
    fn test_neighbor_cache_state() {
        assert_eq!(NeighborCacheState::from_u8(0), Some(NeighborCacheState::Incomplete));
        assert_eq!(NeighborCacheState::from_u8(1), Some(NeighborCacheState::Reachable));
        assert_eq!(NeighborCacheState::from_u8(2), Some(NeighborCacheState::Stale));
    }

    #[test]
    fn test_solicited_node_multicast() {
        let target = [0x20, 0x01, 0x0d, 0xb8, 0x00, 0x00, 0x00, 0x00,
                      0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x02, 0x03];
        let mc = solicited_node_multicast(&target);
        // 被请求节点组播地址应该以 ff02::1:ffXX:XXXX 结尾
        assert_eq!(mc[0], 0xff);
        assert_eq!(mc[1], 0x02);
        // 最后 3 字节应该与目标地址相同
        assert_eq!(mc[13..], target[13..]);
    }
}
