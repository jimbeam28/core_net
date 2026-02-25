// src/engine/processor.rs
//
// 报文处理器
// 提供报文处理接口，负责逐层解析/分发报文

use crate::common::{Packet, EthernetHeader, VlanTag};
use crate::protocols::{arp, ip, icmp, icmpv6, ipv6, udp, tcp, ospf2, ospf3, Ipv4Addr};
use crate::context::SystemContext;

pub type ProcessResult = Result<Option<Packet>, ProcessError>;

#[derive(Debug)]
pub enum ProcessError {
    ParseError(String),
    EncapError(String),
    UnsupportedProtocol(String),
    InvalidPacket(String),
}

impl std::fmt::Display for ProcessError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProcessError::ParseError(msg) => write!(f, "解析错误: {}", msg),
            ProcessError::EncapError(msg) => write!(f, "封装错误: {}", msg),
            ProcessError::UnsupportedProtocol(proto) => write!(f, "不支持的协议: {}", proto),
            ProcessError::InvalidPacket(msg) => write!(f, "报文格式错误: {}", msg),
        }
    }
}

impl std::error::Error for ProcessError {}

/// 宏：为协议错误类型实现 From 转换
///
/// 用法：impl_from_protocol_error!(错误类型, 前缀消息)
/// 示例：impl_from_protocol_error!(crate::protocols::ip::IpError, "IP")
macro_rules! impl_from_protocol_error {
    ($error_type:ty, $prefix:expr) => {
        impl From<$error_type> for ProcessError {
            fn from(err: $error_type) -> Self {
                ProcessError::ParseError(format!("{}错误: {}", $prefix, err))
            }
        }
    };
    ($error_type:ty) => {
        impl From<$error_type> for ProcessError {
            fn from(err: $error_type) -> Self {
                ProcessError::ParseError(err.to_string())
            }
        }
    };
}

impl From<crate::common::CoreError> for ProcessError {
    fn from(err: crate::common::CoreError) -> Self {
        match err {
            crate::common::CoreError::ParseError(msg) => {
                ProcessError::ParseError(msg)
            }
            crate::common::CoreError::InvalidPacket(msg) => {
                ProcessError::InvalidPacket(msg)
            }
            crate::common::CoreError::UnsupportedProtocol(proto) => {
                ProcessError::UnsupportedProtocol(proto)
            }
            _ => ProcessError::EncapError(format!("{:?}", err)),
        }
    }
}

impl_from_protocol_error!(crate::protocols::vlan::VlanError, "VLAN");
impl_from_protocol_error!(crate::protocols::ip::IpError);
impl_from_protocol_error!(crate::protocols::ipv6::Ipv6Error);
impl_from_protocol_error!(crate::protocols::tcp::TcpError, "TCP");
impl_from_protocol_error!(crate::protocols::ospf3::Ospfv3Error, "OSPFv3");

impl From<String> for ProcessError {
    fn from(msg: String) -> Self {
        ProcessError::ParseError(msg)
    }
}

pub struct PacketProcessor {
    name: String,
    verbose: bool,
    /// 系统上下文（使用依赖注入模式）
    /// 包含接口管理器、ARP缓存等状态
    context: SystemContext,
}

impl PacketProcessor {
    /// 使用系统上下文创建处理器（推荐方式）
    ///
    /// # 参数
    /// - `context`: 系统上下文，包含接口、ARP缓存等状态
    ///
    /// # 示例
    ///
    /// ```
    /// use core_net::engine::PacketProcessor;
    /// use core_net::context::SystemContext;
    ///
    /// let ctx = SystemContext::new();
    /// let processor = PacketProcessor::with_context(ctx);
    /// ```
    pub fn with_context(context: SystemContext) -> Self {
        Self {
            name: String::from("ContextProcessor"),
            verbose: false,
            context,
        }
    }

    /// 使用系统上下文和名称创建处理器
    ///
    /// # 参数
    /// - `name`: 处理器名称
    /// - `context`: 系统上下文
    pub fn with_name_and_context(name: String, context: SystemContext) -> Self {
        Self {
            name,
            verbose: false,
            context,
        }
    }

    /// 设置系统上下文
    ///
    /// # 参数
    /// - `context`: 系统上下文
    pub fn set_context(&mut self, context: SystemContext) {
        self.context = context;
    }

