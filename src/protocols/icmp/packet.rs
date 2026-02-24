// src/protocols/icmp/packet.rs
//
// ICMP 报文结构定义

use crate::common::{CoreError, Packet, Result};
use super::types::*;
use crate::protocols::ip::{calculate_checksum, calculate_icmpv6_checksum, IP_MIN_HEADER_LEN};
use crate::protocols::Ipv6Addr;

// ========== 辅助常量 ==========

/// ICMP 错误报文需要返回的最小数据量（IP 头部 + 8 字节数据）
pub const ICMP_ORIGINAL_DATAGRAM_MIN_LEN: usize = IP_MIN_HEADER_LEN + 8;

/// Broadcast address: 255.255.255.255
const IPV4_BROADCAST: [u8; 4] = [255, 255, 255, 255];

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

        // 读取代码并验证（RFC 792: Echo Request/Reply 的 Code 必须为 0）
        let code = packet.read(1)
            .ok_or_else(|| CoreError::parse_error("读取Echo代码失败"))?[0];

        if code != 0 {
            return Err(CoreError::invalid_packet(format!(
                "Echo报文Code字段无效：{}（必须为0）",
                code
            )));
        }

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

    /// 原始 IP 数据报头部 + 8 字节数据
    pub original_datagram: Vec<u8>,
}

impl IcmpDestUnreachable {
    /// Destination Unreachable 最小长度（头部 8 字节）
    pub const MIN_LEN: usize = 8;

    /// 从 Packet 解析 Destination Unreachable 报文
    pub fn from_packet(packet: &mut Packet) -> Result<Self> {
        parse_icmp_with_original_datagram(packet, "Destination Unreachable")
            .map(|(type_, code, checksum, original_datagram)| IcmpDestUnreachable {
                type_, code, checksum, original_datagram
            })
    }

    /// 编码为字节数组
    pub fn to_bytes(&self) -> Vec<u8> {
        encode_icmp_with_original_datagram(self.type_, self.code, &self.original_datagram)
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

    /// 原始 IP 数据报头部 + 8 字节数据
    pub original_datagram: Vec<u8>,
}

impl IcmpTimeExceeded {
    /// Time Exceeded 最小长度（头部 8 字节）
    pub const MIN_LEN: usize = 8;

    /// 从 Packet 解析 Time Exceeded 报文
    pub fn from_packet(packet: &mut Packet) -> Result<Self> {
        parse_icmp_with_original_datagram(packet, "Time Exceeded")
            .map(|(type_, code, checksum, original_datagram)| IcmpTimeExceeded {
                type_, code, checksum, original_datagram
            })
    }

    /// 编码为字节数组
    pub fn to_bytes(&self) -> Vec<u8> {
        encode_icmp_with_original_datagram(self.type_, self.code, &self.original_datagram)
    }
}

// ========== Parameter Problem ==========

/// Parameter Problem 报文
///
/// RFC 792: IP 头部参数错误时发送此消息
#[derive(Debug, Clone, PartialEq)]
pub struct IcmpParameterProblem {
    /// 类型 (12)
    pub type_: u8,

    /// 代码 (0=指针指示错误, 1=缺少必需选项, 2=错误长度)
    pub code: u8,

    /// 校验和
    pub checksum: u16,

    /// 指针：指向原始数据报中错误的字节
    pub pointer: u8,

    /// 原始 IP 数据报头部 + 8 字节数据
    pub original_datagram: Vec<u8>,
}

impl IcmpParameterProblem {
    /// Parameter Problem 最小长度（头部 8 字节）
    pub const MIN_LEN: usize = 8;

    /// 创建新的 Parameter Problem 报文
    ///
    /// # 参数
    /// - code: 错误代码
    /// - pointer: 指向错误字节的指针
    /// - original_datagram: 原始 IP 数据报
    pub fn new(code: u8, pointer: u8, original_datagram: Vec<u8>) -> Self {
        IcmpParameterProblem {
            type_: ICMP_TYPE_PARAMETER_PROBLEM,
            code,
            checksum: 0,
            pointer,
            original_datagram,
        }
    }

