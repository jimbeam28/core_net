// VLAN 协议集成测试
//
// 根据 docs/design/protocols/vlan.md 第八章的测试设计实现
// 测试 802.1Q VLAN 标签的解析和封装

use core_net::testframework::TestHarness;
use core_net::interface::MacAddr;
use core_net::protocols::vlan::{VlanTag, VlanFrame, VlanError};
use core_net::protocols::vlan::{has_vlan_tag, is_vlan_tpid, process_vlan_packet};
use core_net::common::Packet;

use serial_test::serial;

mod common;
use common::{create_test_context, inject_packet_to_context};

// ========== 测试辅助函数 ==========

/// 创建带VLAN标签的以太网帧（IPv4负载）
fn create_vlan_ipv4_packet(
    dst_mac: MacAddr,
    src_mac: MacAddr,
    vlan_tag: VlanTag,
    tpid: u16,
) -> Packet {
    use core_net::protocols::ETH_P_IP;

    let mut frame = Vec::new();

    // 以太网头
    frame.extend_from_slice(&dst_mac.bytes);
    frame.extend_from_slice(&src_mac.bytes);

    // TPID + TCI (VLAN标签)
    frame.extend_from_slice(&tpid.to_be_bytes());
    frame.extend_from_slice(&vlan_tag.to_bytes());

    // EtherType (IPv4)
    frame.extend_from_slice(&ETH_P_IP.to_be_bytes());

    // 简单的 IPv4 头部 (20字节最小头部)
    frame.extend_from_slice(&[
        0x45, 0x00, 0x00, 0x14, // Version=4, IHL=5, TOS=0, TotalLen=20
        0x00, 0x00, 0x00, 0x00, // ID=0, Flags=0, FragmentOffset=0
        0x40, 0x00, 0x00, 0x00, // TTL=64, Protocol=0, Checksum=0 (无效但测试用)
        0xc0, 0xa8, 0x01, 0x0a, // SrcIP=192.168.1.10
        0xc0, 0xa8, 0x01, 0x64, // DstIP=192.168.1.100
    ]);

    Packet::from_bytes(frame)
}

/// 创建带VLAN标签的以太网帧（ARP负载）
fn create_vlan_arp_packet(
    dst_mac: MacAddr,
    src_mac: MacAddr,
    vlan_tag: VlanTag,
) -> Packet {
    use core_net::protocols::ETH_P_ARP;

    let mut frame = Vec::new();

    // 以太网头
    frame.extend_from_slice(&dst_mac.bytes);
    frame.extend_from_slice(&src_mac.bytes);

    // TPID + TCI (VLAN标签)
    frame.extend_from_slice(&0x8100u16.to_be_bytes());
    frame.extend_from_slice(&vlan_tag.to_bytes());

    // EtherType (ARP)
    frame.extend_from_slice(&ETH_P_ARP.to_be_bytes());

    // ARP 头部
    frame.extend_from_slice(&[
        0x00, 0x01, // Hardware Type = Ethernet
        0x08, 0x00, // Protocol Type = IPv4
        0x06,       // Hardware Addr Len = 6
        0x04,       // Protocol Addr Len = 4
        0x00, 0x01, // Operation = Request
    ]);

    // SHA (Sender Hardware Addr)
    frame.extend_from_slice(&src_mac.bytes);
    // SPA (Sender Protocol Addr)
    frame.extend_from_slice(&[0xc0, 0xa8, 0x01, 0x0a]); // 192.168.1.10
    // THA (Target Hardware Addr)
    frame.extend_from_slice(&[0x00; 6]);
    // TPA (Target Protocol Addr)
    frame.extend_from_slice(&[0xc0, 0xa8, 0x01, 0x64]); // 192.168.1.100

    Packet::from_bytes(frame)
}

