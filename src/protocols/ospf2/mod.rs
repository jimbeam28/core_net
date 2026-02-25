// src/protocols/ospf2/mod.rs
//
// OSPFv2 (Open Shortest Path First Version 2) 协议模块
// 用于 IPv4 网络的链路状态路由协议

// 报文结构和解析
pub mod packet;

// LSA 类型
pub mod lsa;

// 错误类型
pub mod error;

// 配置
pub mod config;

// 报文处理
pub mod process;

// 接口状态机
pub mod interface;

// 邻居状态机
pub mod neighbor;

// 链路状态数据库
pub mod lsdb;

// 重新导出核心类型
pub use packet::{
    OspfHeader, OspfHello, OspfDatabaseDescription, OspfLinkStateRequest,
    OspfLinkStateUpdate, OspfLinkStateAck,
    OspfType,
};

pub use lsa::{
    LsaHeader, LsaType, RouterLsa, RouterLink, NetworkLsa, SummaryLsa,
    AsExternalLsa, Lsa,
};

pub use error::{OspfError, OspfResult};

pub use config::{OspfV2Config, OspfV2InterfaceConfig};

pub use process::{
    OspfProcessResult, OspfProcessor,
    process_ospfv2_packet, encapsulate_ospfv2_hello,
};

pub use interface::{OspfInterface, InterfaceState};

pub use neighbor::{OspfNeighbor, NeighborState};

pub use lsdb::{LinkStateDatabase, LsaEntry};

// OSPFv2 常量

/// OSPFv2 版本号
pub const OSPFV2_VERSION: u8 = 2;

/// OSPFv2 协议号（IPv4）
pub const IP_PROTO_OSPF: u8 = 89;

/// OSPFv2 组播地址 - AllSPFRouters
pub const OSPF_ALL_SPF_ROUTERS: Ipv4Addr = Ipv4Addr::new(224, 0, 0, 5);

/// OSPFv2 组播地址 - AllDRouters
pub const OSPF_ALL_D_ROUTERS: Ipv4Addr = Ipv4Addr::new(224, 0, 0, 6);

/// 最小报文长度
pub const OSPF_MIN_PACKET_LENGTH: usize = 24;

/// 最大报文长度（受 MTU 限制）
pub const OSPF_MAX_PACKET_LENGTH: usize = 65535;

/// OSPF 报文头部
pub const OSPF_HEADER_LENGTH: usize = 24;

use crate::common::Ipv4Addr;
