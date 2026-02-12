// src/common/packet.rs
//
// 报文描述符设计
// Packet是CoreNet的核心数据结构，用于在协议栈各层之间传递报文数据

use std::time::Instant;
use crate::common::{CoreError, Result, MacAddr, IpAddr, IpVersion, EtherType, IpProtocol, Layer};

/// 接口ID类型
pub type InterfaceId = u32;

/// 报文描述符
///
/// Packet是协议栈中核心的数据结构，封装了：
/// - 原始buffer及其管理
/// - 协议解析过程的元数据
/// - 解析状态跟踪
pub struct Packet {
    // === Buffer管理 ===
    /// 报文数据缓冲区
    buffer: Vec<u8>,

    /// 报文实际数据长度
    length: usize,

    /// buffer总容量
    capacity: usize,

    // === 元数据 ===
    /// 接收时间戳
    pub timestamp: Instant,

    /// 接收接口ID
    pub interface: InterfaceId,

    // === 协议信息（逐步填充） ===
    /// 以太网源地址
    pub eth_src: Option<MacAddr>,

    /// 以太网目的地址
    pub eth_dst: Option<MacAddr>,

    /// 以太网类型
    pub eth_type: Option<EtherType>,

    /// IP版本 (4/6)
    pub ip_version: Option<IpVersion>,

    /// IP源地址
    pub ip_src: Option<IpAddr>,

    /// IP目的地址
    pub ip_dst: Option<IpAddr>,

    /// TTL/Hop Limit
    pub ip_ttl: Option<u8>,

    /// 传输层协议
    pub protocol: Option<IpProtocol>,

    /// 传输层源端口
    pub transport_src: Option<u16>,

    /// 传输层目的端口
    pub transport_dst: Option<u16>,

    // === 解析状态 ===
    /// 当前解析位置
    offset: usize,

    /// 已解析的协议层
    layers: Vec<Layer>,
}

impl Packet {
    // ========== 构造函数 ==========

    /// 创建新的空Packet
    ///
    /// # 参数
    /// - `capacity`: buffer容量
    pub fn new(capacity: usize) -> Self {
        let buffer = vec
![0u8; capacity];
        Packet {
            buffer,
            length: 0,
            capacity,
            timestamp: Instant::now(),
            interface: 0,
            eth_src: None,
            eth_dst: None,
            eth_type: None,
            ip_version: None,
            ip_src: None,
            ip_dst: None,
            ip_ttl: None,
            protocol: None,
            transport_src: None,
            transport_dst: None,
            offset: 0,
            layers: Vec::new(),
        }
    }

    /// 从已有数据创建Packet
    ///
    /// # 参数
    /// - `data`: 原始报文数据
    pub fn from_bytes(data: Vec<u8>) -> Self {
        let length = data.len();
        let capacity = data.capacity();
        Packet {
            buffer: data,
            length,
            capacity,
            timestamp: Instant::now(),
            interface: 0,
            eth_src: None,
            eth_dst: None,
            eth_type: None,
            ip_version: None,
            ip_src: None,
            ip_dst: None,
            ip_ttl: None,
            protocol: None,
            transport_src: None,
            transport_dst: None,
            offset: 0,
            layers: Vec::new(),
        }
    }

    /// 从静态字节切片创建Packet
    pub fn from_slice(data: &[u8]) -> Self {
        Packet {
            buffer: data.to_vec(),
            length: data.len(),
            capacity: data.len(),
            timestamp: Instant::now(),
            interface: 0,
            eth_src: None,
            eth_dst: None,
            eth_type: None,
            ip_version: None,
            ip_src: None,
            ip_dst: None,
            ip_ttl: None,
            protocol: None,
            transport_src: None,
            transport_dst: None,
            offset: 0,
            layers: Vec::new(),
        }
    }

    // ========== Buffer访问 ==========

    /// 获取buffer的不可变引用
    pub fn as_bytes(&self) -> &[u8] {
        &self.buffer[..self.length]
    }

    /// 获取buffer的可变引用
    pub fn as_mut_bytes(&mut self) -> &mut [u8] {
        &mut self.buffer[..self.length]
    }

    /// 获取指定范围的切片
    pub fn slice(&self, start: usize, end: usize) -> Result<&[u8]> {
        if start > end || end > self.length {
            return Err(CoreError::InvalidOffset {
                offset: end,
                max: self.length,
            });
        }
        Ok(&self.buffer[start..end])
    }

