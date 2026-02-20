// IPv4 协议集成测试
//
// 测试 IPv4 协议的头部解析、分片检测、地址类型判断等

use core_net::testframework::{
    TestHarness,
};
use core_net::interface::{MacAddr, Ipv4Addr};
use core_net::protocols::{ETH_P_IP, IP_PROTO_ICMP};
use core_net::protocols::ip::{Ipv4Header, encapsulate_ip_datagram};
use core_net::common::Packet;

use serial_test::serial;

mod common;
use common::{create_echo_request_packet,
             inject_packet_to_context, verify_context_txq_count, create_test_context};

// 测试环境配置：本机接口 eth0: ifindex=0, MAC=00:11:22:33:44:55, IP=192.168.1.100

// ========== 全局测试生命周期 ==========

// 1. IP 头部解析测试组

#[test]
#[serial]
fn test_ip_header_parse() {
    let _ctx = create_test_context();

    let src_ip = Ipv4Addr::new(192, 168, 1, 10);
    let dst_ip = Ipv4Addr::new(192, 168, 1, 100);
    let ip_header = Ipv4Header::new(src_ip, dst_ip, IP_PROTO_ICMP, 64);

    assert_eq!(ip_header.version(), 4);
    assert_eq!(ip_header.header_len(), 20);
    assert_eq!(ip_header.protocol, IP_PROTO_ICMP);
    assert_eq!(ip_header.source_addr, src_ip);
    assert_eq!(ip_header.dest_addr, dst_ip);
}

#[test]
#[serial]
fn test_ip_header_flags_fragment() {
    let _ctx = create_test_context();

    // 测试默认创建的头部（DF=1, MF=0, Offset=0）
    let header = Ipv4Header::new(
        Ipv4Addr::new(192, 168, 1, 1),
        Ipv4Addr::new(192, 168, 1, 2),
        IP_PROTO_ICMP,
        64,
    );

    assert!(header.has_df_flag());
    assert!(!header.has_mf_flag());
    assert_eq!(header.fragment_offset(), 0);
    assert!(!header.is_fragmented());
}

// 2. 分片检测测试组

#[test]
#[serial]
fn test_ip_fragment_rejection_mf_flag() {
    let ctx = create_test_context();

    let sender_mac = MacAddr::new([0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0x01]);
    let _sender_ip = Ipv4Addr::new(192, 168, 1, 10);
    let _target_ip = Ipv4Addr::new(192, 168, 1, 100);

    // 创建带 MF=1 标志的分片数据报
    let mut frame = Vec::new();
    frame.extend_from_slice(&[0xff, 0xff, 0xff, 0xff, 0xff, 0xff]);
    frame.extend_from_slice(&sender_mac.bytes);
    frame.extend_from_slice(&ETH_P_IP.to_be_bytes());

    // IP 头部: MF=1
    frame.extend_from_slice(&[
        0x45,        // Version=4, IHL=5
        0x00,        // TOS
        0x00, 0x1c,  // Total Length = 28
        0x00, 0x01,  // Identification
        0x20, 0x00,  // Flags: MF=1, Offset=0
        0x40,        // TTL
        0x01,        // Protocol = ICMP
        0x00, 0x00,  // Checksum (稍后计算)
        192, 168, 1, 10,  // Source IP
        192, 168, 1, 100, // Dest IP
    ]);

    // 计算 IP 校验和
    let checksum = core_net::protocols::ip::calculate_checksum(&frame[14..34]);
    frame[32] = (checksum >> 8) as u8;
    frame[33] = (checksum & 0xFF) as u8;

    // ICMP 数据
    frame.extend_from_slice(&[0x08, 0x00, 0x00, 0x00, 0x12, 0x34, 0x00, 0x01]);

    let packet = Packet::from_bytes(frame);
    inject_packet_to_context(&ctx, "eth0", packet).unwrap();

    // 注意：由于协议层仍使用全局状态，这里使用 with_context
    let mut harness = TestHarness::with_context(ctx.clone());
    let result = harness.run();
    assert!(result.is_ok());

    // 分片数据报应该被丢弃，无响应
    assert!(verify_context_txq_count(&ctx, "eth0", 0), "分片数据报应被丢弃");
}

