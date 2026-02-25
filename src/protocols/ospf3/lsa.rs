// src/protocols/ospf3/lsa.rs
//
// OSPFv3 LSA (Link State Advertisement) 类型定义

use crate::common::Ipv6Addr;
use super::error::{Ospfv3Error, Ospfv3Result};
use super::packet::LsaHeader;

/// OSPFv3 LSA 类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum LsaType {
    /// Router-LSA
    RouterLsa = 0x2001,
    /// Network-LSA
    NetworkLsa = 0x2002,
    /// Inter-Area-Prefix-LSA
    InterAreaPrefixLsa = 0x2003,
    /// Inter-Area-Router-LSA
    InterAreaRouterLsa = 0x2004,
    /// AS-External-LSA
    AsExternalLsa = 0x4005,
    /// Link-LSA
    LinkLsa = 0x0008,
    /// Intra-Area-Prefix-LSA
    IntraAreaPrefixLsa = 0x2009,
}

impl LsaType {
    pub fn from_u16(value: u16) -> Ospfv3Result<Self> {
        match value {
            0x2001 => Ok(LsaType::RouterLsa),
            0x2002 => Ok(LsaType::NetworkLsa),
            0x2003 => Ok(LsaType::InterAreaPrefixLsa),
            0x2004 => Ok(LsaType::InterAreaRouterLsa),
            0x4005 => Ok(LsaType::AsExternalLsa),
            0x0008 => Ok(LsaType::LinkLsa),
            0x2009 => Ok(LsaType::IntraAreaPrefixLsa),
            _ => Err(Ospfv3Error::InvalidLsaType { lsa_type: value as u32 }),
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            LsaType::RouterLsa => "Router LSA",
            LsaType::NetworkLsa => "Network LSA",
            LsaType::InterAreaPrefixLsa => "Inter-Area Prefix LSA",
            LsaType::InterAreaRouterLsa => "Inter-Area Router LSA",
            LsaType::AsExternalLsa => "AS External LSA",
            LsaType::LinkLsa => "Link LSA",
            LsaType::IntraAreaPrefixLsa => "Intra-Area Prefix LSA",
        }
    }
}

/// Router-LSA 链路
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RouterLink {
    /// 链路类型
    pub link_type: u8,
    /// 链路度量
    pub metric: u16,
    /// 链路接口 ID
    pub link_interface_id: u32,
    /// 邻路由器 ID (32-bit)
    pub neighbor_router_id: u32,
    /// 邻居接口 ID
    pub neighbor_interface_id: u32,
}

impl RouterLink {
    pub const LENGTH: usize = 16;

    /// 点到点链路
    pub const TYPE_POINT_TO_POINT: u8 = 1;

    /// 跨网段链路
    pub const TYPE_TRANSIT: u8 = 2;

    /// 虚链路
    pub const TYPE_VIRTUAL: u8 = 4;

    pub fn new(
        link_type: u8,
        metric: u16,
        link_interface_id: u32,
        neighbor_router_id: u32,
        neighbor_interface_id: u32,
    ) -> Self {
        Self {
            link_type,
            metric,
            link_interface_id,
            neighbor_router_id,
            neighbor_interface_id,
        }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(Self::LENGTH);

        bytes.push(self.link_type);
        bytes.push(0); // 保留字段
        bytes.push(0); // 保留字段
        bytes.push(0); // 保留字段
        bytes.extend_from_slice(&self.metric.to_be_bytes());
        bytes.extend_from_slice(&self.link_interface_id.to_be_bytes());
        bytes.extend_from_slice(&self.neighbor_router_id.to_be_bytes());
        bytes.extend_from_slice(&self.neighbor_interface_id.to_be_bytes());

        bytes
    }
}

/// Router-LSA
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RouterLsa {
    pub header: LsaHeader,
    /// 链路数量
    pub link_count: u32,
    /// 链路列表
    pub links: Vec<RouterLink>,
}

impl RouterLsa {
    pub fn new(link_state_id: u32, advertising_router: u32) -> Self {
        let header = LsaHeader::new(0x2001, link_state_id, advertising_router);

        Self {
            header,
            link_count: 0,
            links: Vec::new(),
        }
    }

    pub fn add_link(&mut self, link: RouterLink) {
        self.links.push(link);
        self.link_count = self.links.len() as u32;
        self.header.length = (LsaHeader::LENGTH + 4 + self.links.len() * RouterLink::LENGTH) as u16;
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = self.header.to_bytes();

        bytes.extend_from_slice(&[0u8, 0, 0, 0]); // 保留字段
        bytes.extend_from_slice(&self.link_count.to_be_bytes());

        for link in &self.links {
            bytes.extend_from_slice(&link.to_bytes()[..]);
        }

        bytes
    }

    pub fn as_bytes(&self) -> Vec<u8> {
        self.to_bytes()
    }
}

