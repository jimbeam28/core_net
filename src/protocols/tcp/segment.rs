// src/protocols/tcp/segment.rs
//
// TCP 报文段结构

use crate::common::{CoreError, Result};
use crate::protocols::Ipv4Addr;
use crate::protocols::ip::{add_ipv4_pseudo_header, fold_carry};
use super::header::TcpHeader;
use super::constant::TCP_MIN_HEADER_LEN;

/// TCP 报文段
///
/// 包含 TCP 头部、选项和数据载荷。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TcpSegment<'a> {
    /// TCP 头部
    pub header: TcpHeader,
    /// TCP 选项（如果存在）
    pub options: Vec<u8>,
    /// TCP 数据载荷
    pub payload: &'a [u8],
}

impl<'a> TcpSegment<'a> {
    /// 从字节流解析 TCP 报文段
    ///
    /// # 参数
    /// - data: 包含 TCP 报文段的字节切片
    ///
    /// # 返回
    /// - Ok(TcpSegment): 解析成功
    /// - Err(CoreError): 解析失败
    pub fn parse(data: &'a [u8]) -> Result<Self> {
        if data.len() < TCP_MIN_HEADER_LEN {
            return Err(CoreError::ParseError(format!(
                "TCP header too short: expected at least {} bytes, got {}",
                TCP_MIN_HEADER_LEN,
                data.len()
            )));
        }

        let header = TcpHeader::parse(data)?;

        let header_len = header.header_len();
        if data.len() < header_len {
            return Err(CoreError::ParseError(format!(
                "TCP data too short: expected {} bytes for header, got {}",
                header_len,
                data.len()
            )));
        }

        // 提取选项（如果有）
        let options_start = TCP_MIN_HEADER_LEN;
        let options_end = header_len;
        let options = if options_end > options_start {
            data[options_start..options_end].to_vec()
        } else {
            Vec::new()
        };

        // 提取载荷
        let payload = if data.len() > header_len {
            &data[header_len..]
        } else {
            &[]
        };

        Ok(Self {
            header,
            options,
            payload,
        })
    }

    /// 计算 TCP 校验和（包含伪头部）
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
        add_ipv4_pseudo_header(&mut sum, source_ip, dest_ip);
        sum += u32::from(6u16) << 8; // 协议号 TCP=6

        let tcp_len = (self.header.header_len() + self.payload.len() + self.options.len()) as u16;
        sum += u32::from(tcp_len >> 8) << 8; // 高字节
        sum += u32::from(tcp_len & 0xFF) << 8; // 低字节

