// src/protocols/udp/packet.rs
//
// UDP 数据报结构和处理

use crate::common::{CoreError, Packet, Result};
use crate::protocols::Ipv4Addr;

use super::header::UdpHeader;

/// UDP 数据报
///
/// 包含 UDP 头部和数据载荷
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UdpDatagram<'a> {
    /// UDP 头部
    pub header: UdpHeader,
    /// UDP 数据载荷
    pub payload: &'a [u8],
}

impl<'a> UdpDatagram<'a> {
    /// 从字节流解析 UDP 数据报
    ///
    /// # 参数
    /// - data: 包含 UDP 数据报的字节切片
    ///
    /// # 返回
    /// - Ok(UdpDatagram): 解析成功
    /// - Err(CoreError): 解析失败
    pub fn parse(data: &'a [u8]) -> Result<Self> {
        let header = UdpHeader::parse(data)?;

        // 验证长度
        if !header.is_valid_length() {
            return Err(CoreError::ParseError(format!(
                "Invalid UDP length: {} (minimum {})",
                header.length,
                UdpHeader::MIN_LENGTH
            )));
        }

        let payload_len = (header.length as usize) - UdpHeader::HEADER_SIZE;
        if data.len() < UdpHeader::HEADER_SIZE + payload_len {
            return Err(CoreError::ParseError(format!(
                "UDP data too short: expected {} bytes, got {}",
                UdpHeader::HEADER_SIZE + payload_len,
                data.len()
            )));
        }

        let payload = &data[UdpHeader::HEADER_SIZE..UdpHeader::HEADER_SIZE + payload_len];

        Ok(Self { header, payload })
    }

    /// 从 Packet 解析 UDP 数据报
    ///
    /// # 参数
    /// - packet: 可变引用的 Packet
    ///
    /// # 返回
    /// - Ok(UdpDatagram): 解析成功
    /// - Err(CoreError): 解析失败
    pub fn from_packet(packet: &'a Packet) -> Result<Self> {
        let data = packet.peek(packet.remaining()).unwrap_or(&[]);
        Self::parse(data)
    }

    /// 计算 UDP 校验和（包含伪头部）
    ///
    /// # 参数
    /// - source_ip: 源 IP 地址
    /// - dest_ip: 目标 IP 地址
    ///
    /// # 返回
    /// - u16: 计算得到的校验和
    pub fn calculate_checksum(&self, source_ip: Ipv4Addr, dest_ip: Ipv4Addr) -> u16 {
        let mut sum = 0u32;

        // 伪头部 (12 字节)
        // 源 IP 地址（2 个 16 位字）
        sum += u32::from(u16::from_be_bytes([source_ip.bytes[0], source_ip.bytes[1]]));
        sum += u32::from(u16::from_be_bytes([source_ip.bytes[2], source_ip.bytes[3]]));
        // 目标 IP 地址（2 个 16 位字）
        sum += u32::from(u16::from_be_bytes([dest_ip.bytes[0], dest_ip.bytes[1]]));
        sum += u32::from(u16::from_be_bytes([dest_ip.bytes[2], dest_ip.bytes[3]]));
        // 协议号和 UDP 长度
        sum += u32::from(17u16) << 8 | u32::from(self.header.length >> 8);
        sum += u32::from(self.header.length & 0xFF) << 8;

        // UDP 头部
        sum += u32::from(self.header.source_port);
        sum += u32::from(self.header.destination_port);
        sum += u32::from(self.header.length);

        // 跳过校验和字段（偏移 6-7）
        // 数据
        let mut i = 0;
        while i + 1 < self.payload.len() {
            let word = u16::from_be_bytes([self.payload[i], self.payload[i + 1]]);
            sum += u32::from(word);
            i += 2;
        }

        // 处理奇数字节
        if i < self.payload.len() {
            sum += u32::from(self.payload[i]) << 8;
        }

        // 处理进位
        while sum >> 16 != 0 {
            sum = (sum & 0xFFFF) + (sum >> 16);
        }

        !sum as u16
    }

    /// 验证校验和
    ///
    /// # 参数
    /// - source_ip: 源 IP 地址
    /// - dest_ip: 目标 IP 地址
    /// - enforce: 是否强制验证校验和（IPv4 中校验和可选）
    ///
    /// # 返回
    /// - bool: 校验和是否有效
    pub fn verify_checksum(&self, source_ip: Ipv4Addr, dest_ip: Ipv4Addr, enforce: bool) -> bool {
        // IPv4 中校验和可选，如果为 0 表示未计算
        if self.header.checksum == 0 {
            return !enforce; // 如果不强制验证，则接受
        }

        self.calculate_checksum(source_ip, dest_ip) == self.header.checksum
    }