/// 创建双层VLAN标签的以太网帧 (Q-in-Q)
fn create_double_vlan_packet(
    dst_mac: MacAddr,
    src_mac: MacAddr,
    outer_vlan: VlanTag,
    inner_vlan: VlanTag,
) -> Packet {
    use core_net::protocols::ETH_P_IP;

    let mut frame = Vec::new();

    // 以太网头
    frame.extend_from_slice(&dst_mac.bytes);
    frame.extend_from_slice(&src_mac.bytes);

    // 外层 VLAN (0x9100 Q-in-Q)
    frame.extend_from_slice(&0x9100u16.to_be_bytes());
    frame.extend_from_slice(&outer_vlan.to_bytes());

    // 内层 VLAN (0x8100)
    frame.extend_from_slice(&0x8100u16.to_be_bytes());
    frame.extend_from_slice(&inner_vlan.to_bytes());

    // EtherType (IPv4)
    frame.extend_from_slice(&ETH_P_IP.to_be_bytes());

    // 简单的 IP 负载
    frame.extend_from_slice(&[0x00; 20]);

    Packet::from_bytes(frame)
}

/// 创建不带VLAN标签的普通以太网帧
fn create_normal_ethernet_packet(dst_mac: MacAddr, src_mac: MacAddr) -> Packet {
    use core_net::protocols::ETH_P_IP;

    let mut frame = Vec::new();

    // 以太网头
    frame.extend_from_slice(&dst_mac.bytes);
    frame.extend_from_slice(&src_mac.bytes);
    frame.extend_from_slice(&ETH_P_IP.to_be_bytes());

    // 简单的 IP 负载
    frame.extend_from_slice(&[0x00; 20]);

    Packet::from_bytes(frame)
}

// ========== 8.1 单元测试：VlanTag 创建和验证 ==========

#[test]
#[serial]
fn test_vlan_tag_creation_valid() {
    // 测试正常 VLAN ID (1-4094)
    let tag = VlanTag::new(3, true, 100);
    assert!(tag.is_ok());
    let tag = tag.unwrap();
    assert_eq!(tag.pcp, 3);
    assert!(tag.dei);
    assert_eq!(tag.vid, 100);
}

#[test]
#[serial]
fn test_vlan_tag_boundary_values() {
    // 测试边界值
    let tag_min = VlanTag::new(0, false, 1);
    assert!(tag_min.is_ok());
    assert_eq!(tag_min.unwrap().vid, 1);

    let tag_max = VlanTag::new(7, true, 4094);
    assert!(tag_max.is_ok());
    assert_eq!(tag_max.unwrap().vid, 4094);
}

#[test]
#[serial]
fn test_vlan_tag_invalid_vid() {
    // 测试无效 VLAN ID (0)
    let result = VlanTag::new(0, false, 0);
    assert!(result.is_err());
    assert!(matches!(result, Err(VlanError::InvalidVlanId { vid: 0 })));

    // 测试无效 VLAN ID (4095)
    let result = VlanTag::new(0, false, 4095);
    assert!(result.is_err());
    assert!(matches!(result, Err(VlanError::InvalidVlanId { vid: 4095 })));

    // 测试无效 VLAN ID (4096)
    let result = VlanTag::new(0, false, 4096);
    assert!(result.is_err());
}

#[test]
#[serial]
fn test_pcp_validation() {
    // 测试有效 PCP (0-7)
    for pcp in 0..=7 {
        assert!(VlanTag::is_valid_pcp(pcp));
        let tag = VlanTag::new(pcp, false, 100);
        assert!(tag.is_ok());
    }

    // 测试无效 PCP (>7)
    assert!(!VlanTag::is_valid_pcp(8));
    assert!(!VlanTag::is_valid_pcp(15));
    assert!(!VlanTag::is_valid_pcp(255));
}

#[test]
#[serial]
fn test_vlan_tag_encode_decode() {
    // 测试编码为字节后解码的一致性
    let original = VlanTag::new(5, true, 1234).unwrap();
    let bytes = original.to_bytes();
    let decoded = VlanTag::from_bytes(bytes).unwrap();

    assert_eq!(original.pcp, decoded.pcp);
    assert_eq!(original.dei, decoded.dei);
    assert_eq!(original.vid, decoded.vid);
}

#[test]
#[serial]
fn test_vlan_tag_default() {
    let tag = VlanTag::default();
    assert_eq!(tag.pcp, 0);
    assert!(!tag.dei);
    assert_eq!(tag.vid, 1);
}

