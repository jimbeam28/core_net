// IPv6 协议集成测试
//
// 测试 IPv6 协议的头部解析、地址类型判断、基本功能等

use core_net::testframework::{
    TestHarness,
};
use core_net::interface::MacAddr;
use core_net::protocols::{Ipv6Addr, ETH_P_IPV6};
use core_net::protocols::ipv6::{
    Ipv6Header, IpProtocol, IPV6_VERSION, IPV6_HEADER_LEN, IPV6_MIN_MTU,
    DEFAULT_HOP_LIMIT, encapsulate_ipv6_packet,
};
use core_net::common::Packet;

use serial_test::serial;

mod common;
use common::{create_test_context, inject_packet_to_context, verify_context_txq_count};

// 测试环境配置：本机接口 eth0: ifindex=0, MAC=00:11:22:33:44:55

// ========== 全局测试生命周期 ==========

// 1. IPv6 头部解析测试组

#[test]
#[serial]
fn test_ipv6_header_constants() {
    let _ctx = create_test_context();

    assert_eq!(IPV6_VERSION, 6);
    assert_eq!(IPV6_HEADER_LEN, 40);
    assert_eq!(IPV6_MIN_MTU, 1280);
    assert_eq!(DEFAULT_HOP_LIMIT, 64);
}

#[test]
#[serial]
fn test_ipv6_header_parse() {
    let _ctx = create_test_context();

    let src_ip = Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 1);
    let dst_ip = Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 2);
    let ip_header = Ipv6Header::new(src_ip, dst_ip, 64, IpProtocol::IcmpV6, 64);

    assert_eq!(ip_header.version, 6);
    assert_eq!(ip_header.header_len(), 40);
    assert_eq!(ip_header.next_header, IpProtocol::IcmpV6);
    assert_eq!(ip_header.source_addr, src_ip);
    assert_eq!(ip_header.destination_addr, dst_ip);
}

#[test]
#[serial]
fn test_ipv6_header_encode_decode() {
    let _ctx = create_test_context();

    let src_ip = Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 1);
    let dst_ip = Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 2);
    let header = Ipv6Header::new(src_ip, dst_ip, 64, IpProtocol::IcmpV6, 64);

    // 编码
    let bytes = header.to_bytes();

    // 验证基本字段
    assert_eq!(bytes[0] >> 4, 6); // Version
    assert_eq!(bytes[6], 58); // Next Header = ICMPv6
    assert_eq!(bytes[7], 64); // Hop Limit

    // 解码
    let mut packet = Packet::from_bytes(bytes.to_vec());
    let decoded = Ipv6Header::from_packet(&mut packet).unwrap();

    assert_eq!(decoded.source_addr, src_ip);
    assert_eq!(decoded.destination_addr, dst_ip);
    assert_eq!(decoded.next_header, IpProtocol::IcmpV6);
}

// 2. IPv6 协议枚举测试组

#[test]
#[serial]
fn test_ipv6_protocol_from_u8() {
    let _ctx = create_test_context();

    assert_eq!(IpProtocol::from(58), IpProtocol::IcmpV6);
    assert_eq!(IpProtocol::from(6), IpProtocol::Tcp);
    assert_eq!(IpProtocol::from(17), IpProtocol::Udp);
    assert_eq!(IpProtocol::from(255), IpProtocol::Unknown(255));
}

#[test]
#[serial]
fn test_ipv6_protocol_is_extension_header() {
    let _ctx = create_test_context();

    assert!(IpProtocol::HopByHopOptions.is_extension_header());
    assert!(IpProtocol::Ipv6Route.is_extension_header());
    assert!(IpProtocol::Ipv6Fragment.is_extension_header());
    assert!(IpProtocol::Esp.is_extension_header());
    assert!(IpProtocol::Ah.is_extension_header());
    assert!(IpProtocol::Ipv6DestOptions.is_extension_header());

    assert!(!IpProtocol::IcmpV6.is_extension_header());
    assert!(!IpProtocol::Tcp.is_extension_header());
    assert!(!IpProtocol::Udp.is_extension_header());
}

#[test]
#[serial]
fn test_ipv6_protocol_is_upper_layer() {
    let _ctx = create_test_context();

    assert!(IpProtocol::Tcp.is_upper_layer());
    assert!(IpProtocol::Udp.is_upper_layer());
    assert!(IpProtocol::IcmpV6.is_upper_layer());
    assert!(IpProtocol::Icmp.is_upper_layer());

    assert!(!IpProtocol::HopByHopOptions.is_upper_layer());
    assert!(!IpProtocol::Ipv6Route.is_upper_layer());
}

