// src/protocols/icmp/packet.rs
//
// ICMP 报文结构定义

use crate::common::{CoreError, Packet, Result};
use super::types::*;
use crate::protocols::ip::calculate_checksum;

// ========== ICMP 通用头部 ==========

/// ICMP 通用头部（所有 ICMP 消息共有）
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct IcmpHeader {
    /// ICMP 消息类型
    pub type_: u8,

    /// 类型子代码
    pub code: u8,

    /// 校验和
    pub checksum: u16,
}

impl IcmpHeader {
    /// ICMP 头部最小长度
    pub const MIN_LEN: usize = 4;

    /// 从 Packet 解析 ICMP 通用头部
    pub fn from_packet(packet: &mut Packet) -> Result<Self> {
        if packet.remaining() < Self::MIN_LEN {
            return Err(CoreError::invalid_packet(format!(
                "ICMP数据包长度不足：{} < {}",
                packet.remaining(),
                Self::MIN_LEN
            )));
        }

        let type_ = packet.read(1)
            .ok_or_else(|| CoreError::parse_error("读取ICMP类型失败"))?[0];
        let code = packet.read(1)
            .ok_or_else(|| CoreError::parse_error("读取ICMP代码失败"))?[0];
        let checksum_bytes = packet.read(2)
            .ok_or_else(|| CoreError::parse_error("读取ICMP校验和失败"))?;
        let checksum = u16::from_be_bytes([checksum_bytes[0], checksum_bytes[1]]);

        Ok(IcmpHeader { type_, code, checksum })
    }
}

// ========== Echo Request/Reply ==========

/// Echo Request/Reply 报文
#[derive(Debug, Clone, PartialEq)]
pub struct IcmpEcho {
    /// 类型 (0=Reply, 8=Request)
    pub type_: u8,

    /// 代码 (始终为 0)
    pub code: u8,

    /// 校验和
    pub checksum: u16,

    /// 标识符（用于匹配请求和响应）
    pub identifier: u16,

    /// 序列号
    pub sequence: u16,

    /// 数据负载
    pub data: Vec<u8>,
}

impl IcmpEcho {
    /// Echo 报文最小长度（头部 8 字节，无数据）
    pub const MIN_LEN: usize = 8;

    /// 创建新的 Echo Request
    pub fn new_request(identifier: u16, sequence: u16, data: Vec<u8>) -> Self {
        Self::new(ICMP_TYPE_ECHO_REQUEST, identifier, sequence, data)
    }

    /// 创建新的 Echo Reply
    pub fn new_reply(identifier: u16, sequence: u16, data: Vec<u8>) -> Self {
        Self::new(ICMP_TYPE_ECHO_REPLY, identifier, sequence, data)
    }

    fn new(type_: u8, identifier: u16, sequence: u16, data: Vec<u8>) -> Self {
        IcmpEcho {
            type_,
            code: 0,
            checksum: 0,
            identifier,
            sequence,
            data,
        }
    }

    /// 从 Packet 解析 Echo 报文
    pub fn from_packet(packet: &mut Packet) -> Result<Self> {
        if packet.remaining() < Self::MIN_LEN {
            return Err(CoreError::invalid_packet(format!(
                "Echo报文长度不足：{} < {}",
                packet.remaining(),
                Self::MIN_LEN
            )));
        }

        // 读取类型
        let type_ = packet.read(1)
            .ok_or_else(|| CoreError::parse_error("读取Echo类型失败"))?[0];

        // 读取代码
        let code = packet.read(1)
            .ok_or_else(|| CoreError::parse_error("读取Echo代码失败"))?[0];

        // 读取校验和
        let checksum_bytes = packet.read(2)
            .ok_or_else(|| CoreError::parse_error("读取Echo校验和失败"))?;
        let checksum = u16::from_be_bytes([checksum_bytes[0], checksum_bytes[1]]);

        // 读取标识符
        let identifier_bytes = packet.read(2)
            .ok_or_else(|| CoreError::parse_error("读取Echo标识符失败"))?;
        let identifier = u16::from_be_bytes([identifier_bytes[0], identifier_bytes[1]]);

        // 读取序列号
        let sequence_bytes = packet.read(2)
            .ok_or_else(|| CoreError::parse_error("读取Echo序列号失败"))?;
        let sequence = u16::from_be_bytes([sequence_bytes[0], sequence_bytes[1]]);

        // 读取剩余数据
        let mut data = Vec::new();
        while packet.remaining() > 0 {
            if let Some(byte) = packet.read(1) {
                data.push(byte[0]);
            }
        }

        Ok(IcmpEcho {
            type_,
            code,
            checksum,
            identifier,
            sequence,
            data,
        })
    }

    /// 编码为字节数组
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(8 + self.data.len());

