// IPv4 协议集成测试
//
// 测试 IPv4 协议的头部解析、分片检测、地址类型判断等
//
// 设计原则：
// 1. 在本文件实现创建报文和校验本地资源/响应报文
// 2. 在所有用例执行前初始化全局资源，在所有用例执行后释放全局资源
// 3. 在每个用例执行后清空全局资源
// 4. 报文的测试用例序列化执行，使用 serial_test 确保串行

use core_net::testframework::{
    TestHarness, HarnessError, HarnessResult, GlobalStateManager,
};
use core_net::interface::{MacAddr, Ipv4Addr};
use core_net::protocols::{ETH_P_IP, IP_PROTO_ICMP};
use core_net::protocols::ip::{Ipv4Header, encapsulate_ip_datagram};
use core_net::protocols::icmp::create_echo_request;
use core_net::common::Packet;

// 使用 serial_test 确保测试串行执行
use serial_test::serial;

// ========== 测试环境配置 ==========
//
// 本测试使用与 src/interface/interface.toml 一致的配置
// 本机接口配置：
// - eth0: ifindex=0, MAC=00:11:22:33:44:55, IP=192.168.1.100

// ========== 全局测试生命周期管理 ==========

/// 全局测试设置：在所有测试前执行一次
fn global_setup() {
    GlobalStateManager::setup_global_state().expect("全局状态初始化失败");
}

/// 每个测试后的清理函数
fn clear_test_state() {
    GlobalStateManager::clear_global_state().expect("全局状态清理失败");
}

// ========== 报文创建辅助函数 ==========

/// 创建 IP 头部
fn create_ip_header(src_ip: Ipv4Addr, dst_ip: Ipv4Addr, payload_len: usize) -> Vec<u8> {
    let ip_header = Ipv4Header::new(src_ip, dst_ip, IP_PROTO_ICMP, payload_len);
    ip_header.to_bytes()
}

/// 创建 Echo Request 报文（带 IP 封装）
///
/// # 参数
/// - src_mac: 源MAC地址
/// - src_ip: 源IP地址
/// - dst_ip: 目标IP地址
/// - identifier: Echo 标识符
/// - sequence: Echo 序列号
///
/// # 返回
/// 完整的以太网帧（包含 IP 和 ICMP Echo Request）
fn create_echo_request_packet(
    src_mac: MacAddr,
    src_ip: Ipv4Addr,
    dst_ip: Ipv4Addr,
    identifier: u16,
    sequence: u16,
) -> Packet {
    // ICMP Echo Request
    let icmp_data = vec![0x42; 32]; // 测试数据
    let icmp_packet = create_echo_request(identifier, sequence, icmp_data);

    // IP 头部
    let mut ip_data = create_ip_header(src_ip, dst_ip, icmp_packet.len());
    ip_data.extend_from_slice(&icmp_packet);

    // 以太网帧
    let mut frame = Vec::new();
    frame.extend_from_slice(&[0xff, 0xff, 0xff, 0xff, 0xff, 0xff]); // 广播 MAC
    frame.extend_from_slice(&src_mac.bytes);
    frame.extend_from_slice(&ETH_P_IP.to_be_bytes());
    frame.extend_from_slice(&ip_data);

    Packet::from_bytes(frame)
}

/// 注入报文到全局接口的 RxQ
fn inject_packet_to_interface(iface_name: &str, packet: Packet) -> HarnessResult<()> {
    let mut guard = GlobalStateManager::get_or_recover_interface_lock();
    let iface = guard.get_by_name_mut(iface_name)?;
    iface.rxq.enqueue(packet).map_err(|e| HarnessError::QueueError(format!("{:?}", e)))?;
    Ok(())
}

/// 验证 TxQ 中的报文数量
fn verify_txq_count(iface_name: &str, expected: usize) -> bool {
    let guard = GlobalStateManager::get_or_recover_interface_lock();
    guard.get_by_name(iface_name)
        .map(|iface| iface.txq.len() == expected)
        .unwrap_or(false)
}

// ========== 1. IP 头部解析测试组 ==========