/// Network-LSA
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NetworkLsa {
    pub header: LsaHeader,
    /// 连接的路由器选项
    pub options: u32,
    /// 连接的路由器列表 (32-bit Router IDs)
    pub attached_routers: Vec<u32>,
}

impl NetworkLsa {
    pub fn new(link_state_id: u32, advertising_router: u32) -> Self {
        let header = LsaHeader::new(0x2002, link_state_id, advertising_router);

        Self {
            header,
            options: 0,
            attached_routers: Vec::new(),
        }
    }

    pub fn add_router(&mut self, router_id: u32) {
        if !self.attached_routers.contains(&router_id) {
            self.attached_routers.push(router_id);
            self.header.length = (LsaHeader::LENGTH + 4 + self.attached_routers.len() * 4) as u16;
        }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = self.header.to_bytes();

        bytes.extend_from_slice(&self.options.to_be_bytes());

        for router_id in &self.attached_routers {
            bytes.extend_from_slice(&router_id.to_be_bytes());
        }

        bytes
    }

    pub fn as_bytes(&self) -> Vec<u8> {
        self.to_bytes()
    }
}

/// Intra-Area-Prefix-LSA
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IntraAreaPrefixLsa {
    pub header: LsaHeader,
    /// 前缀数量
    pub prefix_count: u32,
    /// 前缀列表
    pub prefixes: Vec<Prefix>,
}

/// IPv6 前缀
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Prefix {
    /// 前缀长度
    pub prefix_length: u8,
    /// 前缀选项
    pub prefix_options: u8,
    /// 地址前缀
    pub address_prefix: Ipv6Addr,
    /// 度量值
    pub metric: u24,
}

impl IntraAreaPrefixLsa {
    pub fn new(link_state_id: u32, advertising_router: u32) -> Self {
        let header = LsaHeader::new(0x2009, link_state_id, advertising_router);

        Self {
            header,
            prefix_count: 0,
            prefixes: Vec::new(),
        }
    }

    pub fn add_prefix(&mut self, prefix: Prefix) {
        // 计算 prefix 长度：(prefix_length + 7) / 8 字节，向上取整
        let prefix_bytes = ((prefix.prefix_length as usize + 7) / 8) as usize;
        self.prefixes.push(prefix);
        self.prefix_count = self.prefixes.len() as u32;
        self.header.length = (LsaHeader::LENGTH + 4 + self.prefixes.len() * (4 + 16 + prefix_bytes + 3)) as u16;
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = self.header.to_bytes();

        bytes.extend_from_slice(&[0u8, 0, 0]); // 保留字段
        bytes.extend_from_slice(&self.prefix_count.to_be_bytes());

        for prefix in &self.prefixes {
            bytes.push(prefix.prefix_length);
            bytes.push(prefix.prefix_options);
            // 只写入有效的前缀字节
            let prefix_bytes = ((prefix.prefix_length as usize + 7) / 8) as usize;
            bytes.extend_from_slice(&prefix.address_prefix.as_bytes()[..prefix_bytes]);
            // 补齐到 16 字节边界
            for _ in prefix_bytes..16 {
                bytes.push(0);
            }
            bytes.extend_from_slice(&prefix.metric.to_be_bytes());
        }

        bytes
    }

    pub fn as_bytes(&self) -> Vec<u8> {
        self.to_bytes()
    }
}

/// u24 类型用于度量值
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct u24(pub u32);

impl From<u32> for u24 {
    fn from(value: u32) -> Self {
        u24(value & 0x00FFFFFF)
    }
}

impl u24 {
    pub fn to_be_bytes(&self) -> [u8; 3] {
        [
            ((self.0 >> 16) & 0xFF) as u8,
            ((self.0 >> 8) & 0xFF) as u8,
            (self.0 & 0xFF) as u8,
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lsa_type_from_u16() {
        assert_eq!(LsaType::from_u16(0x2001).unwrap(), LsaType::RouterLsa);
        assert_eq!(LsaType::from_u16(0x2002).unwrap(), LsaType::NetworkLsa);
        assert!(LsaType::from_u16(0x9999).is_err());
    }

    #[test]
    fn test_router_link_new() {
        let link = RouterLink::new(
            RouterLink::TYPE_POINT_TO_POINT,
            10,
            1,
            0x00000002,
            2,
        );

        assert_eq!(link.link_type, RouterLink::TYPE_POINT_TO_POINT);
        assert_eq!(link.metric, 10);
    }

    #[test]
    fn test_router_lsa_add_link() {
        let mut lsa = RouterLsa::new(
            0x00000001,
            0x00000001,
        );

        let link = RouterLink::new(
            RouterLink::TYPE_POINT_TO_POINT,
            10,
            1,
            0x00000002,
            2,
        );

        lsa.add_link(link);

        assert_eq!(lsa.links.len(), 1);
        assert_eq!(lsa.link_count, 1);
    }
}

/// LSA 统一类型
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Lsa {
    Router(RouterLsa),
    Network(NetworkLsa),
    IntraAreaPrefix(IntraAreaPrefixLsa),
}
