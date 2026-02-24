// src/protocols/vlan/parse.rs
//
// VLAN解析和封装辅助函数

use crate::protocols::{MacAddr, Packet};
use crate::protocols::vlan::VlanTag;
use crate::protocols::vlan::error::VlanError;

// ========== 封装常量 ==========

/// 标准 802.1Q TPID
pub const TPID_8021Q: u16 = 0x8100;
/// Q-in-Q TPID
pub const TPID_QINQ: u16 = 0x9100;
/// 802.1ad Provider Bridge TPID
pub const TPID_8021AD: u16 = 0x88A8;

// ========== 封装参数结构体 ==========

/// VLAN 封装参数
///
/// 用于封装单标签 VLAN 帧。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VlanEncapParams {
    /// 目标 MAC 地址
    pub dst_mac: MacAddr,
    /// 源 MAC 地址
    pub src_mac: MacAddr,
    /// VLAN 标签
    pub vlan_tag: VlanTag,
    /// TPID（标签协议标识符，通常 0x8100）
    pub tpid: u16,
    /// 内层协议类型（如 0x0800 表示 IPv4）
    pub inner_type: u16,
}

/// QinQ 封装参数
///
/// 用于封装双标签 VLAN 帧（Q-in-Q）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QinQEncapParams {
    /// 目标 MAC 地址
    pub dst_mac: MacAddr,
    /// 源 MAC 地址
    pub src_mac: MacAddr,
    /// 外层 VLAN 标签（服务 VLAN）
    pub outer_tag: VlanTag,
    /// 外层 TPID（通常 0x9100 或 0x88A8）
    pub outer_tpid: u16,
    /// 内层 VLAN 标签（用户 VLAN）
    pub inner_tag: VlanTag,
    /// 内层 TPID（通常 0x8100）
    pub inner_tpid: u16,
    /// 内层协议类型（如 0x0800 表示 IPv4）
    pub inner_type: u16,
}

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
    packet.vlan_id = outer_vlan.vid;

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
        packet.vlan_id = inner.vid;
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

// ========== VLAN 封装函数 ==========

/// 封装 VLAN 帧（单标签）
///
/// 构造带单个 VLAN 标签的以太网帧。
///
/// # 参数
/// - dst_mac: 目标 MAC 地址
/// - src_mac: 源 MAC 地址
/// - vlan_tag: VLAN 标签
/// - tpid: 标签协议标识符（默认 0x8100）
/// - inner_type: 内层协议类型（如 0x0800 表示 IPv4）
/// - payload: 负载数据
///
/// # 返回
/// - Vec<u8>: 完整的 VLAN 标记以太网帧
///
/// # 帧格式
/// 注：本实现不包含 FCS（由硬件添加）
/// ```text
/// | DA(6) | SA(6) | TPID(2) | TCI(2) | Type(2) | Payload | FCS(4) |
/// ```
///
/// # 参数
/// - params: VLAN 封装参数
/// - payload: 负载数据
pub fn encapsulate_vlan_frame_with_params(params: VlanEncapParams, payload: &[u8]) -> Vec<u8> {
    // 计算总长度：以太网头部(14) + VLAN标签(4) + 负载
    let total_len = 14 + 4 + payload.len();
    let mut frame = Vec::with_capacity(total_len);

    // 以太网头部
    frame.extend_from_slice(&params.dst_mac.bytes);
    frame.extend_from_slice(&params.src_mac.bytes);

    // VLAN 标签：TPID + TCI
    frame.extend_from_slice(&params.tpid.to_be_bytes());
    frame.extend_from_slice(&params.vlan_tag.to_bytes());

    // 内层协议类型
    frame.extend_from_slice(&params.inner_type.to_be_bytes());

    // 负载
    frame.extend_from_slice(payload);

    frame
}

/// 封装 VLAN 帧（单标签）- 便捷函数
///
/// 构造带单个 VLAN 标签的以太网帧。
///
/// # 参数
/// - dst_mac: 目标 MAC 地址
/// - src_mac: 源 MAC 地址
/// - vlan_tag: VLAN 标签
/// - tpid: 标签协议标识符（默认 0x8100）
/// - inner_type: 内层协议类型（如 0x0800 表示 IPv4）
/// - payload: 负载数据
///
/// # 返回
/// - Vec<u8>: 完整的 VLAN 标记以太网帧
///
/// # 帧格式
/// 注：本实现不包含 FCS（由硬件添加）
/// ```text
/// | DA(6) | SA(6) | TPID(2) | TCI(2) | Type(2) | Payload | FCS(4) |
/// ```
#[allow(clippy::too_many_arguments)]
pub fn encapsulate_vlan_frame(
    dst_mac: MacAddr,
    src_mac: MacAddr,
    vlan_tag: VlanTag,
    tpid: u16,
    inner_type: u16,
    payload: &[u8],
) -> Vec<u8> {
    let params = VlanEncapParams {
        dst_mac,
        src_mac,
        vlan_tag,
        tpid,
        inner_type,
    };
    encapsulate_vlan_frame_with_params(params, payload)
}

