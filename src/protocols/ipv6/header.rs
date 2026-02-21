// src/protocols/ipv6/header.rs
//
// IPv6 头部结构定义

use crate::common::{CoreError, Packet, Result};
use crate::protocols::Ipv6Addr;
use super::protocol::IpProtocol;

// --- IPv6 协议常量 ---

/// IP 版本 6
pub const IPV6_VERSION: u8 = 6;

/// IPv6 头部长度（固定 40 字节）
pub const IPV6_HEADER_LEN: usize = 40;

/// IPv6 最小 MTU (RFC 8200 要求)
pub const IPV6_MIN_MTU: u16 = 1280;

/// 默认 Hop Limit 值
pub const DEFAULT_HOP_LIMIT: u8 = 64;

// --- IPv6 头部 ---

/// IPv6 头部结构
#[derive(Debug, Clone, PartialEq)]
pub struct Ipv6Header {
    /// 版本号 (4 位), 固定为 6
    pub version: u8,

    /// 流量类别 (8 位), 用于 QoS 和 DiffServ
    pub traffic_class: u8,

    /// 流标签 (20 位), 用于标识属于同一数据流的包
    pub flow_label: u32,

    /// 负载长度 (16 位), 包括扩展头和上层协议数据
    pub payload_length: u16,

    /// 下一头部类型 (8 位), 指示紧跟的扩展头或上层协议
    pub next_header: IpProtocol,

    /// 跳数限制 (8 位), 类似 IPv4 的 TTL
    pub hop_limit: u8,

    /// 源 IPv6 地址 (128 位)
    pub source_addr: Ipv6Addr,

    /// 目的 IPv6 地址 (128 位)
    pub destination_addr: Ipv6Addr,
}

impl Ipv6Header {
    /// IPv6 头部固定长度
    pub const HEADER_SIZE: usize = IPV6_HEADER_LEN;

    /// 从 Packet 解析 IPv6 头部
    ///
    /// # 参数
    /// - packet: 可变引用的 Packet
    ///
    /// # 返回
    /// - Ok(Ipv6Header): 解析成功
    /// - Err(CoreError): 解析失败
    pub fn from_packet(packet: &mut Packet) -> Result<Self> {
        // 检查是否有足够的数据（至少 40 字节）
        if packet.remaining() < Self::HEADER_SIZE {
            return Err(CoreError::invalid_packet(format!(
                "IPv6数据包长度不足：需要{} 实际剩余{}",
                Self::HEADER_SIZE,
                packet.remaining()
            )));
        }

        // 读取前 4 个字节（Version + TC + Flow Label 高 4 位）
        let bytes0_3 = packet.read(4)
            .ok_or_else(|| CoreError::parse_error("读取IPv6头部前4字节失败"))?;

        // 解析 Version (高 4 位)
        let version = bytes0_3[0] >> 4;

        // 验证版本
        if version != IPV6_VERSION {
            return Err(CoreError::UnsupportedProtocol(format!(
                "IPv6版本不支持: {}", version
            )));
        }

        // Traffic Class (接下来的 8 位)
        let traffic_class = ((bytes0_3[0] & 0x0F) << 4) | (bytes0_3[1] >> 4);

        // Flow Label (20 位)
        let flow_label = (((bytes0_3[1] & 0x0F) as u32) << 16)
            | ((bytes0_3[2] as u32) << 8)
            | (bytes0_3[3] as u32);

        // 读取 Payload Length (2 字节)
        let payload_len_bytes = packet.read(2)
            .ok_or_else(|| CoreError::parse_error("读取Payload Length失败"))?;
        let payload_length = u16::from_be_bytes([payload_len_bytes[0], payload_len_bytes[1]]);

        // 读取 Next Header (1 字节)
        let next_header_byte = packet.read(1)
            .ok_or_else(|| CoreError::parse_error("读取Next Header失败"))?[0];
        let next_header = IpProtocol::from(next_header_byte);

        // 读取 Hop Limit (1 字节)
        let hop_limit = packet.read(1)
            .ok_or_else(|| CoreError::parse_error("读取Hop Limit失败"))?[0];

        // 读取源 IPv6 地址 (16 字节)
        let source_bytes = packet.read(16)
            .ok_or_else(|| CoreError::parse_error("读取源IPv6地址失败"))?;
        let mut source_addr_array = [0u8; 16];
        source_addr_array.copy_from_slice(source_bytes);
        let source_addr = Ipv6Addr::from_bytes(source_addr_array);

        // 读取目的 IPv6 地址 (16 字节)
        let dest_bytes = packet.read(16)
            .ok_or_else(|| CoreError::parse_error("读取目的IPv6地址失败"))?;
        let mut dest_addr_array = [0u8; 16];
        dest_addr_array.copy_from_slice(dest_bytes);
        let destination_addr = Ipv6Addr::from_bytes(dest_addr_array);

        Ok(Ipv6Header {
            version,
            traffic_class,
            flow_label,
            payload_length,
            next_header,
            hop_limit,
            source_addr,
            destination_addr,
        })
    }

    /// 获取头部长度（固定 40 字节）
    pub fn header_len(&self) -> usize {
        Self::HEADER_SIZE
    }

