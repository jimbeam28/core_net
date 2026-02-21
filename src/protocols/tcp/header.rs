// src/protocols/tcp/header.rs
//
// TCP 头部结构定义

use crate::common::{CoreError, Result};
use super::constant::{TCP_MIN_DATA_OFFSET, TCP_MIN_HEADER_LEN, TCP_MAX_HEADER_LEN};
use super::flags;

/// TCP 头部结构
///
/// RFC 793 / RFC 9293 定义的 TCP 头部格式：
/// ```text
///  0                   1                   2                   3
///  0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
/// |          Source Port          |       Destination Port        |
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
/// |                        Sequence Number                        |
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
/// |                    Acknowledgment Number                      |
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
/// |  Data |           |U|A|P|R|S|F|                               |
/// | Offset| Reserved  |R|C|S|S|Y|I|            Window             |
/// |       |           |G|K|H|T|N|N|                               |
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
/// |           Checksum            |         Urgent Pointer        |
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
/// |                    Options                    |    Padding    |
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TcpHeader {
    /// 源端口号
    pub source_port: u16,
    /// 目标端口号
    pub destination_port: u16,
    /// 序列号
    pub sequence_number: u32,
    /// 确认号
    pub acknowledgment_number: u32,
    /// 数据偏移（高 4 位）+ 保留（高 4 位）+ 标志位（低 8 位）
    data_offset_and_flags: u16,
    /// 窗口大小
    pub window_size: u16,
    /// 校验和
    pub checksum: u16,
    /// 紧急指针
    pub urgent_pointer: u16,
}

impl TcpHeader {
    /// TCP 头部固定大小（不含选项）
    pub const HEADER_SIZE: usize = 20;

    /// 创建新的 TCP 头部
    ///
    /// # 参数
    /// - source_port: 源端口号
    /// - destination_port: 目标端口号
    /// - sequence_number: 序列号
    /// - acknowledgment_number: 确认号
    /// - data_offset: 数据偏移（以 4 字节为单位）
    /// - flags: 标志位
    /// - window_size: 窗口大小
    pub fn new(
        source_port: u16,
        destination_port: u16,
        sequence_number: u32,
        acknowledgment_number: u32,
        data_offset: u8,
        flags: u8,
        window_size: u16,
    ) -> Self {
        let data_offset_and_flags = ((data_offset as u16) << 12) | (flags as u16);

        Self {
            source_port,
            destination_port,
            sequence_number,
            acknowledgment_number,
            data_offset_and_flags,
            window_size,
            checksum: 0,
            urgent_pointer: 0,
        }
    }

    /// 从字节流解析 TCP 头部
    ///
    /// # 参数
    /// - data: 包含 TCP 头部的字节切片
    ///
    /// # 返回
    /// - Ok(TcpHeader): 解析成功
    /// - Err(CoreError): 解析失败（数据长度不足）
    pub fn parse(data: &[u8]) -> Result<Self> {
        if data.len() < Self::HEADER_SIZE {
            return Err(CoreError::ParseError(format!(
                "TCP header too short: expected {} bytes, got {}",
                Self::HEADER_SIZE,
                data.len()
            )));
        }

        let source_port = u16::from_be_bytes([data[0], data[1]]);
        let destination_port = u16::from_be_bytes([data[2], data[3]]);
        let sequence_number = u32::from_be_bytes([data[4], data[5], data[6], data[7]]);
        let acknowledgment_number = u32::from_be_bytes([data[8], data[9], data[10], data[11]]);
        let data_offset_and_flags = u16::from_be_bytes([data[12], data[13]]);
        let window_size = u16::from_be_bytes([data[14], data[15]]);
        let checksum = u16::from_be_bytes([data[16], data[17]]);
        let urgent_pointer = u16::from_be_bytes([data[18], data[19]]);

        let header = Self {
            source_port,
            destination_port,
            sequence_number,
            acknowledgment_number,
            data_offset_and_flags,
            window_size,
            checksum,
            urgent_pointer,
        };

        // 验证数据偏移
        if !header.is_valid_data_offset() {
            return Err(CoreError::ParseError(format!(
                "Invalid TCP data offset: {} (minimum {})",
                header.data_offset(),
                TCP_MIN_DATA_OFFSET
            )));
        }

        Ok(header)
    }