#[test]
#[serial]
fn test_ip_header_parse() {
    global_setup();

    let src_ip = Ipv4Addr::new(192, 168, 1, 10);
    let dst_ip = Ipv4Addr::new(192, 168, 1, 100);
    let ip_header = Ipv4Header::new(src_ip, dst_ip, IP_PROTO_ICMP, 64);

    assert_eq!(ip_header.version(), 4);
    assert_eq!(ip_header.header_len(), 20);
    assert_eq!(ip_header.protocol, IP_PROTO_ICMP);
    assert_eq!(ip_header.source_addr, src_ip);
    assert_eq!(ip_header.dest_addr, dst_ip);

    clear_test_state();
}

#[test]
#[serial]
fn test_ip_header_flags_fragment() {
    global_setup();

    // 测试默认创建的头部（DF=1, MF=0, Offset=0）
    let header = Ipv4Header::new(
        Ipv4Addr::new(192, 168, 1, 1),
        Ipv4Addr::new(192, 168, 1, 2),
        IP_PROTO_ICMP,
        64,
    );

    assert!(header.has_df_flag());
    assert!(!header.has_mf_flag());
    assert_eq!(header.fragment_offset(), 0);
    assert!(!header.is_fragmented());

    clear_test_state();
}

// ========== 2. 分片检测测试组 ==========

