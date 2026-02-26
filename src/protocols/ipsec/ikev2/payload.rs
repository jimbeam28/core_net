// src/protocols/ipsec/ikev2/payload.rs
//
// IKEv2 Payload 结构定义和解析

use super::*;
use std::net::{Ipv4Addr, Ipv6Addr};

// ========== Payload 通用头部 ==========

/// Payload 通用头部
///
/// RFC 7296 Section 2.5: 文档格式略
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IkePayloadHeader {
    /// 下一个 Payload 类型
    pub next_payload: u8,
    /// Critical 标志位（bit 7）
    pub critical: bool,
    /// Payload 长度
    pub length: u16,
}

impl IkePayloadHeader {
    /// 创建新的 Payload 头部
    pub fn new(next_payload: IkePayloadType, critical: bool, length: u16) -> Self {
        Self {
            next_payload: next_payload.as_u8(),
            critical,
            length,
        }
    }

    /// 序列化头部为字节
    pub fn to_bytes(&self) -> [u8; 4] {
        let critical_byte = if self.critical { 0x80 } else { 0x00 };
        [
            self.next_payload,
            critical_byte,
            (self.length >> 8) as u8,
            (self.length & 0xFF) as u8,
        ]
    }

    /// 从字节解析头部
    pub fn from_bytes(data: &[u8]) -> Result<Self, IkeError> {
        if data.len() < 4 {
            return Err(IkeError::InvalidLength);
        }

        Ok(Self {
            next_payload: data[0],
            critical: (data[1] & 0x80) != 0,
            length: u16::from_be_bytes([data[2], data[3]]),
        })
    }
}

// ========== Payload 枚举 ==========

/// IKEv2 Payload
#[derive(Debug, Clone, PartialEq)]
pub enum IkePayload {
    /// Security Association
    SA(IkeSaPayload),
    /// Key Exchange (DH)
    KE(IkeKePayload),
    /// Identification - Initiator
    IDi(IkeIdPayload),
    /// Identification - Responder
    IDr(IkeIdPayload),
    /// Certificate
    CERT(IkeCertPayload),
    /// Certificate Request
    CERTREQ(IkeCertReqPayload),
    /// Authentication
    AUTH(IkeAuthPayload),
    /// Nonce
    Nonce(IkeNoncePayload),
    /// Notification
    Notify(IkeNotifyPayload),
    /// Delete
    Delete(IkeDeletePayload),
    /// Vendor ID
    Vendor(IkeVendorPayload),
    /// Traffic Selector - Initiator
    TSi(IkeTsPayload),
    /// Traffic Selector - Responder
    TSr(IkeTsPayload),
}

impl IkePayload {
    /// 获取 Payload 类型
    pub fn get_type(&self) -> IkePayloadType {
        match self {
            Self::SA(_) => IkePayloadType::SA,
            Self::KE(_) => IkePayloadType::KE,
            Self::IDi(_) => IkePayloadType::IDi,
            Self::IDr(_) => IkePayloadType::IDr,
            Self::CERT(_) => IkePayloadType::CERT,
            Self::CERTREQ(_) => IkePayloadType::CERTREQ,
            Self::AUTH(_) => IkePayloadType::AUTH,
            Self::Nonce(_) => IkePayloadType::Nonce,
            Self::Notify(_) => IkePayloadType::Notify,
            Self::Delete(_) => IkePayloadType::Delete,
            Self::Vendor(_) => IkePayloadType::Vendor,
            Self::TSi(_) => IkePayloadType::TSi,
            Self::TSr(_) => IkePayloadType::TSr,
        }
    }

    /// 序列化 Payload 为字节
    pub fn to_bytes(&self) -> Vec<u8> {
        match self {
            Self::SA(p) => p.to_bytes(),
            Self::KE(p) => p.to_bytes(),
            Self::IDi(p) => p.to_bytes(),
            Self::IDr(p) => p.to_bytes(),
            Self::CERT(p) => p.to_bytes(),
            Self::CERTREQ(p) => p.to_bytes(),
            Self::AUTH(p) => p.to_bytes(),
            Self::Nonce(p) => p.to_bytes(),
            Self::Notify(p) => p.to_bytes(),
            Self::Delete(p) => p.to_bytes(),
            Self::Vendor(p) => p.to_bytes(),
            Self::TSi(p) => p.to_bytes(),
            Self::TSr(p) => p.to_bytes(),
        }
    }

