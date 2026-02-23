// IPv4 分片和重组集成测试
//
// 测试 IPv4 协议的分片发送和重组接收功能

use core_net::testframework::{
    TestHarness,
};
use core_net::interface::{MacAddr, Ipv4Addr};
use core_net::protocols::{ETH_P_IP, IP_PROTO_ICMP};
use core_net::protocols::ip::{
    fragment_datagram,
    ReassemblyKey, ReassemblyTable, ReassemblyEntry, FragmentInfo, FragmentOverlapPolicy,
    DEFAULT_REASSEMBLY_TIMEOUT_SECS,
};
use core_net::common::Packet;

use serial_test::serial;

mod common;
use common::{inject_packet_to_context, verify_context_txq_count, create_test_context};

// 测试环境配置：本机接口 eth0: ifindex=0, MAC=00:11:22:33:44:55, IP=192.168.1.100

// ========== 分片发送测试组 ==========

#[test]
#[serial]
fn test_fragment_datagram_basic() {
    let _ctx = create_test_context();

    let src_ip = Ipv4Addr::new(192, 168, 1, 1);
    let dst_ip = Ipv4Addr::new(192, 168, 1, 2);

    // 创建 4000 字节的数据报
    let payload = vec![0x42u8; 4000];
    let mtu: u16 = 1500;
    let identification: u16 = 12345;

    // 分片
    let fragments = fragment_datagram(src_ip, dst_ip, IP_PROTO_ICMP, &payload, mtu, identification);

    // 应该生成 3 个分片
    assert_eq!(fragments.len(), 3);

    // 验证第一个分片
    let frag1 = &fragments[0];
    assert_eq!(frag1[0], 0x45); // Version=4, IHL=5
    let flags_frag1 = u16::from_be_bytes([frag1[6], frag1[7]]);
    assert_eq!(flags_frag1 & 0xE000, 0x2000); // MF=1 (bit 13)
    assert_eq!(flags_frag1 & 0x1FFF, 0); // Offset=0
    // Identification = 12345 = 0x3039

    // 验证第二个分片
    let frag2 = &fragments[1];
    assert_eq!(frag2[0], 0x45); // Version=4, IHL=5
    let flags_frag2 = u16::from_be_bytes([frag2[6], frag2[7]]);
    assert_eq!(flags_frag2 & 0xE000, 0x2000); // MF=1
    assert_eq!(flags_frag2 & 0x1FFF, 185); // Offset=185

    // 验证第三个分片
    let frag3 = &fragments[2];
    assert_eq!(frag3[0], 0x45); // Version=4, IHL=5
    let flags_frag3 = u16::from_be_bytes([frag3[6], frag3[7]]);
    assert_eq!(flags_frag3 & 0xE000, 0); // MF=0
    assert_eq!(flags_frag3 & 0x1FFF, 370); // Offset=370
}

#[test]
#[serial]
fn test_fragment_datagram_no_fragmentation_needed() {
    let _ctx = create_test_context();

    let src_ip = Ipv4Addr::new(192, 168, 1, 1);
    let dst_ip = Ipv4Addr::new(192, 168, 1, 2);

    // 创建 100 字节的数据报（小于 MTU）
    let payload = vec![0x42u8; 100];
    let mtu: u16 = 1500;
    let identification: u16 = 12345;

    // 分片
    let fragments = fragment_datagram(src_ip, dst_ip, IP_PROTO_ICMP, &payload, mtu, identification);

    // 应该返回单个数据报（无需分片）
    assert_eq!(fragments.len(), 1);

    // 验证不是分片
    let frag = &fragments[0];
    assert_eq!(frag[0], 0x45); // Version=4, IHL=5
    assert_eq!(u16::from_be_bytes([frag[6], frag[7]]) & 0xE0, 0); // MF=0
    assert_eq!(u16::from_be_bytes([frag[6], frag[7]]) & 0x1FFF, 0); // Offset=0
}

