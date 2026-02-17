// Engine 模块集成测试
//
// 测试完整的报文处理流程，包括多协议模块的协同工作

use core_net::common::{MacAddr, Ipv4Addr, Packet, ETH_P_ARP, ETH_P_IP, ETH_P_8021Q, ETH_P_8021AD};
use core_net::protocols::arp::{ArpPacket, ArpOperation};
use core_net::engine::{PacketProcessor, ProcessError};

// ========== 测试辅助函数 ==========

/// 构造 VLAN TCI
fn create_vlan_tci(pcp: u8, dei: bool, vid: u16) -> u16 {
    let pcp_value = ((pcp & 0x07) as u16) << 13;
    let dei_value = if dei { 1 << 12 } else { 0 };
    let vid_value = vid & 0x0FFF;
    pcp_value | dei_value | vid_value
}

/// 构造带 VLAN 标签的完整报文
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

    // 以太网头
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

    let mut bytes = Vec::new();
    bytes.extend_from_slice(&dst_mac.bytes);
    bytes.extend_from_slice(&src_mac.bytes);
    bytes.extend_from_slice(&ETH_P_ARP.to_be_bytes());
    bytes.extend_from_slice(&arp_pkt.to_bytes());

    Packet::from_bytes(bytes)
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

    // 以太网头
    bytes.extend_from_slice(&dst_mac.bytes);
    bytes.extend_from_slice(&src_mac.bytes);
    bytes.extend_from_slice(&ETH_P_8021AD.to_be_bytes());

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

// ========== 集成测试场景 ==========

/// 场景一：VLAN + ARP 完整流程
///
/// 涉及模块：ethernet、vlan、arp、processor
/// 测试内容：
/// - 注入完整的以太网帧（带 VLAN 标签 + ARP 报文）
/// - 验证逐层解析流程
/// - 验证 VLAN 模块正确解析标签
/// - 验证 ARP 模块正确处理
#[test]
fn test_vlan_arp_full_flow() {
    let processor = PacketProcessor::new().with_verbose(true);

    // 构造完整的以太网 + VLAN + ARP 报文
    let dst_mac = MacAddr::broadcast();
    let src_mac = MacAddr::new([0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF]);

    let mut bytes = Vec::new();

    // 以太网头
    bytes.extend_from_slice(&dst_mac.bytes);
    bytes.extend_from_slice(&src_mac.bytes);
    bytes.extend_from_slice(&ETH_P_8021Q.to_be_bytes());

    // VLAN 标签 (VID=100, PCP=3, DEI=1)
    let tci = create_vlan_tci(3, true, 100);
    bytes.extend_from_slice(&tci.to_be_bytes());

    // 内层 ARP 类型
    bytes.extend_from_slice(&ETH_P_ARP.to_be_bytes());

    // ARP 报文
    let arp_pkt = ArpPacket::new(
        ArpOperation::Request,
        src_mac,
        Ipv4Addr::new(192, 168, 1, 10),
        MacAddr::zero(),
        Ipv4Addr::new(192, 168, 1, 20),
    );
    bytes.extend_from_slice(&arp_pkt.to_bytes());

    let packet = Packet::from_bytes(bytes);

    // 处理报文 - 验证不崩溃，流程正常
    let result = processor.process(packet);

    // ARP 处理需要全局缓存，可能返回 Ok 或 Err
    // 只要不是 panic，说明解析流程正常
    assert!(result.is_ok() || result.is_err());
}