/// 封装 QinQ 帧（双标签）- 使用参数结构体版本
///
/// 构造带双层 VLAN 标签的以太网帧（Q-in-Q）。
///
/// # 参数
/// - params: QinQ 封装参数
/// - payload: 负载数据
///
/// # 返回
/// - Vec<u8>: 完整的 QinQ 标记以太网帧
///
/// # 帧格式
/// ```text
/// | DA(6) | SA(6) | OuterTPID(2) | OuterTCI(2) | InnerTPID(2) | InnerTCI(2) | Type(2) | Payload | FCS(4) |
/// ```
pub fn encapsulate_qinq_frame_with_params(params: QinQEncapParams, payload: &[u8]) -> Vec<u8> {
    // 计算总长度：以太网头部(14) + 双VLAN标签(8) + 负载
    let total_len = 14 + 8 + payload.len();
    let mut frame = Vec::with_capacity(total_len);

    // 以太网头部
    frame.extend_from_slice(&params.dst_mac.bytes);
    frame.extend_from_slice(&params.src_mac.bytes);

    // 外层 VLAN 标签
    frame.extend_from_slice(&params.outer_tpid.to_be_bytes());
    frame.extend_from_slice(&params.outer_tag.to_bytes());

    // 内层 VLAN 标签
    frame.extend_from_slice(&params.inner_tpid.to_be_bytes());
    frame.extend_from_slice(&params.inner_tag.to_bytes());

    // 内层协议类型
    frame.extend_from_slice(&params.inner_type.to_be_bytes());

    // 负载
    frame.extend_from_slice(payload);

    frame
}

/// 封装 QinQ 帧（双标签）- 便捷函数
///
/// 构造带双层 VLAN 标签的以太网帧（Q-in-Q）。
///
/// # 参数
/// - dst_mac: 目标 MAC 地址
/// - src_mac: 源 MAC 地址
/// - outer_tag: 外层 VLAN 标签（服务 VLAN）
/// - outer_tpid: 外层 TPID（通常 0x9100 或 0x88A8）
/// - inner_tag: 内层 VLAN 标签（用户 VLAN）
/// - inner_tpid: 内层 TPID（通常 0x8100）
/// - inner_type: 内层协议类型（如 0x0800 表示 IPv4）
/// - payload: 负载数据
///
/// # 返回
/// - Vec<u8>: 完整的 QinQ 标记以太网帧
///
/// # 帧格式
/// ```text
/// | DA(6) | SA(6) | OuterTPID(2) | OuterTCI(2) | InnerTPID(2) | InnerTCI(2) | Type(2) | Payload | FCS(4) |
/// ```
#[allow(clippy::too_many_arguments)]
pub fn encapsulate_qinq_frame(
    dst_mac: MacAddr,
    src_mac: MacAddr,
    outer_tag: VlanTag,
    outer_tpid: u16,
    inner_tag: VlanTag,
    inner_tpid: u16,
    inner_type: u16,
    payload: &[u8],
) -> Vec<u8> {
    let params = QinQEncapParams {
        dst_mac,
        src_mac,
        outer_tag,
        outer_tpid,
        inner_tag,
        inner_tpid,
        inner_type,
    };
    encapsulate_qinq_frame_with_params(params, payload)
}

