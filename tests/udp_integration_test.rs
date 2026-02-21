// UDP 协议集成测试
//
// 测试 UDP 协议的数据报解析、封装、校验和验证

use core_net::testframework::TestHarness;
use core_net::common::Packet;
use core_net::interface::{MacAddr, Ipv4Addr};
use core_net::protocols::ETH_P_IP;
use core_net::protocols::udp::{UdpDatagram, UdpHeader, encapsulate_udp_datagram};
use serial_test::serial;

mod common;
use common::{create_ip_header, inject_packet_to_context, verify_context_txq_count,
             create_test_context};

// 测试环境配置：本机接口 eth0: ifindex=0, MAC=00:11:22:33:44:55, IP=192.168.1.100

// ========== 基本功能测试组 ==========

#[test]
#[serial]
fn test_udp_basic_send_receive() {
    let ctx = create_test_context();

    let sender_mac = MacAddr::new([0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0x01]);
    let sender_ip = Ipv4Addr::new(192, 168, 1, 10);
    let target_ip = Ipv4Addr::new(192, 168, 1, 100); // 本机 IP

    // 创建 UDP 数据报
    let udp_data = encapsulate_udp_datagram(
        1234,  // 源端口
        5678,  // 目标端口
        sender_ip,
        target_ip,
        b"Hello, UDP!",
        true,   // 计算校验和
    );

    // IP 封装
    let mut ip_data = create_ip_header(sender_ip, target_ip, udp_data.len());
    ip_data.extend_from_slice(&udp_data);

    // 以太网封装
    let mut frame = Vec::new();
    frame.extend_from_slice(&[0xff, 0xff, 0xff, 0xff, 0xff, 0xff]);
    frame.extend_from_slice(&sender_mac.bytes);
    frame.extend_from_slice(&ETH_P_IP.to_be_bytes());
    frame.extend_from_slice(&ip_data);

    let packet = Packet::from_bytes(frame);
    inject_packet_to_context(&ctx, "eth0", packet).unwrap();

    // 运行调度器
    let mut harness = TestHarness::with_context(ctx.clone());
    let result = harness.run();
    assert!(result.is_ok(), "调度器运行失败: {:?}", result.err());

    // UDP 是无连接协议，通常不发送响应
    // 但当前实现会将数据输出到 TxQ 用于验证
    assert!(verify_context_txq_count(&ctx, "eth0", 0), "UDP 不应有响应");
}

#[test]
#[serial]
fn test_udp_header_parse() {
    let bytes = [
        0x04, 0xD2, // Source Port: 1234
        0x16, 0x2E, // Dest Port: 5678
        0x00, 0x0C, // Length: 12
        0xAB, 0xCD, // Checksum: 0xABCD
        0x01, 0x02, 0x03, 0x04, // Payload
    ];

    let header = UdpHeader::parse(&bytes).unwrap();
    assert_eq!(header.source_port, 1234);
    assert_eq!(header.destination_port, 5678);
    assert_eq!(header.length, 12);
    assert_eq!(header.checksum, 0xABCD);
}

#[test]
#[serial]
fn test_udp_datagram_parse() {
    let bytes = [
        0x04, 0xD2, // Source Port: 1234
        0x16, 0x2E, // Dest Port: 5678
        0x00, 0x0C, // Length: 12 (8 + 4)
        0x00, 0x00, // Checksum
        0x48, 0x65, 0x6C, 0x6C, // "Hell" (4 bytes)
    ];

    let datagram = UdpDatagram::parse(&bytes).unwrap();
    assert_eq!(datagram.header.source_port, 1234);
    assert_eq!(datagram.header.destination_port, 5678);
    assert_eq!(datagram.payload, b"Hell");
}

// ========== 边界条件测试组 ==========

#[test]
#[serial]
fn test_udp_minimal_datagram() {
    let ctx = create_test_context();

    let sender_mac = MacAddr::new([0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0x01]);
    let sender_ip = Ipv4Addr::new(192, 168, 1, 10);
    let target_ip = Ipv4Addr::new(192, 168, 1, 100);

    // 最小 UDP 数据报（仅头部，无数据）
    let udp_data = encapsulate_udp_datagram(
        1234,
        5678,
        sender_ip,
        target_ip,
        b"",    // 空载荷
        false,
    );

    // 验证长度
    assert_eq!(udp_data.len(), 8); // 仅头部

    let mut ip_data = create_ip_header(sender_ip, target_ip, udp_data.len());
    ip_data.extend_from_slice(&udp_data);

    let mut frame = Vec::new();
    frame.extend_from_slice(&[0xff, 0xff, 0xff, 0xff, 0xff, 0xff]);
    frame.extend_from_slice(&sender_mac.bytes);
    frame.extend_from_slice(&ETH_P_IP.to_be_bytes());
    frame.extend_from_slice(&ip_data);

    let packet = Packet::from_bytes(frame);
    inject_packet_to_context(&ctx, "eth0", packet).unwrap();

    let mut harness = TestHarness::with_context(ctx.clone());
    let result = harness.run();
    assert!(result.is_ok());
}

