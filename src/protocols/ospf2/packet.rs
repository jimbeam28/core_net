// src/protocols/ospf2/packet.rs
//
// OSPFv2 报文结构定义和解析/封装函数

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
///
/// OSPF 报文直接封装在 IP 报文中（协议号 89），不使用 UDP 或 TCP。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OspfHeader {
    /// OSPF 版本号 (OSPFv2 = 2)
    pub version: u8,
    /// 报文类型 (1-5)
    pub packet_type: OspfType,
    /// 报文总长度（含头部）
    pub length: u16,
    /// 路由器 ID
    pub router_id: Ipv4Addr,
    /// 区域 ID
    pub area_id: Ipv4Addr,
    /// 校验和
    pub checksum: u16,
    /// 认证类型 (0=None, 1=Simple, 2=Crypto)
    pub auth_type: u16,
    /// 认证数据
    pub authentication: u64,
}

impl OspfHeader {
    /// OSPF 报文头部长度
    pub const LENGTH: usize = 24;

    /// 创建新的 OSPF 头部
    pub fn new(
        packet_type: OspfType,
        router_id: Ipv4Addr,
        area_id: Ipv4Addr,
    ) -> Self {
        Self {
            version: 2,  // OSPFv2
            packet_type,
            length: Self::LENGTH as u16,  // 初始值，后续会更新
            router_id,
            area_id,
            checksum: 0,  // 初始值，后续会计算
            auth_type: 0,
            authentication: 0,
        }
    }

    /// 从字节解析
    pub fn from_bytes(data: &[u8]) -> OspfResult<Self> {
        if data.len() < Self::LENGTH {
            return Err(OspfError::packet_too_short(Self::LENGTH, data.len()));
        }

        let version = data[0];
        if version != 2 {
            return Err(OspfError::parse_error("version", format!("expected 2, got {}", version)));
        }

        let packet_type = OspfType::from_u8(data[1])?;
        let length = u16::from_be_bytes([data[2], data[3]]);

        let router_id_bytes: [u8; 4] = data[4..8].try_into().unwrap();
        let router_id = Ipv4Addr::from_bytes(router_id_bytes);

        let area_id_bytes: [u8; 4] = data[8..12].try_into().unwrap();
        let area_id = Ipv4Addr::from_bytes(area_id_bytes);
        let checksum = u16::from_be_bytes([data[12], data[13]]);
        let auth_type = u16::from_be_bytes([data[14], data[15]]);
        let authentication = u64::from_be_bytes({
            let mut arr = [0u8; 8];
            arr.copy_from_slice(&data[16..24]);
            arr
        });

        Ok(Self {
            version,
            packet_type,
            length,
            router_id,
            area_id,
            checksum,
            auth_type,
            authentication,
        })
    }

    /// 转换为字节
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(Self::LENGTH);

        bytes.push(self.version);
        bytes.push(self.packet_type.into());
        bytes.extend_from_slice(&self.length.to_be_bytes());
        bytes.extend_from_slice(&self.router_id.as_bytes()[..]);
        bytes.extend_from_slice(&self.area_id.as_bytes()[..]);
        bytes.extend_from_slice(&self.checksum.to_be_bytes());
        bytes.extend_from_slice(&self.auth_type.to_be_bytes());
        bytes.extend_from_slice(&self.authentication.to_be_bytes());

        bytes
    }

    /// 计算校验和
    ///
    /// OSPF 使用与 IPv4 相同的校验和算法（RFC 1071）
    pub fn calculate_checksum(&mut self, payload: &[u8]) {
        // 临时设置校验和为 0
        self.checksum = 0;

        let mut header_bytes = self.to_bytes();
        header_bytes.extend_from_slice(payload);

        self.checksum = Self::compute_checksum(&header_bytes);
    }

    /// 计算校验和（RFC 1071）
    fn compute_checksum(data: &[u8]) -> u16 {
        let mut sum: u32 = 0;

        // 处理 16 位字
        let mut i = 0;
        while i < data.len() - 1 {
            let word = u16::from_be_bytes([data[i], data[i + 1]]) as u32;
            sum += word;
            i += 2;
        }

        // 处理奇数长度
        if i < data.len() {
            sum += (data[i] as u32) << 8;
        }

        // 处理进位
        while sum >> 16 != 0 {
            sum = (sum & 0xFFFF) + (sum >> 16);
        }

        // 取反
        !sum as u16
    }

    /// 转换为字节切片（临时实现）
    pub fn as_bytes(&self) -> Vec<u8> {
        self.to_bytes()
    }
}

