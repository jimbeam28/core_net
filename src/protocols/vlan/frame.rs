// src/protocols/vlan/frame.rs
//
// VLAN帧封装信息结构

use super::tag::VlanTag;

/// VLAN帧封装信息
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VlanFrame {
    /// VLAN标签
    pub tag: VlanTag,

    /// 标签协议标识符 (Tag Protocol Identifier)
    /// 0x8100: 标准802.1Q
    /// 0x9100: Q-in-Q
    /// 0x88A8: 802.1ad Provider Bridge
    pub tpid: u16,
}

impl VlanFrame {
    /// 创建新的VLAN帧封装信息
    ///
    /// # 参数
    /// - tag: VLAN标签
    /// - tpid: 标签协议标识符 (默认0x8100)
    pub fn new(tag: VlanTag, tpid: u16) -> Self {
        VlanFrame { tag, tpid }
    }

    /// 创建标准802.1Q VLAN帧 (TPID=0x8100)
    pub fn standard_8021q(tag: VlanTag) -> Self {
        VlanFrame {
            tag,
            tpid: 0x8100,
        }
    }
}