    /// 从 Packet 解析 Parameter Problem 报文
    pub fn from_packet(packet: &mut Packet) -> Result<Self> {
        const MIN_LEN: usize = 8;

        if packet.remaining() < MIN_LEN {
            return Err(CoreError::invalid_packet(format!(
                "Parameter Problem报文长度不足：{} < {}",
                packet.remaining(),
                MIN_LEN
            )));
        }

        let type_ = packet.read(1)
            .ok_or_else(|| CoreError::parse_error("读取类型失败"))?[0];
        let code = packet.read(1)
            .ok_or_else(|| CoreError::parse_error("读取代码失败"))?[0];

        // 验证 Code 字段是否合法
        if code > 2 {
            return Err(CoreError::invalid_packet(format!(
                "Parameter Problem Code无效：{}",
                code
            )));
        }

        let checksum_bytes = packet.read(2)
            .ok_or_else(|| CoreError::parse_error("读取校验和失败"))?;
        let checksum = u16::from_be_bytes([checksum_bytes[0], checksum_bytes[1]]);

        // 读取指针
        let pointer = packet.read(1)
            .ok_or_else(|| CoreError::parse_error("读取指针失败"))?[0];

        // 跳过 3 字节的保留字段
        packet.read(3)
            .ok_or_else(|| CoreError::parse_error("读取保留字段失败"))?;

        // 读取原始数据报
        let mut original_datagram = Vec::new();
        while packet.remaining() > 0 {
            if let Some(byte) = packet.read(1) {
                original_datagram.push(byte[0]);
            }
        }

        // 验证原始数据报长度
        validate_original_datagram(&original_datagram, "Parameter Problem")?;

        Ok(IcmpParameterProblem {
            type_, code, checksum, pointer, original_datagram
        })
    }

    /// 编码为字节数组
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(8 + self.original_datagram.len());

        bytes.push(self.type_);
        bytes.push(self.code);
        bytes.extend_from_slice(&[0, 0]); // 校验和占位
        bytes.push(self.pointer);
        bytes.extend_from_slice(&[0, 0, 0]); // 保留字段填充为 0
        bytes.extend_from_slice(&self.original_datagram);

        // 计算校验和
        let checksum = calculate_checksum(&bytes);
        bytes[2] = (checksum >> 8) as u8;
        bytes[3] = (checksum & 0xFF) as u8;

        bytes
    }
}

// ========== 辅助函数 ==========

/// 从原始数据报中提取 IP 头部加上前 8 字节数据
///
/// # 参数
/// - datagram: 原始 IP 数据报
///
/// # 返回
/// - Ok(Vec<u8>): IP 头部 + 8 字节数据
/// - Err(CoreError): 数据报长度不足
pub fn extract_ip_header_plus_data(datagram: &[u8]) -> Result<Vec<u8>> {
    if datagram.len() < IP_MIN_HEADER_LEN {
        return Err(CoreError::invalid_packet(format!(
            "原始数据报长度不足：{} < {}",
            datagram.len(),
            IP_MIN_HEADER_LEN
        )));
    }

    // 读取 IHL 计算实际头部长度
    let ihl = datagram[0] & 0x0F;
    let header_len = (ihl as usize) * 4;

    if datagram.len() < header_len {
        return Err(CoreError::invalid_packet(format!(
            "原始数据报头部长度无效：IHL={} 需要{} 实际{}",
            ihl,
            header_len,
            datagram.len()
        )));
    }

    // 计算 IP 头部 + 8 字节数据的总长度
    let extract_len = header_len + 8;
    if datagram.len() < extract_len {
        return Err(CoreError::invalid_packet(format!(
            "原始数据报长度不足以提取头部+8字节：需要{} 实际{}",
            extract_len,
            datagram.len()
        )));
    }

    Ok(datagram[..extract_len].to_vec())
}

