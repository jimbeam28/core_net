// TCP 协议集成测试
//
// 测试 TCP 协议的报文解析、封装、连接管理

use core_net::testframework::TestHarness;
use core_net::common::Packet;
use core_net::interface::{MacAddr, Ipv4Addr};
use core_net::protocols::ETH_P_IP;
use core_net::protocols::tcp::{TcpHeader, TcpSegment, encapsulate_tcp_segment, create_syn, create_ack};
use serial_test::serial;

mod common;
use common::{create_ip_header, inject_packet_to_context, verify_context_txq_count,
             create_test_context};

// 测试环境配置：本机接口 eth0: ifindex=0, MAC=00:11:22:33:44:55, IP=192.168.1.100

// ========== 基本功能测试组 ==========

#[test]
#[serial]
fn test_tcp_header_parse() {
    let bytes = [
        // TCP 头部
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
    assert_eq!(header.acknowledgment_number, 500);
    assert!(header.is_ack());
    assert!(header.is_psh());
}

#[test]
#[serial]
fn test_tcp_segment_parse() {
    let bytes = [
        // TCP 头部
        0x04, 0xD2, // Source Port
        0x16, 0x2E, // Dest Port
        0x00, 0x00, 0x03, 0xE8, // Seq
        0x00, 0x00, 0x01, 0xF4, // Ack
        0x50, 0x18, // Flags: ACK + PSH
        0x20, 0x00, // Window
        0x00, 0x00, // Checksum
        0x00, 0x00, // Urgent Pointer
        // 数据
        0x48, 0x65, 0x6C, 0x6C, 0x6F, // "Hello"
    ];

    let segment = TcpSegment::parse(&bytes).unwrap();
    assert_eq!(segment.header.source_port, 1234);
    assert_eq!(segment.header.destination_port, 5678);
    assert_eq!(segment.payload, b"Hello");
}

#[test]
#[serial]
fn test_tcp_syn_packet() {
    let sender_ip = Ipv4Addr::new(192, 168, 1, 10);
    let target_ip = Ipv4Addr::new(192, 168, 1, 100);

    // 创建 SYN 报文
    let tcp_data = create_syn(1234, 80, sender_ip, target_ip, 1000, 8192);

    // 验证长度
    assert_eq!(tcp_data.len(), 20); // 基本头部

    // 验证源端口
    assert_eq!(tcp_data[0..2], 1234u16.to_be_bytes());

    // 验证 SYN 标志
    assert!(tcp_data[13] & 0x02 != 0);
}

#[test]
#[serial]
fn test_tcp_ack_packet() {
    let sender_ip = Ipv4Addr::new(192, 168, 1, 10);
    let target_ip = Ipv4Addr::new(192, 168, 1, 100);

    // 创建 ACK 报文
    let tcp_data = create_ack(80, 1234, target_ip, sender_ip, 2000, 1001, 8192);

    // 验证 ACK 标志
    assert!(tcp_data[13] & 0x10 != 0);
}

#[test]
#[serial]
fn test_tcp_encapsulate_segment() {
    let src_ip = Ipv4Addr::new(192, 168, 1, 1);
    let dst_ip = Ipv4Addr::new(192, 168, 1, 2);

    let header = TcpHeader::ack(1234, 5678, 1000, 500, 8192);
    let bytes = encapsulate_tcp_segment(&header, &[], src_ip, dst_ip);

    assert_eq!(bytes.len(), 20);
    // 验证校验和已计算
    let checksum = u16::from_be_bytes([bytes[16], bytes[17]]);
    assert_ne!(checksum, 0);
}

// ========== 边界条件测试组 ==========

#[test]
#[serial]
fn test_tcp_minimal_header() {
    let bytes = [
        0x04, 0xD2, // Source Port
        0x16, 0x2E, // Dest Port
        0x00, 0x00, 0x03, 0xE8, // Seq
        0x00, 0x00, 0x01, 0xF4, // Ack
        0x50, 0x10, // Data Offset: 5, Flags: ACK
        0x20, 0x00, // Window
        0x00, 0x00, // Checksum
        0x00, 0x00, // Urgent Pointer
    ];

    let segment = TcpSegment::parse(&bytes).unwrap();
    assert_eq!(segment.header.data_offset(), 5);
    assert!(segment.payload.is_empty());
}

#[test]
#[serial]
fn test_tcp_invalid_data_offset() {
    let bytes = [
        0x04, 0xD2, // Source Port
        0x16, 0x2E, // Dest Port
        0x00, 0x00, 0x03, 0xE8, // Seq
        0x00, 0x00, 0x01, 0xF4, // Ack
        0x40, 0x10, // Data Offset: 4 (invalid, < 5), Flags: ACK
        0x20, 0x00, // Window
        0x00, 0x00, // Checksum
        0x00, 0x00, // Urgent Pointer
    ];

    let result = TcpSegment::parse(&bytes);
    assert!(result.is_err());
}

#[test]
#[serial]
fn test_tcp_header_too_short() {
    let bytes = [0x04, 0xD2, 0x16, 0x2E]; // 只有 4 字节

    let result = TcpSegment::parse(&bytes);
    assert!(result.is_err());
}

#[test]
#[serial]
fn test_tcp_port_boundaries() {
    let bytes = [
        0x00, 0x00, // Port: 0
        0xFF, 0xFF, // Port: 65535
        0x00, 0x00, 0x03, 0xE8, // Seq
        0x00, 0x00, 0x01, 0xF4, // Ack
        0x50, 0x10, // Data Offset: 5, Flags: ACK
        0x20, 0x00, // Window
        0x00, 0x00, // Checksum
        0x00, 0x00, // Urgent Pointer
    ];

    let segment = TcpSegment::parse(&bytes).unwrap();
    assert_eq!(segment.header.source_port, 0);
    assert_eq!(segment.header.destination_port, 65535);
}

// ========== 标志位测试组 ==========

#[test]
#[serial]
fn test_tcp_flags_syn() {
    let bytes = [
        0x04, 0xD2, 0x16, 0x2E,
        0x00, 0x00, 0x03, 0xE8,
        0x00, 0x00, 0x00, 0x00,
        0x50, 0x02, // Flags: SYN
        0x20, 0x00,
        0x00, 0x00,
        0x00, 0x00,
    ];

    let segment = TcpSegment::parse(&bytes).unwrap();
    assert!(segment.header.is_syn());
    assert!(!segment.header.is_ack());
    assert!(!segment.header.is_fin());
}

#[test]
#[serial]
fn test_tcp_flags_syn_ack() {
    let bytes = [
        0x04, 0xD2, 0x16, 0x2E,
        0x00, 0x00, 0x03, 0xE8,
        0x00, 0x00, 0x01, 0xF4,
        0x50, 0x12, // Flags: SYN + ACK
        0x20, 0x00,
        0x00, 0x00,
        0x00, 0x00,
    ];

    let segment = TcpSegment::parse(&bytes).unwrap();
    assert!(segment.header.is_syn());
    assert!(segment.header.is_ack());
}

#[test]
#[serial]
fn test_tcp_flags_fin_ack() {
    let bytes = [
        0x04, 0xD2, 0x16, 0x2E,
        0x00, 0x00, 0x03, 0xE8,
        0x00, 0x00, 0x01, 0xF4,
        0x50, 0x11, // Flags: FIN + ACK
        0x20, 0x00,
        0x00, 0x00,
        0x00, 0x00,
    ];

    let segment = TcpSegment::parse(&bytes).unwrap();
    assert!(segment.header.is_fin());
    assert!(segment.header.is_ack());
}

#[test]
#[serial]
fn test_tcp_flags_rst_ack() {
    let bytes = [
        0x04, 0xD2, 0x16, 0x2E,
        0x00, 0x00, 0x03, 0xE8,
        0x00, 0x00, 0x01, 0xF4,
        0x50, 0x14, // Flags: RST + ACK
        0x00, 0x00,
        0x00, 0x00,
        0x00, 0x00,
    ];

    let segment = TcpSegment::parse(&bytes).unwrap();
    assert!(segment.header.is_rst());
    assert!(segment.header.is_ack());
}

// ========== 序列号和确认号测试组 ==========

#[test]
#[serial]
fn test_tcp_sequence_wraparound() {
    let bytes = [
        0x04, 0xD2, 0x16, 0x2E,
        0xFF, 0xFF, 0xFF, 0xFF, // Seq: 0xFFFFFFFF
        0x00, 0x00, 0x00, 0x01, // Ack: 1
        0x50, 0x10,
        0x20, 0x00,
        0x00, 0x00,
        0x00, 0x00,
    ];

    let segment = TcpSegment::parse(&bytes).unwrap();
    assert_eq!(segment.header.sequence_number, 0xFFFFFFFF);
    assert_eq!(segment.header.acknowledgment_number, 1);
}

// ========== 窗口大小测试组 ==========

#[test]
#[serial]
fn test_tcp_window_size() {
    let bytes = [
        0x04, 0xD2, 0x16, 0x2E,
        0x00, 0x00, 0x03, 0xE8,
        0x00, 0x00, 0x01, 0xF4,
        0x50, 0x10,
        0xFF, 0xFF, // Window: 65535
        0x00, 0x00,
        0x00, 0x00,
    ];

    let segment = TcpSegment::parse(&bytes).unwrap();
    assert_eq!(segment.header.window_size, 65535);
}

#[test]
#[serial]
fn test_tcp_zero_window() {
    let bytes = [
        0x04, 0xD2, 0x16, 0x2E,
        0x00, 0x00, 0x03, 0xE8,
        0x00, 0x00, 0x01, 0xF4,
        0x50, 0x10,
        0x00, 0x00, // Window: 0
        0x00, 0x00,
        0x00, 0x00,
    ];

    let segment = TcpSegment::parse(&bytes).unwrap();
    assert_eq!(segment.header.window_size, 0);
}
