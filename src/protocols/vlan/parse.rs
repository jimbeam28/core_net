// src/protocols/vlan/parse.rs
//
// VLAN解析和封装辅助函数

use crate::protocols::Packet;

/// 检查以太网帧是否包含VLAN标签
///
/// # 参数
/// - packet: Packet引用
///
/// # 返回
/// - Some(tpid): 包含VLAN标签，返回TPID
/// - None: 不包含VLAN标签
///
/// # 行为
/// - 检查当前offset位置的EtherType是否为VLAN TPID
/// - 不移动offset
pub fn has_vlan_tag(packet: &Packet) -> Option<u16> {
    if packet.remaining() < 2 {
        return None;
    }

    let ether_type_bytes = packet.peek(2)?;
    let ether_type = u16::from_be_bytes([ether_type_bytes[0], ether_type_bytes[1]]);

    if is_vlan_tpid(ether_type) {
        Some(ether_type)
    } else {
        None
    }
}

/// 检查指定的EtherType是否为VLAN TPID
///
/// # 参数
/// - ether_type: 以太网类型字段
///
/// # 返回
/// - true: 是VLAN TPID
/// - false: 不是VLAN TPID
pub fn is_vlan_tpid(ether_type: u16) -> bool {
    match ether_type {
        0x8100 => true,  // 802.1Q
        0x9100 => true,  // Q-in-Q
        0x88A8 => true,  // 802.1ad Provider Bridge
        _ => false,
    }
}
