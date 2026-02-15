// src/protocols/arp/mod.rs
//
// ARP（Address Resolution Protocol）地址解析协议实现
// 参考：RFC 826

use crate::common::{CoreError, Result};
use crate::protocols::{Packet, MacAddr, Ipv4Addr};

// ARP 表模块
pub mod tables;

// ========== ARP 操作码 ==========

/// ARP 操作码
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum ArpOperation {
    /// ARP 请求
    Request = 1,
    /// ARP 响应
    Reply = 2,
}

impl ArpOperation {
    /// 从 u16 转换
    pub fn from_u16(value: u16) -> Option<Self> {
        match value {
            1 => Some(ArpOperation::Request),
            2 => Some(ArpOperation::Reply),
            _ => None,
        }
    }

    /// 转换为 u16
    pub fn to_u16(self) -> u16 {
        self as u16
    }
}

// ========== ARP 报文结构 ==========

/// ARP 报文
#[derive(Debug, Clone)]
pub struct ArpPacket {
    /// 硬件类型（1=以太网）
    pub hardware_type: u16,
    /// 协议类型（0x0800=IPv4）
    pub protocol_type: u16,
    /// 硬件地址长度
    pub hardware_addr_len: u8,
    /// 协议地址长度
    pub protocol_addr_len: u8,
    /// 操作码
    pub operation: ArpOperation,
    /// 发送方硬件地址
    pub sender_hardware_addr: MacAddr,
    /// 发送方协议地址
    pub sender_protocol_addr: Ipv4Addr,
    /// 目标硬件地址
    pub target_hardware_addr: MacAddr,
    /// 目标协议地址
    pub target_protocol_addr: Ipv4Addr,
}

impl ArpPacket {
    /// ARP 报文最小长度（不包含以太网头）
    pub const MIN_LEN: usize = 28;

    /// 以太网硬件类型
    pub const ARPHRD_ETHER: u16 = 1;
    /// IPv4 协议类型
    pub const ETH_P_IP: u16 = 0x0800;

    /// 创建新的 ARP 报文
    pub fn new(
        operation: ArpOperation,
        sender_hardware_addr: MacAddr,
        sender_protocol_addr: Ipv4Addr,
        target_hardware_addr: MacAddr,
        target_protocol_addr: Ipv4Addr,
    ) -> Self {
        ArpPacket {
            hardware_type: Self::ARPHRD_ETHER,
            protocol_type: Self::ETH_P_IP,
            hardware_addr_len: 6,
            protocol_addr_len: 4,
            operation,
            sender_hardware_addr,
            sender_protocol_addr,
            target_hardware_addr,
            target_protocol_addr,
        }
    }

    /// 从 Packet 解析 ARP 报文
    pub fn from_packet(packet: &mut Packet) -> Result<Self> {
        // 检查最小长度
        if packet.remaining() < Self::MIN_LEN {
            return Err(CoreError::invalid_packet(
                format!("ARP报文长度不足：{} < {}", packet.remaining(), Self::MIN_LEN)
            ));
        }

        // 读取硬件类型
        let hardware_type = match packet.read(2) {
            Some(data) => u16::from_be_bytes([data[0], data[1]]),
            None => return Err(CoreError::parse_error("读取硬件类型失败")),
        };

        // 读取协议类型
        let protocol_type = match packet.read(2) {
            Some(data) => u16::from_be_bytes([data[0], data[1]]),
            None => return Err(CoreError::parse_error("读取协议类型失败")),
        };

        // 读取硬件地址长度
        let hardware_addr_len = match packet.read(1) {
            Some(data) => data[0],
            None => return Err(CoreError::parse_error("读取硬件地址长度失败")),
        };

        // 读取协议地址长度
        let protocol_addr_len = match packet.read(1) {
            Some(data) => data[0],
            None => return Err(CoreError::parse_error("读取协议地址长度失败")),
        };

        // 读取操作码
        let operation = match packet.read(2) {
            Some(data) => u16::from_be_bytes([data[0], data[1]]),
            None => return Err(CoreError::parse_error("读取操作码失败")),
        };

        let operation = ArpOperation::from_u16(operation)
            .ok_or_else(|| CoreError::invalid_packet(format!("无效的ARP操作码：{}", operation)))?;

        // 读取发送方硬件地址
        let mut sender_hardware_bytes = [0u8; 6];
        for i in 0..6 {
            sender_hardware_bytes[i] = match packet.read(1) {
                Some(data) => data[0],
                None => return Err(CoreError::parse_error("读取发送方硬件地址失败")),
            };
        }
        let sender_hardware_addr = MacAddr::new(sender_hardware_bytes);

        // 读取发送方协议地址
        let mut sender_protocol_bytes = [0u8; 4];
        for i in 0..4 {
            sender_protocol_bytes[i] = match packet.read(1) {
                Some(data) => data[0],
                None => return Err(CoreError::parse_error("读取发送方协议地址失败")),
            };
        }
        let sender_protocol_addr = Ipv4Addr::new(sender_protocol_bytes);

        // 读取目标硬件地址
        let mut target_hardware_bytes = [0u8; 6];
        for i in 0..6 {
            target_hardware_bytes[i] = match packet.read(1) {
                Some(data) => data[0],
                None => return Err(CoreError::parse_error("读取目标硬件地址失败")),
            };
        }
        let target_hardware_addr = MacAddr::new(target_hardware_bytes);

        // 读取目标协议地址
        let mut target_protocol_bytes = [0u8; 4];
        for i in 0..4 {
            target_protocol_bytes[i] = match packet.read(1) {
                Some(data) => data[0],
                None => return Err(CoreError::parse_error("读取目标协议地址失败")),
            };
        }
        let target_protocol_addr = Ipv4Addr::new(target_protocol_bytes);

        Ok(ArpPacket {
            hardware_type,
            protocol_type,
            hardware_addr_len,
            protocol_addr_len,
            operation,
            sender_hardware_addr,
            sender_protocol_addr,
            target_hardware_addr,
            target_protocol_addr,
        })
    }

