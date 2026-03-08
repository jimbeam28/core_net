// src/protocols/bgp/message.rs
//
// BGP 报文结构定义（简化版）

use std::net::{IpAddr, Ipv4Addr};
use crate::common::addr::Ipv4Addr as CoreIpv4Addr;
use crate::protocols::bgp::BGP_MARKER_SIZE;

/// IP 前缀（用于 NLRI 和 Withdrawn Routes）
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct IpPrefix {
    /// IP 地址
    pub prefix: IpAddr,
    /// 前缀长度
    pub prefix_len: u8,
}

impl IpPrefix {
    /// 创建新的 IP 前缀
    pub fn new(prefix: IpAddr, prefix_len: u8) -> Self {
        Self { prefix, prefix_len }
    }

    /// 创建 IPv4 前缀
    pub fn ipv4(prefix: CoreIpv4Addr, prefix_len: u8) -> Self {
        Self {
            prefix: IpAddr::V4(Ipv4Addr::new(prefix.bytes[0], prefix.bytes[1], prefix.bytes[2], prefix.bytes[3])),
            prefix_len,
        }
    }
}

/// BGP 报文头部（所有 BGP 报文通用）
#[derive(Debug, Clone)]
pub struct BgpHeader {
    /// 同步标记（16 字节）
    pub marker: [u8; BGP_MARKER_SIZE],
    /// 报文总长度（包含头部）
    pub length: u16,
    /// 报文类型：1=OPEN, 2=UPDATE, 3=NOTIFICATION, 4=KEEPALIVE, 5=ROUTE-REFRESH
    pub msg_type: u8,
}

impl BgpHeader {
    /// 创建默认 Marker（全 1）
    pub fn default_marker() -> [u8; BGP_MARKER_SIZE] {
        [0xFF; BGP_MARKER_SIZE]
    }

    /// 创建新的 BGP 头部
    pub fn new(length: u16, msg_type: u8) -> Self {
        Self {
            marker: Self::default_marker(),
            length,
            msg_type,
        }
    }
}

/// BGP OPEN 报文
#[derive(Debug, Clone)]
pub struct BgpOpen {
    /// BGP 版本号（必须为 4）
    pub version: u8,
    /// 本地 AS 号
    pub my_as: u16,
    /// 保活时间（秒）
    pub hold_time: u16,
    /// BGP 标识符
    pub bgp_identifier: CoreIpv4Addr,
    /// 可选参数
    pub optional_parameters: Vec<OptionalParameter>,
}

/// 可选参数
#[derive(Debug, Clone)]
pub enum OptionalParameter {
    /// 认证信息
    Authentication {
        auth_code: u8,
        data: Vec<u8>,
    },
    /// 能力通告
    Capabilities {
        capabilities: Vec<BgpCapability>,
    },
}

/// BGP 能力类型
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BgpCapability {
    /// 多协议扩展
    MultiProtocol {
        afi: u16,
        safi: u8,
    },
    /// 路由刷新
    RouteRefresh,
    /// 支持 4 字节 AS 号
    FourOctetAsNumber {
        as_number: u32,
    },
    /// 支持 Capability 参数
    CapabilityNegotiation,
    /// 其他未知能力
    Unknown {
        code: u8,
        data: Vec<u8>,
    },
}

/// BGP UPDATE 报文
#[derive(Debug, Clone)]
pub struct BgpUpdate {
    /// 撤销的路由前缀列表
    pub withdrawn_routes: Vec<IpPrefix>,
    /// 路径属性
    pub path_attributes: Vec<PathAttribute>,
    /// 网络层可达性信息
    pub nlri: Vec<IpPrefix>,
}

/// 路径属性
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PathAttribute {
    /// ORIGIN：路由起源
    Origin { origin: u8 },
    /// AS_PATH：AS 路径
    AsPath { as_sequence: Vec<u32>, as_set: Vec<u32> },
    /// NEXT_HOP：下一跳 IP
    NextHop { next_hop: CoreIpv4Addr },
    /// MULTI_EXIT_DISC：MED
    MultiExitDisc { med: u32 },
    /// LOCAL_PREF：本地优先级
    LocalPref { local_pref: u32 },
    /// ATOMIC_AGGREGATE：聚合路由标志
    AtomicAggregate,
    /// AGGREGATOR：聚合者信息
    Aggregator { as_number: u32, router_id: CoreIpv4Addr },
    /// COMMUNITY：BGP 团体
    Community { communities: Vec<u32> },
    /// MP_REACH_NLRI：多协议可达 NLRI
    MpReachNlri { afi: u16, safi: u8, next_hop: Vec<u8>, nlri: Vec<Vec<u8>> },
    /// MP_UNREACH_NLRI：多协议不可达 NLRI
    MpUnreachNlri { afi: u16, safi: u8, nlri: Vec<Vec<u8>> },
}

/// BGP NOTIFICATION 报文
#[derive(Debug, Clone)]
pub struct BgpNotification {
    /// 错误码
    pub error_code: u8,
    /// 子错误码
    pub error_subcode: u8,
    /// 错误数据
    pub data: Vec<u8>,
}

/// 错误码定义
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum BgpErrorCode {
    MessageHeaderError = 1,
    OpenMessageError = 2,
    UpdateMessageError = 3,
    HoldTimerExpired = 4,
    FiniteStateMachineError = 5,
    Cease = 6,
}

/// BGP KEEPALIVE 报文
#[derive(Debug, Clone)]
pub struct BgpKeepalive;

/// BGP ROUTE-REFRESH 报文
#[derive(Debug, Clone)]
pub struct BgpRouteRefresh {
    /// 地址族标识
    pub afi: u16,
    /// 保留
    pub reserved: u8,
    /// 子地址族标识
    pub safi: u8,
}

/// BGP 报文枚举
#[derive(Debug, Clone)]
pub enum BgpMessage {
    Open(BgpOpen),
    Update(BgpUpdate),
    Notification(BgpNotification),
    Keepalive(BgpKeepalive),
    RouteRefresh(BgpRouteRefresh),
}

impl BgpMessage {
    /// 获取消息类型
    pub fn msg_type(&self) -> u8 {
        match self {
            BgpMessage::Open(_) => 1,
            BgpMessage::Update(_) => 2,
            BgpMessage::Notification(_) => 3,
            BgpMessage::Keepalive(_) => 4,
            BgpMessage::RouteRefresh(_) => 5,
        }
    }
}