    /// 获取从offset开始的切片
    pub fn slice_from(&self, start: usize) -> Result<&[u8]> {
        if start > self.length {
            return Err(CoreError::InvalidOffset {
                offset: start,
                max: self.length,
            });
        }
        Ok(&self.buffer[start..self.length])
    }

    /// 追加数据到buffer末尾
    pub fn extend_from_slice(&mut self, data: &[u8]) -> Result<()> {
        let new_len = self.length + data.len();
        if new_len > self.capacity {
            return Err(CoreError::BufferOverflow);
        }
        self.buffer[self.length..new_len].copy_from_slice(data);
        self.length = new_len;
        Ok(())
    }

    /// 写入单个字节
    pub fn write_u8(&mut self, value: u8) -> Result<()> {
        if self.length >= self.capacity {
            return Err(CoreError::BufferOverflow);
        }
        self.buffer[self.length] = value;
        self.length += 1;
        Ok(())
    }

    /// 写入u16（大端序）
    pub fn write_u16(&mut self, value: u16) -> Result<()> {
        if self.length + 2 > self.capacity {
            return Err(CoreError::BufferOverflow);
        }
        self.buffer[self.length] = (value >> 8) as u8;
        self.buffer[self.length + 1] = value as u8;
        self.length += 2;
        Ok(())
    }

    /// 写入u32（大端序）
    pub fn write_u32(&mut self, value: u32) -> Result<()> {
        if self.length + 4 > self.capacity {
            return Err(CoreError::BufferOverflow);
        }
        self.buffer[self.length] = (value >> 24) as u8;
        self.buffer[self.length + 1] = (value >> 16) as u8;
        self.buffer[self.length + 2] = (value >> 8) as u8;
        self.buffer[self.length + 3] = value as u8;
        self.length += 4;
        Ok(())
    }

    // ========== 解析操作 ==========

    /// 获取剩余可读取长度
    pub fn remaining(&self) -> usize {
        self.length.saturating_sub(self.offset)
    }

    /// 检查是否有足够的数据可读
    pub fn has_remaining(&self, len: usize) -> bool {
        self.remaining() >= len
    }

    /// 读取指定字节数，不移动offset
    pub fn peek(&self, len: usize) -> Result<&[u8]> {
        if !self.has_remaining(len) {
            return Err(CoreError::BufferUnderflow);
        }
        Ok(&self.buffer[self.offset..self.offset + len])
    }

    /// 读取指定字节数，移动offset
    pub fn read(&mut self, len: usize) -> Result<&[u8]> {
        if !self.has_remaining(len) {
            return Err(CoreError::BufferUnderflow);
        }
        let start = self.offset;
        self.offset += len;
        Ok(&self.buffer[start..self.offset])
    }

    /// 跳过指定字节数
    pub fn skip(&mut self, len: usize) -> Result<()> {
        if !self.has_remaining(len) {
            return Err(CoreError::BufferUnderflow);
        }
        self.offset += len;
        Ok(())
    }

    /// 重置offset到指定位置
    pub fn seek(&mut self, offset: usize) -> Result<()> {
        if offset > self.length {
            return Err(CoreError::InvalidOffset {
                offset,
                max: self.length,
            });
        }
        self.offset = offset;
        Ok(())
    }

    /// 读取u8并移动offset
    pub fn read_u8(&mut self) -> Result<u8> {
        if !self.has_remaining(1) {
            return Err(CoreError::BufferUnderflow);
        }
        let value = self.buffer[self.offset];
        self.offset += 1;
        Ok(value)
    }

    /// 读取u16（大端序）并移动offset
    pub fn read_u16(&mut self) -> Result<u16> {
        if !self.has_remaining(2) {
            return Err(CoreError::BufferUnderflow);
        }
        let value = u16::from_be_bytes([
            self.buffer[self.offset],
            self.buffer[self.offset + 1],
        ]);
        self.offset += 2;
        Ok(value)
    }

    /// 读取u32（大端序）并移动offset
    pub fn read_u32(&mut self) -> Result<u32> {
        if !self.has_remaining(4) {
            return Err(CoreError::BufferUnderflow);
        }
        let value = u32::from_be_bytes([
            self.buffer[self.offset],
            self.buffer[self.offset + 1],
            self.buffer[self.offset + 2],
            self.buffer[self.offset + 3],
        ]);
        self.offset += 4;
        Ok(value)
    }