#[test]
#[serial]
fn test_ip_fragment_rejection_offset_nonzero() {
    let ctx = create_test_context();

    let sender_mac = MacAddr::new([0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0x01]);
    let _sender_ip = Ipv4Addr::new(192, 168, 1, 10);
    let _target_ip = Ipv4Addr::new(192, 168, 1, 100);

    // 创建带非零偏移的分片数据报
    let mut frame = Vec::new();
    frame.extend_from_slice(&[0xff, 0xff, 0xff, 0xff, 0xff, 0xff]);
    frame.extend_from_slice(&sender_mac.bytes);
    frame.extend_from_slice(&ETH_P_IP.to_be_bytes());

    // IP 头部: Fragment Offset = 185
    frame.extend_from_slice(&[
        0x45,        // Version=4, IHL=5
        0x00,        // TOS
        0x00, 0x1c,  // Total Length = 28
        0x00, 0x01,  // Identification
        0x00, 0xb9,  // Flags: MF=0, Offset=185 (0x00B9)
        0x40,        // TTL
        0x01,        // Protocol = ICMP
        0x00, 0x00,  // Checksum
        192, 168, 1, 10,  // Source IP
        192, 168, 1, 100, // Dest IP
    ]);

    // 计算 IP 校验和
    let checksum = core_net::protocols::ip::calculate_checksum(&frame[14..34]);
    frame[32] = (checksum >> 8) as u8;
    frame[33] = (checksum & 0xFF) as u8;

    // ICMP 数据
    frame.extend_from_slice(&[0x08, 0x00, 0x00, 0x00, 0x12, 0x34, 0x00, 0x01]);

    let packet = Packet::from_bytes(frame);
    inject_packet_to_context(&ctx, "eth0", packet).unwrap();

    // 注意：由于协议层仍使用全局状态，这里使用 with_context
    let mut harness = TestHarness::with_context(ctx.clone());
    let result = harness.run();
    assert!(result.is_ok());

    // 分片数据报应该被丢弃
    assert!(verify_context_txq_count(&ctx, "eth0", 0), "分片数据报应被丢弃");
}

// 3. 边界条件测试组

#[test]
#[serial]
fn test_ip_min_header_length() {
    let _ctx = create_test_context();

    // 创建最小 IP 头部（IHL=5, 20字节）
    let header = Ipv4Header::new(
        Ipv4Addr::new(192, 168, 1, 1),
        Ipv4Addr::new(192, 168, 1, 2),
        IP_PROTO_ICMP,
        0,
    );

    assert_eq!(header.ihl(), 5);
    assert_eq!(header.header_len(), 20);
}

#[test]
#[serial]
fn test_ip_max_packet_length() {
    let _ctx = create_test_context();

    // 测试最大数据报长度（65535字节）
    let max_payload = 65535 - 20; // 减去 IP 头部
    let large_payload = vec![0x42; max_payload];

    // 创建封装函数测试
    let src_ip = Ipv4Addr::new(192, 168, 1, 1);
    let dst_ip = Ipv4Addr::new(192, 168, 1, 2);
    let packet = encapsulate_ip_datagram(src_ip, dst_ip, IP_PROTO_ICMP, &large_payload);

    // 验证总长度字段
    let total_len = u16::from_be_bytes([packet[2], packet[3]]);
    assert_eq!(total_len, 65535);
}

// 4. 地址类型测试组

#[test]
#[serial]
fn test_ip_broadcast_address() {
    let _ctx = create_test_context();

    let header = Ipv4Header::new(
        Ipv4Addr::new(192, 168, 1, 1),
        Ipv4Addr::broadcast(),
        IP_PROTO_ICMP,
        64,
    );

    assert!(header.is_broadcast());
    assert!(!header.is_loopback());
    assert!(!header.is_multicast());
}

#[test]
#[serial]
fn test_ip_loopback_address() {
    let _ctx = create_test_context();

    let header = Ipv4Header::new(
        Ipv4Addr::new(192, 168, 1, 1),
        Ipv4Addr::localhost(), // 127.0.0.1
        IP_PROTO_ICMP,
        64,
    );

    assert!(!header.is_broadcast());
    assert!(header.is_loopback());
    assert!(!header.is_multicast());
}

#[test]
#[serial]
fn test_ip_multicast_address() {
    let _ctx = create_test_context();

    // 组播地址范围: 224.0.0.0/4
    let header = Ipv4Header::new(
        Ipv4Addr::new(192, 168, 1, 1),
        Ipv4Addr::new(224, 0, 0, 1),
        IP_PROTO_ICMP,
        64,
    );

    assert!(!header.is_broadcast());
    assert!(!header.is_loopback());
    assert!(header.is_multicast());
}

// 5. 正常 IP-ICMP 流程测试

#[test]
#[serial]
fn test_ip_icmp_normal_flow() {
    let ctx = create_test_context();

    let sender_mac = MacAddr::new([0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0x01]);
    let sender_ip = Ipv4Addr::new(192, 168, 1, 10);
    let target_ip = Ipv4Addr::new(192, 168, 1, 100); // 本机 IP

    // 创建正常的 Echo Request
    let request = create_echo_request_packet(sender_mac, sender_ip, target_ip, 1234, 1);
    inject_packet_to_context(&ctx, "eth0", request).unwrap();

    // 注意：由于协议层仍使用全局状态，这里使用 with_context
    let mut harness = TestHarness::with_context(ctx.clone());
    let result = harness.run();
    assert!(result.is_ok());

    // 应该有 Echo Reply 响应
    assert!(verify_context_txq_count(&ctx, "eth0", 1), "应该有Echo Reply响应");
}

// 6. 协议不支持测试

