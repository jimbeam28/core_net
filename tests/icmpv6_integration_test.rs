// ICMPv6 协议集成测试（精简版）
//
// 核心功能测试：ICMPv6 Echo、邻居发现

use core_net::protocols::icmpv6::{Icmpv6Echo, Icmpv6NeighborSolicitation};
use core_net::protocols::Ipv6Addr;
use serial_test::serial;

// 测试1：ICMPv6 Echo创建
#[test]
#[serial]
fn test_icmpv6_echo_creation() {
    let echo = Icmpv6Echo::new_request(1, 1, vec![0xAA, 0xBB, 0xCC]);

    assert_eq!(echo.identifier, 1);
    assert_eq!(echo.sequence, 1);
    assert_eq!(echo.data, vec![0xAA, 0xBB, 0xCC]);
}

// 测试2：ICMPv6 Echo Reply创建
#[test]
#[serial]
fn test_icmpv6_echo_reply() {
    let echo = Icmpv6Echo::new_request(1, 1, vec![0x01, 0x02]);
    let reply = Icmpv6Echo::new_reply(1, 1, vec![0x01, 0x02]);

    assert_eq!(reply.identifier, echo.identifier);
    assert_eq!(reply.sequence, echo.sequence);
}
