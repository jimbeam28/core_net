// src/protocols/ospf2/lsa.rs
//
// OSPFv2 LSA (Link State Advertisement) 类型定义

use crate::common::Ipv4Addr;
use super::error::{OspfError, OspfResult};

/// LSA 类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum LsaType {
    /// 路由器 LSA (Type-1)
    RouterLsa = 1,
    /// 网络 LSA (Type-2)
    NetworkLsa = 2,
    /// 网络汇总 LSA (Type-3)
    SummaryNetworkLsa = 3,
    /// ASBR 汇总 LSA (Type-4)
    SummaryAsbrLsa = 4,
    /// AS 外部 LSA (Type-5)
    ASExternalLsa = 5,
    /// 组成员 LSA (Type-6，MOSPF，已废弃)
    GroupMembershipLsa = 6,
    /// NSSA LSA (Type-7)
    Type7Lsa = 7,
    /// 外部属性 LSA (Type-8，未使用)
    ExternalAttributesLsa = 8,
    /// Opaque LSA (Type-9, 10, 11)
    OpaqueLsa = 9,
}

impl LsaType {
    /// 从字节解析
    pub fn from_u8(value: u8) -> OspfResult<Self> {
        match value {
            1 => Ok(LsaType::RouterLsa),
            2 => Ok(LsaType::NetworkLsa),
            3 => Ok(LsaType::SummaryNetworkLsa),
            4 => Ok(LsaType::SummaryAsbrLsa),
            5 => Ok(LsaType::ASExternalLsa),
            6 => Ok(LsaType::GroupMembershipLsa),
            7 => Ok(LsaType::Type7Lsa),
            8 => Ok(LsaType::ExternalAttributesLsa),
            9..=11 => Ok(LsaType::OpaqueLsa),
            _ => Err(OspfError::InvalidLsaType { lsa_type: value }),
        }
    }

    /// 获取 LSA 类型名称
    pub fn name(&self) -> &'static str {
        match self {
            LsaType::RouterLsa => "Router LSA",
            LsaType::NetworkLsa => "Network LSA",
            LsaType::SummaryNetworkLsa => "Summary Network LSA",
            LsaType::SummaryAsbrLsa => "Summary ASBR LSA",
            LsaType::ASExternalLsa => "AS External LSA",
            LsaType::GroupMembershipLsa => "Group Membership LSA",
            LsaType::Type7Lsa => "Type-7 LSA",
            LsaType::ExternalAttributesLsa => "External Attributes LSA",
            LsaType::OpaqueLsa => "Opaque LSA",
        }
    }
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

    /// 最小 LSA 年龄
    pub const MIN_AGE: u16 = 0;

    /// 最大 LSA 年龄（3600 秒 = 1 小时）
    pub const MAX_AGE: u16 = 3600;

    /// 初始 LSA 序列号
    pub const INITIAL_SEQUENCE: u32 = 0x80000001;

    /// 最大 LSA 序列号
    pub const MAX_SEQUENCE: u32 = 0x7FFFFFFF;

    /// 创建新的 LSA 头部
    pub fn new(
        lsa_type: u8,
        link_state_id: Ipv4Addr,
        advertising_router: Ipv4Addr,
    ) -> Self {
        Self {
            age: 0,
            options: 0,
            lsa_type,
            link_state_id,
            advertising_router,
            sequence_number: Self::INITIAL_SEQUENCE,
            checksum: 0,
            length: Self::LENGTH as u16,
        }
    }

    /// 从字节解析
    pub fn from_bytes(data: &[u8]) -> OspfResult<Self> {
        if data.len() < Self::LENGTH {
            return Err(OspfError::packet_too_short(Self::LENGTH, data.len()));
        }

        let age = u16::from_be_bytes([data[0], data[1]]);
        let options = data[2];
        let lsa_type = data[3];

        let link_state_id_bytes: [u8; 4] = data[4..8].try_into().unwrap();
        let link_state_id = Ipv4Addr::from_bytes(link_state_id_bytes);

        let advertising_router_bytes: [u8; 4] = data[8..12].try_into().unwrap();
        let advertising_router = Ipv4Addr::from_bytes(advertising_router_bytes);
        let sequence_number = u32::from_be_bytes({
            let mut arr = [0u8; 4];
            arr.copy_from_slice(&data[12..16]);
            arr
        });
        let checksum = u16::from_be_bytes([data[16], data[17]]);
        let length = u16::from_be_bytes([data[18], data[19]]);

        Ok(Self {
            age,
            options,
            lsa_type,
            link_state_id,
            advertising_router,
            sequence_number,
            checksum,
            length,
        })
    }

    /// 转换为字节
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(Self::LENGTH);

        bytes.extend_from_slice(&self.age.to_be_bytes());
        bytes.push(self.options);
        bytes.push(self.lsa_type);
        bytes.extend_from_slice(&self.link_state_id.as_bytes()[..]);
        bytes.extend_from_slice(&self.advertising_router.as_bytes()[..]);
        bytes.extend_from_slice(&self.sequence_number.to_be_bytes());
        bytes.extend_from_slice(&self.checksum.to_be_bytes());
        bytes.extend_from_slice(&self.length.to_be_bytes());

        bytes
    }

    /// 转换为字节切片（临时实现）
    pub fn as_bytes(&self) -> Vec<u8> {
        self.to_bytes()
    }

    /// 检查 LSA 是否已过期
    pub fn is_expired(&self) -> bool {
        self.age >= Self::MAX_AGE
    }

    /// 检查序列号是否有效
    pub fn is_valid_sequence(&self) -> bool {
        self.sequence_number >= Self::INITIAL_SEQUENCE
    }
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

