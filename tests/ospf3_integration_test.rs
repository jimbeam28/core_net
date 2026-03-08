// OSPFv3 协议集成测试（精简版）
//
// 测试 OSPFv3 协议的报文解析、封装基础功能

use core_net::protocols::ospf3::{
    Ospfv3Header, Ospfv3Type,
};

use serial_test::serial;

// 测试 OSPFv3 头部解析
#[test]
#[serial]
fn test_ospfv3_header_parse() {
    // 构造一个简单的 OSPFv3 Hello 报文头部
    let ospf_header_bytes = [
        0x03, // 版本 (OSPFv3)
        0x01, // 类型 (Hello)
        0x00, 0x2c, // 长度 (44字节)
        0x00, 0x00, 0x00, 0x01, // 路由器 ID (1)
        0x00, 0x00, 0x00, 0x00, // 区域 ID (0)
        0x00, 0x00, // 校验和
        0x00, 0x00, // 实例 ID + 保留
        0x00, 0x00, // 保留
    ];

    let header = Ospfv3Header::from_bytes(&ospf_header_bytes).unwrap();

    assert_eq!(header.version, 3);
    assert_eq!(header.packet_type, 1); // Hello
    assert_eq!(header.length, 44);
    assert_eq!(header.router_id, 1);
    assert_eq!(header.area_id, 0);
}

// 测试 OSPFv3 类型定义
#[test]
#[serial]
fn test_ospfv3_types() {
    assert_eq!(Ospfv3Type::Hello as u8, 1);
    assert_eq!(Ospfv3Type::DatabaseDescription as u8, 2);
    assert_eq!(Ospfv3Type::LinkStateRequest as u8, 3);
    assert_eq!(Ospfv3Type::LinkStateUpdate as u8, 4);
    assert_eq!(Ospfv3Type::LinkStateAck as u8, 5);
}
