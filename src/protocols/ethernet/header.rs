// src/protocols/ethernet/header.rs
//
// 以太网头部结构定义

use crate::common::{CoreError, Result};
use crate::protocols::{MacAddr, Packet};

// ========== 以太网类型常量 ==========

/// 以太网类型：IPv4
pub const ETH_P_IP: u16 = 0x0800;

/// 以太网类型：ARP
pub const ETH_P_ARP: u16 = 0x0806;

/// 以太网类型：IPv6
pub const ETH_P_IPV6: u16 = 0x86DD;

/// 以太网类型：802.1Q VLAN
pub const ETH_P_8021Q: u16 = 0x8100;

/// 以太网类型：Q-in-Q VLAN
pub const ETH_P_8021AD: u16 = 0x88A8;

// ========== 以太网头部 ==========

/// 以太网头部
#[derive(Debug, Clone)]
pub struct EthernetHeader {
    /// 目标 MAC 地址
    pub dst_mac: MacAddr,

    /// 源 MAC 地址
    pub src_mac: MacAddr,

    /// 以太网类型
    pub ether_type: u16,
}

impl EthernetHeader {
    /// 以太网头部最小长度
    pub const MIN_LEN: usize = 14;

    /// 从 Packet 解析以太网头部
    ///
    /// # 参数
    /// - packet: 可变引用的 Packet（解析后偏移量移动 14 字节）
    ///
    /// # 返回
    /// - Ok(EthernetHeader): 解析成功
    /// - Err(CoreError): 解析失败
    pub fn from_packet(packet: &mut Packet) -> Result<Self> {
        // 检查最小长度
        if packet.remaining() < Self::MIN_LEN {
            return Err(CoreError::invalid_packet(format!(
                "以太网报文长度不足：{} < {}",
                packet.remaining(),
                Self::MIN_LEN
            )));
        }

        // 读取目标 MAC 地址 (6 字节)
        let mut dst_bytes = [0u8; 6];
        for i in 0..6 {
            dst_bytes[i] = packet.read(1)
                .ok_or_else(|| CoreError::parse_error("读取目标MAC地址失败"))?[0];
        }
        let dst_mac = MacAddr::new(dst_bytes);

        // 读取源 MAC 地址 (6 字节)
        let mut src_bytes = [0u8; 6];
        for i in 0..6 {
            src_bytes[i] = packet.read(1)
                .ok_or_else(|| CoreError::parse_error("读取源MAC地址失败"))?[0];
        }
        let src_mac = MacAddr::new(src_bytes);

        // 读取以太网类型 (2 字节)
        let ether_type_bytes = packet.read(2)
            .ok_or_else(|| CoreError::parse_error("读取以太网类型失败"))?;
        let ether_type = u16::from_be_bytes([ether_type_bytes[0], ether_type_bytes[1]]);

        Ok(EthernetHeader {
            dst_mac,
            src_mac,
            ether_type,
        })
    }
}
