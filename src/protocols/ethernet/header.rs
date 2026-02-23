// 以太网头部结构定义
use crate::common::{CoreError, Result};
use crate::protocols::{MacAddr, Packet};

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

/// IEEE 802.3 最大帧长（不含 FCS）
/// 标准以太网最大帧长为 1500 字节（不含 FCS）
pub const ETH_MAX_FRAME_LEN: usize = 1500;

/// Ethernet II 格式的最小 EtherType 值
/// 当类型字段 >= 1536 时表示 EtherType（Ethernet II 格式）
/// 当类型字段 <= 1500 时表示长度（IEEE 802.3 格式）
pub const ETH_P_8023_MIN: u16 = 1536;

/// 以太网头部
#[derive(Debug, Clone)]
pub struct EthernetHeader {
    pub dst_mac: MacAddr,
    pub src_mac: MacAddr,
    pub ether_type: u16,
}

impl EthernetHeader {
    /// 以太网头部最小长度
    pub const MIN_LEN: usize = 14;

    /// 从 Packet 解析以太网头部
    ///
    /// 支持解析 Ethernet II 格式和带 VLAN 标签的帧。
    /// 当检测到 VLAN 标签（0x8100 或 0x88A8）时，会解析 VLAN 标签
    /// 并继续读取实际的协议类型。
    ///
    /// # FCS 说明
    /// 本实现不处理 4 字节的 FCS (Frame Check Sequence)。
    /// 在实际网络中，FCS 由网卡硬件验证。
    /// 在纯模拟环境中，我们假设帧已通过硬件校验。
    ///
    /// # 帧长验证
    /// 根据标准，以太网帧（不含 FCS）最小长度为 60 字节。
    /// 由于这是纯模拟环境，我们只验证头部最小长度。
    pub fn from_packet(packet: &mut Packet) -> Result<Self> {
        if packet.remaining() < Self::MIN_LEN {
            return Err(CoreError::invalid_packet(format!(
                "以太网报文长度不足：{} < {}",
                packet.remaining(),
                Self::MIN_LEN
            )));
        }

        // 预先构造错误，避免循环中重复创建
        const DST_MAC_ERR: &str = "读取目标MAC地址失败";
        const SRC_MAC_ERR: &str = "读取源MAC地址失败";
        const ETHER_TYPE_ERR: &str = "读取以太网类型失败";

        // 一次性读取目标 MAC 地址（6 字节）
        let dst_bytes = packet.read(6)
            .ok_or_else(|| CoreError::parse_error(DST_MAC_ERR))?;
        let dst_mac = MacAddr::new(dst_bytes.try_into()
            .map_err(|_| CoreError::parse_error("MAC地址长度错误"))?);

        // 一次性读取源 MAC 地址（6 字节）
        let src_bytes = packet.read(6)
            .ok_or_else(|| CoreError::parse_error(SRC_MAC_ERR))?;
        let src_mac = MacAddr::new(src_bytes.try_into()
            .map_err(|_| CoreError::parse_error("MAC地址长度错误"))?);

        // 读取以太网类型（2 字节）
        let ether_type_bytes = packet.read(2)
            .ok_or_else(|| CoreError::parse_error(ETHER_TYPE_ERR))?;
        let mut ether_type = u16::from_be_bytes([ether_type_bytes[0], ether_type_bytes[1]]);

        // 处理 VLAN 标签
        // 当检测到 VLAN 标签时，需要解析 VLAN 标签并读取实际的协议类型
        // 最多支持两层 VLAN 标签（Q-in-Q）
        let mut vlan_depth = 0;
        const MAX_VLAN_DEPTH: usize = 2;

        while vlan_depth < MAX_VLAN_DEPTH {
            match ether_type {
                ETH_P_8021Q | ETH_P_8021AD => {
                    // VLAN 标签格式：TPCI (2字节) + TCI (2字节)
                    // TCI 包含：PCP (3位) + DEI (1位) + VID (12位)
                    let vlan_tag_bytes = packet.read(4)
                        .ok_or_else(|| CoreError::parse_error("读取VLAN标签失败"))?;

                    // 提取 VLAN ID (低 12 位)
                    let vid = u16::from_be_bytes([vlan_tag_bytes[2], vlan_tag_bytes[3]]) & 0x0FFF;

                    // 保存 VLAN ID 到 Packet（最内层的 VLAN ID）
                    packet.set_vlan_id(vid);

                    // 读取实际的协议类型
                    let next_ether_type_bytes = packet.read(2)
                        .ok_or_else(|| CoreError::parse_error("读取VLAN后协议类型失败"))?;
                    ether_type = u16::from_be_bytes([next_ether_type_bytes[0], next_ether_type_bytes[1]]);

                    vlan_depth += 1;
                }
                _ => break,
            }
        }

        // 验证以太网类型范围
        // IEEE 802.3：类型字段 <= 1500 表示长度，>= 1536 表示 EtherType
        // 我们只支持 Ethernet II 格式（EtherType）
        if ether_type < ETH_P_8023_MIN && ether_type > ETH_MAX_FRAME_LEN as u16 {
            return Err(CoreError::parse_error(format!(
                "无效的以太网类型：{} (不支持 IEEE 802.3 长度格式)",
                ether_type
            )));
        }

        Ok(EthernetHeader {
            dst_mac,
            src_mac,
            ether_type,
        })
    }
}
