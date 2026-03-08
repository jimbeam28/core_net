// ARP 协议集成测试（精简版）
//
// 核心功能测试：ARP请求/响应处理、缓存学习

use core_net::testframework::TestHarness;
use core_net::interface::{MacAddr, Ipv4Addr};
use core_net::protocols::arp::{ArpState, ArpPacket, ArpOperation};
use core_net::protocols::arp::encapsulate_ethernet;
use core_net::common::Packet;
use core_net::context::SystemContext;

use serial_test::serial;

mod common;
use common::create_test_context;

// 创建ARP请求报文
fn create_arp_request_packet(src_mac: MacAddr, src_ip: Ipv4Addr, dst_ip: Ipv4Addr) -> Packet {
    let arp_packet = ArpPacket::new(
        ArpOperation::Request,
        src_mac, src_ip, MacAddr::broadcast(), dst_ip,
    );
    let frame = encapsulate_ethernet(&arp_packet, MacAddr::broadcast(), src_mac);
    Packet::from_bytes(frame)
}

// 创建ARP响应报文
fn create_arp_reply_packet(src_mac: MacAddr, src_ip: Ipv4Addr, dst_mac: MacAddr, dst_ip: Ipv4Addr) -> Packet {
    let arp_packet = ArpPacket::new(
        ArpOperation::Reply,
        src_mac, src_ip, dst_mac, dst_ip,
    );
    let frame = encapsulate_ethernet(&arp_packet, dst_mac, src_mac);
    Packet::from_bytes(frame)
}

// 注入报文到接口
fn inject_packet(context: &SystemContext, iface_name: &str, packet: Packet) {
    let mut interfaces = context.interfaces.lock().unwrap();
    let iface = interfaces.get_by_name_mut(iface_name).unwrap();
    iface.rxq.enqueue(packet).unwrap();
}

// 验证ARP缓存条目
fn verify_arp_entry(context: &SystemContext, ifindex: u32, ip: Ipv4Addr, expected_mac: MacAddr) -> bool {
    let cache = context.arp_cache.lock().unwrap();
    cache.lookup_arp(ifindex, ip)
        .map(|e| e.hardware_addr == expected_mac && e.state == ArpState::Reachable)
        .unwrap_or(false)
}

// 测试1：收到ARP请求（目标IP是本机），应发送响应
#[test]
#[serial]
fn test_arp_request_to_local() {
    let ctx = create_test_context();
    let sender_mac = MacAddr::new([0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0x01]);
    let sender_ip = Ipv4Addr::new(192, 168, 1, 10);
    let target_ip = Ipv4Addr::new(192, 168, 1, 100); // 本机IP

    inject_packet(&ctx, "eth0", create_arp_request_packet(sender_mac, sender_ip, target_ip));

    let mut harness = TestHarness::with_context(ctx.clone());
    harness.run().unwrap();

    // 验证：发送队列有1个响应报文，且缓存学习了发送方MAC
    let interfaces = ctx.interfaces.lock().unwrap();
    let iface = interfaces.get_by_name("eth0").unwrap();
    assert_eq!(iface.txq.len(), 1, "应发送ARP响应");
    assert!(verify_arp_entry(&ctx, 0, sender_ip, sender_mac), "应学习发送方MAC");
}

// 测试2：收到ARP响应，应更新缓存
#[test]
#[serial]
fn test_arp_reply_updates_cache() {
    let ctx = create_test_context();
    let sender_mac = MacAddr::new([0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0x01]);
    let sender_ip = Ipv4Addr::new(192, 168, 1, 10);
    let local_mac = MacAddr::new([0x00, 0x11, 0x22, 0x33, 0x44, 0x55]);
    let local_ip = Ipv4Addr::new(192, 168, 1, 100);

    inject_packet(&ctx, "eth0", create_arp_reply_packet(sender_mac, sender_ip, local_mac, local_ip));

    let mut harness = TestHarness::with_context(ctx.clone());
    harness.run().unwrap();

    assert!(verify_arp_entry(&ctx, 0, sender_ip, sender_mac), "应学习ARP响应中的MAC");
}

// 测试3：收到格式错误的ARP报文，不崩溃
#[test]
#[serial]
fn test_malformed_arp_packet() {
    let ctx = create_test_context();
    // 创建过短的ARP报文
    let short_packet = vec![
        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, // DST MAC
        0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0x01, // SRC MAC
        0x08, 0x06, // Ether Type = ARP
        0x00, 0x01, 0x08, 0x00, 0x06, 0x04, // ARP头（不完整）
    ];

    inject_packet(&ctx, "eth0", Packet::from_bytes(short_packet));

    let mut harness = TestHarness::with_context(ctx.clone());
    let result = harness.run();

    // 应该正常完成，不崩溃
    assert!(result.is_ok(), "格式错误的ARP报文不应导致崩溃");
}