/// OSPFv2 Hello 报文
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OspfHello {
    /// 网络掩码
    pub network_mask: Ipv4Addr,
    /// Hello 发送间隔（秒）
    pub hello_interval: u16,
    /// 选项位
    pub options: u8,
    /// 路由器优先级（用于 DR/BDR 选举）
    pub router_priority: u8,
    /// 路由器死亡间隔（秒）
    pub router_dead_interval: u32,
    /// 指定路由器
    pub designated_router: Ipv4Addr,
    /// 备份指定路由器
    pub backup_designated_router: Ipv4Addr,
    /// 邻居列表
    pub neighbors: Vec<Ipv4Addr>,
}

impl OspfHello {
    /// 最小 Hello 报文长度（头部 24 字节 + 至少一个邻居）
    pub const MIN_LENGTH: usize = 24 + 4;

    /// 创建新的 Hello 报文
    pub fn new(
        network_mask: Ipv4Addr,
        hello_interval: u16,
        router_dead_interval: u32,
        router_priority: u8,
    ) -> Self {
        Self {
            network_mask,
            hello_interval,
            options: 0,  // 默认无选项
            router_priority,
            router_dead_interval,
            designated_router: Ipv4Addr::unspecified(),
            backup_designated_router: Ipv4Addr::unspecified(),
            neighbors: Vec::new(),
        }
    }

    /// 从字节解析（不含 OSPF 头部）
    pub fn from_bytes(data: &[u8]) -> OspfResult<Self> {
        if data.len() < 20 {
            return Err(OspfError::packet_too_short(20, data.len()));
        }

        let network_mask_bytes: [u8; 4] = data[0..4].try_into().unwrap();
        let network_mask = Ipv4Addr::from_bytes(network_mask_bytes);

        let hello_interval = u16::from_be_bytes([data[4], data[5]]);
        let options = data[6];
        let router_priority = data[7];
        let router_dead_interval = u32::from_be_bytes({
            let mut arr = [0u8; 4];
            arr.copy_from_slice(&data[8..12]);
            arr
        });

        let dr_bytes: [u8; 4] = data[12..16].try_into().unwrap();
        let designated_router = Ipv4Addr::from_bytes(dr_bytes);

        let bdr_bytes: [u8; 4] = data[16..20].try_into().unwrap();
        let backup_designated_router = Ipv4Addr::from_bytes(bdr_bytes);

        // 解析邻居列表
        let mut neighbors = Vec::new();
        let mut offset = 20;
        while offset + 4 <= data.len() {
            let neighbor_bytes: [u8; 4] = data[offset..offset + 4].try_into().unwrap();
            let neighbor = Ipv4Addr::from_bytes(neighbor_bytes);
            neighbors.push(neighbor);
            offset += 4;
        }

        Ok(Self {
            network_mask,
            hello_interval,
            options,
            router_priority,
            router_dead_interval,
            designated_router,
            backup_designated_router,
            neighbors,
        })
    }

    /// 转换为字节（不含 OSPF 头部）
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();

        bytes.extend_from_slice(&self.network_mask.as_bytes()[..]);
        bytes.extend_from_slice(&self.hello_interval.to_be_bytes());
        bytes.push(self.options);
        bytes.push(self.router_priority);
        bytes.extend_from_slice(&self.router_dead_interval.to_be_bytes());
        bytes.extend_from_slice(&self.designated_router.as_bytes()[..]);
        bytes.extend_from_slice(&self.backup_designated_router.as_bytes()[..]);

        for neighbor in &self.neighbors {
            bytes.extend_from_slice(&neighbor.as_bytes()[..]);
        }

        bytes
    }

    /// 添加邻居
    pub fn add_neighbor(&mut self, neighbor: Ipv4Addr) {
        if !self.neighbors.contains(&neighbor) {
            self.neighbors.push(neighbor);
        }
    }

    /// 移除邻居
    pub fn remove_neighbor(&mut self, neighbor: Ipv4Addr) {
        self.neighbors.retain(|&n| n != neighbor);
    }

    /// 获取报文长度
    pub fn length(&self) -> usize {
        20 + self.neighbors.len() * 4
    }

    /// 转换为字节切片（临时实现）
    pub fn as_bytes(&self) -> Vec<u8> {
        self.to_bytes()
    }
}

