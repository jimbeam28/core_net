// OSPFv3 协议集成测试（精简版）
//
// 测试 OSPFv3 协议的类型定义

use core_net::protocols::ospf3::Ospfv3Type;
use serial_test::serial;

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
