// src/protocols/ipsec/ikev2/message.rs
//
// IKEv2 消息结构定义和解析

use super::*;
use crate::common::error::CoreError;

// ========== IKE 消息头部 ==========

/// IKEv2 消息头部
///
/// RFC 7296 Section 2.4:
/// 文档格式略，参见 RFC 7296
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IkeHeader {
    /// 发起方 SPI
    pub initiator_spi: [u8; SPI_LEN],
    /// 响应方 SPI
    pub responder_spi: [u8; SPI_LEN],
    /// 下一个 Payload 类型
    pub next_payload: u8,
    /// 版本号
    pub version: u8,
    /// 交换类型
    pub exchange_type: IkeExchangeType,
    /// 标志位
    pub flags: IkeFlags,
    /// 消息 ID
    pub message_id: u32,
    /// 消息总长度
    pub length: u32,
}

impl IkeHeader {
    /// 创建新的 IKE 消息头部
    pub fn new(
        initiator_spi: [u8; SPI_LEN],
        responder_spi: [u8; SPI_LEN],
        next_payload: IkePayloadType,
        exchange_type: IkeExchangeType,
        flags: IkeFlags,
        message_id: u32,
    ) -> Self {
        Self {
            initiator_spi,
            responder_spi,
            next_payload: next_payload.as_u8(),
            version: IKEV2_VERSION,
            exchange_type,
            flags,
            message_id,
            length: IKE_HEADER_LEN as u32,
        }
    }

    /// 创建 IKE_SA_INIT 请求头部
    pub fn init_request(initiator_spi: [u8; SPI_LEN]) -> Self {
        Self {
            initiator_spi,
            responder_spi: [0u8; SPI_LEN],
            next_payload: IkePayloadType::SA.as_u8(),
            version: IKEV2_VERSION,
            exchange_type: IkeExchangeType::IkeSaInit,
            flags: IkeFlags::request(true),
            message_id: 0,
            length: IKE_HEADER_LEN as u32,
        }
    }

    /// 创建响应消息头部
    pub fn response(&self, next_payload: IkePayloadType, length: u32) -> Self {
        Self {
            initiator_spi: self.initiator_spi,
            responder_spi: self.responder_spi,
            next_payload: next_payload.as_u8(),
            version: IKEV2_VERSION,
            exchange_type: self.exchange_type,
            flags: IkeFlags::response(self.flags.initiator),
            message_id: self.message_id,
            length,
        }
    }

    /// 是否为响应消息
    pub fn is_response(&self) -> bool {
        self.flags.response
    }

    /// 是否为发起方发送的
    pub fn is_from_initiator(&self) -> bool {
        self.flags.initiator
    }

    /// 序列化头部为字节
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(IKE_HEADER_LEN);

        bytes.extend_from_slice(&self.initiator_spi);
        bytes.extend_from_slice(&self.responder_spi);
        bytes.push(self.next_payload);
        bytes.push(self.version);
        bytes.push(self.exchange_type.as_u8());
        bytes.push(self.flags.as_u8());
        bytes.extend_from_slice(&self.message_id.to_be_bytes());
        bytes.extend_from_slice(&self.length.to_be_bytes());

        bytes
    }

    /// 从字节解析头部
    pub fn from_bytes(data: &[u8]) -> Result<Self, IkeError> {
        if data.len() < IKE_HEADER_LEN {
            return Err(IkeError::InvalidLength);
        }

        let initiator_spi = {
            let mut spi = [0u8; SPI_LEN];
            spi.copy_from_slice(&data[0..8]);
            spi
        };

        let responder_spi = {
            let mut spi = [0u8; SPI_LEN];
            spi.copy_from_slice(&data[8..16]);
            spi
        };

        let next_payload = data[16];
        let version = data[17];
        let exchange_type = IkeExchangeType::from_u8(data[18])
            .ok_or_else(|| IkeError::UnsupportedExchangeType(data[18]))?;
        let flags = IkeFlags::from_u8(data[19]);
        let message_id = u32::from_be_bytes([data[20], data[21], data[22], data[23]]);
        let length = u32::from_be_bytes([data[24], data[25], data[26], data[27]]);

        Ok(Self {
            initiator_spi,
            responder_spi,
            next_payload,
            version,
            exchange_type,
            flags,
            message_id,
            length,
        })
    }
}

// ========== IKE 消息 ==========

/// IKEv2 消息
#[derive(Debug, Clone, PartialEq)]
pub struct IkeMessage {
    /// 消息头部
    pub header: IkeHeader,
    /// Payload 列表
    pub payloads: Vec<IkePayload>,
}

impl IkeMessage {
    /// 创建新的 IKE 消息
    pub fn new(header: IkeHeader, payloads: Vec<IkePayload>) -> Self {
        Self { header, payloads }
    }

    /// 创建 IKE_SA_INIT 请求消息
    pub fn init_request(
        initiator_spi: [u8; SPI_LEN],
        payloads: Vec<IkePayload>,
    ) -> Self {
        let mut header = IkeHeader::init_request(initiator_spi);
        if !payloads.is_empty() {
            header.next_payload = payloads[0].get_type().as_u8();
        }
        Self { header, payloads }
    }

    /// 序列化消息为字节
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = self.header.to_bytes();

        // 序列化所有 Payloads
        for payload in &self.payloads {
            bytes.extend_from_slice(&payload.to_bytes());
        }

        // 更新消息长度
        let total_length = bytes.len() as u32;
        bytes[24..28].copy_from_slice(&total_length.to_be_bytes());

