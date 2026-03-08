// src/protocols/ospf2/mod.rs
//
// OSPFv2 (Open Shortest Path First Version 2) 协议模块（精简版）

// 报文结构和解析
pub mod packet;

// 错误类型
pub mod error;

// 配置
pub mod config;

// 报文处理
pub mod process;

// 重新导出核心类型
pub use packet::{
    OspfHeader, OspfHello, OspfDatabaseDescription, OspfLinkStateRequest,
    OspfLinkStateUpdate, OspfLinkStateAck,
    OspfType,
};

pub use error::{OspfError, OspfResult};

pub use config::OspfV2Config;

pub use process::{process_ospfv2_packet, OspfProcessResult};

// OSPFv2 常量

/// OSPFv2 版本号
pub const OSPFV2_VERSION: u8 = 2;

/// OSPFv2 协议号（IPv4）
pub const IP_PROTO_OSPF: u8 = 89;

/// OSPFv2 组播地址 - AllSPFRouters
pub const OSPF_ALL_SPF_ROUTERS: crate::protocols::Ipv4Addr =
    crate::protocols::Ipv4Addr::new(224, 0, 0, 5);

/// OSPFv2 组播地址 - AllDRouters
pub const OSPF_ALL_D_ROUTERS: crate::protocols::Ipv4Addr =
    crate::protocols::Ipv4Addr::new(224, 0, 0, 6);
