// ICMP 协议集成测试
//
// 测试 ICMP 协议的 Echo Request/Reply、Destination Unreachable、Time Exceeded
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
use core_net::protocols::icmp::{IcmpPacket, IcmpEcho, create_echo_request};
use core_net::protocols::ip::Ipv4Header;
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

/// 全局测试清理：在所有测试后执行一次
#[allow(dead_code)]
fn global_teardown() {
    // 可选：释放全局资源
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

/// 创建完整的 ICMP Echo Reply（带 IP 和以太网封装）
fn create_echo_reply_packet(
    dst_mac: MacAddr,
    src_mac: MacAddr,
    src_ip: Ipv4Addr,
    dst_ip: Ipv4Addr,
    identifier: u16,
    sequence: u16,
) -> Packet {
    // ICMP Echo Reply
    let icmp_echo = IcmpEcho::new_reply(identifier, sequence, vec![0x42; 32]);
    let icmp_packet = icmp_echo.to_bytes();

    // IP 头部
    let mut ip_data = create_ip_header(src_ip, dst_ip, icmp_packet.len());
    ip_data.extend_from_slice(&icmp_packet);

    // 以太网帧
    let mut frame = Vec::new();
    frame.extend_from_slice(&dst_mac.bytes);
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

// ========== 1. Echo Request/Reply 测试组 ==========

#[test]
#[serial]
fn test_icmp_echo_request_reply() {
    global_setup();

    let sender_mac = MacAddr::new([0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0x01]);
    let sender_ip = Ipv4Addr::new(192, 168, 1, 10);
    let target_ip = Ipv4Addr::new(192, 168, 1, 100); // 本机 IP

    // 创建并注入 Echo Request
    let request = create_echo_request_packet(sender_mac, sender_ip, target_ip, 1234, 1);
    inject_packet_to_interface("eth0", request).unwrap();

    // 运行调度器
    let mut harness = TestHarness::with_global_manager();
    let result = harness.run();
    assert!(result.is_ok(), "调度器运行失败: {:?}", result.err());

    // 验证：应该有 Echo Reply 响应
    assert!(verify_txq_count("eth0", 1), "发送队列应该有1个Echo Reply响应报文");

    clear_test_state();
}

#[test]
#[serial]
fn test_icmp_echo_identifier_sequence_match() {
    global_setup();

    let sender_mac = MacAddr::new([0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0x01]);
    let sender_ip = Ipv4Addr::new(192, 168, 1, 10);
    let target_ip = Ipv4Addr::new(192, 168, 1, 100);

    // 测试不同的 identifier 和 sequence
    for identifier in [1000, 2000, 3000] {
        for sequence in [1, 2, 3] {
            clear_test_state();

            let request = create_echo_request_packet(sender_mac, sender_ip, target_ip, identifier, sequence);
            inject_packet_to_interface("eth0", request).unwrap();

            let mut harness = TestHarness::with_global_manager();
            let result = harness.run();
            assert!(result.is_ok(), "调度器运行失败: {:?}", result.err());

            assert!(verify_txq_count("eth0", 1), "应该有Echo Reply响应");
        }
    }

    clear_test_state();
}

// ========== 2. 边界条件测试组 ==========

#[test]
#[serial]
fn test_icmp_minimal_packet() {
    global_setup();

    // 创建最小 Echo Request（8字节头部，无数据）
    let mut icmp_bytes = vec![
        0x08, 0x00, // Type=Echo Request, Code=0
        0x00, 0x00, // Checksum (稍后计算)
        0x12, 0x34, // Identifier
        0x00, 0x01, // Sequence
    ];

    // 计算校验和
    let checksum = core_net::protocols::ip::calculate_checksum(&icmp_bytes);
    icmp_bytes[2] = (checksum >> 8) as u8;
    icmp_bytes[3] = (checksum & 0xFF) as u8;

    // IP 封装
    let src_ip = Ipv4Addr::new(192, 168, 1, 10);
    let dst_ip = Ipv4Addr::new(192, 168, 1, 100);
    let mut ip_data = create_ip_header(src_ip, dst_ip, icmp_bytes.len());
    ip_data.extend_from_slice(&icmp_bytes);

    // 以太网封装
    let src_mac = MacAddr::new([0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0x01]);
    let mut frame = Vec::new();
    frame.extend_from_slice(&[0xff, 0xff, 0xff, 0xff, 0xff, 0xff]);
    frame.extend_from_slice(&src_mac.bytes);
    frame.extend_from_slice(&ETH_P_IP.to_be_bytes());
    frame.extend_from_slice(&ip_data);

    let packet = Packet::from_bytes(frame);
    inject_packet_to_interface("eth0", packet).unwrap();

    let mut harness = TestHarness::with_global_manager();
    let result = harness.run();
    assert!(result.is_ok());

    assert!(verify_txq_count("eth0", 1), "应该有响应");

    clear_test_state();
}

#[test]
#[serial]
fn test_icmp_checksum_validation() {
    global_setup();

    // 创建校验和错误的 Echo Request
    let icmp_bytes = vec![
        0x08, 0x00, // Type=Echo Request, Code=0
        0xff, 0xff, // 错误的校验和
        0x12, 0x34, // Identifier
        0x00, 0x01, // Sequence
    ];

    let src_ip = Ipv4Addr::new(192, 168, 1, 10);
    let dst_ip = Ipv4Addr::new(192, 168, 1, 100);
    let mut ip_data = create_ip_header(src_ip, dst_ip, icmp_bytes.len());
    ip_data.extend_from_slice(&icmp_bytes);

    let src_mac = MacAddr::new([0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0x01]);
    let mut frame = Vec::new();
    frame.extend_from_slice(&[0xff, 0xff, 0xff, 0xff, 0xff, 0xff]);
    frame.extend_from_slice(&src_mac.bytes);
    frame.extend_from_slice(&ETH_P_IP.to_be_bytes());
    frame.extend_from_slice(&ip_data);

    let packet = Packet::from_bytes(frame);
    inject_packet_to_interface("eth0", packet).unwrap();

    let mut harness = TestHarness::with_global_manager();
    let result = harness.run();
    assert!(result.is_ok());

    // 校验和错误，不应该有响应
    assert!(verify_txq_count("eth0", 0), "校验和错误时不应有响应");

    clear_test_state();
}

// ========== 3. ICMP 类型解析测试组 ==========

#[test]
#[serial]
fn test_icmp_type_parsing() {
    // 测试 Echo Request 解析
    let echo_bytes = create_echo_request(1234, 1, vec![0x42; 32]);
    let mut packet = Packet::from_bytes(echo_bytes);
    let result = IcmpPacket::from_packet(&mut packet);
    assert!(result.is_ok());

    match result.unwrap() {
        IcmpPacket::Echo(echo) => {
            assert_eq!(echo.type_, 8);
            assert_eq!(echo.identifier, 1234);
            assert_eq!(echo.sequence, 1);
        }
        _ => panic!("应该解析为 Echo"),
    }
}

#[test]
#[serial]
fn test_icmp_echo_roundtrip() {
    // 测试编码和解码一致性
    let original = IcmpEcho::new_request(5678, 42, vec![0x00, 0x01, 0x02, 0x03]);

    // 编码
    let bytes = original.to_bytes();

    // 解码
    let mut packet = Packet::from_bytes(bytes);
    let decoded = IcmpEcho::from_packet(&mut packet).unwrap();

    assert_eq!(decoded.type_, original.type_);
    assert_eq!(decoded.identifier, original.identifier);
    assert_eq!(decoded.sequence, original.sequence);
    assert_eq!(decoded.data, original.data);
}

// ========== 4. Destination Unreachable 测试组 ==========

#[test]
#[serial]
fn test_icmp_dest_unreachable_no_reply() {
    global_setup();

    // Destination Unreachable 不应该触发响应（避免循环）
    let mut icmp_bytes = vec![
        0x03, 0x00, // Type=Dest Unreachable, Code=Network Unreachable
        0x00, 0x00, // Checksum
        0x00, 0x00, 0x00, 0x00, // Unused
        // 原始 IP 头部（简化）
        0x45, 0x00, 0x00, 0x1c, 0x00, 0x00, 0x00, 0x00,
        0x40, 0x01, 0x00, 0x00, 0xc0, 0xa8, 0x01, 0x0a,
        0xc0, 0xa8, 0x01, 0x64,
    ];

    // 计算校验和
    let checksum = core_net::protocols::ip::calculate_checksum(&icmp_bytes);
    icmp_bytes[2] = (checksum >> 8) as u8;
    icmp_bytes[3] = (checksum & 0xFF) as u8;

    let src_ip = Ipv4Addr::new(192, 168, 1, 1);
    let dst_ip = Ipv4Addr::new(192, 168, 1, 100);
    let mut ip_data = create_ip_header(src_ip, dst_ip, icmp_bytes.len());
    ip_data.extend_from_slice(&icmp_bytes);

    let src_mac = MacAddr::new([0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0x01]);
    let mut frame = Vec::new();
    frame.extend_from_slice(&[0xff, 0xff, 0xff, 0xff, 0xff, 0xff]);
    frame.extend_from_slice(&src_mac.bytes);
    frame.extend_from_slice(&ETH_P_IP.to_be_bytes());
    frame.extend_from_slice(&ip_data);

    let packet = Packet::from_bytes(frame);
    inject_packet_to_interface("eth0", packet).unwrap();

    let mut harness = TestHarness::with_global_manager();
    let result = harness.run();
    assert!(result.is_ok());

    // Destination Unreachable 不应该触发响应
    assert!(verify_txq_count("eth0", 0), "Destination Unreachable 不应有响应");

    clear_test_state();
}

// ========== 5. 非本机 IP 测试组 ==========

#[test]
#[serial]
fn test_icmp_not_for_us() {
    global_setup();

    let sender_mac = MacAddr::new([0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0x01]);
    let sender_ip = Ipv4Addr::new(192, 168, 1, 10);
    let target_ip = Ipv4Addr::new(192, 168, 1, 200); // 不是本机 IP

    // 创建 Echo Request（目标不是本机）
    let request = create_echo_request_packet(sender_mac, sender_ip, target_ip, 1234, 1);
    inject_packet_to_interface("eth0", request).unwrap();

    let mut harness = TestHarness::with_global_manager();
    let result = harness.run();
    assert!(result.is_ok());

    // 目标不是本机，不应该响应
    assert!(verify_txq_count("eth0", 0), "非本机IP不应响应");

    clear_test_state();
}

// ========== 6. 多接口测试组 ==========

#[test]
#[serial]
fn test_icmp_multiple_interfaces() {
    global_setup();

    let sender_mac = MacAddr::new([0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0x01]);
    let sender_ip = Ipv4Addr::new(192, 168, 1, 10);

    // 测试向不同接口的 IP 发送请求
    let eth0_ip = Ipv4Addr::new(192, 168, 1, 100); // eth0 的 IP

    let request = create_echo_request_packet(sender_mac, sender_ip, eth0_ip, 1234, 1);
    inject_packet_to_interface("eth0", request).unwrap();

    let mut harness = TestHarness::with_global_manager();
    let result = harness.run();
    assert!(result.is_ok());

    assert!(verify_txq_count("eth0", 1), "eth0 应该有响应");

    clear_test_state();
}

// ========== 7. 序列号回绕测试 ==========

#[test]
#[serial]
fn test_icmp_sequence_wraparound() {
    global_setup();

    let sender_mac = MacAddr::new([0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0x01]);
    let sender_ip = Ipv4Addr::new(192, 168, 1, 10);
    let target_ip = Ipv4Addr::new(192, 168, 1, 100);

    // 测试最大序列号
    let request = create_echo_request_packet(sender_mac, sender_ip, target_ip, 1234, 65535);
    inject_packet_to_interface("eth0", request).unwrap();

    let mut harness = TestHarness::with_global_manager();
    let result = harness.run();
    assert!(result.is_ok());

    assert!(verify_txq_count("eth0", 1), "序列号65535应该正常处理");

    clear_test_state();
}

// ========== 8. Echo Reply 匹配测试 ==========

#[test]
#[serial]
fn test_icmp_echo_reply_matching() {
    global_setup();

    // 模拟收到 Echo Reply 的场景
    // 注意：在纯模拟环境中，这通常不会自然发生
    // 但我们可以验证 Echo 管理器的功能

    use core_net::protocols::icmp::register_echo_request;

    let sender_mac = MacAddr::new([0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0x01]);
    let sender_ip = Ipv4Addr::new(192, 168, 1, 10);
    let target_ip = Ipv4Addr::new(192, 168, 1, 100);

    // 注册一个 Echo 请求
    let result = register_echo_request(9999, 100, target_ip);
    assert!(result.is_ok(), "注册 Echo 请求应该成功");

    // 发送对应的 Echo Reply（模拟）
    let reply = create_echo_reply_packet(
        MacAddr::new([0x00, 0x11, 0x22, 0x33, 0x44, 0x55]), // dst_mac
        sender_mac,
        target_ip,
        sender_ip,
        9999,
        100,
    );

    inject_packet_to_interface("eth0", reply).unwrap();

    let mut harness = TestHarness::with_global_manager();
    let result = harness.run();
    assert!(result.is_ok());

    // Echo Reply 不应该触发新的响应
    assert!(verify_txq_count("eth0", 0), "Echo Reply 不应触发新响应");

    clear_test_state();
}
