// OSPF 协议集成测试（精简版）
//
// 测试 OSPF 协议的报文头部

use core_net::interface::Ipv4Addr;
use core_net::protocols::ospf2::{OspfHeader, OspfType};
use serial_test::serial;

// 测试 OSPF 头部解析
#[test]
#[serial]
fn test_ospf_header_parse() {
    let ospf_header_bytes = [
        0x02, // 版本 (OSPFv2)
        0x01, // 类型 (Hello)
        0x00, 0x30, // 长度 (48字节)
        0x01, 0x01, 0x01, 0x01, // 路由器 ID (1.1.1.1)
        0x00, 0x00, 0x00, 0x00, // 区域 ID (0.0.0.0)
        0x00, 0x00, // 校验和
        0x00, 0x00, // 认证类型
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // 认证数据
    ];

    let header = OspfHeader::from_bytes(&ospf_header_bytes).unwrap();

    assert_eq!(header.version, 2);
    assert_eq!(header.packet_type, OspfType::Hello);
    assert_eq!(header.length, 48);
}

// 测试 OSPF 头部封装
#[test]
#[serial]
fn test_ospf_header_to_bytes() {
    let header = OspfHeader {
        version: 2,
        packet_type: OspfType::Hello,
        length: 48,
        router_id: Ipv4Addr::new(1, 1, 1, 1),
        area_id: Ipv4Addr::new(0, 0, 0, 0),
        checksum: 0,
        auth_type: 0,
        auth_data: [0; 8],
    };

    let bytes = header.to_bytes();

    assert_eq!(bytes.len(), 24);
    assert_eq!(bytes[0], 2); // 版本
    assert_eq!(bytes[1], 1); // 类型 (Hello)
}
