// TCP 协议集成测试（精简版）
//
// 核心功能测试：TCP头部解析、连接建立

use core_net::testframework::TestHarness;
use core_net::common::Packet;
use core_net::interface::{MacAddr, Ipv4Addr};
use core_net::protocols::ETH_P_IP;
use core_net::protocols::tcp::{TcpHeader, TcpSegment, create_syn, create_ack};
use core_net::context::SystemContext;

use serial_test::serial;

mod common;
use common::create_test_context;

fn inject_packet(context: &SystemContext, iface_name: &str, packet: Packet) {
    let mut interfaces = context.interfaces.lock().unwrap();
    let iface = interfaces.get_by_name_mut(iface_name).unwrap();
    iface.rxq.enqueue(packet).unwrap();
}

// 创建带以太网和IP封装的TCP报文
fn create_tcp_packet(src_mac: MacAddr, src_ip: Ipv4Addr, dst_ip: Ipv4Addr, tcp_data: Vec<u8>) -> Packet {
    use core_net::protocols::ip::Ipv4Header;
    use core_net::protocols::IP_PROTO_TCP;

    let ip_header = Ipv4Header::new(src_ip, dst_ip, IP_PROTO_TCP, tcp_data.len());
    let mut ip_packet = ip_header.to_bytes();
    ip_packet.extend_from_slice(&tcp_data);

    let mut frame = Vec::new();
    frame.extend_from_slice(&src_mac.bytes);
    frame.extend_from_slice(&src_mac.bytes); // 简化：使用相同MAC作为目标
    frame.extend_from_slice(&ETH_P_IP.to_be_bytes());
    frame.extend_from_slice(&ip_packet);

    Packet::from_bytes(frame)
}

// 测试1：TCP头部解析
#[test]
#[serial]
fn test_tcp_header_parse() {
    let bytes = [
        0x04, 0xD2, // Source Port: 1234
        0x16, 0x2E, // Dest Port: 5678
        0x00, 0x00, 0x03, 0xE8, // Seq: 1000
        0x00, 0x00, 0x01, 0xF4, // Ack: 500
        0x50, 0x18, // Data Offset: 5, Flags: ACK + PSH
        0x20, 0x00, // Window: 8192
        0x00, 0x00, // Checksum
        0x00, 0x00, // Urgent Pointer
    ];

    let header = TcpHeader::parse(&bytes).unwrap();
    assert_eq!(header.source_port, 1234);
    assert_eq!(header.destination_port, 5678);
    assert_eq!(header.sequence_number, 1000);
    assert!(header.is_ack());
}

// 测试2：TCP段解析
#[test]
#[serial]
fn test_tcp_segment_parse() {
    let bytes = [
        0x04, 0xD2, 0x16, 0x2E, // Ports
        0x00, 0x00, 0x03, 0xE8, // Seq
        0x00, 0x00, 0x01, 0xF4, // Ack
        0x50, 0x18, 0x20, 0x00, // Flags, Window
        0x00, 0x00, 0x00, 0x00, // Checksum, Urgent
        0x48, 0x65, 0x6C, 0x6C, 0x6F, // "Hello"
    ];

    let segment = TcpSegment::parse(&bytes).unwrap();
    assert_eq!(segment.header.source_port, 1234);
    assert_eq!(segment.payload, b"Hello");
}

// 测试3：创建SYN报文
#[test]
#[serial]
fn test_tcp_syn_creation() {
    let sender_ip = Ipv4Addr::new(192, 168, 1, 10);
    let target_ip = Ipv4Addr::new(192, 168, 1, 100);

    let tcp_data = create_syn(1234, 80, sender_ip, target_ip, 1000, 8192);

    assert_eq!(tcp_data.len(), 20); // 基本头部
    assert_eq!(u16::from_be_bytes([tcp_data[0], tcp_data[1]]), 1234);
    assert!(tcp_data[13] & 0x02 != 0, "应有SYN标志");
}

// 测试4：创建ACK报文
#[test]
#[serial]
fn test_tcp_ack_creation() {
    let sender_ip = Ipv4Addr::new(192, 168, 1, 10);
    let target_ip = Ipv4Addr::new(192, 168, 1, 100);

    let tcp_data = create_ack(80, 1234, target_ip, sender_ip, 2000, 1001, 8192);

    assert!(tcp_data[13] & 0x10 != 0, "应有ACK标志");
}

// 测试5：处理TCP SYN报文（简化处理流程验证）
#[test]
#[serial]
fn test_tcp_syn_processing() {
    let ctx = create_test_context();
    let src_mac = MacAddr::new([0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0x01]);
    let src_ip = Ipv4Addr::new(192, 168, 1, 10);
    let dst_ip = Ipv4Addr::new(192, 168, 1, 100);

    // 创建SYN报文
    let tcp_data = create_syn(12345, 80, src_ip, dst_ip, 1000, 8192);
    let packet = create_tcp_packet(src_mac, src_ip, dst_ip, tcp_data);

    inject_packet(&ctx, "eth0", packet);

    let mut harness = TestHarness::with_context(ctx.clone());
    let result = harness.run();

    // 验证处理不崩溃
    assert!(result.is_ok(), "TCP SYN处理不应崩溃");
}
