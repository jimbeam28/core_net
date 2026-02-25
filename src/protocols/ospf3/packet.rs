// src/protocols/ospf3/packet.rs
//
// OSPFv3 报文结构定义和解析/封装函数

use super::error::{Ospfv3Error, Ospfv3Result};

/// OSPFv3 通用报文头部
///
/// OSPFv3 报文直接封装在 IPv6 中（Next Header = 89）
///
/// 根据 RFC 5340，OSPFv3 头部为 16 字节：
/// - Version (1 byte) + Type (1 byte) + Length (2 bytes) = 4 bytes
/// - Router ID (4 bytes) = 4 bytes
/// - Area ID (4 bytes) = 4 bytes
/// - Checksum (2 bytes) + Instance ID (2 bytes) + Reserved (2 bytes) = 6 bytes
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ospfv3Header {
    /// OSPF 版本号 (OSPFv3 = 3)
    pub version: u8,
    /// 报文类型 (1-5)
    pub packet_type: u8,
    /// 报文总长度（含头部）
    pub length: u16,
    /// 路由器 ID (32-bit)
    pub router_id: u32,
    /// 区域 ID (32-bit)
    pub area_id: u32,
    /// 校验和
    pub checksum: u16,
    /// 实例 ID
    pub instance_id: u16,
    /// 保留字段 (2 bytes)
    pub reserved: u16,
}

impl Ospfv3Header {
    /// OSPFv3 报文头部长度
    /// 根据 RFC 5340：Version(1) + Type(1) + Length(2) + RouterID(4) + AreaID(4) + Checksum(2) + InstanceID(2) + Reserved(2) = 18
    pub const LENGTH: usize = 18;

    /// 创建新的 OSPFv3 头部
    pub fn new(packet_type: u8, router_id: u32, area_id: u32) -> Self {
        Self {
            version: 3,
            packet_type,
            length: Self::LENGTH as u16,
            router_id,
            area_id,
            checksum: 0,
            instance_id: 0,
            reserved: 0,
        }
    }

    /// 从字节解析
    pub fn from_bytes(data: &[u8]) -> Ospfv3Result<Self> {
        if data.len() < Self::LENGTH {
            return Err(Ospfv3Error::packet_too_short(Self::LENGTH, data.len()));
        }

        let version = data[0];
        if version != 3 {
            return Err(Ospfv3Error::VersionMismatch { expected: 3, actual: version });
        }

        let packet_type = data[1];
        let length = u16::from_be_bytes([data[2], data[3]]);

        let router_id = u32::from_be_bytes([data[4], data[5], data[6], data[7]]);
        let area_id = u32::from_be_bytes([data[8], data[9], data[10], data[11]]);

        let checksum = u16::from_be_bytes([data[12], data[13]]);
        let instance_id = u16::from_be_bytes([data[14], data[15]]);
        let reserved = u16::from_be_bytes([data[16], data[17]]);

        Ok(Self {
            version,
            packet_type,
            length,
            router_id,
            area_id,
            checksum,
            instance_id,
            reserved,
        })
    }

    /// 转换为字节
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(Self::LENGTH);

        bytes.push(self.version);
        bytes.push(self.packet_type);
        bytes.extend_from_slice(&self.length.to_be_bytes());
        bytes.extend_from_slice(&self.router_id.to_be_bytes());
        bytes.extend_from_slice(&self.area_id.to_be_bytes());
        bytes.extend_from_slice(&self.checksum.to_be_bytes());
        bytes.extend_from_slice(&self.instance_id.to_be_bytes());
        bytes.extend_from_slice(&self.reserved.to_be_bytes());

        bytes
    }

    /// 转换为字节切片
    pub fn as_bytes(&self) -> Vec<u8> {
        self.to_bytes()
    }

    /// 计算校验和（RFC 1071）
    pub fn calculate_checksum(&mut self, payload: &[u8]) {
        self.checksum = 0;

        let mut header_bytes = self.to_bytes();
        header_bytes.extend_from_slice(payload);

        self.checksum = Self::compute_checksum(&header_bytes);
    }

    fn compute_checksum(data: &[u8]) -> u16 {
        let mut sum: u32 = 0;

        let mut i = 0;
        while i < data.len() - 1 {
            let word = u16::from_be_bytes([data[i], data[i + 1]]) as u32;
            sum += word;
            i += 2;
        }

        if i < data.len() {
            sum += (data[i] as u32) << 8;
        }

        while sum >> 16 != 0 {
            sum = (sum & 0xFFFF) + (sum >> 16);
        }

        !sum as u16
    }
}

