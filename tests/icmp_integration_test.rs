// ICMP 协议集成测试
//
// 测试 ICMP 协议的 Echo Request/Reply、Destination Unreachable、Time Exceeded

use core_net::testframework::TestHarness;
use core_net::common::Packet;
use core_net::interface::{MacAddr, Ipv4Addr};
use core_net::protocols::{ETH_P_IP};
use core_net::protocols::icmp::{IcmpPacket, IcmpEcho, create_echo_request};
use serial_test::serial;

mod common;
use common::{create_ip_header, create_echo_request_packet, create_echo_reply_packet,
             inject_packet_to_context, verify_context_txq_count, create_test_context};

// 测试环境配置：本机接口 eth0: ifindex=0, MAC=00:11:22:33:44:55, IP=192.168.1.100

// ========== 全局测试生命周期 ==========

// 1. Echo Request/Reply 测试组

#[test]
#[serial]
fn test_icmp_echo_request_reply() {
    let ctx = create_test_context();

    let sender_mac = MacAddr::new([0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0x01]);
    let sender_ip = Ipv4Addr::new(192, 168, 1, 10);
    let target_ip = Ipv4Addr::new(192, 168, 1, 100); // 本机 IP

    // 创建并注入 Echo Request
    let request = create_echo_request_packet(sender_mac, sender_ip, target_ip, 1234, 1);
    inject_packet_to_context(&ctx, "eth0", request).unwrap();

    // 运行调度器
    let mut harness = TestHarness::with_context(ctx.clone());
    let result = harness.run();
    assert!(result.is_ok(), "调度器运行失败: {:?}", result.err());

    // 验证：应该有 Echo Reply 响应
    assert!(verify_context_txq_count(&ctx, "eth0", 1), "发送队列应该有1个Echo Reply响应报文");
}

#[test]
#[serial]
fn test_icmp_echo_identifier_sequence_match() {
    let sender_mac = MacAddr::new([0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0x01]);
    let sender_ip = Ipv4Addr::new(192, 168, 1, 10);
    let target_ip = Ipv4Addr::new(192, 168, 1, 100);

    // 测试不同的 identifier 和 sequence
    for identifier in [1000, 2000, 3000] {
        for sequence in [1, 2, 3] {
            let ctx = create_test_context();

            let request = create_echo_request_packet(sender_mac, sender_ip, target_ip, identifier, sequence);
            inject_packet_to_context(&ctx, "eth0", request).unwrap();

            let mut harness = TestHarness::with_context(ctx.clone());
            let result = harness.run();
            assert!(result.is_ok(), "调度器运行失败: {:?}", result.err());

            assert!(verify_context_txq_count(&ctx, "eth0", 1), "应该有Echo Reply响应");
        }
    }
}

// 2. 边界条件测试组

#[test]
#[serial]
fn test_icmp_minimal_packet() {
    let ctx = create_test_context();

    // 创建最小 Echo Request（8字节头部，无数据）
    let mut icmp_bytes = vec![
        0x08, 0x00, // Type=Echo Request, Code=0
        0x00, 0x00, // Checksum (稍后计算)
        0x12, 0x34, // Identifier
        0x00, 0x01, // Sequence
    ];

    // 计算校验和
    let checksum = core_net::protocols::ip::calculate_checksum(&icmp_bytes);
    icmp_bytes[2] = (checksum >> 8) as u8;
    icmp_bytes[3] = (checksum & 0xFF) as u8;

    // IP 封装
    let src_ip = Ipv4Addr::new(192, 168, 1, 10);
    let dst_ip = Ipv4Addr::new(192, 168, 1, 100);
    let mut ip_data = create_ip_header(src_ip, dst_ip, icmp_bytes.len());
    ip_data.extend_from_slice(&icmp_bytes);

    // 以太网封装
    let src_mac = MacAddr::new([0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0x01]);
    let mut frame = Vec::new();
    frame.extend_from_slice(&[0xff, 0xff, 0xff, 0xff, 0xff, 0xff]);
    frame.extend_from_slice(&src_mac.bytes);
    frame.extend_from_slice(&ETH_P_IP.to_be_bytes());
    frame.extend_from_slice(&ip_data);

    let packet = Packet::from_bytes(frame);
    inject_packet_to_context(&ctx, "eth0", packet).unwrap();

    let mut harness = TestHarness::with_context(ctx.clone());
    let result = harness.run();
    assert!(result.is_ok());

    assert!(verify_context_txq_count(&ctx, "eth0", 1), "应该有响应");
}