    /// 从字节解析 Payload
    pub fn from_bytes(data: &[u8], payload_type: IkePayloadType) -> Result<Self, IkeError> {
        match payload_type {
            IkePayloadType::SA => Ok(Self::SA(IkeSaPayload::from_bytes(data)?)),
            IkePayloadType::KE => Ok(Self::KE(IkeKePayload::from_bytes(data)?)),
            IkePayloadType::IDi => Ok(Self::IDi(IkeIdPayload::from_bytes(data)?)),
            IkePayloadType::IDr => Ok(Self::IDr(IkeIdPayload::from_bytes(data)?)),
            IkePayloadType::CERT => Ok(Self::CERT(IkeCertPayload::from_bytes(data)?)),
            IkePayloadType::CERTREQ => Ok(Self::CERTREQ(IkeCertReqPayload::from_bytes(data)?)),
            IkePayloadType::AUTH => Ok(Self::AUTH(IkeAuthPayload::from_bytes(data)?)),
            IkePayloadType::Nonce => Ok(Self::Nonce(IkeNoncePayload::from_bytes(data)?)),
            IkePayloadType::Notify => Ok(Self::Notify(IkeNotifyPayload::from_bytes(data)?)),
            IkePayloadType::Delete => Ok(Self::Delete(IkeDeletePayload::from_bytes(data)?)),
            IkePayloadType::Vendor => Ok(Self::Vendor(IkeVendorPayload::from_bytes(data)?)),
            IkePayloadType::TSi => Ok(Self::TSi(IkeTsPayload::from_bytes(data)?)),
            IkePayloadType::TSr => Ok(Self::TSr(IkeTsPayload::from_bytes(data)?)),
            _ => Err(IkeError::UnsupportedPayloadType(payload_type.as_u8())),
        }
    }
}

// ========== SA Payload ==========

/// SA Payload
#[derive(Debug, Clone, PartialEq)]
pub struct IkeSaPayload {
    /// 下一个 Payload
    pub next_payload: IkePayloadType,
    /// Critical 标志
    pub critical: bool,
    /// 协商提议列表
    pub proposals: Vec<IkeProposal>,
}

impl IkeSaPayload {
    /// 序列化为字节
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();

        // 计算总长度
        let total_length = 4 + self.proposals.iter()
            .map(|p| p.to_bytes_len())
            .sum::<usize>();

        let header = IkePayloadHeader::new(self.next_payload, self.critical, total_length as u16);
        bytes.extend_from_slice(&header.to_bytes());
        bytes.extend_from_slice(&[0, 0]); // DOI (2 bytes) - 仅用于 IKEv1

        for proposal in &self.proposals {
            bytes.extend_from_slice(&proposal.to_bytes());
        }

        bytes
    }

    /// 从字节解析
    pub fn from_bytes(data: &[u8]) -> Result<Self, IkeError> {
        if data.len() < 4 {
            return Err(IkeError::InvalidLength);
        }

        let header = IkePayloadHeader::from_bytes(data)?;
        let next_payload = IkePayloadType::from_u8(header.next_payload)
            .unwrap_or(IkePayloadType::None);
        let critical = header.critical;

        // 跳过头部和 DOI
        let mut offset = 6;
        let mut proposals = Vec::new();

        while offset < data.len() {
            let proposal = IkeProposal::from_bytes(&data[offset..])?;
            offset += proposal.to_bytes_len();
            proposals.push(proposal);

            if offset >= data.len() {
                break;
            }
        }

        Ok(Self {
            next_payload,
            critical,
            proposals,
        })
    }
}

/// SA Payload 中的提议
#[derive(Debug, Clone, PartialEq)]
pub struct IkeProposal {
    /// 是否为最后一个提议
    pub is_last: bool,
    /// 提议编号
    pub proposal_num: u8,
    /// 协议 ID
    pub protocol_id: IkeProtocolId,
    /// SPI 大小
    pub spi_size: u8,
    /// Transform 数量
    pub num_transforms: u8,
    /// SPI 值
    pub spi: Vec<u8>,
    /// Transform 列表
    pub transforms: Vec<IkeTransform>,
}

impl IkeProposal {
    /// 计算字节长度
    pub fn to_bytes_len(&self) -> usize {
        8 + self.spi.len() + self.transforms.iter()
            .map(|t| t.to_bytes_len())
            .sum::<usize>()
    }

    /// 序列化为字节
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();

        let last_byte = if self.is_last { 0 } else { 2 }; // 0 表示最后一个
        bytes.push(last_byte);
        bytes.push(self.proposal_num);
        bytes.push(self.protocol_id.as_u8());
        bytes.push(self.spi_size);
        bytes.push(self.transforms.len() as u8);
        bytes.extend_from_slice(&[0, 0]); // 保留
        bytes.extend_from_slice(&self.spi);

        for transform in &self.transforms {
            bytes.extend_from_slice(&transform.to_bytes());
        }