        // 构建完整的字节数组（头部 + 选项 + 数据）
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&self.header.serialize());
        bytes.extend_from_slice(&self.options);
        bytes.extend_from_slice(self.payload);

        // TCP 数据（使用 16 位字累加，跳过校验和字段）
        let mut i = 0;
        while i + 1 < bytes.len() {
            if i != 16 && i != 17 { // 跳过校验和字段（字节 16-17）
                let word = u16::from_be_bytes([bytes[i], bytes[i + 1]]);
                sum += u32::from(word);
            }
            i += 2;
        }
        if i < bytes.len() && i != 16 && i != 17 {
            sum += u32::from(bytes[i]) << 8;
        }

        // 处理进位
        !fold_carry(sum)
    }

    /// 验证校验和
    pub fn verify_checksum(&self, source_ip: Ipv4Addr, dest_ip: Ipv4Addr) -> bool {
        self.calculate_checksum(source_ip, dest_ip) == self.header.checksum
    }

    /// 获取报文段总长度
    pub fn len(&self) -> usize {
        self.header.header_len() + self.payload.len()
    }

    /// 检查报文段是否为空
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// 检查是否为纯 ACK（无数据，只有 ACK 标志）
    pub fn is_pure_ack(&self) -> bool {
        self.header.is_ack() && !self.header.is_syn() && !self.header.is_fin()
            && self.payload.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_segment_parse_basic() {
        let bytes = [
            // TCP 头部
            0x04, 0xD2, // Source Port: 1234
            0x16, 0x2E, // Dest Port: 5678
            0x00, 0x00, 0x03, 0xE8, // Seq: 1000
            0x00, 0x00, 0x01, 0xF4, // Ack: 500
            0x50, 0x18, // Data Offset: 5, Flags: ACK + PSH
            0x20, 0x00, // Window: 8192
            0x00, 0x00, // Checksum
            0x00, 0x00, // Urgent Pointer
            // 数据
            0x48, 0x65, 0x6C, 0x6C, 0x6F, // "Hello"
        ];

        let segment = TcpSegment::parse(&bytes).unwrap();
        assert_eq!(segment.header.source_port, 1234);
        assert_eq!(segment.header.destination_port, 5678);
        assert!(segment.header.is_ack());
        assert!(segment.header.is_psh());
        assert_eq!(segment.payload, b"Hello");
    }

    #[test]
    fn test_segment_parse_with_options() {
        let bytes = [
            // TCP 头部
            0x04, 0xD2, // Source Port
            0x16, 0x2E, // Dest Port
            0x00, 0x00, 0x03, 0xE8, // Seq
            0x00, 0x00, 0x01, 0xF4, // Ack
            0x60, 0x02, // Data Offset: 6, Flags: SYN
            0x20, 0x00, // Window
            0x00, 0x00, // Checksum
            0x00, 0x00, // Urgent Pointer
            // 选项 (4 字节)
            0x02, 0x04, 0x05, 0xB4, // MSS=1460
            // 无数据
        ];

        let segment = TcpSegment::parse(&bytes).unwrap();
        assert_eq!(segment.header.data_offset(), 6);
        assert_eq!(segment.options.len(), 4);
        assert_eq!(segment.options[0], 2); // MSS Kind
        assert!(segment.payload.is_empty());
    }

    #[test]
    fn test_segment_parse_too_short() {
        let bytes = [0x04, 0xD2, 0x16, 0x2E]; // 只有 4 字节
        let result = TcpSegment::parse(&bytes);
        assert!(result.is_err());
    }

    #[test]
    fn test_segment_len() {
        let bytes = [
            0x04, 0xD2, 0x16, 0x2E, // Ports
            0x00, 0x00, 0x03, 0xE8, // Seq
            0x00, 0x00, 0x01, 0xF4, // Ack
            0x50, 0x18, // Data Offset: 5, Flags
            0x20, 0x00, // Window
            0x00, 0x00, // Checksum
            0x00, 0x00, // Urgent Pointer
            0x48, 0x65, 0x6C, 0x6C, 0x6F, // Data
        ];

        let segment = TcpSegment::parse(&bytes).unwrap();
        assert_eq!(segment.len(), 25); // 20 + 5
    }

    #[test]
    fn test_segment_is_pure_ack() {
        let bytes = [
            0x04, 0xD2, 0x16, 0x2E,
            0x00, 0x00, 0x03, 0xE8,
            0x00, 0x00, 0x01, 0xF4,
            0x50, 0x10, // Flags: ACK only
            0x20, 0x00,
            0x00, 0x00,
            0x00, 0x00,
            // 无数据
        ];

        let segment = TcpSegment::parse(&bytes).unwrap();
        assert!(segment.is_pure_ack());
    }

    #[test]
    fn test_segment_syn_is_not_pure_ack() {
        let bytes = [
            0x04, 0xD2, 0x16, 0x2E,
            0x00, 0x00, 0x03, 0xE8,
            0x00, 0x00, 0x00, 0x00,
            0x50, 0x02, // Flags: SYN
            0x20, 0x00,
            0x00, 0x00,
            0x00, 0x00,
        ];

        let segment = TcpSegment::parse(&bytes).unwrap();
        assert!(!segment.is_pure_ack());
    }

    #[test]
    fn test_segment_with_data_is_not_pure_ack() {
        let bytes = [
            0x04, 0xD2, 0x16, 0x2E,
            0x00, 0x00, 0x03, 0xE8,
            0x00, 0x00, 0x01, 0xF4,
            0x50, 0x18, // Flags: ACK + PSH
            0x20, 0x00,
            0x00, 0x00,
            0x00, 0x00,
            0x48, 0x65, 0x6C, 0x6C, 0x6F, // Data
        ];

        let segment = TcpSegment::parse(&bytes).unwrap();
        assert!(!segment.is_pure_ack());
    }
}