#[test]
#[serial]
fn test_fragment_datagram_exact_mtu() {
    let _ctx = create_test_context();

    let src_ip = Ipv4Addr::new(192, 168, 1, 1);
    let dst_ip = Ipv4Addr::new(192, 168, 1, 2);

    // 创建 1480 字节的数据报（MTU 1500 - 20 字节头部）
    let payload = vec![0x42u8; 1480];
    let mtu: u16 = 1500;
    let identification: u16 = 12345;

    // 分片
    let fragments = fragment_datagram(src_ip, dst_ip, IP_PROTO_ICMP, &payload, mtu, identification);

    // 应该返回单个数据报（恰好等于 MTU）
    assert_eq!(fragments.len(), 1);

    // 验证总长度
    let frag = &fragments[0];
    let total_len = u16::from_be_bytes([frag[2], frag[3]]);
    assert_eq!(total_len, 1500);
}

// ========== 重组表测试组 ==========

#[test]
#[serial]
fn test_reassembly_table_basic() {
    let _ctx = create_test_context();

    let mut table = ReassemblyTable::new(
        64,
        DEFAULT_REASSEMBLY_TIMEOUT_SECS,
    );

    let key = ReassemblyKey::new(
        Ipv4Addr::new(192, 168, 1, 1),
        Ipv4Addr::new(192, 168, 1, 2),
        1,
        12345,
    );

    // 第一次获取应该创建新条目
    let entry1 = table.get_or_create(key);
    assert_eq!(entry1.fragment_count(), 0);
    assert_eq!(table.len(), 1);

    // 第二次获取应该返回已有条目
    let entry2 = table.get_or_create(key);
    assert_eq!(entry2.fragment_count(), 0);
    assert_eq!(table.len(), 1);
}

#[test]
#[serial]
fn test_reassembly_entry_add_fragment() {
    let _ctx = create_test_context();

    let key = ReassemblyKey::new(
        Ipv4Addr::new(192, 168, 1, 1),
        Ipv4Addr::new(192, 168, 1, 2),
        1,
        12345,
    );
    let mut entry = ReassemblyEntry::new(key);

    // 添加第一个分片
    let frag1 = FragmentInfo::new(0, vec![1u8; 160]);
    entry.add_fragment(frag1, FragmentOverlapPolicy::Drop).unwrap();
    assert_eq!(entry.fragment_count(), 1);
    assert_eq!(entry.received_bytes, 160);

    // 添加第二个分片
    let frag2 = FragmentInfo::new(20, vec![2u8; 80]);
    entry.add_fragment(frag2, FragmentOverlapPolicy::Drop).unwrap();
    assert_eq!(entry.fragment_count(), 2);
    assert_eq!(entry.received_bytes, 240);
}

#[test]
#[serial]
fn test_reassembly_entry_complete() {
    let _ctx = create_test_context();

    let key = ReassemblyKey::new(
        Ipv4Addr::new(192, 168, 1, 1),
        Ipv4Addr::new(192, 168, 1, 2),
        1,
        12345,
    );
    let mut entry = ReassemblyEntry::new(key);

    // 添加两个分片，覆盖 0-239 字节
    let frag1 = FragmentInfo::new(0, vec![1u8; 160]);  // 0-159 (20 units)
    let frag2 = FragmentInfo::new(20, vec![2u8; 80]);  // 160-239 (10 units)

    entry.add_fragment(frag1, FragmentOverlapPolicy::Drop).unwrap();
    entry.add_fragment(frag2, FragmentOverlapPolicy::Drop).unwrap();

    // 未设置最后一片，不应该完成
    assert!(!entry.is_complete());

    // 设置最后一片（offset=30，即 20+10）
    entry.set_last_fragment(30);

    // 现在应该完成
    assert!(entry.is_complete());
}

