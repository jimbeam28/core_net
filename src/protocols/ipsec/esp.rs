// src/protocols/ipsec/esp.rs
//
// ESP (Encapsulating Security Payload) 协议实现
// RFC 4303: IP Encapsulating Security Payload

use super::{IpsecError, IpsecResult};

/// ESP 协议号
pub const IP_PROTO_ESP: u8 = 50;

/// ESP 头最小长度（字节）
pub const ESP_HEADER_MIN_LEN: usize = 8;

/// ESP 头固定部分长度
const ESP_FIXED_HEADER_LEN: usize = 8;

/// ESP 尾固定部分长度
const ESP_TRAILER_FIXED_LEN: usize = 2;

/// ESP 报文头
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EspHeader {
    /// 安全参数索引
    pub spi: u32,
    /// 序列号
    pub sequence_number: u32,
}

impl EspHeader {
    /// 头部大小（字节）
    pub const fn header_size() -> usize {
        ESP_FIXED_HEADER_LEN
    }

    /// 创建新的 ESP 头
    pub fn new(spi: u32, sequence_number: u32) -> Self {
        Self {
            spi,
            sequence_number,
        }
    }

    /// 从字节流解析 ESP 头
    pub fn parse(data: &[u8]) -> IpsecResult<EspHeader> {
        if data.len() < ESP_HEADER_MIN_LEN {
            return Err(IpsecError::InvalidLength);
        }

        let spi = u32::from_be_bytes([data[0], data[1], data[2], data[3]]);
        let sequence_number = u32::from_be_bytes([data[4], data[5], data[6], data[7]]);

        Ok(Self {
            spi,
            sequence_number,
        })
    }

    /// 将 ESP 头序列化为字节
    pub fn to_bytes(&self) -> [u8; ESP_FIXED_HEADER_LEN] {
        let mut buffer = [0u8; ESP_FIXED_HEADER_LEN];
        buffer[0..4].copy_from_slice(&self.spi.to_be_bytes());
        buffer[4..8].copy_from_slice(&self.sequence_number.to_be_bytes());
        buffer
    }
}

/// ESP 报文尾
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EspTrailer {
    /// 填充长度
    pub pad_length: u8,
    /// 下一个头部
    pub next_header: u8,
    /// 填充数据
    pub padding: Vec<u8>,
}

impl EspTrailer {
    /// 尾部大小（字节）
    pub fn trailer_size(&self) -> usize {
        ESP_TRAILER_FIXED_LEN + self.padding.len()
    }

    /// 创建新的 ESP 尾
    pub fn new(pad_length: u8, next_header: u8, padding: Vec<u8>) -> Self {
        Self {
            pad_length,
            next_header,
            padding,
        }
    }

    /// 根据块大小计算填充长度
    ///
    /// # 参数
    /// - `payload_len`: 载荷长度
    /// - `block_size`: 加密块大小（字节）
    pub fn calculate_padding(payload_len: usize, block_size: usize) -> usize {
        let total_len = payload_len + ESP_TRAILER_FIXED_LEN;
        let pad_len = (block_size - (total_len % block_size)) % block_size;
        pad_len.max(1) // 至少需要 1 字节填充
    }

    /// 从字节流解析 ESP 尾
    pub fn parse(data: &[u8]) -> IpsecResult<(EspTrailer, usize)> {
        if data.len() < ESP_TRAILER_FIXED_LEN {
            return Err(IpsecError::InvalidLength);
        }

        // ESP 尾在数据包的末尾
        // 格式: ... [Padding] [PadLen] [NextHeader]
        let pad_length = data[data.len() - 2] as usize;
        let next_header = data[data.len() - 1];

        if data.len() < ESP_TRAILER_FIXED_LEN + pad_length {
            return Err(IpsecError::InvalidLength);
        }

        let padding_start = data.len() - ESP_TRAILER_FIXED_LEN - pad_length;
        let padding = data[padding_start..data.len() - ESP_TRAILER_FIXED_LEN].to_vec();

        let trailer = EspTrailer {
            pad_length: pad_length as u8,
            next_header,
            padding,
        };

        // 返回尾部和载荷长度（不含尾和 ICV）
        Ok((trailer, padding_start))
    }

    /// 将 ESP 尾序列化为字节
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut buffer = Vec::with_capacity(self.trailer_size());
        buffer.extend_from_slice(&self.padding);
        buffer.push(self.pad_length);
        buffer.push(self.next_header);
        buffer
    }
}