        bytes
    }

    /// 从字节解析消息
    pub fn from_bytes(data: &[u8]) -> Result<Self, IkeError> {
        // 解析头部
        let header = IkeHeader::from_bytes(data)?;

        // 检查消息长度
        if data.len() < header.length as usize {
            return Err(IkeError::InvalidLength);
        }

        // 解析 Payloads
        let mut payloads = Vec::new();
        let mut offset = IKE_HEADER_LEN;
        let mut next_payload = IkePayloadType::from_u8(header.next_payload);

        while offset < header.length as usize {
            let payload_type = match next_payload {
                Some(t) if t != IkePayloadType::None => t,
                _ => break,
            };

            // 解析 Payload 头部
            if offset + 4 > data.len() {
                break;
            }

            let payload_next = data[offset];
            let payload_length = u16::from_be_bytes([data[offset + 2], data[offset + 3]]) as usize;

            if offset + payload_length > data.len() {
                return Err(IkeError::InvalidLength);
            }

            // 解析 Payload
            let payload_data = &data[offset..offset + payload_length];
            let payload = IkePayload::from_bytes(payload_data, payload_type)?;

            payloads.push(payload);

            next_payload = IkePayloadType::from_u8(payload_next);
            offset += payload_length;
        }

        Ok(Self { header, payloads })
    }

    /// 获取指定类型的 Payload
    pub fn get_payload(&self, payload_type: IkePayloadType) -> Option<&IkePayload> {
        self.payloads.iter()
            .find(|p| p.get_type() == payload_type)
    }

    /// 获取所有指定类型的 Payloads
    pub fn get_payloads(&self, payload_type: IkePayloadType) -> Vec<&IkePayload> {
        self.payloads.iter()
            .filter(|p| p.get_type() == payload_type)
            .collect()
    }

    /// 添加 Payload
    pub fn add_payload(&mut self, payload: IkePayload) {
        self.payloads.push(payload);
    }

    /// 获取消息 ID
    pub fn message_id(&self) -> u32 {
        self.header.message_id
    }

    /// 获取交换类型
    pub fn exchange_type(&self) -> IkeExchangeType {
        self.header.exchange_type
    }

    /// 是否为响应消息
    pub fn is_response(&self) -> bool {
        self.header.is_response()
    }
}

impl From<CoreError> for IkeError {
    fn from(err: CoreError) -> Self {
        match err {
            CoreError::ParseError(msg) => IkeError::ParseError(msg),
            CoreError::InvalidPacket(msg) => IkeError::ParseError(msg),
            _ => IkeError::Other(format!("{:?}", err)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_header_to_bytes() {
        let header = IkeHeader::new(
            [0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08],
            [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
            IkePayloadType::SA,
            IkeExchangeType::IkeSaInit,
            IkeFlags::request(true),
            0,
        );

        let bytes = header.to_bytes();
        assert_eq!(bytes.len(), IKE_HEADER_LEN);
        assert_eq!(bytes[0..8], header.initiator_spi);
        assert_eq!(bytes[8..16], header.responder_spi);
        assert_eq!(bytes[16], IkePayloadType::SA.as_u8());
        assert_eq!(bytes[17], IKEV2_VERSION);
        assert_eq!(bytes[18], IkeExchangeType::IkeSaInit.as_u8());
        assert!(bytes[19] & 0x08 != 0); // Initiator flag
        assert_eq!(u32::from_be_bytes([bytes[20], bytes[21], bytes[22], bytes[23]]), 0);
    }

    #[test]
    fn test_header_from_bytes() {
        let mut data = vec![0u8; IKE_HEADER_LEN];
        data[0..8].copy_from_slice(&[0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08]);
        data[8..16].copy_from_slice(&[0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]);
        data[16] = IkePayloadType::SA.as_u8();
        data[17] = IKEV2_VERSION;
        data[18] = IkeExchangeType::IkeSaInit.as_u8();
        data[19] = 0x08; // Initiator flag

        let header = IkeHeader::from_bytes(&data).unwrap();
        assert_eq!(header.initiator_spi, [0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08]);
        assert_eq!(header.responder_spi, [0u8; 8]);
        assert_eq!(header.next_payload, IkePayloadType::SA.as_u8());
        assert_eq!(header.version, IKEV2_VERSION);
        assert_eq!(header.exchange_type, IkeExchangeType::IkeSaInit);
        assert!(header.flags.initiator);
        assert!(!header.flags.response);
    }

    #[test]
    fn test_header_invalid_length() {
        let data = vec![0u8; 10];
        let result = IkeHeader::from_bytes(&data);
        assert!(matches!(result, Err(IkeError::InvalidLength)));
    }

    #[test]
    fn test_init_request_header() {
        let spi = [0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08];
        let header = IkeHeader::init_request(spi);

        assert_eq!(header.initiator_spi, spi);
        assert_eq!(header.responder_spi, [0u8; 8]);
        assert_eq!(header.exchange_type, IkeExchangeType::IkeSaInit);
        assert!(header.flags.initiator);
        assert!(!header.flags.response);
        assert_eq!(header.message_id, 0);
    }

    #[test]
    fn test_response_header() {
        let request_header = IkeHeader::init_request([0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08]);
        let response_header = request_header.response(IkePayloadType::SA, 100);

        assert_eq!(response_header.initiator_spi, request_header.initiator_spi);
        assert_eq!(response_header.responder_spi, request_header.responder_spi);
        assert_eq!(response_header.exchange_type, request_header.exchange_type);
        assert!(response_header.flags.response);
        assert_eq!(response_header.length, 100);
    }
}
