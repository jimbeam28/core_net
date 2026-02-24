// 以太网协议定义
mod header;

pub use header::{
    EthernetHeader,
    ETH_P_IP,
    ETH_P_ARP,
    ETH_P_IPV6,
    ETH_P_8021Q,
    ETH_P_8021AD,
};

use crate::protocols::{MacAddr, vlan::VlanTag};

/// 构造以太网帧
///
/// 创建一个完整的以太网帧，包含 14 字节头部和负载。
///
/// # 参数
/// - `dst_mac`: 目标 MAC 地址
/// - `src_mac`: 源 MAC 地址
/// - `ether_type`: 以太网类型字段（如 0x0800 表示 IPv4）
/// - `payload`: 负载数据
///
/// # 返回
/// 完整的以太网帧字节数组
///
/// # 示例
/// ```
/// use core_net::protocols::{MacAddr, ethernet::build_ethernet_frame};
///
/// let dst = MacAddr::broadcast();
/// let src = MacAddr::new([0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF]);
/// let frame = build_ethernet_frame(dst, src, 0x0800, &[0x01, 0x02, 0x03]);
/// ```
pub fn build_ethernet_frame(dst_mac: MacAddr, src_mac: MacAddr, ether_type: u16, payload: &[u8]) -> Vec<u8> {
    let mut frame = Vec::with_capacity(14 + payload.len());
    frame.extend_from_slice(&dst_mac.bytes);
    frame.extend_from_slice(&src_mac.bytes);
    frame.extend_from_slice(&ether_type.to_be_bytes());
    frame.extend_from_slice(payload);
    frame
}

/// 构造带 VLAN 标签的以太网帧
///
/// 创建一个带 802.1Q VLAN 标签的以太网帧。
///
/// # 参数
/// - `dst_mac`: 目标 MAC 地址
/// - `src_mac`: 源 MAC 地址
/// - `vlan_tag`: VLAN 标签
/// - `tpid`: 标签协议标识符（默认 0x8100）
/// - `inner_type`: 内层协议类型（如 0x0800 表示 IPv4）
/// - `payload`: 负载数据
///
/// # 返回
/// 完整的 VLAN 标记以太网帧
///
/// # 示例
/// ```
/// use core_net::protocols::{MacAddr, ethernet::build_vlan_frame, vlan::VlanTag};
///
/// let dst = MacAddr::broadcast();
/// let src = MacAddr::new([0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF]);
/// let vlan = VlanTag::new(5, false, 100).unwrap();
/// let frame = build_vlan_frame(dst, src, vlan, 0x8100, 0x0800, &[0x01, 0x02, 0x03]);
/// ```
pub fn build_vlan_frame(
    dst_mac: MacAddr,
    src_mac: MacAddr,
    vlan_tag: VlanTag,
    tpid: u16,
    inner_type: u16,
    payload: &[u8],
) -> Vec<u8> {
    crate::protocols::vlan::encapsulate_vlan_frame(
        dst_mac,
        src_mac,
        vlan_tag,
        tpid,
        inner_type,
        payload,
    )
}

/// 构造 QinQ（双标签）以太网帧
///
/// 创建一个带双层 VLAN 标签的以太网帧。
///
/// # 参数
/// - `dst_mac`: 目标 MAC 地址
/// - `src_mac`: 源 MAC 地址
/// - `outer_tag`: 外层 VLAN 标签（服务 VLAN）
/// - `outer_tpid`: 外层 TPID（通常 0x9100 或 0x88A8）
/// - `inner_tag`: 内层 VLAN 标签（用户 VLAN）
/// - `inner_tpid`: 内层 TPID（通常 0x8100）
/// - `inner_type`: 内层协议类型
/// - `payload`: 负载数据
///
/// # 返回
/// 完整的 QinQ 标记以太网帧
pub fn build_qinq_frame(
    dst_mac: MacAddr,
    src_mac: MacAddr,
    outer_tag: VlanTag,
    outer_tpid: u16,
    inner_tag: VlanTag,
    inner_tpid: u16,
    inner_type: u16,
    payload: &[u8],
) -> Vec<u8> {
    crate::protocols::vlan::encapsulate_qinq_frame(
        dst_mac,
        src_mac,
        outer_tag,
        outer_tpid,
        inner_tag,
        inner_tpid,
        inner_type,
        payload,
    )
}
