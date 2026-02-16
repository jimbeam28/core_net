// src/engine/processor.rs
//
// 报文处理器
// 提供报文处理接口，负责逐层解析/分发报文

use crate::common::{Packet, EthernetHeader, VlanTag};

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

    fn handle_vlan(&self, _eth_hdr: EthernetHeader, mut packet: Packet) -> ProcessResult {
        let tpid_opt = crate::common::has_vlan_tag(&packet);
        let tpid = match tpid_opt {
            Some(t) => t,
            None => {
                return Err(ProcessError::ParseError(String::from("No VLAN tag detected")));
            }
        };

        let vlan_tag = VlanTag::parse_from_packet(&mut packet)?;

        if self.verbose {
            println!("VLAN tag: TPID=0x{:04x}, PCP={}, DEI={}, VID={}",
                tpid, vlan_tag.pcp, vlan_tag.dei, vlan_tag.vid);
        }

        let inner_vlan_opt = crate::common::has_vlan_tag(&packet);

        if inner_vlan_opt.is_some() {
            let inner_vlan = VlanTag::parse_from_packet(&mut packet)?;
            if self.verbose {
                println!("Inner VLAN: PCP={}, DEI={}, VID={}",
                    inner_vlan.pcp, inner_vlan.dei, inner_vlan.vid);
            }
            if packet.remaining() < 2 {
                return Err(ProcessError::InvalidPacket(String::from("Packet too short")));
            }
            let inner_type_bytes = packet.read(2)
                .ok_or_else(|| ProcessError::ParseError(String::from("Failed to read inner ethertype")))?;
            let inner_type = u16::from_be_bytes([inner_type_bytes[0], inner_type_bytes[1]]);
            if self.verbose {
                println!("Inner ethertype: 0x{:04x}", inner_type);
            }
            self.dispatch_inner_vlan(_eth_hdr, Some(vlan_tag), Some(inner_vlan), inner_type, packet)?;
        } else {
            if packet.remaining() < 2 {
                return Err(ProcessError::InvalidPacket(String::from("Packet too short")));
            }
            let inner_type_bytes = packet.read(2)
                .ok_or_else(|| ProcessError::ParseError(String::from("Failed to read inner ethertype")))?;
            let inner_type = u16::from_be_bytes([inner_type_bytes[0], inner_type_bytes[1]]);
            if self.verbose {
                println!("Inner ethertype: 0x{:04x}", inner_type);
            }
            self.dispatch_inner_vlan(_eth_hdr, Some(vlan_tag), None, inner_type, packet)?;
        }
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
                self.handle_arp_ip(packet)?;
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

    fn handle_arp(&self, eth_hdr: EthernetHeader, mut packet: Packet) -> ProcessResult {
        if !eth_hdr.dst_mac().is_broadcast() && !eth_hdr.dst_mac().is_zero() {
            return Err(ProcessError::InvalidPacket(
                format!("Invalid ARP destination MAC: {}", eth_hdr.dst_mac())
            ));
        }

        let arp_pkt = crate::protocols::arp::ArpPacket::from_packet(&mut packet)?;
        #[allow(deprecated)]
        crate::protocols::arp::handle_arp_packet(&arp_pkt, self.verbose)?;

        Ok(None)
    }

    fn handle_arp_ip(&self, mut packet: Packet) -> ProcessResult {
        let arp_pkt = crate::protocols::arp::ArpPacket::from_packet(&mut packet)?;
        #[allow(deprecated)]
        crate::protocols::arp::handle_arp_packet(&arp_pkt, self.verbose)?;
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