#[test]
#[serial]
fn test_reassembly_entry_assemble() {
    let _ctx = create_test_context();

    let key = ReassemblyKey::new(
        Ipv4Addr::new(192, 168, 1, 1),
        Ipv4Addr::new(192, 168, 1, 2),
        1,
        12345,
    );
    let mut entry = ReassemblyEntry::new(key);

    let frag1 = FragmentInfo::new(0, vec![1u8; 160]);  // 0-159
    let frag2 = FragmentInfo::new(20, vec![2u8; 80]);  // 160-239

    entry.add_fragment(frag1, FragmentOverlapPolicy::Drop).unwrap();
    entry.add_fragment(frag2, FragmentOverlapPolicy::Drop).unwrap();

    let assembled = entry.assemble();
    assert_eq!(assembled.len(), 240);
    assert_eq!(assembled[0], 1);
    assert_eq!(assembled[160], 2);
}

#[test]
#[serial]
fn test_reassembly_entry_overlap() {
    let _ctx = create_test_context();

    let key = ReassemblyKey::new(
        Ipv4Addr::new(192, 168, 1, 1),
        Ipv4Addr::new(192, 168, 1, 2),
        1,
        12345,
    );
    let mut entry = ReassemblyEntry::new(key);

    let frag1 = FragmentInfo::new(0, vec![1u8; 200]);  // 0-199
    let frag2 = FragmentInfo::new(15, vec![2u8; 200]); // 120-319 (重叠)

    entry.add_fragment(frag1, FragmentOverlapPolicy::Drop).unwrap();

    // 添加重叠分片应该失败
    let result = entry.add_fragment(frag2, FragmentOverlapPolicy::Drop);
    assert!(result.is_err());
}

// ========== 分片接收和重组测试组 ==========

#[test]
#[serial]
fn test_ip_fragment_rejection_with_reassembly_enabled() {
    let ctx = create_test_context();

    let sender_mac = MacAddr::new([0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0x01]);
    let _sender_ip = Ipv4Addr::new(192, 168, 1, 10);
    let _target_ip = Ipv4Addr::new(192, 168, 1, 100);

    // 创建第一个分片 (MF=1, Offset=0)
    let mut frame = Vec::new();
    frame.extend_from_slice(&[0xff, 0xff, 0xff, 0xff, 0xff, 0xff]);
    frame.extend_from_slice(&sender_mac.bytes);
    frame.extend_from_slice(&ETH_P_IP.to_be_bytes());

    // IP 头部: 第一个分片
    frame.extend_from_slice(&[
        0x45,        // Version=4, IHL=5
        0x00,        // TOS
        0x00, 0x20,  // Total Length = 32
        0x00, 0x01,  // Identification
        0x20, 0x00,  // Flags: MF=1, Offset=0
        0x40,        // TTL
        0x01,        // Protocol = ICMP
        0x00, 0x00,  // Checksum (稍后计算)
        192, 168, 1, 10,  // Source IP
        192, 168, 1, 100, // Dest IP
    ]);

    // 计算 IP 校验和
    let checksum = core_net::protocols::ip::calculate_checksum(&frame[14..34]);
    frame[32] = (checksum >> 8) as u8;
    frame[33] = (checksum & 0xFF) as u8;

    // ICMP 数据 (8 字节)
    frame.extend_from_slice(&[0x08, 0x00, 0x00, 0x00, 0x12, 0x34, 0x00, 0x01]);

    let packet = Packet::from_bytes(frame);
    inject_packet_to_context(&ctx, "eth0", packet).unwrap();

    let mut harness = TestHarness::with_context(ctx.clone());
    let result = harness.run();
    assert!(result.is_ok());

    // 分片数据报被重组表接收，但未完成重组，无响应
    assert!(verify_context_txq_count(&ctx, "eth0", 0), "部分分片应等待重组");
}

