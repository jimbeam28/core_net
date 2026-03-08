// BGP 协议集成测试（精简版）
//
// 测试 BGP 协议的报文头部创建、Open报文

use core_net::protocols::bgp::{
    BgpHeader, BgpOpen, BGP_VERSION, BGP_MARKER_SIZE, BGP_MIN_MESSAGE_SIZE,
};
use core_net::interface::Ipv4Addr;
use serial_test::serial;

// 测试1：BGP头部创建
#[test]
#[serial]
fn test_bgp_header_creation() {
    let header = BgpHeader::new(19, 4); // Keepalive长度19，类型4

    assert_eq!(header.marker, [0xff; BGP_MARKER_SIZE]);
    assert_eq!(header.length, 19);
    assert_eq!(header.msg_type, 4);
}

// 测试2：BGP头部验证
#[test]
#[serial]
fn test_bgp_header_validation() {
    let header = BgpHeader::new(19, 4);
    assert!(header.validate_length());

    // 过短的头部
    let short_header = BgpHeader::new(10, 4);
    assert!(!short_header.validate_length());
}

// 测试3：BGP Open报文创建
#[test]
#[serial]
fn test_bgp_open_message() {
    let open = BgpOpen {
        version: BGP_VERSION,
        my_as: 65001,
        hold_time: 180,
        bgp_identifier: Ipv4Addr::new(192, 168, 1, 1),
        optional_parameters: vec![],
    };

    assert_eq!(open.version, 4);
    assert_eq!(open.my_as, 65001);
    assert_eq!(open.hold_time, 180);
}