        bytes
    }

    /// 从字节解析
    pub fn from_bytes(data: &[u8]) -> Result<Self, IkeError> {
        if data.len() < 8 {
            return Err(IkeError::InvalidLength);
        }

        let is_last = data[0] == 0;
        let proposal_num = data[1];
        let protocol_id = IkeProtocolId::from_u8(data[2])
            .ok_or_else(|| IkeError::ParseError("无效的协议 ID".to_string()))?;
        let spi_size = data[3] as usize;
        let num_transforms = data[4] as usize;

        let spi_start = 8;
        let spi_end = spi_start + spi_size;
        if spi_end > data.len() {
            return Err(IkeError::InvalidLength);
        }

        let spi = data[spi_start..spi_end].to_vec();

        // 解析 Transforms
        let mut transforms = Vec::new();
        let mut offset = spi_end;

        for _ in 0..num_transforms {
            if offset >= data.len() {
                break;
            }
            let transform = IkeTransform::from_bytes(&data[offset..])?;
            offset += transform.to_bytes_len();
            transforms.push(transform);
        }

        Ok(Self {
            is_last,
            proposal_num,
            protocol_id,
            spi_size: spi_size as u8,
            num_transforms: num_transforms as u8,
            spi,
            transforms,
        })
    }
}

/// Transform 子结构
#[derive(Debug, Clone, PartialEq)]
pub struct IkeTransform {
    /// 是否为最后一个 Transform
    pub is_last: bool,
    /// Transform 类型
    pub transform_type: IkeTransformType,
    /// Transform ID
    pub transform_id: u16,
    /// Transform 属性
    pub attributes: Vec<IkeTransformAttribute>,
}

impl IkeTransform {
    /// 计算字节长度
    pub fn to_bytes_len(&self) -> usize {
        8 + self.attributes.iter()
            .map(|a| a.to_bytes_len())
            .sum::<usize>()
    }

    /// 序列化为字节
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();

        let last_byte = if self.is_last { 0 } else { 3 }; // 0 表示最后一个
        bytes.push(last_byte);
        bytes.push(0); // 保留
        bytes.push(self.transform_type.as_u8());
        bytes.push(0); // 保留
        bytes.extend_from_slice(&self.transform_id.to_be_bytes());

        for attribute in &self.attributes {
            bytes.extend_from_slice(&attribute.to_bytes());
        }

        bytes
    }

    /// 从字节解析
    pub fn from_bytes(data: &[u8]) -> Result<Self, IkeError> {
        if data.len() < 8 {
            return Err(IkeError::InvalidLength);
        }

        let is_last = data[0] == 0;
        let transform_type = IkeTransformType::from_u8(data[2])
            .ok_or_else(|| IkeError::ParseError("无效的 Transform 类型".to_string()))?;
        let transform_id = u16::from_be_bytes([data[4], data[5]]);

        // 解析属性
        let mut attributes = Vec::new();
        let mut offset = 8;

        while offset < data.len() {
            if offset + 4 > data.len() {
                break;
            }

            let attr_type = u16::from_be_bytes([data[offset], data[offset + 1]]);
            let is_format = (attr_type & 0x8000) != 0;
            let attr_value_len = if is_format { 2 } else { 4 };

            if offset + 4 + attr_value_len > data.len() {
                break;
            }

            let attribute = IkeTransformAttribute {
                is_format,
                attr_type: attr_type & 0x7FFF,
                value: data[offset + 4..offset + 4 + attr_value_len].to_vec(),
            };

            attributes.push(attribute);
            offset += 4 + attr_value_len;

            // 如果不是最后一个属性，继续
            if !is_format {
                break;
            }
        }

        Ok(Self {
            is_last,
            transform_type,
            transform_id,
            attributes,
        })
    }
}

/// Transform 属性
#[derive(Debug, Clone, PartialEq)]
pub struct IkeTransformAttribute {
    /// 是否使用 AF 格式（16 位）
    pub is_format: bool,
    /// 属性类型
    pub attr_type: u16,
    /// 属性值
    pub value: Vec<u8>,
}

impl IkeTransformAttribute {
    /// 计算字节长度
    pub fn to_bytes_len(&self) -> usize {
        if self.is_format {
            4 + self.value.len()
        } else {
            4
        }
    }

    /// 序列化为字节
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();

        let attr_type = if self.is_format {
            self.attr_type | 0x8000
        } else {
            self.attr_type
        };

        bytes.extend_from_slice(&attr_type.to_be_bytes());