// 3. IPv6 地址类型测试组

#[test]
#[serial]
fn test_ipv6_addr_constants() {
    let _ctx = create_test_context();

    // 未指定地址
    assert!(Ipv6Addr::UNSPECIFIED.is_unspecified());

    // 环回地址
    assert!(Ipv6Addr::LOOPBACK.is_loopback());

    // 组播地址
    assert!(Ipv6Addr::ALL_NODES_MULTICAST.is_multicast());
    assert!(Ipv6Addr::LINK_LOCAL_ALL_NODES.is_multicast());
    assert!(Ipv6Addr::LINK_LOCAL_ALL_ROUTERS.is_multicast());
}

#[test]
#[serial]
fn test_ipv6_addr_is_unspecified() {
    let _ctx = create_test_context();

    assert!(Ipv6Addr::UNSPECIFIED.is_unspecified());
    assert!(Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 0).is_unspecified());

    let addr = Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 1);
    assert!(!addr.is_unspecified());
}

#[test]
#[serial]
fn test_ipv6_addr_is_loopback() {
    let _ctx = create_test_context();

    assert!(Ipv6Addr::LOOPBACK.is_loopback());
    assert!(Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 1).is_loopback());

    let addr = Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 2);
    assert!(!addr.is_loopback());
}

#[test]
#[serial]
fn test_ipv6_addr_is_multicast() {
    let _ctx = create_test_context();

    // 组播地址 ff00::/8
    assert!(Ipv6Addr::new(0xff00, 0, 0, 0, 0, 0, 0, 1).is_multicast());
    assert!(Ipv6Addr::new(0xff02, 0, 0, 0, 0, 0, 0, 1).is_multicast());

    // 非组播地址
    assert!(!Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 1).is_multicast());
}

#[test]
#[serial]
fn test_ipv6_addr_is_link_local() {
    let _ctx = create_test_context();

    // 链路本地地址 fe80::/10
    assert!(Ipv6Addr::new(0xfe80, 0, 0, 0, 0, 0, 0, 1).is_link_local());

    // 非链路本地地址
    assert!(!Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 1).is_link_local());
    assert!(!Ipv6Addr::LOOPBACK.is_link_local());
}

#[test]
#[serial]
fn test_ipv6_addr_is_global_unicast() {
    let _ctx = create_test_context();

    // 全球单播地址 2000::/3
    assert!(Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 1).is_global_unicast());

    // 非全球单播地址
    assert!(!Ipv6Addr::LOOPBACK.is_global_unicast());
    assert!(!Ipv6Addr::new(0xfe80, 0, 0, 0, 0, 0, 0, 1).is_global_unicast());
}

#[test]
#[serial]
fn test_ipv6_addr_display() {
    let _ctx = create_test_context();

    // 环回地址
    assert_eq!(format!("{}", Ipv6Addr::LOOPBACK), "::1");

    // 未指定地址
    assert_eq!(format!("{}", Ipv6Addr::UNSPECIFIED), "::");

    // 完整地址
    let addr = Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 1);
    assert_eq!(format!("{}", addr), "2001:db8::1");

    // 组播地址
    let addr = Ipv6Addr::LINK_LOCAL_ALL_NODES;
    assert_eq!(format!("{}", addr), "ff02::1");
}

#[test]
#[serial]
fn test_ipv6_addr_from_str() {
    let _ctx = create_test_context();

    // 环回地址
    let addr: Ipv6Addr = "::1".parse().unwrap();
    assert_eq!(addr, Ipv6Addr::LOOPBACK);

    // 未指定地址
    let addr: Ipv6Addr = "::".parse().unwrap();
    assert_eq!(addr, Ipv6Addr::UNSPECIFIED);

    // 完整地址
    let addr: Ipv6Addr = "2001:db8::1".parse().unwrap();
    assert_eq!(addr, Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 1));
}

// 4. 边界条件测试组

#[test]
#[serial]
fn test_ipv6_min_packet_length() {
    let _ctx = create_test_context();

    // 创建最小 IPv6 数据包（40 字节头部，无负载）
    let src_ip = Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 1);
    let dst_ip = Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 2);
    let packet = encapsulate_ipv6_packet(src_ip, dst_ip, IpProtocol::IcmpV6, &[], 64);

    assert_eq!(packet.len(), IPV6_HEADER_LEN);
}

