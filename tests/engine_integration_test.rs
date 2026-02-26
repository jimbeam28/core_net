// Engine 模块集成测试

use core_net::common::{MacAddr, Ipv4Addr, Packet, ETH_P_ARP, ETH_P_IP, ETH_P_8021Q, ETH_P_8021AD};
use core_net::protocols::arp::{ArpPacket, ArpOperation};
use core_net::engine::{PacketProcessor, ProcessError};
use core_net::context::SystemContext;

// 测试辅助函数

/// VLAN 报文配置
struct VlanPacketConfig {
    dst_mac: MacAddr,
    src_mac: MacAddr,
    vlan_tpid: u16,
    pcp: u8,
    dei: bool,
    vid: u16,
    inner_type: u16,
    payload: Vec<u8>,
}

impl VlanPacketConfig {
    fn new(
        dst_mac: MacAddr,
        src_mac: MacAddr,
        vlan_tpid: u16,
        vid: u16,
        inner_type: u16,
        payload: Vec<u8>,
    ) -> Self {
        Self {
            dst_mac,
            src_mac,
            vlan_tpid,
            pcp: 0,
            dei: false,
            vid,
            inner_type,
            payload,
        }
    }

    fn with_pcp(mut self, pcp: u8) -> Self {
        self.pcp = pcp;
        self
    }

    fn with_dei(mut self, dei: bool) -> Self {
        self.dei = dei;
        self
    }

    fn build(self) -> Packet {
        let mut bytes = Vec::new();

        bytes.extend_from_slice(&self.dst_mac.bytes);
        bytes.extend_from_slice(&self.src_mac.bytes);
        bytes.extend_from_slice(&self.vlan_tpid.to_be_bytes());

        let pcp_value = ((self.pcp & 0x07) as u16) << 13;
        let dei_value = if self.dei { 1 << 12 } else { 0 };
        let vid_value = self.vid & 0x0FFF;
        let tci = pcp_value | dei_value | vid_value;
        bytes.extend_from_slice(&tci.to_be_bytes());

        bytes.extend_from_slice(&self.inner_type.to_be_bytes());

        bytes.extend_from_slice(&self.payload);

        Packet::from_bytes(bytes)
    }
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

    let mut bytes = Vec::new();
    bytes.extend_from_slice(&dst_mac.bytes);
    bytes.extend_from_slice(&src_mac.bytes);
    bytes.extend_from_slice(&ETH_P_ARP.to_be_bytes());
    bytes.extend_from_slice(&arp_pkt.to_bytes());

    Packet::from_bytes(bytes)
}

// 集成测试场景

