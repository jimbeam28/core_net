// src/protocols/ip/header.rs
//
// IPv4 头部结构定义

use crate::common::{CoreError, Packet, Result};
use crate::protocols::Ipv4Addr;
use super::checksum::{calculate_checksum, verify_checksum};

// ========== IP 协议常量 ==========

/// IP 版本 4
pub const IP_VERSION: u8 = 4;

/// IP 头部最小长度（无选项）
pub const IP_MIN_HEADER_LEN: usize = 20;

/// IP 协议号：ICMP
pub const IP_PROTO_ICMP: u8 = 1;

/// IP 协议号：TCP
pub const IP_PROTO_TCP: u8 = 6;

/// IP 协议号：UDP
pub const IP_PROTO_UDP: u8 = 17;

/// 默认 TTL 值
pub const DEFAULT_TTL: u8 = 64;

// ========== IPv4 头部 ==========

/// IPv4 头部
#[derive(Debug, Clone, PartialEq)]
pub struct Ipv4Header {
    /// 版本 (4) 和头部长度 (IHL)
    pub version_ihl: u8,

    /// 服务类型 (TOS)
    pub tos: u8,

    /// 总长度（包括头部和数据）
    pub total_length: u16,

    /// 标识字段
    pub identification: u16,

    /// 标志和分片偏移
    pub flags_fragment: u16,

    /// 生存时间 (TTL)
    pub ttl: u8,

    /// 上层协议
    pub protocol: u8,

    /// 头部校验和
    pub checksum: u16,

    /// 源 IP 地址
    pub source_addr: Ipv4Addr,

    /// 目的 IP 地址
    pub dest_addr: Ipv4Addr,

    /// IP 选项（如果有）
    pub options: Vec<u8>,
}

impl Ipv4Header {
    /// IP 头部最小长度
    pub const MIN_LEN: usize = IP_MIN_HEADER_LEN;

    /// 从 Packet 解析 IPv4 头部
    ///
    /// # 参数
    /// - packet: 可变引用的 Packet
    ///
    /// # 返回
    /// - Ok(Ipv4Header): 解析成功
    /// - Err(CoreError): 解析失败
    pub fn from_packet(packet: &mut Packet) -> Result<Self> {
        // 先使用 peek 查看第一个字节获取 IHL，不移动 offset
        let version_ihl_peek = packet.peek(1)
            .ok_or_else(|| CoreError::parse_error("读取版本/IHL失败"))?[0];

        let ihl = version_ihl_peek & 0x0F;

        // 计算 IP 头部长度
        let header_len = (ihl as usize) * 4;
        if header_len < Self::MIN_LEN {
            return Err(CoreError::invalid_packet(format!(
                "IP头部长度无效: IHL={} 长度={}", ihl, header_len
            )));
        }

        // 检查是否有足够的数据（在 offset=0 时检查）
        if packet.remaining() < header_len {
            return Err(CoreError::invalid_packet(format!(
                "IP数据包长度不足：需要头部{} 实际剩余{}",
                header_len,
                packet.remaining()
            )));
        }

        // 读取第一个字节（版本和 IHL）
        let version_ihl = packet.read(1)
            .ok_or_else(|| CoreError::parse_error("读取版本/IHL失败"))?[0];

        let version = version_ihl >> 4;

        // 验证版本
        if version != IP_VERSION {
            return Err(CoreError::UnsupportedProtocol(format!(
                "IPv4版本不支持: {}", version
            )));
        }

        // 读取服务类型
        let tos = packet.read(1)
            .ok_or_else(|| CoreError::parse_error("读取TOS失败"))?[0];

        // 读取总长度
        let total_length_bytes = packet.read(2)
            .ok_or_else(|| CoreError::parse_error("读取总长度失败"))?;
        let total_length = u16::from_be_bytes([total_length_bytes[0], total_length_bytes[1]]);

        // 读取标识
        let identification_bytes = packet.read(2)
            .ok_or_else(|| CoreError::parse_error("读取标识失败"))?;
        let identification = u16::from_be_bytes([identification_bytes[0], identification_bytes[1]]);

        // 读取标志和分片偏移
        let flags_fragment_bytes = packet.read(2)
            .ok_or_else(|| CoreError::parse_error("读取标志/分片偏移失败"))?;
        let flags_fragment = u16::from_be_bytes([flags_fragment_bytes[0], flags_fragment_bytes[1]]);

        // 读取 TTL
        let ttl = packet.read(1)
            .ok_or_else(|| CoreError::parse_error("读取TTL失败"))?[0];

        // 读取协议
        let protocol = packet.read(1)
            .ok_or_else(|| CoreError::parse_error("读取协议失败"))?[0];

        // 读取校验和
        let checksum_bytes = packet.read(2)
            .ok_or_else(|| CoreError::parse_error("读取校验和失败"))?;
        let checksum = u16::from_be_bytes([checksum_bytes[0], checksum_bytes[1]]);

        // 读取源 IP 地址
        let source_bytes = packet.read(4)
            .ok_or_else(|| CoreError::parse_error("读取源IP地址失败"))?;
        let source_addr = Ipv4Addr::from_bytes([source_bytes[0], source_bytes[1], source_bytes[2], source_bytes[3]]);

        // 读取目的 IP 地址
        let dest_bytes = packet.read(4)
            .ok_or_else(|| CoreError::parse_error("读取目的IP地址失败"))?;
        let dest_addr = Ipv4Addr::from_bytes([dest_bytes[0], dest_bytes[1], dest_bytes[2], dest_bytes[3]]);

        // 读取选项（如果有）
        let options_len = header_len - Self::MIN_LEN;
        let mut options = Vec::new();
        if options_len > 0 {
            for _ in 0..options_len {
                if let Some(byte) = packet.read(1) {
                    options.push(byte[0]);
                }
            }
        }

        Ok(Ipv4Header {
            version_ihl,
            tos,
            total_length,
            identification,
            flags_fragment,
            ttl,
            protocol,
            checksum,
            source_addr,
            dest_addr,
            options,
        })
    }

