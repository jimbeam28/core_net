// src/protocols/ospf3/packet.rs
//
// OSPFv3 报文结构定义（简化版）

/// OSPFv3 通用报文头部
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
    pub const LENGTH: usize = 18;
}

/// OSPFv3 Hello 报文
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ospfv3Hello {
    /// 接口 ID
    pub interface_id: u32,
    /// 路由器优先级（用于 DR/BDR 选举）
    pub router_priority: u8,
    /// 选项位（24 位）
    pub options: u32,
    /// Hello 发送间隔（秒）
    pub hello_interval: u16,
    /// 路由器死亡间隔（秒）
    pub router_dead_interval: u32,
    /// 指定路由器 ID (32-bit)
    pub designated_router: u32,
    /// 备份指定路由器 ID (32-bit)
    pub backup_designated_router: u32,
    /// 邻居路由器 ID 列表
    pub neighbors: Vec<u32>,
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
}