/// ESP 完整报文
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EspPacket {
    /// ESP 头
    pub header: EspHeader,
    /// 加密的载荷数据（可能包含 IV）
    pub encrypted_data: Vec<u8>,
    /// ESP 尾
    pub trailer: EspTrailer,
    /// 完整性校验值（可选）
    pub icv: Option<Vec<u8>>,
}

impl EspPacket {
    /// 创建新的 ESP 报文
    pub fn new(
        spi: u32,
        sequence_number: u32,
        encrypted_data: Vec<u8>,
        trailer: EspTrailer,
        icv: Option<Vec<u8>>,
    ) -> Self {
        let header = EspHeader::new(spi, sequence_number);
        Self {
            header,
            encrypted_data,
            trailer,
            icv,
        }
    }

    /// 从字节流解析 ESP 报文
    ///
    /// # 参数
    /// - `data`: ESP 报文数据（不含外层 IP 头）
    /// - `icv_len`: ICV 长度（0 表示无 ICV）
    pub fn parse(data: &[u8], icv_len: usize) -> IpsecResult<Self> {
        if data.len() < ESP_HEADER_MIN_LEN + ESP_TRAILER_FIXED_LEN {
            return Err(IpsecError::InvalidLength);
        }

        // 解析 ESP 头
        let header = EspHeader::parse(data)?;

        // 计算加密数据部分的结束位置
        // 格式: [ESP 头] [加密数据] [填充] [PadLen] [NextHeader] [ICV]
        let data_without_icv = if icv_len > 0 {
            if data.len() < ESP_HEADER_MIN_LEN + icv_len {
                return Err(IpsecError::InvalidLength);
            }
            &data[..data.len() - icv_len]
        } else {
            data
        };

        // 解析 ESP 尾
        let (trailer, payload_end) = EspTrailer::parse(data_without_icv)?;

        // 提取加密数据
        let encrypted_data = data[ESP_FIXED_HEADER_LEN..payload_end].to_vec();

        // 提取 ICV
        let icv = if icv_len > 0 {
            Some(data[data.len() - icv_len..].to_vec())
        } else {
            None
        };

        Ok(Self {
            header,
            encrypted_data,
            trailer,
            icv,
        })
    }

    /// 将 ESP 报文序列化为字节
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut buffer = Vec::new();

        // ESP 头
        buffer.extend_from_slice(&self.header.to_bytes());

        // 加密数据
        buffer.extend_from_slice(&self.encrypted_data);

        // ESP 尾
        buffer.extend_from_slice(&self.trailer.to_bytes());

        // ICV
        if let Some(ref icv) = self.icv {
            buffer.extend_from_slice(icv);
        }