    /// 创建 UDP 数据报
    ///
    /// # 参数
    /// - source_port: 源端口号
    /// - destination_port: 目标端口号
    /// - payload: 数据载荷
    ///
    /// # 返回
    /// - UdpDatagram: UDP 数据报（校验和未计算）
    pub fn create(source_port: u16, destination_port: u16, payload: &'a [u8]) -> Self {
        let length = (UdpHeader::HEADER_SIZE + payload.len()) as u16;
        let header = UdpHeader::new(source_port, destination_port, length);

        Self { header, payload }
    }

    /// 序列化为字节
    ///
    /// # 参数
    /// - source_ip: 源 IP 地址（用于计算校验和）
    /// - dest_ip: 目标 IP 地址（用于计算校验和）
    /// - calculate_checksum: 是否计算校验和
    ///
    /// # 返回
    /// - Vec<u8>: 序列化后的 UDP 数据报
    pub fn to_bytes(&self, source_ip: Ipv4Addr, dest_ip: Ipv4Addr, calculate_checksum: bool) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(self.header.length as usize);

        // 序列化头部
        let mut header = self.header;
        if calculate_checksum {
            let checksum = self.calculate_checksum(source_ip, dest_ip);
            header.set_checksum(checksum);
        }
        bytes.extend_from_slice(&header.serialize());

        // 添加数据
        bytes.extend_from_slice(self.payload);

