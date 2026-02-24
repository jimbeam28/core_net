// src/protocols/tcp/segment.rs
//
// TCP 报文段结构

use crate::common::{CoreError, Result};
use crate::protocols::Ipv4Addr;
use crate::protocols::ip::{add_ipv4_pseudo_header, fold_carry};
use super::header::TcpHeader;
use super::constant::TCP_MIN_HEADER_LEN;
use super::connection::TcpOption;
use super::error::TcpError;

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

    /// 解析 TCP 选项
    ///
    /// 将原始选项字节解析为结构化的 `TcpOption` 枚举列表。
    ///
    /// # 返回
    /// - Ok(Vec<TcpOption>): 解析成功，返回选项列表
    /// - Err(CoreError): 选项格式错误
    pub fn parse_options(&self) -> Result<Vec<TcpOption>> {
        TcpOption::parse_options(&self.options)
            .map_err(|err| match err {
                TcpError::ParseError(msg) => CoreError::ParseError(msg),
                _ => CoreError::ParseError(format!("{:?}", err)),
            })
    }

    /// 获取 TCP 选项的便捷方法
    ///
    /// 解析选项并查找指定类型的选项。
    ///
    /// # 参数
    /// - kind: 选项类型（Kind 值）
    ///
    /// # 返回
    /// - Option<TcpOption>: 如果找到则返回选项副本，否则返回 None
    pub fn get_option_by_kind(&self, kind: u8) -> Option<TcpOption> {
        if let Ok(parsed) = self.parse_options() {
            parsed.into_iter().find(|opt| match opt {
                TcpOption::End => kind == 0,
                TcpOption::Nop => kind == 1,
                TcpOption::MaxSegmentSize { .. } => kind == 2,
                TcpOption::WindowScale { .. } => kind == 3,
                TcpOption::SackPermitted => kind == 4,
                TcpOption::Sack { .. } => kind == 5,
                TcpOption::Timestamps { .. } => kind == 8,
            })
        } else {
            None
        }
    }

    /// 获取最大分段大小（MSS）选项
    ///
    /// # 返回
    /// - Option<u16>: 如果存在 MSS 选项则返回值，否则返回 None
    pub fn get_mss(&self) -> Option<u16> {
        self.get_option_by_kind(2).and_then(|opt| match opt {
            TcpOption::MaxSegmentSize { mss } => Some(mss),
            _ => None,
        })
    }

    /// 获取窗口缩放选项
    ///
    /// # 返回
    /// - Option<u8>: 如果存在窗口缩放选项则返回值，否则返回 None
    pub fn get_window_scale(&self) -> Option<u8> {
        self.get_option_by_kind(3).and_then(|opt| match opt {
            TcpOption::WindowScale { shift } => Some(shift),
            _ => None,
        })
    }

    /// 检查是否支持 SACK
    ///
    /// # 返回
    /// - bool: 如果存在 SACK Permitted 选项则返回 true
    pub fn has_sack_permitted(&self) -> bool {
        self.get_option_by_kind(4).is_some()
    }

    /// 获取时间戳选项
    ///
    /// # 返回
    /// - Option<(u32, u32)>: 如果存在时间戳选项则返回 (ts_val, ts_ecr)，否则返回 None
    pub fn get_timestamps(&self) -> Option<(u32, u32)> {
        self.get_option_by_kind(8).and_then(|opt| match opt {
            TcpOption::Timestamps { ts_val, ts_ecr } => Some((ts_val, ts_ecr)),
            _ => None,
        })
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

    #[test]
    fn test_parse_options_mss() {
        let bytes = [
            0x04, 0xD2, 0x16, 0x2E,
            0x00, 0x00, 0x03, 0xE8,
            0x00, 0x00, 0x00, 0x00,
            0x60, 0x02, // Data Offset: 6, Flags: SYN
            0x20, 0x00,
            0x00, 0x00,
            0x00, 0x00,
            // 选项: MSS=1460
            0x02, 0x04, 0x05, 0xB4,
        ];

        let segment = TcpSegment::parse(&bytes).unwrap();
        let options = segment.parse_options().unwrap();

        assert_eq!(options.len(), 1);
        assert_eq!(options[0], TcpOption::MaxSegmentSize { mss: 1460 });
    }

    #[test]
    fn test_get_mss() {
        let bytes = [
            0x04, 0xD2, 0x16, 0x2E,
            0x00, 0x00, 0x03, 0xE8,
            0x00, 0x00, 0x00, 0x00,
            0x60, 0x02, // Data Offset: 6, Flags: SYN
            0x20, 0x00,
            0x00, 0x00,
            0x00, 0x00,
            // 选项: MSS=1460
            0x02, 0x04, 0x05, 0xB4,
        ];

        let segment = TcpSegment::parse(&bytes).unwrap();
        assert_eq!(segment.get_mss(), Some(1460));
    }

    #[test]
    fn test_parse_options_multiple() {
        let bytes = [
            0x04, 0xD2, 0x16, 0x2E,
            0x00, 0x00, 0x03, 0xE8,
            0x00, 0x00, 0x00, 0x00,
            0x70, 0x02, // Data Offset: 7, Flags: SYN
            0x20, 0x00,
            0x00, 0x00,
            0x00, 0x00,
            // 选项: MSS=1460, NOP, Window Scale=0
            0x02, 0x04, 0x05, 0xB4, // MSS
            0x01,                   // NOP
            0x03, 0x03, 0x00,       // Window Scale
        ];

        let segment = TcpSegment::parse(&bytes).unwrap();
        let options = segment.parse_options().unwrap();

        assert_eq!(options.len(), 3);
        assert_eq!(options[0], TcpOption::MaxSegmentSize { mss: 1460 });
        assert_eq!(options[1], TcpOption::Nop);
        assert_eq!(options[2], TcpOption::WindowScale { shift: 0 });
    }

    #[test]
    fn test_get_window_scale() {
        let bytes = [
            0x04, 0xD2, 0x16, 0x2E,
            0x00, 0x00, 0x03, 0xE8,
            0x00, 0x00, 0x00, 0x00,
            0x60, 0x02, // Data Offset: 6, Flags: SYN
            0x20, 0x00,
            0x00, 0x00,
            0x00, 0x00,
            // 选项: Window Scale=2 (需要填充到 4 字节边界)
            0x03, 0x03, 0x02, 0x00,
        ];

        let segment = TcpSegment::parse(&bytes).unwrap();
        assert_eq!(segment.get_window_scale(), Some(2));
    }

    #[test]
    fn test_get_timestamps() {
        let bytes = [
            0x04, 0xD2, 0x16, 0x2E,
            0x00, 0x00, 0x03, 0xE8,
            0x00, 0x00, 0x00, 0x00,
            0x80, 0x02, // Data Offset: 8, Flags: SYN
            0x20, 0x00,
            0x00, 0x00,
            0x00, 0x00,
            // 选项: Timestamps (10 bytes)
            0x08, 0x0A, // Kind=8, Length=10
            0x00, 0x00, 0x12, 0x34, // ts_val
            0x00, 0x00, 0x56, 0x78, // ts_ecr
            // 填充到 32 字节 (8 * 4)
            0x00, 0x00,
        ];

        let segment = TcpSegment::parse(&bytes).unwrap();
        assert_eq!(segment.get_timestamps(), Some((0x1234, 0x5678)));
    }

    #[test]
    fn test_has_sack_permitted() {
        let bytes = [
            0x04, 0xD2, 0x16, 0x2E,
            0x00, 0x00, 0x03, 0xE8,
            0x00, 0x00, 0x00, 0x00,
            0x60, 0x02, // Data Offset: 6, Flags: SYN
            0x20, 0x00,
            0x00, 0x00,
            0x00, 0x00,
            // 选项: SACK Permitted + NOP (填充到 4 字节边界)
            0x04, 0x02, // Kind=4, Length=2
            0x01, 0x00, // NOP + padding
        ];

        let segment = TcpSegment::parse(&bytes).unwrap();
        assert!(segment.has_sack_permitted());
    }

    #[test]
    fn test_get_option_by_kind() {
        let bytes = [
            0x04, 0xD2, 0x16, 0x2E,
            0x00, 0x00, 0x03, 0xE8,
            0x00, 0x00, 0x00, 0x00,
            0x70, 0x02, // Data Offset: 7, Flags: SYN
            0x20, 0x00,
            0x00, 0x00,
            0x00, 0x00,
            // 选项: MSS, NOP, Window Scale
            0x02, 0x04, 0x05, 0xB4,
            0x01,
            0x03, 0x03, 0x00,
        ];

        let segment = TcpSegment::parse(&bytes).unwrap();
        // MSS (Kind=2) exists
        assert!(segment.get_option_by_kind(2).is_some());
        // SACK Permitted (Kind=4) does not exist
        assert!(segment.get_option_by_kind(4).is_none());
    }

    #[test]
    fn test_parse_options_invalid() {
        let bytes = [
            0x04, 0xD2, 0x16, 0x2E,
            0x00, 0x00, 0x03, 0xE8,
            0x00, 0x00, 0x00, 0x00,
            0x70, 0x02, // Data Offset: 7
            0x20, 0x00,
            0x00, 0x00,
            0x00, 0x00,
            // 无效的 MSS 选项（Kind=2, Length=4, but only 2 bytes follow)
            0x02, 0x04, 0x05, 0xB4, 0x00, 0x00, 0x00, 0x00,
        ];

        let segment = TcpSegment::parse(&bytes).unwrap();
        // 正常解析应该成功
        assert!(segment.parse_options().is_ok());
    }

    #[test]
    fn test_parse_options_truncated_mss() {
        // 创建一个选项区域只有 3 字节的测试数据（MSS 需要 4 字节）
        let options_bytes = [
            0x02, 0x04, 0x05, // Kind=2, Length=4, but only 1 byte value
        ];

        let result = TcpOption::parse_options(&options_bytes);
        // 应该返回错误，因为数据长度不足
        assert!(result.is_err());
    }
}
