// src/engine/processor.rs
//
// 报文处理器
// 提供报文处理接口，负责逐层解析/分发报文

use crate::common::{Packet, EthernetHeader, VlanTag};
use crate::protocols::arp;

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

impl From<crate::protocols::vlan::VlanError> for ProcessError {
    fn from(err: crate::protocols::vlan::VlanError) -> Self {
        ProcessError::ParseError(format!("VLAN错误: {}", err))
    }
}

impl From<String> for ProcessError {
    fn from(msg: String) -> Self {
        ProcessError::ParseError(msg)
    }
}

pub struct PacketProcessor {
    name: String,
    verbose: bool,
}

impl PacketProcessor {
    pub fn new() -> Self {
        Self {
            name: String::from("DefaultProcessor"),
            verbose: false,
        }
    }

    pub fn with_name(name: String) -> Self {
        Self {
            name,
            verbose: false,
        }
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

        match eth_hdr.ether_type() {
            ETH_P_8021Q | ETH_P_8021AD => {
                self.handle_vlan(eth_hdr, packet)?;
            }
            ETH_P_ARP => {
                self.handle_arp(eth_hdr, packet)?;
            }
            ETH_P_IP => {
                return Err(ProcessError::UnsupportedProtocol(
                    String::from("IPv4 protocol not implemented")
                ));
            }
            ETH_P_IPV6 => {
                return Err(ProcessError::UnsupportedProtocol(
                    String::from("IPv6 protocol not implemented")
                ));
            }
            _ => {
                return Err(ProcessError::UnsupportedProtocol(
                    format!("Unknown ethernet type: 0x{:04x}", eth_hdr.ether_type())
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
        _eth_hdr: EthernetHeader,
        _outer_vlan: Option<VlanTag>,
        _inner_vlan: Option<VlanTag>,
        inner_type: u16,
        packet: Packet,
    ) -> ProcessResult {
        use crate::common::ETH_P_ARP;
        use crate::common::ETH_P_IP;

        match inner_type {
            ETH_P_ARP => {
                // VLAN 内的 ARP 报文处理
                self.handle_arp_packet(packet)?;
            }
            ETH_P_IP => {
                return Err(ProcessError::UnsupportedProtocol(
                    String::from("IPv4 in VLAN not implemented")
                ));
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
        // 验证目标 MAC 地址（广播或本机）
        if !eth_hdr.dst_mac().is_broadcast() && !eth_hdr.dst_mac().is_zero() {
            return Err(ProcessError::InvalidPacket(
                format!("Invalid ARP destination MAC: {}", eth_hdr.dst_mac())
            ));
        }

        self.handle_arp_packet(packet)
    }

    /// 处理 ARP 报文（统一入口）
    ///
    /// 负责调用 ARP 模块进行解析和处理。
    /// TODO: 后续需要传递接口上下文（MAC、IP）以支持生成响应报文。
    fn handle_arp_packet(&self, mut packet: Packet) -> ProcessResult {
        // 调用 ARP 模块解析报文
        let arp_pkt = arp::ArpPacket::from_packet(&mut packet)?;

        // 输出 ARP 报文信息
        if self.verbose {
            println!("ARP报文:");
            println!("  操作: {:?}", arp_pkt.operation);
            println!("  发送方: MAC={}, IP={}",
                arp_pkt.sender_hardware_addr, arp_pkt.sender_protocol_addr);
            println!("  目标: MAC={}, IP={}",
                arp_pkt.target_hardware_addr, arp_pkt.target_protocol_addr);

            if arp_pkt.is_gratuitous() {
                println!("  [免费ARP]");
            }

            match arp_pkt.operation {
                arp::ArpOperation::Request => {
                    println!("  收到ARP请求: {} -> {}",
                        arp_pkt.sender_protocol_addr,
                        arp_pkt.target_protocol_addr);
                }
                arp::ArpOperation::Reply => {
                    println!("  收到ARP响应: {} -> {}",
                        arp_pkt.sender_protocol_addr,
                        arp_pkt.sender_hardware_addr);
                }
            }
        }

        // TODO: 后续添加接口上下文后，调用 process_arp 生成响应报文
        // let reply = arp::process_arp(&mut packet, local_mac, local_ip, src_mac, ...)?;
        // if let ArpProcessResult::Reply(frame) = reply {
        //     return Ok(Some(Packet::from_bytes(frame)));
        // }

        Ok(None)
    }

    fn print_packet_info(&self, packet: &Packet) {
        if self.verbose {
            println!("=== [{}] ===", self.name);
            println!("Length: {} bytes", packet.len());
            println!("Offset: {} bytes", packet.get_offset());
            println!("Remaining: {} bytes", packet.remaining());
        } else {
            println!("[{}]: {} bytes", self.name, packet.len());
        }
    }

    fn print_eth_header(&self, hdr: &EthernetHeader) {
        println!("Ethernet header:");
        println!("  DST: {}", hdr.dst_mac());
        println!("  SRC: {}", hdr.src_mac());
        println!("  Type: 0x{:04x}", hdr.ether_type());
    }
}

impl Default for PacketProcessor {
    fn default() -> Self {
        Self::new()
    }
}

pub fn process_packet(packet: Packet) -> ProcessResult {
    PacketProcessor::new().process(packet)
}

pub fn process_packet_verbose(packet: Packet) -> ProcessResult {
    PacketProcessor::new().with_verbose(true).process(packet)
}