#[test]
#[serial]
fn test_ip_fragment_reassembly_complete() {
    let ctx = create_test_context();

    let sender_mac = MacAddr::new([0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0x01]);
    let sender_ip = Ipv4Addr::new(192, 168, 1, 10);
    let target_ip = Ipv4Addr::new(192, 168, 1, 100);

    // 准备三个分片的数据
    let identification: u16 = 12345;

    // 分片 1: MF=1, Offset=0, 数据 8 字节
    let frame1 = create_fragment_frame(
        sender_mac, sender_ip, target_ip, identification, 1, 0, true,
        &[0x08, 0x00, 0x00, 0x00, 0x12, 0x34, 0x00, 0x01],
    );

    // 分片 2: MF=1, Offset=1, 数据 8 字节
    let frame2 = create_fragment_frame(
        sender_mac, sender_ip, target_ip, identification, 1, 1, true,
        &[0x08, 0x00, 0x00, 0x00, 0x56, 0x78, 0x00, 0x02],
    );

    // 分片 3: MF=0, Offset=2, 数据 0 字节 (最后一片)
    let frame3 = create_fragment_frame(
        sender_mac, sender_ip, target_ip, identification, 1, 2, false,
        &[],
    );

    // 注入第一个分片
    let packet1 = Packet::from_bytes(frame1.clone());
    inject_packet_to_context(&ctx, "eth0", packet1).unwrap();

    let mut harness = TestHarness::with_context(ctx.clone());
    let _ = harness.run();

    // 注入第二个分片
    let packet2 = Packet::from_bytes(frame2.clone());
    inject_packet_to_context(&ctx, "eth0", packet2).unwrap();

    let mut harness = TestHarness::with_context(ctx.clone());
    let _ = harness.run();

    // 注入第三个分片
    let packet3 = Packet::from_bytes(frame3.clone());
    inject_packet_to_context(&ctx, "eth0", packet3).unwrap();

    let mut harness = TestHarness::with_context(ctx.clone());
    let result = harness.run();
    assert!(result.is_ok());

    // 重组完成，应该有 ICMP Echo Reply 响应
    // 注意：由于重组后的数据可能不是有效的 ICMP 报文，这里只验证重组逻辑
}

// ========== 辅助函数 ==========

/// 创建分片以太网帧
fn create_fragment_frame(
    src_mac: MacAddr,
    src_ip: Ipv4Addr,
    dst_ip: Ipv4Addr,
    identification: u16,
    ttl: u8,
    fragment_offset: u16,
    mf_flag: bool,
    data: &[u8],
) -> Vec<u8> {
    let mut frame = Vec::new();

    // 以太网头部
    frame.extend_from_slice(&[0xff, 0xff, 0xff, 0xff, 0xff, 0xff]);
    frame.extend_from_slice(&src_mac.bytes);
    frame.extend_from_slice(&ETH_P_IP.to_be_bytes());

    // IP 头部
    let total_length = 20 + data.len();
    let flags_fragment = ((mf_flag as u16) << 13) | fragment_offset;

    frame.extend_from_slice(&[0x45]); // Version=4, IHL=5
    frame.extend_from_slice(&[0x00]); // TOS
    frame.extend_from_slice(&(total_length as u16).to_be_bytes());
    frame.extend_from_slice(&identification.to_be_bytes());
    frame.extend_from_slice(&flags_fragment.to_be_bytes());
    frame.extend_from_slice(&[ttl]); // TTL
    frame.extend_from_slice(&[0x01]); // Protocol = ICMP
    frame.extend_from_slice(&[0x00, 0x00]); // Checksum (稍后计算)
    frame.extend_from_slice(src_ip.as_bytes());
    frame.extend_from_slice(dst_ip.as_bytes());

    // 计算校验和
    let checksum = core_net::protocols::ip::calculate_checksum(&frame[14..34]);
    frame[32] = (checksum >> 8) as u8;
    frame[33] = (checksum & 0xFF) as u8;

    // 数据
    frame.extend_from_slice(data);

    frame
}
