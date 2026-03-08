// src/protocols/ospf2/packet.rs
//
// OSPFv2 报文结构定义（简化版）

use crate::common::Ipv4Addr;
use super::error::{OspfError, OspfResult};

/// OSPFv2 报文类型
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
    /// 从字节解析
    pub fn from_u8(value: u8) -> OspfResult<Self> {
        match value {
            1 => Ok(OspfType::Hello),
            2 => Ok(OspfType::DatabaseDescription),
            3 => Ok(OspfType::LinkStateRequest),
            4 => Ok(OspfType::LinkStateUpdate),
            5 => Ok(OspfType::LinkStateAck),
            _ => Err(OspfError::invalid_packet_type(value)),
        }
    }

    /// 获取报文类型名称
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

impl From<OspfType> for u8 {
    fn from(t: OspfType) -> Self {
        t as u8
    }
}

/// OSPFv2 通用报文头部
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OspfHeader {
    /// 版本号（OSPFv2 = 2）
    pub version: u8,
    /// 报文类型
    pub packet_type: OspfType,
    /// 报文长度（字节）
    pub length: u16,
    /// 路由器 ID
    pub router_id: Ipv4Addr,
    /// 区域 ID
    pub area_id: Ipv4Addr,
    /// 校验和
    pub checksum: u16,
    /// 认证类型
    pub auth_type: u16,
    /// 认证数据
    pub auth_data: [u8; 8],
}

impl OspfHeader {
    /// 报文头部长度
    pub const LENGTH: usize = 24;
}

/// OSPFv2 Hello 报文（简化版）
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OspfHello {
    /// 网络掩码
    pub network_mask: Ipv4Addr,
    /// Hello 间隔（秒）
    pub hello_interval: u16,
    /// 选项
    pub options: u8,
    /// 路由器优先级
    pub priority: u8,
    /// 路由器死亡间隔（秒）
    pub dead_interval: u32,
    /// 指定路由器
    pub designated_router: Ipv4Addr,
    /// 备份指定路由器
    pub backup_designated_router: Ipv4Addr,
    /// 邻居路由器 ID 数量
    pub neighbor_count: u32,
}

/// OSPFv2 数据库描述报文（简化版）
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OspfDatabaseDescription {
    /// 接口 MTU
    pub interface_mtu: u16,
    /// 选项
    pub options: u8,
    /// I 位：Init
    pub i_bit: bool,
    /// M 位：More
    pub m_bit: bool,
    /// MS 位：Master/Slave
    pub ms_bit: bool,
    /// 数据库描述序列号
    pub dd_sequence_number: u32,
    /// LSA 头部数量
    pub lsa_header_count: u32,
}

/// OSPFv2 链路状态请求报文（简化版）
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OspfLinkStateRequest {
    /// 请求数量
    pub request_count: u32,
}

/// OSPFv2 链路状态更新报文（简化版）
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OspfLinkStateUpdate {
    /// LSA 数量
    pub lsa_count: u32,
}

/// OSPFv2 链路状态确认报文（简化版）
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OspfLinkStateAck {
    /// LSA 头部数量
    pub lsa_header_count: u32,
}

/// OSPFv2 报文类型枚举
#[derive(Debug, Clone)]
pub enum OspfPacket {
    Hello(OspfHello),
    DatabaseDescription(OspfDatabaseDescription),
    LinkStateRequest(OspfLinkStateRequest),
    LinkStateUpdate(OspfLinkStateUpdate),
    LinkStateAck(OspfLinkStateAck),
}