        bytes
    }

    /// 获取数据报长度
    pub fn len(&self) -> usize {
        self.header.length as usize
    }

    /// 检查数据报是否为空
    pub fn is_empty(&self) -> bool {
        self.payload.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_datagram_parse_basic() {
        let bytes = [
            0x04, 0xD2, // Source Port: 1234
            0x16, 0x2E, // Dest Port: 5678
            0x00, 0x0C, // Length: 12 (8 + 4)
            0x00, 0x00, // Checksum: 0
            0x01, 0x02, 0x03, 0x04, // Payload
        ];

        let datagram = UdpDatagram::parse(&bytes).unwrap();
        assert_eq!(datagram.header.source_port, 1234);
        assert_eq!(datagram.header.destination_port, 5678);
        assert_eq!(datagram.header.length, 12);
        assert_eq!(datagram.payload, &[0x01, 0x02, 0x03, 0x04]);
    }

    #[test]
    fn test_datagram_parse_too_short() {
        let bytes = [
            0x04, 0xD2, // Source Port
            0x16, 0x2E, // Dest Port
            0x00, 0x0C, // Length: 12
            0x00, 0x00, // Checksum
            0x01, 0x02, // Only 2 bytes of payload (need 4)
        ];

        let result = UdpDatagram::parse(&bytes);
        assert!(result.is_err());
    }

    #[test]
    fn test_datagram_parse_invalid_length() {
        let bytes = [
            0x04, 0xD2, // Source Port
            0x16, 0x2E, // Dest Port
            0x00, 0x07, // Length: 7 (invalid, < 8)
            0x00, 0x00, // Checksum
        ];

        let result = UdpDatagram::parse(&bytes);
        assert!(result.is_err());
    }

    #[test]
    fn test_datagram_parse_minimal() {
        let bytes = [
            0x04, 0xD2, // Source Port: 1234
            0x16, 0x2E, // Dest Port: 5678
            0x00, 0x08, // Length: 8 (only header)
            0x00, 0x00, // Checksum: 0
        ];

        let datagram = UdpDatagram::parse(&bytes).unwrap();
        assert_eq!(datagram.header.source_port, 1234);
        assert_eq!(datagram.header.destination_port, 5678);
        assert_eq!(datagram.header.length, 8);
        assert!(datagram.payload.is_empty());
    }

    #[test]
    fn test_datagram_create() {
        let payload = b"Hello";
        let datagram = UdpDatagram::create(8080, 53, payload);

        assert_eq!(datagram.header.source_port, 8080);
        assert_eq!(datagram.header.destination_port, 53);
        assert_eq!(datagram.header.length, 13); // 8 + 5
        assert_eq!(datagram.payload, b"Hello");
    }

    #[test]
    fn test_datagram_to_bytes() {
        let payload = b"ABC";
        let datagram = UdpDatagram::create(1234, 5678, payload);
        let src_ip = Ipv4Addr::new(192, 168, 1, 1);
        let dst_ip = Ipv4Addr::new(192, 168, 1, 2);

        let bytes = datagram.to_bytes(src_ip, dst_ip, false);

        assert_eq!(bytes.len(), 11); // 8 + 3
        assert_eq!(bytes[0..2], 1234u16.to_be_bytes());
        assert_eq!(bytes[2..4], 5678u16.to_be_bytes());
        assert_eq!(bytes[4..6], 11u16.to_be_bytes());
        assert_eq!(&bytes[8..], b"ABC");
    }

    #[test]
    fn test_checksum_zero_payload() {
        let payload = b"";
        let datagram = UdpDatagram::create(1234, 5678, payload);
        let src_ip = Ipv4Addr::new(192, 168, 1, 1);
        let dst_ip = Ipv4Addr::new(192, 168, 1, 2);

        let checksum = datagram.calculate_checksum(src_ip, dst_ip);
        // 奇数长度（8）需要填充
        assert_ne!(checksum, 0);
    }

    #[test]
    fn test_checksum_odd_payload() {
        // 测试奇数长度载荷的校验和计算
        let payload = b"ABC"; // 3 bytes
        let datagram = UdpDatagram::create(1234, 5678, payload);
        let src_ip = Ipv4Addr::new(192, 168, 1, 1);
        let dst_ip = Ipv4Addr::new(192, 168, 1, 2);

        let checksum = datagram.calculate_checksum(src_ip, dst_ip);
        // 验证校验和计算不崩溃
        assert_ne!(checksum, datagram.header.checksum); // Should be different from 0
    }

    #[test]
    fn test_checksum_roundtrip() {
        let payload = b"Hello, World!";
        let mut datagram = UdpDatagram::create(8080, 53, payload);
        let src_ip = Ipv4Addr::new(192, 168, 1, 10);
        let dst_ip = Ipv4Addr::new(192, 168, 1, 100);

        // 计算校验和
        let checksum = datagram.calculate_checksum(src_ip, dst_ip);
        datagram.header.set_checksum(checksum);

        // 验证校验和
        assert!(datagram.verify_checksum(src_ip, dst_ip, true));

        // 错误的校验和应该验证失败
        datagram.header.set_checksum(checksum.wrapping_add(1));
        assert!(!datagram.verify_checksum(src_ip, dst_ip, true));
    }

    #[test]
    fn test_verify_checksum_zero() {
        let payload = b"Test";
        let mut datagram = UdpDatagram::create(1234, 5678, payload);
        datagram.header.set_checksum(0);

        let src_ip = Ipv4Addr::new(192, 168, 1, 1);
        let dst_ip = Ipv4Addr::new(192, 168, 1, 2);

        // 零校验和在强制验证模式下应该失败
        assert!(!datagram.verify_checksum(src_ip, dst_ip, true));
        // 零校验和在非强制模式下应该通过
        assert!(datagram.verify_checksum(src_ip, dst_ip, false));
    }

    #[test]
    fn test_datagram_len_and_empty() {
        let payload1 = b"";
        let datagram1 = UdpDatagram::create(1234, 5678, payload1);
        assert_eq!(datagram1.len(), 8);
        assert!(datagram1.is_empty());

        let payload2 = b"Hello";
        let datagram2 = UdpDatagram::create(1234, 5678, payload2);
        assert_eq!(datagram2.len(), 13);
        assert!(!datagram2.is_empty());
    }

    #[test]
    fn test_datagram_from_packet() {
        let bytes = [
            0x04, 0xD2, // Source Port: 1234
            0x16, 0x2E, // Dest Port: 5678
            0x00, 0x0C, // Length: 12
            0x00, 0x00, // Checksum
            0x01, 0x02, 0x03, 0x04, // Payload
        ];

        let packet = Packet::from_bytes(bytes.to_vec());
        let datagram = UdpDatagram::from_packet(&packet).unwrap();

        assert_eq!(datagram.header.source_port, 1234);
        assert_eq!(datagram.header.destination_port, 5678);
        assert_eq!(datagram.payload, &[0x01, 0x02, 0x03, 0x04]);
    }
}
