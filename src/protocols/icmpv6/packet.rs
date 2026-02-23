// src/protocols/icmpv6/packet.rs
//
// ICMPv6 报文结构定义
// RFC 4443: ICMPv6 报文格式

use crate::common::Packet;
use crate::protocols::Ipv6Addr;
use crate::protocols::ip::calculate_checksum;

use super::types::*;
use super::error::{Icmpv6Error, Icmpv6Result};

// ========== ICMPv6 通用头部 ==========

/// ICMPv6 通用头部
#[repr(C, packed)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Icmpv6Header {
    /// 消息类型
    pub type_: u8,
    /// 代码
    pub code: u8,
    /// 校验和
    pub checksum: u16,
}

impl Icmpv6Header {
    pub const SIZE: usize = 4;

    /// 从字节数组解析
    pub fn from_bytes(bytes: &[u8]) -> Icmpv6Result<Self> {
        if bytes.len() < Self::SIZE {
            return Err(Icmpv6Error::ParseError(format!(
                "ICMPv6头部长度不足: {} < {}",
                bytes.len(),
                Self::SIZE
            )));
        }

        Ok(Icmpv6Header {
            type_: bytes[0],
            code: bytes[1],
            checksum: u16::from_be_bytes([bytes[2], bytes[3]]),
        })
    }

    /// 编码为字节数组
    pub fn to_bytes(&self) -> [u8; Self::SIZE] {
        let mut bytes = [0u8; Self::SIZE];
        bytes[0] = self.type_;
        bytes[1] = self.code;
        bytes[2..4].copy_from_slice(&self.checksum.to_be_bytes());
        bytes
    }
}

// ========== Echo Request/Reply ==========

/// ICMPv6 Echo Request/Reply 报文
#[derive(Debug, Clone, PartialEq)]
pub struct Icmpv6Echo {
    /// 类型 (128=Request, 129=Reply)
    pub type_: u8,
    /// 代码 (始终为 0)
    pub code: u8,
    /// 校验和
    pub checksum: u16,
    /// 标识符
    pub identifier: u16,
    /// 序列号
    pub sequence: u16,
    /// 数据负载
    pub data: Vec<u8>,
}

impl Icmpv6Echo {
    pub const MIN_LEN: usize = 8;

    /// 创建新的 Echo Request
    pub fn new_request(identifier: u16, sequence: u16, data: Vec<u8>) -> Self {
        Self::new(Icmpv6Type::EchoRequest.as_u8(), identifier, sequence, data)
    }

    /// 创建新的 Echo Reply
    pub fn new_reply(identifier: u16, sequence: u16, data: Vec<u8>) -> Self {
        Self::new(Icmpv6Type::EchoReply.as_u8(), identifier, sequence, data)
    }

    fn new(type_: u8, identifier: u16, sequence: u16, data: Vec<u8>) -> Self {
        Icmpv6Echo {
            type_,
            code: 0,
            checksum: 0,
            identifier,
            sequence,
            data,
        }
    }

    /// 从 Packet 解析 ICMPv6 Echo 报文
    ///
    /// # 参数
    /// - `packet`: 包含 ICMPv6 Echo 报文的数据包
    ///
    /// # 返回
    /// - `Ok(Icmpv6Echo)`: 解析成功的 ICMPv6 Echo 报文
    /// - `Err(Icmpv6Error)`: 解析失败（长度不足或字段无效）
    pub fn from_packet(packet: &mut Packet) -> Icmpv6Result<Self> {
        if packet.remaining() < Self::MIN_LEN {
            return Err(Icmpv6Error::ParseError(format!(
                "Echo报文长度不足: {} < {}",
                packet.remaining(),
                Self::MIN_LEN
            )));
        }

        let type_ = packet.read(1)
            .ok_or_else(|| Icmpv6Error::ParseError("读取类型失败".to_string()))?[0];
        let code = packet.read(1)
            .ok_or_else(|| Icmpv6Error::ParseError("读取代码失败".to_string()))?[0];

        if code != 0 {
            return Err(Icmpv6Error::InvalidPacket(format!(
                "Echo报文Code字段无效: {} (必须为0)",
                code
            )));
        }

        let checksum_bytes = packet.read(2)
            .ok_or_else(|| Icmpv6Error::ParseError("读取校验和失败".to_string()))?;
        let checksum = u16::from_be_bytes([checksum_bytes[0], checksum_bytes[1]]);

        let id_bytes = packet.read(2)
            .ok_or_else(|| Icmpv6Error::ParseError("读取标识符失败".to_string()))?;
        let identifier = u16::from_be_bytes([id_bytes[0], id_bytes[1]]);

        let seq_bytes = packet.read(2)
            .ok_or_else(|| Icmpv6Error::ParseError("读取序列号失败".to_string()))?;
        let sequence = u16::from_be_bytes([seq_bytes[0], seq_bytes[1]]);

        let mut data = Vec::new();
        while packet.remaining() > 0 {
            if let Some(byte) = packet.read(1) {
                data.push(byte[0]);
            }
        }

        Ok(Icmpv6Echo {
            type_,
            code,
            checksum,
            identifier,
            sequence,
            data,
        })
    }

    /// 编码为字节数组（不含伪头部，需要外部添加伪头部计算校验和）
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(Self::MIN_LEN + self.data.len());

        bytes.push(self.type_);
        bytes.push(self.code);
        bytes.extend_from_slice(&self.checksum.to_be_bytes());
        bytes.extend_from_slice(&self.identifier.to_be_bytes());
        bytes.extend_from_slice(&self.sequence.to_be_bytes());
        bytes.extend_from_slice(&self.data);

        // 计算校验和（不包含伪头部，调用方需要处理）
        let checksum = calculate_checksum(&bytes);
        bytes[2] = (checksum >> 8) as u8;
        bytes[3] = (checksum & 0xFF) as u8;