        buffer
    }

    /// 获取载荷数据（加密前的原始数据）
    ///
    /// 注意：此方法仅用于模拟，实际应用中需要解密
    pub fn get_payload(&self) -> Vec<u8> {
        // 在模拟环境中，我们假设"加密"数据就是原始数据
        // 实际应用中需要解密
        self.encrypted_data.clone()
    }

    /// 创建简单的 ESP 报文（模拟加密）
    ///
    /// # 参数
    /// - `spi`: 安全参数索引
    /// - `sequence_number`: 序列号
    /// - `payload`: 原始载荷数据
    /// - `next_header`: 下一个协议号
    /// - `block_size`: 加密块大小（用于填充计算）
    pub fn create_simple(
        spi: u32,
        sequence_number: u32,
        mut payload: Vec<u8>,
        next_header: u8,
        block_size: usize,
    ) -> Self {
        // 计算填充
        let pad_len = EspTrailer::calculate_padding(payload.len(), block_size);
        let padding = vec![0x00; pad_len];

        // 创建 ESP 尾
        let trailer = EspTrailer {
            pad_length: pad_len as u8,
            next_header,
            padding,
        };

        // 在模拟环境中，"加密"就是原始数据
        let encrypted_data = payload;

        Self {
            header: EspHeader::new(spi, sequence_number),
            encrypted_data,
            trailer,
            icv: None, // 简单版本不含 ICV
        }
    }

    /// 验证 ICV（如果有）
    pub fn verify_icv(&self, key: &[u8]) -> bool {
        if let Some(ref icv) = self.icv {
            // 简化的 ICV 验证
            // 实际应用中应使用 HMAC-SHA1 或 HMAC-SHA256
            let computed = self.compute_icv(key);
            computed == *icv
        } else {
            true // 无 ICV 时默认通过
        }
    }

    /// 计算 ICV（简化实现）
    fn compute_icv(&self, key: &[u8]) -> Vec<u8> {
        // 简化实现：使用 key 与 SPI/序列号的异或
        let mut result = vec![0u8; 12.min(key.len())];

        let spi_bytes = self.header.spi.to_be_bytes();
        let seq_bytes = self.header.sequence_number.to_be_bytes();

        for (i, byte) in result.iter_mut().enumerate() {
            let key_byte = key.get(i).copied().unwrap_or(0);
            let data_byte = spi_bytes.get(i % 4).copied().unwrap_or(0)
                ^ seq_bytes.get(i % 4).copied().unwrap_or(0);
            *byte = key_byte ^ data_byte;
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_esp_header_creation() {
        let header = EspHeader::new(0x12345678, 42);

        assert_eq!(header.spi, 0x12345678);
        assert_eq!(header.sequence_number, 42);
    }

    #[test]
    fn test_esp_header_roundtrip() {
        let original = EspHeader::new(0xDEADBEEF, 100);
        let bytes = original.to_bytes();
        let parsed = EspHeader::parse(&bytes).unwrap();

        assert_eq!(parsed.spi, original.spi);
        assert_eq!(parsed.sequence_number, original.sequence_number);
    }

    #[test]
    fn test_esp_trailer_creation() {
        let trailer = EspTrailer::new(3, 6, vec![0, 0, 0]);

        assert_eq!(trailer.pad_length, 3);
        assert_eq!(trailer.next_header, 6);
        assert_eq!(trailer.padding.len(), 3);
        assert_eq!(trailer.trailer_size(), 5);
    }

    #[test]
    fn test_esp_trailer_calculate_padding() {
        // 块大小 16 字节
        let payload_len = 10;
        let block_size = 16;
        let pad_len = EspTrailer::calculate_padding(payload_len, block_size);

        // 10 + 2 (尾) = 12, 需要填充到 16
        assert_eq!(pad_len, 4);
    }

    #[test]
    fn test_esp_trailer_roundtrip() {
        let original = EspTrailer::new(3, 17, vec![1, 2, 3]);
        let bytes = original.to_bytes();
        let data_with_trailer = {
            let mut data = vec![0u8; 10]; // 模拟载荷
            data.extend_from_slice(&bytes);
            data
        };

        let (parsed, _) = EspTrailer::parse(&data_with_trailer).unwrap();

        assert_eq!(parsed.pad_length, original.pad_length);
        assert_eq!(parsed.next_header, original.next_header);
        assert_eq!(parsed.padding, original.padding);
    }

    #[test]
    fn test_esp_packet_creation() {
        let packet = EspPacket::create_simple(
            0x12345678,
            1,
            vec![1, 2, 3, 4],
            6, // TCP
            16, // AES 块大小
        );

        assert_eq!(packet.header.spi, 0x12345678);
        assert_eq!(packet.header.sequence_number, 1);
        assert_eq!(packet.encrypted_data, vec![1, 2, 3, 4]);
        assert_eq!(packet.trailer.next_header, 6);
    }

    #[test]
    fn test_esp_packet_roundtrip() {
        let original = EspPacket::create_simple(
            0xABCD1234,
            100,
            vec![1, 2, 3, 4, 5, 6, 7, 8],
            17, // UDP
            16,
        );

        let bytes = original.to_bytes();
        let parsed = EspPacket::parse(&bytes, 0).unwrap();

        assert_eq!(parsed.header.spi, original.header.spi);
        assert_eq!(parsed.header.sequence_number, original.header.sequence_number);
        assert_eq!(parsed.encrypted_data, original.encrypted_data);
        assert_eq!(parsed.trailer.next_header, original.trailer.next_header);
    }

    #[test]
    fn test_esp_packet_with_icv() {
        let mut packet = EspPacket::create_simple(
            0x12345678,
            1,
            vec![1, 2, 3, 4],
            6,
            16,
        );

        // 添加 ICV
        packet.icv = Some(vec![0xAA; 12]);

        let bytes = packet.to_bytes();
        let parsed = EspPacket::parse(&bytes, 12).unwrap();

        assert!(parsed.icv.is_some());
        assert_eq!(parsed.icv.as_ref().unwrap().len(), 12);
    }

    #[test]
    fn test_invalid_length() {
        let data = [1, 2, 3]; // 太短
        let result = EspHeader::parse(&data);
        assert!(matches!(result, Err(IpsecError::InvalidLength)));
    }
}