// ========== 8.1 单元测试：Packet 解析和封装 ==========

#[test]
#[serial]
fn test_parse_vlan_from_packet() {
    let tag = VlanTag::new(3, true, 100).unwrap();
    let bytes = tag.to_bytes();

    let mut packet = Packet::from_bytes(bytes.to_vec());
    let parsed = VlanTag::parse_from_packet(&mut packet);

    assert!(parsed.is_ok());
    let parsed_tag = parsed.unwrap();
    assert_eq!(parsed_tag.pcp, 3);
    assert!(parsed_tag.dei);
    assert_eq!(parsed_tag.vid, 100);
}

#[test]
#[serial]
fn test_parse_vlan_offset_movement() {
    let tag = VlanTag::new(3, true, 100).unwrap();
    let bytes = tag.to_bytes();

    let mut packet = Packet::from_bytes(bytes.to_vec());
    let initial_offset = packet.offset;

    VlanTag::parse_from_packet(&mut packet).unwrap();

    // 验证 offset 移动了 2 字节
    assert_eq!(packet.offset, initial_offset + 2);
}

#[test]
#[serial]
fn test_write_vlan_to_packet() {
    let tag = VlanTag::new(3, true, 100).unwrap();
    let mut packet = Packet::new();

    tag.write_to_packet(&mut packet, 0x8100).unwrap();

    let data = packet.as_slice();
    assert_eq!(data[0], 0x81); // TPID 高字节
    assert_eq!(data[1], 0x00); // TPID 低字节

    // TCI: PCP=3, DEI=1, VID=100
    // 3 << 13 = 0x6000, 1 << 12 = 0x1000, 100 = 0x0064
    // 总计 = 0x7064
    assert_eq!(data[2], 0x70); // TCI 高字节
    assert_eq!(data[3], 0x64); // TCI 低字节
}

#[test]
#[serial]
fn test_peek_vlan_from_packet() {
    let tag = VlanTag::new(3, true, 100).unwrap();
    let bytes = tag.to_bytes();

    let packet = Packet::from_bytes(bytes.to_vec());
    let initial_offset = packet.offset;

    let parsed = VlanTag::peek_from_packet(&packet);

    assert!(parsed.is_ok());
    // 验证 offset 没有移动
    assert_eq!(packet.offset, initial_offset);
}

// ========== 8.1 单元测试：VLAN 检测 ==========

#[test]
#[serial]
fn test_has_vlan_tag_with_vlan() {
    let tag = VlanTag::new(0, false, 100).unwrap();
    let packet = create_vlan_ipv4_packet(
        MacAddr::broadcast(),
        MacAddr::new([0x00, 0x11, 0x22, 0x33, 0x44, 0x55]),
        tag,
        0x8100,
    );

    // 跳过 DST MAC 和 SRC MAC (12 字节)
    let mut packet_clone = packet.clone();
    packet_clone.seek(12);

    let tpid = has_vlan_tag(&packet_clone);
    assert_eq!(tpid, Some(0x8100));
}

#[test]
#[serial]
fn test_has_vlan_tag_without_vlan() {
    let packet = create_normal_ethernet_packet(
        MacAddr::broadcast(),
        MacAddr::new([0x00, 0x11, 0x22, 0x33, 0x44, 0x55]),
    );

    // 跳过 DST MAC 和 SRC MAC (12 字节)
    let mut packet_clone = packet.clone();
    packet_clone.seek(12);

    let tpid = has_vlan_tag(&packet_clone);
    assert_eq!(tpid, None);
}

#[test]
#[serial]
fn test_is_vlan_tpid() {
    // 测试标准 802.1Q
    assert!(is_vlan_tpid(0x8100));

    // 测试 Q-in-Q
    assert!(is_vlan_tpid(0x9100));

    // 测试 802.1ad Provider Bridge
    assert!(is_vlan_tpid(0x88A8));

    // 测试非 VLAN TPID
    assert!(!is_vlan_tpid(0x0800)); // IPv4
    assert!(!is_vlan_tpid(0x0806)); // ARP
    assert!(!is_vlan_tpid(0x86DD)); // IPv6
}