impl RouterLink {
    /// 链路条目长度
    pub const LENGTH: usize = 12;

    /// 点对点链路
    pub const TYPE_POINT_TO_POINT: u8 = 1;

    /// 跨网段链路
    pub const TYPE_TRANSIT: u8 = 2;

    /// 虚链路
    pub const TYPE_VIRTUAL: u8 = 4;

    /// 创建新的路由器链路
    pub fn new(link_type: u8, link_id: Ipv4Addr, link_data: Ipv4Addr, metric: u16) -> Self {
        Self {
            link_id,
            link_data,
            link_type,
            metric,
        }
    }

    /// 从字节解析
    pub fn from_bytes(data: &[u8]) -> OspfResult<Self> {
        if data.len() < Self::LENGTH {
            return Err(OspfError::packet_too_short(Self::LENGTH, data.len()));
        }

        let link_id_bytes: [u8; 4] = data[0..4].try_into().unwrap();
        let link_id = Ipv4Addr::from_bytes(link_id_bytes);

        let link_data_bytes: [u8; 4] = data[4..8].try_into().unwrap();
        let link_data = Ipv4Addr::from_bytes(link_data_bytes);
        let link_type = data[8];
        let metric = u16::from_be_bytes([data[10], data[11]]);

        Ok(Self {
            link_id,
            link_data,
            link_type,
            metric,
        })
    }

    /// 转换为字节
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(Self::LENGTH);

        bytes.extend_from_slice(&self.link_id.as_bytes()[..]);
        bytes.extend_from_slice(&self.link_data.as_bytes()[..]);
        bytes.push(self.link_type);
        bytes.push(0);  // 保留字段
        bytes.extend_from_slice(&self.metric.to_be_bytes());

        bytes
    }

    /// 转换为字节切片（临时实现）
    pub fn as_bytes(&self) -> Vec<u8> {
        self.to_bytes()
    }
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

impl RouterLsa {
    /// 创建新的路由器 LSA
    pub fn new(link_state_id: Ipv4Addr, advertising_router: Ipv4Addr) -> Self {
        let mut header = LsaHeader::new(1, link_state_id, advertising_router);
        header.lsa_type = LsaType::RouterLsa as u8;

        Self {
            header,
            options: 0,
            link_count: 0,
            links: Vec::new(),
        }
    }

    /// 添加链路
    pub fn add_link(&mut self, link: RouterLink) {
        self.links.push(link);
        self.link_count = self.links.len() as u16;
        self.header.length = (LsaHeader::LENGTH + 4 + self.links.len() * RouterLink::LENGTH) as u16;
    }

    /// 转换为字节
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = self.header.to_bytes();

        bytes.push(self.options);
        bytes.extend_from_slice(&[0u8, 0x1u8, 0x1u8]);  // 保留字段
        bytes.extend_from_slice(&self.link_count.to_be_bytes());

