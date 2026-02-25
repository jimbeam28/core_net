// src/protocols/ipsec/ah.rs
//
// AH (Authentication Header) 协议实现
// RFC 4302: IP Authentication Header

use super::{IpsecError, IpsecResult};

/// AH 协议号
pub const IP_PROTO_AH: u8 = 51;

/// AH 头最小长度（字节）
pub const AH_HEADER_MIN_LEN: usize = 12;

/// AH 头固定部分长度（不含 ICV）
const AH_FIXED_HEADER_LEN: usize = 12;

/// AH 报文头
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AhHeader {
    /// 紧跟 AH 头的协议类型
    pub next_header: u8,
    /// AH 头长度（以 32 位字为单位，减 2）
    pub payload_len: u8,
    /// 安全参数索引
    pub spi: u32,
    /// 序列号
    pub sequence_number: u32,
}

impl AhHeader {
    /// 头部大小（字节）
    pub const fn header_size() -> usize {
        AH_FIXED_HEADER_LEN
    }

    /// 创建新的 AH 头
    pub fn new(next_header: u8, spi: u32, sequence_number: u32, icv_len: usize) -> Self {
        // payload_len = (固定头长度 + ICV 长度) / 4 - 2
        let total_len = AH_FIXED_HEADER_LEN + icv_len;
        let payload_len = (total_len / 4).saturating_sub(2) as u8;

        Self {
            next_header,
            payload_len,
            spi,
            sequence_number,
        }
    }

    /// 获取 ICV 长度
    pub fn icv_len(&self) -> usize {
        // (payload_len + 2) * 4 - 固定头长度
        let total_len = (self.payload_len as usize + 2) * 4;
        total_len.saturating_sub(AH_FIXED_HEADER_LEN)
    }

    /// 从字节流解析 AH 头
    pub fn parse(data: &[u8]) -> IpsecResult<(AhHeader, Vec<u8>)> {
        if data.len() < AH_HEADER_MIN_LEN {
            return Err(IpsecError::InvalidLength);
        }

        let next_header = data[0];
        let payload_len = data[1];
        let spi = u32::from_be_bytes([data[4], data[5], data[6], data[7]]);
        let sequence_number = u32::from_be_bytes([data[8], data[9], data[10], data[11]]);

        // 计算总长度和 ICV 长度
        let total_len = (payload_len as usize + 2) * 4;

        if data.len() < total_len {
            return Err(IpsecError::InvalidLength);
        }

        // 提取 ICV
        let icv = data[AH_FIXED_HEADER_LEN..total_len].to_vec();

        let header = AhHeader {
            next_header,
            payload_len,
            spi,
            sequence_number,
        };

        Ok((header, icv))
    }

    /// 将 AH 头序列化为字节
    pub fn to_bytes(&self, icv: &[u8]) -> Vec<u8> {
        let mut buffer = Vec::with_capacity(AH_FIXED_HEADER_LEN + icv.len());

        buffer.push(self.next_header);
        buffer.push(self.payload_len);
        buffer.extend_from_slice(&[0u8; 2]); // 保留字段
        buffer.extend_from_slice(&self.spi.to_be_bytes());
        buffer.extend_from_slice(&self.sequence_number.to_be_bytes());
        buffer.extend_from_slice(icv);

        buffer
    }
}

/// AH 完整报文
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AhPacket {
    /// AH 头
    pub header: AhHeader,
    /// 完整性校验值
    pub icv: Vec<u8>,
    /// 载荷数据（不含 AH 头）
    pub payload: Vec<u8>,
}

impl AhPacket {
    /// 创建新的 AH 报文
    pub fn new(next_header: u8, spi: u32, sequence_number: u32,
               icv: Vec<u8>, payload: Vec<u8>) -> Self {
        let header = AhHeader::new(next_header, spi, sequence_number, icv.len());
        Self {
            header,
            icv,
            payload,
        }
    }

    /// 从字节流解析 AH 报文
    pub fn parse(data: &[u8]) -> IpsecResult<Self> {
        let (header, icv) = AhHeader::parse(data)?;

        let total_len = (header.payload_len as usize + 2) * 4;
        let payload_start = total_len;

        let payload = if data.len() > payload_start {
            data[payload_start..].to_vec()
        } else {
            Vec::new()
        };

        Ok(Self {
            header,
            icv,
            payload,
        })
    }