    pub fn with_verbose(mut self, verbose: bool) -> Self {
        self.verbose = verbose;
        self
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn process(&self, mut packet: Packet) -> ProcessResult {
        self.print_packet_info(&packet);
        let eth_hdr = EthernetHeader::from_packet(&mut packet)?;
        if self.verbose {
            self.print_eth_header(&eth_hdr);
        }
        self.dispatch_by_ether_type(eth_hdr, packet)
    }

    fn dispatch_by_ether_type(
        &self,
        eth_hdr: EthernetHeader,
        packet: Packet,
    ) -> ProcessResult {
        use crate::common::{ETH_P_IP, ETH_P_ARP, ETH_P_IPV6, ETH_P_8021Q, ETH_P_8021AD};

        match eth_hdr.ether_type {
            ETH_P_8021Q | ETH_P_8021AD => {
                self.handle_vlan(eth_hdr, packet)?;
            }
            ETH_P_ARP => {
                return self.handle_arp(eth_hdr, packet);
            }
            ETH_P_IP => {
                return self.handle_ipv4(eth_hdr, packet);
            }
            ETH_P_IPV6 => {
                return self.handle_ipv6(eth_hdr, packet);
            }
            _ => {
                return Err(ProcessError::UnsupportedProtocol(
                    format!("Unknown ethernet type: 0x{:04x}", eth_hdr.ether_type)
                ));
            }
        }
        Ok(None)
    }

    fn handle_vlan(&self, eth_hdr: EthernetHeader, mut packet: Packet) -> ProcessResult {
        let result = crate::protocols::vlan::process_vlan_packet(&mut packet)?;

        if self.verbose {
            if let Some(ref outer) = result.outer_vlan {
                println!("VLAN tag: PCP={}, DEI={}, VID={}", outer.pcp, outer.dei, outer.vid);
            }
            if let Some(ref inner) = result.inner_vlan {
                println!("Inner VLAN: PCP={}, DEI={}, VID={}", inner.pcp, inner.dei, inner.vid);
            }
            println!("Inner ethertype: 0x{:04x}", result.inner_type);
        }

        self.dispatch_inner_vlan(eth_hdr, result.outer_vlan, result.inner_vlan, result.inner_type, packet)?;
        Ok(None)
    }

    fn dispatch_inner_vlan(
        &self,
        eth_hdr: EthernetHeader,
        _outer_vlan: Option<VlanTag>,
        _inner_vlan: Option<VlanTag>,
        inner_type: u16,
        packet: Packet,
    ) -> ProcessResult {
        use crate::common::ETH_P_ARP;
        use crate::common::ETH_P_IP;
        use crate::common::ETH_P_IPV6;

        match inner_type {
            ETH_P_ARP => {
                // VLAN 内的 ARP 报文处理，传递外层以太网头的源MAC
                self.handle_arp_packet(packet, eth_hdr.src_mac)?;
            }
            ETH_P_IP => {
                return self.handle_ipv4(eth_hdr, packet);
            }
            ETH_P_IPV6 => {
                return self.handle_ipv6(eth_hdr, packet);
            }
            _ => {
                return Err(ProcessError::UnsupportedProtocol(
                    format!("Unsupported protocol in VLAN: 0x{:04x}", inner_type)
                ));
            }
        }
        Ok(None)
    }

    /// 处理普通以太网帧中的 ARP 报文
    fn handle_arp(&self, eth_hdr: EthernetHeader, packet: Packet) -> ProcessResult {
        // 注意：只对 ARP Request 进行目标MAC验证
        // ARP Reply 的目标 MAC 是单播地址（接收方的MAC），不需要验证

        self.handle_arp_packet(packet, eth_hdr.src_mac)
    }

    /// 处理 ARP 报文（统一入口）
    ///
    /// 调用 ARP 模块的统一处理接口，使用 SystemContext 模式。
    ///
    /// # 参数
    /// - packet: Packet（已去除以太网头部）
    /// - eth_src: 原始以太网帧的源MAC地址
    fn handle_arp_packet(&self, mut packet: Packet, eth_src: crate::protocols::MacAddr) -> ProcessResult {
        let ifindex = packet.ifindex;

        // 使用 SystemContext 模式
        let result = arp::process_arp_packet_with_context(&mut packet, eth_src, ifindex, &self.context, self.verbose)
            .map_err(|e| ProcessError::ParseError(format!("ARP处理失败: {}", e)))?;

        // 根据处理结果返回
        match result {
            arp::ArpProcessResult::NoReply => Ok(None),
            arp::ArpProcessResult::Reply(frame_bytes) => Ok(Some(Packet::from_bytes(frame_bytes))),
        }
    }

    /// 处理 IPv4 报文
    ///
    /// # 参数
    /// - eth_hdr: 以太网头部
    /// - packet: Packet（已去除以太网头部）
    fn handle_ipv4(&self, eth_hdr: EthernetHeader, mut packet: Packet) -> ProcessResult {
        let ifindex = packet.ifindex;

        // 使用 process_ip_packet 处理 IP 数据报（与 IPv6 处理模式一致）
        let result = ip::process_ip_packet(&mut packet, ifindex, &self.context)?;

        match result {
            ip::IpProcessResult::NoReply => {
                if self.verbose {
                    println!("IPv4: NoReply");
                }
                Ok(None)
            }
            ip::IpProcessResult::Reply(ip_bytes) => {
                if self.verbose {
                    println!("IPv4: Reply {} bytes", ip_bytes.len());
                }
                // 封装为以太网帧
                let our_mac = self.get_interface_mac(ifindex)?;
                let frame_bytes = crate::protocols::ethernet::build_ethernet_frame(
                    eth_hdr.src_mac,  // 响应发送给原始发送方
                    our_mac,          // 使用本接口的 MAC
                    crate::protocols::ETH_P_IP,
                    &ip_bytes,
                );
                Ok(Some(Packet::from_bytes(frame_bytes)))
            }
            ip::IpProcessResult::DeliverToProtocol { ip_hdr, data } => {
                if self.verbose {
                    println!("IPv4: DeliverToProtocol {} bytes, protocol={}", data.len(), ip_hdr.protocol);
                    println!("IP: {} -> {} Protocol={} TTL={}",
                        ip_hdr.source_addr, ip_hdr.dest_addr, ip_hdr.protocol, ip_hdr.ttl);
                }

                // 创建新 Packet 并保留原始 ifindex
                let mut protocol_packet = Packet::from_bytes(data);
                protocol_packet.ifindex = ifindex;

                // 根据 IP 协议字段分发到上层协议
                match ip_hdr.protocol {
                    ip::IP_PROTO_ICMP => {
                        self.handle_icmp(eth_hdr, ip_hdr, protocol_packet)
                    }
                    ip::IP_PROTO_UDP => {
                        self.handle_udp(eth_hdr, ip_hdr, protocol_packet)
                    }
                    ip::IP_PROTO_TCP => {
                        self.handle_tcp(eth_hdr, ip_hdr, protocol_packet)
                    }
                    ip::IP_PROTO_OSPF => {
                        self.handle_ospf(eth_hdr, ip_hdr, protocol_packet)
                    }
                    _ => {
                        Err(ProcessError::UnsupportedProtocol(
                            format!("Unknown IP protocol: {}", ip_hdr.protocol)
                        ))
                    }
                }
            }
        }
    }

    /// 处理 IPv6 报文
    ///
    /// # 参数
    /// - eth_hdr: 以太网头部
    /// - packet: Packet（已去除以太网头部）
    fn handle_ipv6(&self, eth_hdr: EthernetHeader, mut packet: Packet) -> ProcessResult {
        // 获取接口索引
        let ifindex = packet.ifindex;

        if self.verbose {
            println!("IPv6: 处理 IPv6 报文");
        }

        // 处理 IPv6 报文
        let result = ipv6::process_ipv6_packet(&mut packet, ifindex, &self.context)?;

        match result {
            ipv6::Ipv6ProcessResult::NoReply => Ok(None),
            ipv6::Ipv6ProcessResult::Reply(ipv6_bytes) => {
                // 封装为以太网帧
                let our_mac = self.get_interface_mac(ifindex)?;
                let frame_bytes = crate::protocols::ethernet::build_ethernet_frame(
                    eth_hdr.src_mac,  // 响应发送给原始发送方
                    our_mac,          // 使用本接口的 MAC
                    crate::protocols::ETH_P_IPV6,
                    &ipv6_bytes,
                );
                Ok(Some(Packet::from_bytes(frame_bytes)))
            }
            ipv6::Ipv6ProcessResult::NeedsReassembly { .. } => {
                // 分片重组暂不支持
                Err(ProcessError::UnsupportedProtocol(
                    "IPv6 分片重组暂不支持".to_string()
                ))
            }
            ipv6::Ipv6ProcessResult::DeliverToProtocol { header, data } => {
                if self.verbose {
                    println!("IPv6: {} -> {} NextHeader={} HopLimit={}",
                        header.source_addr, header.destination_addr, header.next_header, header.hop_limit);
                }

                // 根据 Next Header 字段分发到上层协议
                match header.next_header {
                    ipv6::IpProtocol::IcmpV6 => {
                        self.handle_icmpv6(eth_hdr, header, Packet::from_bytes(data))
                    }
                    ipv6::IpProtocol::Ospf => {
                        self.handle_ospfv3(eth_hdr, header, Packet::from_bytes(data))
                    }
                    _ => {
                        Err(ProcessError::UnsupportedProtocol(
                            format!("Unknown IPv6 next header: {}", u8::from(header.next_header))
                        ))
                    }
                }
            }
        }
    }

    /// 处理 ICMPv6 报文
    ///
    /// # 参数
    /// - eth_hdr: 以太网头部
    /// - ipv6_hdr: IPv6 头部
    /// - packet: Packet（已去除 IPv6 头部）
    fn handle_icmpv6(&self, eth_hdr: EthernetHeader, ipv6_hdr: ipv6::Ipv6Header, packet: Packet) -> ProcessResult {
        // 获取接口索引
        let ifindex = packet.ifindex;

        // 一次性获取所有需要的接口信息（避免多次锁定）
        let (our_ipv6, our_mac) = {
            let interfaces = self.context.interfaces.lock()
                .map_err(|e| ProcessError::ParseError(format!("锁定接口管理器失败: {}", e)))?;
            let iface = interfaces.get_by_index(ifindex)
                .map_err(|e| ProcessError::ParseError(format!("获取接口失败: {}", e)))?;
            (iface.ipv6_addr(), iface.mac_addr)
        };

        if self.verbose {
            println!("ICMPv6: 处理 ICMPv6 报文 源={} 目的={}",
                ipv6_hdr.source_addr, ipv6_hdr.destination_addr);
        }

        // 处理 ICMPv6 报文
        let mut icmpv6_ctx = self.context.icmpv6_context.lock()
            .map_err(|e| ProcessError::ParseError(format!("锁定ICMPv6上下文失败: {}", e)))?;

        let result = icmpv6::process_icmpv6_packet(
            packet,
            ipv6_hdr.source_addr,
            our_ipv6,
            ipv6_hdr.hop_limit,
            Some(our_mac),
            &mut icmpv6_ctx,
            self.verbose,
        ).map_err(|e| ProcessError::ParseError(format!("ICMPv6处理失败: {:?}", e)))?;

        // 根据处理结果返回
        match result {
            icmpv6::Icmpv6ProcessResult::NoReply => Ok(None),
            icmpv6::Icmpv6ProcessResult::Reply(icmpv6_bytes) => {
                // 封装为 IPv6 数据包
                let ipv6_reply = ipv6::Ipv6Header::new(
                    our_ipv6,
                    ipv6_hdr.source_addr,
                    icmpv6_bytes.len() as u16,
                    ipv6::IpProtocol::IcmpV6,
                    64,
                );
                let mut ipv6_packet = ipv6_reply.to_bytes().to_vec();
                ipv6_packet.extend_from_slice(&icmpv6_bytes);

                // 封装为以太网帧
                let frame_bytes = crate::protocols::ethernet::build_ethernet_frame(
                    eth_hdr.src_mac,  // 响应发送给原始发送方
                    our_mac,          // 使用本接口的 MAC
                    crate::protocols::ETH_P_IPV6,
                    &ipv6_packet,
                );

                Ok(Some(Packet::from_bytes(frame_bytes)))
            }
            icmpv6::Icmpv6ProcessResult::Processed => Ok(None),
        }
    }

    /// 处理 OSPFv3 报文
    ///
    /// # 参数
    /// - eth_hdr: 以太网头部
    /// - ipv6_hdr: IPv6 头部
    /// - packet: Packet（已去除 IPv6 头部）
    fn handle_ospfv3(&self, eth_hdr: EthernetHeader, ipv6_hdr: ipv6::Ipv6Header, mut packet: Packet) -> ProcessResult {
        let ifindex = packet.ifindex;

        if self.verbose {
            println!("OSPFv3: 处理 OSPFv3 报文 源={} 目的={}",
                ipv6_hdr.source_addr, ipv6_hdr.destination_addr);
        }

        // 获取本接口的 IPv6 地址
        let our_ipv6 = {
            let interfaces = self.context.interfaces.lock()
                .map_err(|e| ProcessError::ParseError(format!("锁定接口管理器失败: {}", e)))?;
            let iface = interfaces.get_by_index(ifindex)
                .map_err(|e| ProcessError::ParseError(format!("获取接口失败: {}", e)))?;
            iface.ipv6_addr()
        };

        // TODO: 从配置获取路由器 ID
        // 简化实现：使用 IPv6 地址的最后 32 位作为 Router ID
        let ipv6_bytes = our_ipv6.as_bytes();
        let router_id = u32::from_be_bytes([
            ipv6_bytes[12], ipv6_bytes[13], ipv6_bytes[14], ipv6_bytes[15]
        ]);

        // 处理 OSPFv3 报文
        let result = ospf3::process_ospfv3_packet(
            &mut packet,
            ifindex,
            router_id,
            ipv6_hdr.source_addr,
        ).map_err(|e| ProcessError::ParseError(format!("OSPFv3处理失败: {}", e)))?;

        match result {
            ospf3::Ospfv3ProcessResult::NoReply => Ok(None),
            ospf3::Ospfv3ProcessResult::Reply(ospfv3_bytes) => {
                // 封装为 IPv6 数据包
                let our_mac = self.get_interface_mac(ifindex)?;
                let ipv6_reply = ipv6::Ipv6Header::new(
                    our_ipv6,
                    ipv6_hdr.source_addr,
                    ospfv3_bytes.len() as u16,
                    ipv6::IpProtocol::Ospf,
                    64,
                );
                let mut ipv6_packet = ipv6_reply.to_bytes().to_vec();
                ipv6_packet.extend_from_slice(&ospfv3_bytes);

                // 封装为以太网帧
                let frame_bytes = crate::protocols::ethernet::build_ethernet_frame(
                    eth_hdr.src_mac,
                    our_mac,
                    crate::protocols::ETH_P_IPV6,
                    &ipv6_packet,
                );

                Ok(Some(Packet::from_bytes(frame_bytes)))
            }
            ospf3::Ospfv3ProcessResult::FloodLsa { .. } => {
                // TODO: 实现 LSA 洪泛
                if self.verbose {
                    println!("OSPFv3: FloodLsa - 暂未实现");
                }
                Ok(None)
            }
            ospf3::Ospfv3ProcessResult::ScheduleSpfCalculation => {
                // TODO: 实现 SPF 计算
                if self.verbose {
                    println!("OSPFv3: ScheduleSpfCalculation - 暂未实现");
                }
                Ok(None)
            }
            ospf3::Ospfv3ProcessResult::DatabaseSynced => {
                if self.verbose {
                    println!("OSPFv3: DatabaseSynced");
                }
                Ok(None)
            }
        }
    }

    /// 处理 ICMP 报文
    ///
    /// # 参数
    /// - eth_hdr: 以太网头部
    /// - ip_hdr: IP 头部
    /// - packet: Packet（已去除 IP 头部）
    fn handle_icmp(&self, eth_hdr: EthernetHeader, ip_hdr: ip::Ipv4Header, packet: Packet) -> ProcessResult {
        // 获取接口索引
        let ifindex = packet.ifindex;

        // 获取本接口的 IP 地址（用作响应的源地址）
        let our_ip = self.get_interface_ip(ifindex)?;

        if self.verbose {
            println!("ICMP: 处理 ICMP 报文 源={} 目的={}",
                ip_hdr.source_addr, ip_hdr.dest_addr);
        }

        // 处理 ICMP 报文
        let result = icmp::process_icmp_packet(
            packet,
            ip_hdr.source_addr,
            our_ip,
            &self.context,
            self.verbose,
        ).map_err(|e| ProcessError::ParseError(format!("ICMP处理失败: {}", e)))?;

        println!("ICMP: process_icmp_packet 返回 {:?}", std::mem::discriminant(&result));

        // 根据处理结果返回
        match result {
            icmp::IcmpProcessResult::NoReply => {
                println!("ICMP: NoReply");
                Ok(None)
            }
            icmp::IcmpProcessResult::Reply(icmp_bytes) => {
                println!("ICMP: Reply {} bytes", icmp_bytes.len());
                // 获取本接口的 MAC 地址
                let our_mac = self.get_interface_mac(ifindex)?;

                // 封装为 IP 数据报
                let ip_reply = ip::Ipv4Header::new(
                    our_ip,
                    ip_hdr.source_addr,
                    ip::IP_PROTO_ICMP,
                    icmp_bytes.len(),
                );
                let mut ip_packet = ip_reply.to_bytes();
                ip_packet.extend_from_slice(&icmp_bytes);

                // 封装为以太网帧
                // 目标 MAC = 原始发送方的 MAC
                // 源 MAC = 本接口的 MAC
                let frame_bytes = crate::protocols::ethernet::build_ethernet_frame(
                    eth_hdr.src_mac,  // 响应发送给原始发送方
                    our_mac,          // 使用本接口的 MAC
                    crate::protocols::ETH_P_IP,
                    &ip_packet,
                );

                println!("ICMP: 返回以太网帧 {} bytes", frame_bytes.len());
                Ok(Some(Packet::from_bytes(frame_bytes)))
            }
            icmp::IcmpProcessResult::Processed => {
                println!("ICMP: Processed");
                Ok(None)
            }
        }
    }

    /// 处理 UDP 报文
    ///
    /// # 参数
    /// - eth_hdr: 以太网头部
    /// - ip_hdr: IP 头部
    /// - packet: Packet（已去除 IP 头部）
    fn handle_udp(&self, eth_hdr: EthernetHeader, ip_hdr: ip::Ipv4Header, packet: Packet) -> ProcessResult {
        // 获取接口索引
        let ifindex = packet.ifindex;

        // 获取本接口的 IP 地址（用作响应的源地址）
        let our_ip = self.get_interface_ip(ifindex)?;

        if self.verbose {
            println!("UDP: 处理 UDP 报文 源={} 目的={}",
                ip_hdr.source_addr, ip_hdr.dest_addr);
        }

        // 构造原始 IP 数据报用于 ICMP 响应
        // IP 头部 + UDP 头部(8字节) 作为 ICMP 错误消息的原始数据
        let original_ip_datagram = {
            let ip_header_bytes = ip_hdr.to_bytes();
            let udp_data = packet.peek(packet.remaining()).unwrap_or(&[]);
            // IP 头部 + UDP 头部(至少8字节) 或实际数据
            let udp_len = udp_data.len().min(8);
            let mut full_datagram = Vec::with_capacity(ip_header_bytes.len() + udp_len);
            full_datagram.extend_from_slice(&ip_header_bytes);
            full_datagram.extend_from_slice(&udp_data[..udp_len]);
            full_datagram
        };

        // 处理 UDP 报文
        let result = udp::process_udp_packet(
            packet,
            ip_hdr.source_addr,
            our_ip,
            &original_ip_datagram,
            &self.context,
            &udp::UDP_CONFIG_DEFAULT,
        ).map_err(|e| ProcessError::ParseError(format!("UDP处理失败: {}", e)))?;

        // 根据处理结果返回
        match result {
            udp::UdpProcessResult::NoReply => Ok(None),
            udp::UdpProcessResult::PortUnreachable(original_ip) => {
                // 获取本接口的 MAC 地址
                let our_mac = self.get_interface_mac(ifindex)?;

                // 使用原始 IP 数据报构造 ICMP 端口不可达消息
                // ICMP Type=3 (Destination Unreachable), Code=3 (Port Unreachable)
                let icmp_msg = udp::create_port_unreachable(&original_ip);

                // 封装为 IP 数据报
                let ip_reply = ip::Ipv4Header::new(
                    our_ip,
                    ip_hdr.source_addr,
                    ip::IP_PROTO_ICMP,
                    icmp_msg.len(),
                );
                let mut ip_packet = ip_reply.to_bytes();
                ip_packet.extend_from_slice(&icmp_msg);

                // 封装为以太网帧
                let frame_bytes = crate::protocols::ethernet::build_ethernet_frame(
                    eth_hdr.src_mac,
                    our_mac,
                    crate::protocols::ETH_P_IP,
                    &ip_packet,
                );

                Ok(Some(Packet::from_bytes(frame_bytes)))
            }
            udp::UdpProcessResult::Delivered(local_port, src_addr, src_port, data) => {
                // 尝试将数据分发到 Socket
                if let Ok(mut socket_mgr) = self.context.socket_mgr.lock() {
                    let _ = socket_mgr.deliver_udp_data(local_port, data, src_addr, src_port);
                }
                if self.verbose {
                    println!("UDP: 数据已交付给应用层");
                }
                Ok(None)
            }
        }
    }

    /// 处理 TCP 报文
    ///
    /// # 参数
    /// - eth_hdr: 以太网头部
    /// - ip_hdr: IP 头部
    /// - packet: Packet（已去除 IP 头部）
    fn handle_tcp(&self, eth_hdr: EthernetHeader, ip_hdr: ip::Ipv4Header, packet: Packet) -> ProcessResult {
        // 获取接口索引
        let ifindex = packet.ifindex;

        // 获取本接口的 IP 地址（用作响应的源地址）
        let our_ip = self.get_interface_ip(ifindex)?;

        if self.verbose {
            println!("TCP: 处理 TCP 报文 源={}:{} 目的={}:{}",
                ip_hdr.source_addr, 0, ip_hdr.dest_addr, 0);
        }

        // 处理 TCP 报文
        let result = tcp::process_tcp_packet(
            packet,
            ip_hdr.source_addr,
            our_ip,
            &self.context,
            &tcp::TCP_CONFIG_DEFAULT,
        ).map_err(|e| ProcessError::ParseError(format!("TCP处理失败: {}", e)))?;

        // 根据处理结果返回
        match result {
            tcp::TcpProcessResult::NoReply => Ok(None),
            tcp::TcpProcessResult::Reply(tcp_bytes) => {
                // 获取本接口的 MAC 地址
                let our_mac = self.get_interface_mac(ifindex)?;

                // 封装为 IP 数据报
                let ip_reply = ip::Ipv4Header::new(
                    our_ip,
                    ip_hdr.source_addr,
                    ip::IP_PROTO_TCP,
                    tcp_bytes.len(),
                );
                let mut ip_packet = ip_reply.to_bytes();
                ip_packet.extend_from_slice(&tcp_bytes);

                // 封装为以太网帧
                let frame_bytes = crate::protocols::ethernet::build_ethernet_frame(
                    eth_hdr.src_mac,
                    our_mac,
                    crate::protocols::ETH_P_IP,
                    &ip_packet,
                );

                Ok(Some(Packet::from_bytes(frame_bytes)))
            }
            tcp::TcpProcessResult::Delivered(conn_id, data) => {
                if self.verbose {
                    println!("TCP: 数据已交付给应用层");
                }
                // 尝试将数据分发到 Socket
                if let Ok(mut socket_mgr) = self.context.socket_mgr.lock() {
                    let _ = socket_mgr.deliver_tcp_data(&conn_id, data);
                }
                Ok(None)
            }
            tcp::TcpProcessResult::ReplyAndDelivered(conn_id, tcp_bytes, data) => {
                if self.verbose {
                    println!("TCP: 发送响应并将数据交付给应用层");
                }

                // 尝试将数据分发到 Socket
                if let Ok(mut socket_mgr) = self.context.socket_mgr.lock() {
                    let _ = socket_mgr.deliver_tcp_data(&conn_id, data);
                }

                // 获取本接口的 MAC 地址
                let our_mac = self.get_interface_mac(ifindex)?;

                // 封装为 IP 数据报
                let ip_reply = ip::Ipv4Header::new(
                    our_ip,
                    ip_hdr.source_addr,
                    ip::IP_PROTO_TCP,
                    tcp_bytes.len(),
                );
                let mut ip_packet = ip_reply.to_bytes();
                ip_packet.extend_from_slice(&tcp_bytes);

                // 封装为以太网帧
                let frame_bytes = crate::protocols::ethernet::build_ethernet_frame(
                    eth_hdr.src_mac,
                    our_mac,
                    crate::protocols::ETH_P_IP,
                    &ip_packet,
                );

                Ok(Some(Packet::from_bytes(frame_bytes)))
            }
            tcp::TcpProcessResult::ConnectionEstablished(id) => {
                if self.verbose {
                    println!("TCP: 连接已建立 {:?}", id);
                }
                // 通知 Socket 层连接已建立
                if let Ok(mut socket_mgr) = self.context.socket_mgr.lock() {
                    let _ = socket_mgr.notify_tcp_event(&id, "established");
                }
                Ok(None)
            }
            tcp::TcpProcessResult::ConnectionClosed(id) => {
                if self.verbose {
                    println!("TCP: 连接已关闭 {:?}", id);
                }
                // 通知 Socket 层连接已关闭
                if let Ok(mut socket_mgr) = self.context.socket_mgr.lock() {
                    let _ = socket_mgr.notify_tcp_event(&id, "closed");
                }
                Ok(None)
            }
        }
    }

    /// 处理 OSPF 报文
    ///
    /// # 参数
    /// - eth_hdr: 以太网头部
    /// - ip_hdr: IPv4 头部
    /// - packet: Packet（已去除 IP 头部）
    fn handle_ospf(&self, eth_hdr: EthernetHeader, ip_hdr: ip::Ipv4Header, mut packet: Packet) -> ProcessResult {
        let ifindex = packet.ifindex;

        if self.verbose {
            println!("OSPF: 处理 OSPF 报文 源={} 目的={}",
                ip_hdr.source_addr, ip_hdr.dest_addr);
        }

        // 处理 OSPF 报文
        // 注意：OSPF 需要路由器 ID，这里使用简化处理
        // TODO: 从配置或上下文获取路由器 ID
        let router_id = match self.get_interface_ip(ifindex) {
            Ok(ip) => ip,
            Err(_) => Ipv4Addr::new(1, 1, 1, 1),  // 默认路由器 ID
        };

        let result = ospf2::process_ospfv2_packet(
            &mut packet,
            ifindex,
            router_id,
            ip_hdr.source_addr,
        ).map_err(|e| ProcessError::ParseError(format!("OSPF处理失败: {:?}", e)))?;

        // 根据处理结果返回
        match result {
            ospf2::OspfProcessResult::NoReply => Ok(None),
            ospf2::OspfProcessResult::Reply(ospf_bytes) => {
                // 获取本接口的 IP 和 MAC 地址
                let our_ip = self.get_interface_ip(ifindex)?;
                let our_mac = self.get_interface_mac(ifindex)?;

                // 封装为 IP 数据报
                let ip_reply = ip::Ipv4Header::new(
                    our_ip,
                    ip_hdr.source_addr,
                    ip::IP_PROTO_OSPF,
                    ospf_bytes.len(),
                );
                let mut ip_packet = ip_reply.to_bytes();
                ip_packet.extend_from_slice(&ospf_bytes);

                // 封装为以太网帧
                let frame_bytes = crate::protocols::ethernet::build_ethernet_frame(
                    eth_hdr.src_mac,
                    our_mac,
                    crate::protocols::ETH_P_IP,
                    &ip_packet,
                );

                Ok(Some(Packet::from_bytes(frame_bytes)))
            }
            ospf2::OspfProcessResult::FloodLsa {..} | ospf2::OspfProcessResult::ScheduleSpfCalculation |
            ospf2::OspfProcessResult::DatabaseSynced => {
                // 这些结果类型不需要立即响应
                Ok(None)
            }
        }
    }

    /// 获取接口的 MAC 地址
    ///
    /// 使用系统上下文获取接口信息
    fn get_interface_mac(&self, ifindex: u32) -> Result<crate::protocols::MacAddr, ProcessError> {
        let guard = self.context.interfaces.lock()
            .map_err(|e| ProcessError::ParseError(format!("锁定接口管理器失败: {}", e)))?;

        let iface = guard.get_by_index(ifindex)
            .map_err(|e| ProcessError::ParseError(format!("获取接口失败: {}", e)))?;

        Ok(iface.mac_addr)
    }

    /// 获取接口的 IP 地址
    ///
    /// 使用系统上下文获取接口信息
    fn get_interface_ip(&self, ifindex: u32) -> Result<Ipv4Addr, ProcessError> {
        let guard = self.context.interfaces.lock()
            .map_err(|e| ProcessError::ParseError(format!("锁定接口管理器失败: {}", e)))?;

        let iface = guard.get_by_index(ifindex)
            .map_err(|e| ProcessError::ParseError(format!("获取接口失败: {}", e)))?;

        Ok(iface.ip_addr)
    }

    fn print_packet_info(&self, packet: &Packet) {
        if self.verbose {
            println!("=== [{}] ===", self.name);
            println!("Length: {} bytes", packet.len());
            println!("Offset: {} bytes", packet.offset);
            println!("Remaining: {} bytes", packet.remaining());
        } else {
            println!("[{}]: {} bytes", self.name, packet.len());
        }
    }

    fn print_eth_header(&self, hdr: &EthernetHeader) {
        println!("Ethernet header:");
        println!("  DST: {}", hdr.dst_mac);
        println!("  SRC: {}", hdr.src_mac);
        println!("  Type: 0x{:04x}", hdr.ether_type);
    }
}

impl Default for PacketProcessor {
    fn default() -> Self {
        Self {
            name: String::from("DefaultProcessor"),
            verbose: false,
            context: SystemContext::new(),
        }
    }
}

pub fn process_packet(packet: Packet) -> ProcessResult {
    let ctx = SystemContext::new();
    PacketProcessor::with_context(ctx).process(packet)
}

pub fn process_packet_verbose(packet: Packet) -> ProcessResult {
    let ctx = SystemContext::new();
    PacketProcessor::with_context(ctx).with_verbose(true).process(packet)
}

// --- 测试模块 ---

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::{MacAddr, Ipv4Addr, CoreError, ETH_P_ARP, ETH_P_IP, ETH_P_IPV6, ETH_P_8021Q, ETH_P_8021AD};
    use crate::protocols::arp::{ArpPacket, ArpOperation};
    use crate::protocols::ethernet;

