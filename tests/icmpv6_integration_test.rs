// ICMPv6 协议集成测试
//
// 测试 ICMPv6 协议的 Echo Request/Reply、邻居发现功能

use core_net::testframework::TestHarness;
use core_net::common::Packet;
use core_net::interface::MacAddr;
use core_net::protocols::{Ipv6Addr, ETH_P_IPV6};
use core_net::protocols::icmpv6::{Icmpv6Packet, Icmpv6Echo, create_icmpv6_echo_request, Icmpv6Context, NeighborCacheState, Icmpv6ProcessResult};
use serial_test::serial;

mod common;
use common::{create_ipv6_echo_request_packet, inject_packet_to_context, verify_context_txq_count, create_test_context};

// 测试环境配置：本机接口 eth0: ifindex=0, MAC=00:11:22:33:44:55, IPv6=fe80::1

// ========== 直接处理测试 ==========

#[test]
#[serial]
fn test_scheduler_simple_packet() {
    use core_net::protocols::arp::{ArpPacket, ArpOperation};
    use core_net::protocols::ethernet;
    use core_net::interface::MacAddr;
    use core_net::protocols::Ipv4Addr;

    let ctx = create_test_context();

    // 创建一个简单的 ARP 请求报文
    let src_mac = MacAddr::new([0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF]);
    let src_ip = Ipv4Addr::new(192, 168, 1, 1);
    let dst_ip = Ipv4Addr::new(192, 168, 1, 100); // 本机 IP

    let arp_pkt = ArpPacket::new(
        ArpOperation::Request,
        src_mac,
        src_ip,
        MacAddr::zero(),
        dst_ip,
    );

    let frame_bytes = ethernet::build_ethernet_frame(
        MacAddr::broadcast(),
        src_mac,
        0x0806, // ETH_P_ARP
        &arp_pkt.to_bytes(),
    );

    let packet = Packet::from_bytes(frame_bytes);
    inject_packet_to_context(&ctx, "eth0", packet).unwrap();

    // 运行调度器
    let mut harness = TestHarness::with_context(ctx.clone());
    let result = harness.run();
    assert!(result.is_ok(), "调度器运行失败: {:?}", result.err());

    println!("Scheduler processed {} packets", result.unwrap());
}

// ========== 直接处理测试 ==========

#[test]
#[serial]
fn test_icmpv6_echo_direct_processing() {
    let mut context = Icmpv6Context::default();
    let source = Ipv6Addr::new(0xfe80, 0, 0, 0, 0, 0, 0, 0x1000);
    let dest = Ipv6Addr::new(0xfe80, 0, 0, 0, 0, 0, 0, 1);

    // 创建 Echo Request（使用正确的 ICMPv6 校验和计算）
    let echo_bytes = create_icmpv6_echo_request(source, dest, 1234, 1, vec![0x42; 32]);
    let mut packet = Packet::from_bytes(echo_bytes);

    // 直接处理
    let result = core_net::protocols::icmpv6::process_icmpv6_packet(
        packet, source, dest, &mut context, false
    );

    assert!(result.is_ok());
    match result.unwrap() {
        Icmpv6ProcessResult::Reply(reply_bytes) => {
            // 验证响应类型
            assert_eq!(reply_bytes[0], 129); // Echo Reply
        }
        _ => panic!("Expected Reply"),
    }
}

// ========== 接口配置验证测试 ==========

#[test]
#[serial]
fn test_icmpv6_interface_has_ipv6_addr() {
    let ctx = create_test_context();
    let interfaces = ctx.interfaces.lock().unwrap();
    let iface = interfaces.get_by_name("eth0").unwrap();
    assert_eq!(iface.ipv6_addr(), Ipv6Addr::new(0xfe80, 0, 0, 0, 0, 0, 0, 1));
}

// ========== Echo Request/Reply 测试组 ==========