        if self.is_format {
            // AF 格式
            if self.value.len() >= 2 {
                bytes.extend_from_slice(&self.value[0..2]);
            } else {
                bytes.extend_from_slice(&[0, 0]);
            }
        } else {
            // TLV 格式
            let length = self.value.len() as u16;
            bytes.extend_from_slice(&length.to_be_bytes());
            bytes.extend_from_slice(&self.value);
        }

        bytes
    }
}

// ========== KE Payload ==========

/// KE Payload
#[derive(Debug, Clone, PartialEq)]
pub struct IkeKePayload {
    /// 下一个 Payload
    pub next_payload: IkePayloadType,
    /// Critical 标志
    pub critical: bool,
    /// DH 组
    pub dh_group: IkeDhGroup,
    /// 公钥数据
    pub public_key: Vec<u8>,
}

impl IkeKePayload {
    /// 创建新的 KE Payload
    pub fn new(next_payload: IkePayloadType, dh_group: IkeDhGroup, public_key: Vec<u8>) -> Self {
        Self {
            next_payload,
            critical: false,
            dh_group,
            public_key,
        }
    }

    /// 序列化为字节
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();

        let total_length = 4 + self.public_key.len();
        let header = IkePayloadHeader::new(self.next_payload, self.critical, total_length as u16);
        bytes.extend_from_slice(&header.to_bytes());
        bytes.extend_from_slice(&self.dh_group.as_u16().to_be_bytes());
        bytes.extend_from_slice(&[0, 0]); // 保留
        bytes.extend_from_slice(&self.public_key);

        bytes
    }

    /// 从字节解析
    pub fn from_bytes(data: &[u8]) -> Result<Self, IkeError> {
        if data.len() < 8 {
            return Err(IkeError::InvalidLength);
        }

        let header = IkePayloadHeader::from_bytes(data)?;
        let next_payload = IkePayloadType::from_u8(header.next_payload)
            .unwrap_or(IkePayloadType::None);
        let critical = header.critical;

        let dh_group = IkeDhGroup::from_u16(u16::from_be_bytes([data[4], data[5]]))
            .ok_or_else(|| IkeError::ParseError("无效的 DH 组".to_string()))?;

        let public_key = data[8..].to_vec();

        Ok(Self {
            next_payload,
            critical,
            dh_group,
            public_key,
        })
    }
}

// ========== ID Payload ==========

/// ID Payload
#[derive(Debug, Clone, PartialEq)]
pub struct IkeIdPayload {
    /// 下一个 Payload
    pub next_payload: IkePayloadType,
    /// Critical 标志
    pub critical: bool,
    /// ID 类型
    pub id_type: IkeIdType,
    /// 协议 ID
    pub protocol_id: Option<u8>,
    /// 端口
    pub port: Option<u16>,
    /// 身份数据
    pub id_data: Vec<u8>,
}

impl IkeIdPayload {
    /// 创建新的 ID Payload
    pub fn new(next_payload: IkePayloadType, id_type: IkeIdType, id_data: Vec<u8>) -> Self {
        Self {
            next_payload,
            critical: false,
            id_type,
            protocol_id: None,
            port: None,
            id_data,
        }
    }

    /// 序列化为字节
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();

        let total_length = 8 + self.id_data.len();
        let header = IkePayloadHeader::new(self.next_payload, self.critical, total_length as u16);
        bytes.extend_from_slice(&header.to_bytes());
        bytes.push(self.id_type.as_u8());

        let protocol_port = match (self.protocol_id, self.port) {
            (Some(proto), Some(port)) => ((proto as u16) << 8) | port,
            (Some(proto), None) => (proto as u16) << 8,
            (None, _) => 0,
        };
        bytes.extend_from_slice(&protocol_port.to_be_bytes());

        bytes.extend_from_slice(&[0, 0, 0]); // 保留
        bytes.extend_from_slice(&self.id_data);

        bytes
    }

    /// 从字节解析
    pub fn from_bytes(data: &[u8]) -> Result<Self, IkeError> {
        if data.len() < 8 {
            return Err(IkeError::InvalidLength);
        }

        let header = IkePayloadHeader::from_bytes(data)?;
        let next_payload = IkePayloadType::from_u8(header.next_payload)
            .unwrap_or(IkePayloadType::None);
        let critical = header.critical;

        let id_type = IkeIdType::from_u8(data[4])
            .ok_or_else(|| IkeError::ParseError("无效的 ID 类型".to_string()))?;

        let protocol_port = u16::from_be_bytes([data[5], data[6]]);
        let protocol_id = if protocol_port & 0xFF00 != 0 {
            Some((protocol_port >> 8) as u8)
        } else {
            None
        };
        let port = if protocol_port & 0x00FF != 0 {
            Some(protocol_port & 0x00FF)
        } else {
            None
        };

        let id_data = data[8..].to_vec();

        Ok(Self {
            next_payload,
            critical,
            id_type,
            protocol_id,
            port,
            id_data,
        })
    }
}