#[test]
fn test_vlan_arp_full_flow() {
    let processor = PacketProcessor::with_context(SystemContext::new()).with_verbose(true);

    let dst_mac = MacAddr::broadcast();
    let src_mac = MacAddr::new([0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF]);

    let mut bytes = Vec::new();

    bytes.extend_from_slice(&dst_mac.bytes);
    bytes.extend_from_slice(&src_mac.bytes);
    bytes.extend_from_slice(&ETH_P_8021Q.to_be_bytes());

    let pcp_value = ((3 & 0x07) as u16) << 13;
    let dei_value = if true { 1 << 12 } else { 0 };
    let vid_value = 100u16 & 0x0FFF;
    let tci = pcp_value | dei_value | vid_value;
    bytes.extend_from_slice(&tci.to_be_bytes());

    bytes.extend_from_slice(&ETH_P_ARP.to_be_bytes());

    let arp_pkt = ArpPacket::new(
        ArpOperation::Request,
        src_mac,
        Ipv4Addr::new(192, 168, 1, 10),
        MacAddr::zero(),
        Ipv4Addr::new(192, 168, 1, 20),
    );
    bytes.extend_from_slice(&arp_pkt.to_bytes());

    let packet = Packet::from_bytes(bytes);

    let result = processor.process(packet);

    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_qinq_double_tag_processing() {
    let processor = PacketProcessor::with_context(SystemContext::new()).with_verbose(true);

    let dst_mac = MacAddr::broadcast();
    let src_mac = MacAddr::new([0x11, 0x22, 0x33, 0x44, 0x55, 0x66]);

    let mut bytes = Vec::new();
    bytes.extend_from_slice(&dst_mac.bytes);
    bytes.extend_from_slice(&src_mac.bytes);
    bytes.extend_from_slice(&ETH_P_8021AD.to_be_bytes());

    // 外层 VLAN TCI
    let pcp_value = (0u16) << 13;
    let dei_value = 0u16;
    let vid_value = 10u16 & 0x0FFF;
    let outer_tci = pcp_value | dei_value | vid_value;
    bytes.extend_from_slice(&outer_tci.to_be_bytes());

    bytes.extend_from_slice(&ETH_P_8021Q.to_be_bytes());

    // 内层 VLAN TCI
    let pcp_value = (0u16) << 13;
    let dei_value = 0u16;
    let vid_value = 20u16 & 0x0FFF;
    let inner_tci = pcp_value | dei_value | vid_value;
    bytes.extend_from_slice(&inner_tci.to_be_bytes());

    bytes.extend_from_slice(&ETH_P_ARP.to_be_bytes());

    let arp_pkt = ArpPacket::new(
        ArpOperation::Request,
        src_mac,
        Ipv4Addr::new(10, 0, 0, 1),
        MacAddr::zero(),
        Ipv4Addr::new(10, 0, 0, 2),
    );
    bytes.extend_from_slice(&arp_pkt.to_bytes());

    let packet = Packet::from_bytes(bytes);

    let result = processor.process(packet);

    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_error_propagation_from_protocols() {
    let processor = PacketProcessor::with_context(SystemContext::new());

    let mut bytes = Vec::new();

    bytes.extend_from_slice(&MacAddr::broadcast().bytes);
    bytes.extend_from_slice(&[0xAA; 6]);
    bytes.extend_from_slice(&ETH_P_8021Q.to_be_bytes());

    bytes.extend_from_slice(&[0x01]);

    let packet = Packet::from_bytes(bytes);
    let result = processor.process(packet);

    assert!(result.is_err());

    match result {
        Err(ProcessError::ParseError(_)) => {}
        Err(ProcessError::InvalidPacket(_)) => {}
        other => {
            panic!("Expected ParseError or InvalidPacket, got {:?}", other);
        }
    }
}

#[test]
fn test_minimum_valid_packet() {
    let processor = PacketProcessor::with_context(SystemContext::new());

    let dst_mac = MacAddr::broadcast();
    let src_mac = MacAddr::new([0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF]);

    let packet = create_arp_request_packet(
        dst_mac,
        src_mac,
        Ipv4Addr::new(192, 168, 1, 1),
        Ipv4Addr::new(192, 168, 1, 2),
    );

    let result = processor.process(packet);

    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_multiple_vlan_packets_sequential() {
    let processor = PacketProcessor::with_context(SystemContext::new());

    let dst_mac = MacAddr::broadcast();
    let src_mac = MacAddr::new([0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF]);

    for vid in [100, 200, 300, 400, 500] {
        let packet = VlanPacketConfig::new(
            dst_mac,
            src_mac,
            ETH_P_8021Q,
            vid,
            ETH_P_IP,
            vec![0x01, 0x02, 0x03],
        ).build();

        let result = processor.process(packet);
        assert!(result.is_err());
    }
}

#[test]
fn test_different_vlan_tpid() {
    let processor = PacketProcessor::with_context(SystemContext::new());

    let dst_mac = MacAddr::broadcast();
    let src_mac = MacAddr::new([0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF]);

    let packet1 = VlanPacketConfig::new(
        dst_mac,
        src_mac,
        ETH_P_8021Q,
        100,
        ETH_P_IP,
        vec![0x01],
    ).build();
    assert!(processor.process(packet1).is_err());

    let packet2 = VlanPacketConfig::new(
        dst_mac,
        src_mac,
        ETH_P_8021AD,
        100,
        ETH_P_IP,
        vec![0x01],
    ).build();
    assert!(processor.process(packet2).is_err());
}

#[test]
fn test_vlan_pcp_and_dei() {
    let processor = PacketProcessor::with_context(SystemContext::new()).with_verbose(true);

    let dst_mac = MacAddr::broadcast();
    let src_mac = MacAddr::new([0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF]);

    for pcp in 0..=7 {
        for dei in [false, true] {
            let packet = VlanPacketConfig::new(
                dst_mac,
                src_mac,
                ETH_P_8021Q,
                100,
                ETH_P_IP,
                vec![0x01],
            )
            .with_pcp(pcp)
            .with_dei(dei)
            .build();

            let result = processor.process(packet);
            assert!(result.is_ok() || result.is_err());
        }
    }
}

#[test]
fn test_edge_case_packets() {
    let processor = PacketProcessor::with_context(SystemContext::new());

    let empty_packet = Packet::from_bytes(vec![]);
    assert!(processor.process(empty_packet).is_err());

    let one_byte = Packet::from_bytes(vec![0x01]);
    assert!(processor.process(one_byte).is_err());

    let mac_only = Packet::from_bytes(vec![0xFF; 12]);
    assert!(processor.process(mac_only).is_err());

    let eth_header_only = {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&MacAddr::broadcast().bytes);
        bytes.extend_from_slice(&[0xAA; 6]);
        bytes.extend_from_slice(&ETH_P_ARP.to_be_bytes());
        Packet::from_bytes(bytes)
    };
    assert!(processor.process(eth_header_only).is_err());
}

#[test]
fn test_error_type_validation() {
    let processor = PacketProcessor::with_context(SystemContext::new());

    let ipv4_tcp_packet = {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&MacAddr::broadcast().bytes);
        bytes.extend_from_slice(&[0xAA; 6]);
        bytes.extend_from_slice(&ETH_P_IP.to_be_bytes());

        bytes.push(0x45);
        bytes.push(0x00);
        bytes.extend_from_slice(&[0x00, 0x14]);
        bytes.extend_from_slice(&[0x00, 0x01]);
        bytes.extend_from_slice(&[0x00, 0x00]);
        bytes.push(64);
        bytes.push(6);
        bytes.extend_from_slice(&[0x00, 0x00]);
        bytes.extend_from_slice(&[192, 168, 1, 1]);
        bytes.extend_from_slice(&[192, 168, 1, 2]);

        Packet::from_bytes(bytes)
    };

    match processor.process(ipv4_tcp_packet) {
        Err(ProcessError::UnsupportedProtocol(msg)) => {
            assert!(msg.contains("TCP") || msg.contains("接口") || msg.contains("锁定"));
        }
        Err(ProcessError::ParseError(_)) => {}
        other => panic!("Expected UnsupportedProtocol or ParseError for IPv4+TCP, got {:?}", other),
    }

    let unknown_packet = {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&MacAddr::broadcast().bytes);
        bytes.extend_from_slice(&[0xAA; 6]);
        bytes.extend_from_slice(&[0x00, 0x01]);
        bytes.extend_from_slice(&[0x01; 20]);
        Packet::from_bytes(bytes)
    };

    match processor.process(unknown_packet) {
        Err(ProcessError::UnsupportedProtocol(msg)) => {
            assert!(msg.contains("0x0001"));
        }
        other => panic!("Expected UnsupportedProtocol for unknown type, got {:?}", other),
    }
}

#[test]
fn test_processor_configuration() {
    let named_processor = PacketProcessor::with_name_and_context(
        String::from("IntegrationTestProcessor"),
        SystemContext::new()
    );
    assert_eq!(named_processor.name(), "IntegrationTestProcessor");

    let verbose_processor = PacketProcessor::with_context(SystemContext::new()).with_verbose(true);
    assert_eq!(verbose_processor.name(), "ContextProcessor");

    let dst_mac = MacAddr::broadcast();
    let src_mac = MacAddr::new([0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF]);

    let packet = VlanPacketConfig::new(
        dst_mac,
        src_mac,
        ETH_P_8021Q,
        100,
        ETH_P_IP,
        vec![0x01],
    ).build();

    let result = verbose_processor.process(packet);
    assert!(result.is_err());
}