        bytes
    }

    /// 编码为字节数组（校验和字段设为 0，由外部计算正确的 ICMPv6 校验和）
    pub fn to_bytes_without_checksum(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(Self::MIN_LEN + self.data.len());

        bytes.push(self.type_);
        bytes.push(self.code);
        bytes.extend_from_slice(&[0u8, 0u8]); // 校验和占位为 0
        bytes.extend_from_slice(&self.identifier.to_be_bytes());
        bytes.extend_from_slice(&self.sequence.to_be_bytes());
        bytes.extend_from_slice(&self.data);

        bytes
    }

    /// 创建对应的 Reply
    pub fn make_reply(&self) -> Self {
        Self {
            type_: Icmpv6Type::EchoReply.as_u8(),
            code: 0,
            checksum: 0,
            identifier: self.identifier,
            sequence: self.sequence,
            data: self.data.clone(),
        }
    }

    /// 是否为 Echo Request
    pub fn is_request(&self) -> bool {
        self.type_ == Icmpv6Type::EchoRequest.as_u8()
    }

    /// 是否为 Echo Reply
    pub fn is_reply(&self) -> bool {
        self.type_ == Icmpv6Type::EchoReply.as_u8()
    }
}

// ========== Destination Unreachable ==========

/// ICMPv6 Destination Unreachable 报文
#[derive(Debug, Clone, PartialEq)]
pub struct Icmpv6DestUnreachable {
    /// 类型 (1)
    pub type_: u8,
    /// 不可达代码
    pub code: u8,
    /// 校验和
    pub checksum: u16,
    /// 未使用（4字节）
    pub unused: u32,
    /// 原始 IPv6 数据报头部 + 数据
    pub original_datagram: Vec<u8>,
}

impl Icmpv6DestUnreachable {
    pub const MIN_LEN: usize = 8;

    /// 从 Packet 解析 ICMPv6 Destination Unreachable 报文
    ///
    /// # 参数
    /// - `packet`: 包含 ICMPv6 Destination Unreachable 报文的数据包
    ///
    /// # 返回
    /// - `Ok(Icmpv6DestUnreachable)`: 解析成功的报文
    /// - `Err(Icmpv6Error)`: 解析失败（长度不足或代码无效）
    pub fn from_packet(packet: &mut Packet) -> Icmpv6Result<Self> {
        if packet.remaining() < Self::MIN_LEN {
            return Err(Icmpv6Error::ParseError(format!(
                "Destination Unreachable报文长度不足: {} < {}",
                packet.remaining(),
                Self::MIN_LEN
            )));
        }

        let type_ = packet.read(1)
            .ok_or_else(|| Icmpv6Error::ParseError("读取类型失败".to_string()))?[0];
        let code = packet.read(1)
            .ok_or_else(|| Icmpv6Error::ParseError("读取代码失败".to_string()))?[0];

        // 验证代码
        if DestUnreachableCode::from_u8(code).is_none() {
            return Err(Icmpv6Error::InvalidPacket(format!(
                "Destination Unreachable Code无效: {}",
                code
            )));
        }

        let checksum_bytes = packet.read(2)
            .ok_or_else(|| Icmpv6Error::ParseError("读取校验和失败".to_string()))?;
        let checksum = u16::from_be_bytes([checksum_bytes[0], checksum_bytes[1]]);

        let unused_bytes = packet.read(4)
            .ok_or_else(|| Icmpv6Error::ParseError("读取未使用字段失败".to_string()))?;
        let unused = u32::from_be_bytes([unused_bytes[0], unused_bytes[1], unused_bytes[2], unused_bytes[3]]);

        let mut original_datagram = Vec::new();
        while packet.remaining() > 0 {
            if let Some(byte) = packet.read(1) {
                original_datagram.push(byte[0]);
            }
        }

        Ok(Icmpv6DestUnreachable {
            type_,
            code,
            checksum,
            unused,
            original_datagram,
        })
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(Self::MIN_LEN + self.original_datagram.len());

        bytes.push(self.type_);
        bytes.push(self.code);
        bytes.extend_from_slice(&[0, 0]); // 校验和占位
        bytes.extend_from_slice(&self.unused.to_be_bytes());
        bytes.extend_from_slice(&self.original_datagram);

        let checksum = calculate_checksum(&bytes);
        bytes[2] = (checksum >> 8) as u8;
        bytes[3] = (checksum & 0xFF) as u8;

        bytes
    }
}

// ========== Packet Too Big ==========

/// ICMPv6 Packet Too Big 报文
#[derive(Debug, Clone, PartialEq)]
pub struct Icmpv6PacketTooBig {
    /// 类型 (2)
    pub type_: u8,
    /// 代码 (始终为 0)
    pub code: u8,
    /// 校验和
    pub checksum: u16,
    /// MTU
    pub mtu: u32,
    /// 原始 IPv6 数据报
    pub original_datagram: Vec<u8>,
}

impl Icmpv6PacketTooBig {
    pub const MIN_LEN: usize = 8;

    /// 从 Packet 解析 ICMPv6 Packet Too Big 报文
    ///
    /// # 参数
    /// - `packet`: 包含 ICMPv6 Packet Too Big 报文的数据包
    ///
    /// # 返回
    /// - `Ok(Icmpv6PacketTooBig)`: 解析成功的报文
    /// - `Err(Icmpv6Error)`: 解析失败
    pub fn from_packet(packet: &mut Packet) -> Icmpv6Result<Self> {
        if packet.remaining() < Self::MIN_LEN {
            return Err(Icmpv6Error::ParseError(format!(
                "Packet Too Big报文长度不足: {} < {}",
                packet.remaining(),
                Self::MIN_LEN
            )));
        }

        let type_ = packet.read(1)
            .ok_or_else(|| Icmpv6Error::ParseError("读取类型失败".to_string()))?[0];
        let code = packet.read(1)
            .ok_or_else(|| Icmpv6Error::ParseError("读取代码失败".to_string()))?[0];

        if code != 0 {
            return Err(Icmpv6Error::InvalidPacket(format!(
                "Packet Too Big Code字段无效: {} (必须为0)",
                code
            )));
        }

        let checksum_bytes = packet.read(2)
            .ok_or_else(|| Icmpv6Error::ParseError("读取校验和失败".to_string()))?;
        let checksum = u16::from_be_bytes([checksum_bytes[0], checksum_bytes[1]]);

        let mtu_bytes = packet.read(4)
            .ok_or_else(|| Icmpv6Error::ParseError("读取MTU失败".to_string()))?;
        let mtu = u32::from_be_bytes([mtu_bytes[0], mtu_bytes[1], mtu_bytes[2], mtu_bytes[3]]);

        let mut original_datagram = Vec::new();
        while packet.remaining() > 0 {
            if let Some(byte) = packet.read(1) {
                original_datagram.push(byte[0]);
            }
        }

        Ok(Icmpv6PacketTooBig {
            type_,
            code,
            checksum,
            mtu,
            original_datagram,
        })
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(Self::MIN_LEN + self.original_datagram.len());

        bytes.push(self.type_);
        bytes.push(self.code);
        bytes.extend_from_slice(&[0, 0]); // 校验和占位
        bytes.extend_from_slice(&self.mtu.to_be_bytes());
        bytes.extend_from_slice(&self.original_datagram);

        let checksum = calculate_checksum(&bytes);
        bytes[2] = (checksum >> 8) as u8;
        bytes[3] = (checksum & 0xFF) as u8;

        bytes
    }
}