    /// 编码为字节数组
    pub fn to_bytes(&self) -> [u8; Self::HEADER_SIZE] {
        let mut bytes = [0u8; Self::HEADER_SIZE];

        // Version (4 位) + Traffic Class 高 4 位
        bytes[0] = (self.version << 4) | ((self.traffic_class >> 4) & 0x0F);

        // Traffic Class 低 4 位 + Flow Label 高 4 位
        bytes[1] = ((self.traffic_class & 0x0F) << 4) | ((self.flow_label >> 16) as u8 & 0x0F);

        // Flow Label 中 8 位
        bytes[2] = ((self.flow_label >> 8) & 0xFF) as u8;

        // Flow Label 低 8 位
        bytes[3] = (self.flow_label & 0xFF) as u8;

        // Payload Length
        bytes[4] = (self.payload_length >> 8) as u8;
        bytes[5] = (self.payload_length & 0xFF) as u8;

        // Next Header
        bytes[6] = u8::from(self.next_header);

        // Hop Limit
        bytes[7] = self.hop_limit;

        // Source Address
        bytes[8..24].copy_from_slice(&self.source_addr.bytes);

        // Destination Address
        bytes[24..40].copy_from_slice(&self.destination_addr.bytes);

        bytes
    }

    /// 创建新的 IPv6 头部
    pub fn new(
        source: Ipv6Addr,
        destination: Ipv6Addr,
        payload_length: u16,
        next_header: IpProtocol,
        hop_limit: u8,
    ) -> Self {
        Ipv6Header {
            version: IPV6_VERSION,
            traffic_class: 0,
            flow_label: 0,
            payload_length,
            next_header,
            hop_limit,
            source_addr: source,
            destination_addr: destination,
        }
    }

    /// 创建带有流标签的 IPv6 头部
    pub fn with_flow_label(
        source: Ipv6Addr,
        destination: Ipv6Addr,
        payload_length: u16,
        next_header: IpProtocol,
        hop_limit: u8,
        flow_label: u32,
    ) -> Self {
        Ipv6Header {
            version: IPV6_VERSION,
            traffic_class: 0,
            flow_label: flow_label & 0xFFFFF, // 20 位
            payload_length,
            next_header,
            hop_limit,
            source_addr: source,
            destination_addr: destination,
        }
    }

    /// 设置流量类别
    pub fn set_traffic_class(&mut self, tc: u8) {
        self.traffic_class = tc;
    }

    /// 设置流标签
    pub fn set_flow_label(&mut self, label: u32) {
        self.flow_label = label & 0xFFFFF; // 限制为 20 位
    }
}

impl std::fmt::Display for Ipv6Header {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "IPv6 {} -> {} NextHeader={} HopLimit={} Len={}",
            self.source_addr, self.destination_addr, self.next_header, self.hop_limit, self.payload_length
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ipv6_header_constants() {
        assert_eq!(IPV6_VERSION, 6);
        assert_eq!(IPV6_HEADER_LEN, 40);
        assert_eq!(IPV6_MIN_MTU, 1280);
        assert_eq!(DEFAULT_HOP_LIMIT, 64);
    }

    #[test]
    fn test_ipv6_header_new() {
        let src = Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 1);
        let dst = Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 2);
        let header = Ipv6Header::new(src, dst, 64, IpProtocol::IcmpV6, 64);

        assert_eq!(header.version, 6);
        assert_eq!(header.next_header, IpProtocol::IcmpV6);
        assert_eq!(header.source_addr, src);
        assert_eq!(header.destination_addr, dst);
        assert_eq!(header.payload_length, 64);
        assert_eq!(header.hop_limit, 64);
    }

    #[test]
    fn test_ipv6_header_encode_decode() {
        let src = Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 1);
        let dst = Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 2);
        let header = Ipv6Header::new(src, dst, 64, IpProtocol::IcmpV6, 64);

        // 编码
        let bytes = header.to_bytes();

        // 验证基本字段
        assert_eq!(bytes[0] >> 4, 6); // Version
        assert_eq!(bytes[6], 58); // Next Header = ICMPv6
        assert_eq!(bytes[7], 64); // Hop Limit

        // 解码
        let mut packet = Packet::from_bytes(bytes.to_vec());
        let decoded = Ipv6Header::from_packet(&mut packet).unwrap();

        assert_eq!(decoded.source_addr, src);
        assert_eq!(decoded.destination_addr, dst);
        assert_eq!(decoded.next_header, IpProtocol::IcmpV6);
    }

    #[test]
    fn test_ipv6_header_flow_label() {
        let src = Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 1);
        let dst = Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 2);
        let flow_label = 0x12345;
        let header = Ipv6Header::with_flow_label(src, dst, 64, IpProtocol::IcmpV6, 64, flow_label);

        assert_eq!(header.flow_label, 0x12345);

        // 编码并验证流标签
        let bytes = header.to_bytes();
        let decoded_flow = (((bytes[1] & 0x0F) as u32) << 16)
            | ((bytes[2] as u32) << 8)
            | (bytes[3] as u32);
        assert_eq!(decoded_flow, 0x12345);
    }

    #[test]
    fn test_ipv6_header_set_traffic_class() {
        let mut header = Ipv6Header::new(
            Ipv6Addr::UNSPECIFIED,
            Ipv6Addr::UNSPECIFIED,
            0,
            IpProtocol::IcmpV6,
            64,
        );

        header.set_traffic_class(0xAB);
        assert_eq!(header.traffic_class, 0xAB);

        let bytes = header.to_bytes();
        let tc = ((bytes[0] & 0x0F) << 4) | (bytes[1] >> 4);
        assert_eq!(tc, 0xAB);
    }

    #[test]
    fn test_ipv6_invalid_version() {
        let mut bytes = [0u8; 40];
        bytes[0] = 4 << 4; // Version = 4 (IPv4)

        let mut packet = Packet::from_bytes(bytes.to_vec());
        let result = Ipv6Header::from_packet(&mut packet);

        assert!(result.is_err());
    }
}
