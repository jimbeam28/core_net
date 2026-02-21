// src/protocols/arp/mod.rs
//
// ARP（Address Resolution Protocol）地址解析协议实现
// 参考：RFC 826

use crate::common::{CoreError, Result};
use crate::protocols::{Packet, MacAddr, Ipv4Addr};
use crate::protocols::ethernet;

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

        // 验证硬件类型（必须是以太网）
        if hardware_type != Self::ARPHRD_ETHER {
            return Err(CoreError::invalid_packet(
                format!("不支持的硬件类型：{}（仅支持以太网类型1）", hardware_type)
            ));
        }

        // 验证协议类型（必须是IPv4）
        if protocol_type != Self::ETH_P_IP {
            return Err(CoreError::invalid_packet(
                format!("不支持的协议类型：0x{:04x}（仅支持IPv4协议0x0800）", protocol_type)
            ));
        }

        // 验证硬件地址长度（必须是6）
        if hardware_addr_len != 6 {
            return Err(CoreError::invalid_packet(
                format!("无效的硬件地址长度：{}（以太网MAC地址应为6字节）", hardware_addr_len)
            ));
        }

        // 验证协议地址长度（必须是4）
        if protocol_addr_len != 4 {
            return Err(CoreError::invalid_packet(
                format!("无效的协议地址长度：{}（IPv4地址应为4字节）", protocol_addr_len)
            ));
        }

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

/// ARP 处理返回结果
#[derive(Debug)]
pub struct ArpHandleResult {
    /// 需要发送的ARP响应报文
    pub reply: Option<ArpPacket>,
    /// 等待队列中的数据包（ARP解析完成后需要发送）
    pub pending_packets: Vec<Packet>,
}

impl ArpHandleResult {
    pub fn no_reply() -> Self {
        Self {
            reply: None,
            pending_packets: Vec::new(),
        }
    }

    pub fn with_reply(reply: ArpPacket) -> Self {
        Self {
            reply: Some(reply),
            pending_packets: Vec::new(),
        }
    }
}