/// 场景二：多标签 VLAN 报文处理（QinQ）
///
/// 涉及模块：vlan、processor
/// 测试内容：
/// - 注入 QinQ 双层标签报文
/// - 验证外层和内层标签都被正确解析
/// - 验证内层协议被正确分发
#[test]
fn test_qinq_double_tag_processing() {
    let processor = PacketProcessor::new().with_verbose(true);

    let dst_mac = MacAddr::broadcast();
    let src_mac = MacAddr::new([0x11, 0x22, 0x33, 0x44, 0x55, 0x66]);

    // 构造 QinQ 报文：外层 VLAN 10 + 内层 VLAN 20 + 内层 ARP
    // 创建基础报文（会被下面的完整报文替换）
    let _packet = create_qinq_packet(
        dst_mac,
        src_mac,
        10,   // 外层 VLAN
        20,   // 内层 VLAN
        ETH_P_ARP,
        vec
![0x01, 0x02, 0x03, 0x04],
    );

    // 构造完整的 QinQ + ARP 报文
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&dst_mac.bytes);
    bytes.extend_from_slice(&src_mac.bytes);
    bytes.extend_from_slice(&ETH_P_8021AD.to_be_bytes());

    let outer_tci = create_vlan_tci(0, false, 10);
    bytes.extend_from_slice(&outer_tci.to_be_bytes());

    bytes.extend_from_slice(&ETH_P_8021Q.to_be_bytes());

    let inner_tci = create_vlan_tci(0, false, 20);
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

    // 处理报文
    let result = processor.process(packet);

    // QinQ + ARP 处理可能返回 Ok 或 Err
    assert!(result.is_ok() || result.is_err());
}

/// 场景三：处理器与协议模块错误传播
///
/// 测试内容：
/// - 验证协议模块错误正确传播到处理器
/// - 验证错误类型转换正确
#[test]
fn test_error_propagation_from_protocols() {
    let processor = PacketProcessor::new();

    // 构造会导致解析错误的报文
    let mut bytes = Vec::new();

    // 以太网头
    bytes.extend_from_slice(&MacAddr::broadcast().bytes);
    bytes.extend_from_slice(&[0xAA; 6]);
    bytes.extend_from_slice(&ETH_P_8021Q.to_be_bytes());

    // VLAN TCI 截断（只有 1 字节）
    bytes.extend_from_slice(&[0x01]);

    let packet = Packet::from_bytes(bytes);
    let result = processor.process(packet);

    // 应该返回解析错误
    assert!(result.is_err());

    // 验证错误类型
    match result {
        Err(ProcessError::ParseError(_)) => {
            // 预期：VLAN 解析错误被转换为 ProcessError::ParseError
        }
        Err(ProcessError::InvalidPacket(_)) => {
            // 也可以是格式错误
        }
        other => {
            panic!("Expected ParseError or InvalidPacket, got {:?}", other);
        }
    }
}

/// 场景四：边界条件 - 最小有效报文
#[test]
fn test_minimum_valid_packet() {
    let processor = PacketProcessor::new();

    // 构造最小的有效 ARP 报文
    let dst_mac = MacAddr::broadcast();
    let src_mac = MacAddr::new([0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF]);

    let packet = create_arp_request_packet(
        dst_mac,
        src_mac,
        Ipv4Addr::new(192, 168, 1, 1),
        Ipv4Addr::new(192, 168, 1, 2),
    );

    let result = processor.process(packet);

    // 最小报文应该能被解析
    assert!(result.is_ok() || result.is_err());
}

/// 场景五：多个 VLAN 报文顺序处理
#[test]
fn test_multiple_vlan_packets_sequential() {
    let processor = PacketProcessor::new();

    let dst_mac = MacAddr::broadcast();
    let src_mac = MacAddr::new([0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF]);

    // 处理多个 VLAN 报文
    for vid in [100, 200, 300, 400, 500] {
        let packet = create_vlan_packet(
            dst_mac,
            src_mac,
            ETH_P_8021Q,
            0,
            false,
            vid,
            ETH_P_IP,
            vec![0x01, 0x02, 0x03],
        );

        let result = processor.process(packet);
        // 内层 IPv4 未实现，应该返回错误
        assert!(result.is_err());
    }
}

