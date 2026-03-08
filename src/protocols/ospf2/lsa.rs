// src/protocols/ospf2/lsa.rs
//
// OSPFv2 LSA (Link State Advertisement) 类型定义（简化版）

use crate::common::Ipv4Addr;

/// LSA 类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum LsaType {
    RouterLsa = 1,
    NetworkLsa = 2,
    SummaryNetworkLsa = 3,
    SummaryAsbrLsa = 4,
    ASExternalLsa = 5,
}

/// LSA 头部（所有 LSA 类型共享）
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LsaHeader {
    /// LSA 年龄（秒）
    pub age: u16,
    /// 选项位
    pub options: u8,
    /// LSA 类型
    pub lsa_type: u8,
    /// 链路状态 ID
    pub link_state_id: Ipv4Addr,
    /// 通告路由器
    pub advertising_router: Ipv4Addr,
    /// LSA 序列号
    pub sequence_number: u32,
    /// LSA 校验和
    pub checksum: u16,
    /// LSA 长度（含头部）
    pub length: u16,
}

impl LsaHeader {
    /// LSA 头部长度
    pub const LENGTH: usize = 20;
    /// 初始 LSA 序列号
    pub const INITIAL_SEQUENCE: u32 = 0x80000001;
}

/// 路由器 LSA 链路
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RouterLink {
    /// 链路 ID
    pub link_id: Ipv4Addr,
    /// 链路数据
    pub link_data: Ipv4Addr,
    /// 链路类型 (1-4)
    pub link_type: u8,
    /// 度量值 (cost)
    pub metric: u16,
}

/// 路由器 LSA (Type-1)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RouterLsa {
    /// LSA 头部
    pub header: LsaHeader,
    /// 选项位
    pub options: u8,
    /// 链路数量
    pub link_count: u16,
    /// 链路列表
    pub links: Vec<RouterLink>,
}

/// 网络 LSA (Type-2)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NetworkLsa {
    /// LSA 头部
    pub header: LsaHeader,
    /// 网络掩码
    pub network_mask: Ipv4Addr,
    /// 连接的路由器列表
    pub attached_routers: Vec<Ipv4Addr>,
}

/// 汇总 LSA (Type-3/4)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SummaryLsa {
    /// LSA 头部
    pub header: LsaHeader,
    /// 网络掩码
    pub network_mask: Ipv4Addr,
    /// 度量值
    pub metric: u32,
}

/// AS 外部 LSA (Type-5)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AsExternalLsa {
    /// LSA 头部
    pub header: LsaHeader,
    /// 网络掩码
    pub network_mask: Ipv4Addr,
    /// E 位：外部路由类型
    pub e_bit: bool,
    /// 度量值
    pub metric: u32,
    /// 转发地址
    pub forwarding_address: Ipv4Addr,
    /// 外部路由标签
    pub external_route_tag: u32,
}

/// LSA 统一类型
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Lsa {
    Router(RouterLsa),
    Network(NetworkLsa),
    SummaryNetwork(SummaryLsa),
    SummaryAsbr(SummaryLsa),
    ASExternal(AsExternalLsa),
}