/// 检查IP地址冲突
///
/// 当收到Gratuitous ARP时，检查是否与其他条目的MAC地址冲突
///
/// # 参数
/// - cache: ARP缓存
/// - ifindex: 接口索引
/// - ip: IP地址
/// - mac: 新的MAC地址
///
/// # 返回
/// - Ok(()): 没有冲突
/// - Err(CoreError): 检测到IP冲突
fn check_ip_conflict(
    cache: &ArpCache,
    ifindex: u32,
    ip: Ipv4Addr,
    mac: MacAddr,
) -> Result<()> {
    if let Some(existing) = cache.lookup_arp(ifindex, ip) {
        // 如果已存在相同IP但不同MAC的条目，报告冲突
        if existing.hardware_addr != mac {
            return Err(CoreError::ip_conflict(
                ip.to_string(),
                mac.to_string(),
                existing.hardware_addr.to_string(),
            ));
        }
    }
    Ok(())
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
/// - Ok(ArpHandleResult): 包含响应报文和等待队列数据包
/// - Err(CoreError): 处理失败
pub fn handle_arp_packet(
    cache: &mut ArpCache,
    ifindex: u32,
    packet: &ArpPacket,
    local_ips: &[Ipv4Addr],
    local_mac: MacAddr,
) -> Result<ArpHandleResult> {
    // 根据操作类型处理
    match packet.operation {
        ArpOperation::Request => {
            // 检查是否为Gratuitous ARP（免费ARP）
            let is_garp = packet.is_gratuitous();

            // 如果是GARP，检查IP冲突
            if is_garp {
                // GARP特征：SPA == TPA
                check_ip_conflict(
                    cache,
                    ifindex,
                    packet.sender_protocol_addr,
                    packet.sender_hardware_addr,
                )?;
            }

            // 第一步：自动学习（更新缓存）
            // 对于ARP请求，学习发送方的MAC地址，状态设为Reachable
            cache.update_arp(
                ifindex,
                packet.sender_protocol_addr,
                packet.sender_hardware_addr,
                ArpState::Reachable,
            );

            // 第二步：检查目标 IP 是否是本机
            // 对于普通ARP请求：TPA是本机IP时响应
            // 对于GARP请求：SPA=TPA且是本机IP时也需要响应
            let should_reply = if is_garp {
                // GARP: SPA == TPA，如果TPA是本机IP则需要响应
                local_ips.contains(&packet.target_protocol_addr)
            } else {
                // 普通ARP请求：TPA是本机IP时响应
                local_ips.contains(&packet.target_protocol_addr)
            };

            if should_reply {
                // 需要响应
                let reply = ArpPacket::new(
                    ArpOperation::Reply,
                    local_mac,
                    packet.target_protocol_addr,  // 本机 IP
                    packet.sender_hardware_addr, // 目标 MAC = 请求的源 MAC
                    packet.sender_protocol_addr, // 目标 IP = 请求的源 IP
                );
                return Ok(ArpHandleResult::with_reply(reply));
            }
            // 不是发给本机的请求，不响应
            Ok(ArpHandleResult::no_reply())
        }
        ArpOperation::Reply => {
            let mut pending_packets = Vec::new();

            // 检查是否为Gratuitous ARP（免费ARP）
            let is_garp = packet.is_gratuitous();

            // 如果是GARP，检查IP冲突
            if is_garp {
                check_ip_conflict(
                    cache,
                    ifindex,
                    packet.sender_protocol_addr,
                    packet.sender_hardware_addr,
                )?;
            }

            // 对于ARP响应，需要处理不同的状态转换场景
            if let Some(entry) = cache.lookup_mut_arp(ifindex, packet.sender_protocol_addr) {
                // 场景1：有条目存在
                if matches!(entry.state, ArpState::Incomplete | ArpState::Delay | ArpState::Probe) {
                    // Incomplete/Delay/Probe -> Reachable：收到匹配的响应
                    let _old_state = entry.state;
                    entry.state = ArpState::Reachable;
                    entry.hardware_addr = packet.sender_hardware_addr;
                    entry.updated_at = std::time::Instant::now();
                    entry.confirmed_at = std::time::Instant::now();
                    entry.retry_count = 0;

                    // 取出等待队列中的数据包（如果有）
                    pending_packets = entry.take_pending().into_iter().collect();

                    if !pending_packets.is_empty() {
                        // 记录：从 {old_state:?} 状态恢复，有 {count} 个数据包待发送
                    }
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

            // 收到响应后不需要发送ARP响应，但可能需要发送等待队列中的数据包
            Ok(ArpHandleResult {
                reply: None,
                pending_packets,
            })
        }
    }
}

/// 将 ARP 报文封装为以太网帧
///
/// # 参数
/// - arp_packet: ARP 报文
/// - dst_mac: 目标 MAC 地址
/// - src_mac: 源 MAC 地址
///
/// # 返回
/// - Vec<u8>: 封装后的以太网帧（包含以太网头部和 ARP 报文）
pub fn encapsulate_ethernet(
    arp_packet: &ArpPacket,
    dst_mac: MacAddr,
    src_mac: MacAddr,
) -> Vec<u8> {
    ethernet::build_ethernet_frame(dst_mac, src_mac, 0x0806, &arp_packet.to_bytes())
}

/// ARP 处理结果（用于 process_arp_packet_with_context）
#[derive(Debug)]
pub enum ArpProcessResult {
    /// 不需要响应
    NoReply,
    /// 需要响应（封装好的以太网帧）
    Reply(Vec<u8>),
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
    let handle_result = handle_arp_packet(&mut cache, ifindex, &arp_pkt, &[local_ip], local_mac)?;

    // 5. 处理等待队列中的数据包（如果有）
    // 将等待的报文封装到以太网帧并放入接口的txq
    if !handle_result.pending_packets.is_empty() {
        // 获取目标MAC地址（ARP响应中的发送方MAC）
        let target_mac = arp_pkt.sender_hardware_addr;

        // 尝试将等待的报文放入接口的txq
        if let Ok(mut interfaces) = context.interfaces.lock() {
            if let Ok(iface) = interfaces.get_by_index_mut(ifindex) {
                for pending_pkt in handle_result.pending_packets {
                    // 封装为以太网帧
                    let frame = ethernet::build_ethernet_frame(
                        target_mac,
                        local_mac,
                        0x0800, // IP协议
                        pending_pkt.as_slice()
                    );
                    let out_packet = Packet::from_bytes(frame);

                    // 尝试放入发送队列
                    if let Err(_) = iface.txq.enqueue(out_packet) {
                        if verbose {
                            println!("  警告: TxQ已满，等待的数据包丢失");
                        }
                    } else if verbose {
                        println!("  发送等待队列中的数据包到 {}", target_mac);
                    }
                }
            }
        }
    }

    // 6. 封装ARP响应（如有）
    match handle_result.reply {
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

// ========== 主动ARP解析功能 ==========

/// ARP解析结果
#[derive(Debug)]
pub enum ArpResolveResult {
    /// MAC地址已解析，可以直接发送
    Resolved(MacAddr),
    /// 正在解析中，数据包已加入等待队列
    Pending,
    /// 解析失败（如特殊IP地址）
    Failed(CoreError),
}

/// 解析目标IP地址的MAC地址
///
/// 根据设计文档第5.1节的规范实现：
/// 1. 查找目标IP的ARP缓存
/// 2. 如果状态为 REACHABLE：直接获取MAC地址发送
/// 3. 如果状态为 NONE 或 STALE：创建/更新条目为 INCOMPLETE，发送ARP请求，将数据包加入等待队列
/// 4. 如果状态为 INCOMPLETE：将数据包加入等待队列
///
/// # 参数
/// - context: 系统上下文
/// - ifindex: 接口索引
/// - target_ip: 目标IP地址
/// - packet: 需要发送的数据包（用于加入等待队列）
/// - verbose: 是否启用详细输出
///
/// # 返回
/// - ArpResolveResult: 解析结果
pub fn resolve_ip(
    context: &crate::context::SystemContext,
    ifindex: u32,
    target_ip: Ipv4Addr,
    packet: Packet,
    verbose: bool,
) -> ArpResolveResult {
    // 第一步：检查特殊IP地址（拒绝解析）
    if target_ip.is_unspecified() || target_ip.is_broadcast() || target_ip.is_multicast() {
        return ArpResolveResult::Failed(CoreError::invalid_packet(
            format!("拒绝解析特殊IP地址: {}", target_ip)
        ));
    }

    // 获取ARP缓存
    let mut cache = match context.arp_cache.lock() {
        Ok(guard) => guard,
        Err(e) => return ArpResolveResult::Failed(CoreError::parse_error(format!("锁定ARP缓存失败: {}", e))),
    };

    // 检查现有条目
    if let Some(entry) = cache.lookup_arp(ifindex, target_ip) {
        match entry.state {
            ArpState::Reachable => {
                // 已解析，直接返回MAC地址
                return ArpResolveResult::Resolved(entry.hardware_addr);
            }
            ArpState::Incomplete | ArpState::Delay | ArpState::Probe => {
                // 正在解析中/延迟探测/探测中：将数据包加入等待队列
                if let Err(e) = cache.add_pending_packet(ifindex, target_ip, packet) {
                    return ArpResolveResult::Failed(e);
                }
                return ArpResolveResult::Pending;
            }
            ArpState::Stale => {
                // 陈旧状态：按照设计文档，需要将数据包加入等待队列
                // 1. 触发Stale -> Delay转换
                cache.mark_used(ifindex, target_ip);

                // 2. 将数据包加入等待队列（Delay状态支持等待队列）
                if let Err(e) = cache.add_pending_packet(ifindex, target_ip, packet) {
                    return ArpResolveResult::Failed(e);
                }

                if verbose {
                    println!("ARP条目陈旧，转为Delay状态，数据包加入等待队列");
                }

                return ArpResolveResult::Pending;
            }
            ArpState::None => {
                // None状态：转为Incomplete并开始解析
                cache.update_arp(ifindex, target_ip, MacAddr::zero(), ArpState::Incomplete);

                // 将数据包加入等待队列
                if let Err(e) = cache.add_pending_packet(ifindex, target_ip, packet) {
                    return ArpResolveResult::Failed(e);
                }

                // 释放缓存锁
                drop(cache);

                // 发送ARP请求
                if let Err(e) = send_arp_request(context, ifindex, target_ip, verbose) {
                    return ArpResolveResult::Failed(e);
                }

                return ArpResolveResult::Pending;
            }
        }
    }

    // 没有条目，创建None状态条目（初始状态）
    cache.update_arp(ifindex, target_ip, MacAddr::zero(), ArpState::None);

    // 立即转为Incomplete状态
    cache.update_arp(ifindex, target_ip, MacAddr::zero(), ArpState::Incomplete);

    // 将数据包加入等待队列
    if let Err(e) = cache.add_pending_packet(ifindex, target_ip, packet) {
        return ArpResolveResult::Failed(e);
    }

    // 释放缓存锁
    drop(cache);

    // 发送ARP请求
    if let Err(e) = send_arp_request(context, ifindex, target_ip, verbose) {
        return ArpResolveResult::Failed(e);
    }

    ArpResolveResult::Pending
}

/// 处理ARP定时器
///
/// 遍历ARP缓存中的所有条目，处理到期的定时器：
/// - INCOMPLETE: 重传ARP请求
/// - REACHABLE: 转为STALE状态
/// - DELAY: 转为PROBE状态，发送探测请求
/// - PROBE: 重传探测请求
///
/// # 参数
/// - context: 系统上下文
/// - verbose: 是否启用详细输出
///
/// # 返回
/// 处理的条目数量
pub fn process_arp_timers(
    context: &crate::context::SystemContext,
    verbose: bool,
) -> usize {
    // 获取所有需要处理的条目（在一次锁操作中完成）
    let pending_requests: Vec<(u32, Ipv4Addr, bool)> = {
        let mut cache = match context.arp_cache.lock() {
            Ok(guard) => guard,
            Err(_) => return 0,
        };

        // 获取需要发送请求的条目列表：(ifindex, ip, is_probe)
        cache.get_pending_requests()
    };

    let mut processed_count = 0;

    // 处理每个需要发送ARP请求的条目
    for (ifindex, target_ip, is_probe) in pending_requests {
        if let Err(e) = send_arp_request(context, ifindex, target_ip, verbose) {
            if verbose {
                println!("ARP定时器: 发送{}请求失败 {}: {}",
                    if is_probe { "探测" } else { "解析" },
                    target_ip, e);
            }
        } else {
            processed_count += 1;
            if verbose {
                println!("ARP定时器: 发送{}请求到 {}",
                    if is_probe { "探测" } else { "解析" },
                    target_ip);
            }
        }
    }

    processed_count
}

/// 发送ARP请求
///
/// 构造并发送ARP请求报文到指定接口
///
/// # 参数
/// - context: 系统上下文
/// - ifindex: 接口索引
/// - target_ip: 目标IP地址
/// - verbose: 是否启用详细输出
///
/// # 返回
/// - Ok(()): 发送成功
/// - Err(CoreError): 发送失败
pub fn send_arp_request(
    context: &crate::context::SystemContext,
    ifindex: u32,
    target_ip: Ipv4Addr,
    verbose: bool,
) -> Result<()> {
    // 获取接口信息
    let (local_mac, local_ip) = {
        let guard = context.interfaces.lock()
            .map_err(|e| CoreError::parse_error(format!("锁定接口管理器失败: {}", e)))?;
        let iface = guard.get_by_index(ifindex)
            .map_err(|e| CoreError::parse_error(format!("获取接口失败: {}", e)))?;
        (iface.mac_addr, iface.ip_addr)
    };

    // 构造ARP请求报文
    let arp_request = ArpPacket::new(
        ArpOperation::Request,
        local_mac,
        local_ip,
        MacAddr::zero(), // 目标MAC在请求时未知
        target_ip,
    );

    // 封装为以太网帧（广播）
    let frame = ethernet::build_ethernet_frame(
        MacAddr::broadcast(),
        local_mac,
        0x0806, // ARP协议
        &arp_request.to_bytes()
    );

    // 放入接口的发送队列
    let mut interfaces = context.interfaces.lock()
        .map_err(|e| CoreError::parse_error(format!("锁定接口管理器失败: {}", e)))?;
    let iface = interfaces.get_by_index_mut(ifindex)
        .map_err(|e| CoreError::parse_error(format!("获取接口失败: {}", e)))?;

    let out_packet = Packet::from_bytes(frame);
    match iface.txq.enqueue(out_packet) {
        Ok(_) => {
            if verbose {
                println!("发送ARP请求: {} -> {} (广播)", local_ip, target_ip);
            }
            Ok(())
        }
        Err(_) => Err(CoreError::invalid_packet("TxQ已满，ARP请求发送失败")),
    }
}