/// OSPFv2 Database Description 报文
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OspfDatabaseDescription {
    /// 接口 MTU
    pub interface_mtu: u16,
    /// 选项位
    pub options: u8,
    /// I 位：Initialize
    pub i_bit: bool,
    /// M 位：More
    pub m_bit: bool,
    /// MS 位：Master/Slave
    pub ms_bit: bool,
    /// 数据库描述序列号
    pub dd_sequence_number: u32,
    /// LSA 头部列表
    pub lsa_headers: Vec<super::lsa::LsaHeader>,
}

impl OspfDatabaseDescription {
    /// 最小 DD 报文长度
    pub const MIN_LENGTH: usize = 8;

    /// 创建新的 DD 报文
    pub fn new(interface_mtu: u16, dd_sequence_number: u32) -> Self {
        Self {
            interface_mtu,
            options: 0,
            i_bit: false,
            m_bit: false,
            ms_bit: false,
            dd_sequence_number,
            lsa_headers: Vec::new(),
        }
    }

    /// 设置选项
    pub fn with_options(mut self, options: u8) -> Self {
        self.options = options;
        self
    }

    /// 设置 I/M/MS 位
    pub fn with_flags(mut self, i: bool, m: bool, ms: bool) -> Self {
        self.i_bit = i;
        self.m_bit = m;
        self.ms_bit = ms;
        self
    }

    /// 转换为字节
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();

        bytes.extend_from_slice(&self.interface_mtu.to_be_bytes());
        bytes.push(self.options);
        bytes.push(0);  // 保留字段
        bytes.extend_from_slice(&self.dd_sequence_number.to_be_bytes());

        for lsa_header in &self.lsa_headers {
            bytes.extend_from_slice(&lsa_header.to_bytes()[..]);
        }

        bytes
    }
}

/// OSPFv2 Link State Request 报文条目
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LsaRequest {
    /// LSA 类型
    pub lsa_type: u8,
    /// 链路状态 ID
    pub link_state_id: Ipv4Addr,
    /// 通告路由器
    pub advertising_router: Ipv4Addr,
}

impl LsaRequest {
    /// LSA 请求条目长度
    pub const LENGTH: usize = 12;

    /// 创建新的 LSA 请求
    pub fn new(lsa_type: u8, link_state_id: Ipv4Addr, advertising_router: Ipv4Addr) -> Self {
        Self {
            lsa_type,
            link_state_id,
            advertising_router,
        }
    }

    /// 转换为字节
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(Self::LENGTH);

        bytes.push(self.lsa_type);
        bytes.extend_from_slice(&[0u8, 0x1u8, 0x1u8]);  // 保留字段
        bytes.extend_from_slice(&self.link_state_id.as_bytes()[..]);
        bytes.extend_from_slice(&self.advertising_router.as_bytes()[..]);

        bytes
    }

    /// 转换为字节切片（临时实现）
    pub fn as_bytes(&self) -> Vec<u8> {
        self.to_bytes()
    }
}

/// OSPFv2 Link State Request 报文
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OspfLinkStateRequest {
    /// LSA 请求列表
    pub requests: Vec<LsaRequest>,
}

impl OspfLinkStateRequest {
    /// 创建新的 LSR 报文
    pub fn new() -> Self {
        Self {
            requests: Vec::new(),
        }
    }

    /// 添加请求
    pub fn add_request(&mut self, request: LsaRequest) {
        self.requests.push(request);
    }

    /// 转换为字节
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();

        for request in &self.requests {
            bytes.extend_from_slice(&request.as_bytes());
        }

        bytes
    }
}

/// OSPFv2 Link State Update 报文
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OspfLinkStateUpdate {
    /// LSA 数量
    pub lsa_count: u32,
    /// LSA 列表
    pub lsas: Vec<super::lsa::Lsa>,
}

impl OspfLinkStateUpdate {
    /// 创建新的 LSU 报文
    pub fn new() -> Self {
        Self {
            lsa_count: 0,
            lsas: Vec::new(),
        }
    }