// ========== 8.2 集成测试：完整 VLAN 帧处理 ==========

#[test]
#[serial]
fn test_vlan_frame_roundtrip() {
    // 1. 创建原始 VLAN 标签
    let original_tag = VlanTag::new(5, true, 1234).unwrap();

    // 2. 创建带 VLAN 的以太网帧
    let src_mac = MacAddr::new([0x00, 0x11, 0x22, 0x33, 0x44, 0x55]);
    let dst_mac = MacAddr::new([0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0xff]);
    let packet = create_vlan_ipv4_packet(dst_mac, src_mac, original_tag, 0x8100);

    // 3. 解析 VLAN 标签
    let mut parse_packet = packet.clone();
    parse_packet.seek(12); // 跳过以太网头

    let result = process_vlan_packet(&mut parse_packet);
    assert!(result.is_ok());

    let parsed = result.unwrap();
    assert_eq!(parsed.inner_type, 0x0800); // IPv4
    assert!(parsed.outer_vlan.is_some());
    assert!(parsed.inner_vlan.is_none()); // 单层 VLAN

    let outer_tag = parsed.outer_vlan.unwrap();
    assert_eq!(outer_tag.pcp, original_tag.pcp);
    assert_eq!(outer_tag.dei, original_tag.dei);
    assert_eq!(outer_tag.vid, original_tag.vid);
}

#[test]
#[serial]
fn test_vlan_arp_packet() {
    let tag = VlanTag::new(0, false, 100).unwrap();
    let src_mac = MacAddr::new([0x00, 0x11, 0x22, 0x33, 0x44, 0x55]);
    let dst_mac = MacAddr::broadcast();

    let packet = create_vlan_arp_packet(dst_mac, src_mac, tag);

    // 解析
    let mut parse_packet = packet.clone();
    parse_packet.seek(12); // 跳过以太网头

    let result = process_vlan_packet(&mut parse_packet);
    assert!(result.is_ok());

    let parsed = result.unwrap();
    assert_eq!(parsed.inner_type, 0x0806); // ARP
    assert_eq!(parsed.outer_vlan.unwrap().vid, 100);
}

#[test]
#[serial]
fn test_double_vlan_detection() {
    let outer_vlan = VlanTag::new(3, false, 100).unwrap();
    let inner_vlan = VlanTag::new(5, true, 200).unwrap();

    let src_mac = MacAddr::new([0x00, 0x11, 0x22, 0x33, 0x44, 0x55]);
    let dst_mac = MacAddr::new([0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0xff]);

    let packet = create_double_vlan_packet(dst_mac, src_mac, outer_vlan, inner_vlan);

    // 解析
    let mut parse_packet = packet.clone();
    parse_packet.seek(12); // 跳过以太网头

    let result = process_vlan_packet(&mut parse_packet);
    assert!(result.is_ok());

    let parsed = result.unwrap();
    assert_eq!(parsed.inner_type, 0x0800); // IPv4
    assert!(parsed.outer_vlan.is_some());
    assert!(parsed.inner_vlan.is_some());

    assert_eq!(parsed.outer_vlan.unwrap().vid, 100);
    assert_eq!(parsed.inner_vlan.unwrap().vid, 200);
}

// ========== 8.3 边界测试 ==========

#[test]
#[serial]
fn test_vlan_id_zero() {
    let result = VlanTag::new(0, false, 0);
    assert!(matches!(result, Err(VlanError::InvalidVlanId { vid: 0 })));
}

#[test]
#[serial]
fn test_vlan_id_4095() {
    let result = VlanTag::new(0, false, 4095);
    assert!(matches!(result, Err(VlanError::InvalidVlanId { vid: 4095 })));
}

#[test]
#[serial]
fn test_vlan_id_max_valid() {
    let tag = VlanTag::new(0, false, 4094);
    assert!(tag.is_ok());
    assert_eq!(tag.unwrap().vid, 4094);
}

#[test]
#[serial]
fn test_insufficient_packet_length() {
    let mut packet = Packet::from_bytes(vec![0x00]); // 只有 1 字节

    let result = VlanTag::parse_from_packet(&mut packet);
    assert!(matches!(
        result,
        Err(VlanError::InsufficientPacketLength { expected: 2, actual: 1 })
    ));
}