/// 验证原始数据报是否满足 ICMP 错误报文要求
///
/// # 参数
/// - datagram: 原始 IP 数据报
/// - name: 报文类型名称（用于错误消息）
///
/// # 返回
/// - Ok(()): 验证通过
/// - Err(CoreError): 验证失败
pub fn validate_original_datagram(datagram: &[u8], name: &str) -> Result<()> {
    if datagram.len() < ICMP_ORIGINAL_DATAGRAM_MIN_LEN {
        return Err(CoreError::invalid_packet(format!(
            "{}原始数据报长度不足：需要至少{}字节（IP头部+8字节）实际{}字节",
            name,
            ICMP_ORIGINAL_DATAGRAM_MIN_LEN,
            datagram.len()
        )));
    }

    // 验证 IP 头部有效性
    if datagram.len() < IP_MIN_HEADER_LEN {
        return Err(CoreError::invalid_packet(format!(
            "{}原始数据报长度不足以包含IP头部",
            name
        )));
    }

    // 读取 IHL 并验证
    let ihl = datagram[0] & 0x0F;
    if ihl < 5 {
        return Err(CoreError::invalid_packet(format!(
            "{}原始数据报IP头部IHL无效：{}",
            name, ihl
        )));
    }

    Ok(())
}

/// Check if IPv4 address is broadcast address
pub fn is_broadcast_addr(addr: &[u8; 4]) -> bool {
    *addr == IPV4_BROADCAST
}

/// Check if IPv4 address is multicast address
pub fn is_multicast_addr(addr: &[u8; 4]) -> bool {
    // Multicast range: 224.0.0.0/4, high 4 bits of first byte are 1110 (0xE0)
    (addr[0] & 0xF0) == 0xE0
}

/// 解析带原始数据报的 ICMP 报文（用于 Destination Unreachable 和 Time Exceeded）
fn parse_icmp_with_original_datagram(
    packet: &mut Packet,
    name: &str,
) -> Result<(u8, u8, u16, Vec<u8>)> {
    const MIN_LEN: usize = 8;

    if packet.remaining() < MIN_LEN {
        return Err(CoreError::invalid_packet(format!(
            "{}报文长度不足：{} < {}",
            name,
            packet.remaining(),
            MIN_LEN
        )));
    }

    let type_ = packet.read(1)
        .ok_or_else(|| CoreError::parse_error("读取类型失败"))?[0];
    let code = packet.read(1)
        .ok_or_else(|| CoreError::parse_error("读取代码失败"))?[0];

    // 验证 Code 字段是否合法
    validate_error_code(type_, code)?;

    let checksum_bytes = packet.read(2)
        .ok_or_else(|| CoreError::parse_error("读取校验和失败"))?;
    let checksum = u16::from_be_bytes([checksum_bytes[0], checksum_bytes[1]]);

    // 跳过 4 字节的 unused 字段
    packet.read(4)
        .ok_or_else(|| CoreError::parse_error("读取未使用字段失败"))?;

    // 读取原始数据报
    let mut original_datagram = Vec::new();
    while packet.remaining() > 0 {
        if let Some(byte) = packet.read(1) {
            original_datagram.push(byte[0]);
        }
    }

    // 验证原始数据报长度
    validate_original_datagram(&original_datagram, name)?;

    Ok((type_, code, checksum, original_datagram))
}

/// 验证 ICMP 错误报文的 Code 字段是否合法
///
/// # 参数
/// - type_: ICMP 类型
/// - code: ICMP 代码
///
/// # 返回
/// - Ok(()): 验证通过
/// - Err(CoreError): 验证失败
fn validate_error_code(type_: u8, code: u8) -> Result<()> {
    match type_ {
        ICMP_TYPE_DEST_UNREACHABLE => {
            // Destination Unreachable Code 范围：0-5
            if code > 5 {
                return Err(CoreError::invalid_packet(format!(
                    "Destination Unreachable Code无效：{}",
                    code
                )));
            }
        }
        ICMP_TYPE_TIME_EXCEEDED => {
            // Time Exceeded Code 范围：0-1
            if code > 1 {
                return Err(CoreError::invalid_packet(format!(
                    "Time Exceeded Code无效：{}",
                    code
                )));
            }
        }
        ICMP_TYPE_PARAMETER_PROBLEM => {
            // Parameter Problem Code 范围：0-2
            if code > 2 {
                return Err(CoreError::invalid_packet(format!(
                    "Parameter Problem Code无效：{}",
                    code
                )));
            }
        }
        _ => {}
    }
    Ok(())
}