#[test]
#[serial]
fn test_ipv6_max_packet_length() {
    let _ctx = create_test_context();

    // 最大负载长度（65535 字节）
    let max_payload = vec![0x42; 65535];
    let src_ip = Ipv6Addr::UNSPECIFIED;
    let dst_ip = Ipv6Addr::UNSPECIFIED;
    let packet = encapsulate_ipv6_packet(src_ip, dst_ip, IpProtocol::IcmpV6, &max_payload, 64);

    // 验证总长度
    assert_eq!(packet.len(), IPV6_HEADER_LEN + 65535);

    // 验证 Payload Length 字段
    let payload_len = u16::from_be_bytes([packet[4], packet[5]]);
    assert_eq!(payload_len, 65535);
}

#[test]
#[serial]
fn test_ipv6_min_mtu() {
    let _ctx = create_test_context();

    // IPv6 要求最小 MTU 为 1280 字节
    assert_eq!(IPV6_MIN_MTU, 1280);

    // 创建 1280 字节的数据包（符合最小 MTU）
    let payload_1280 = vec![0x42; 1280 - IPV6_HEADER_LEN];
    let src_ip = Ipv6Addr::UNSPECIFIED;
    let dst_ip = Ipv6Addr::UNSPECIFIED;
    let packet = encapsulate_ipv6_packet(src_ip, dst_ip, IpProtocol::IcmpV6, &payload_1280, 64);

    assert_eq!(packet.len(), 1280);
}

// 5. 异常情况测试组

#[test]
#[serial]
fn test_ipv6_invalid_version() {
    let ctx = create_test_context();

    let sender_mac = MacAddr::new([0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0x01]);

    // 创建版本号错误的 IPv6 数据包
    let mut frame = Vec::new();
    frame.extend_from_slice(&[0xff, 0xff, 0xff, 0xff, 0xff, 0xff]);
    frame.extend_from_slice(&sender_mac.bytes);
    frame.extend_from_slice(&ETH_P_IPV6.to_be_bytes());

    // IPv6 头部: Version=4 (错误)
    frame.push(0x40);  // Version=4 (应该为 6)
    frame.extend_from_slice(&[0x00; 39]);  // 其余头部

    let packet = Packet::from_bytes(frame);
    inject_packet_to_context(&ctx, "eth0", packet).unwrap();

    let mut harness = TestHarness::with_context(ctx.clone());
    let result = harness.run();
    assert!(result.is_ok());

    // 版本错误，数据包应被丢弃
    assert!(verify_context_txq_count(&ctx, "eth0", 0), "版本错误的数据包应被丢弃");
}

#[test]
#[serial]
fn test_ipv6_extension_header_not_supported() {
    let ctx = create_test_context();

    let sender_mac = MacAddr::new([0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0x01]);

    // 创建带扩展头的 IPv6 数据包
    let mut frame = Vec::new();
    frame.extend_from_slice(&[0xff, 0xff, 0xff, 0xff, 0xff, 0xff]);
    frame.extend_from_slice(&sender_mac.bytes);
    frame.extend_from_slice(&ETH_P_IPV6.to_be_bytes());

    // IPv6 头部: Next Header = 0 (逐跳选项，当前不支持)
    frame.extend_from_slice(&[
        0x60,        // Version=6, TC=0
        0x00, 0x00, 0x00,  // Flow Label
        0x00, 0x00,  // Payload Length = 0
        0x00,        // Next Header = HopByHopOptions (不支持)
        0x40,        // Hop Limit
    ]);
    // 源地址和目的地址
    frame.extend_from_slice(&[0x00; 32]);

    let packet = Packet::from_bytes(frame);
    inject_packet_to_context(&ctx, "eth0", packet).unwrap();

    let mut harness = TestHarness::with_context(ctx.clone());
    let result = harness.run();
    assert!(result.is_ok());

    // 扩展头不支持，数据包应被丢弃
    assert!(verify_context_txq_count(&ctx, "eth0", 0), "扩展头不支持的数据包应被丢弃");
}

