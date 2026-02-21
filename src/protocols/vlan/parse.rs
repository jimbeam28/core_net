// src/protocols/vlan/parse.rs
//
// VLAN解析和封装辅助函数

use crate::protocols::Packet;
use crate::protocols::vlan::VlanTag;
use crate::protocols::vlan::error::VlanError;

/// VLAN 处理结果
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VlanProcessResult {
    /// 内层 EtherType (去掉 VLAN 标签后的协议类型)
    pub inner_type: u16,
    /// 外层 VLAN 标签（如果存在）
    pub outer_vlan: Option<VlanTag>,
    /// 内层 VLAN 标签（如果存在，QinQ 场景）
    pub inner_vlan: Option<VlanTag>,
}

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

/// 处理 VLAN 报文
///
/// # 参数
/// - packet: 可变 Packet 引用
///
/// # 返回
/// - Ok(VlanProcessResult): 处理成功，返回内层协议类型和 VLAN 信息
/// - Err(VlanError): 处理失败
///
/// # 行为
/// - 解析外层 VLAN 标签（如果存在）
/// - 解析内层 VLAN 标签（如果存在，QinQ）
/// - 设置 packet.vlan_id（内层 VLAN 优先）
/// - 读取并返回内层 EtherType
pub fn process_vlan_packet(packet: &mut Packet) -> Result<VlanProcessResult, VlanError> {
    // 检测外层 VLAN 标签
    let tpid_opt = has_vlan_tag(packet);
    if tpid_opt.is_none() {
        return Err(VlanError::ParseError("No VLAN tag detected".to_string()));
    }
    let _tpid = tpid_opt.unwrap();

    // 跳过 TPID (2字节)，然后读取 TCI
    if !packet.skip(2) {
        return Err(VlanError::InsufficientPacketLength {
            expected: 2,
            actual: packet.remaining(),
        });
    }

    // 解析外层 VLAN 标签 (TCI)
    let outer_vlan = VlanTag::parse_from_packet(packet)?;

    // 设置外层 VLAN ID 到 packet（对于 QinQ，后续会被内层覆盖）
    packet.set_vlan_id(outer_vlan.vid);

    // 检测是否有内层 VLAN 标签（QinQ）
    let inner_vlan_opt = if has_vlan_tag(packet).is_some() {
        // 跳过内层 TPID
        if !packet.skip(2) {
            return Err(VlanError::InsufficientPacketLength {
                expected: 2,
                actual: packet.remaining(),
            });
        }
        Some(VlanTag::parse_from_packet(packet)?)
    } else {
        None
    };

    // 如果有内层 VLAN，使用内层 VLAN ID（用户 VLAN 更具有业务意义）
    if let Some(ref inner) = inner_vlan_opt {
        packet.set_vlan_id(inner.vid);
    }

    // 读取内层 EtherType
    if packet.remaining() < 2 {
        return Err(VlanError::InsufficientPacketLength {
            expected: 2,
            actual: packet.remaining(),
        });
    }

    let inner_type_bytes = packet.read(2).ok_or_else(|| VlanError::ParseError(
        "Failed to read inner ethertype".to_string()
    ))?;
    let inner_type = u16::from_be_bytes([inner_type_bytes[0], inner_type_bytes[1]]);

    Ok(VlanProcessResult {
        inner_type,
        outer_vlan: Some(outer_vlan),
        inner_vlan: inner_vlan_opt,
    })
}