    /// peek u8不移动offset
    pub fn peek_u8(&self) -> Result<u8> {
        if !self.has_remaining(1) {
            return Err(CoreError::BufferUnderflow);
        }
        Ok(self.buffer[self.offset])
    }

    /// peek u16不移动offset
    pub fn peek_u16(&self) -> Result<u16> {
        if !self.has_remaining(2) {
            return Err(CoreError::BufferUnderflow);
        }
        Ok(u16::from_be_bytes([
            self.buffer[self.offset],
            self.buffer[self.offset + 1],
        ]))
    }

    // ========== 协议层管理 ==========

    /// 添加协议层
    pub fn push_layer(&mut self, layer: Layer) {
        self.layers.push(layer);
    }

    /// 移除最后一个协议层
    pub fn pop_layer(&mut self) -> Option<Layer> {
        self.layers.pop()
    }

    /// 获取当前（最后添加的）协议层
    pub fn current_layer(&self) -> Option<&Layer> {
        self.layers.last()
    }

    /// 获取所有协议层
    pub fn layers(&self) -> &[Layer] {
        &self.layers
    }

    /// 检查是否包含指定协议层
    pub fn has_layer(&self, layer: Layer) -> bool {
        self.layers.iter().any(|l| l == &layer)
    }

    // ========== 封装操作 ==========

    /// 在buffer头部预留空间（用于添加协议头）
    ///
    /// 这会将现有数据后移，为在头部添加协议头预留空间
    pub fn reserve_header(&mut self, len: usize) -> Result<()> {
        if self.length + len > self.capacity {
            return Err(CoreError::BufferOverflow);
        }
        // 将现有数据后移
        self.buffer.copy_within(self.offset..self.length, len);
        self.length += len;
        // 更新offset（数据已经后移）
        self.offset = 0;
        Ok(())
    }

    /// 在当前位置插入指定长度空间（将数据后移）
    pub fn insert_space(&mut self, len: usize) -> Result<()> {
        if self.length + len > self.capacity {
            return Err(CoreError::BufferOverflow);
        }
        // 从当前offset开始，将数据后移
        self.buffer.copy_within(self.offset..self.length, self.offset + len);
        self.length += len;
        Ok(())
    }

    /// 在当前位置写入协议头（先预留空间，再写入）
    pub fn write_header(&mut self, data: &[u8]) -> Result<()> {
        self.insert_space(data.len())?;
        self.buffer[self.offset..self.offset + data.len()].copy_from_slice(data);
        self.offset += data.len();
        Ok(())
    }

    // ========== 状态管理 ==========

    /// 清空数据，保留buffer
    pub fn clear(&mut self) {
        self.length = 0;
        self.offset = 0;
        self.layers.clear();

        // 清空协议元数据
        self.eth_src = None;
        self.eth_dst = None;
        self.eth_type = None;
        self.ip_version = None;
        self.ip_src = None;
        self.ip_dst = None;
        self.ip_ttl = None;
        self.protocol = None;
        self.transport_src = None;
        self.transport_dst = None;
    }

    /// 重置offset到数据起始位置
    pub fn reset_offset(&mut self) {
        self.offset = 0;
    }

    /// 获取当前offset位置
    pub fn get_offset(&self) -> usize {
        self.offset
    }

    /// 设置时间戳
    pub fn set_timestamp(&mut self, timestamp: Instant) {
        self.timestamp = timestamp;
    }

    /// 设置接口ID
    pub fn set_interface(&mut self, interface: InterfaceId) {
        self.interface = interface;
    }

    // ========== Getter方法 ==========

    /// 获取数据长度
    pub fn len(&self) -> usize {
        self.length
    }

    /// 获取buffer容量
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// 判断是否为空
    pub fn is_empty(&self) -> bool {
        self.length == 0
    }

    /// 获取元数据的不可变引用（用于调试或分析）
    pub fn metadata(&self) -> PacketMetadata {
        PacketMetadata {
            eth_src: self.eth_src,
            eth_dst: self.eth_dst,
            eth_type: self.eth_type,
            ip_version: self.ip_version,
            ip_src: self.ip_src,
            ip_dst: self.ip_dst,
            ip_ttl: self.ip_ttl,
            protocol: self.protocol,
            transport_src: self.transport_src,
            transport_dst: self.transport_dst,
        }
    }
}