        for link in &self.links {
            bytes.extend_from_slice(&link.to_bytes()[..]);
        }

        bytes
    }

    /// 转换为字节切片（临时实现）
    pub fn as_bytes(&self) -> Vec<u8> {
        self.to_bytes()
    }
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

impl NetworkLsa {
    /// 创建新的网络 LSA
    pub fn new(link_state_id: Ipv4Addr, advertising_router: Ipv4Addr, network_mask: Ipv4Addr) -> Self {
        let mut header = LsaHeader::new(2, link_state_id, advertising_router);
        header.lsa_type = LsaType::NetworkLsa as u8;

        Self {
            header,
            network_mask,
            attached_routers: Vec::new(),
        }
    }

    /// 添加连接的路由器
    pub fn add_router(&mut self, router_id: Ipv4Addr) {
        if !self.attached_routers.contains(&router_id) {
            self.attached_routers.push(router_id);
            self.header.length = (LsaHeader::LENGTH + 4 + self.attached_routers.len() * 4) as u16;
        }
    }

    /// 转换为字节
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = self.header.to_bytes();

        bytes.extend_from_slice(&self.network_mask.as_bytes()[..]);

        for router_id in &self.attached_routers {
            bytes.extend_from_slice(&router_id.as_bytes()[..]);
        }

        bytes
    }

    /// 转换为字节切片（临时实现）
    pub fn as_bytes(&self) -> Vec<u8> {
        self.to_bytes()
    }
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

impl SummaryLsa {
    /// 创建新的汇总 LSA
    pub fn new(
        lsa_type: LsaType,
        link_state_id: Ipv4Addr,
        advertising_router: Ipv4Addr,
        network_mask: Ipv4Addr,
        metric: u32,
    ) -> Self {
        let mut header = LsaHeader::new(lsa_type as u8, link_state_id, advertising_router);

        Self {
            header,
            network_mask,
            metric,
        }
    }

    /// 转换为字节
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = self.header.to_bytes();

        bytes.extend_from_slice(&self.network_mask.as_bytes()[..]);
        bytes.extend_from_slice(&[0u8, 0x1u8, 0x1u8, 0x1u8]);  // 保留字段
        bytes.extend_from_slice(&self.metric.to_be_bytes());

        bytes
    }

    /// 转换为字节切片（临时实现）
    pub fn as_bytes(&self) -> Vec<u8> {
        self.to_bytes()
    }
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

impl AsExternalLsa {
    /// 创建新的 AS 外部 LSA
    pub fn new(
        link_state_id: Ipv4Addr,
        advertising_router: Ipv4Addr,
        network_mask: Ipv4Addr,
        metric: u32,
        e_bit: bool,
    ) -> Self {
        let mut header = LsaHeader::new(5, link_state_id, advertising_router);
        header.lsa_type = LsaType::ASExternalLsa as u8;

        Self {
            header,
            network_mask,
            e_bit,
            metric,
            forwarding_address: Ipv4Addr::UNSPECIFIED,
            external_route_tag: 0,
        }
    }

    /// 转换为字节
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = self.header.to_bytes();

        bytes.extend_from_slice(&self.network_mask.as_bytes()[..]);

        let mut flags = 0u8;
        if self.e_bit { flags |= 0x80; }
        bytes.push(flags);

        bytes.extend_from_slice(&[0u8, 0x1u8, 0x1u8]);  // 保留字段
        bytes.extend_from_slice(&self.metric.to_be_bytes());
        bytes.extend_from_slice(&self.forwarding_address.as_bytes()[..]);
        bytes.extend_from_slice(&self.external_route_tag.to_be_bytes());

        bytes
    }

    /// 转换为字节切片（临时实现）
    pub fn as_bytes(&self) -> Vec<u8> {
        self.to_bytes()
    }
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

impl Lsa {
    /// 获取 LSA 头部
    pub fn header(&self) -> &LsaHeader {
        match self {
            Lsa::Router(lsa) => &lsa.header,
            Lsa::Network(lsa) => &lsa.header,
            Lsa::SummaryNetwork(lsa) => &lsa.header,
            Lsa::SummaryAsbr(lsa) => &lsa.header,
            Lsa::ASExternal(lsa) => &lsa.header,
        }
    }