    /// 添加 LSA
    pub fn add_lsa(&mut self, lsa: super::lsa::Lsa) {
        self.lsa_count += 1;
        self.lsas.push(lsa);
    }

    /// 转换为字节
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();

        bytes.extend_from_slice(&self.lsa_count.to_be_bytes());

        for lsa in &self.lsas {
            bytes.extend_from_slice(&lsa.as_bytes());
        }

        bytes
    }
}

/// OSPFv2 Link State Acknowledgment 报文
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OspfLinkStateAck {
    /// LSA 头部列表
    pub lsa_headers: Vec<super::lsa::LsaHeader>,
}

impl OspfLinkStateAck {
    /// 创建新的 LSAck 报文
    pub fn new() -> Self {
        Self {
            lsa_headers: Vec::new(),
        }
    }

    /// 添加 LSA 头部
    pub fn add_lsa_header(&mut self, header: super::lsa::LsaHeader) {
        self.lsa_headers.push(header);
    }

    /// 转换为字节
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();

        for header in &self.lsa_headers {
            bytes.extend_from_slice(&header.as_bytes());
        }

        bytes
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ospf_type_from_u8() {
        assert_eq!(OspfType::from_u8(1).unwrap(), OspfType::Hello);
        assert_eq!(OspfType::from_u8(2).unwrap(), OspfType::DatabaseDescription);
        assert!(OspfType::from_u8(99).is_err());
    }

    #[test]
    fn test_ospf_type_to_u8() {
        assert_eq!(u8::from(OspfType::Hello), 1);
        assert_eq!(u8::from(OspfType::LinkStateUpdate), 4);
    }

    #[test]
    fn test_ospf_header_new() {
        let header = OspfHeader::new(
            OspfType::Hello,
            Ipv4Addr::new(1, 1, 1, 1),
            Ipv4Addr::new(0, 0, 0, 0),
        );

        assert_eq!(header.version, 2);
        assert_eq!(header.packet_type, OspfType::Hello);
        assert_eq!(header.router_id, Ipv4Addr::new(1, 1, 1, 1));
    }

    #[test]
    fn test_ospf_header_round_trip() {
        let original = OspfHeader::new(
            OspfType::Hello,
            Ipv4Addr::new(1, 2, 3, 4),
            Ipv4Addr::new(0, 0, 0, 1),
        );

        let bytes = original.as_bytes();
        let parsed = OspfHeader::from_bytes(&bytes).unwrap();

        assert_eq!(parsed.version, original.version);
        assert_eq!(parsed.packet_type, original.packet_type);
        assert_eq!(parsed.router_id, original.router_id);
        assert_eq!(parsed.area_id, original.area_id);
    }

    #[test]
    fn test_ospf_hello_new() {
        let hello = OspfHello::new(
            Ipv4Addr::new(255, 255, 255, 0),
            10,
            40,
            1,
        );

        assert_eq!(hello.network_mask, Ipv4Addr::new(255, 255, 255, 0));
        assert_eq!(hello.hello_interval, 10);
        assert_eq!(hello.router_dead_interval, 40);
        assert_eq!(hello.router_priority, 1);
    }

    #[test]
    fn test_ospf_hello_round_trip() {
        let mut original = OspfHello::new(
            Ipv4Addr::new(255, 255, 255, 0),
            10,
            40,
            1,
        );
        original.add_neighbor(Ipv4Addr::new(1, 1, 1, 2));
        original.add_neighbor(Ipv4Addr::new(1, 1, 1, 3));

        let bytes = original.as_bytes();
        let parsed = OspfHello::from_bytes(&bytes).unwrap();

        assert_eq!(parsed.network_mask, original.network_mask);
        assert_eq!(parsed.hello_interval, original.hello_interval);
        assert_eq!(parsed.neighbors.len(), 2);
    }

    #[test]
    fn test_lsa_request_as_bytes() {
        let request = LsaRequest::new(
            1,
            Ipv4Addr::new(1, 1, 1, 1),
            Ipv4Addr::new(2, 2, 2, 2),
        );

        let bytes = request.as_bytes();
        assert_eq!(bytes.len(), LsaRequest::LENGTH);
    }
}
