// src/protocols/ip/checksum.rs
//
// IP 校验和计算

use crate::common::Ipv4Addr;

/// 将 IPv4 伪头部添加到校验和计算中
///
/// TCP/UDP 校验和需要包含伪头部（源IP、目标IP、协议号、长度）
///
/// # 参数
/// - sum: 当前的校验和累加值
/// - source_ip: 源 IPv4 地址
/// - dest_ip: 目的 IPv4 地址
pub fn add_ipv4_pseudo_header(sum: &mut u32, source_ip: Ipv4Addr, dest_ip: Ipv4Addr) {
    *sum += u32::from(u16::from_be_bytes([source_ip.bytes[0], source_ip.bytes[1]]));
    *sum += u32::from(u16::from_be_bytes([source_ip.bytes[2], source_ip.bytes[3]]));
    *sum += u32::from(u16::from_be_bytes([dest_ip.bytes[0], dest_ip.bytes[1]]));
    *sum += u32::from(u16::from_be_bytes([dest_ip.bytes[2], dest_ip.bytes[3]]));
}

/// 处理校验和进位，返回最终的 16 位校验和
///
/// # 参数
/// - sum: 包含进位的 32 位累加和
///
/// # 返回
/// - 折叠进位后的 16 位校验和
pub fn fold_carry(sum: u32) -> u16 {
    let mut s = sum;
    while s >> 16 != 0 {
        s = (s & 0xFFFF) + (s >> 16);
    }
    s as u16
}

/// 计算 IP 校验和
///
/// # 算法说明
/// IP 校验和是互联网校验和（Internet Checksum）：
/// 1. 将校验和字段置为 0
/// 2. 将所有 16 位字相加
/// 3. 将进位加到结果的低位（反码求和）
/// 4. 取反得到校验和
///
/// # 参数
/// - data: 需要计算校验和的字节数组
///
/// # 返回
/// - 16 位校验和（大端序）
pub fn calculate_checksum(data: &[u8]) -> u16 {
    let mut sum: u32 = 0;

    // 按 16 位字处理
    let mut chunks = data.chunks_exact(2);
    for chunk in &mut chunks {
        let word = u16::from_be_bytes([chunk[0], chunk[1]]) as u32;
        sum += word;
    }

    // 处理剩余的单字节（如果有）
    if let Some(&byte) = chunks.remainder().first() {
        sum += (byte as u32) << 8;
    }

    // 处理进位
    while sum >> 16 != 0 {
        sum = (sum & 0xFFFF) + (sum >> 16);
    }

    // 取反
    !sum as u16
}

/// 验证 IP 校验和
///
/// # 参数
/// - data: 包含校验和的字节数组
/// - checksum_offset: 校验和字段在数据中的偏移量（字节）
///
/// # 返回
/// - true: 校验和正确
/// - false: 校验和错误
pub fn verify_checksum(data: &[u8], checksum_offset: usize) -> bool {
    if data.len() < checksum_offset + 2 {
        return false;
    }

    // 读取原校验和
    let original_checksum = u16::from_be_bytes([
        data[checksum_offset],
        data[checksum_offset + 1],
    ]);

    // 将校验和字段置为 0
    let mut data_without_checksum = Vec::from(data);
    data_without_checksum[checksum_offset] = 0;
    data_without_checksum[checksum_offset + 1] = 0;

    // 计算校验和
    let calculated_checksum = calculate_checksum(&data_without_checksum);

    original_checksum == calculated_checksum
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_checksum() {
        // 测试数据：简单的 IP 头部
        let data = [
            0x45, 0x00, 0x00, 0x3c,  // Version/IHL, TOS, Total Length
            0x00, 0x00, 0x00, 0x00,  // ID, Flags/Fragment
            0x40, 0x01, 0x00, 0x00,  // TTL, Protocol, Checksum (置0)
            0xc0, 0xa8, 0x01, 0x01,  // Source IP
            0xc0, 0xa8, 0x01, 0x02,  // Dest IP
        ];

        let checksum = calculate_checksum(&data);
        // 校验和应该非零
        assert_ne!(checksum, 0);
    }

    #[test]
    fn test_verify_checksum() {
        // 构造一个带正确校验和的 IP 头部
        let mut data = [
            0x45, 0x00, 0x00, 0x3c,
            0x00, 0x00, 0x00, 0x00,
            0x40, 0x01, 0x00, 0x00,  // Checksum at offset 10
            0xc0, 0xa8, 0x01, 0x01,
            0xc0, 0xa8, 0x01, 0x02,
        ];

        // 计算并填入校验和
        let checksum = calculate_checksum(&data);
        data[10] = (checksum >> 8) as u8;
        data[11] = (checksum & 0xFF) as u8;

        // 验证
        assert!(verify_checksum(&data, 10));

        // 篡改校验和
        data[10] = 0xFF;
        assert!(!verify_checksum(&data, 10));
    }

    #[test]
    fn test_odd_length_data() {
        // 奇数长度数据
        let data = [0x45, 0x00, 0x00, 0x3c, 0x00];
        let checksum = calculate_checksum(&data);
        // 应该能处理而不崩溃
        assert_ne!(checksum, 0xFFFF);
    }
}
