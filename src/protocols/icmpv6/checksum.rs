// src/protocols/icmpv6/checksum.rs
//
// ICMPv6 校验和计算
// RFC 4443: ICMPv6 校验和使用 IPv6 伪头部

use crate::protocols::Ipv6Addr;
use crate::protocols::ip::fold_carry;

/// ICMPv6 伪头部
///
/// RFC 2460: IPv6 伪头部用于校验和计算
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct Icmpv6PseudoHeader {
    /// 源 IPv6 地址
    pub src_addr: [u8; 16],
    /// 目的 IPv6 地址
    pub dst_addr: [u8; 16],
    /// 上层协议包长度
    pub length: u32,
    /// 零填充（3字节）+ 下一个头部（1字节）
    pub zeros_and_next_header: [u8; 4],
}

impl Icmpv6PseudoHeader {
    pub const SIZE: usize = 40;

    /// 创建 ICMPv6 伪头部
    pub fn new(src_addr: Ipv6Addr, dst_addr: Ipv6Addr, length: u32) -> Self {
        Icmpv6PseudoHeader {
            src_addr: src_addr.bytes,
            dst_addr: dst_addr.bytes,
            length, // 保持原值，在 to_bytes 中转换为 big-endian
            zeros_and_next_header: [0, 0, 0, 58], // 58 = ICMPv6
        }
    }

    /// 编码为字节数组
    pub fn to_bytes(&self) -> [u8; Self::SIZE] {
        let mut bytes = [0u8; Self::SIZE];
        let mut offset = 0;

        // 源地址
        bytes[offset..offset + 16].copy_from_slice(&self.src_addr);
        offset += 16;

        // 目的地址
        bytes[offset..offset + 16].copy_from_slice(&self.dst_addr);
        offset += 16;

        // 长度
        bytes[offset..offset + 4].copy_from_slice(&self.length.to_be_bytes());
        offset += 4;

        // 零 + 下一个头部
        bytes[offset..offset + 4].copy_from_slice(&self.zeros_and_next_header);

        bytes
    }
}

/// 计算 ICMPv6 校验和（包含伪头部）
///
/// # 参数
/// - src_addr: 源 IPv6 地址
/// - dst_addr: 目的 IPv6 地址
/// - data: ICMPv6 报文数据
///
/// # 返回
/// - u16: 计算出的校验和
pub fn calculate_icmpv6_checksum(src_addr: Ipv6Addr, dst_addr: Ipv6Addr, data: &[u8]) -> u16 {
    let pseudo = Icmpv6PseudoHeader::new(src_addr, dst_addr, data.len() as u32);
    let pseudo_bytes = pseudo.to_bytes();

    // 组合伪头部和数据
    let mut checksum_data = Vec::with_capacity(pseudo_bytes.len() + data.len());
    checksum_data.extend_from_slice(&pseudo_bytes);
    checksum_data.extend_from_slice(data);

    // 计算校验和
    calculate_checksum(&checksum_data)
}

/// 计算简单的校验和（不包含伪头部）
///
/// # 参数
/// - data: 数据
///
/// # 返回
/// - u16: 计算出的校验和
pub fn calculate_checksum(data: &[u8]) -> u16 {
    let mut sum: u32 = 0;

    // 按 16 位处理
    let mut chunks = data.chunks_exact(2);
    for chunk in chunks.by_ref() {
        sum += u16::from_be_bytes([chunk[0], chunk[1]]) as u32;
    }

    // 处理剩余字节
    if let Some(&byte) = chunks.remainder().first() {
        sum += (byte as u32) << 8;
    }

    fold_carry(sum)
}

/// 验证 ICMPv6 校验和（包含伪头部）
///
/// # 参数
/// - src_addr: 源 IPv6 地址
/// - dst_addr: 目的 IPv6 地址
/// - data: ICMPv6 报文数据
/// - checksum_offset: 校验和字段偏移量（通常为 2）
///
/// # 返回
/// - bool: 校验和是否有效
pub fn verify_icmpv6_checksum(
    src_addr: Ipv6Addr,
    dst_addr: Ipv6Addr,
    data: &[u8],
    checksum_offset: usize,
) -> bool {
    if data.len() < checksum_offset + 2 {
        return false;
    }

    // 读取存储的校验和
    let stored_checksum = u16::from_be_bytes([data[checksum_offset], data[checksum_offset + 1]]);

    // 计算校验和（忽略原校验和字段）
    let mut data_without_checksum = data.to_vec();
    data_without_checksum[checksum_offset] = 0;
    data_without_checksum[checksum_offset + 1] = 0;

    let calculated_checksum = calculate_icmpv6_checksum(src_addr, dst_addr, &data_without_checksum);

    stored_checksum == calculated_checksum
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pseudo_header() {
        let src = Ipv6Addr::new(0xfe80, 0, 0, 0, 0, 0, 0, 1);
        let dst = Ipv6Addr::new(0xfe80, 0, 0, 0, 0, 0, 0, 2);
        let pseudo = Icmpv6PseudoHeader::new(src, dst, 8);

        let bytes = pseudo.to_bytes();

        // 验证源地址
        assert_eq!(&bytes[0..16], &src.bytes[..]);
        // 验证目的地址
        assert_eq!(&bytes[16..32], &dst.bytes[..]);
        // 验证长度
        assert_eq!(u32::from_be_bytes([bytes[32], bytes[33], bytes[34], bytes[35]]), 8);
        // 验证下一个头部
        assert_eq!(bytes[39], 58); // ICMPv6
    }

    #[test]
    fn test_calculate_checksum() {
        let data = vec![0x01, 0x02, 0x03, 0x04, 0x05, 0x06];
        let checksum = calculate_checksum(&data);

        // 简单验证校验和不为零
        assert_ne!(checksum, 0);
    }

    #[test]
    fn test_calculate_icmpv6_checksum() {
        let src = Ipv6Addr::new(0xfe80, 0, 0, 0, 0, 0, 0, 1);
        let dst = Ipv6Addr::new(0xfe80, 0, 0, 0, 0, 0, 0, 2);

        // 简单的 ICMPv6 Echo Request
        let mut icmp_data = vec![
            128, 0, // Type=Echo Request, Code=0
            0, 0,   // Checksum (placeholder)
            0x12, 0x34, // Identifier
            0x00, 0x01, // Sequence
        ];

        // 计算校验和
        let checksum = calculate_icmpv6_checksum(src, dst, &icmp_data);

        // 验证校验和
        icmp_data[2] = (checksum >> 8) as u8;
        icmp_data[3] = (checksum & 0xFF) as u8;

        assert!(verify_icmpv6_checksum(src, dst, &icmp_data, 2));
    }
}