/// 编码带原始数据报的 ICMP 报文（用于 Destination Unreachable 和 Time Exceeded）
fn encode_icmp_with_original_datagram(
    type_: u8,
    code: u8,
    original_datagram: &[u8],
) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(8 + original_datagram.len());

    bytes.push(type_);
    bytes.push(code);
    bytes.extend_from_slice(&[0, 0]); // 校验和占位
    bytes.extend_from_slice(&[0, 0, 0, 0]); // unused 字段填充为 0
    bytes.extend_from_slice(original_datagram);

    // 计算校验和
    let checksum = calculate_checksum(&bytes);
    bytes[2] = (checksum >> 8) as u8;
    bytes[3] = (checksum & 0xFF) as u8;

    bytes
}

// ========== ICMP 报文枚举 ==========

/// ICMP 报文类型（枚举所有支持的报文）
#[derive(Debug, Clone, PartialEq)]
pub enum IcmpPacket {
    Echo(IcmpEcho),
    DestUnreachable(IcmpDestUnreachable),
    TimeExceeded(IcmpTimeExceeded),
    ParameterProblem(IcmpParameterProblem),
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
            ICMP_TYPE_PARAMETER_PROBLEM => {
                Ok(IcmpPacket::ParameterProblem(IcmpParameterProblem::from_packet(packet)?))
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
            IcmpPacket::ParameterProblem(param) => param.type_,
        }
    }

    /// 编码为字节数组
    pub fn to_bytes(&self) -> Vec<u8> {
        match self {
            IcmpPacket::Echo(echo) => echo.to_bytes(),
            IcmpPacket::DestUnreachable(dest) => dest.to_bytes(),
            IcmpPacket::TimeExceeded(time) => time.to_bytes(),
            IcmpPacket::ParameterProblem(param) => param.to_bytes(),
        }
    }
}

// ========== ICMPv6 Echo Request/Reply ==========

/// ICMPv6 Echo Request/Reply 报文
#[derive(Debug, Clone, PartialEq)]
pub struct IcmpV6Echo {
    /// 类型 (128=Request, 129=Reply)
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

impl IcmpV6Echo {
    /// Echo 报文最小长度（头部 8 字节，无数据）
    pub const MIN_LEN: usize = 8;

    /// 创建新的 Echo Request
    pub fn new_request(identifier: u16, sequence: u16, data: Vec<u8>) -> Self {
        Self::new(ICMPV6_TYPE_ECHO_REQUEST, identifier, sequence, data)
    }

    /// 创建新的 Echo Reply
    pub fn new_reply(identifier: u16, sequence: u16, data: Vec<u8>) -> Self {
        Self::new(ICMPV6_TYPE_ECHO_REPLY, identifier, sequence, data)
    }

