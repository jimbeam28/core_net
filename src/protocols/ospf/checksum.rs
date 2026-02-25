// src/protocols/ospf/checksum.rs
//
// OSPF 校验和计算
// RFC 2328 定义了 OSPF 专用的 Fletcher 校验和算法
// 此模块提供 OSPFv2 和 OSPFv3 共享的校验和计算功能

/// 计算 OSPF 校验和（RFC 2328 Fletcher 校验和）
///
/// # 参数
/// - `data`: 需要计算校验和的数据
/// - `checksum_offset`: 校验和字段在数据中的偏移量（字节）
///
/// # 返回
/// 计算出的 16 位校验和值
///
/// # 算法说明
/// RFC 2328 Section 12.1.7 定义了 OSPF 使用的 Fletcher 校验和算法：
/// 1. 初始化两个 8 位累加器 c0 和 c1
/// 2. 对每个字节（除了校验和字段，按 0 处理）更新累加器
/// 3. c0 = (c0 + byte) % 255
/// 4. c1 = (c1 + c0) % 255
/// 5. 最后返回 c0 << 8 | c1
pub fn calculate_fletcher_checksum(data: &[u8], checksum_offset: usize) -> u16 {
    const MODULO: u32 = 255;

    let mut c0: u32 = 0;
    let mut c1: u32 = 0;

    for (i, &byte) in data.iter().enumerate() {
        // 跳过校验和字段（当作 0 处理）
        let value = if i >= checksum_offset && i < checksum_offset + 2 {
            0
        } else {
            u32::from(byte)
        };

        c0 = (c0 + value) % MODULO;
        c1 = (c1 + c0) % MODULO;
    }

    // 处理特殊情况：当 c0 或 c1 为 0 时，需要设置为 255
    // 根据 Fletcher 算法，0 表示 255
    let c0 = if c0 == 0 { MODULO } else { c0 };
    let c1 = if c1 == 0 { MODULO } else { c1 };

    ((c1 << 8) | c0) as u16
}

/// 验证 OSPF 校验和
///
/// # 参数
/// - `data`: 包含校验和的数据
/// - `checksum_offset`: 校验和字段在数据中的偏移量
///
/// # 返回
/// 如果校验和有效返回 true，否则返回 false
pub fn verify_fletcher_checksum(data: &[u8], checksum_offset: usize) -> bool {
    // 读取存储的校验和
    let stored_checksum = if checksum_offset + 2 <= data.len() {
        u16::from_be_bytes([data[checksum_offset], data[checksum_offset + 1]])
    } else {
        return false;
    };

    // 计算校验和
    let calculated_checksum = calculate_fletcher_checksum(data, checksum_offset);

    stored_checksum == calculated_checksum
}

/// 更新数据包的校验和字段
///
/// # 参数
/// - `data`: 需要更新校验和的数据（可变）
/// - `checksum_offset`: 校验和字段在数据中的偏移量
///
/// # 说明
/// 此函数会计算校验和并直接写入到数据的校验和字段中
pub fn update_checksum(data: &mut [u8], checksum_offset: usize) {
    let checksum = calculate_fletcher_checksum(data, checksum_offset);

    if checksum_offset + 2 <= data.len() {
        data[checksum_offset] = (checksum >> 8) as u8;
        data[checksum_offset + 1] = checksum as u8;
    }
}

/// 计算 OSPFv3 校验和（使用标准 IP 校验和）
///
/// OSPFv3 不使用 Fletcher 校验和，而是使用标准的 Internet 校验和
///
/// # 参数
/// - `data`: 需要计算校验和的数据
///
/// # 返回
/// 计算出的 16 位校验和值
pub fn calculate_ip_checksum(data: &[u8]) -> u16 {
    let mut sum: u32 = 0;

    // 按 16 位处理数据
    let mut chunks = data.chunks_exact(2);
    while let Some(chunk) = chunks.next() {
        let word = u16::from_be_bytes([chunk[0], chunk[1]]) as u32;
        sum += word;
    }

    // 处理剩余的单字节
    if let Some(&remainder) = chunks.remainder().first() {
        sum += (remainder as u32) << 8;
    }

    // 处理进位
    while sum >> 16 != 0 {
        sum = (sum & 0xFFFF) + (sum >> 16);
    }

    // 取反
    !sum as u16
}