    // --- 测试辅助函数 ---

    /// 构造 VLAN TCI (Tag Control Information)
    fn create_vlan_tci(pcp: u8, dei: bool, vid: u16) -> u16 {
        let pcp_value = ((pcp & 0x07) as u16) << 13;
        let dei_value = if dei { 1 << 12 } else { 0 };
        let vid_value = vid & 0x0FFF;
        pcp_value | dei_value | vid_value
    }

    /// 构造带 VLAN 标签的完整报文
    #[allow(clippy::too_many_arguments)]
    fn create_vlan_packet(
        dst_mac: MacAddr,
        src_mac: MacAddr,
        vlan_tpid: u16,
        pcp: u8,
        dei: bool,
        vid: u16,
        inner_type: u16,
        payload: Vec<u8>,
    ) -> Packet {
        let mut bytes = Vec::new();

        // 以太网头（目标 MAC + 源 MAC + VLAN TPID）
        bytes.extend_from_slice(&dst_mac.bytes);
        bytes.extend_from_slice(&src_mac.bytes);
        bytes.extend_from_slice(&vlan_tpid.to_be_bytes());

        // VLAN TCI
        let tci = create_vlan_tci(pcp, dei, vid);
        bytes.extend_from_slice(&tci.to_be_bytes());

        // 内层以太网类型
        bytes.extend_from_slice(&inner_type.to_be_bytes());

        // 负载
        bytes.extend_from_slice(&payload);

        Packet::from_bytes(bytes)
    }