    /// 获取数据偏移（以 4 字节为单位）
    pub const fn data_offset(&self) -> u8 {
        ((self.data_offset_and_flags >> 12) & 0x0F) as u8
    }

    /// 设置数据偏移
    pub fn set_data_offset(&mut self, offset: u8) {
        let mask = 0x0FFF;
        self.data_offset_and_flags = (self.data_offset_and_flags & mask) | ((offset as u16) << 12);
    }

    /// 获取标志位
    pub const fn flags(&self) -> u8 {
        (self.data_offset_and_flags & 0xFF) as u8
    }

    /// 设置标志位
    pub fn set_flags(&mut self, flags: u8) {
        let mask = 0xFF00;
        self.data_offset_and_flags = (self.data_offset_and_flags & mask) | (flags as u16);
    }

    /// 检查 FIN 标志
    pub const fn is_fin(&self) -> bool {
        self.flags() & flags::FIN != 0
    }

    /// 检查 SYN 标志
    pub const fn is_syn(&self) -> bool {
        self.flags() & flags::SYN != 0
    }

    /// 检查 RST 标志
    pub const fn is_rst(&self) -> bool {
        self.flags() & flags::RST != 0
    }

    /// 检查 PSH 标志
    pub const fn is_psh(&self) -> bool {
        self.flags() & flags::PSH != 0
    }

    /// 检查 ACK 标志
    pub const fn is_ack(&self) -> bool {
        self.flags() & flags::ACK != 0
    }

    /// 检查 URG 标志
    pub const fn is_urg(&self) -> bool {
        self.flags() & flags::URG != 0
    }

    /// 检查 ECE 标志
    pub const fn is_ece(&self) -> bool {
        self.flags() & flags::ECE != 0
    }

    /// 检查 CWR 标志
    pub const fn is_cwr(&self) -> bool {
        self.flags() & flags::CWR != 0
    }

    /// 验证数据偏移是否有效
    pub const fn is_valid_data_offset(&self) -> bool {
        let offset = self.data_offset();
        offset >= TCP_MIN_DATA_OFFSET && offset <= (TCP_MAX_HEADER_LEN / 4) as u8
    }

    /// 获取头部总长度（包含选项）
    pub const fn header_len(&self) -> usize {
        (self.data_offset() as usize) * 4
    }

    /// 设置校验和
    pub fn set_checksum(&mut self, checksum: u16) {
        self.checksum = checksum;
    }

    /// 将头部序列化为字节（不含选项）
    pub fn serialize(&self) -> [u8; Self::HEADER_SIZE] {
        let mut buf = [0u8; Self::HEADER_SIZE];
        buf[0..2].copy_from_slice(&self.source_port.to_be_bytes());
        buf[2..4].copy_from_slice(&self.destination_port.to_be_bytes());
        buf[4..8].copy_from_slice(&self.sequence_number.to_be_bytes());
        buf[8..12].copy_from_slice(&self.acknowledgment_number.to_be_bytes());
        buf[12..14].copy_from_slice(&self.data_offset_and_flags.to_be_bytes());
        buf[14..16].copy_from_slice(&self.window_size.to_be_bytes());
        buf[16..18].copy_from_slice(&self.checksum.to_be_bytes());
        buf[18..20].copy_from_slice(&self.urgent_pointer.to_be_bytes());
        buf
    }

    /// 创建 SYN 头部
    pub fn syn(
        source_port: u16,
        destination_port: u16,
        seq: u32,
        window_size: u16,
    ) -> Self {
        Self::new(
            source_port,
            destination_port,
            seq,
            0,
            TCP_MIN_DATA_OFFSET,
            flags::SYN,
            window_size,
        )
    }