    /// 获取 LSA 类型
    pub fn lsa_type(&self) -> u8 {
        self.header().lsa_type
    }

    /// 转换为字节
    pub fn to_bytes(&self) -> Vec<u8> {
        match self {
            Lsa::Router(lsa) => lsa.to_bytes(),
            Lsa::Network(lsa) => lsa.to_bytes(),
            Lsa::SummaryNetwork(lsa) => lsa.to_bytes(),
            Lsa::SummaryAsbr(lsa) => lsa.to_bytes(),
            Lsa::ASExternal(lsa) => lsa.to_bytes(),
        }
    }

    /// 转换为字节切片（临时实现）
    pub fn as_bytes(&self) -> Vec<u8> {
        self.to_bytes()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lsa_type_from_u8() {
        assert_eq!(LsaType::from_u8(1).unwrap(), LsaType::RouterLsa);
        assert_eq!(LsaType::from_u8(2).unwrap(), LsaType::NetworkLsa);
        assert!(LsaType::from_u8(99).is_err());
    }

    #[test]
    fn test_lsa_header_new() {
        let header = LsaHeader::new(
            1,
            Ipv4Addr::new(1, 1, 1, 1),
            Ipv4Addr::new(2, 2, 2, 2),
        );

        assert_eq!(header.age, 0);
        assert_eq!(header.lsa_type, 1);
        assert_eq!(header.sequence_number, LsaHeader::INITIAL_SEQUENCE);
    }

    #[test]
    fn test_lsa_header_round_trip() {
        let mut original = LsaHeader::new(
            1,
            Ipv4Addr::new(1, 1, 1, 1),
            Ipv4Addr::new(2, 2, 2, 2),
        );
        original.age = 100;
        original.sequence_number = 0x80000005;

        let bytes = original.as_bytes();
        let parsed = LsaHeader::from_bytes(&bytes).unwrap();

        assert_eq!(parsed.age, original.age);
        assert_eq!(parsed.sequence_number, original.sequence_number);
        assert_eq!(parsed.link_state_id, original.link_state_id);
    }

    #[test]
    fn test_router_link_new() {
        let link = RouterLink::new(
            RouterLink::TYPE_POINT_TO_POINT,
            Ipv4Addr::new(1, 1, 1, 2),
            Ipv4Addr::new(10, 0, 0, 1),
            10,
        );

        assert_eq!(link.link_type, RouterLink::TYPE_POINT_TO_POINT);
        assert_eq!(link.metric, 10);
    }

    #[test]
    fn test_router_lsa_add_link() {
        let mut lsa = RouterLsa::new(
            Ipv4Addr::new(1, 1, 1, 1),
            Ipv4Addr::new(1, 1, 1, 1),
        );

        let link = RouterLink::new(
            RouterLink::TYPE_POINT_TO_POINT,
            Ipv4Addr::new(1, 1, 1, 2),
            Ipv4Addr::new(10, 0, 0, 1),
            10,
        );

        lsa.add_link(link);

        assert_eq!(lsa.links.len(), 1);
        assert_eq!(lsa.link_count, 1);
    }

    #[test]
    fn test_network_lsa_add_router() {
        let mut lsa = NetworkLsa::new(
            Ipv4Addr::new(10, 0, 0, 1),
            Ipv4Addr::new(1, 1, 1, 1),
            Ipv4Addr::new(255, 255, 255, 0),
        );

        lsa.add_router(Ipv4Addr::new(1, 1, 1, 1));
        lsa.add_router(Ipv4Addr::new(1, 1, 1, 2));
        lsa.add_router(Ipv4Addr::new(1, 1, 1, 1));  // 重复

        assert_eq!(lsa.attached_routers.len(), 2);
    }

    #[test]
    fn test_as_external_lsa() {
        let lsa = AsExternalLsa::new(
            Ipv4Addr::new(10, 0, 0, 0),
            Ipv4Addr::new(1, 1, 1, 1),
            Ipv4Addr::new(255, 255, 255, 0),
            20,
            true,  // E1 路由
        );

        assert!(lsa.e_bit);
        assert_eq!(lsa.metric, 20);
    }
}