    /// 将 AH 报文序列化为字节
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut buffer = self.header.to_bytes(&self.icv);
        buffer.extend_from_slice(&self.payload);
        buffer
    }

    /// 计算 ICV（简化实现，实际应使用 HMAC）
    ///
    /// 注意：这是模拟实现，实际应用中应使用真正的加密库
    pub fn compute_icv(data: &[u8], key: &[u8], _icv_len: usize) -> Vec<u8> {
        // 简化的 ICV 计算：key 与数据的异或后取前 12 字节
        // 实际应用中应使用 HMAC-SHA1 或 HMAC-SHA256
        let mut result = vec![0u8; 12.min(key.len().max(data.len()))];

        for (i, byte) in result.iter_mut().enumerate() {
            let key_byte = key.get(i).copied().unwrap_or(0);
            let data_byte = data.get(i).copied().unwrap_or(0);
            *byte = key_byte ^ data_byte;
        }

        result
    }

    /// 验证 ICV
    pub fn verify_icv(&self, data: &[u8], key: &[u8]) -> bool {
        let computed = Self::compute_icv(data, key, self.icv.len());
        // 简单的比较，实际应使用恒定时间比较
        computed == self.icv
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ah_header_creation() {
        let header = AhHeader::new(6, 0x12345678, 1, 12);

        assert_eq!(header.next_header, 6);
        assert_eq!(header.spi, 0x12345678);
        assert_eq!(header.sequence_number, 1);
        assert_eq!(header.icv_len(), 12);
    }

    #[test]
    fn test_ah_header_roundtrip() {
        let original = AhHeader::new(17, 0xDEADBEEF, 42, 12);
        let icv = vec![0u8; 12];
        let bytes = original.to_bytes(&icv);

        let (parsed, parsed_icv) = AhHeader::parse(&bytes).unwrap();

        assert_eq!(parsed.next_header, original.next_header);
        assert_eq!(parsed.spi, original.spi);
        assert_eq!(parsed.sequence_number, original.sequence_number);
        assert_eq!(parsed_icv, icv);
    }

    #[test]
    fn test_ah_packet_creation() {
        let packet = AhPacket::new(
            6,
            0x12345678,
            1,
            vec![0u8; 12],
            vec![1, 2, 3, 4],
        );

        assert_eq!(packet.header.next_header, 6);
        assert_eq!(packet.icv.len(), 12);
        assert_eq!(packet.payload, vec![1, 2, 3, 4]);
    }

    #[test]
    fn test_ah_packet_roundtrip() {
        let original = AhPacket::new(
            17,
            0xABCD1234,
            100,
            vec![0xAA; 12],
            vec![1, 2, 3, 4, 5],
        );

        let bytes = original.to_bytes();
        let parsed = AhPacket::parse(&bytes).unwrap();

        assert_eq!(parsed.header.next_header, original.header.next_header);
        assert_eq!(parsed.header.spi, original.header.spi);
        assert_eq!(parsed.header.sequence_number, original.header.sequence_number);
        assert_eq!(parsed.icv, original.icv);
        assert_eq!(parsed.payload, original.payload);
    }

    #[test]
    fn test_icv_computation() {
        let data = [1, 2, 3, 4];
        let key = [0xAA, 0xBB, 0xCC, 0xDD];

        let icv = AhPacket::compute_icv(&data, &key, 12);

        assert_eq!(icv.len(), 12);
        // 验证计算结果
        assert_eq!(icv[0], 0xAA ^ 1);
        assert_eq!(icv[1], 0xBB ^ 2);
        assert_eq!(icv[2], 0xCC ^ 3);
        assert_eq!(icv[3], 0xDD ^ 4);
    }

    #[test]
    fn test_icv_verification() {
        let data = [1, 2, 3, 4];
        let key = [0xAA, 0xBB, 0xCC, 0xDD];

        let icv = AhPacket::compute_icv(&data, &key, 12);
        let packet = AhPacket::new(6, 0x1234, 1, icv.clone(), data.to_vec());

        assert!(packet.verify_icv(&data, &key));
    }

    #[test]
    fn test_invalid_length() {
        let data = [1, 2, 3]; // 太短
        let result = AhHeader::parse(&data);
        assert!(matches!(result, Err(IpsecError::InvalidLength)));
    }
}