#[test]
#[serial]
fn test_udp_zero_source_port() {
    let bytes = [
        0x00, 0x00, // Source Port: 0 (未使用)
        0x16, 0x2E, // Dest Port: 5678
        0x00, 0x08, // Length: 8
        0x00, 0x00, // Checksum
    ];

    let datagram = UdpDatagram::parse(&bytes).unwrap();
    assert_eq!(datagram.header.source_port, 0);
    assert_eq!(datagram.header.destination_port, 5678);
    assert!(datagram.payload.is_empty());
}

#[test]
#[serial]
fn test_udp_odd_length_payload() {
    let _sender_ip = Ipv4Addr::new(192, 168, 1, 1);
    let _dest_ip = Ipv4Addr::new(192, 168, 1, 2);

    // 奇数长度载荷
    let datagram = UdpDatagram::create(1234, 5678, b"ABC"); // 3 bytes

    let checksum = datagram.calculate_checksum(_sender_ip, _dest_ip);
    // 验证校验和计算不崩溃
    assert_ne!(checksum, 0);
}

#[test]
#[serial]
fn test_udp_large_payload() {
    let sender_ip = Ipv4Addr::new(192, 168, 1, 1);
    let dest_ip = Ipv4Addr::new(192, 168, 1, 2);

    // 较大载荷（100 字节）
    let large_payload = vec![0x42u8; 100];
    let datagram = UdpDatagram::create(1234, 5678, &large_payload);

    assert_eq!(datagram.len(), 108); // 8 + 100

    let checksum = datagram.calculate_checksum(sender_ip, dest_ip);
    assert_ne!(checksum, 0);
}

// ========== 异常情况测试组 ==========

#[test]
#[serial]
fn test_udp_invalid_length() {
    // 长度字段小于 8
    let bytes = [
        0x04, 0xD2, // Source Port
        0x16, 0x2E, // Dest Port
        0x00, 0x07, // Length: 7 (invalid, < 8)
        0x00, 0x00, // Checksum
    ];

    let result = UdpDatagram::parse(&bytes);
    assert!(result.is_err());
}

#[test]
#[serial]
fn test_udp_data_too_short() {
    let ctx = create_test_context();

    let sender_mac = MacAddr::new([0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0x01]);
    let sender_ip = Ipv4Addr::new(192, 168, 1, 10);
    let target_ip = Ipv4Addr::new(192, 168, 1, 100);

    // 创建长度字段与实际数据不符的 UDP 数据报
    let udp_data = vec![
        0x04, 0xD2, // Source Port
        0x16, 0x2E, // Dest Port
        0x00, 0x14, // Length: 20
        0x00, 0x00, // Checksum
        0x01, 0x02, 0x03, 0x04, // 只有 4 字节数据
    ];

    let mut ip_data = create_ip_header(sender_ip, target_ip, udp_data.len());
    ip_data.extend_from_slice(&udp_data);

    let mut frame = Vec::new();
    frame.extend_from_slice(&[0xff, 0xff, 0xff, 0xff, 0xff, 0xff]);
    frame.extend_from_slice(&sender_mac.bytes);
    frame.extend_from_slice(&ETH_P_IP.to_be_bytes());
    frame.extend_from_slice(&ip_data);

    let packet = Packet::from_bytes(frame);
    inject_packet_to_context(&ctx, "eth0", packet).unwrap();

    let mut harness = TestHarness::with_context(ctx.clone());
    let result = harness.run();
    // 应该静默丢弃（不响应）
    assert!(result.is_ok());
}

#[test]
#[serial]
fn test_udp_checksum_validation() {
    let ctx = create_test_context();

    let sender_mac = MacAddr::new([0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0x01]);
    let sender_ip = Ipv4Addr::new(192, 168, 1, 10);
    let target_ip = Ipv4Addr::new(192, 168, 1, 100);

    // 创建带校验和的 UDP 数据报
    let udp_data = encapsulate_udp_datagram(
        1234,
        5678,
        sender_ip,
        target_ip,
        b"Test",
        true,   // 计算校验和
    );

    let mut ip_data = create_ip_header(sender_ip, target_ip, udp_data.len());
    ip_data.extend_from_slice(&udp_data);

    let mut frame = Vec::new();
    frame.extend_from_slice(&[0xff, 0xff, 0xff, 0xff, 0xff, 0xff]);
    frame.extend_from_slice(&sender_mac.bytes);
    frame.extend_from_slice(&ETH_P_IP.to_be_bytes());
    frame.extend_from_slice(&ip_data);

    let packet = Packet::from_bytes(frame);
    inject_packet_to_context(&ctx, "eth0", packet).unwrap();

    let mut harness = TestHarness::with_context(ctx.clone());
    let result = harness.run();
    assert!(result.is_ok());
}