/// OSPFv3 Hello 报文
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ospfv3Hello {
    /// 接口 ID
    pub interface_id: u32,
    /// 路由器优先级（用于 DR/BDR 选举）
    pub router_priority: u8,
    /// 选项位（24 位：1 字节保留 + 2 字节选项 + 1 字节保留）
    pub options: u32,
    /// Hello 发送间隔（秒）
    pub hello_interval: u16,
    /// 路由器死亡间隔（秒）
    pub router_dead_interval: u32,
    /// 指定路由器 ID (32-bit)
    pub designated_router: u32,
    /// 备份指定路由器 ID (32-bit)
    pub backup_designated_router: u32,
    /// 邻居路由器 ID 列表 (32-bit Router IDs)
    pub neighbors: Vec<u32>,
}

impl Ospfv3Hello {
    /// 最小 Hello 报文长度（不含 OSPF 头部）
    pub const MIN_LENGTH: usize = 22;

    /// 创建新的 Hello 报文
    pub fn new(
        interface_id: u32,
        hello_interval: u16,
        router_dead_interval: u32,
        router_priority: u8,
    ) -> Self {
        Self {
            interface_id,
            hello_interval,
            options: 0,
            router_priority,
            router_dead_interval,
            designated_router: 0,
            backup_designated_router: 0,
            neighbors: Vec::new(),
        }
    }

    /// 从字节解析（不含 OSPF 头部）
    pub fn from_bytes(data: &[u8]) -> Ospfv3Result<Self> {
        if data.len() < Self::MIN_LENGTH {
            return Err(Ospfv3Error::packet_too_short(Self::MIN_LENGTH, data.len()));
        }

        let interface_id = u32::from_be_bytes([data[0], data[1], data[2], data[3]]);
        let router_priority = data[4];
        // Options 是 3 字节 (24 位)
        let options = u32::from_be_bytes([0, data[5], data[6], data[7]]);
        let hello_interval = u16::from_be_bytes([data[8], data[9]]);
        let router_dead_interval = u32::from_be_bytes({
            let mut arr = [0u8; 4];
            arr.copy_from_slice(&data[10..14]);
            arr
        });

        let designated_router = u32::from_be_bytes({
            let mut arr = [0u8; 4];
            arr.copy_from_slice(&data[14..18]);
            arr
        });

        let backup_designated_router = u32::from_be_bytes({
            let mut arr = [0u8; 4];
            arr.copy_from_slice(&data[18..22]);
            arr
        });

        // 解析邻居列表
        let mut neighbors = Vec::new();
        let mut offset = 22;
        while offset + 4 <= data.len() {
            let neighbor_bytes: [u8; 4] = data[offset..offset + 4].try_into().unwrap();
            let neighbor = u32::from_be_bytes(neighbor_bytes);
            neighbors.push(neighbor);
            offset += 4;
        }

        Ok(Self {
            interface_id,
            router_priority,
            options,
            hello_interval,
            router_dead_interval,
            designated_router,
            backup_designated_router,
            neighbors,
        })
    }

    /// 转换为字节（不含 OSPF 头部）
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();

        bytes.extend_from_slice(&self.interface_id.to_be_bytes());
        bytes.push(self.router_priority);
        // Options 是 3 字节 (24 位)
        bytes.extend_from_slice(&self.options.to_be_bytes()[1..4]);  // 只取低 3 字节
        bytes.extend_from_slice(&self.hello_interval.to_be_bytes());
        bytes.extend_from_slice(&self.router_dead_interval.to_be_bytes());
        bytes.extend_from_slice(&self.designated_router.to_be_bytes());
        bytes.extend_from_slice(&self.backup_designated_router.to_be_bytes());

        for neighbor in &self.neighbors {
            bytes.extend_from_slice(&neighbor.to_be_bytes());
        }

        bytes
    }

    /// 转换为字节切片
    pub fn as_bytes(&self) -> Vec<u8> {
        self.to_bytes()
    }

    /// 添加邻居
    pub fn add_neighbor(&mut self, neighbor: u32) {
        if !self.neighbors.contains(&neighbor) {
            self.neighbors.push(neighbor);
        }
    }

    /// 获取报文长度（不含 OSPF 头部）
    pub fn length(&self) -> usize {
        // Interface ID (4) + Priority (1) + Options (3) + Hello Interval (2) +
        // Router Dead Interval (4) + DR (4) + BDR (4) + Neighbors (4 each)
        22 + self.neighbors.len() * 4
    }
}

