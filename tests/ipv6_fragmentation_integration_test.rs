// IPv6 分片与重组集成测试（精简版）
//
// 核心功能测试：分片创建

use core_net::protocols::ipv6::create_fragments_simple;
use serial_test::serial;

// 测试1：创建分片
#[test]
#[serial]
fn test_create_fragments() {
    let payload = vec![0xAAu8; 2000];

    let fragments = create_fragments_simple(&payload, 1280, 0x12345678, 58);

    assert!(!fragments.is_empty());
}
