// IPv4 协议集成测试（精简版）
//
// 核心功能测试：IP头部解析、分片检测

use core_net::interface::Ipv4Addr;
use core_net::protocols::ip::Ipv4Header;
use core_net::protocols::IP_PROTO_ICMP;
use serial_test::serial;

// 测试1：IP头部解析
#[test]
#[serial]
fn test_ip_header_parse() {
    let src_ip = Ipv4Addr::new(192, 168, 1, 10);
    let dst_ip = Ipv4Addr::new(192, 168, 1, 100);
    let ip_header = Ipv4Header::new(src_ip, dst_ip, IP_PROTO_ICMP, 64);

    assert_eq!(ip_header.version(), 4);
    assert_eq!(ip_header.header_len(), 20);
    assert_eq!(ip_header.protocol, IP_PROTO_ICMP);
    assert_eq!(ip_header.source_addr, src_ip);
    assert_eq!(ip_header.dest_addr, dst_ip);
}

// 测试2：IP头部标志位
#[test]
#[serial]
fn test_ip_header_flags() {
    let header = Ipv4Header::new(
        Ipv4Addr::new(192, 168, 1, 1),
        Ipv4Addr::new(192, 168, 1, 2),
        IP_PROTO_ICMP,
        64,
    );

    assert!(header.has_df_flag());
    assert!(!header.has_mf_flag());
    assert_eq!(header.fragment_offset(), 0);
}

// 测试3：IP头部序列化
#[test]
#[serial]
fn test_ip_header_serialization() {
    let src = Ipv4Addr::new(192, 168, 1, 1);
    let dst = Ipv4Addr::new(192, 168, 1, 2);
    let header = Ipv4Header::new(src, dst, IP_PROTO_ICMP, 64);

    let bytes = header.to_bytes();
    assert_eq!(bytes.len(), 20);

    // 验证版本 (高4位 = 4)
    assert_eq!(bytes[0] >> 4, 4);
}

// 测试4：地址类型判断
#[test]
#[serial]
fn test_ip_address_types() {
    let multicast = Ipv4Addr::new(224, 0, 0, 1);
    let unicast = Ipv4Addr::new(192, 168, 1, 1);

    assert!(multicast.is_multicast());
    assert!(!unicast.is_multicast());
}