// ========== CERT Payload（简化） ==========

#[derive(Debug, Clone, PartialEq)]
pub struct IkeCertPayload {
    pub next_payload: IkePayloadType,
    pub critical: bool,
    pub cert_data: Vec<u8>,
}

impl IkeCertPayload {
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        let total_length = 5 + self.cert_data.len();
        let header = IkePayloadHeader::new(self.next_payload, self.critical, total_length as u16);
        bytes.extend_from_slice(&header.to_bytes());
        bytes.push(4); // ASN.1 DER 编码
        bytes.extend_from_slice(&self.cert_data);
        bytes
    }

    pub fn from_bytes(data: &[u8]) -> Result<Self, IkeError> {
        if data.len() < 5 {
            return Err(IkeError::InvalidLength);
        }
        let header = IkePayloadHeader::from_bytes(data)?;
        let next_payload = IkePayloadType::from_u8(header.next_payload).unwrap_or(IkePayloadType::None);
        let critical = header.critical;
        let cert_data = data[5..].to_vec();
        Ok(Self { next_payload, critical, cert_data })
    }
}

// ========== CERTREQ Payload（简化） ==========

#[derive(Debug, Clone, PartialEq)]
pub struct IkeCertReqPayload {
    pub next_payload: IkePayloadType,
    pub critical: bool,
    pub cert_type: u8,
    pub ca_data: Vec<u8>,
}

impl IkeCertReqPayload {
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        let total_length = 5 + self.ca_data.len();
        let header = IkePayloadHeader::new(self.next_payload, self.critical, total_length as u16);
        bytes.extend_from_slice(&header.to_bytes());
        bytes.push(self.cert_type);
        bytes.extend_from_slice(&self.ca_data);
        bytes
    }

    pub fn from_bytes(data: &[u8]) -> Result<Self, IkeError> {
        if data.len() < 5 {
            return Err(IkeError::InvalidLength);
        }
        let header = IkePayloadHeader::from_bytes(data)?;
        let next_payload = IkePayloadType::from_u8(header.next_payload).unwrap_or(IkePayloadType::None);
        let critical = header.critical;
        let cert_type = data[4];
        let ca_data = data[5..].to_vec();
        Ok(Self { next_payload, critical, cert_type, ca_data })
    }
}

// ========== AUTH Payload ==========

/// AUTH Payload
#[derive(Debug, Clone, PartialEq)]
pub struct IkeAuthPayload {
    /// 下一个 Payload
    pub next_payload: IkePayloadType,
    /// Critical 标志
    pub critical: bool,
    /// 认证方法
    pub auth_method: IkeAuthMethod,
    /// 认证数据
    pub auth_data: Vec<u8>,
}

impl IkeAuthPayload {
    /// 创建新的 AUTH Payload
    pub fn new(next_payload: IkePayloadType, auth_method: IkeAuthMethod, auth_data: Vec<u8>) -> Self {
        Self {
            next_payload,
            critical: false,
            auth_method,
            auth_data,
        }
    }

    /// 序列化为字节
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();

        let total_length = 5 + self.auth_data.len();
        let header = IkePayloadHeader::new(self.next_payload, self.critical, total_length as u16);
        bytes.extend_from_slice(&header.to_bytes());
        bytes.push(self.auth_method.as_u8());
        bytes.extend_from_slice(&[0, 0, 0]); // 保留
        bytes.extend_from_slice(&self.auth_data);

        bytes
    }

    /// 从字节解析
    pub fn from_bytes(data: &[u8]) -> Result<Self, IkeError> {
        if data.len() < 8 {
            return Err(IkeError::InvalidLength);
        }

        let header = IkePayloadHeader::from_bytes(data)?;
        let next_payload = IkePayloadType::from_u8(header.next_payload)
            .unwrap_or(IkePayloadType::None);
        let critical = header.critical;

        let auth_method = IkeAuthMethod::from_u8(data[4])
            .ok_or_else(|| IkeError::ParseError("无效的认证方法".to_string()))?;

        let auth_data = data[8..].to_vec();

        Ok(Self {
            next_payload,
            critical,
            auth_method,
            auth_data,
        })
    }
}

// ========== Nonce Payload ==========

/// Nonce Payload
#[derive(Debug, Clone, PartialEq)]
pub struct IkeNoncePayload {
    /// 下一个 Payload
    pub next_payload: IkePayloadType,
    /// Critical 标志
    pub critical: bool,
    /// Nonce 数据
    pub nonce_data: Vec<u8>,
}

