// src/protocols/ospf3/mod.rs
//
// OSPFv3 协议模块（精简版）

pub mod error;
pub mod packet;
pub mod config;
pub mod process;

pub use error::{Ospfv3Error, Ospfv3Result};
pub use packet::*;
pub use process::{process_ospfv3_packet, Ospfv3ProcessResult};

// OSPFv3 常量定义

/// OSPFv3 协议号（IPv6 Next Header）
pub const IP_PROTO_OSPFV3: u8 = 89;

/// OSPFv3 版本号
pub const OSPFV3_VERSION: u8 = 3;

/// OSPFv3 组播地址 - AllSPFRouters
pub fn all_spf_routers() -> crate::protocols::Ipv6Addr {
    crate::protocols::Ipv6Addr::new(0xff02, 0, 0, 0, 0, 0, 0, 5)
}

/// OSPFv3 组播地址 - AllDRouters
pub fn all_d_routers() -> crate::protocols::Ipv6Addr {
    crate::protocols::Ipv6Addr::new(0xff02, 0, 0, 0, 0, 0, 0, 6)
}

/// OSPFv3 报文类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Ospfv3Type {
    Hello = 1,
    DatabaseDescription = 2,
    LinkStateRequest = 3,
    LinkStateUpdate = 4,
    LinkStateAck = 5,
}

impl Ospfv3Type {
    /// 从字节解析
    pub fn from_u8(value: u8) -> Ospfv3Result<Self> {
        match value {
            1 => Ok(Ospfv3Type::Hello),
            2 => Ok(Ospfv3Type::DatabaseDescription),
            3 => Ok(Ospfv3Type::LinkStateRequest),
            4 => Ok(Ospfv3Type::LinkStateUpdate),
            5 => Ok(Ospfv3Type::LinkStateAck),
            _ => Err(Ospfv3Error::invalid_packet_type(value)),
        }
    }

    /// 获取报文类型名称
    pub fn name(&self) -> &'static str {
        match self {
            Ospfv3Type::Hello => "Hello",
            Ospfv3Type::DatabaseDescription => "Database Description",
            Ospfv3Type::LinkStateRequest => "Link State Request",
            Ospfv3Type::LinkStateUpdate => "Link State Update",
            Ospfv3Type::LinkStateAck => "Link State Acknowledgment",
        }
    }
}

impl From<Ospfv3Type> for u8 {
    fn from(t: Ospfv3Type) -> Self {
        t as u8
    }
}