/// Packet元数据快照（不包含buffer）
#[derive(Debug, Clone)]
pub struct PacketMetadata {
    pub eth_src: Option<MacAddr>,
    pub eth_dst: Option<MacAddr>,
    pub eth_type: Option<EtherType>,
    pub ip_version: Option<IpVersion>,
    pub ip_src: Option<IpAddr>,
    pub ip_dst: Option<IpAddr>,
    pub ip_ttl: Option<u8>,
    pub protocol: Option<IpProtocol>,
    pub transport_src: Option<u16>,
    pub transport_dst: Option<u16>,
}

// ========== 实现Clone（深度克隆） ==========

impl Clone for Packet {
    fn clone(&self) -> Self {
        Packet {
            buffer: self.buffer.clone(),
            length: self.length,
            capacity: self.capacity,
            timestamp: self.timestamp,
            interface: self.interface,
            eth_src: self.eth_src,
            eth_dst: self.eth_dst,
            eth_type: self.eth_type,
            ip_version: self.ip_version,
            ip_src: self.ip_src,
            ip_dst: self.ip_dst,
            ip_ttl: self.ip_ttl,
            protocol: self.protocol,
            transport_src: self.transport_src,
            transport_dst: self.transport_dst,
            offset: self.offset,
            layers: self.layers.clone(),
        }
    }
}

// ========== 实现 Clear trait ==========

use crate::common::pool::Clear;

impl Clear for Packet {
    fn clear(&mut self) {
        self.length = 0;
        self.offset = 0;
        self.layers.clear();

        // 清空协议元数据
        self.eth_src = None;
        self.eth_dst = None;
        self.eth_type = None;
        self.ip_version = None;
        self.ip_src = None;
        self.ip_dst = None;
        self.ip_ttl = None;
        self.protocol = None;
        self.transport_src = None;
        self.transport_dst = None;
    }
}

// ========== Debug实现 ==========

impl std::fmt::Debug for Packet {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Packet")
            .field("length", &self.length)
            .field("capacity", &self.capacity)
            .field("offset", &self.offset)
            .field("interface", &self.interface)
            .field("eth_src", &self.eth_src)
            .field("eth_dst", &self.eth_dst)
            .field("eth_type", &self.eth_type)
            .field("ip_version", &self.ip_version)
            .field("ip_src", &self.ip_src)
            .field("ip_dst", &self.ip_dst)
            .field("protocol", &self.protocol)
            .field("transport_src", &self.transport_src)
            .field("transport_dst", &self.transport_dst)
            .field("layers", &self.layers)
            .field("data", &format_args!("{:02x?}", self.as_bytes()))
            .finish()
    }
}

