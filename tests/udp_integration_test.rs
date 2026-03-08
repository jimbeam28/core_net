// UDP 协议集成测试（精简版）
//
// 核心功能测试：UDP头部解析、数据报封装

use core_net::common::Packet;
use core_net::interface::Ipv4Addr;
use core_net::protocols::udp::{UdpDatagram, UdpHeader, encapsulate_udp_datagram};
use serial_test::serial;

// 测试1：UDP头部解析
#[test]
#[serial]
fn test_udp_header_parse() {
    let bytes = [
        0x04, 0xD2, // Source Port: 1234
        0x16, 0x2E, // Dest Port: 5678
        0x00, 0x0C, // Length: 12
        0xAB, 0xCD, // Checksum: 0xABCD
    ];

    let header = UdpHeader::parse(&bytes).unwrap();
    assert_eq!(header.source_port, 1234);
    assert_eq!(header.destination_port, 5678);
    assert_eq!(header.length, 12);
}

// 测试2：UDP数据报解析
#[test]
#[serial]
fn test_udp_datagram_parse() {
    let bytes = [
        0x04, 0xD2, // Source Port
        0x16, 0x2E, // Dest Port
        0x00, 0x10, // Length: 16 (8 + 8)
        0x00, 0x00, // Checksum
        0x48, 0x65, 0x6C, 0x6C, 0x6F, 0x2C, 0x55, 0x44, // "Hello,UD"
    ];

    let datagram = UdpDatagram::parse(&bytes).unwrap();
    assert_eq!(datagram.header.source_port, 1234);
    assert_eq!(datagram.header.destination_port, 5678);
    assert_eq!(datagram.payload, b"Hello,UD");
}

// 测试3：UDP数据报封装
#[test]
#[serial]
fn test_udp_encapsulation() {
    let src_ip = Ipv4Addr::new(192, 168, 1, 10);
    let dst_ip = Ipv4Addr::new(192, 168, 1, 100);
    let payload = b"Test data";

    let udp_data = encapsulate_udp_datagram(
        1234, 5678, src_ip, dst_ip, payload, false
    );

    // 验证长度: 头部8字节 + 数据9字节 = 17字节
    assert_eq!(udp_data.len(), 17);

    // 验证端口
    assert_eq!(u16::from_be_bytes([udp_data[0], udp_data[1]]), 1234);
    assert_eq!(u16::from_be_bytes([udp_data[2], udp_data[3]]), 5678);
}

// 测试4：UDP校验和计算
#[test]
#[serial]
fn test_udp_checksum() {
    let src_ip = Ipv4Addr::new(192, 168, 1, 10);
    let dst_ip = Ipv4Addr::new(192, 168, 1, 100);

    let udp_data = encapsulate_udp_datagram(
        1234, 5678, src_ip, dst_ip, b"Hello", true
    );

    // 验证头部中的校验和字段不为0（已计算）
    let checksum = u16::from_be_bytes([udp_data[6], udp_data[7]]);
    assert_ne!(checksum, 0, "校验和应已计算");
}

// 测试5：无效UDP数据报（长度过短）
#[test]
#[serial]
fn test_udp_invalid_length() {
    let bytes = [0x04, 0xD2, 0x16, 0x2E]; // 只有4字节，不足8字节头部

    let result = UdpDatagram::parse(&bytes);
    assert!(result.is_err(), "过短的数据报应解析失败");
}
