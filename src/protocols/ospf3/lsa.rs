// src/protocols/ospf3/lsa.rs
//
// OSPFv3 LSA (Link State Advertisement) 类型定义（简化版）

use crate::common::Ipv6Addr;
use super::packet::LsaHeader;

/// OSPFv3 LSA 类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum LsaType {
    RouterLsa = 0x2001,
    NetworkLsa = 0x2002,
    InterAreaPrefixLsa = 0x2003,
    InterAreaRouterLsa = 0x2004,
    AsExternalLsa = 0x4005,
    LinkLsa = 0x0008,
    IntraAreaPrefixLsa = 0x2009,
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

/// Router-LSA
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RouterLsa {
    pub header: LsaHeader,
    /// 链路数量
    pub link_count: u32,
    /// 链路列表
    pub links: Vec<RouterLink>,
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
    pub metric: u32,
}

/// LSA 统一类型
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Lsa {
    Router(RouterLsa),
    Network(NetworkLsa),
    IntraAreaPrefix(IntraAreaPrefixLsa),
}