/// 为现有以太网帧添加 VLAN 标签
///
/// 将不带 VLAN 标签的以太网帧转换为带 VLAN 标签的帧。
/// 假设输入帧格式为：| DA(6) | SA(6) | Type(2) | Payload |
///
/// # 参数
/// - frame: 原始以太网帧（不含 VLAN 标签）
/// - vlan_tag: 要添加的 VLAN 标签
/// - tpid: 标签协议标识符（默认 0x8100）
///
/// # 返回
/// - Ok(Vec<u8>): 带 VLAN 标签的以太网帧
/// - Err(VlanError): 帧长度不足
pub fn add_vlan_tag(
    frame: &[u8],
    vlan_tag: VlanTag,
    tpid: u16,
) -> Result<Vec<u8>, VlanError> {
    // 检查最小长度：DA(6) + SA(6) + Type(2) = 14
    if frame.len() < 14 {
        return Err(VlanError::InsufficientPacketLength {
            expected: 14,
            actual: frame.len(),
        });
    }

    // 新帧长度：原长度 + VLAN标签(4)
    let mut tagged_frame = Vec::with_capacity(frame.len() + 4);

    // 复制 DA 和 SA（前12字节）
    tagged_frame.extend_from_slice(&frame[..12]);

    // 插入 VLAN 标签
    tagged_frame.extend_from_slice(&tpid.to_be_bytes());
    tagged_frame.extend_from_slice(&vlan_tag.to_bytes());

    // 复制剩余部分（Type + Payload）
    tagged_frame.extend_from_slice(&frame[12..]);

    Ok(tagged_frame)
}

/// 从以太网帧移除 VLAN 标签
///
/// 将带 VLAN 标签的以太网帧转换为不带 VLAN 标签的帧。
/// 假设输入帧格式为：| DA(6) | SA(6) | TPID(2) | TCI(2) | Type(2) | Payload |
///
/// # 参数
/// - frame: 带 VLAN 标签的以太网帧
///
/// # 返回
/// - Ok(Vec<u8>): 不带 VLAN 标签的以太网帧
/// - Err(VlanError): 帧长度不足或不是 VLAN 帧
pub fn remove_vlan_tag(frame: &[u8]) -> Result<Vec<u8>, VlanError> {
    // 检查最小长度：DA(6) + SA(6) + TPID(2) + TCI(2) + Type(2) = 18
    if frame.len() < 18 {
        return Err(VlanError::InsufficientPacketLength {
            expected: 18,
            actual: frame.len(),
        });
    }

    // 检查是否有 VLAN 标签
    let tpid_bytes = [frame[12], frame[13]];
    let tpid = u16::from_be_bytes(tpid_bytes);
    if !is_vlan_tpid(tpid) {
        return Err(VlanError::ParseError("Frame does not contain VLAN tag".to_string()));
    }

    // 新帧长度：原长度 - VLAN标签(4)
    let mut untagged_frame = Vec::with_capacity(frame.len() - 4);

    // 复制 DA 和 SA（前12字节）
    untagged_frame.extend_from_slice(&frame[..12]);

    // 跳过 VLAN 标签（TPID + TCI = 4字节），复制剩余部分
    untagged_frame.extend_from_slice(&frame[16..]);

    Ok(untagged_frame)
}

#[cfg(test)]
mod encapsulation_tests {
    use super::*;
    use crate::protocols::MacAddr;

    #[test]
    fn test_encapsulate_vlan_frame() {
        let dst_mac = MacAddr::new([0x01, 0x02, 0x03, 0x04, 0x05, 0x06]);
        let src_mac = MacAddr::new([0x0A, 0x0B, 0x0C, 0x0D, 0x0E, 0x0F]);
        let vlan_tag = VlanTag::new(5, true, 100).unwrap();
        let payload = vec![0x08, 0x00, 0x45, 0x00]; // IP头部示例

        let frame = encapsulate_vlan_frame(
            dst_mac,
            src_mac,
            vlan_tag,
            TPID_8021Q,
            0x0800,
            &payload,
        );

        // 验证帧长度：14 + 4 + 4 = 22
        assert_eq!(frame.len(), 22);

        // 验证 DA
        assert_eq!(&frame[0..6], &[0x01, 0x02, 0x03, 0x04, 0x05, 0x06]);

        // 验证 SA
        assert_eq!(&frame[6..12], &[0x0A, 0x0B, 0x0C, 0x0D, 0x0E, 0x0F]);

        // 验证 TPID
        assert_eq!(u16::from_be_bytes([frame[12], frame[13]]), TPID_8021Q);

        // 验证 TCI
        let tci = u16::from_be_bytes([frame[14], frame[15]]);
        assert_eq!(tci >> 13, 5); // PCP
        assert_eq!((tci >> 12) & 0x01, 1); // DEI
        assert_eq!(tci & 0x0FFF, 100); // VID

        // 验证内层 Type
        assert_eq!(u16::from_be_bytes([frame[16], frame[17]]), 0x0800);

        // 验证负载
        assert_eq!(&frame[18..], &payload[..]);
    }

