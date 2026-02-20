// src/protocols/arp/mod.rs
//
// ARP（Address Resolution Protocol）地址解析协议实现
// 参考：RFC 826

use crate::common::{CoreError, Result};
use crate::protocols::{Packet, MacAddr, Ipv4Addr};
use crate::protocols::ethernet;
use std::collections::VecDeque;

// ARP 表模块
pub mod tables;

// 重新导出 ARP 表相关的公共类型
pub use tables::{
    ArpState, ArpConfig, ArpEntry, ArpCache, ArpKey
};

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
        for sender_hardware_byte in &mut sender_hardware_bytes {
            *sender_hardware_byte = match packet.read(1) {
                Some(data) => data[0],
                None => return Err(CoreError::parse_error("读取发送方硬件地址失败")),
            };
        }
        let sender_hardware_addr = MacAddr::new(sender_hardware_bytes);

        // 读取发送方协议地址
        let mut sender_protocol_bytes = [0u8; 4];
        for sender_protocol_byte in &mut sender_protocol_bytes {
            *sender_protocol_byte = match packet.read(1) {
                Some(data) => data[0],
                None => return Err(CoreError::parse_error("读取发送方协议地址失败")),
            };
        }
        let sender_protocol_addr = Ipv4Addr::from_bytes(sender_protocol_bytes);

        // 读取目标硬件地址
        let mut target_hardware_bytes = [0u8; 6];
        for target_hardware_byte in &mut target_hardware_bytes {
            *target_hardware_byte = match packet.read(1) {
                Some(data) => data[0],
                None => return Err(CoreError::parse_error("读取目标硬件地址失败")),
            };
        }
        let target_hardware_addr = MacAddr::new(target_hardware_bytes);

        // 读取目标协议地址
        let mut target_protocol_bytes = [0u8; 4];
        for target_protocol_byte in &mut target_protocol_bytes {
            *target_protocol_byte = match packet.read(1) {
                Some(data) => data[0],
                None => return Err(CoreError::parse_error("读取目标协议地址失败")),
            };
        }
        let target_protocol_addr = Ipv4Addr::from_bytes(target_protocol_bytes);

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

/// 处理已解析的 ArpPacket
///
/// 根据设计文档第 4.2 节的规范实现：
/// 1. 自动学习：无论什么类型的 ARP 报文，都更新缓存
/// 2. 判断响应：如果是发给本机的请求，构造响应
/// 3. 处理等待队列：如果是响应，检查并处理等待的请求
///
/// # 参数
/// - cache: 可变引用的 ARP 缓存
/// - ifindex: 网络接口索引
/// - packet: 已解析的 ARP 报文
/// - local_ips: 本机接口的 IP 地址列表
/// - local_mac: 本机接口的 MAC 地址
///
/// # 返回
/// - Ok(Some(reply_packet)): 需要发送的响应报文
/// - Ok(None): 不需要发送响应
/// - Err(CoreError): 处理失败
pub fn handle_arp_packet(
    cache: &mut ArpCache,
    ifindex: u32,
    packet: &ArpPacket,
    local_ips: &[Ipv4Addr],
    local_mac: MacAddr,
) -> Result<Option<ArpPacket>> {
    // 根据操作类型处理
    match packet.operation {
        ArpOperation::Request => {
            // 第一步：自动学习（更新缓存）
            // 对于ARP请求，学习发送方的MAC地址，状态设为Reachable
            cache.update_arp(
                ifindex,
                packet.sender_protocol_addr,
                packet.sender_hardware_addr,
                ArpState::Reachable,
            );

            // 第二步：检查目标 IP 是否是本机
            if local_ips.contains(&packet.target_protocol_addr) {
                // 需要响应
                let reply = ArpPacket::new(
                    ArpOperation::Reply,
                    local_mac,
                    packet.target_protocol_addr,  // 本机 IP
                    packet.sender_hardware_addr, // 目标 MAC = 请求的源 MAC
                    packet.sender_protocol_addr, // 目标 IP = 请求的源 IP
                );
                return Ok(Some(reply));
            }
            // 不是发给本机的请求，不响应
            Ok(None)
        }
        ArpOperation::Reply => {
            // 对于ARP响应，需要处理不同的状态转换场景
            if let Some(entry) = cache.lookup_mut_arp(ifindex, packet.sender_protocol_addr) {
                // 场景1：有条目存在
                if entry.state == ArpState::Incomplete {
                    // Incomplete -> Reachable：收到匹配的响应
                    entry.state = ArpState::Reachable;
                    entry.hardware_addr = packet.sender_hardware_addr;
                    entry.updated_at = std::time::Instant::now();
                    entry.confirmed_at = std::time::Instant::now();
                    entry.retry_count = 0;

                    // 清空等待队列
                    entry.take_pending();
                } else {
                    // 其他状态：自动学习，更新MAC地址和时间戳
                    entry.hardware_addr = packet.sender_hardware_addr;
                    entry.updated_at = std::time::Instant::now();
                    entry.confirmed_at = std::time::Instant::now();
                    entry.state = ArpState::Reachable;
                }
            } else {
                // 场景2：没有条目存在，创建新的Reachable条目（自动学习）
                cache.update_arp(
                    ifindex,
                    packet.sender_protocol_addr,
                    packet.sender_hardware_addr,
                    ArpState::Reachable,
                );
            }

            // 收到响应后不需要发送任何报文
            Ok(None)
        }
    }
}