#[test]
#[serial]
fn test_udp_checksum_error() {
    let ctx = create_test_context();

    let sender_mac = MacAddr::new([0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0x01]);
    let sender_ip = Ipv4Addr::new(192, 168, 1, 10);
    let target_ip = Ipv4Addr::new(192, 168, 1, 100);

    // 创建带错误校验和的 UDP 数据报
    let mut udp_data = encapsulate_udp_datagram(
        1234,
        5678,
        sender_ip,
        target_ip,
        b"Test",
        false,  // 不计算校验和
    );

    // 修改校验和为错误的值
    udp_data[6] = 0xFF;
    udp_data[7] = 0xFF;

    let mut ip_data = create_ip_header(sender_ip, target_ip, udp_data.len());
    ip_data.extend_from_slice(&udp_data);

    let mut frame = Vec::new();
    frame.extend_from_slice(&[0xff, 0xff, 0xff, 0xff, 0xff, 0xff]);
    frame.extend_from_slice(&sender_mac.bytes);
    frame.extend_from_slice(&ETH_P_IP.to_be_bytes());
    frame.extend_from_slice(&ip_data);

    let packet = Packet::from_bytes(frame);
    inject_packet_to_context(&ctx, "eth0", packet).unwrap();

    let mut harness = TestHarness::with_context(ctx.clone());
    let result = harness.run();
    // 校验和错误，应该静默丢弃
    assert!(result.is_ok());
}

#[test]
#[serial]
fn test_udp_zero_checksum() {
    let bytes = [
        0x04, 0xD2, // Source Port
        0x16, 0x2E, // Dest Port
        0x00, 0x0C, // Length
        0x00, 0x00, // Checksum: 0 (IPv4 允许)
        0x01, 0x02, 0x03, 0x04,
    ];

    let datagram = UdpDatagram::parse(&bytes).unwrap();

    // 零校验和在强制模式下应该失败
    assert!(!datagram.verify_checksum(
        Ipv4Addr::new(192, 168, 1, 1),
        Ipv4Addr::new(192, 168, 1, 2),
        true
    ));

    // 零校验和在非强制模式下应该通过
    assert!(datagram.verify_checksum(
        Ipv4Addr::new(192, 168, 1, 1),
        Ipv4Addr::new(192, 168, 1, 2),
        false
    ));
}

// ========== 多端口测试组 ==========

#[test]
#[serial]
fn test_udp_multiple_ports() {
    let _sender_ip = Ipv4Addr::new(192, 168, 1, 1);
    let _dest_ip = Ipv4Addr::new(192, 168, 1, 2);

    // 测试不同的端口号组合
    let test_cases = vec![
        (53, 5353),      // DNS
        (123, 1234),     // NTP
        (161, 161),      // SNMP
        (8080, 80),      // HTTP 代理
    ];

    for (src_port, dst_port) in test_cases {
        let datagram = UdpDatagram::create(src_port, dst_port, b"test");
        assert_eq!(datagram.header.source_port, src_port);
        assert_eq!(datagram.header.destination_port, dst_port);
    }
}

// ========== 非本机 IP 测试组 ==========

#[test]
#[serial]
fn test_udp_not_for_us() {
    let ctx = create_test_context();

    let sender_mac = MacAddr::new([0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0x01]);
    let sender_ip = Ipv4Addr::new(192, 168, 1, 10);
    let target_ip = Ipv4Addr::new(192, 168, 1, 200); // 不是本机 IP

    let udp_data = encapsulate_udp_datagram(
        1234,
        5678,
        sender_ip,
        target_ip,
        b"Hello",
        false,
    );

    let mut ip_data = create_ip_header(sender_ip, target_ip, udp_data.len());
    ip_data.extend_from_slice(&udp_data);

    let mut frame = Vec::new();
    frame.extend_from_slice(&[0xff, 0xff, 0xff, 0xff, 0xff, 0xff]);
    frame.extend_from_slice(&sender_mac.bytes);
    frame.extend_from_slice(&ETH_P_IP.to_be_bytes());
    frame.extend_from_slice(&ip_data);

    let packet = Packet::from_bytes(frame);
    inject_packet_to_context(&ctx, "eth0", packet).unwrap();

    let mut harness = TestHarness::with_context(ctx.clone());
    let result = harness.run();
    assert!(result.is_ok());

    // 目标不是本机，不应该处理
    assert!(verify_context_txq_count(&ctx, "eth0", 0));
}

// ========== 端口边界测试组 ==========

#[test]
#[serial]
fn test_udp_port_boundaries() {
    let bytes = [
        0x00, 0x00, // Port: 0
        0xFF, 0xFF, // Port: 65535
        0x00, 0x08, // Length: 8
        0x00, 0x00, // Checksum
    ];

    let datagram = UdpDatagram::parse(&bytes).unwrap();
    assert_eq!(datagram.header.source_port, 0);
    assert_eq!(datagram.header.destination_port, 65535);
}
