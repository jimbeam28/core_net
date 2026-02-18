// src/common/protocols/ethernet/mod.rs
//
// 以太网协议定义
// 包含以太网头部结构和常量

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
/// 将目标MAC、源MAC、以太网类型和负载组合成完整的以太网帧。
///
/// # 参数
/// - dst_mac: 目标 MAC 地址
/// - src_mac: 源 MAC 地址
/// - ether_type: 以太网类型
/// - payload: 负载数据
///
/// # 返回
/// 完整的以太网帧字节数组（14字节头部 + 负载）
pub fn build_ethernet_frame(dst_mac: MacAddr, src_mac: MacAddr, ether_type: u16, payload: &[u8]) -> Vec<u8> {
    let mut frame = Vec::with_capacity(14 + payload.len());
    frame.extend_from_slice(&dst_mac.bytes);
    frame.extend_from_slice(&src_mac.bytes);
    frame.extend_from_slice(&ether_type.to_be_bytes());
    frame.extend_from_slice(payload);
    frame
}
