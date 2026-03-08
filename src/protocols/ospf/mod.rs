// src/protocols/ospf/mod.rs
//
// OSPF (Open Shortest Path First) 协议核心共享模块（精简版）

// SPF 算法和路由计算（简化版）
pub mod spf;

// 共享类型定义
pub mod types;

// 配置
pub mod config;

// 校验和计算
pub mod checksum;

// 简化导出
pub use types::{
    InterfaceState, NeighborState, InterfaceType,
    LsaSequenceNumber, OspfOptions,
};

pub use config::{
    OspfConfig, OspfInterfaceConfig, AuthAlgorithm, CryptoAuthConfig,
    HELLO_INTERVAL_DEFAULT, DEAD_INTERVAL_DEFAULT, PRIORITY_DEFAULT,
    RETRANSMIT_INTERVAL_DEFAULT, TRANSMIT_DELAY_DEFAULT,
};

// 简化版 OSPF 管理器（仅包含基础信息）
#[derive(Debug, Clone)]
pub struct OspfManager {
    pub router_id: u32,
}

impl OspfManager {
    pub fn new(router_id: u32) -> Self {
        Self { router_id }
    }
}

pub use checksum::{
    calculate_fletcher_checksum, verify_fletcher_checksum, update_checksum,
    calculate_ip_checksum, verify_ip_checksum,
};

// OSPF 常量定义

/// OSPF 协议号（IPv4）
pub const IP_PROTO_OSPF: u8 = 89;

/// OSPF 组播地址 - AllSPFRouters
pub const OSPF_ALL_SPF_ROUTERS: crate::protocols::Ipv4Addr =
    crate::protocols::Ipv4Addr::new(224, 0, 0, 5);

/// OSPF 组播地址 - AllDRouters
pub const OSPF_ALL_D_ROUTERS: crate::protocols::Ipv4Addr =
    crate::protocols::Ipv4Addr::new(224, 0, 0, 6);

/// OSPF 报文类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum OspfType {
    Hello = 1,
    DatabaseDescription = 2,
    LinkStateRequest = 3,
    LinkStateUpdate = 4,
    LinkStateAck = 5,
}

impl OspfType {
    pub fn name(&self) -> &'static str {
        match self {
            OspfType::Hello => "Hello",
            OspfType::DatabaseDescription => "Database Description",
            OspfType::LinkStateRequest => "Link State Request",
            OspfType::LinkStateUpdate => "Link State Update",
            OspfType::LinkStateAck => "Link State Acknowledgment",
        }
    }
}

impl TryFrom<u8> for OspfType {
    type Error = crate::common::CoreError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(OspfType::Hello),
            2 => Ok(OspfType::DatabaseDescription),
            3 => Ok(OspfType::LinkStateRequest),
            4 => Ok(OspfType::LinkStateUpdate),
            5 => Ok(OspfType::LinkStateAck),
            _ => Err(crate::common::CoreError::unsupported_protocol(
                format!("Unknown OSPF packet type: {}", value)
            )),
        }
    }
}

impl From<OspfType> for u8 {
    fn from(t: OspfType) -> Self {
        t as u8
    }
}