// ========== Time Exceeded ==========

/// ICMPv6 Time Exceeded 报文
#[derive(Debug, Clone, PartialEq)]
pub struct Icmpv6TimeExceeded {
    /// 类型 (3)
    pub type_: u8,
    /// 超时代码
    pub code: u8,
    /// 校验和
    pub checksum: u16,
    /// 未使用（4字节）
    pub unused: u32,
    /// 原始 IPv6 数据报
    pub original_datagram: Vec<u8>,
}

impl Icmpv6TimeExceeded {
    pub const MIN_LEN: usize = 8;

    /// 从 Packet 解析 ICMPv6 Time Exceeded 报文
    ///
    /// # 参数
    /// - `packet`: 包含 ICMPv6 Time Exceeded 报文的数据包
    ///
    /// # 返回
    /// - `Ok(Icmpv6TimeExceeded)`: 解析成功的报文
    /// - `Err(Icmpv6Error)`: 解析失败
    pub fn from_packet(packet: &mut Packet) -> Icmpv6Result<Self> {
        if packet.remaining() < Self::MIN_LEN {
            return Err(Icmpv6Error::ParseError(format!(
                "Time Exceeded报文长度不足: {} < {}",
                packet.remaining(),
                Self::MIN_LEN
            )));
        }

        let type_ = packet.read(1)
            .ok_or_else(|| Icmpv6Error::ParseError("读取类型失败".to_string()))?[0];
        let code = packet.read(1)
            .ok_or_else(|| Icmpv6Error::ParseError("读取代码失败".to_string()))?[0];

        // 验证代码
        if TimeExceededCode::from_u8(code).is_none() {
            return Err(Icmpv6Error::InvalidPacket(format!(
                "Time Exceeded Code无效: {}",
                code
            )));
        }

        let checksum_bytes = packet.read(2)
            .ok_or_else(|| Icmpv6Error::ParseError("读取校验和失败".to_string()))?;
        let checksum = u16::from_be_bytes([checksum_bytes[0], checksum_bytes[1]]);

        let unused_bytes = packet.read(4)
            .ok_or_else(|| Icmpv6Error::ParseError("读取未使用字段失败".to_string()))?;
        let unused = u32::from_be_bytes([unused_bytes[0], unused_bytes[1], unused_bytes[2], unused_bytes[3]]);

        let mut original_datagram = Vec::new();
        while packet.remaining() > 0 {
            if let Some(byte) = packet.read(1) {
                original_datagram.push(byte[0]);
            }
        }

        Ok(Icmpv6TimeExceeded {
            type_,
            code,
            checksum,
            unused,
            original_datagram,
        })
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(Self::MIN_LEN + self.original_datagram.len());

        bytes.push(self.type_);
        bytes.push(self.code);
        bytes.extend_from_slice(&[0, 0]); // 校验和占位
        bytes.extend_from_slice(&self.unused.to_be_bytes());
        bytes.extend_from_slice(&self.original_datagram);

        let checksum = calculate_checksum(&bytes);
        bytes[2] = (checksum >> 8) as u8;
        bytes[3] = (checksum & 0xFF) as u8;

        bytes
    }
}

// ========== Parameter Problem ==========

/// ICMPv6 Parameter Problem 报文
#[derive(Debug, Clone, PartialEq)]
pub struct Icmpv6ParameterProblem {
    /// 类型 (4)
    pub type_: u8,
    /// 参数问题代码
    pub code: u8,
    /// 校验和
    pub checksum: u16,
    /// 指向错误字节的指针
    pub pointer: u32,
    /// 原始 IPv6 数据报
    pub original_datagram: Vec<u8>,
}

impl Icmpv6ParameterProblem {
    pub const MIN_LEN: usize = 8;

    /// 从 Packet 解析 ICMPv6 Parameter Problem 报文
    ///
    /// # 参数
    /// - `packet`: 包含 ICMPv6 Parameter Problem 报文的数据包
    ///
    /// # 返回
    /// - `Ok(Icmpv6ParameterProblem)`: 解析成功的报文
    /// - `Err(Icmpv6Error)`: 解析失败
    pub fn from_packet(packet: &mut Packet) -> Icmpv6Result<Self> {
        if packet.remaining() < Self::MIN_LEN {
            return Err(Icmpv6Error::ParseError(format!(
                "Parameter Problem报文长度不足: {} < {}",
                packet.remaining(),
                Self::MIN_LEN
            )));
        }

        let type_ = packet.read(1)
            .ok_or_else(|| Icmpv6Error::ParseError("读取类型失败".to_string()))?[0];
        let code = packet.read(1)
            .ok_or_else(|| Icmpv6Error::ParseError("读取代码失败".to_string()))?[0];

        // 验证代码
        if ParameterProblemCode::from_u8(code).is_none() {
            return Err(Icmpv6Error::InvalidPacket(format!(
                "Parameter Problem Code无效: {}",
                code
            )));
        }

        let checksum_bytes = packet.read(2)
            .ok_or_else(|| Icmpv6Error::ParseError("读取校验和失败".to_string()))?;
        let checksum = u16::from_be_bytes([checksum_bytes[0], checksum_bytes[1]]);

        let ptr_bytes = packet.read(4)
            .ok_or_else(|| Icmpv6Error::ParseError("读取指针失败".to_string()))?;
        let pointer = u32::from_be_bytes([ptr_bytes[0], ptr_bytes[1], ptr_bytes[2], ptr_bytes[3]]);

        let mut original_datagram = Vec::new();
        while packet.remaining() > 0 {
            if let Some(byte) = packet.read(1) {
                original_datagram.push(byte[0]);
            }
        }

        Ok(Icmpv6ParameterProblem {
            type_,
            code,
            checksum,
            pointer,
            original_datagram,
        })
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(Self::MIN_LEN + self.original_datagram.len());

        bytes.push(self.type_);
        bytes.push(self.code);
        bytes.extend_from_slice(&[0, 0]); // 校验和占位
        bytes.extend_from_slice(&self.pointer.to_be_bytes());
        bytes.extend_from_slice(&self.original_datagram);

        let checksum = calculate_checksum(&bytes);
        bytes[2] = (checksum >> 8) as u8;
        bytes[3] = (checksum & 0xFF) as u8;

        bytes
    }
}