    #[test]
    fn test_encapsulate_qinq_frame() {
        let dst_mac = MacAddr::new([0x01, 0x02, 0x03, 0x04, 0x05, 0x06]);
        let src_mac = MacAddr::new([0x0A, 0x0B, 0x0C, 0x0D, 0x0E, 0x0F]);
        let outer_tag = VlanTag::new(3, false, 10).unwrap();
        let inner_tag = VlanTag::new(5, true, 100).unwrap();
        let payload = vec![0x08, 0x00, 0x45, 0x00];

        let frame = encapsulate_qinq_frame(
            dst_mac,
            src_mac,
            outer_tag,
            TPID_QINQ,
            inner_tag,
            TPID_8021Q,
            0x0800,
            &payload,
        );

        // 验证帧长度：14 + 8 + 4 = 26
        assert_eq!(frame.len(), 26);

        // 验证外层 TPID
        assert_eq!(u16::from_be_bytes([frame[12], frame[13]]), TPID_QINQ);

        // 验证内层 TPID
        assert_eq!(u16::from_be_bytes([frame[16], frame[17]]), TPID_8021Q);

        // 验证内层 Type
        assert_eq!(u16::from_be_bytes([frame[20], frame[21]]), 0x0800);
    }

    #[test]
    fn test_add_vlan_tag() {
        let mut original_frame = Vec::new();
        // DA
        original_frame.extend_from_slice(&[0x01, 0x02, 0x03, 0x04, 0x05, 0x06]);
        // SA
        original_frame.extend_from_slice(&[0x0A, 0x0B, 0x0C, 0x0D, 0x0E, 0x0F]);
        // Type
        original_frame.extend_from_slice(&0x0800u16.to_be_bytes());
        // Payload
        original_frame.extend_from_slice(&[0x45, 0x00]);

        let vlan_tag = VlanTag::new(0, false, 100).unwrap();
        let tagged = add_vlan_tag(&original_frame, vlan_tag, TPID_8021Q).unwrap();

        // 验证长度增加 4
        assert_eq!(tagged.len(), original_frame.len() + 4);

        // 验证 VLAN 标签位置
        assert_eq!(u16::from_be_bytes([tagged[12], tagged[13]]), TPID_8021Q);
    }

    #[test]
    fn test_remove_vlan_tag() {
        let mut tagged_frame = Vec::new();
        // DA
        tagged_frame.extend_from_slice(&[0x01, 0x02, 0x03, 0x04, 0x05, 0x06]);
        // SA
        tagged_frame.extend_from_slice(&[0x0A, 0x0B, 0x0C, 0x0D, 0x0E, 0x0F]);
        // VLAN TPID
        tagged_frame.extend_from_slice(&TPID_8021Q.to_be_bytes());
        // VLAN TCI
        tagged_frame.extend_from_slice(&100u16.to_be_bytes());
        // Type
        tagged_frame.extend_from_slice(&0x0800u16.to_be_bytes());
        // Payload
        tagged_frame.extend_from_slice(&[0x45, 0x00]);

        let untagged = remove_vlan_tag(&tagged_frame).unwrap();

        // 验证长度减少 4
        assert_eq!(untagged.len(), tagged_frame.len() - 4);

        // 验证 Type 位置在 12
        assert_eq!(u16::from_be_bytes([untagged[12], untagged[13]]), 0x0800);
    }

    #[test]
    fn test_remove_vlan_tag_no_vlan() {
        let mut frame = Vec::new();
        frame.extend_from_slice(&[0u8; 6]); // DA
        frame.extend_from_slice(&[0u8; 6]); // SA
        frame.extend_from_slice(&0x0800u16.to_be_bytes()); // Type (not VLAN)
        frame.extend_from_slice(&[0x45, 0x00]); // Payload

        let result = remove_vlan_tag(&frame);
        assert!(result.is_err());
    }

    #[test]
    fn test_add_then_remove_vlan_tag() {
        let original = vec![
            0x01, 0x02, 0x03, 0x04, 0x05, 0x06, // DA
            0x0A, 0x0B, 0x0C, 0x0D, 0x0E, 0x0F, // SA
            0x08, 0x00, // Type
            0x45, 0x00, 0x00, 0x14, // Payload
        ];

        let vlan_tag = VlanTag::new(3, true, 500).unwrap();
        let tagged = add_vlan_tag(&original, vlan_tag, TPID_8021Q).unwrap();
        let untagged = remove_vlan_tag(&tagged).unwrap();

        // 验证往返后内容一致
        assert_eq!(untagged, original);
    }
}