#[test]
#[serial]
fn test_unsupported_tpid() {
    // 注意：当前实现不检查不支持的 TPID，所有 0x8100/0x9100/0x88A8 都支持
    // 此测试用于确保未来添加 TPID 验证时正确处理
    assert!(is_vlan_tpid(0x8100));
    assert!(is_vlan_tpid(0x9100));
    assert!(is_vlan_tpid(0x88A8));
}

// ========== 8.4 异常情况测试 ==========

#[test]
#[serial]
fn test_invalid_pcp_values() {
    // 测试 PCP = 8
    let result = VlanTag::new(8, false, 100);
    assert!(matches!(result, Err(VlanError::InvalidPcp { pcp: 8 })));

    // 测试 PCP = 15
    let result = VlanTag::new(15, false, 100);
    assert!(matches!(result, Err(VlanError::InvalidPcp { pcp: 15 })));

    // 测试 PCP = 255
    let result = VlanTag::new(255, false, 100);
    assert!(matches!(result, Err(VlanError::InvalidPcp { pcp: 255 })));
}

#[test]
#[serial]
fn test_malformed_vlan_tag() {
    // 创建格式错误的字节序列
    let malformed_bytes = [0xFF, 0xFF]; // 有效的字节结构，但 PCP=31 超出范围

    let result = VlanTag::from_bytes(malformed_bytes);
    // 解析时不会失败，因为 PCP 在解码后才验证
    // 但创建时应该验证
    assert!(result.is_err());
}

#[test]
#[serial]
fn test_oversized_packet() {
    // 创建超大报文
    let large_data = vec![0x00u8; 10000];
    let mut packet = Packet::from_bytes(large_data);

    // 添加 VLAN 标签
    let tag = VlanTag::new(0, false, 100).unwrap();
    let result = tag.write_to_packet(&mut packet, 0x8100);

    // 验证不会 panic，应该成功
    assert!(result.is_ok());
}

#[test]
#[serial]
fn test_empty_packet_handling() {
    let empty_packet = Packet::new();

    let result = VlanTag::parse_from_packet(&mut empty_packet.clone());
    assert!(matches!(
        result,
        Err(VlanError::InsufficientPacketLength { expected: 2, actual: 0 })
    ));
}

#[test]
#[serial]
fn test_offset_out_of_bounds() {
    let mut packet = Packet::from_bytes(vec![0x00, 0x01]);
    packet.seek(2); // offset 超出范围

    let result = VlanTag::parse_from_packet(&mut packet);
    assert!(matches!(
        result,
        Err(VlanError::InsufficientPacketLength { .. })
    ));
}

#[test]
#[serial]
fn test_write_to_full_buffer() {
    // 创建一个 Packet
    let mut packet = Packet::new();

    let tag = VlanTag::new(0, false, 100).unwrap();
    let result = tag.write_to_packet(&mut packet, 0x8100);

    // 应该成功写入 4 字节
    assert!(result.is_ok());
    assert_eq!(packet.len(), 4);
}

#[test]
#[serial]
fn test_error_recovery_after_invalid_vlan() {
    // 1. 尝试解析无效 VLAN ID
    let invalid_bytes = [0xFF, 0xFF]; // PCP=31, DEI=1, VID=2047（部分有效但整体可能有误）
    let result1 = VlanTag::from_bytes(invalid_bytes);
    assert!(result1.is_err());

    // 2. 验证可以继续解析正常数据
    let valid_bytes = [0x00, 0x64]; // PCP=0, DEI=0, VID=100
    let result2 = VlanTag::from_bytes(valid_bytes);
    assert!(result2.is_ok());
    assert_eq!(result2.unwrap().vid, 100);
}

// ========== 8.5 性能和压力测试 ==========

#[test]
#[serial]
fn test_bulk_vlan_parsing() {
    // 连续解析 100 个 VLAN 标签
    for i in 1..=100u16 {
        let vid = (i % 4094) + 1; // 确保 VID 在有效范围内
        let tag = VlanTag::new((i % 8) as u8, i % 2 == 0, vid).unwrap();
        let bytes = tag.to_bytes();
        let parsed = VlanTag::from_bytes(bytes);
        assert!(parsed.is_ok());
    }
}