        bytes.push(self.type_);
        bytes.push(self.code);
        bytes.extend_from_slice(&self.checksum.to_be_bytes());
        bytes.extend_from_slice(&self.identifier.to_be_bytes());
        bytes.extend_from_slice(&self.sequence.to_be_bytes());
        bytes.extend_from_slice(&self.data);

        // 计算校验和
        let checksum = calculate_checksum(&bytes);
        bytes[2] = (checksum >> 8) as u8;
        bytes[3] = (checksum & 0xFF) as u8;

        bytes
    }

    /// 验证是否为 Echo Request
    pub fn is_request(&self) -> bool {
        self.type_ == ICMP_TYPE_ECHO_REQUEST
    }

    /// 验证是否为 Echo Reply
    pub fn is_reply(&self) -> bool {
        self.type_ == ICMP_TYPE_ECHO_REPLY
    }

    /// 创建对应的 Reply（从 Request）
    pub fn make_reply(&self) -> Self {
        IcmpEcho::new_reply(self.identifier, self.sequence, self.data.clone())
    }
}

// ========== Destination Unreachable ==========

/// Destination Unreachable 报文
#[derive(Debug, Clone, PartialEq)]
pub struct IcmpDestUnreachable {
    /// 类型 (3)
    pub type_: u8,

    /// 不可达代码
    pub code: u8,

    /// 校验和
    pub checksum: u16,

    /// 未使用（填充为 0）
    pub unused: u32,

    /// 原始 IP 数据报头部 + 8 字节数据
    pub original_datagram: Vec<u8>,
}

impl IcmpDestUnreachable {
    /// Destination Unreachable 最小长度（头部 8 字节）
    pub const MIN_LEN: usize = 8;

    /// 从 Packet 解析 Destination Unreachable 报文
    pub fn from_packet(packet: &mut Packet) -> Result<Self> {
        if packet.remaining() < Self::MIN_LEN {
            return Err(CoreError::invalid_packet(format!(
                "Destination Unreachable报文长度不足：{} < {}",
                packet.remaining(),
                Self::MIN_LEN
            )));
        }

        let type_ = packet.read(1)
            .ok_or_else(|| CoreError::parse_error("读取类型失败"))?[0];
        let code = packet.read(1)
            .ok_or_else(|| CoreError::parse_error("读取代码失败"))?[0];
        let checksum_bytes = packet.read(2)
            .ok_or_else(|| CoreError::parse_error("读取校验和失败"))?;
        let checksum = u16::from_be_bytes([checksum_bytes[0], checksum_bytes[1]]);

        let unused_bytes = packet.read(4)
            .ok_or_else(|| CoreError::parse_error("读取未使用字段失败"))?;
        let unused = u32::from_be_bytes([
            unused_bytes[0],
            unused_bytes[1],
            unused_bytes[2],
            unused_bytes[3],
        ]);

        // 读取原始数据报
        let mut original_datagram = Vec::new();
        while packet.remaining() > 0 {
            if let Some(byte) = packet.read(1) {
                original_datagram.push(byte[0]);
            }
        }

        Ok(IcmpDestUnreachable {
            type_,
            code,
            checksum,
            unused,
            original_datagram,
        })
    }

    /// 编码为字节数组
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(8 + self.original_datagram.len());

        bytes.push(self.type_);
        bytes.push(self.code);
        bytes.extend_from_slice(&self.checksum.to_be_bytes());
        bytes.extend_from_slice(&self.unused.to_be_bytes());
        bytes.extend_from_slice(&self.original_datagram);

        // 计算校验和
        let checksum = calculate_checksum(&bytes);
        bytes[2] = (checksum >> 8) as u8;
        bytes[3] = (checksum & 0xFF) as u8;

        bytes
    }
}

// ========== Time Exceeded ==========

/// Time Exceeded 报文
#[derive(Debug, Clone, PartialEq)]
pub struct IcmpTimeExceeded {
    /// 类型 (11)
    pub type_: u8,

    /// 超时代码 (0=TTL, 1=分片重组)
    pub code: u8,

    /// 校验和
    pub checksum: u16,

    /// 未使用（填充为 0）
    pub unused: u32,

    /// 原始 IP 数据报头部 + 8 字节数据
    pub original_datagram: Vec<u8>,
}

impl IcmpTimeExceeded {
    /// Time Exceeded 最小长度（头部 8 字节）
    pub const MIN_LEN: usize = 8;