// ========== Router Solicitation ==========

/// ICMPv6 Router Solicitation 报文
#[derive(Debug, Clone, PartialEq)]
pub struct Icmpv6RouterSolicitation {
    /// 类型 (133)
    pub type_: u8,
    /// 代码 (始终为 0)
    pub code: u8,
    /// 校验和
    pub checksum: u16,
    /// 保留（必须为 0）
    pub reserved: u32,
    /// 选项
    pub options: Vec<Icmpv6Option>,
}

impl Icmpv6RouterSolicitation {
    pub const MIN_LEN: usize = 8;

    pub fn new() -> Self {
        Icmpv6RouterSolicitation {
            type_: Icmpv6Type::RouterSolicitation.as_u8(),
            code: 0,
            checksum: 0,
            reserved: 0,
            options: Vec::new(),
        }
    }

    /// 从 Packet 解析 ICMPv6 Router Solicitation 报文
    ///
    /// # 参数
    /// - `packet`: 包含 ICMPv6 Router Solicitation 报文的数据包
    ///
    /// # 返回
    /// - `Ok(Icmpv6RouterSolicitation)`: 解析成功的报文
    /// - `Err(Icmpv6Error)`: 解析失败
    pub fn from_packet(packet: &mut Packet) -> Icmpv6Result<Self> {
        if packet.remaining() < Self::MIN_LEN {
            return Err(Icmpv6Error::ParseError(format!(
                "Router Solicitation报文长度不足: {} < {}",
                packet.remaining(),
                Self::MIN_LEN
            )));
        }

        let type_ = packet.read(1)
            .ok_or_else(|| Icmpv6Error::ParseError("读取类型失败".to_string()))?[0];
        let code = packet.read(1)
            .ok_or_else(|| Icmpv6Error::ParseError("读取代码失败".to_string()))?[0];

        let checksum_bytes = packet.read(2)
            .ok_or_else(|| Icmpv6Error::ParseError("读取校验和失败".to_string()))?;
        let checksum = u16::from_be_bytes([checksum_bytes[0], checksum_bytes[1]]);

        let reserved_bytes = packet.read(4)
            .ok_or_else(|| Icmpv6Error::ParseError("读取保留字段失败".to_string()))?;
        let reserved = u32::from_be_bytes([reserved_bytes[0], reserved_bytes[1], reserved_bytes[2], reserved_bytes[3]]);

        // 解析选项
        let mut options = Vec::new();
        while packet.remaining() >= 2 {
            match Icmpv6Option::from_packet(packet) {
                Ok(opt) => options.push(opt),
                Err(_) => break, // 选项解析失败，停止解析
            }
        }

        Ok(Icmpv6RouterSolicitation {
            type_,
            code,
            checksum,
            reserved,
            options,
        })
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();

        bytes.push(self.type_);
        bytes.push(self.code);
        bytes.extend_from_slice(&[0, 0]); // 校验和占位
        bytes.extend_from_slice(&self.reserved.to_be_bytes());

        // 添加选项
        for opt in &self.options {
            bytes.extend_from_slice(&opt.to_bytes());
        }

        // 计算校验和
        let checksum = calculate_checksum(&bytes);
        bytes[2] = (checksum >> 8) as u8;
        bytes[3] = (checksum & 0xFF) as u8;

        bytes
    }
}

// ========== Router Advertisement ==========

/// ICMPv6 Router Advertisement 报文
#[derive(Debug, Clone, PartialEq)]
pub struct Icmpv6RouterAdvertisement {
    /// 类型 (134)
    pub type_: u8,
    /// 代码 (始终为 0)
    pub code: u8,
    /// 校验和
    pub checksum: u16,
    /// 当前 Hop Limit
    pub cur_hop_limit: u8,
    /// 标志位 (M|O|H|Reserved)
    pub flags: u8,
    /// 路由器生命周期（秒）
    pub lifetime: u16,
    /// 可达时间（毫秒）
    pub reachable_time: u32,
    /// 重传定时器（毫秒）
    pub retrans_timer: u32,
    /// 选项
    pub options: Vec<Icmpv6Option>,
}

impl Icmpv6RouterAdvertisement {
    pub const MIN_LEN: usize = 16;