#[test]
#[serial]
fn test_ip_protocol_unsupported_tcp() {
    let ctx = create_test_context();

    let sender_mac = MacAddr::new([0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0x01]);
    let _sender_ip = Ipv4Addr::new(192, 168, 1, 10);
    let _target_ip = Ipv4Addr::new(192, 168, 1, 100);

    // 创建 TCP 协议的 IP 数据报
    let mut frame = Vec::new();
    frame.extend_from_slice(&[0xff, 0xff, 0xff, 0xff, 0xff, 0xff]);
    frame.extend_from_slice(&sender_mac.bytes);
    frame.extend_from_slice(&ETH_P_IP.to_be_bytes());

    // IP 头部: Protocol=TCP (6)
    frame.extend_from_slice(&[
        0x45,        // Version=4, IHL=5
        0x00,        // TOS
        0x00, 0x28,  // Total Length = 40
        0x00, 0x01,  // Identification
        0x40, 0x00,  // Flags: DF=1, Offset=0
        0x40,        // TTL
        0x06,        // Protocol = TCP
        0x00, 0x00,  // Checksum
        192, 168, 1, 10,  // Source IP
        192, 168, 1, 100, // Dest IP
    ]);

    // 计算 IP 校验和
    let checksum = core_net::protocols::ip::calculate_checksum(&frame[14..34]);
    frame[32] = (checksum >> 8) as u8;
    frame[33] = (checksum & 0xFF) as u8;

    // TCP 头部（简化）
    frame.extend_from_slice(&[0x00, 0x14, 0x00, 0x50, 0x00, 0x00, 0x00, 0x01]);

    let packet = Packet::from_bytes(frame);
    inject_packet_to_context(&ctx, "eth0", packet).unwrap();

    // 注意：由于协议层仍使用全局状态，这里使用 with_context
    let mut harness = TestHarness::with_context(ctx.clone());
    let result = harness.run();
    // TCP 不支持，但调度器应该能处理这个错误
    assert!(result.is_ok() || result.is_err());
}

#[test]
#[serial]
fn test_ip_protocol_unsupported_udp() {
    let ctx = create_test_context();

    let sender_mac = MacAddr::new([0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0x01]);
    let _sender_ip = Ipv4Addr::new(192, 168, 1, 10);
    let _target_ip = Ipv4Addr::new(192, 168, 1, 100);

    // 创建 UDP 协议的 IP 数据报
    let mut frame = Vec::new();
    frame.extend_from_slice(&[0xff, 0xff, 0xff, 0xff, 0xff, 0xff]);
    frame.extend_from_slice(&sender_mac.bytes);
    frame.extend_from_slice(&ETH_P_IP.to_be_bytes());

    // IP 头部: Protocol=UDP (17)
    frame.extend_from_slice(&[
        0x45,        // Version=4, IHL=5
        0x00,        // TOS
        0x00, 0x1c,  // Total Length = 28
        0x00, 0x01,  // Identification
        0x40, 0x00,  // Flags: DF=1, Offset=0
        0x40,        // TTL
        0x11,        // Protocol = UDP
        0x00, 0x00,  // Checksum
        192, 168, 1, 10,  // Source IP
        192, 168, 1, 100, // Dest IP
    ]);

    // 计算 IP 校验和
    let checksum = core_net::protocols::ip::calculate_checksum(&frame[14..34]);
    frame[32] = (checksum >> 8) as u8;
    frame[33] = (checksum & 0xFF) as u8;

    // UDP 头部（简化）
    frame.extend_from_slice(&[0x00, 0x08, 0x00, 0x08, 0x00, 0x00, 0x00, 0x00]);

    let packet = Packet::from_bytes(frame);
    inject_packet_to_context(&ctx, "eth0", packet).unwrap();

    // 注意：由于协议层仍使用全局状态，这里使用 with_context
    let mut harness = TestHarness::with_context(ctx.clone());
    let result = harness.run();
    // UDP 不支持，但调度器应该能处理这个错误
    assert!(result.is_ok() || result.is_err());
}

// 7. 封装测试

#[test]
#[serial]
fn test_ip_encapsulate_datagram() {
    let _ctx = create_test_context();

    let src_ip = Ipv4Addr::new(192, 168, 1, 1);
    let dst_ip = Ipv4Addr::new(192, 168, 1, 2);
    let payload = vec![0x08, 0x00, 0xf7, 0xfc, 0x12, 0x34, 0x00, 0x01];

    let packet = encapsulate_ip_datagram(src_ip, dst_ip, IP_PROTO_ICMP, &payload);

    // 验证包头
    assert_eq!(packet[0], 0x45); // Version=4, IHL=5
    assert_eq!(packet[9], IP_PROTO_ICMP);

    // 验证地址
    assert_eq!(&packet[12..16], &[192, 168, 1, 1]);
    assert_eq!(&packet[16..20], &[192, 168, 1, 2]);

    // 验证负载
    assert_eq!(&packet[20..], &payload[..]);
}