    /// 创建 SYN-ACK 头部
    pub fn syn_ack(
        source_port: u16,
        destination_port: u16,
        seq: u32,
        ack: u32,
        window_size: u16,
    ) -> Self {
        Self::new(
            source_port,
            destination_port,
            seq,
            ack,
            TCP_MIN_DATA_OFFSET,
            flags::SYN | flags::ACK,
            window_size,
        )
    }

    /// 创建 ACK 头部
    pub fn ack(
        source_port: u16,
        destination_port: u16,
        seq: u32,
        ack: u32,
        window_size: u16,
    ) -> Self {
        Self::new(
            source_port,
            destination_port,
            seq,
            ack,
            TCP_MIN_DATA_OFFSET,
            flags::ACK,
            window_size,
        )
    }

    /// 创建 FIN 头部
    pub fn fin(
        source_port: u16,
        destination_port: u16,
        seq: u32,
        ack: u32,
        window_size: u16,
    ) -> Self {
        Self::new(
            source_port,
            destination_port,
            seq,
            ack,
            TCP_MIN_DATA_OFFSET,
            flags::FIN | flags::ACK,
            window_size,
        )
    }

    /// 创建 RST 头部
    pub fn rst(
        source_port: u16,
        destination_port: u16,
        seq: u32,
        ack: u32,
    ) -> Self {
        Self::new(
            source_port,
            destination_port,
            seq,
            ack,
            TCP_MIN_DATA_OFFSET,
            flags::RST | flags::ACK,
            0,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_header_size() {
        assert_eq!(TcpHeader::HEADER_SIZE, 20);
    }

    #[test]
    fn test_header_new() {
        let header = TcpHeader::new(1234, 5678, 1000, 500, 5, flags::ACK, 8192);
        assert_eq!(header.source_port, 1234);
        assert_eq!(header.destination_port, 5678);
        assert_eq!(header.sequence_number, 1000);
        assert_eq!(header.acknowledgment_number, 500);
        assert_eq!(header.data_offset(), 5);
        assert!(header.is_ack());
        assert_eq!(header.window_size, 8192);
    }

    #[test]
    fn test_header_parse() {
        let bytes = [
            0x04, 0xD2, // Source Port: 1234
            0x16, 0x2E, // Dest Port: 5678
            0x00, 0x00, 0x03, 0xE8, // Seq: 1000
            0x00, 0x00, 0x01, 0xF4, // Ack: 500
            0x50, 0x10, // Data Offset: 5, Flags: ACK
            0x20, 0x00, // Window: 8192
            0x00, 0x00, // Checksum
            0x00, 0x00, // Urgent Pointer
        ];

        let header = TcpHeader::parse(&bytes).unwrap();
        assert_eq!(header.source_port, 1234);
        assert_eq!(header.destination_port, 5678);
        assert_eq!(header.sequence_number, 1000);
        assert_eq!(header.acknowledgment_number, 500);
        assert_eq!(header.data_offset(), 5);
        assert!(header.is_ack());
        assert_eq!(header.window_size, 8192);
    }

    #[test]
    fn test_header_serialize() {
        let header = TcpHeader::new(1234, 5678, 1000, 500, 5, flags::ACK, 8192);
        let bytes = header.serialize();

        assert_eq!(bytes[0..2], 1234u16.to_be_bytes());
        assert_eq!(bytes[2..4], 5678u16.to_be_bytes());
        assert_eq!(bytes[4..8], 1000u32.to_be_bytes());
        assert_eq!(bytes[8..12], 500u32.to_be_bytes());
        assert_eq!(bytes[12..14], 0x5010u16.to_be_bytes()); // Data Offset + Flags
        assert_eq!(bytes[14..16], 8192u16.to_be_bytes());
    }

    #[test]
    fn test_header_flags() {
        let mut header = TcpHeader::new(0, 0, 0, 0, 5, 0, 0);

        // 测试 SYN
        header.set_flags(flags::SYN);
        assert!(header.is_syn());
        assert!(!header.is_ack());
        assert!(!header.is_fin());

        // 测试 SYN + ACK
        header.set_flags(flags::SYN | flags::ACK);
        assert!(header.is_syn());
        assert!(header.is_ack());
        assert!(!header.is_fin());

        // 测试 FIN + ACK
        header.set_flags(flags::FIN | flags::ACK);
        assert!(header.is_fin());
        assert!(header.is_ack());
        assert!(!header.is_syn());
    }

    #[test]
    fn test_header_data_offset() {
        let mut header = TcpHeader::new(0, 0, 0, 0, 5, 0, 0);
        assert_eq!(header.data_offset(), 5);
        assert_eq!(header.header_len(), 20);

        header.set_data_offset(6);
        assert_eq!(header.data_offset(), 6);
        assert_eq!(header.header_len(), 24);
    }

    #[test]
    fn test_header_syn() {
        let header = TcpHeader::syn(1234, 5678, 1000, 8192);
        assert!(header.is_syn());
        assert!(!header.is_ack());
        assert_eq!(header.sequence_number, 1000);
    }

    #[test]
    fn test_header_syn_ack() {
        let header = TcpHeader::syn_ack(1234, 5678, 2000, 1001, 8192);
        assert!(header.is_syn());
        assert!(header.is_ack());
        assert_eq!(header.sequence_number, 2000);
        assert_eq!(header.acknowledgment_number, 1001);
    }

    #[test]
    fn test_header_ack() {
        let header = TcpHeader::ack(1234, 5678, 1001, 3000, 8192);
        assert!(header.is_ack());
        assert!(!header.is_syn());
        assert!(!header.is_fin());
        assert_eq!(header.acknowledgment_number, 3000);
    }

    #[test]
    fn test_header_fin() {
        let header = TcpHeader::fin(1234, 5678, 5000, 4000, 8192);
        assert!(header.is_fin());
        assert!(header.is_ack());
        assert_eq!(header.sequence_number, 5000);
        assert_eq!(header.acknowledgment_number, 4000);
    }

    #[test]
    fn test_header_rst() {
        let header = TcpHeader::rst(1234, 5678, 0, 0);
        assert!(header.is_rst());
        assert!(header.is_ack());
        assert_eq!(header.window_size, 0);
    }

    #[test]
    fn test_header_serialize_parse_roundtrip() {
        let mut original = TcpHeader::new(
            8080,
            443,
            12345,
            54321,
            6,
            flags::ACK | flags::PSH,
            16384,
        );
        original.set_checksum(0xABCD);

        let bytes = original.serialize();
        let parsed = TcpHeader::parse(&bytes).unwrap();

        assert_eq!(parsed.source_port, original.source_port);
        assert_eq!(parsed.destination_port, original.destination_port);
        assert_eq!(parsed.sequence_number, original.sequence_number);
        assert_eq!(parsed.acknowledgment_number, original.acknowledgment_number);
        assert_eq!(parsed.data_offset(), original.data_offset());
        assert_eq!(parsed.flags(), original.flags());
        assert_eq!(parsed.window_size, original.window_size);
        assert_eq!(parsed.checksum, original.checksum);
    }

    #[test]
    fn test_header_parse_too_short() {
        let bytes = [0x04, 0xD2, 0x16, 0x2E]; // 只有 4 字节
        let result = TcpHeader::parse(&bytes);
        assert!(result.is_err());
    }

    #[test]
    fn test_header_invalid_data_offset() {
        let bytes = [
            0x04, 0xD2, // Source Port
            0x16, 0x2E, // Dest Port
            0x00, 0x00, 0x03, 0xE8, // Seq
            0x00, 0x00, 0x01, 0xF4, // Ack
            0x40, 0x10, // Data Offset: 4 (invalid, < 5), Flags: ACK
            0x20, 0x00, // Window
            0x00, 0x00, // Checksum
            0x00, 0x00, // Urgent Pointer
        ];

        let result = TcpHeader::parse(&bytes);
        assert!(result.is_err());
    }
}
