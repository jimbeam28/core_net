// OSPF 协议集成测试（精简版）
//
// 测试 OSPF 协议的类型定义

use core_net::protocols::ospf2::OspfType;
use serial_test::serial;

// 测试 OSPF 类型定义
#[test]
#[serial]
fn test_ospf_types() {
    assert_eq!(OspfType::Hello as u8, 1);
    assert_eq!(OspfType::DatabaseDescription as u8, 2);
    assert_eq!(OspfType::LinkStateRequest as u8, 3);
    assert_eq!(OspfType::LinkStateUpdate as u8, 4);
    assert_eq!(OspfType::LinkStateAck as u8, 5);
}
