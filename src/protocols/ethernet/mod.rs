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

use crate::protocols::MacAddr;

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