/// 验证 OSPFv3 校验和
///
/// # 参数
/// - `data`: 包含校验和的数据
/// - `checksum_offset`: 校验和字段在数据中的偏移量
///
/// # 返回
/// 如果校验和有效返回 true，否则返回 false
pub fn verify_ip_checksum(data: &[u8], checksum_offset: usize) -> bool {
    if checksum_offset + 2 > data.len() {
        return false;
    }

    // 读取存储的校验和
    let stored_checksum = u16::from_be_bytes([
        data[checksum_offset],
        data[checksum_offset + 1],
    ]);

    // 计算校验和（校验和字段当作 0）
    let calculated_checksum = {
        let mut temp_data = data.to_vec();
        temp_data[checksum_offset] = 0;
        temp_data[checksum_offset + 1] = 0;
        calculate_ip_checksum(&temp_data)
    };

    stored_checksum == calculated_checksum
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_fletcher_checksum() {
        // 测试数据：简单的 OSPF 头部
        let mut data = vec![
            0x02, 0x01, 0x00, 0x28, // 版本, 类型, 长度
            0x01, 0x01, 0x01, 0x01, // 路由器 ID
            0x00, 0x00, 0x00, 0x00, // 区域 ID
            0x00, 0x00, 0x00, 0x00, // 校验和（占位）
            0x00, 0x00, 0x00, 0x00, // 实例 ID (OSPFv3)
            0x00, 0x00,             // 保留
        ];

        let checksum = calculate_fletcher_checksum(&data, 12);
        assert_ne!(checksum, 0);

        // 更新校验和
        update_checksum(&mut data, 12);

        // 验证校验和
        assert!(verify_fletcher_checksum(&data, 12));
    }

    #[test]
    fn test_verify_fletcher_checksum_invalid() {
        let data = vec![
            0x02, 0x01, 0x00, 0x28,
            0x01, 0x01, 0x01, 0x01,
            0x00, 0x00, 0x00, 0x00,
            0xFF, 0xFF, // 错误的校验和
            0x00, 0x00, 0x00, 0x00,
            0x00, 0x00,
        ];

        assert!(!verify_fletcher_checksum(&data, 12));
    }

    #[test]
    fn test_calculate_ip_checksum() {
        // 测试标准 IP 校验和
        let data = [0x45, 0x00, 0x00, 0x3c, 0x00, 0x00, 0x00, 0x00];

        let checksum = calculate_ip_checksum(&data);
        // 校验和不应该是 0 或 0xFFFF（除非数据全为 0）
        assert!(checksum != 0);
    }

    #[test]
    fn test_update_checksum_short_data() {
        // 测试数据太短的情况
        let mut short_data = vec![0u8; 10];
        let original = short_data.clone();

        // 校验和偏移量超出范围，不应 panic
        update_checksum(&mut short_data, 9);
        assert_eq!(&short_data[..9], &original[..9]);
    }

    #[test]
    fn test_fletcher_checksum_zeros() {
        // 全零数据的校验和
        let data = vec![0u8; 24];
        let checksum = calculate_fletcher_checksum(&data, 12);

        // 全零数据应该产生非零校验和
        assert_ne!(checksum, 0);
    }

    #[test]
    fn test_fletcher_checksum_consistency() {
        // 测试相同数据产生相同校验和
        let data = vec![
            0x02, 0x01, 0x00, 0x28,
            0x01, 0x01, 0x01, 0x01,
            0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00,
            0x00, 0x00,
        ];

        let checksum1 = calculate_fletcher_checksum(&data, 12);
        let checksum2 = calculate_fletcher_checksum(&data, 12);

        assert_eq!(checksum1, checksum2);
    }

    #[test]
    fn test_verify_ip_checksum() {
        // 创建一个有效的 IP 数据包
        let mut data = vec![
            0x45, 0x00, 0x00, 0x1c, // 版本, IHL, TOS, 长度
            0x00, 0x01, 0x00, 0x00, // ID, 标志, 片偏移
            0x40, 0x11, 0x00, 0x00, // TTL, 协议, 校验和（占位）
            0x0a, 0x00, 0x00, 0x01, // 源地址
            0x0a, 0x00, 0x00, 0x02, // 目的地址
        ];

        // 计算并更新校验和
        let checksum = calculate_ip_checksum(&data);
        data[10] = (checksum >> 8) as u8;
        data[11] = checksum as u8;

        // 验证校验和
        assert!(verify_ip_checksum(&data, 10));

        // 修改数据，校验和应该失效
        data[0] = 0x46;
        assert!(!verify_ip_checksum(&data, 10));
    }

    #[test]
    fn test_ip_checksum_round_trip() {
        let data = vec![
            0x45, 0x00, 0x00, 0x1c,
            0x00, 0x01, 0x00, 0x00,
            0x40, 0x11, 0x00, 0x00,
            0x0a, 0x00, 0x00, 0x01,
            0x0a, 0x00, 0x00, 0x02,
        ];

        let checksum = calculate_ip_checksum(&data);

        // 构造带校验和的数据
        let mut with_checksum = data.clone();
        with_checksum[10] = (checksum >> 8) as u8;
        with_checksum[11] = checksum as u8;

        // 验证
        assert!(verify_ip_checksum(&with_checksum, 10));
    }
}