#[test]
#[serial]
fn test_multiple_tpid_types() {
    let tag = VlanTag::new(3, true, 100).unwrap();

    // 测试不同 TPID
    let tpid_list = [0x8100, 0x9100, 0x88A8];

    for tpid in tpid_list {
        assert!(is_vlan_tpid(tpid));

        let mut packet = Packet::new();
        tag.write_to_packet(&mut packet, tpid).unwrap();

        let data = packet.as_slice();
        let parsed_tpid = u16::from_be_bytes([data[0], data[1]]);
        assert_eq!(parsed_tpid, tpid);
    }
}

#[test]
#[serial]
fn test_vlan_frame_structure() {
    let tag = VlanTag::new(5, true, 1234).unwrap();
    let frame = VlanFrame::new(tag, 0x9100);

    assert_eq!(frame.tpid, 0x9100);
    assert_eq!(frame.tag.vid, 1234);
    assert_eq!(frame.tag.pcp, 5);
    assert!(frame.tag.dei);
}

#[test]
#[serial]
fn test_vlan_frame_standard_8021q() {
    let tag = VlanTag::new(0, false, 100).unwrap();
    let frame = VlanFrame::standard_8021q(tag);

    assert_eq!(frame.tpid, 0x8100);
}

// ========== 8.6 测试框架使用 ==========

#[test]
#[serial]
fn test_vlan_with_test_framework() {
    // 1. 创建独立的测试上下文
    let ctx = create_test_context();

    // 2. 创建带 VLAN 的测试报文
    let tag = VlanTag::new(0, false, 100).unwrap();
    let src_mac = MacAddr::new([0x00, 0x11, 0x22, 0x33, 0x44, 0x55]);
    let dst_mac = MacAddr::broadcast();
    let packet = create_vlan_ipv4_packet(dst_mac, src_mac, tag, 0x8100);

    // 3. 注入报文到接口
    inject_packet_to_context(&ctx, "eth0", packet).unwrap();

    // 4. 运行测试线束
    let mut harness = TestHarness::with_context(ctx.clone());
    let result = harness.run();
    assert!(result.is_ok(), "调度器运行失败: {:?}", result.err());

    // 5. 验证结果（注意：VLAN 解析在以太网模块中完成）
    // 此测试主要验证带 VLAN 的报文可以正常通过系统处理
}

// ========== 补充：Packet VLAN ID 设置测试 ==========

#[test]
#[serial]
fn test_packet_vlan_id_setting() {
    let tag = VlanTag::new(3, true, 100).unwrap();
    let src_mac = MacAddr::new([0x00, 0x11, 0x22, 0x33, 0x44, 0x55]);
    let dst_mac = MacAddr::broadcast();
    let packet = create_vlan_ipv4_packet(dst_mac, src_mac, tag, 0x8100);

    // 解析后应该设置 packet.vlan_id
    let mut parse_packet = packet.clone();
    parse_packet.seek(12);

    let result = process_vlan_packet(&mut parse_packet);
    assert!(result.is_ok());

    // 验证 VLAN ID 被设置到 packet
    assert_eq!(parse_packet.vlan_id, 100);
}

#[test]
#[serial]
fn test_packet_vlan_id_qinq() {
    let outer_vlan = VlanTag::new(1, false, 100).unwrap();
    let inner_vlan = VlanTag::new(5, true, 200).unwrap();

    let src_mac = MacAddr::new([0x00, 0x11, 0x22, 0x33, 0x44, 0x55]);
    let dst_mac = MacAddr::broadcast();
    let packet = create_double_vlan_packet(dst_mac, src_mac, outer_vlan, inner_vlan);

    // 解析后应该设置内层 VLAN ID
    let mut parse_packet = packet.clone();
    parse_packet.seek(12);

    let result = process_vlan_packet(&mut parse_packet);
    assert!(result.is_ok());

    // 验证使用内层 VLAN ID
    assert_eq!(parse_packet.vlan_id, 200);
}
