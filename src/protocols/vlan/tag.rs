// src/protocols/vlan/tag.rs
//
// 802.1Q VLAN标签结构定义和实现

use crate::protocols::Packet;
use super::error::VlanError;

/// 802.1Q VLAN标签
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VlanTag {
    /// 优先级代码点 (Priority Code Point), 3 bits, 范围 0-7
    pub pcp: u8,

    /// 丢弃指示 (Drop Eligible Indicator), 1 bit
    pub dei: bool,

    /// VLAN标识符 (VLAN Identifier), 12 bits, 范围 0-4095
    /// 有效范围: 1-4094 (0保留，4095预留)
    pub vid: u16,
}

impl VlanTag {
    /// 创建新的VLAN标签
    ///
    /// # 参数
    /// - pcp: 优先级 (0-7)
    /// - dei: 丢弃指示
    /// - vid: VLAN ID (1-4094)
    ///
    /// # 返回
    /// - Ok(VlanTag): 创建成功
    /// - Err(VlanError): 参数无效
    pub fn new(pcp: u8, dei: bool, vid: u16) -> Result<Self, VlanError> {
        if !Self::is_valid_pcp(pcp) {
            return Err(VlanError::InvalidPcp { pcp });
        }
        if !Self::is_valid_vid(vid) {
            return Err(VlanError::InvalidVlanId { vid });
        }
        Ok(VlanTag { pcp, dei, vid })
    }

    /// 创建默认VLAN标签 (PCP=0, DEI=false, VID=1)
    pub fn default() -> Self {
        VlanTag {
            pcp: 0,
            dei: false,
            vid: 1,
        }
    }

    /// 验证VLAN ID是否有效
    ///
    /// 有效范围: 1-4094
    pub fn is_valid_vid(vid: u16) -> bool {
        (1..=4094).contains(&vid)
    }

    /// 验证PCP是否有效
    ///
    /// 有效范围: 0-7
    pub fn is_valid_pcp(pcp: u8) -> bool {
        pcp <= 7
    }

    /// 将VLAN标签编码为2字节 (网络字节序)
    pub fn to_bytes(&self) -> [u8; 2] {
        // TCI格式: [PCP(3bit) | DEI(1bit) | VID(高12bit)] [VID(低12bit补全)]
        let value = ((self.pcp as u16) << 13)
            | ((self.dei as u16) << 12)
            | (self.vid & 0x0FFF);
        value.to_be_bytes()
    }

    /// 从2字节解析VLAN标签 (网络字节序)
    pub fn from_bytes(data: [u8; 2]) -> Result<Self, VlanError> {
        let value = u16::from_be_bytes(data);
        let pcp = ((value >> 13) & 0x07) as u8;
        let dei = (value & 0x1000) != 0;
        let vid = value & 0x0FFF;

        Self::new(pcp, dei, vid)
    }

    /// 从Packet中解析VLAN标签
    ///
    /// # 参数
    /// - packet: 可变引用的Packet (读取后会移动offset)
    ///
    /// # 返回
    /// - Ok(VlanTag): 解析成功
    /// - Err(VlanError): 解析失败
    ///
    /// # 行为
    /// - 从当前offset读取2字节
    /// - 自动移动offset 2字节
    pub fn parse_from_packet(packet: &mut Packet) -> Result<Self, VlanError> {
        if packet.remaining() < 2 {
            return Err(VlanError::InsufficientPacketLength {
                expected: 2,
                actual: packet.remaining(),
            });
        }

        let data = packet.read(2).ok_or_else(|| VlanError::ParseError(
            "读取VLAN标签失败".to_string()
        ))?;

        let bytes = [data[0], data[1]];
        Self::from_bytes(bytes)
    }

    /// 查看Packet中的VLAN标签 (不移动offset)
    pub fn peek_from_packet(packet: &Packet) -> Result<Self, VlanError> {
        if packet.remaining() < 2 {
            return Err(VlanError::InsufficientPacketLength {
                expected: 2,
                actual: packet.remaining(),
            });
        }

        let data = packet.peek(2).ok_or_else(|| VlanError::ParseError(
            "查看VLAN标签失败".to_string()
        ))?;

        let bytes = [data[0], data[1]];
        Self::from_bytes(bytes)
    }

    /// 将VLAN标签写入Packet (在当前位置插入)
    ///
    /// # 参数
    /// - packet: 可变引用的Packet
    /// - tpid: 标签协议标识符 (默认0x8100)
    ///
    /// # 返回
    /// - Ok(()): 写入成功
    /// - Err(VlanError): 写入失败
    ///
    /// # 行为
    /// - 在当前offset位置写入TPID (2字节)
    /// - 在TPID后写入VLAN标签 (2字节)
    /// - 移动offset 4字节
    pub fn write_to_packet(&self, packet: &mut Packet, tpid: u16) -> Result<(), VlanError> {
        let tpid_bytes = tpid.to_be_bytes();
        let tag_bytes = self.to_bytes();

        packet.extend(&tpid_bytes);
        packet.extend(&tag_bytes);

        Ok(())
    }

    /// 追加VLAN标签到Packet末尾
    pub fn append_to_packet(&self, packet: &mut Packet, tpid: u16) -> Result<(), VlanError> {
        self.write_to_packet(packet, tpid)
    }
}

// 实现Default trait
impl Default for VlanTag {
    fn default() -> Self {
        Self::default()
    }
}