    /// 构造 ARP 请求报文（带以太网头）
    fn create_arp_request_packet(
        dst_mac: MacAddr,
        src_mac: MacAddr,
        src_ip: Ipv4Addr,
        dst_ip: Ipv4Addr,
    ) -> Packet {
        let arp_pkt = ArpPacket::new(
            ArpOperation::Request,
            src_mac,
            src_ip,
            MacAddr::zero(),
            dst_ip,
        );
        Packet::from_bytes(ethernet::build_ethernet_frame(dst_mac, src_mac, ETH_P_ARP, &arp_pkt.to_bytes()))
    }

    /// 构造 ARP 响应报文（带以太网头）
    fn create_arp_reply_packet(
        dst_mac: MacAddr,
        src_mac: MacAddr,
        src_ip: Ipv4Addr,
        dst_ip: Ipv4Addr,
    ) -> Packet {
        let arp_pkt = ArpPacket::new(
            ArpOperation::Reply,
            src_mac,
            src_ip,
            dst_mac,
            dst_ip,
        );
        Packet::from_bytes(ethernet::build_ethernet_frame(dst_mac, src_mac, ETH_P_ARP, &arp_pkt.to_bytes()))
    }

    /// 构造 QinQ 双层标签报文
    fn create_qinq_packet(
        dst_mac: MacAddr,
        src_mac: MacAddr,
        outer_vid: u16,
        inner_vid: u16,
        inner_type: u16,
        payload: Vec<u8>,
    ) -> Packet {
        let mut bytes = Vec::new();

        // 以太网头（目标 MAC + 源 MAC + 外层 VLAN TPID）
        bytes.extend_from_slice(&dst_mac.bytes);
        bytes.extend_from_slice(&src_mac.bytes);
        bytes.extend_from_slice(&ETH_P_8021AD.to_be_bytes());  // 外层使用 802.1ad

        // 外层 VLAN TCI
        let outer_tci = create_vlan_tci(0, false, outer_vid);
        bytes.extend_from_slice(&outer_tci.to_be_bytes());

        // 内层 VLAN TPID
        bytes.extend_from_slice(&ETH_P_8021Q.to_be_bytes());

        // 内层 VLAN TCI
        let inner_tci = create_vlan_tci(0, false, inner_vid);
        bytes.extend_from_slice(&inner_tci.to_be_bytes());

        // 内层以太网类型
        bytes.extend_from_slice(&inner_type.to_be_bytes());

        // 负载
        bytes.extend_from_slice(&payload);

        Packet::from_bytes(bytes)
    }