// ========== 单元测试 ==========

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_packet_new() {
        let packet = Packet::new(1500);
        assert_eq!(packet.len(), 0);
        assert_eq!(packet.capacity(), 1500);
        assert!(packet.is_empty());
        assert_eq!(packet.remaining(), 0);
    }

    #[test]
    fn test_packet_from_bytes() {
        let data = vec
![0x00, 0x01, 0x02, 0x03, 0x04];
        let packet = Packet::from_bytes(data);
        assert_eq!(packet.len(), 5);
        assert_eq!(packet.as_bytes(), &[0x00, 0x01, 0x02, 0x03, 0x04]);
        assert_eq!(packet.remaining(), 5);
    }

    #[test]
    fn test_read_operations() {
        let data = vec
![0x00, 0x01, 0x02, 0x03, 0x04];
        let mut packet = Packet::from_bytes(data);

        // 读取单字节
        assert_eq!(packet.read_u8().unwrap(), 0x00);
        assert_eq!(packet.get_offset(), 1);

        // 读取u16
        assert_eq!(packet.read_u16().unwrap(), 0x0102);
        assert_eq!(packet.get_offset(), 3);

        // peek
        assert_eq!(packet.peek_u16().unwrap(), 0x0304);
        assert_eq!(packet.get_offset(), 3); // peek不移动offset

        // skip
        packet.skip(2).unwrap();
        assert!(packet.remaining() == 0);
    }

    #[test]
    fn test_write_operations() {
        let mut packet = Packet::new(1500);

        // 写入单字节
        packet.write_u8(0x01).unwrap();
        assert_eq!(packet.len(), 1);

        // 写入u16
        packet.write_u16(0x0203).unwrap();
        assert_eq!(packet.len(), 3);

        // 写入u32
        packet.write_u32(0x04050607).unwrap();
        assert_eq!(packet.len(), 7);

        assert_eq!(packet.as_bytes(), &[0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07]);
    }

    #[test]
    fn test_extend_from_slice() {
        let mut packet = Packet::new(1500);
        packet.extend_from_slice(&[0x01, 0x02, 0x03]).unwrap();
        assert_eq!(packet.len(), 3);
        assert_eq!(packet.as_bytes(), &[0x01, 0x02, 0x03]);
    }

    #[test]
    fn test_reserve_header() {
        let mut packet = Packet::new(1500);
        packet.extend_from_slice(&[0x01, 0x02, 0x03]).unwrap();
        packet.reset_offset();

        // 预留2字节空间
        packet.reserve_header(2).unwrap();

        // 原有数据后移
        assert_eq!(packet.len(), 5);
        assert_eq!(&packet.buffer[..2], &[0, 0]);
        assert_eq!(&packet.buffer[2..5], &[0x01, 0x02, 0x03]);
    }

    #[test]
    fn test_layers() {
        let mut packet = Packet::new(1500);

        packet.push_layer(Layer::Ethernet);
        packet.push_layer(Layer::IPv4);
        packet.push_layer(Layer::TCP);

        assert_eq!(packet.current_layer(), Some(&Layer::TCP));
        assert!(packet.has_layer(Layer::Ethernet));
        assert!(packet.has_layer(Layer::IPv4));
        assert!(!packet.has_layer(Layer::UDP));

        assert_eq!(packet.pop_layer(), Some(Layer::TCP));
        assert_eq!(packet.current_layer(), Some(&Layer::IPv4));
    }

    #[test]
    fn test_clear() {
        let mut packet = Packet::new(1500);
        packet.extend_from_slice(&[0x01, 0x02, 0x03]).unwrap();
        packet.push_layer(Layer::Ethernet);
        packet.ip_src = Some(IpAddr::v4(192, 168, 1, 1));

        packet.clear();

        assert_eq!(packet.len(), 0);
        assert_eq!(packet.get_offset(), 0);
        assert!(packet.layers().is_empty());
        assert!(packet.ip_src.is_none());
    }

    #[test]
    fn test_error_handling() {
        let mut packet = Packet::new(2);

        // Buffer溢出
        packet.write_u32(0x12345678).unwrap_err();

        // Buffer下溢
        let result = packet.read_u8();
        assert!(matches!(result, Err(CoreError::BufferUnderflow)));
    }

    #[test]
    fn test_metadata() {
        let mut packet = Packet::new(1500);
        packet.eth_src = Some(MacAddr::BROADCAST);
        packet.ip_src = Some(IpAddr::v4(192, 168, 1, 1));

        let metadata = packet.metadata();
        assert_eq!(metadata.eth_src, Some(MacAddr::BROADCAST));
        assert_eq!(metadata.ip_src, Some(IpAddr::v4(192, 168, 1, 1)));
    }

    #[test]
    fn test_clone() {
        let mut packet1 = Packet::new(1500);
        packet1.extend_from_slice(&[0x01, 0x02, 0x03]).unwrap();
        packet1.push_layer(Layer::Ethernet);
        packet1.ip_src = Some(IpAddr::v4(192, 168, 1, 1));

        let packet2 = packet1.clone();

        assert_eq!(packet2.len(), 3);
        assert_eq!(packet2.as_bytes(), &[0x01, 0x02, 0x03]);
        assert!(packet2.has_layer(Layer::Ethernet));
        assert_eq!(packet2.ip_src, Some(IpAddr::v4(192, 168, 1, 1)));
    }

    #[test]
    fn test_seek() {
        let data = vec
![0x00, 0x01, 0x02, 0x03, 0x04];
        let mut packet = Packet::from_bytes(data);

        packet.skip(2).unwrap();
        assert_eq!(packet.get_offset(), 2);

        packet.seek(1).unwrap();
        assert_eq!(packet.get_offset(), 1);

        // 越界seek应该失败
        packet.seek(100).unwrap_err();
    }
}
