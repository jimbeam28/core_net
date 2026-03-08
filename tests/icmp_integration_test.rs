// ICMP 协议集成测试（精简版）
//
// 核心功能测试：Echo Request/Reply

use core_net::protocols::icmp::{IcmpEcho, create_echo_request};
use core_net::interface::Ipv4Addr;
use serial_test::serial;

// 测试1：ICMP Echo创建
#[test]
#[serial]
fn test_icmp_echo_creation() {
    let echo = IcmpEcho::new_request(1234, 1, vec![0xAA, 0xBB]);

    assert_eq!(echo.identifier, 1234);
    assert_eq!(echo.sequence, 1);
    assert_eq!(echo.data, vec![0xAA, 0xBB]);
}

// 测试2：ICMP Echo序列化
#[test]
#[serial]
fn test_icmp_echo_serialization() {
    let echo = IcmpEcho::new_request(1, 1, vec![0x01, 0x02, 0x03]);
    let bytes = echo.to_bytes();

    assert_eq!(bytes.len(), 8 + 3); // 头部8字节 + 数据3字节
}

// 测试3：创建Echo Request
#[test]
#[serial]
fn test_icmp_echo_request() {
    let echo_data = create_echo_request(1234, 1, vec![0xAA; 10]);

    // 验证ICMP类型 (8 = Echo Request)
    assert_eq!(echo_data[0], 8);
    assert_eq!(echo_data[1], 0); // Code = 0
}