    /// 从 Packet 解析 ICMPv6 Router Advertisement 报文
    ///
    /// # 参数
    /// - `packet`: 包含 ICMPv6 Router Advertisement 报文的数据包
    ///
    /// # 返回
    /// - `Ok(Icmpv6RouterAdvertisement)`: 解析成功的报文
    /// - `Err(Icmpv6Error)`: 解析失败
    pub fn from_packet(packet: &mut Packet) -> Icmpv6Result<Self> {
        if packet.remaining() < Self::MIN_LEN {
            return Err(Icmpv6Error::ParseError(format!(
                "Router Advertisement报文长度不足: {} < {}",
                packet.remaining(),
                Self::MIN_LEN
            )));
        }

        let type_ = packet.read(1)
            .ok_or_else(|| Icmpv6Error::ParseError("读取类型失败".to_string()))?[0];
        let code = packet.read(1)
            .ok_or_else(|| Icmpv6Error::ParseError("读取代码失败".to_string()))?[0];

        let checksum_bytes = packet.read(2)
            .ok_or_else(|| Icmpv6Error::ParseError("读取校验和失败".to_string()))?;
        let checksum = u16::from_be_bytes([checksum_bytes[0], checksum_bytes[1]]);

        let cur_hop_limit = packet.read(1)
            .ok_or_else(|| Icmpv6Error::ParseError("读取Hop Limit失败".to_string()))?[0];
        let flags = packet.read(1)
            .ok_or_else(|| Icmpv6Error::ParseError("读取标志失败".to_string()))?[0];

        let lifetime_bytes = packet.read(2)
            .ok_or_else(|| Icmpv6Error::ParseError("读取生命周期失败".to_string()))?;
        let lifetime = u16::from_be_bytes([lifetime_bytes[0], lifetime_bytes[1]]);

        let reachable_bytes = packet.read(4)
            .ok_or_else(|| Icmpv6Error::ParseError("读取可达时间失败".to_string()))?;
        let reachable_time = u32::from_be_bytes([reachable_bytes[0], reachable_bytes[1], reachable_bytes[2], reachable_bytes[3]]);

        let retrans_bytes = packet.read(4)
            .ok_or_else(|| Icmpv6Error::ParseError("读取重传定时器失败".to_string()))?;
        let retrans_timer = u32::from_be_bytes([retrans_bytes[0], retrans_bytes[1], retrans_bytes[2], retrans_bytes[3]]);

        // 解析选项
        let mut options = Vec::new();
        while packet.remaining() >= 2 {
            match Icmpv6Option::from_packet(packet) {
                Ok(opt) => options.push(opt),
                Err(_) => break,
            }
        }

        Ok(Icmpv6RouterAdvertisement {
            type_,
            code,
            checksum,
            cur_hop_limit,
            flags,
            lifetime,
            reachable_time,
            retrans_timer,
            options,
        })
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();

        bytes.push(self.type_);
        bytes.push(self.code);
        bytes.extend_from_slice(&[0, 0]); // 校验和占位
        bytes.push(self.cur_hop_limit);
        bytes.push(self.flags);
        bytes.extend_from_slice(&self.lifetime.to_be_bytes());
        bytes.extend_from_slice(&self.reachable_time.to_be_bytes());
        bytes.extend_from_slice(&self.retrans_timer.to_be_bytes());

        // 添加选项
        for opt in &self.options {
            bytes.extend_from_slice(&opt.to_bytes());
        }

        // 计算校验和
        let checksum = calculate_checksum(&bytes);
        bytes[2] = (checksum >> 8) as u8;
        bytes[3] = (checksum & 0xFF) as u8;

        bytes
    }

    /// 获取 M 标志（Managed Address Configuration）
    pub fn managed_flag(&self) -> bool {
        (self.flags & 0x80) != 0
    }

    /// 获取 O 标志（Other Configuration）
    pub fn other_flag(&self) -> bool {
        (self.flags & 0x40) != 0
    }

    /// 获取 H 标志（Home Agent）
    pub fn home_agent_flag(&self) -> bool {
        (self.flags & 0x20) != 0
    }
}

// ========== Neighbor Solicitation ==========

/// ICMPv6 Neighbor Solicitation 报文
#[derive(Debug, Clone, PartialEq)]
pub struct Icmpv6NeighborSolicitation {
    /// 类型 (135)
    pub type_: u8,
    /// 代码 (始终为 0)
    pub code: u8,
    /// 校验和
    pub checksum: u16,
    /// 保留（必须为 0）
    pub reserved: u32,
    /// 目标 IPv6 地址
    pub target_address: Ipv6Addr,
    /// 选项
    pub options: Vec<Icmpv6Option>,
}

impl Icmpv6NeighborSolicitation {
    pub const MIN_LEN: usize = 24;

    pub fn new(target_address: Ipv6Addr) -> Self {
        Icmpv6NeighborSolicitation {
            type_: Icmpv6Type::NeighborSolicitation.as_u8(),
            code: 0,
            checksum: 0,
            reserved: 0,
            target_address,
            options: Vec::new(),
        }
    }

    /// 从 Packet 解析 ICMPv6 Neighbor Solicitation 报文
    ///
    /// # 参数
    /// - `packet`: 包含 ICMPv6 Neighbor Solicitation 报文的数据包
    ///
    /// # 返回
    /// - `Ok(Icmpv6NeighborSolicitation)`: 解析成功的报文
    /// - `Err(Icmpv6Error)`: 解析失败
    pub fn from_packet(packet: &mut Packet) -> Icmpv6Result<Self> {
        if packet.remaining() < Self::MIN_LEN {
            return Err(Icmpv6Error::ParseError(format!(
                "Neighbor Solicitation报文长度不足: {} < {}",
                packet.remaining(),
                Self::MIN_LEN
            )));
        }

        let type_ = packet.read(1)
            .ok_or_else(|| Icmpv6Error::ParseError("读取类型失败".to_string()))?[0];
        let code = packet.read(1)
            .ok_or_else(|| Icmpv6Error::ParseError("读取代码失败".to_string()))?[0];

        let checksum_bytes = packet.read(2)
            .ok_or_else(|| Icmpv6Error::ParseError("读取校验和失败".to_string()))?;
        let checksum = u16::from_be_bytes([checksum_bytes[0], checksum_bytes[1]]);

        let reserved_bytes = packet.read(4)
            .ok_or_else(|| Icmpv6Error::ParseError("读取保留字段失败".to_string()))?;
        let reserved = u32::from_be_bytes([reserved_bytes[0], reserved_bytes[1], reserved_bytes[2], reserved_bytes[3]]);

        let target_bytes = packet.read(16)
            .ok_or_else(|| Icmpv6Error::ParseError("读取目标地址失败".to_string()))?;
        let mut target_array = [0u8; 16];
        target_array.copy_from_slice(target_bytes);
        let target_address = Ipv6Addr::from_bytes(target_array);

        // 解析选项
        let mut options = Vec::new();
        while packet.remaining() >= 2 {
            match Icmpv6Option::from_packet(packet) {
                Ok(opt) => options.push(opt),
                Err(_) => break,
            }
        }

        Ok(Icmpv6NeighborSolicitation {
            type_,
            code,
            checksum,
            reserved,
            target_address,
            options,
        })
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();

        bytes.push(self.type_);
        bytes.push(self.code);
        bytes.extend_from_slice(&[0, 0]); // 校验和占位
        bytes.extend_from_slice(&self.reserved.to_be_bytes());
        bytes.extend_from_slice(&self.target_address.bytes);

        // 添加选项
        for opt in &self.options {
            bytes.extend_from_slice(&opt.to_bytes());
        }

        // 计算校验和
        let checksum = calculate_checksum(&bytes);
        bytes[2] = (checksum >> 8) as u8;
        bytes[3] = (checksum & 0xFF) as u8;

        bytes
    }
}