#[test]
#[serial]
fn test_ip_fragment_rejection_mf_flag() {
    global_setup();

    let sender_mac = MacAddr::new([0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0x01]);
    let sender_ip = Ipv4Addr::new(192, 168, 1, 10);
    let target_ip = Ipv4Addr::new(192, 168, 1, 100);

    // 创建带 MF=1 标志的分片数据报
    let mut frame = Vec::new();
    frame.extend_from_slice(&[0xff, 0xff, 0xff, 0xff, 0xff, 0xff]);
    frame.extend_from_slice(&sender_mac.bytes);
    frame.extend_from_slice(&ETH_P_IP.to_be_bytes());

    // IP 头部: MF=1
    frame.extend_from_slice(&[
        0x45,        // Version=4, IHL=5
        0x00,        // TOS
        0x00, 0x1c,  // Total Length = 28
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

    // ICMP 数据
    frame.extend_from_slice(&[0x08, 0x00, 0x00, 0x00, 0x12, 0x34, 0x00, 0x01]);

    let packet = Packet::from_bytes(frame);
    inject_packet_to_interface("eth0", packet).unwrap();

    let mut harness = TestHarness::with_global_manager();
    let result = harness.run();
    assert!(result.is_ok());

    // 分片数据报应该被丢弃，无响应
    assert!(verify_txq_count("eth0", 0), "分片数据报应被丢弃");

    clear_test_state();
}

#[test]
#[serial]
fn test_ip_fragment_rejection_offset_nonzero() {
    global_setup();

    let sender_mac = MacAddr::new([0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0x01]);
    let sender_ip = Ipv4Addr::new(192, 168, 1, 10);
    let target_ip = Ipv4Addr::new(192, 168, 1, 100);

    // 创建带非零偏移的分片数据报
    let mut frame = Vec::new();
    frame.extend_from_slice(&[0xff, 0xff, 0xff, 0xff, 0xff, 0xff]);
    frame.extend_from_slice(&sender_mac.bytes);
    frame.extend_from_slice(&ETH_P_IP.to_be_bytes());

    // IP 头部: Fragment Offset = 185
    frame.extend_from_slice(&[
        0x45,        // Version=4, IHL=5
        0x00,        // TOS
        0x00, 0x1c,  // Total Length = 28
        0x00, 0x01,  // Identification
        0x00, 0xb9,  // Flags: MF=0, Offset=185 (0x00B9)
        0x40,        // TTL
        0x01,        // Protocol = ICMP
        0x00, 0x00,  // Checksum
        192, 168, 1, 10,  // Source IP
        192, 168, 1, 100, // Dest IP
    ]);

    // 计算 IP 校验和
    let checksum = core_net::protocols::ip::calculate_checksum(&frame[14..34]);
    frame[32] = (checksum >> 8) as u8;
    frame[33] = (checksum & 0xFF) as u8;

    // ICMP 数据
    frame.extend_from_slice(&[0x08, 0x00, 0x00, 0x00, 0x12, 0x34, 0x00, 0x01]);

    let packet = Packet::from_bytes(frame);
    inject_packet_to_interface("eth0", packet).unwrap();

    let mut harness = TestHarness::with_global_manager();
    let result = harness.run();
    assert!(result.is_ok());

    // 分片数据报应该被丢弃
    assert!(verify_txq_count("eth0", 0), "分片数据报应被丢弃");

    clear_test_state();
}

// ========== 3. 边界条件测试组 ==========

#[test]
#[serial]
fn test_ip_min_header_length() {
    global_setup();

    // 创建最小 IP 头部（IHL=5, 20字节）
    let header = Ipv4Header::new(
        Ipv4Addr::new(192, 168, 1, 1),
        Ipv4Addr::new(192, 168, 1, 2),
        IP_PROTO_ICMP,
        0,
    );

    assert_eq!(header.ihl(), 5);
    assert_eq!(header.header_len(), 20);

    clear_test_state();
}

#[test]
#[serial]
fn test_ip_max_packet_length() {
    global_setup();

    // 测试最大数据报长度（65535字节）
    let max_payload = 65535 - 20; // 减去 IP 头部
    let large_payload = vec![0x42; max_payload];

    // 创建封装函数测试
    let src_ip = Ipv4Addr::new(192, 168, 1, 1);
    let dst_ip = Ipv4Addr::new(192, 168, 1, 2);
    let packet = encapsulate_ip_datagram(src_ip, dst_ip, IP_PROTO_ICMP, &large_payload);

    // 验证总长度字段
    let total_len = u16::from_be_bytes([packet[2], packet[3]]);
    assert_eq!(total_len, 65535);

    clear_test_state();
}

// ========== 4. 地址类型测试组 ==========

#[test]
#[serial]
fn test_ip_broadcast_address() {
    global_setup();

    let header = Ipv4Header::new(
        Ipv4Addr::new(192, 168, 1, 1),
        Ipv4Addr::broadcast(),
        IP_PROTO_ICMP,
        64,
    );

    assert!(header.is_broadcast());
    assert!(!header.is_loopback());
    assert!(!header.is_multicast());

    clear_test_state();
}

#[test]
#[serial]
fn test_ip_loopback_address() {
    global_setup();

    let header = Ipv4Header::new(
        Ipv4Addr::new(192, 168, 1, 1),
        Ipv4Addr::localhost(), // 127.0.0.1
        IP_PROTO_ICMP,
        64,
    );

    assert!(!header.is_broadcast());
    assert!(header.is_loopback());
    assert!(!header.is_multicast());

    clear_test_state();
}

#[test]
#[serial]
fn test_ip_multicast_address() {
    global_setup();

    // 组播地址范围: 224.0.0.0/4
    let header = Ipv4Header::new(
        Ipv4Addr::new(192, 168, 1, 1),
        Ipv4Addr::new(224, 0, 0, 1),
        IP_PROTO_ICMP,
        64,
    );

    assert!(!header.is_broadcast());
    assert!(!header.is_loopback());
    assert!(header.is_multicast());

    clear_test_state();
}

// ========== 5. 正常 IP-ICMP 流程测试 ==========

#[test]
#[serial]
fn test_ip_icmp_normal_flow() {
    global_setup();

    let sender_mac = MacAddr::new([0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0x01]);
    let sender_ip = Ipv4Addr::new(192, 168, 1, 10);
    let target_ip = Ipv4Addr::new(192, 168, 1, 100); // 本机 IP

    // 创建正常的 Echo Request
    let request = create_echo_request_packet(sender_mac, sender_ip, target_ip, 1234, 1);
    inject_packet_to_interface("eth0", request).unwrap();

    let mut harness = TestHarness::with_global_manager();
    let result = harness.run();
    assert!(result.is_ok());

    // 应该有 Echo Reply 响应
    assert!(verify_txq_count("eth0", 1), "应该有Echo Reply响应");

    clear_test_state();
}

// ========== 6. 协议不支持测试 ==========

#[test]
#[serial]
fn test_ip_protocol_unsupported_tcp() {
    global_setup();

    let sender_mac = MacAddr::new([0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0x01]);
    let sender_ip = Ipv4Addr::new(192, 168, 1, 10);
    let target_ip = Ipv4Addr::new(192, 168, 1, 100);

    // 创建 TCP 协议的 IP 数据报
    let mut frame = Vec::new();
    frame.extend_from_slice(&[0xff, 0xff, 0xff, 0xff, 0xff, 0xff]);
    frame.extend_from_slice(&sender_mac.bytes);
    frame.extend_from_slice(&ETH_P_IP.to_be_bytes());

    // IP 头部: Protocol=TCP (6)
    frame.extend_from_slice(&[
        0x45,        // Version=4, IHL=5
        0x00,        // TOS
        0x00, 0x28,  // Total Length = 40
        0x00, 0x01,  // Identification
        0x40, 0x00,  // Flags: DF=1, Offset=0
        0x40,        // TTL
        0x06,        // Protocol = TCP
        0x00, 0x00,  // Checksum
        192, 168, 1, 10,  // Source IP
        192, 168, 1, 100, // Dest IP
    ]);

    // 计算 IP 校验和
    let checksum = core_net::protocols::ip::calculate_checksum(&frame[14..34]);
    frame[32] = (checksum >> 8) as u8;
    frame[33] = (checksum & 0xFF) as u8;

    // TCP 头部（简化）
    frame.extend_from_slice(&[0x00, 0x14, 0x00, 0x50, 0x00, 0x00, 0x00, 0x01]);

    let packet = Packet::from_bytes(frame);
    inject_packet_to_interface("eth0", packet).unwrap();

    let mut harness = TestHarness::with_global_manager();
    let result = harness.run();
    // TCP 不支持，但调度器应该能处理这个错误
    assert!(result.is_ok() || result.is_err());

    clear_test_state();
}

#[test]
#[serial]
fn test_ip_protocol_unsupported_udp() {
    global_setup();

    let sender_mac = MacAddr::new([0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0x01]);
    let sender_ip = Ipv4Addr::new(192, 168, 1, 10);
    let target_ip = Ipv4Addr::new(192, 168, 1, 100);

    // 创建 UDP 协议的 IP 数据报
    let mut frame = Vec::new();
    frame.extend_from_slice(&[0xff, 0xff, 0xff, 0xff, 0xff, 0xff]);
    frame.extend_from_slice(&sender_mac.bytes);
    frame.extend_from_slice(&ETH_P_IP.to_be_bytes());

    // IP 头部: Protocol=UDP (17)
    frame.extend_from_slice(&[
        0x45,        // Version=4, IHL=5
        0x00,        // TOS
        0x00, 0x1c,  // Total Length = 28
        0x00, 0x01,  // Identification
        0x40, 0x00,  // Flags: DF=1, Offset=0
        0x40,        // TTL
        0x11,        // Protocol = UDP
        0x00, 0x00,  // Checksum
        192, 168, 1, 10,  // Source IP
        192, 168, 1, 100, // Dest IP
    ]);

    // 计算 IP 校验和
    let checksum = core_net::protocols::ip::calculate_checksum(&frame[14..34]);
    frame[32] = (checksum >> 8) as u8;
    frame[33] = (checksum & 0xFF) as u8;

    // UDP 头部（简化）
    frame.extend_from_slice(&[0x00, 0x08, 0x00, 0x08, 0x00, 0x00, 0x00, 0x00]);

    let packet = Packet::from_bytes(frame);
    inject_packet_to_interface("eth0", packet).unwrap();

    let mut harness = TestHarness::with_global_manager();
    let result = harness.run();
    // UDP 不支持，但调度器应该能处理这个错误
    assert!(result.is_ok() || result.is_err());

    clear_test_state();
}

// ========== 7. 封装测试 ==========

#[test]
#[serial]
fn test_ip_encapsulate_datagram() {
    global_setup();

    let src_ip = Ipv4Addr::new(192, 168, 1, 1);
    let dst_ip = Ipv4Addr::new(192, 168, 1, 2);
    let payload = vec![0x08, 0x00, 0xf7, 0xfc, 0x12, 0x34, 0x00, 0x01];

    let packet = encapsulate_ip_datagram(src_ip, dst_ip, IP_PROTO_ICMP, &payload);

    // 验证包头
    assert_eq!(packet[0], 0x45); // Version=4, IHL=5
    assert_eq!(packet[9], IP_PROTO_ICMP);

    // 验证地址
    assert_eq!(&packet[12..16], &[192, 168, 1, 1]);
    assert_eq!(&packet[16..20], &[192, 168, 1, 2]);

    // 验证负载
    assert_eq!(&packet[20..], &payload[..]);

    clear_test_state();
}