impl IkeNoncePayload {
    /// 创建新的 Nonce Payload
    pub fn new(next_payload: IkePayloadType, nonce_data: Vec<u8>) -> Self {
        Self {
            next_payload,
            critical: false,
            nonce_data,
        }
    }

    /// 序列化为字节
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();

        let total_length = 4 + self.nonce_data.len();
        let header = IkePayloadHeader::new(self.next_payload, self.critical, total_length as u16);
        bytes.extend_from_slice(&header.to_bytes());
        bytes.extend_from_slice(&self.nonce_data);

        bytes
    }

    /// 从字节解析
    pub fn from_bytes(data: &[u8]) -> Result<Self, IkeError> {
        if data.len() < 4 {
            return Err(IkeError::InvalidLength);
        }

        let header = IkePayloadHeader::from_bytes(data)?;
        let next_payload = IkePayloadType::from_u8(header.next_payload)
            .unwrap_or(IkePayloadType::None);
        let critical = header.critical;

        let nonce_data = data[4..].to_vec();

        Ok(Self {
            next_payload,
            critical,
            nonce_data,
        })
    }
}

// ========== Notify Payload ==========

/// Notify Payload
#[derive(Debug, Clone, PartialEq)]
pub struct IkeNotifyPayload {
    /// 下一个 Payload
    pub next_payload: IkePayloadType,
    /// Critical 标志
    pub critical: bool,
    /// 协议 ID
    pub protocol_id: u8,
    /// SPI 大小
    pub spi_size: u8,
    /// 通知消息类型
    pub notify_type: u16,
    /// SPI
    pub spi: Vec<u8>,
    /// 通知数据
    pub notify_data: Vec<u8>,
}

impl IkeNotifyPayload {
    /// 序列化为字节
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();

        let total_length = 10 + self.spi.len() + self.notify_data.len();
        let header = IkePayloadHeader::new(self.next_payload, self.critical, total_length as u16);
        bytes.extend_from_slice(&header.to_bytes());
        bytes.push(self.protocol_id);
        bytes.push(self.spi_size);
        bytes.extend_from_slice(&self.notify_type.to_be_bytes());
        bytes.extend_from_slice(&self.spi);
        bytes.extend_from_slice(&self.notify_data);

        bytes
    }

    pub fn from_bytes(data: &[u8]) -> Result<Self, IkeError> {
        if data.len() < 10 {
            return Err(IkeError::InvalidLength);
        }
        let header = IkePayloadHeader::from_bytes(data)?;
        let next_payload = IkePayloadType::from_u8(header.next_payload).unwrap_or(IkePayloadType::None);
        let critical = header.critical;
        let protocol_id = data[4];
        let spi_size = data[5] as usize;
        let notify_type = u16::from_be_bytes([data[6], data[7]]);
        let spi_end = 10 + spi_size;
        if spi_end > data.len() {
            return Err(IkeError::InvalidLength);
        }
        let spi = data[10..spi_end].to_vec();
        let notify_data = data[spi_end..].to_vec();
        Ok(Self { next_payload, critical, protocol_id, spi_size: spi_size as u8, notify_type, spi, notify_data })
    }
}

// ========== Delete Payload ==========

#[derive(Debug, Clone, PartialEq)]
pub struct IkeDeletePayload {
    pub next_payload: IkePayloadType,
    pub critical: bool,
    pub protocol_id: u8,
    pub spi_size: u8,
    pub num_spi: u16,
    pub spis: Vec<Vec<u8>>,
}

impl IkeDeletePayload {
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        let total_length = 10 + self.spis.iter().map(|s| s.len()).sum::<usize>();
        let header = IkePayloadHeader::new(self.next_payload, self.critical, total_length as u16);
        bytes.extend_from_slice(&header.to_bytes());
        bytes.push(self.protocol_id);
        bytes.push(self.spi_size);
        bytes.extend_from_slice(&self.num_spi.to_be_bytes());
        for spi in &self.spis {
            bytes.extend_from_slice(spi);
        }
        bytes
    }

    pub fn from_bytes(data: &[u8]) -> Result<Self, IkeError> {
        if data.len() < 10 {
            return Err(IkeError::InvalidLength);
        }
        let header = IkePayloadHeader::from_bytes(data)?;
        let next_payload = IkePayloadType::from_u8(header.next_payload).unwrap_or(IkePayloadType::None);
        let critical = header.critical;
        let protocol_id = data[4];
        let spi_size = data[5] as usize;
        let num_spi = u16::from_be_bytes([data[6], data[7]]) as usize;
        let mut spis = Vec::new();
        let mut offset = 10;
        for _ in 0..num_spi {
            if offset + spi_size > data.len() {
                return Err(IkeError::InvalidLength);
            }
            spis.push(data[offset..offset + spi_size].to_vec());
            offset += spi_size;
        }
        Ok(Self { next_payload, critical, protocol_id, spi_size: spi_size as u8, num_spi: num_spi as u16, spis })
    }
}