/// OSPFv3 Database Description 报文
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ospfv3DatabaseDescription {
    /// 接口 MTU
    pub interface_mtu: u16,
    /// 选项位（16 位）
    pub options: u16,
    /// I 位：Initialize
    pub i_bit: bool,
    /// M 位：More
    pub m_bit: bool,
    /// MS 位：Master/Slave
    pub ms_bit: bool,
    /// 数据库描述序列号
    pub dd_sequence_number: u32,
    /// LSA 头部列表
    pub lsa_headers: Vec<LsaHeader>,
}

impl Ospfv3DatabaseDescription {
    /// 最小 DD 报文长度（不含 OSPF 头部）
    /// MTU(2) + Options(2) + Flags(1) + Reserved(1) + DD Sequence(4) = 10
    pub const MIN_LENGTH: usize = 10;

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

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();

        bytes.extend_from_slice(&self.interface_mtu.to_be_bytes());
        bytes.extend_from_slice(&self.options.to_be_bytes());

        let mut flags = 0u8;
        if self.i_bit { flags |= 0x04; }
        if self.m_bit { flags |= 0x02; }
        if self.ms_bit { flags |= 0x01; }
        bytes.push(flags);
        bytes.push(0); // 保留字段

        bytes.extend_from_slice(&self.dd_sequence_number.to_be_bytes());

        for lsa_header in &self.lsa_headers {
            bytes.extend_from_slice(&lsa_header.to_bytes()[..]);
        }

        bytes
    }
}

/// OSPFv3 LSA 头部
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LsaHeader {
    /// LSA 年龄（秒）
    pub age: u16,
    /// LSA 类型
    pub lsa_type: u16,
    /// LSA 链路状态 ID
    pub link_state_id: u32,
    /// 通告路由器
    pub advertising_router: u32,
    /// LSA 序列号
    pub sequence_number: u32,
    /// LSA 校验和
    pub checksum: u16,
    /// LSA 长度
    pub length: u16,
}

impl LsaHeader {
    pub const LENGTH: usize = 20;

    pub fn new(lsa_type: u16, link_state_id: u32, advertising_router: u32) -> Self {
        Self {
            age: 0,
            lsa_type,
            link_state_id,
            advertising_router,
            sequence_number: 0x80000001,
            checksum: 0,
            length: Self::LENGTH as u16,
        }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(Self::LENGTH);

        bytes.extend_from_slice(&self.age.to_be_bytes());
        bytes.extend_from_slice(&self.lsa_type.to_be_bytes());
        bytes.extend_from_slice(&self.link_state_id.to_be_bytes());
        bytes.extend_from_slice(&self.advertising_router.to_be_bytes());
        bytes.extend_from_slice(&self.sequence_number.to_be_bytes());
        bytes.extend_from_slice(&self.checksum.to_be_bytes());
        bytes.extend_from_slice(&self.length.to_be_bytes());

        bytes
    }

    pub fn as_bytes(&self) -> Vec<u8> {
        self.to_bytes()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ospfv3_header_new() {
        let router_id: u32 = 0x01020304;
        let area_id: u32 = 0x00000001;

        let header = Ospfv3Header::new(1, router_id, area_id);

        assert_eq!(header.version, 3);
        assert_eq!(header.packet_type, 1);
        assert_eq!(header.router_id, router_id);
    }

    #[test]
    fn test_ospfv3_hello_new() {
        let hello = Ospfv3Hello::new(1, 10, 40, 1);

        assert_eq!(hello.interface_id, 1);
        assert_eq!(hello.hello_interval, 10);
        assert_eq!(hello.router_dead_interval, 40);
        assert_eq!(hello.router_priority, 1);
    }

    #[test]
    fn test_ospfv3_hello_add_neighbor() {
        let mut hello = Ospfv3Hello::new(1, 10, 40, 1);

        let neighbor: u32 = 0x01020304;
        hello.add_neighbor(neighbor);

        assert_eq!(hello.neighbors.len(), 1);
    }

    #[test]
    fn test_ospfv3_hello_length() {
        let hello = Ospfv3Hello::new(1, 10, 40, 1);

        // 空邻居列表：22 字节
        assert_eq!(hello.length(), 22);

        let mut hello2 = hello.clone();
        hello2.add_neighbor(0x01020304);

        // 1 个邻居：22 + 4 = 26 字节
        assert_eq!(hello2.length(), 26);
    }
}