/// 处理等待队列中的数据包
///
/// 当 ARP 响应到达后，处理所有等待发送的数据包
///
/// # 参数
/// - pending: 等待队列
///
/// # 返回
/// - 处理的数据包数量
pub fn process_pending_packets(pending: &mut VecDeque<Packet>) -> usize {
    let count = pending.len();
    pending.clear();
    count
}

/// ARP 处理结果
#[derive(Debug)]
pub enum ArpProcessResult {
    /// 不需要响应
    NoReply,
    /// 需要响应（封装好的以太网帧）
    Reply(Vec<u8>),
}

/// 将 ARP 报文封装为以太网帧
pub fn encapsulate_ethernet(
    arp_packet: &ArpPacket,
    dst_mac: MacAddr,
    src_mac: MacAddr,
) -> Vec<u8> {
    ethernet::build_ethernet_frame(dst_mac, src_mac, 0x0806, &arp_packet.to_bytes())
}

/// 处理ARP报文（使用 SystemContext）
///
/// 这是使用依赖注入模式的 ARP 处理接口，使用传入的 SystemContext
/// 而不是全局状态。
///
/// # 参数
/// - packet: 可变引用的 Packet（已去除以太网头部）
/// - eth_src: 原始以太网帧的源MAC地址（用于响应时的目标MAC）
/// - ifindex: 接口索引
/// - context: 系统上下文，包含接口和 ARP 缓存
/// - verbose: 是否启用详细输出
///
/// # 返回
/// - Ok(ArpProcessResult): 处理结果（NoReply 或 Reply(完整以太网帧)）
/// - Err(CoreError): 解析或处理失败
pub fn process_arp_packet_with_context(
    packet: &mut Packet,
    eth_src: MacAddr,
    ifindex: u32,
    context: &crate::context::SystemContext,
    verbose: bool,
) -> Result<ArpProcessResult> {
    // 1. 解析 ARP 报文
    let arp_pkt = ArpPacket::from_packet(packet)?;

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

    // 2. 获取接口信息（从 context.interfaces）
    let (local_mac, local_ip) = {
        let guard = context.interfaces.lock()
            .map_err(|e| CoreError::parse_error(format!("锁定接口管理器失败: {}", e)))?;
        let iface = guard.get_by_index(ifindex)
            .map_err(|e| CoreError::parse_error(format!("获取接口失败: {}", e)))?;
        // 锁在这里自动释放
        (iface.mac_addr, iface.ip_addr)
    };

    // 3. 获取 ARP 缓存（从 context.arp_cache）
    let mut cache = context.arp_cache.lock()
        .map_err(|e| CoreError::parse_error(format!("锁定 ARP 缓存失败: {}", e)))?;

    // 4. 调用核心处理逻辑
    let reply = handle_arp_packet(&mut cache, ifindex, &arp_pkt, &[local_ip], local_mac)?;

    // 5. 封装响应（如有）
    match reply {
        Some(reply_arp) => {
            if verbose {
                println!("  发送ARP响应: {} -> {}",
                    reply_arp.sender_protocol_addr,
                    reply_arp.target_protocol_addr);
            }
            let frame = ethernet::build_ethernet_frame(eth_src, local_mac, 0x0806, &reply_arp.to_bytes());
            Ok(ArpProcessResult::Reply(frame))
        }
        None => Ok(ArpProcessResult::NoReply),
    }
}