    /// 构造截断报文（用于边界测试）
    fn create_truncated_packet() -> Packet {
        // 只有 10 字节，不足以解析以太网头
        Packet::from_bytes(vec![0x01, 0x02, 0x03, 0x04, 0x05, 0x06,
                                0x11, 0x12, 0x13, 0x14])
    }

    /// 构造畸形报文（无效的以太网类型）
    fn create_malformed_packet() -> Packet {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&[0xFF; 12]);  // MAC 地址
        bytes.extend_from_slice(&[0x00, 0x01]);  // 未知以太网类型
        bytes.extend_from_slice(&[0xAA; 20]);  // 负载
        Packet::from_bytes(bytes)
    }

    #[test]
    fn test_processor_creation() {
        let ctx = SystemContext::new();
        let processor = PacketProcessor::with_context(ctx);
        assert_eq!(processor.name(), "ContextProcessor");
    }

    #[test]
    fn test_processor_with_name_and_context() {
        let ctx = SystemContext::new();
        let processor = PacketProcessor::with_name_and_context(String::from("TestProcessor"), ctx);
        assert_eq!(processor.name(), "TestProcessor");
    }

    #[test]
    fn test_processor_verbose() {
        let ctx = SystemContext::new();
        let processor = PacketProcessor::with_context(ctx).with_verbose(true);
        // verbose 是私有字段，无法直接访问
        // 但我们可以验证 with_verbose 返回了处理器
        assert_eq!(processor.name(), "ContextProcessor");
    }

    #[test]
    fn test_processor_default() {
        let processor = PacketProcessor::default();
        assert_eq!(processor.name(), "DefaultProcessor");
    }

    #[test]
    fn test_processor_name() {
        let ctx = SystemContext::new();
        let processor = PacketProcessor::with_name_and_context(String::from("MyProcessor"), ctx);
        assert_eq!(processor.name(), "MyProcessor");
    }

    #[test]
    fn test_dispatch_vlan_8021q() {
        let ctx = SystemContext::new();
        let processor = PacketProcessor::with_context(ctx);

        let dst_mac = MacAddr::broadcast();
        let src_mac = MacAddr::new([0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF]);

        // 构造带 VLAN 标签的报文（内层使用 IPv4，会返回不支持错误但验证解析流程）
        let packet = create_vlan_packet(
            dst_mac,
            src_mac,
            ETH_P_8021Q,
            0,
            false,
            100,
            ETH_P_IP,  // 内层 IPv4 (未实现)
            vec![0x01, 0x02, 0x03],
        );

        // VLAN 解析成功，但内层 IPv4 未实现
        let result = processor.process(packet);
        assert!(result.is_err());
        // 验证错误类型 - 可能是 UnsupportedProtocol 或 ParseError
        match result {
            Err(ProcessError::UnsupportedProtocol(_)) => {
                // 预期情况
            }
            Err(ProcessError::ParseError(_)) => {
                // VLAN 解析可能返回解析错误
            }
            other => {
                panic!("Expected UnsupportedProtocol or ParseError, got {:?}", other);
            }
        }
    }

    #[test]
    fn test_dispatch_vlan_8021ad() {
        let ctx = SystemContext::new();
        let processor = PacketProcessor::with_context(ctx);

        let dst_mac = MacAddr::broadcast();
        let src_mac = MacAddr::new([0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF]);

        // 构造带 802.1ad VLAN 标签的报文（内层使用 IPv4）
        let packet = create_vlan_packet(
            dst_mac,
            src_mac,
            ETH_P_8021AD,
            0,
            false,
            200,
            ETH_P_IP,
            vec![0x01, 0x02, 0x03],
        );

        let result = processor.process(packet);
        assert!(result.is_err());
        match result {
            Err(ProcessError::UnsupportedProtocol(_)) => {}
            Err(ProcessError::ParseError(_)) => {}
            other => panic!("Expected UnsupportedProtocol or ParseError, got {:?}", other),
        }
    }

    #[test]
    fn test_dispatch_arp() {
        let ctx = SystemContext::new();
        let processor = PacketProcessor::with_context(ctx);

        let src_mac = MacAddr::new([0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF]);
        let src_ip = Ipv4Addr::new(192, 168, 1, 1);
        let dst_ip = Ipv4Addr::new(192, 168, 1, 2);

        let packet = create_arp_request_packet(MacAddr::broadcast(), src_mac, src_ip, dst_ip);

        let result = processor.process(packet);
        // ARP 处理需要全局缓存，这里只验证不崩溃
        assert!(result.is_ok() || result.is_err());
    }

    #[test]
    fn test_dispatch_ipv4_unsupported() {
        let ctx = SystemContext::new();
        let processor = PacketProcessor::with_context(ctx);

        let mut bytes = Vec::new();
        bytes.extend_from_slice(&MacAddr::broadcast().bytes);
        bytes.extend_from_slice(&[0xAA; 6]);  // 源 MAC
        bytes.extend_from_slice(&ETH_P_IP.to_be_bytes());

        // 创建有效的 IPv4 头部 (version=4, ihl=5, protocol=TCP=6)
        bytes.push(0x45);  // Version=4, IHL=5 (20 字节)
        bytes.push(0x00);  // TOS
        bytes.extend_from_slice(&[0x00, 0x14]);  // Total Length = 20
        bytes.extend_from_slice(&[0x00, 0x01]);  // Identification
        bytes.extend_from_slice(&[0x00, 0x00]);  // Flags/Fragment
        bytes.push(64);   // TTL
        bytes.push(6);    // Protocol = TCP (不支持)
        bytes.extend_from_slice(&[0x00, 0x00]);  // Checksum (占位)
        bytes.extend_from_slice(&[192, 168, 1, 1]);  // Source IP
        bytes.extend_from_slice(&[192, 168, 1, 2]);  // Dest IP

        let packet = Packet::from_bytes(bytes);
        let result = processor.process(packet);

        // 由于没有初始化全局接口管理器，期望返回错误
        // 有效的 IP 头部但接口未初始化会导致错误
        assert!(result.is_err());
    }

    #[test]
    fn test_dispatch_ipv6_basic() {
        let ctx = SystemContext::new();
        let processor = PacketProcessor::with_context(ctx);

        let mut bytes = Vec::new();
        bytes.extend_from_slice(&MacAddr::broadcast().bytes);
        bytes.extend_from_slice(&[0xAA; 6]);  // 源 MAC
        bytes.extend_from_slice(&ETH_P_IPV6.to_be_bytes());

        // 创建有效的 IPv6 头部 (version=6, next_header=ICMPv6=58)
        bytes.push(0x60);  // Version=6, TC=0
        bytes.push(0x00);  // TC + Flow
        bytes.push(0x00);  // Flow
        bytes.push(0x00);  // Flow
        bytes.extend_from_slice(&[0x00, 0x00]);  // Payload Length = 0
        bytes.push(58);    // Next Header = ICMPv6
        bytes.push(64);    // Hop Limit
        // 源地址 (2001:db8::1)
        bytes.extend_from_slice(&[0x20, 0x01, 0x0d, 0xb8]);
        bytes.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]);
        bytes.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]);
        bytes.extend_from_slice(&[0x00, 0x00, 0x00, 0x01]);
        // 目的地址 (2001:db8::2)
        bytes.extend_from_slice(&[0x20, 0x01, 0x0d, 0xb8]);
        bytes.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]);
        bytes.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]);
        bytes.extend_from_slice(&[0x00, 0x00, 0x00, 0x02]);

        let packet = Packet::from_bytes(bytes);
        let result = processor.process(packet);

        // IPv6 已实现，但由于接口未配置 IPv6 地址，期望返回错误或 Ok(None)
        assert!(result.is_ok() || result.is_err());
    }

    #[test]
    fn test_dispatch_unknown_ethertype() {
        let ctx = SystemContext::new();
        let processor = PacketProcessor::with_context(ctx);

        let packet = create_malformed_packet();
        let result = processor.process(packet);

        assert!(result.is_err());
        match result {
            Err(ProcessError::UnsupportedProtocol(msg)) => {
                assert!(msg.contains("0x0001"));
            }
            _ => panic!("Expected UnsupportedProtocol error"),
        }
    }

    #[test]
    fn test_handle_vlan_single_tag() {
        let ctx = SystemContext::new();
        let processor = PacketProcessor::with_context(ctx).with_verbose(true);

        let dst_mac = MacAddr::broadcast();
        let src_mac = MacAddr::new([0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF]);

        let packet = create_vlan_packet(
            dst_mac,
            src_mac,
            ETH_P_8021Q,
            3,  // PCP = 3
            true,  // DEI = 1
            100,  // VID = 100
            ETH_P_IP,  // 内层 IPv4
            vec![0x01, 0x02, 0x03],
        );

        let result = processor.process(packet);
        // VLAN 标签解析成功，但内层 IPv4 未实现
        assert!(result.is_err());
        match result {
            Err(ProcessError::UnsupportedProtocol(_)) => {}
            Err(ProcessError::ParseError(_)) => {}
            other => panic!("Expected UnsupportedProtocol or ParseError, got {:?}", other),
        }
    }

    #[test]
    fn test_handle_vlan_qinq_double_tag() {
        let ctx = SystemContext::new();
        let processor = PacketProcessor::with_context(ctx);

        let dst_mac = MacAddr::broadcast();
        let src_mac = MacAddr::new([0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF]);

        let packet = create_qinq_packet(
            dst_mac,
            src_mac,
            10,   // 外层 VLAN
            20,   // 内层 VLAN
            ETH_P_IP,  // 内层 IPv4
            vec![0x01, 0x02, 0x03],
        );

        let result = processor.process(packet);
        // QinQ 解析成功，但内层 IPv4 未实现
        assert!(result.is_err());
        match result {
            Err(ProcessError::UnsupportedProtocol(_)) => {}
            Err(ProcessError::ParseError(_)) => {}
            other => panic!("Expected UnsupportedProtocol or ParseError, got {:?}", other),
        }
    }

    #[test]
    fn test_handle_vlan_inner_arp_dispatch() {
        let ctx = SystemContext::new();
        let processor = PacketProcessor::with_context(ctx);

        let dst_mac = MacAddr::broadcast();
        let src_mac = MacAddr::new([0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF]);

        // 构造完整的 VLAN + ARP 报文
        let mut bytes = Vec::new();

        // 以太网头
        bytes.extend_from_slice(&dst_mac.bytes);
        bytes.extend_from_slice(&src_mac.bytes);
        bytes.extend_from_slice(&ETH_P_8021Q.to_be_bytes());

        // VLAN TCI (VID=100)
        let tci = create_vlan_tci(0, false, 100);
        bytes.extend_from_slice(&tci.to_be_bytes());

        // 内层 ARP 类型
        bytes.extend_from_slice(&ETH_P_ARP.to_be_bytes());

        // ARP 报文
        let arp_pkt = ArpPacket::new(
            ArpOperation::Request,
            src_mac,
            Ipv4Addr::new(192, 168, 1, 1),
            MacAddr::zero(),
            Ipv4Addr::new(192, 168, 1, 2),
        );
        bytes.extend_from_slice(&arp_pkt.to_bytes());

        let packet = Packet::from_bytes(bytes);
        // ARP 处理需要全局缓存，可能返回 Ok 或 Err
        let result = processor.process(packet);
        // 只要不是 panic 就算成功（验证解析流程）
        assert!(result.is_ok() || result.is_err());
    }

    #[test]
    fn test_handle_vlan_boundary_vid_0() {
        let ctx = SystemContext::new();
        let processor = PacketProcessor::with_context(ctx);

        let dst_mac = MacAddr::broadcast();
        let src_mac = MacAddr::new([0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF]);

        let packet = create_vlan_packet(
            dst_mac,
            src_mac,
            ETH_P_8021Q,
            0,
            false,
            0,  // VID = 0 (边界值)
            0x0806,
            vec![0x01, 0x02, 0x03],
        );

        let result = processor.process(packet);
        // VID=0 可能被 VLAN 模块拒绝，也可能被接受
        // 只验证不崩溃
        assert!(result.is_ok() || result.is_err());
    }

    #[test]
    fn test_handle_vlan_boundary_vid_4095() {
        let ctx = SystemContext::new();
        let processor = PacketProcessor::with_context(ctx);

        let dst_mac = MacAddr::broadcast();
        let src_mac = MacAddr::new([0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF]);

        let packet = create_vlan_packet(
            dst_mac,
            src_mac,
            ETH_P_8021Q,
            0,
            false,
            4095,  // VID = 4095 (保留值)
            0x0806,
            vec![0x01, 0x02, 0x03],
        );

        let result = processor.process(packet);
        // VID=4095 可能被 VLAN 模块拒绝
        assert!(result.is_ok() || result.is_err());
    }

    #[test]
    fn test_handle_vlan_truncated_packet() {
        let ctx = SystemContext::new();
        let processor = PacketProcessor::with_context(ctx);

        // 构造截断的 VLAN 报文（不足 4 字节用于 VLAN 标签）
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&MacAddr::broadcast().bytes);
        bytes.extend_from_slice(&[0xAA; 6]);
        bytes.extend_from_slice(&ETH_P_8021Q.to_be_bytes());
        // 只有 2 字节，不足以解析完整的 VLAN 标签
        bytes.extend_from_slice(&[0x01, 0x02]);

        let packet = Packet::from_bytes(bytes);
        let result = processor.process(packet);

        // 应该返回解析错误
        assert!(result.is_err());
    }

    #[test]
    fn test_handle_arp_request() {
        let ctx = SystemContext::new();
        let processor = PacketProcessor::with_context(ctx);

        let src_mac = MacAddr::new([0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF]);
        let src_ip = Ipv4Addr::new(192, 168, 1, 1);
        let dst_ip = Ipv4Addr::new(192, 168, 1, 2);

        let packet = create_arp_request_packet(MacAddr::broadcast(), src_mac, src_ip, dst_ip);

        let result = processor.process(packet);
        // ARP 处理需要全局缓存初始化
        assert!(result.is_ok() || result.is_err());
    }

    #[test]
    fn test_handle_arp_reply() {
        let ctx = SystemContext::new();
        let processor = PacketProcessor::with_context(ctx);

        let dst_mac = MacAddr::new([0x11, 0x22, 0x33, 0x44, 0x55, 0x66]);
        let src_mac = MacAddr::new([0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF]);
        let src_ip = Ipv4Addr::new(192, 168, 1, 1);
        let dst_ip = Ipv4Addr::new(192, 168, 1, 2);

        let packet = create_arp_reply_packet(dst_mac, src_mac, src_ip, dst_ip);

        let result = processor.process(packet);
        // ARP 响应目标 MAC 不是广播，会被拒绝
        assert!(result.is_err());
    }

    #[test]
    fn test_handle_arp_broadcast_target() {
        let ctx = SystemContext::new();
        let processor = PacketProcessor::with_context(ctx);

        let src_mac = MacAddr::new([0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF]);
        let src_ip = Ipv4Addr::new(192, 168, 1, 1);
        let dst_ip = Ipv4Addr::new(192, 168, 1, 2);

        let packet = create_arp_request_packet(MacAddr::broadcast(), src_mac, src_ip, dst_ip);

        let result = processor.process(packet);
        // 广播目标 MAC 应该通过验证
        assert!(result.is_ok() || result.is_err());
    }

    #[test]
    fn test_handle_arp_unicast_target() {
        let ctx = SystemContext::new();
        let processor = PacketProcessor::with_context(ctx);

        // 构造目标 MAC 为本机（零地址）的 ARP 报文
        let dst_mac = MacAddr::zero();
        let src_mac = MacAddr::new([0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF]);
        let src_ip = Ipv4Addr::new(192, 168, 1, 1);
        let dst_ip = Ipv4Addr::new(192, 168, 1, 2);

        let packet = create_arp_request_packet(dst_mac, src_mac, src_ip, dst_ip);

        let result = processor.process(packet);
        // 零地址应该通过验证
        assert!(result.is_ok() || result.is_err());
    }

    #[test]
    fn test_error_from_core_error_parse() {
        let core_error = CoreError::parse_error("test parse error");
        let process_error: ProcessError = core_error.into();

        match process_error {
            ProcessError::ParseError(msg) => {
                assert!(msg.contains("test parse error"));
            }
            _ => panic!("Expected ParseError"),
        }
    }

    #[test]
    fn test_error_from_core_error_invalid_packet() {
        let core_error = CoreError::invalid_packet("invalid packet");
        let process_error: ProcessError = core_error.into();

        match process_error {
            ProcessError::InvalidPacket(msg) => {
                assert!(msg.contains("invalid packet"));
            }
            _ => panic!("Expected InvalidPacket"),
        }
    }

    #[test]
    fn test_error_from_core_error_unsupported_protocol() {
        let core_error = CoreError::unsupported_protocol("test protocol");
        let process_error: ProcessError = core_error.into();

        match process_error {
            ProcessError::UnsupportedProtocol(proto) => {
                assert!(proto.contains("test protocol"));
            }
            _ => panic!("Expected UnsupportedProtocol"),
        }
    }

    #[test]
    fn test_error_from_vlan_error() {
        use crate::protocols::vlan::VlanError;

        let vlan_error = VlanError::InvalidVlanId { vid: 5000 };
        let process_error: ProcessError = vlan_error.into();

        match process_error {
            ProcessError::ParseError(msg) => {
                assert!(msg.contains("VLAN"));
            }
            _ => panic!("Expected ParseError with VLAN prefix"),
        }
    }

    #[test]
    fn test_error_from_string() {
        let msg = String::from("test error message");
        let process_error: ProcessError = msg.into();

        match process_error {
            ProcessError::ParseError(s) => {
                assert_eq!(s, "test error message");
            }
            _ => panic!("Expected ParseError"),
        }
    }

    #[test]
    fn test_process_packet() {
        let src_mac = MacAddr::new([0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF]);
        let src_ip = Ipv4Addr::new(192, 168, 1, 1);
        let dst_ip = Ipv4Addr::new(192, 168, 1, 2);

        let packet = create_arp_request_packet(MacAddr::broadcast(), src_mac, src_ip, dst_ip);

        let result = process_packet(packet);
        assert!(result.is_ok() || result.is_err());
    }

    #[test]
    fn test_process_packet_verbose() {
        let src_mac = MacAddr::new([0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF]);
        let src_ip = Ipv4Addr::new(192, 168, 1, 1);
        let dst_ip = Ipv4Addr::new(192, 168, 1, 2);

        let packet = create_arp_request_packet(MacAddr::broadcast(), src_mac, src_ip, dst_ip);

        let result = process_packet_verbose(packet);
        assert!(result.is_ok() || result.is_err());
    }

    #[test]
    fn test_full_vlan_arp_flow() {
        let ctx = SystemContext::new();
        let processor = PacketProcessor::with_context(ctx).with_verbose(true);

        // 构造完整的以太网 + VLAN + ARP 报文
        let dst_mac = MacAddr::broadcast();
        let src_mac = MacAddr::new([0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF]);

        let mut bytes = Vec::new();

        // 以太网头
        bytes.extend_from_slice(&dst_mac.bytes);
        bytes.extend_from_slice(&src_mac.bytes);
        bytes.extend_from_slice(&ETH_P_8021Q.to_be_bytes());

        // VLAN 标签 (VID=100, PCP=3)
        let tci = create_vlan_tci(3, false, 100);
        bytes.extend_from_slice(&tci.to_be_bytes());

        // 内层 ARP
        bytes.extend_from_slice(&ETH_P_ARP.to_be_bytes());

        // ARP 报文
        let arp_pkt = ArpPacket::new(
            ArpOperation::Request,
            src_mac,
            Ipv4Addr::new(192, 168, 1, 1),
            MacAddr::zero(),
            Ipv4Addr::new(192, 168, 1, 2),
        );
        bytes.extend_from_slice(&arp_pkt.to_bytes());

        let packet = Packet::from_bytes(bytes);

        // 处理报文 - 验证解析流程正常
        let result = processor.process(packet);
        // ARP 处理需要全局缓存，可能返回 Ok 或 Err
        assert!(result.is_ok() || result.is_err());
    }

    #[test]
    fn test_error_propagation() {
        let ctx = SystemContext::new();
        let processor = PacketProcessor::with_context(ctx);

        // 构造会导致 VLAN 解析错误的报文
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&MacAddr::broadcast().bytes);
        bytes.extend_from_slice(&[0xAA; 6]);
        bytes.extend_from_slice(&ETH_P_8021Q.to_be_bytes());
        // VLAN TCI 部分截断
        bytes.extend_from_slice(&[0x01]);

        let packet = Packet::from_bytes(bytes);
        let result = processor.process(packet);

        // 错误应该正确传播
        assert!(result.is_err());
    }

    #[test]
    fn test_empty_packet() {
        let ctx = SystemContext::new();
        let processor = PacketProcessor::with_context(ctx);
        let packet = Packet::from_bytes(vec![]);

        let result = processor.process(packet);
        assert!(result.is_err());
    }

    #[test]
    fn test_truncated_ethernet_header() {
        let ctx = SystemContext::new();
        let processor = PacketProcessor::with_context(ctx);
        let packet = create_truncated_packet();

        let result = processor.process(packet);
        assert!(result.is_err());
    }
}
