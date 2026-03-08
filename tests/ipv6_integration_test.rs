// IPv6 协议集成测试（精简版）
//
// 核心功能测试：IPv6头部解析和封装

use core_net::protocols::Ipv6Addr;
use core_net::protocols::ipv6::{Ipv6Header, IpProtocol, encapsulate_ipv6_packet};
use serial_test::serial;

// 测试1：IPv6头部解析
#[test]
#[serial]
fn test_ipv6_header_parse() {
    let src_ip = Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 1);
    let dst_ip = Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 2);
    let header = Ipv6Header::new(src_ip, dst_ip, 64, IpProtocol::IcmpV6, 64);

    assert_eq!(header.version, 6);
    assert_eq!(header.header_len(), 40);
    assert_eq!(header.next_header, IpProtocol::IcmpV6);
    assert_eq!(header.source_addr, src_ip);
    assert_eq!(header.payload_length, 64);
}

// 测试2：IPv6地址类型判断
#[test]
#[serial]
fn test_ipv6_address_types() {
    let link_local = Ipv6Addr::new(0xfe80, 0, 0, 0, 0, 0, 0, 1);
    let multicast = Ipv6Addr::new(0xff02, 0, 0, 0, 0, 0, 0, 1);
    let global = Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 1);

    assert!(link_local.is_link_local());
    assert!(multicast.is_multicast());
    assert!(!global.is_link_local());
}

// 测试3：IPv6数据包封装
#[test]
#[serial]
fn test_ipv6_encapsulation() {
    let src = Ipv6Addr::new(0xfe80, 0, 0, 0, 0, 0, 0, 1);
    let dst = Ipv6Addr::new(0xfe80, 0, 0, 0, 0, 0, 0, 2);
    let payload = vec![0x80, 0x00, 0x00, 0x00]; // ICMPv6 Echo Request

    let packet = encapsulate_ipv6_packet(src, dst, IpProtocol::IcmpV6, &payload, 64);

    assert!(packet.len() >= 40); // 至少40字节IPv6头部
}

// 测试4：IPv6头部序列化
#[test]
#[serial]
fn test_ipv6_header_serialization() {
    let src = Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 1);
    let dst = Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 2);
    let header = Ipv6Header::new(src, dst, 100, IpProtocol::Tcp, 64);

    let bytes = header.to_bytes();
    assert_eq!(bytes.len(), 40);

    // 验证版本 (高4位 = 6)
    assert_eq!(bytes[0] >> 4, 6);
}