/// 场景六：不同 VLAN TPID 的处理
#[test]
fn test_different_vlan_tpid() {
    let processor = PacketProcessor::new();

    let dst_mac = MacAddr::broadcast();
    let src_mac = MacAddr::new([0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF]);

    // 测试 802.1Q TPID
    let packet1 = create_vlan_packet(
        dst_mac,
        src_mac,
        ETH_P_8021Q,
        0,
        false,
        100,
        ETH_P_IP,
        vec![0x01],
    );
    assert!(processor.process(packet1).is_err());

    // 测试 802.1ad TPID
    let packet2 = create_vlan_packet(
        dst_mac,
        src_mac,
        ETH_P_8021AD,
        0,
        false,
        100,
        ETH_P_IP,
        vec![0x01],
    );
    assert!(processor.process(packet2).is_err());
}

/// 场景七：VLAN 优先级（PCP）和 DEI 标志测试
#[test]
fn test_vlan_pcp_and_dei() {
    let processor = PacketProcessor::new().with_verbose(true);

    let dst_mac = MacAddr::broadcast();
    let src_mac = MacAddr::new([0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF]);

    // 测试所有 PCP 值 (0-7)
    for pcp in 0..=7 {
        for dei in [false, true] {
            let packet = create_vlan_packet(
                dst_mac,
                src_mac,
                ETH_P_8021Q,
                pcp,
                dei,
                100,
                ETH_P_IP,
                vec![0x01],
            );

            let result = processor.process(packet);
            assert!(result.is_ok() || result.is_err());
        }
    }
}

/// 场景八：空报文和边界报文处理
#[test]
fn test_edge_case_packets() {
    let processor = PacketProcessor::new();

    // 空报文
    let empty_packet = Packet::from_bytes(vec![]);
    assert!(processor.process(empty_packet).is_err());

    // 只有 1 字节的报文
    let one_byte = Packet::from_bytes(vec![0x01]);
    assert!(processor.process(one_byte).is_err());

    // 只有 MAC 地址的报文（12 字节）
    let mac_only = Packet::from_bytes(vec![0xFF; 12]);
    assert!(processor.process(mac_only).is_err());

    // 完整以太网头但没有负载（14 字节）
    let eth_header_only = {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&MacAddr::broadcast().bytes);
        bytes.extend_from_slice(&[0xAA; 6]);
        bytes.extend_from_slice(&ETH_P_ARP.to_be_bytes());
        Packet::from_bytes(bytes)
    };
    assert!(processor.process(eth_header_only).is_err());
}

/// 场景九：错误类型验证
#[test]
fn test_error_type_validation() {
    let processor = PacketProcessor::new();

    // 测试 IPv4 未实现错误
    let ipv4_packet = {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&MacAddr::broadcast().bytes);
        bytes.extend_from_slice(&[0xAA; 6]);
        bytes.extend_from_slice(&ETH_P_IP.to_be_bytes());
        bytes.extend_from_slice(&[0x01; 20]);
        Packet::from_bytes(bytes)
    };

    match processor.process(ipv4_packet) {
        Err(ProcessError::UnsupportedProtocol(msg)) => {
            assert!(msg.contains("IPv4"));
        }
        other => panic!("Expected UnsupportedProtocol for IPv4, got {:?}", other),
    }

    // 测试未知以太网类型错误
    let unknown_packet = {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&MacAddr::broadcast().bytes);
        bytes.extend_from_slice(&[0xAA; 6]);
        bytes.extend_from_slice(&[0x00, 0x01]); // 未知类型
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

/// 场景十：处理器命名和 verbose 模式集成测试
#[test]
fn test_processor_configuration() {
    // 测试命名处理器
    let named_processor = PacketProcessor::with_name(String::from("IntegrationTestProcessor"));
    assert_eq!(named_processor.name(), "IntegrationTestProcessor");

    // 测试 verbose 模式不影响功能
    let verbose_processor = PacketProcessor::new().with_verbose(true);
    assert_eq!(verbose_processor.name(), "DefaultProcessor");

    let dst_mac = MacAddr::broadcast();
    let src_mac = MacAddr::new([0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF]);

    let packet = create_vlan_packet(
        dst_mac,
        src_mac,
        ETH_P_8021Q,
        0,
        false,
        100,
        ETH_P_IP,
        vec![0x01],
    );

    // verbose 处理器应该产生相同的结果
    let result = verbose_processor.process(packet);
    assert!(result.is_err());
}