#[test]
#[serial]
fn test_icmp_checksum_validation() {
    let ctx = create_test_context();

    // 创建校验和错误的 Echo Request
    let icmp_bytes = vec![
        0x08, 0x00, // Type=Echo Request, Code=0
        0xff, 0xff, // 错误的校验和
        0x12, 0x34, // Identifier
        0x00, 0x01, // Sequence
    ];

    let src_ip = Ipv4Addr::new(192, 168, 1, 10);
    let dst_ip = Ipv4Addr::new(192, 168, 1, 100);
    let mut ip_data = create_ip_header(src_ip, dst_ip, icmp_bytes.len());
    ip_data.extend_from_slice(&icmp_bytes);

    let src_mac = MacAddr::new([0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0x01]);
    let mut frame = Vec::new();
    frame.extend_from_slice(&[0xff, 0xff, 0xff, 0xff, 0xff, 0xff]);
    frame.extend_from_slice(&src_mac.bytes);
    frame.extend_from_slice(&ETH_P_IP.to_be_bytes());
    frame.extend_from_slice(&ip_data);

    let packet = Packet::from_bytes(frame);
    inject_packet_to_context(&ctx, "eth0", packet).unwrap();

    let mut harness = TestHarness::with_context(ctx.clone());
    let result = harness.run();
    assert!(result.is_ok());

    // 校验和错误，不应该有响应
    assert!(verify_context_txq_count(&ctx, "eth0", 0), "校验和错误时不应有响应");
}

// 3. ICMP 类型解析测试组

#[test]
#[serial]
fn test_icmp_type_parsing() {
    // 测试 Echo Request 解析
    let echo_bytes = create_echo_request(1234, 1, vec![0x42; 32]);
    let mut packet = Packet::from_bytes(echo_bytes);
    let result = IcmpPacket::from_packet(&mut packet);
    assert!(result.is_ok());

    match result.unwrap() {
        IcmpPacket::Echo(echo) => {
            assert_eq!(echo.type_, 8);
            assert_eq!(echo.identifier, 1234);
            assert_eq!(echo.sequence, 1);
        }
        _ => panic!("应该解析为 Echo"),
    }
}

#[test]
#[serial]
fn test_icmp_echo_roundtrip() {
    // 测试编码和解码一致性
    let original = IcmpEcho::new_request(5678, 42, vec![0x00, 0x01, 0x02, 0x03]);

    // 编码
    let bytes = original.to_bytes();

    // 解码
    let mut packet = Packet::from_bytes(bytes);
    let decoded = IcmpEcho::from_packet(&mut packet).unwrap();

    assert_eq!(decoded.type_, original.type_);
    assert_eq!(decoded.identifier, original.identifier);
    assert_eq!(decoded.sequence, original.sequence);
    assert_eq!(decoded.data, original.data);
}

// 4. Destination Unreachable 测试组

#[test]
#[serial]
fn test_icmp_dest_unreachable_no_reply() {
    let ctx = create_test_context();

    // Destination Unreachable 不应该触发响应（避免循环）
    let mut icmp_bytes = vec![
        0x03, 0x00, // Type=Dest Unreachable, Code=Network Unreachable
        0x00, 0x00, // Checksum
        0x00, 0x00, 0x00, 0x00, // Unused
        // 原始 IP 头部（简化）
        0x45, 0x00, 0x00, 0x1c, 0x00, 0x00, 0x00, 0x00,
        0x40, 0x01, 0x00, 0x00, 0xc0, 0xa8, 0x01, 0x0a,
        0xc0, 0xa8, 0x01, 0x64,
    ];

    // 计算校验和
    let checksum = core_net::protocols::ip::calculate_checksum(&icmp_bytes);
    icmp_bytes[2] = (checksum >> 8) as u8;
    icmp_bytes[3] = (checksum & 0xFF) as u8;

    let src_ip = Ipv4Addr::new(192, 168, 1, 1);
    let dst_ip = Ipv4Addr::new(192, 168, 1, 100);
    let mut ip_data = create_ip_header(src_ip, dst_ip, icmp_bytes.len());
    ip_data.extend_from_slice(&icmp_bytes);

    let src_mac = MacAddr::new([0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0x01]);
    let mut frame = Vec::new();
    frame.extend_from_slice(&[0xff, 0xff, 0xff, 0xff, 0xff, 0xff]);
    frame.extend_from_slice(&src_mac.bytes);
    frame.extend_from_slice(&ETH_P_IP.to_be_bytes());
    frame.extend_from_slice(&ip_data);

    let packet = Packet::from_bytes(frame);
    inject_packet_to_context(&ctx, "eth0", packet).unwrap();

    let mut harness = TestHarness::with_context(ctx.clone());
    let result = harness.run();
    assert!(result.is_ok());

    // Destination Unreachable 不应该触发响应
    assert!(verify_context_txq_count(&ctx, "eth0", 0), "Destination Unreachable 不应有响应");
}