#[test]
#[serial]
fn test_icmpv6_echo_request_reply() {
    let ctx = create_test_context();

    let sender_mac = MacAddr::new([0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0x01]);
    let sender_ipv6 = Ipv6Addr::new(0xfe80, 0, 0, 0, 0, 0, 0, 0x1000);
    let target_ipv6 = Ipv6Addr::new(0xfe80, 0, 0, 0, 0, 0, 0, 1); // 本机 IPv6

    // 创建并注入 Echo Request
    let request = create_ipv6_echo_request_packet(sender_mac, sender_ipv6, target_ipv6, 1234, 1);
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
fn test_icmpv6_echo_identifier_sequence_match() {
    let sender_mac = MacAddr::new([0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0x01]);
    let sender_ipv6 = Ipv6Addr::new(0xfe80, 0, 0, 0, 0, 0, 0, 0x1000);
    let target_ipv6 = Ipv6Addr::new(0xfe80, 0, 0, 0, 0, 0, 0, 1);

    // 测试不同的 identifier 和 sequence
    for identifier in [1000, 2000, 3000] {
        for sequence in [1, 2, 3] {
            let ctx = create_test_context();

            let request = create_ipv6_echo_request_packet(sender_mac, sender_ipv6, target_ipv6, identifier, sequence);
            inject_packet_to_context(&ctx, "eth0", request).unwrap();

            let mut harness = TestHarness::with_context(ctx.clone());
            let result = harness.run();
            assert!(result.is_ok(), "调度器运行失败: {:?}", result.err());

            assert!(verify_context_txq_count(&ctx, "eth0", 1), "应该有Echo Reply响应");
        }
    }
}

// ========== 边界条件测试组 ==========

#[test]
#[serial]
fn test_icmpv6_minimal_packet() {
    let ctx = create_test_context();

    // 创建最小 Echo Request（8字节头部，无数据）
    let icmp_bytes = vec![
        0x80, 0x00, // Type=Echo Request, Code=0
        0x00, 0x00, // Checksum (占位)
        0x12, 0x34, // Identifier
        0x00, 0x01, // Sequence
    ];

    // IPv6 封装
    let src_ipv6 = Ipv6Addr::new(0xfe80, 0, 0, 0, 0, 0, 0, 0x1000);
    let dst_ipv6 = Ipv6Addr::new(0xfe80, 0, 0, 0, 0, 0, 0, 1);

    // 使用 ICMPv6 伪头部计算校验和
    let checksum = core_net::protocols::icmpv6::calculate_icmpv6_checksum(
        src_ipv6, dst_ipv6, &icmp_bytes
    );

    let mut icmp_with_checksum = icmp_bytes.clone();
    icmp_with_checksum[2] = (checksum >> 8) as u8;
    icmp_with_checksum[3] = (checksum & 0xFF) as u8;

    let ipv6_packet = core_net::protocols::ipv6::encapsulate_ipv6_packet(
        src_ipv6,
        dst_ipv6,
        core_net::protocols::ipv6::IpProtocol::IcmpV6,
        &icmp_with_checksum,
        64,
    );

    // 以太网封装
    let src_mac = MacAddr::new([0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0x01]);
    let mut frame = Vec::new();
    frame.extend_from_slice(&[0xff, 0xff, 0xff, 0xff, 0xff, 0xff]);
    frame.extend_from_slice(&src_mac.bytes);
    frame.extend_from_slice(&ETH_P_IPV6.to_be_bytes());
    frame.extend_from_slice(&ipv6_packet);

    let packet = Packet::from_bytes(frame);
    inject_packet_to_context(&ctx, "eth0", packet).unwrap();

    let mut harness = TestHarness::with_context(ctx.clone());
    let result = harness.run();
    assert!(result.is_ok());

    assert!(verify_context_txq_count(&ctx, "eth0", 1), "应该有响应");
}

#[test]
#[serial]
fn test_icmpv6_checksum_validation() {
    let ctx = create_test_context();

    // 创建校验和错误的 Echo Request
    let icmp_bytes = vec![
        0x80, 0x00, // Type=Echo Request, Code=0
        0xff, 0xff, // 错误的校验和
        0x12, 0x34, // Identifier
        0x00, 0x01, // Sequence
    ];

    let src_ipv6 = Ipv6Addr::new(0xfe80, 0, 0, 0, 0, 0, 0, 0x1000);
    let dst_ipv6 = Ipv6Addr::new(0xfe80, 0, 0, 0, 0, 0, 0, 1);
    let ipv6_packet = core_net::protocols::ipv6::encapsulate_ipv6_packet(
        src_ipv6,
        dst_ipv6,
        core_net::protocols::ipv6::IpProtocol::IcmpV6,
        &icmp_bytes,
        64,
    );

    let src_mac = MacAddr::new([0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0x01]);
    let mut frame = Vec::new();
    frame.extend_from_slice(&[0xff, 0xff, 0xff, 0xff, 0xff, 0xff]);
    frame.extend_from_slice(&src_mac.bytes);
    frame.extend_from_slice(&ETH_P_IPV6.to_be_bytes());
    frame.extend_from_slice(&ipv6_packet);

    let packet = Packet::from_bytes(frame);
    inject_packet_to_context(&ctx, "eth0", packet).unwrap();

    let mut harness = TestHarness::with_context(ctx.clone());
    let result = harness.run();
    assert!(result.is_ok());

    // 校验和错误，不应该有响应
    assert!(verify_context_txq_count(&ctx, "eth0", 0), "校验和错误时不应有响应");
}

// ========== ICMPv6 类型解析测试组 ==========

#[test]
#[serial]
fn test_icmpv6_type_parsing() {
    // 测试 Echo Request 解析（使用正确的 ICMPv6 校验和计算）
    let src = Ipv6Addr::new(0xfe80, 0, 0, 0, 0, 0, 0, 0x1000);
    let dst = Ipv6Addr::new(0xfe80, 0, 0, 0, 0, 0, 0, 1);
    let echo_bytes = create_icmpv6_echo_request(src, dst, 1234, 1, vec![0x42; 32]);
    let mut packet = Packet::from_bytes(echo_bytes);
    let result = Icmpv6Packet::from_packet(&mut packet);
    assert!(result.is_ok());

    match result.unwrap() {
        Icmpv6Packet::Echo(echo) => {
            assert_eq!(echo.type_, 128);
            assert_eq!(echo.identifier, 1234);
            assert_eq!(echo.sequence, 1);
        }
        _ => panic!("应该解析为 Echo"),
    }
}

#[test]
#[serial]
fn test_icmpv6_echo_roundtrip() {
    // 测试编码和解码一致性
    let original = Icmpv6Echo::new_request(5678, 42, vec![0x00, 0x01, 0x02, 0x03]);

    // 编码
    let bytes = original.to_bytes();

    // 解码
    let mut packet = Packet::from_bytes(bytes);
    let decoded = Icmpv6Echo::from_packet(&mut packet).unwrap();

    assert_eq!(decoded.type_, original.type_);
    assert_eq!(decoded.identifier, original.identifier);
    assert_eq!(decoded.sequence, original.sequence);
    assert_eq!(decoded.data, original.data);
}

// ========== 非本机 IPv6 测试组 ==========

#[test]
#[serial]
fn test_icmpv6_not_for_us() {
    let ctx = create_test_context();

    let sender_mac = MacAddr::new([0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0x01]);
    let sender_ipv6 = Ipv6Addr::new(0xfe80, 0, 0, 0, 0, 0, 0, 0x1000);
    let target_ipv6 = Ipv6Addr::new(0xfe80, 0, 0, 0, 0, 0, 0, 0x2000); // 不是本机 IPv6

    // 创建 Echo Request（目标不是本机）
    let request = create_ipv6_echo_request_packet(sender_mac, sender_ipv6, target_ipv6, 1234, 1);
    inject_packet_to_context(&ctx, "eth0", request).unwrap();

    let mut harness = TestHarness::with_context(ctx.clone());
    let result = harness.run();
    assert!(result.is_ok());

    // 目标不是本机，不应该响应
    assert!(verify_context_txq_count(&ctx, "eth0", 0), "非本机IPv6不应响应");
}

// ========== 序列号回绕测试 ==========

#[test]
#[serial]
fn test_icmpv6_sequence_wraparound() {
    let ctx = create_test_context();

    let sender_mac = MacAddr::new([0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0x01]);
    let sender_ipv6 = Ipv6Addr::new(0xfe80, 0, 0, 0, 0, 0, 0, 0x1000);
    let target_ipv6 = Ipv6Addr::new(0xfe80, 0, 0, 0, 0, 0, 0, 1);

    // 测试最大序列号
    let request = create_ipv6_echo_request_packet(sender_mac, sender_ipv6, target_ipv6, 1234, 65535);
    inject_packet_to_context(&ctx, "eth0", request).unwrap();

    let mut harness = TestHarness::with_context(ctx.clone());
    let result = harness.run();
    assert!(result.is_ok());

    assert!(verify_context_txq_count(&ctx, "eth0", 1), "序列号65535应该正常处理");
}

// ========== Icmpv6Context 测试组 ==========

#[test]
#[serial]
fn test_icmpv6_context_default() {
    let context = Icmpv6Context::default();

    assert!(context.config.enable_echo_reply);
    assert_eq!(context.neighbor_cache.len(), 0);
    assert_eq!(context.echo_manager.pending_count(), 0);
}

#[test]
#[serial]
fn test_icmpv6_neighbor_cache() {
    let mut context = Icmpv6Context::default();
    let addr = Ipv6Addr::new(0xfe80, 0, 0, 0, 0, 0, 0, 0x1000);
    let mac = MacAddr::new([0x00, 0x11, 0x22, 0x33, 0x44, 0x55]);

    context.neighbor_cache.update(addr, mac, false, NeighborCacheState::Reachable).unwrap();
    assert_eq!(context.neighbor_cache.len(), 1);

    let entry = context.neighbor_cache.lookup(&addr).unwrap();
    assert_eq!(entry.ipv6_addr, addr);
    assert_eq!(entry.link_layer_addr, Some(mac));
    assert_eq!(entry.state, NeighborCacheState::Reachable);
}

#[test]
#[serial]
fn test_icmpv6_echo_manager() {
    let mut context = Icmpv6Context::default();
    let addr = Ipv6Addr::new(0xfe80, 0, 0, 0, 0, 0, 0, 0x1000);

    // 注册 Echo 请求
    context.echo_manager.register(1234, 1, addr).unwrap();
    assert_eq!(context.echo_manager.pending_count(), 1);

    // 匹配响应
    let pending = context.echo_manager.match_reply(1234, 1).unwrap();
    assert_eq!(pending.identifier, 1234);
    assert_eq!(pending.sequence, 1);
    assert_eq!(pending.dest_addr, addr);

    assert_eq!(context.echo_manager.pending_count(), 0);
}

// ========== Router Advertisement 测试组 ==========

#[test]
#[serial]
fn test_icmpv6_router_advertisement_parsing() {
    // 创建 Router Advertisement 报文
    let ra_bytes = vec![
        0x86, 0x00, // Type=Router Advertisement, Code=0
        0x00, 0x00, // Checksum (稍后计算)
        0x40,       // Cur Hop Limit = 64
        0x00,       // Flags (M=0, O=0)
        0x07, 0x08, // Lifetime = 1800 秒
        0x00, 0x00, 0x75, 0x30, // Reachable Time = 30000 ms
        0x00, 0x00, 0x03, 0xe8, // Retrans Timer = 1000 ms
        // Source Link-Layer Address 选项
        0x01, 0x01, // Type=Source Link-Layer Address, Length=1 (8 bytes)
        0x00, 0x11, 0x22, 0x33, 0x44, 0x55, // MAC address
        0x00,
    ];

    // 使用 ICMPv6 伪头部计算校验和
    let src_ipv6 = Ipv6Addr::new(0xfe80, 0, 0, 0, 0, 0, 0, 0x1000);
    let dst_ipv6 = Ipv6Addr::new(0xfe80, 0, 0, 0, 0, 0, 0, 1);
    let checksum = core_net::protocols::icmpv6::calculate_icmpv6_checksum(
        src_ipv6, dst_ipv6, &ra_bytes
    );

    let mut ra_with_checksum = ra_bytes.clone();
    ra_with_checksum[2] = (checksum >> 8) as u8;
    ra_with_checksum[3] = (checksum & 0xFF) as u8;

    // 封装 IPv6
    let ipv6_packet = core_net::protocols::ipv6::encapsulate_ipv6_packet(
        src_ipv6,
        dst_ipv6,
        core_net::protocols::ipv6::IpProtocol::IcmpV6,
        &ra_with_checksum,
        255,
    );

    let src_mac = MacAddr::new([0x00, 0x11, 0x22, 0x33, 0x44, 0x55]);
    let mut frame = Vec::new();
    frame.extend_from_slice(&[0x33, 0x33, 0x00, 0x00, 0x00, 0x01]); // IPv6 组播 MAC
    frame.extend_from_slice(&src_mac.bytes);
    frame.extend_from_slice(&ETH_P_IPV6.to_be_bytes());
    frame.extend_from_slice(&ipv6_packet);

    let ctx = create_test_context();
    let packet = Packet::from_bytes(frame);
    inject_packet_to_context(&ctx, "eth0", packet).unwrap();

    let mut harness = TestHarness::with_context(ctx.clone());
    let result = harness.run();
    assert!(result.is_ok());

    // Router Advertisement 不应该触发响应
    assert!(verify_context_txq_count(&ctx, "eth0", 0), "Router Advertisement 不应触发新响应");

    // 验证路由器列表已更新
    let icmpv6_ctx = ctx.icmpv6_context.lock().unwrap();
    assert!(icmpv6_ctx.router_list.routers().len() > 0, "路由器列表应该有更新");
}