// ========== Neighbor Advertisement ==========

/// ICMPv6 Neighbor Advertisement 报文
#[derive(Debug, Clone, PartialEq)]
pub struct Icmpv6NeighborAdvertisement {
    /// 类型 (136)
    pub type_: u8,
    /// 代码 (始终为 0)
    pub code: u8,
    /// 校验和
    pub checksum: u16,
    /// 标志位 (R|S|O|Reserved)
    pub flags: u8,
    /// 保留（必须为 0）
    pub reserved: [u8; 3],
    /// 目标 IPv6 地址
    pub target_address: Ipv6Addr,
    /// 选项
    pub options: Vec<Icmpv6Option>,
}

impl Icmpv6NeighborAdvertisement {
    pub const MIN_LEN: usize = 24;

    pub fn new(target_address: Ipv6Addr, router: bool, solicited: bool, override_: bool) -> Self {
        let mut flags = 0u8;
        if router {
            flags |= 0x80;
        }
        if solicited {
            flags |= 0x40;
        }
        if override_ {
            flags |= 0x20;
        }

        Icmpv6NeighborAdvertisement {
            type_: Icmpv6Type::NeighborAdvertisement.as_u8(),
            code: 0,
            checksum: 0,
            flags,
            reserved: [0, 0, 0],
            target_address,
            options: Vec::new(),
        }
    }

    /// 从 Packet 解析 ICMPv6 Neighbor Advertisement 报文
    ///
    /// # 参数
    /// - `packet`: 包含 ICMPv6 Neighbor Advertisement 报文的数据包
    ///
    /// # 返回
    /// - `Ok(Icmpv6NeighborAdvertisement)`: 解析成功的报文
    /// - `Err(Icmpv6Error)`: 解析失败
    pub fn from_packet(packet: &mut Packet) -> Icmpv6Result<Self> {
        if packet.remaining() < Self::MIN_LEN {
            return Err(Icmpv6Error::ParseError(format!(
                "Neighbor Advertisement报文长度不足: {} < {}",
                packet.remaining(),
                Self::MIN_LEN
            )));
        }

        let type_ = packet.read(1)
            .ok_or_else(|| Icmpv6Error::ParseError("读取类型失败".to_string()))?[0];
        let code = packet.read(1)
            .ok_or_else(|| Icmpv6Error::ParseError("读取代码失败".to_string()))?[0];

        let checksum_bytes = packet.read(2)
            .ok_or_else(|| Icmpv6Error::ParseError("读取校验和失败".to_string()))?;
        let checksum = u16::from_be_bytes([checksum_bytes[0], checksum_bytes[1]]);

        let flags = packet.read(1)
            .ok_or_else(|| Icmpv6Error::ParseError("读取标志失败".to_string()))?[0];

        let mut reserved = [0u8; 3];
        for item in &mut reserved {
            *item = packet.read(1)
                .ok_or_else(|| Icmpv6Error::ParseError("读取保留字段失败".to_string()))?[0];
        }

        let target_bytes = packet.read(16)
            .ok_or_else(|| Icmpv6Error::ParseError("读取目标地址失败".to_string()))?;
        let mut target_array = [0u8; 16];
        target_array.copy_from_slice(target_bytes);
        let target_address = Ipv6Addr::from_bytes(target_array);

        // 解析选项
        let mut options = Vec::new();
        while packet.remaining() >= 2 {
            match Icmpv6Option::from_packet(packet) {
                Ok(opt) => options.push(opt),
                Err(_) => break,
            }
        }

        Ok(Icmpv6NeighborAdvertisement {
            type_,
            code,
            checksum,
            flags,
            reserved,
            target_address,
            options,
        })
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();

        bytes.push(self.type_);
        bytes.push(self.code);
        bytes.extend_from_slice(&[0, 0]); // 校验和占位
        bytes.push(self.flags);
        bytes.extend_from_slice(&self.reserved);
        bytes.extend_from_slice(&self.target_address.bytes);

        // 添加选项
        for opt in &self.options {
            bytes.extend_from_slice(&opt.to_bytes());
        }

        // 计算校验和
        let checksum = calculate_checksum(&bytes);
        bytes[2] = (checksum >> 8) as u8;
        bytes[3] = (checksum & 0xFF) as u8;

        bytes
    }

    /// 获取 R 标志（Router）
    pub fn router_flag(&self) -> bool {
        (self.flags & 0x80) != 0
    }

    /// 获取 S 标志（Solicited）
    pub fn solicited_flag(&self) -> bool {
        (self.flags & 0x40) != 0
    }

    /// 获取 O 标志（Override）
    pub fn override_flag(&self) -> bool {
        (self.flags & 0x20) != 0
    }
}

// ========== Redirect ==========

/// ICMPv6 Redirect 报文
#[derive(Debug, Clone, PartialEq)]
pub struct Icmpv6Redirect {
    /// 类型 (137)
    pub type_: u8,
    /// 代码 (始终为 0)
    pub code: u8,
    /// 校验和
    pub checksum: u16,
    /// 保留（必须为 0）
    pub reserved: u32,
    /// 更好的下一跳地址
    pub target_address: Ipv6Addr,
    /// 最终目标地址
    pub destination_address: Ipv6Addr,
    /// 选项
    pub options: Vec<Icmpv6Option>,
}

impl Icmpv6Redirect {
    pub const MIN_LEN: usize = 40;