    fn new(type_: u8, identifier: u16, sequence: u16, data: Vec<u8>) -> Self {
        IcmpV6Echo {
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
                "ICMPv6 Echo报文长度不足：{} < {}",
                packet.remaining(),
                Self::MIN_LEN
            )));
        }

        // 读取类型
        let type_ = packet.read(1)
            .ok_or_else(|| CoreError::parse_error("读取ICMPv6 Echo类型失败"))?[0];

        // 读取代码并验证（RFC 4443: Echo Request/Reply 的 Code 必须为 0）
        let code = packet.read(1)
            .ok_or_else(|| CoreError::parse_error("读取ICMPv6 Echo代码失败"))?[0];

        if code != 0 {
            return Err(CoreError::invalid_packet(format!(
                "ICMPv6 Echo报文Code字段无效：{}（必须为0）",
                code
            )));
        }

        // 读取校验和
        let checksum_bytes = packet.read(2)
            .ok_or_else(|| CoreError::parse_error("读取ICMPv6 Echo校验和失败"))?;
        let checksum = u16::from_be_bytes([checksum_bytes[0], checksum_bytes[1]]);

        // 读取标识符
        let identifier_bytes = packet.read(2)
            .ok_or_else(|| CoreError::parse_error("读取ICMPv6 Echo标识符失败"))?;
        let identifier = u16::from_be_bytes([identifier_bytes[0], identifier_bytes[1]]);

        // 读取序列号
        let sequence_bytes = packet.read(2)
            .ok_or_else(|| CoreError::parse_error("读取ICMPv6 Echo序列号失败"))?;
        let sequence = u16::from_be_bytes([sequence_bytes[0], sequence_bytes[1]]);

        // 读取剩余数据
        let mut data = Vec::new();
        while packet.remaining() > 0 {
            if let Some(byte) = packet.read(1) {
                data.push(byte[0]);
            }
        }

        Ok(IcmpV6Echo {
            type_,
            code,
            checksum,
            identifier,
            sequence,
            data,
        })
    }

    /// 编码为字节数组
    ///
    /// 注意：此方法不包含 ICMPv6 伪头部校验和计算。
    /// 对于 ICMPv6，应使用 `to_bytes_with_addrs` 方法。
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(8 + self.data.len());

        bytes.push(self.type_);
        bytes.push(self.code);
        bytes.extend_from_slice(&self.checksum.to_be_bytes());
        bytes.extend_from_slice(&self.identifier.to_be_bytes());
        bytes.extend_from_slice(&self.sequence.to_be_bytes());
        bytes.extend_from_slice(&self.data);

        // 计算校验和（不包含伪头部，不适用于 ICMPv6）
        let checksum = calculate_checksum(&bytes);
        bytes[2] = (checksum >> 8) as u8;
        bytes[3] = (checksum & 0xFF) as u8;

        bytes
    }

    /// 编码为字节数组（使用 ICMPv6 伪头部校验和）
    ///
    /// ICMPv6 校验和需要包含伪头部（RFC 4443, RFC 8200）。
    ///
    /// # 参数
    /// - source_addr: 源 IPv6 地址
    /// - dest_addr: 目的 IPv6 地址
    ///
    /// # 返回
    /// - Vec<u8>: 包含正确校验和的 ICMPv6 报文字节数组
    pub fn to_bytes_with_addrs(&self, source_addr: Ipv6Addr, dest_addr: Ipv6Addr) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(8 + self.data.len());

        bytes.push(self.type_);
        bytes.push(self.code);
        bytes.extend_from_slice(&[0, 0]); // 校验和先置为 0
        bytes.extend_from_slice(&self.identifier.to_be_bytes());
        bytes.extend_from_slice(&self.sequence.to_be_bytes());
        bytes.extend_from_slice(&self.data);

        // 计算 ICMPv6 校验和（包含伪头部）
        let checksum = calculate_icmpv6_checksum(source_addr, dest_addr, &bytes);
        bytes[2] = (checksum >> 8) as u8;
        bytes[3] = (checksum & 0xFF) as u8;

        bytes
    }

    /// 验证是否为 Echo Request
    pub fn is_request(&self) -> bool {
        self.type_ == ICMPV6_TYPE_ECHO_REQUEST
    }

    /// 验证是否为 Echo Reply
    pub fn is_reply(&self) -> bool {
        self.type_ == ICMPV6_TYPE_ECHO_REPLY
    }

    /// 创建对应的 Echo Reply
    pub fn make_reply(&self) -> Self {
        Self {
            type_: ICMPV6_TYPE_ECHO_REPLY,
            code: 0,
            checksum: 0,
            identifier: self.identifier,
            sequence: self.sequence,
            data: self.data.clone(),
        }
    }
}

// ========== ICMPv6 报文枚举 ==========

/// ICMPv6 报文类型（枚举所有支持的报文）
#[derive(Debug, Clone, PartialEq)]
pub enum IcmpV6Packet {
    Echo(IcmpV6Echo),
}

impl IcmpV6Packet {
    /// 从 Packet 解析 ICMPv6 报文
    pub fn from_packet(packet: &mut Packet) -> Result<Self> {
        // 读取类型但不消耗
        let type_ = packet.peek(1)
            .ok_or_else(|| CoreError::parse_error("读取ICMPv6类型失败"))?[0];

        match type_ {
            ICMPV6_TYPE_ECHO_REPLY | ICMPV6_TYPE_ECHO_REQUEST => {
                Ok(IcmpV6Packet::Echo(IcmpV6Echo::from_packet(packet)?))
            }
            _ => Err(CoreError::UnsupportedProtocol(format!(
                "不支持的ICMPv6类型: {}", type_
            ))),
        }
    }

    /// 获取 ICMPv6 类型
    pub fn get_type(&self) -> u8 {
        match self {
            IcmpV6Packet::Echo(echo) => echo.type_,
        }
    }

