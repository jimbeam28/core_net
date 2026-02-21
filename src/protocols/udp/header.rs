// src/protocols/udp/header.rs
//
// UDP 头部结构定义

use crate::common::{CoreError, Result};

/// UDP 头部结构
///
/// RFC 768 定义的 UDP 头部格式：
/// ```text
///  0                   1                   2                   3
///  0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
/// |          Source Port          |       Destination Port        |
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
/// |            Length             |           Checksum            |
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UdpHeader {
    /// 源端口号，0 表示未使用
    pub source_port: u16,
    /// 目标端口号
    pub destination_port: u16,
    /// UDP 数据报长度（包含头部）
    pub length: u16,
    /// 校验和
    pub checksum: u16,
}

impl UdpHeader {
    /// UDP 头部固定大小
    pub const HEADER_SIZE: usize = 8;

    /// 最小 UDP 数据报长度（仅头部）
    pub const MIN_LENGTH: u16 = 8;

    /// 创建新的 UDP 头部
    ///
    /// # 参数
    /// - source_port: 源端口号
    /// - destination_port: 目标端口号
    /// - length: UDP 数据报总长度（头部 + 数据）
    pub fn new(source_port: u16, destination_port: u16, length: u16) -> Self {
        Self {
            source_port,
            destination_port,
            length,
            checksum: 0, // 初始化为 0，稍后计算
        }
    }

    /// 从字节流解析 UDP 头部
    ///
    /// # 参数
    /// - data: 包含 UDP 头部的字节切片
    ///
    /// # 返回
    /// - Ok(UdpHeader): 解析成功
    /// - Err(CoreError): 解析失败（数据长度不足）
    pub fn parse(data: &[u8]) -> Result<Self> {
        if data.len() < Self::HEADER_SIZE {
            return Err(CoreError::ParseError(format!(
                "UDP header too short: expected {} bytes, got {}",
                Self::HEADER_SIZE,
                data.len()
            )));
        }

        let source_port = u16::from_be_bytes([data[0], data[1]]);
        let destination_port = u16::from_be_bytes([data[2], data[3]]);
        let length = u16::from_be_bytes([data[4], data[5]]);
        let checksum = u16::from_be_bytes([data[6], data[7]]);

        Ok(Self {
            source_port,
            destination_port,
            length,
            checksum,
        })
    }

    /// 将头部序列化为字节
    ///
    /// # 返回
    /// - [u8; 8]: 8 字节的 UDP 头部字节数组
    pub fn serialize(&self) -> [u8; Self::HEADER_SIZE] {
        let mut buf = [0u8; Self::HEADER_SIZE];
        buf[0..2].copy_from_slice(&self.source_port.to_be_bytes());
        buf[2..4].copy_from_slice(&self.destination_port.to_be_bytes());
        buf[4..6].copy_from_slice(&self.length.to_be_bytes());
        buf[6..8].copy_from_slice(&self.checksum.to_be_bytes());
        buf
    }

    /// 设置校验和
    ///
    /// # 参数
    /// - checksum: 计算得到的校验和值
    pub fn set_checksum(&mut self, checksum: u16) {
        self.checksum = checksum;
    }

    /// 验证长度字段
    ///
    /// # 返回
    /// - bool: 长度是否有效
    pub fn is_valid_length(&self) -> bool {
        self.length >= Self::MIN_LENGTH
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_header_size() {
        assert_eq!(UdpHeader::HEADER_SIZE, 8);
    }

    #[test]
    fn test_header_new() {
        let header = UdpHeader::new(1234, 5678, 20);
        assert_eq!(header.source_port, 1234);
        assert_eq!(header.destination_port, 5678);
        assert_eq!(header.length, 20);
        assert_eq!(header.checksum, 0);
    }

    #[test]
    fn test_header_serialize() {
        let header = UdpHeader::new(1234, 5678, 20);
        let bytes = header.serialize();

        assert_eq!(bytes[0..2], 1234u16.to_be_bytes());
        assert_eq!(bytes[2..4], 5678u16.to_be_bytes());
        assert_eq!(bytes[4..6], 20u16.to_be_bytes());
        assert_eq!(bytes[6..8], 0u16.to_be_bytes());
    }

    #[test]
    fn test_header_parse() {
        let bytes = [
            0x04, 0xD2, // 1234
            0x16, 0x2E, // 5678
            0x00, 0x14, // 20
            0x12, 0x34, // 0x1234
        ];

        let header = UdpHeader::parse(&bytes).unwrap();
        assert_eq!(header.source_port, 1234);
        assert_eq!(header.destination_port, 5678);
        assert_eq!(header.length, 20);
        assert_eq!(header.checksum, 0x1234);
    }

    #[test]
    fn test_header_parse_too_short() {
        let bytes = [0x04, 0xD2, 0x16, 0x2E]; // 只有 4 字节
        let result = UdpHeader::parse(&bytes);
        assert!(result.is_err());
    }

    #[test]
    fn test_header_serialize_parse_roundtrip() {
        let mut original = UdpHeader::new(8080, 53, 512);
        original.set_checksum(0xABCD);

        let bytes = original.serialize();
        let parsed = UdpHeader::parse(&bytes).unwrap();

        assert_eq!(parsed.source_port, original.source_port);
        assert_eq!(parsed.destination_port, original.destination_port);
        assert_eq!(parsed.length, original.length);
        assert_eq!(parsed.checksum, original.checksum);
    }

    #[test]
    fn test_header_valid_length() {
        let mut header = UdpHeader::new(1234, 5678, 8);
        assert!(header.is_valid_length());

        header.length = 7;
        assert!(!header.is_valid_length());

        header.length = 100;
        assert!(header.is_valid_length());
    }

    #[test]
    fn test_header_zero_source_port() {
        let header = UdpHeader::new(0, 80, 20);
        assert_eq!(header.source_port, 0);
        assert!(header.is_valid_length());
    }
}