    /// 从 Packet 解析 Time Exceeded 报文
    pub fn from_packet(packet: &mut Packet) -> Result<Self> {
        if packet.remaining() < Self::MIN_LEN {
            return Err(CoreError::invalid_packet(format!(
                "Time Exceeded报文长度不足：{} < {}",
                packet.remaining(),
                Self::MIN_LEN
            )));
        }

        let type_ = packet.read(1)
            .ok_or_else(|| CoreError::parse_error("读取类型失败"))?[0];
        let code = packet.read(1)
            .ok_or_else(|| CoreError::parse_error("读取代码失败"))?[0];
        let checksum_bytes = packet.read(2)
            .ok_or_else(|| CoreError::parse_error("读取校验和失败"))?;
        let checksum = u16::from_be_bytes([checksum_bytes[0], checksum_bytes[1]]);

        let unused_bytes = packet.read(4)
            .ok_or_else(|| CoreError::parse_error("读取未使用字段失败"))?;
        let unused = u32::from_be_bytes([
            unused_bytes[0],
            unused_bytes[1],
            unused_bytes[2],
            unused_bytes[3],
        ]);

        // 读取原始数据报
        let mut original_datagram = Vec::new();
        while packet.remaining() > 0 {
            if let Some(byte) = packet.read(1) {
                original_datagram.push(byte[0]);
            }
        }

        Ok(IcmpTimeExceeded {
            type_,
            code,
            checksum,
            unused,
            original_datagram,
        })
    }

    /// 编码为字节数组
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(8 + self.original_datagram.len());

        bytes.push(self.type_);
        bytes.push(self.code);
        bytes.extend_from_slice(&self.checksum.to_be_bytes());
        bytes.extend_from_slice(&self.unused.to_be_bytes());
        bytes.extend_from_slice(&self.original_datagram);

        // 计算校验和
        let checksum = calculate_checksum(&bytes);
        bytes[2] = (checksum >> 8) as u8;
        bytes[3] = (checksum & 0xFF) as u8;

        bytes
    }
}

// ========== ICMP 报文枚举 ==========

/// ICMP 报文类型（枚举所有支持的报文）
#[derive(Debug, Clone, PartialEq)]
pub enum IcmpPacket {
    Echo(IcmpEcho),
    DestUnreachable(IcmpDestUnreachable),
    TimeExceeded(IcmpTimeExceeded),
}

impl IcmpPacket {
    /// 从 Packet 解析 ICMP 报文
    pub fn from_packet(packet: &mut Packet) -> Result<Self> {
        // 读取类型但不消耗
        let type_ = packet.peek(1)
            .ok_or_else(|| CoreError::parse_error("读取ICMP类型失败"))?[0];

        match type_ {
            ICMP_TYPE_ECHO_REPLY | ICMP_TYPE_ECHO_REQUEST => {
                Ok(IcmpPacket::Echo(IcmpEcho::from_packet(packet)?))
            }
            ICMP_TYPE_DEST_UNREACHABLE => {
                Ok(IcmpPacket::DestUnreachable(IcmpDestUnreachable::from_packet(packet)?))
            }
            ICMP_TYPE_TIME_EXCEEDED => {
                Ok(IcmpPacket::TimeExceeded(IcmpTimeExceeded::from_packet(packet)?))
            }
            _ => Err(CoreError::UnsupportedProtocol(format!(
                "不支持的ICMP类型: {}", type_
            ))),
        }
    }

    /// 获取 ICMP 类型
    pub fn get_type(&self) -> u8 {
        match self {
            IcmpPacket::Echo(echo) => echo.type_,
            IcmpPacket::DestUnreachable(dest) => dest.type_,
            IcmpPacket::TimeExceeded(time) => time.type_,
        }
    }

    /// 编码为字节数组
    pub fn to_bytes(&self) -> Vec<u8> {
        match self {
            IcmpPacket::Echo(echo) => echo.to_bytes(),
            IcmpPacket::DestUnreachable(dest) => dest.to_bytes(),
            IcmpPacket::TimeExceeded(time) => time.to_bytes(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_echo_encode_decode() {
        let echo = IcmpEcho::new_request(1234, 1, vec![0x42; 32]);

        let bytes = echo.to_bytes();
        assert_eq!(bytes[0], ICMP_TYPE_ECHO_REQUEST);
        assert_eq!(bytes[1], 0);

        // 解析
        let mut packet = Packet::from_bytes(bytes);
        let decoded = IcmpEcho::from_packet(&mut packet).unwrap();

        assert_eq!(decoded.type_, ICMP_TYPE_ECHO_REQUEST);
        assert_eq!(decoded.identifier, 1234);
        assert_eq!(decoded.sequence, 1);
    }

    #[test]
    fn test_echo_make_reply() {
        let request = IcmpEcho::new_request(1234, 1, vec![0x42; 32]);
        let reply = request.make_reply();

        assert_eq!(reply.type_, ICMP_TYPE_ECHO_REPLY);
        assert_eq!(reply.identifier, 1234);
        assert_eq!(reply.sequence, 1);
        assert_eq!(reply.data, request.data);
    }
}