// ========== Vendor Payload ==========

#[derive(Debug, Clone, PartialEq)]
pub struct IkeVendorPayload {
    pub next_payload: IkePayloadType,
    pub critical: bool,
    pub vendor_id: Vec<u8>,
}

impl IkeVendorPayload {
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        let total_length = 4 + self.vendor_id.len();
        let header = IkePayloadHeader::new(self.next_payload, self.critical, total_length as u16);
        bytes.extend_from_slice(&header.to_bytes());
        bytes.extend_from_slice(&self.vendor_id);
        bytes
    }

    pub fn from_bytes(data: &[u8]) -> Result<Self, IkeError> {
        if data.len() < 4 {
            return Err(IkeError::InvalidLength);
        }
        let header = IkePayloadHeader::from_bytes(data)?;
        let next_payload = IkePayloadType::from_u8(header.next_payload).unwrap_or(IkePayloadType::None);
        let critical = header.critical;
        let vendor_id = data[4..].to_vec();
        Ok(Self { next_payload, critical, vendor_id })
    }
}

// ========== Traffic Selector ==========

/// TS 类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum TsType {
    /// IPv4
    Ipv4 = 7,
    /// IPv6
    Ipv6 = 8,
}

impl TsType {
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            7 => Some(Self::Ipv4),
            8 => Some(Self::Ipv6),
            _ => None,
        }
    }

    pub fn as_u8(self) -> u8 {
        self as u8
    }
}

/// 流量选择器
#[derive(Debug, Clone, PartialEq)]
pub struct TrafficSelector {
    /// TS 类型
    pub ts_type: TsType,
    /// IP 协议 ID
    pub protocol_id: u8,
    /// 起始端口
    pub start_port: u16,
    /// 结束端口
    pub end_port: u16,
    /// 起始地址
    pub start_addr: Vec<u8>,
    /// 结束地址
    pub end_addr: Vec<u8>,
}

impl TrafficSelector {
    /// 创建 IPv4 流量选择器
    pub fn ipv4(
        protocol_id: u8,
        start_port: u16,
        end_port: u16,
        start_addr: Ipv4Addr,
        end_addr: Ipv4Addr,
    ) -> Self {
        Self {
            ts_type: TsType::Ipv4,
            protocol_id,
            start_port,
            end_port,
            start_addr: start_addr.octets().to_vec(),
            end_addr: end_addr.octets().to_vec(),
        }
    }

    /// 创建 IPv6 流量选择器
    pub fn ipv6(
        protocol_id: u8,
        start_port: u16,
        end_port: u16,
        start_addr: Ipv6Addr,
        end_addr: Ipv6Addr,
    ) -> Self {
        Self {
            ts_type: TsType::Ipv6,
            protocol_id,
            start_port,
            end_port,
            start_addr: start_addr.octets().to_vec(),
            end_addr: end_addr.octets().to_vec(),
        }
    }

    /// 计算字节长度
    pub fn bytes_len(&self) -> usize {
        8 + self.start_addr.len() + self.end_addr.len()
    }

    /// 序列化为字节
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();

        bytes.push(self.ts_type.as_u8());
        bytes.push(self.protocol_id);
        bytes.extend_from_slice(&self.start_port.to_be_bytes());
        bytes.extend_from_slice(&self.end_port.to_be_bytes());
        bytes.extend_from_slice(&(self.start_addr.len() as u16).to_be_bytes());
        bytes.extend_from_slice(&self.start_addr);
        bytes.extend_from_slice(&self.end_addr);

        bytes
    }

    /// 从字节解析
    pub fn from_bytes(data: &[u8]) -> Result<Self, IkeError> {
        if data.len() < 8 {
            return Err(IkeError::InvalidLength);
        }

        let ts_type = TsType::from_u8(data[0])
            .ok_or_else(|| IkeError::ParseError("无效的 TS 类型".to_string()))?;
        let protocol_id = data[1];
        let start_port = u16::from_be_bytes([data[2], data[3]]);
        let end_port = u16::from_be_bytes([data[4], data[5]]);
        let addr_len = u16::from_be_bytes([data[6], data[7]]) as usize;

        if data.len() < 8 + addr_len * 2 {
            return Err(IkeError::InvalidLength);
        }

        let start_addr = data[8..8 + addr_len].to_vec();
        let end_addr = data[8 + addr_len..8 + addr_len * 2].to_vec();

        Ok(Self {
            ts_type,
            protocol_id,
            start_port,
            end_port,
            start_addr,
            end_addr,
        })
    }
}