    /// 将 ARP 报文编码为字节数组
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(Self::MIN_LEN);

        // 硬件类型
        bytes.extend_from_slice(&self.hardware_type.to_be_bytes());
        // 协议类型
        bytes.extend_from_slice(&self.protocol_type.to_be_bytes());
        // 硬件地址长度
        bytes.push(self.hardware_addr_len);
        // 协议地址长度
        bytes.push(self.protocol_addr_len);
        // 操作码
        bytes.extend_from_slice(&self.operation.to_u16().to_be_bytes());
        // 发送方硬件地址
        bytes.extend_from_slice(&self.sender_hardware_addr.bytes);
        // 发送方协议地址
        bytes.extend_from_slice(&self.sender_protocol_addr.bytes);
        // 目标硬件地址
        bytes.extend_from_slice(&self.target_hardware_addr.bytes);
        // 目标协议地址
        bytes.extend_from_slice(&self.target_protocol_addr.bytes);

        bytes
    }

    /// 判断是否为 Gratuitous ARP
    pub fn is_gratuitous(&self) -> bool {
        self.sender_protocol_addr == self.target_protocol_addr
    }
}

// ========== ARP 处理函数 ==========

/// 处理 ARP 报文
///
/// # 参数
/// - arp_pkt: 已解析的 ARP 报文
/// - verbose: 是否启用详细输出
///
/// # 返回
/// - Ok(()): 处理成功
/// - Err(String): 处理失败
pub fn handle_arp_packet(arp_pkt: &ArpPacket, verbose: bool) -> std::result::Result<(), String> {
    if verbose {
        println!("ARP报文:");
        println!("  操作: {:?}", arp_pkt.operation);
        println!("  发送方: MAC={}, IP={}",
            arp_pkt.sender_hardware_addr, arp_pkt.sender_protocol_addr);
        println!("  目标: MAC={}, IP={}",
            arp_pkt.target_hardware_addr, arp_pkt.target_protocol_addr);

        if arp_pkt.is_gratuitous() {
            println!("  [免费ARP]");
        }
    }

    // 处理 ARP 请求/响应
    match arp_pkt.operation {
        ArpOperation::Request => {
            // 检查是否为目标 IP（暂不实现响应生成）
            if verbose {
                println!("  收到ARP请求: {} -> {}",
                    arp_pkt.sender_protocol_addr,
                    arp_pkt.target_protocol_addr);
            }
        }
        ArpOperation::Reply => {
            if verbose {
                println!("  收到ARP响应: {} -> {}",
                    arp_pkt.sender_protocol_addr,
                    arp_pkt.sender_hardware_addr);
            }
        }
    }

    Ok(())
}