    /// 编码为字节数组
    pub fn to_bytes(&self) -> Vec<u8> {
        match self {
            IcmpV6Packet::Echo(echo) => echo.to_bytes(),
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

    #[test]
    fn test_dest_unreachable_encode_decode() {
        // Need at least IP header (20 bytes) + 8 bytes data
        let original = vec![
            0x45, 0x00, 0x00, 0x1c,  // Version/IHL, TOS, Total Length
            0x00, 0x00, 0x00, 0x00,  // ID, Flags/Fragment
            0x40, 0x01, 0x00, 0x00,  // TTL, Protocol, Checksum
            0xc0, 0xa8, 0x01, 0x01,  // Source IP
            0xc0, 0xa8, 0x01, 0x02,  // Dest IP
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,  // 8 bytes data
        ];
        let dest = IcmpDestUnreachable {
            type_: ICMP_TYPE_DEST_UNREACHABLE,
            code: 0,
            checksum: 0,
            original_datagram: original.clone(),
        };

        let bytes = dest.to_bytes();
        assert_eq!(bytes[0], ICMP_TYPE_DEST_UNREACHABLE);
        assert_eq!(bytes[1], 0);

        // Parse
        let mut packet = Packet::from_bytes(bytes);
        let decoded = IcmpDestUnreachable::from_packet(&mut packet).unwrap();

        assert_eq!(decoded.type_, ICMP_TYPE_DEST_UNREACHABLE);
        assert_eq!(decoded.code, 0);
        assert_eq!(decoded.original_datagram, original);
    }

    #[test]
    fn test_time_exceeded_encode_decode() {
        // Need at least IP header (20 bytes) + 8 bytes data
        let original = vec![
            0x45, 0x00, 0x00, 0x1c,  // Version/IHL, TOS, Total Length
            0x00, 0x00, 0x00, 0x00,  // ID, Flags/Fragment
            0x40, 0x01, 0x00, 0x00,  // TTL, Protocol, Checksum
            0xc0, 0xa8, 0x01, 0x01,  // Source IP
            0xc0, 0xa8, 0x01, 0x02,  // Dest IP
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,  // 8 bytes data
        ];
        let time = IcmpTimeExceeded {
            type_: ICMP_TYPE_TIME_EXCEEDED,
            code: 0,
            checksum: 0,
            original_datagram: original.clone(),
        };

        let bytes = time.to_bytes();
        assert_eq!(bytes[0], ICMP_TYPE_TIME_EXCEEDED);
        assert_eq!(bytes[1], 0);

        // Parse
        let mut packet = Packet::from_bytes(bytes);
        let decoded = IcmpTimeExceeded::from_packet(&mut packet).unwrap();

        assert_eq!(decoded.type_, ICMP_TYPE_TIME_EXCEEDED);
        assert_eq!(decoded.code, 0);
        assert_eq!(decoded.original_datagram, original);
    }

    #[test]
    fn test_parameter_problem_encode_decode() {
        // Need at least IP header (20 bytes) + 8 bytes data
        let original = vec![
            0x45, 0x00, 0x00, 0x1c,  // Version/IHL, TOS, Total Length
            0x00, 0x00, 0x00, 0x00,  // ID, Flags/Fragment
            0x40, 0x01, 0x00, 0x00,  // TTL, Protocol, Checksum
            0xc0, 0xa8, 0x01, 0x01,  // Source IP
            0xc0, 0xa8, 0x01, 0x02,  // Dest IP
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,  // 8 bytes data
        ];
        let param = IcmpParameterProblem {
            type_: ICMP_TYPE_PARAMETER_PROBLEM,
            code: 0,
            checksum: 0,
            pointer: 8,
            original_datagram: original.clone(),
        };

        let bytes = param.to_bytes();
        assert_eq!(bytes[0], ICMP_TYPE_PARAMETER_PROBLEM);
        assert_eq!(bytes[1], 0);
        assert_eq!(bytes[4], 8); // Pointer

        // Parse
        let mut packet = Packet::from_bytes(bytes);
        let decoded = IcmpParameterProblem::from_packet(&mut packet).unwrap();

        assert_eq!(decoded.type_, ICMP_TYPE_PARAMETER_PROBLEM);
        assert_eq!(decoded.code, 0);
        assert_eq!(decoded.pointer, 8);
        assert_eq!(decoded.original_datagram, original);
    }
}
