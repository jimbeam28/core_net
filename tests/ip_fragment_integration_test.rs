// IPv4 分片和重组集成测试（精简版）
//
// 核心功能测试：分片创建、重组

use core_net::interface::Ipv4Addr;
use core_net::protocols::ip::{fragment_datagram, ReassemblyKey, ReassemblyTable};
use core_net::protocols::IP_PROTO_ICMP;
use serial_test::serial;

// 测试1：基本分片
#[test]
#[serial]
fn test_fragment_datagram_basic() {
    let src_ip = Ipv4Addr::new(192, 168, 1, 1);
    let dst_ip = Ipv4Addr::new(192, 168, 1, 2);
    let payload = vec![0x42u8; 4000];

    let fragments = fragment_datagram(src_ip, dst_ip, IP_PROTO_ICMP, &payload, 1500, 12345);

    assert_eq!(fragments.len(), 3);
}

// 测试2：重组表创建
#[test]
#[serial]
fn test_reassembly_table_creation() {
    let table = ReassemblyTable::new(100, 60);
    assert!(table.is_empty());
}

// 测试3：重组键创建
#[test]
#[serial]
fn test_reassembly_key() {
    let src = Ipv4Addr::new(192, 168, 1, 1);
    let dst = Ipv4Addr::new(192, 168, 1, 2);
    let key = ReassemblyKey::new(src, dst, 123, IP_PROTO_ICMP.into());

    assert_eq!(key.source_addr, src);
    assert_eq!(key.dest_addr, dst);
}
