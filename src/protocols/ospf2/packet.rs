// src/protocols/ospf2/packet.rs
//
// OSPFv2 报文结构定义和解析/封装函数（精简版）

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

    /// 从字节解析报文头部
    pub fn from_bytes(data: &[u8]) -> OspfResult<Self> {
        if data.len() < Self::LENGTH {
            return Err(OspfError::packet_too_short(Self::LENGTH, data.len()));
        }

        let version = data[0];
        let packet_type = OspfType::from_u8(data[1])?;
        let length = u16::from_be_bytes([data[2], data[3]]);
        let router_id = Ipv4Addr::new(data[4], data[5], data[6], data[7]);
        let area_id = Ipv4Addr::new(data[8], data[9], data[10], data[11]);
        let checksum = u16::from_be_bytes([data[12], data[13]]);
        let auth_type = u16::from_be_bytes([data[14], data[15]]);
        let mut auth_data = [0u8; 8];
        auth_data.copy_from_slice(&data[16..24]);

        Ok(OspfHeader {
            version,
            packet_type,
            length,
            router_id,
            area_id,
            checksum,
            auth_type,
            auth_data,
        })
    }

    /// 将报文头部转换为字节
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(Self::LENGTH);
        bytes.push(self.version);
        bytes.push(self.packet_type.into());
        bytes.extend_from_slice(&self.length.to_be_bytes());
        bytes.extend_from_slice(&self.router_id.bytes);
        bytes.extend_from_slice(&self.area_id.bytes);
        bytes.extend_from_slice(&self.checksum.to_be_bytes());
        bytes.extend_from_slice(&self.auth_type.to_be_bytes());
        bytes.extend_from_slice(&self.auth_data);
        bytes
    }
}

/// OSPFv2 Hello 报文（精简版）
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
    /// 邻居路由器 ID 列表（精简版：不存储具体列表）
    pub neighbor_count: u32,
}

impl OspfHello {
    /// 最小 Hello 报文长度
    pub const MIN_LENGTH: usize = 20;

    /// 从字节解析 Hello 报文
    pub fn from_bytes(data: &[u8]) -> OspfResult<Self> {
        if data.len() < Self::MIN_LENGTH {
            return Err(OspfError::packet_too_short(Self::MIN_LENGTH, data.len()));
        }

        let network_mask = Ipv4Addr::new(data[0], data[1], data[2], data[3]);
        let hello_interval = u16::from_be_bytes([data[4], data[5]]);
        let options = data[6];
        let priority = data[7];
        let dead_interval = u32::from_be_bytes([data[8], data[9], data[10], data[11]]);
        let designated_router = Ipv4Addr::new(data[12], data[13], data[14], data[15]);
        let backup_designated_router = Ipv4Addr::new(data[16], data[17], data[18], data[19]);

        // 计算邻居数量（剩余字节 / 4 字节每个邻居 ID）
        let neighbor_count = if data.len() > Self::MIN_LENGTH {
            ((data.len() - Self::MIN_LENGTH) / 4) as u32
        } else {
            0
        };

        Ok(OspfHello {
            network_mask,
            hello_interval,
            options,
            priority,
            dead_interval,
            designated_router,
            backup_designated_router,
            neighbor_count,
        })
    }
}

/// OSPFv2 数据库描述报文（精简版）
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
    /// LSA 头部数量（精简版）
    pub lsa_header_count: u32,
}

impl OspfDatabaseDescription {
    /// 最小 DD 报文长度
    pub const MIN_LENGTH: usize = 8;

    /// 从字节解析 DD 报文
    pub fn from_bytes(data: &[u8]) -> OspfResult<Self> {
        if data.len() < Self::MIN_LENGTH {
            return Err(OspfError::packet_too_short(Self::MIN_LENGTH, data.len()));
        }

        let interface_mtu = u16::from_be_bytes([data[0], data[1]]);
        let options = data[2];
        let flags = data[3];
        let dd_sequence_number = u32::from_be_bytes([data[4], data[5], data[6], data[7]]);

        let i_bit = (flags & 0x04) != 0;
        let m_bit = (flags & 0x02) != 0;
        let ms_bit = (flags & 0x01) != 0;

        // 计算 LSA 头部数量（剩余字节 / 20 字节每个 LSA 头部）
        let lsa_header_count = if data.len() > Self::MIN_LENGTH {
            ((data.len() - Self::MIN_LENGTH) / 20) as u32
        } else {
            0
        };

        Ok(OspfDatabaseDescription {
            interface_mtu,
            options,
            i_bit,
            m_bit,
            ms_bit,
            dd_sequence_number,
            lsa_header_count,
        })
    }
}

/// OSPFv2 链路状态请求报文（精简版）
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OspfLinkStateRequest {
    /// 请求数量（精简版）
    pub request_count: u32,
}

impl OspfLinkStateRequest {
    /// 从字节解析 LSR 报文
    pub fn from_bytes(data: &[u8]) -> OspfResult<Self> {
        // 每个请求条目 12 字节
        let request_count = (data.len() / 12) as u32;
        Ok(OspfLinkStateRequest { request_count })
    }
}

/// OSPFv2 链路状态更新报文（精简版）
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OspfLinkStateUpdate {
    /// LSA 数量
    pub lsa_count: u32,
}

impl OspfLinkStateUpdate {
    /// 从字节解析 LSU 报文
    pub fn from_bytes(data: &[u8]) -> OspfResult<Self> {
        if data.len() < 4 {
            return Err(OspfError::packet_too_short(4, data.len()));
        }
        let lsa_count = u32::from_be_bytes([data[0], data[1], data[2], data[3]]);
        Ok(OspfLinkStateUpdate { lsa_count })
    }
}

/// OSPFv2 链路状态确认报文（精简版）
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OspfLinkStateAck {
    /// LSA 头部数量
    pub lsa_header_count: u32,
}

impl OspfLinkStateAck {
    /// 从字节解析 LSAck 报文
    pub fn from_bytes(data: &[u8]) -> OspfResult<Self> {
        // 每个 LSA 头部 20 字节
        let lsa_header_count = (data.len() / 20) as u32;
        Ok(OspfLinkStateAck { lsa_header_count })
    }
}