    /// 从 Packet 解析 ICMPv6 Redirect 报文
    ///
    /// # 参数
    /// - `packet`: 包含 ICMPv6 Redirect 报文的数据包
    ///
    /// # 返回
    /// - `Ok(Icmpv6Redirect)`: 解析成功的报文
    /// - `Err(Icmpv6Error)`: 解析失败
    pub fn from_packet(packet: &mut Packet) -> Icmpv6Result<Self> {
        if packet.remaining() < Self::MIN_LEN {
            return Err(Icmpv6Error::ParseError(format!(
                "Redirect报文长度不足: {} < {}",
                packet.remaining(),
                Self::MIN_LEN
            )));
        }

        let type_ = packet.read(1)
            .ok_or_else(|| Icmpv6Error::ParseError("读取类型失败".to_string()))?[0];
        let code = packet.read(1)
            .ok_or_else(|| Icmpv6Error::ParseError("读取代码失败".to_string()))?[0];

        let checksum_bytes = packet.read(2)
            .ok_or_else(|| Icmpv6Error::ParseError("读取校验和失败".to_string()))?;
        let checksum = u16::from_be_bytes([checksum_bytes[0], checksum_bytes[1]]);

        let reserved_bytes = packet.read(4)
            .ok_or_else(|| Icmpv6Error::ParseError("读取保留字段失败".to_string()))?;
        let reserved = u32::from_be_bytes([reserved_bytes[0], reserved_bytes[1], reserved_bytes[2], reserved_bytes[3]]);

        let target_bytes = packet.read(16)
            .ok_or_else(|| Icmpv6Error::ParseError("读取目标地址失败".to_string()))?;
        let mut target_array = [0u8; 16];
        target_array.copy_from_slice(target_bytes);
        let target_address = Ipv6Addr::from_bytes(target_array);

        let dest_bytes = packet.read(16)
            .ok_or_else(|| Icmpv6Error::ParseError("读取目的地址失败".to_string()))?;
        let mut dest_array = [0u8; 16];
        dest_array.copy_from_slice(dest_bytes);
        let destination_address = Ipv6Addr::from_bytes(dest_array);

        // 解析选项
        let mut options = Vec::new();
        while packet.remaining() >= 2 {
            match Icmpv6Option::from_packet(packet) {
                Ok(opt) => options.push(opt),
                Err(_) => break,
            }
        }

        Ok(Icmpv6Redirect {
            type_,
            code,
            checksum,
            reserved,
            target_address,
            destination_address,
            options,
        })
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();

        bytes.push(self.type_);
        bytes.push(self.code);
        bytes.extend_from_slice(&[0, 0]); // 校验和占位
        bytes.extend_from_slice(&self.reserved.to_be_bytes());
        bytes.extend_from_slice(&self.target_address.bytes);
        bytes.extend_from_slice(&self.destination_address.bytes);

        // 添加选项
        for opt in &self.options {
            bytes.extend_from_slice(&opt.to_bytes());
        }

        // 计算校验和
        let checksum = calculate_checksum(&bytes);
        bytes[2] = (checksum >> 8) as u8;
        bytes[3] = (checksum & 0xFF) as u8;

        bytes
    }
}

// ========== ICMPv6 选项 ==========

/// ICMPv6 选项
#[derive(Debug, Clone, PartialEq)]
pub struct Icmpv6Option {
    /// 选项类型
    pub option_type: u8,
    /// 选项长度（以 8 字节为单位）
    pub option_length: u8,
    /// 选项数据
    pub data: Vec<u8>,
}

impl Icmpv6Option {
    /// 最小选项长度（Type + Length = 2 字节）
    pub const MIN_LEN: usize = 2;

    /// 从 Packet 解析 ICMPv6 选项
    ///
    /// # 参数
    /// - `packet`: 包含 ICMPv6 选项的数据包
    ///
    /// # 返回
    /// - `Ok(Icmpv6Option)`: 解析成功的选项
    /// - `Err(Icmpv6Error)`: 解析失败
    pub fn from_packet(packet: &mut Packet) -> Icmpv6Result<Self> {
        if packet.remaining() < Self::MIN_LEN {
            return Err(Icmpv6Error::ParseError("选项长度不足".to_string()));
        }

        let option_type = packet.read(1)
            .ok_or_else(|| Icmpv6Error::ParseError("读取选项类型失败".to_string()))?[0];
        let option_length = packet.read(1)
            .ok_or_else(|| Icmpv6Error::ParseError("读取选项长度失败".to_string()))?[0];

        // 选项长度以 8 字节为单位，0 表示特殊处理
        let data_len = if option_length == 0 {
            0
        } else {
            (option_length as usize) * 8 - 2
        };

        // 检查是否有足够的数据
        if packet.remaining() < data_len {
            return Err(Icmpv6Error::ParseError(format!(
                "选项数据长度不足: 需要{} 实际{}",
                data_len,
                packet.remaining()
            )));
        }

        let mut data = Vec::new();
        for _ in 0..data_len {
            if let Some(byte) = packet.read(1) {
                data.push(byte[0]);
            }
        }

        Ok(Icmpv6Option {
            option_type,
            option_length,
            data,
        })
    }

    /// 编码为字节数组
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.push(self.option_type);

        // 计算长度（以 8 字节为单位）
        let total_len = 2 + self.data.len();
        let length_field = total_len.div_ceil(8) as u8;
        bytes.push(length_field);

        bytes.extend_from_slice(&self.data);

        // 填充到 8 字节边界
        while bytes.len() % 8 != 0 {
            bytes.push(0);
        }

        bytes
    }

    /// 创建源链路层地址选项
    pub fn source_link_layer_addr(mac: &[u8; 6]) -> Self {
        Icmpv6Option {
            option_type: Icmpv6OptionType::SourceLinkLayerAddr.as_u8(),
            option_length: 1,
            data: mac.to_vec(),
        }
    }

    /// 创建目标链路层地址选项
    pub fn target_link_layer_addr(mac: &[u8; 6]) -> Self {
        Icmpv6Option {
            option_type: Icmpv6OptionType::TargetLinkLayerAddr.as_u8(),
            option_length: 1,
            data: mac.to_vec(),
        }
    }

    /// 获取选项类型
    pub fn get_type(&self) -> Option<Icmpv6OptionType> {
        Icmpv6OptionType::from_u8(self.option_type)
    }
}

// ========== ICMPv6 报文枚举 ==========

/// ICMPv6 报文类型枚举
#[derive(Debug, Clone, PartialEq)]
pub enum Icmpv6Packet {
    Echo(Icmpv6Echo),
    DestUnreachable(Icmpv6DestUnreachable),
    PacketTooBig(Icmpv6PacketTooBig),
    TimeExceeded(Icmpv6TimeExceeded),
    ParameterProblem(Icmpv6ParameterProblem),
    RouterSolicitation(Icmpv6RouterSolicitation),
    RouterAdvertisement(Icmpv6RouterAdvertisement),
    NeighborSolicitation(Icmpv6NeighborSolicitation),
    NeighborAdvertisement(Icmpv6NeighborAdvertisement),
    Redirect(Icmpv6Redirect),
}