    /// 验证校验和
    pub fn verify_checksum(&self, full_header: &[u8]) -> bool {
        verify_checksum(full_header, 10)
    }

    /// 获取 IP 版本
    pub fn version(&self) -> u8 {
        self.version_ihl >> 4
    }

    /// 获取头部长度（字节数）
    pub fn header_len(&self) -> usize {
        ((self.version_ihl & 0x0F) as usize) * 4
    }

    /// 获取 IHL（头部长度，以 4 字节为单位）
    pub fn ihl(&self) -> u8 {
        self.version_ihl & 0x0F
    }

    /// 设置头部长度（通过 IHL）
    pub fn set_header_len(&mut self, len: usize) {
        let ihl = (len / 4) as u8;
        self.version_ihl = (IP_VERSION << 4) | (ihl & 0x0F);
    }

    /// 检查是否有 DF（Don't Fragment）标志
    pub fn has_df_flag(&self) -> bool {
        (self.flags_fragment & 0x4000) != 0
    }

    /// 检查是否有 MF（More Fragments）标志
    pub fn has_mf_flag(&self) -> bool {
        (self.flags_fragment & 0x2000) != 0
    }

    /// 获取分片偏移
    pub fn fragment_offset(&self) -> u16 {
        self.flags_fragment & 0x1FFF
    }

    /// 编码为字节数组
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(self.header_len());

        // Version/IHL
        bytes.push(self.version_ihl);

        // TOS
        bytes.push(self.tos);

        // Total Length
        bytes.extend_from_slice(&self.total_length.to_be_bytes());

        // Identification
        bytes.extend_from_slice(&self.identification.to_be_bytes());

        // Flags/Fragment
        bytes.extend_from_slice(&self.flags_fragment.to_be_bytes());

        // TTL
        bytes.push(self.ttl);

        // Protocol
        bytes.push(self.protocol);

        // Checksum (先填 0，后续计算)
        bytes.push(0);
        bytes.push(0);

        // Source Address
        bytes.extend_from_slice(self.source_addr.as_bytes());

        // Destination Address
        bytes.extend_from_slice(self.dest_addr.as_bytes());

        // Options
        bytes.extend_from_slice(&self.options);

        // 计算并填入校验和
        let checksum = calculate_checksum(&bytes);
        bytes[10] = (checksum >> 8) as u8;
        bytes[11] = (checksum & 0xFF) as u8;

        bytes
    }

    /// 创建新的 IP 头部（简化构造函数）
    pub fn new(
        source_addr: Ipv4Addr,
        dest_addr: Ipv4Addr,
        protocol: u8,
        payload_len: usize,
    ) -> Self {
        let total_length = (IP_MIN_HEADER_LEN + payload_len) as u16;
        let mut header = Ipv4Header {
            version_ihl: (IP_VERSION << 4) | 5, // Version=4, IHL=5 (20字节)
            tos: 0,
            total_length,
            identification: 0,
            flags_fragment: 0x4000, // DF flag set
            ttl: DEFAULT_TTL,
            protocol,
            checksum: 0,
            source_addr,
            dest_addr,
            options: Vec::new(),
        };

        // 计算校验和
        let bytes = header.to_bytes();
        header.checksum = u16::from_be_bytes([bytes[10], bytes[11]]);

        header
    }
}

impl std::fmt::Display for Ipv4Header {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "IPv4 {} -> {} Protocol={} TTL={} Len={}",
            self.source_addr, self.dest_addr, self.protocol, self.ttl, self.total_length
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ipv4_header_min_len() {
        assert_eq!(Ipv4Header::MIN_LEN, 20);
    }

    #[test]
    fn test_ipv4_header_new() {
        let src = Ipv4Addr::new(192, 168, 1, 1);
        let dst = Ipv4Addr::new(192, 168, 1, 2);
        let header = Ipv4Header::new(src, dst, IP_PROTO_ICMP, 64);

        assert_eq!(header.version(), 4);
        assert_eq!(header.protocol, IP_PROTO_ICMP);
        assert_eq!(header.source_addr, src);
        assert_eq!(header.dest_addr, dst);
        assert_eq!(header.total_length, 84); // 20 + 64
    }

    #[test]
    fn test_ipv4_header_flags() {
        let header = Ipv4Header::new(
            Ipv4Addr::new(192, 168, 1, 1),
            Ipv4Addr::new(192, 168, 1, 2),
            IP_PROTO_ICMP,
            64,
        );

        assert!(header.has_df_flag());
        assert!(!header.has_mf_flag());
        assert_eq!(header.fragment_offset(), 0);
    }

    #[test]
    fn test_ipv4_encode_decode() {
        let src = Ipv4Addr::new(192, 168, 1, 1);
        let dst = Ipv4Addr::new(192, 168, 1, 2);
        let header = Ipv4Header::new(src, dst, IP_PROTO_ICMP, 64);

        // 编码
        let bytes = header.to_bytes();

        // 验证校验和
        assert!(verify_checksum(&bytes, 10));

        // 解码
        let mut packet = Packet::from_bytes(bytes.clone());
        let decoded = Ipv4Header::from_packet(&mut packet).unwrap();

        assert_eq!(decoded.source_addr, src);
        assert_eq!(decoded.dest_addr, dst);
        assert_eq!(decoded.protocol, IP_PROTO_ICMP);
    }
}