#[test]
#[serial]
fn test_ipv6_hop_limit_zero() {
    let ctx = create_test_context();

    let sender_mac = MacAddr::new([0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0x01]);

    // 创建 Hop Limit = 0 的 IPv6 数据包
    let mut frame = Vec::new();
    frame.extend_from_slice(&[0xff, 0xff, 0xff, 0xff, 0xff, 0xff]);
    frame.extend_from_slice(&sender_mac.bytes);
    frame.extend_from_slice(&ETH_P_IPV6.to_be_bytes());

    // IPv6 头部: Hop Limit = 0
    frame.extend_from_slice(&[
        0x60,        // Version=6, TC=0
        0x00, 0x00, 0x00,  // Flow Label
        0x00, 0x00,  // Payload Length = 0
        58,          // Next Header = ICMPv6
        0x00,        // Hop Limit = 0 (错误)
    ]);
    // 源地址和目的地址
    frame.extend_from_slice(&[0x00; 32]);

    let packet = Packet::from_bytes(frame);
    inject_packet_to_context(&ctx, "eth0", packet).unwrap();

    let mut harness = TestHarness::with_context(ctx.clone());
    let result = harness.run();
    assert!(result.is_ok());

    // Hop Limit 为 0，数据包应被丢弃
    assert!(verify_context_txq_count(&ctx, "eth0", 0), "Hop Limit为0的数据包应被丢弃");
}

#[test]
#[serial]
fn test_ipv6_source_address_multicast() {
    let ctx = create_test_context();

    let sender_mac = MacAddr::new([0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0x01]);

    // 创建源地址为组播地址的 IPv6 数据包（违反规范）
    let mut frame = Vec::new();
    frame.extend_from_slice(&[0xff, 0xff, 0xff, 0xff, 0xff, 0xff]);
    frame.extend_from_slice(&sender_mac.bytes);
    frame.extend_from_slice(&ETH_P_IPV6.to_be_bytes());

    // IPv6 头部
    frame.extend_from_slice(&[
        0x60,        // Version=6, TC=0
        0x00, 0x00, 0x00,  // Flow Label
        0x00, 0x00,  // Payload Length = 0
        58,          // Next Header = ICMPv6
        0x40,        // Hop Limit
    ]);
    // 源地址为组播地址 ff02::1 (违规)
    frame.extend_from_slice(&[0xff, 0x02, 0x00, 0x00]);
    frame.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]);
    frame.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]);
    frame.extend_from_slice(&[0x00, 0x00, 0x00, 0x01]);
    // 目的地址
    frame.extend_from_slice(&[0x00; 16]);

    let packet = Packet::from_bytes(frame);
    inject_packet_to_context(&ctx, "eth0", packet).unwrap();

    let mut harness = TestHarness::with_context(ctx.clone());
    let result = harness.run();
    assert!(result.is_ok());

    // 源地址为组播地址（违规），数据包应被丢弃
    assert!(verify_context_txq_count(&ctx, "eth0", 0), "源地址为组播地址的数据包应被丢弃");
}

// 6. 封装测试组

#[test]
#[serial]
fn test_ipv6_encapsulate_packet() {
    let _ctx = create_test_context();

    let src_ip = Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 1);
    let dst_ip = Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 2);
    let payload = vec![0x80, 0x00, 0x00, 0x00]; // ICMPv6 示例

    let packet = encapsulate_ipv6_packet(src_ip, dst_ip, IpProtocol::IcmpV6, &payload, 64);

    // 验证包头
    assert_eq!(packet[0] >> 4, 6); // Version=6
    assert_eq!(packet[6], 58); // Next Header=ICMPv6

    // 验证地址
    assert_eq!(&packet[8..24], &src_ip.bytes[..]);
    assert_eq!(&packet[24..40], &dst_ip.bytes[..]);

    // 验证负载
    assert_eq!(&packet[40..], &payload[..]);
}

#[test]
#[serial]
fn test_ipv6_encapsulate_with_flow_label() {
    let _ctx = create_test_context();

    let src_ip = Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 1);
    let dst_ip = Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 2);
    let flow_label = 0x12345;

    let header = Ipv6Header::with_flow_label(
        src_ip, dst_ip, 0, IpProtocol::IcmpV6, 64, flow_label
    );

    assert_eq!(header.flow_label, 0x12345);

    let bytes = header.to_bytes();
    let decoded_flow = (((bytes[1] & 0x0F) as u32) << 16)
        | ((bytes[2] as u32) << 8)
        | (bytes[3] as u32);
    assert_eq!(decoded_flow, 0x12345);
}