impl Icmpv6Packet {
    /// 从 Packet 解析 ICMPv6 报文
    pub fn from_packet(packet: &mut Packet) -> Icmpv6Result<Self> {
        let type_ = packet.peek(1)
            .ok_or_else(|| Icmpv6Error::ParseError("读取类型失败".to_string()))?[0];

        match type_ {
            128 | 129 => Ok(Icmpv6Packet::Echo(Icmpv6Echo::from_packet(packet)?)),
            1 => Ok(Icmpv6Packet::DestUnreachable(Icmpv6DestUnreachable::from_packet(packet)?)),
            2 => Ok(Icmpv6Packet::PacketTooBig(Icmpv6PacketTooBig::from_packet(packet)?)),
            3 => Ok(Icmpv6Packet::TimeExceeded(Icmpv6TimeExceeded::from_packet(packet)?)),
            4 => Ok(Icmpv6Packet::ParameterProblem(Icmpv6ParameterProblem::from_packet(packet)?)),
            133 => Ok(Icmpv6Packet::RouterSolicitation(Icmpv6RouterSolicitation::from_packet(packet)?)),
            134 => Ok(Icmpv6Packet::RouterAdvertisement(Icmpv6RouterAdvertisement::from_packet(packet)?)),
            135 => Ok(Icmpv6Packet::NeighborSolicitation(Icmpv6NeighborSolicitation::from_packet(packet)?)),
            136 => Ok(Icmpv6Packet::NeighborAdvertisement(Icmpv6NeighborAdvertisement::from_packet(packet)?)),
            137 => Ok(Icmpv6Packet::Redirect(Icmpv6Redirect::from_packet(packet)?)),
            _ => Err(Icmpv6Error::UnsupportedMessageType(type_)),
        }
    }

    /// 获取报文类型
    pub fn get_type(&self) -> u8 {
        match self {
            Icmpv6Packet::Echo(e) => e.type_,
            Icmpv6Packet::DestUnreachable(d) => d.type_,
            Icmpv6Packet::PacketTooBig(p) => p.type_,
            Icmpv6Packet::TimeExceeded(t) => t.type_,
            Icmpv6Packet::ParameterProblem(p) => p.type_,
            Icmpv6Packet::RouterSolicitation(r) => r.type_,
            Icmpv6Packet::RouterAdvertisement(r) => r.type_,
            Icmpv6Packet::NeighborSolicitation(n) => n.type_,
            Icmpv6Packet::NeighborAdvertisement(n) => n.type_,
            Icmpv6Packet::Redirect(r) => r.type_,
        }
    }

    /// 编码为字节数组
    pub fn to_bytes(&self) -> Vec<u8> {
        match self {
            Icmpv6Packet::Echo(e) => e.to_bytes(),
            Icmpv6Packet::DestUnreachable(d) => d.to_bytes(),
            Icmpv6Packet::PacketTooBig(p) => p.to_bytes(),
            Icmpv6Packet::TimeExceeded(t) => t.to_bytes(),
            Icmpv6Packet::ParameterProblem(p) => p.to_bytes(),
            Icmpv6Packet::RouterSolicitation(r) => r.to_bytes(),
            Icmpv6Packet::RouterAdvertisement(r) => r.to_bytes(),
            Icmpv6Packet::NeighborSolicitation(n) => n.to_bytes(),
            Icmpv6Packet::NeighborAdvertisement(n) => n.to_bytes(),
            Icmpv6Packet::Redirect(r) => r.to_bytes(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_icmpv6_header() {
        let header = Icmpv6Header {
            type_: 128,
            code: 0,
            checksum: 0x1234,
        };
        let bytes = header.to_bytes();
        assert_eq!(bytes[0], 128);
        assert_eq!(bytes[1], 0);
        assert_eq!(bytes[2], 0x12);
        assert_eq!(bytes[3], 0x34);
    }

    #[test]
    fn test_echo_encode_decode() {
        let echo = Icmpv6Echo::new_request(1234, 1, vec![0x42; 32]);
        let bytes = echo.to_bytes();

        assert_eq!(bytes[0], 128);
        assert_eq!(bytes[1], 0);

        let mut packet = Packet::from_bytes(bytes);
        let decoded = Icmpv6Echo::from_packet(&mut packet).unwrap();
        assert_eq!(decoded.type_, 128);
        assert_eq!(decoded.identifier, 1234);
        assert_eq!(decoded.sequence, 1);
    }

    #[test]
    fn test_echo_make_reply() {
        let request = Icmpv6Echo::new_request(1234, 1, vec![0x42; 32]);
        let reply = request.make_reply();

        assert_eq!(reply.type_, 129);
        assert_eq!(reply.identifier, 1234);
        assert_eq!(reply.sequence, 1);
        assert_eq!(reply.data, request.data);
    }

    #[test]
    fn test_neighbor_solicitation() {
        let target = Ipv6Addr::new(0xfe80, 0, 0, 0, 0, 0, 0, 1);
        let ns = Icmpv6NeighborSolicitation::new(target);

        assert_eq!(ns.type_, 135);
        assert_eq!(ns.target_address, target);
    }

    #[test]
    fn test_neighbor_advertisement_flags() {
        let target = Ipv6Addr::new(0xfe80, 0, 0, 0, 0, 0, 0, 1);
        let na = Icmpv6NeighborAdvertisement::new(target, true, true, false);

        assert!(na.router_flag());
        assert!(na.solicited_flag());
        assert!(!na.override_flag());
    }

    #[test]
    fn test_icmpv6_option() {
        let mac = [0x00, 0x11, 0x22, 0x33, 0x44, 0x55];
        let opt = Icmpv6Option::source_link_layer_addr(&mac);
        let bytes = opt.to_bytes();

        assert_eq!(bytes[0], 1); // Source Link-Layer Address
        assert_eq!(bytes[1], 1); // Length = 1 (8 bytes)
        assert_eq!(&bytes[2..8], &mac[..]);
    }
}