/// TS Payload
#[derive(Debug, Clone, PartialEq)]
pub struct IkeTsPayload {
    /// 下一个 Payload
    pub next_payload: IkePayloadType,
    /// Critical 标志
    pub critical: bool,
    /// 流量选择器数量
    pub num_ts: u8,
    /// 流量选择器列表
    pub traffic_selectors: Vec<TrafficSelector>,
}

impl IkeTsPayload {
    /// 创建新的 TS Payload
    pub fn new(next_payload: IkePayloadType, traffic_selectors: Vec<TrafficSelector>) -> Self {
        let num_ts = traffic_selectors.len() as u8;
        Self {
            next_payload,
            critical: false,
            num_ts,
            traffic_selectors,
        }
    }

    /// 序列化为字节
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();

        let total_length = 8 + self.traffic_selectors.iter()
            .map(|ts| ts.bytes_len())
            .sum::<usize>();

        let header = IkePayloadHeader::new(self.next_payload, self.critical, total_length as u16);
        bytes.extend_from_slice(&header.to_bytes());
        bytes.extend_from_slice(&[0, 0, 0]); // 保留
        bytes.push(self.num_ts);
        bytes.extend_from_slice(&[0, 0, 0]); // 保留

        for ts in &self.traffic_selectors {
            bytes.extend_from_slice(&ts.to_bytes());
        }

        bytes
    }

    /// 从字节解析
    pub fn from_bytes(data: &[u8]) -> Result<Self, IkeError> {
        if data.len() < 8 {
            return Err(IkeError::InvalidLength);
        }

        let header = IkePayloadHeader::from_bytes(data)?;
        let next_payload = IkePayloadType::from_u8(header.next_payload)
            .unwrap_or(IkePayloadType::None);
        let critical = header.critical;

        let num_ts = data[7];

        let mut traffic_selectors = Vec::new();
        let mut offset = 8;

        for _ in 0..num_ts {
            if offset >= data.len() {
                break;
            }
            let ts = TrafficSelector::from_bytes(&data[offset..])?;
            offset += ts.bytes_len();
            traffic_selectors.push(ts);
        }

        Ok(Self {
            next_payload,
            critical,
            num_ts,
            traffic_selectors,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_payload_header() {
        let header = IkePayloadHeader::new(IkePayloadType::SA, false, 100);
        let bytes = header.to_bytes();
        assert_eq!(bytes[0], IkePayloadType::SA.as_u8());
        assert_eq!(bytes[1], 0); // 非关键
        assert_eq!(u16::from_be_bytes([bytes[2], bytes[3]]), 100);
    }

    #[test]
    fn test_ke_payload() {
        let public_key = vec![0x01, 0x02, 0x03, 0x04];
        let payload = IkeKePayload::new(IkePayloadType::None, IkeDhGroup::MODP2048, public_key.clone());

        let bytes = payload.to_bytes();
        assert_eq!(bytes.len(), 4 + 4 + public_key.len()); // 头部 + DH组字段 + 公钥

        let decoded = IkeKePayload::from_bytes(&bytes).unwrap();
        assert_eq!(decoded.dh_group, IkeDhGroup::MODP2048);
        assert_eq!(decoded.public_key, public_key);
    }

    #[test]
    fn test_nonce_payload() {
        let nonce_data = vec![0xAA, 0xBB, 0xCC, 0xDD];
        let payload = IkeNoncePayload::new(IkePayloadType::None, nonce_data.clone());

        let bytes = payload.to_bytes();
        assert_eq!(bytes.len(), 4 + nonce_data.len());

        let decoded = IkeNoncePayload::from_bytes(&bytes).unwrap();
        assert_eq!(decoded.nonce_data, nonce_data);
    }

    #[test]
    fn test_traffic_selector_ipv4() {
        let ts = TrafficSelector::ipv4(
            6,           // TCP
            1000,        // 起始端口
            2000,        // 结束端口
            Ipv4Addr::new(192, 168, 1, 1),
            Ipv4Addr::new(192, 168, 1, 255),
        );

        let bytes = ts.to_bytes();
        assert_eq!(bytes[0], TsType::Ipv4.as_u8());
        assert_eq!(bytes[1], 6);

        let decoded = TrafficSelector::from_bytes(&bytes).unwrap();
        assert_eq!(decoded.ts_type, TsType::Ipv4);
        assert_eq!(decoded.protocol_id, 6);
        assert_eq!(decoded.start_port, 1000);
        assert_eq!(decoded.end_port, 2000);
    }
}