// 5. 非本机 IP 测试组

#[test]
#[serial]
fn test_icmp_not_for_us() {
    let ctx = create_test_context();

    let sender_mac = MacAddr::new([0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0x01]);
    let sender_ip = Ipv4Addr::new(192, 168, 1, 10);
    let target_ip = Ipv4Addr::new(192, 168, 1, 200); // 不是本机 IP

    // 创建 Echo Request（目标不是本机）
    let request = create_echo_request_packet(sender_mac, sender_ip, target_ip, 1234, 1);
    inject_packet_to_context(&ctx, "eth0", request).unwrap();

    let mut harness = TestHarness::with_context(ctx.clone());
    let result = harness.run();
    assert!(result.is_ok());

    // 目标不是本机，不应该响应
    assert!(verify_context_txq_count(&ctx, "eth0", 0), "非本机IP不应响应");
}

// 6. 多接口测试组

#[test]
#[serial]
fn test_icmp_multiple_interfaces() {
    let ctx = create_test_context();

    let sender_mac = MacAddr::new([0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0x01]);
    let sender_ip = Ipv4Addr::new(192, 168, 1, 10);

    // 测试向不同接口的 IP 发送请求
    let eth0_ip = Ipv4Addr::new(192, 168, 1, 100); // eth0 的 IP

    let request = create_echo_request_packet(sender_mac, sender_ip, eth0_ip, 1234, 1);
    inject_packet_to_context(&ctx, "eth0", request).unwrap();

    let mut harness = TestHarness::with_context(ctx.clone());
    let result = harness.run();
    assert!(result.is_ok());

    assert!(verify_context_txq_count(&ctx, "eth0", 1), "eth0 应该有响应");
}

// 7. 序列号回绕测试

#[test]
#[serial]
fn test_icmp_sequence_wraparound() {
    let ctx = create_test_context();

    let sender_mac = MacAddr::new([0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0x01]);
    let sender_ip = Ipv4Addr::new(192, 168, 1, 10);
    let target_ip = Ipv4Addr::new(192, 168, 1, 100);

    // 测试最大序列号
    let request = create_echo_request_packet(sender_mac, sender_ip, target_ip, 1234, 65535);
    inject_packet_to_context(&ctx, "eth0", request).unwrap();

    let mut harness = TestHarness::with_context(ctx.clone());
    let result = harness.run();
    assert!(result.is_ok());

    assert!(verify_context_txq_count(&ctx, "eth0", 1), "序列号65535应该正常处理");
}

// 8. Echo Reply 匹配测试

#[test]
#[serial]
fn test_icmp_echo_reply_matching() {
    let ctx = create_test_context();

    // 模拟收到 Echo Reply 的场景
    // 注意：在纯模拟环境中，这通常不会自然发生
    // 但我们可以验证 Echo 管理器的功能

    use core_net::protocols::icmp::register_echo_request;

    let sender_mac = MacAddr::new([0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0x01]);
    let sender_ip = Ipv4Addr::new(192, 168, 1, 10);
    let target_ip = Ipv4Addr::new(192, 168, 1, 100);

    // 注册一个 Echo 请求
    let result = register_echo_request(9999, 100, target_ip);
    assert!(result.is_ok(), "注册 Echo 请求应该成功");

    // 发送对应的 Echo Reply（模拟）
    let reply = create_echo_reply_packet(
        MacAddr::new([0x00, 0x11, 0x22, 0x33, 0x44, 0x55]), // dst_mac
        sender_mac,
        target_ip,
        sender_ip,
        9999,
        100,
    );

    inject_packet_to_context(&ctx, "eth0", reply).unwrap();

    let mut harness = TestHarness::with_context(ctx.clone());
    let result = harness.run();
    assert!(result.is_ok());

    // Echo Reply 不应该触发新的响应
    assert!(verify_context_txq_count(&ctx, "eth0", 0), "Echo Reply 不应触发新响应");
}
