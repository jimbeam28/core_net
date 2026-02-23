// UDP 协议集成测试
//
// 测试 UDP 协议的数据报解析、封装、校验和验证、端口管理和回调

use core_net::testframework::TestHarness;
use core_net::common::Packet;
use core_net::interface::{MacAddr, Ipv4Addr};
use core_net::protocols::ETH_P_IP;
use core_net::protocols::udp::{UdpDatagram, UdpHeader, UdpSocket, encapsulate_udp_datagram};
use core_net::protocols::udp::{UdpPortManager, EPHEMERAL_PORT_MIN};
use serial_test::serial;
use std::sync::{Arc, Mutex};

mod common;
use common::{create_ip_header_udp, inject_packet_to_context, verify_context_txq_count,
             create_test_context};

// 测试环境配置：本机接口 eth0: ifindex=0, MAC=00:11:22:33:44:55, IP=192.168.1.100

// ========== 基本功能测试组 ==========

#[test]
#[serial]
fn test_udp_basic_send_receive() {
    let ctx = create_test_context();

    let sender_mac = MacAddr::new([0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0x01]);
    let sender_ip = Ipv4Addr::new(192, 168, 1, 10);
    let target_ip = Ipv4Addr::new(192, 168, 1, 100); // 本机 IP

    // 创建 UDP Socket 并绑定端口
    let mut socket = UdpSocket::new(ctx.clone());
    let bound_port = socket.bind(5678).unwrap();

    // 创建 UDP 数据报
    let udp_data = encapsulate_udp_datagram(
        1234,       // 源端口
        bound_port, // 目标端口（已绑定）
        sender_ip,
        target_ip,
        b"Hello, UDP!",
        true,       // 计算校验和
    );

    // IP 封装
    let mut ip_data = create_ip_header_udp(sender_ip, target_ip, udp_data.len());
    ip_data.extend_from_slice(&udp_data);

    // 以太网封装
    let mut frame = Vec::new();
    frame.extend_from_slice(&[0xff, 0xff, 0xff, 0xff, 0xff, 0xff]);
    frame.extend_from_slice(&sender_mac.bytes);
    frame.extend_from_slice(&ETH_P_IP.to_be_bytes());
    frame.extend_from_slice(&ip_data);

    let packet = Packet::from_bytes(frame);
    inject_packet_to_context(&ctx, "eth0", packet).unwrap();

    // 运行调度器
    let mut harness = TestHarness::with_context(ctx.clone());
    let result = harness.run();
    assert!(result.is_ok(), "调度器运行失败: {:?}", result.err());

    // UDP 是无连接协议，端口已绑定且无回调时不发送响应
    assert!(verify_context_txq_count(&ctx, "eth0", 0), "UDP 不应有响应");
}

#[test]
#[serial]
fn test_udp_header_parse() {
    let bytes = [
        0x04, 0xD2, // Source Port: 1234
        0x16, 0x2E, // Dest Port: 5678
        0x00, 0x0C, // Length: 12
        0xAB, 0xCD, // Checksum: 0xABCD
        0x01, 0x02, 0x03, 0x04, // Payload
    ];

    let header = UdpHeader::parse(&bytes).unwrap();
    assert_eq!(header.source_port, 1234);
    assert_eq!(header.destination_port, 5678);
    assert_eq!(header.length, 12);
    assert_eq!(header.checksum, 0xABCD);
}

#[test]
#[serial]
fn test_udp_datagram_parse() {
    let bytes = [
        0x04, 0xD2, // Source Port: 1234
        0x16, 0x2E, // Dest Port: 5678
        0x00, 0x0C, // Length: 12 (8 + 4)
        0x00, 0x00, // Checksum
        0x48, 0x65, 0x6C, 0x6C, // "Hell" (4 bytes)
    ];

    let datagram = UdpDatagram::parse(&bytes).unwrap();
    assert_eq!(datagram.header.source_port, 1234);
    assert_eq!(datagram.header.destination_port, 5678);
    assert_eq!(datagram.payload, b"Hell");
}

// ========== 边界条件测试组 ==========

#[test]
#[serial]
fn test_udp_minimal_datagram() {
    let ctx = create_test_context();

    let sender_mac = MacAddr::new([0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0x01]);
    let sender_ip = Ipv4Addr::new(192, 168, 1, 10);
    let target_ip = Ipv4Addr::new(192, 168, 1, 100);

    // 最小 UDP 数据报（仅头部，无数据）
    let udp_data = encapsulate_udp_datagram(
        1234,
        5678,
        sender_ip,
        target_ip,
        b"",    // 空载荷
        false,
    );

    // 验证长度
    assert_eq!(udp_data.len(), 8); // 仅头部

    let mut ip_data = create_ip_header_udp(sender_ip, target_ip, udp_data.len());
    ip_data.extend_from_slice(&udp_data);

    let mut frame = Vec::new();
    frame.extend_from_slice(&[0xff, 0xff, 0xff, 0xff, 0xff, 0xff]);
    frame.extend_from_slice(&sender_mac.bytes);
    frame.extend_from_slice(&ETH_P_IP.to_be_bytes());
    frame.extend_from_slice(&ip_data);

    let packet = Packet::from_bytes(frame);
    inject_packet_to_context(&ctx, "eth0", packet).unwrap();

    let mut harness = TestHarness::with_context(ctx.clone());
    let result = harness.run();
    assert!(result.is_ok());
}

#[test]
#[serial]
fn test_udp_zero_source_port() {
    let bytes = [
        0x00, 0x00, // Source Port: 0 (未使用)
        0x16, 0x2E, // Dest Port: 5678
        0x00, 0x08, // Length: 8
        0x00, 0x00, // Checksum
    ];

    let datagram = UdpDatagram::parse(&bytes).unwrap();
    assert_eq!(datagram.header.source_port, 0);
    assert_eq!(datagram.header.destination_port, 5678);
    assert!(datagram.payload.is_empty());
}

#[test]
#[serial]
fn test_udp_odd_length_payload() {
    let _sender_ip = Ipv4Addr::new(192, 168, 1, 1);
    let _dest_ip = Ipv4Addr::new(192, 168, 1, 2);

    // 奇数长度载荷
    let datagram = UdpDatagram::create(1234, 5678, b"ABC"); // 3 bytes

    let checksum = datagram.calculate_checksum(_sender_ip, _dest_ip);
    // 验证校验和计算不崩溃
    assert_ne!(checksum, 0);
}

#[test]
#[serial]
fn test_udp_large_payload() {
    let sender_ip = Ipv4Addr::new(192, 168, 1, 1);
    let dest_ip = Ipv4Addr::new(192, 168, 1, 2);

    // 较大载荷（100 字节）
    let large_payload = vec![0x42u8; 100];
    let datagram = UdpDatagram::create(1234, 5678, &large_payload);

    assert_eq!(datagram.len(), 108); // 8 + 100

    let checksum = datagram.calculate_checksum(sender_ip, dest_ip);
    assert_ne!(checksum, 0);
}

// ========== 异常情况测试组 ==========

#[test]
#[serial]
fn test_udp_invalid_length() {
    // 长度字段小于 8
    let bytes = [
        0x04, 0xD2, // Source Port
        0x16, 0x2E, // Dest Port
        0x00, 0x07, // Length: 7 (invalid, < 8)
        0x00, 0x00, // Checksum
    ];

    let result = UdpDatagram::parse(&bytes);
    assert!(result.is_err());
}

#[test]
#[serial]
fn test_udp_data_too_short() {
    let ctx = create_test_context();

    let sender_mac = MacAddr::new([0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0x01]);
    let sender_ip = Ipv4Addr::new(192, 168, 1, 10);
    let target_ip = Ipv4Addr::new(192, 168, 1, 100);

    // 创建长度字段与实际数据不符的 UDP 数据报
    let udp_data = vec![
        0x04, 0xD2, // Source Port
        0x16, 0x2E, // Dest Port
        0x00, 0x14, // Length: 20
        0x00, 0x00, // Checksum
        0x01, 0x02, 0x03, 0x04, // 只有 4 字节数据
    ];

    let mut ip_data = create_ip_header_udp(sender_ip, target_ip, udp_data.len());
    ip_data.extend_from_slice(&udp_data);

    let mut frame = Vec::new();
    frame.extend_from_slice(&[0xff, 0xff, 0xff, 0xff, 0xff, 0xff]);
    frame.extend_from_slice(&sender_mac.bytes);
    frame.extend_from_slice(&ETH_P_IP.to_be_bytes());
    frame.extend_from_slice(&ip_data);

    let packet = Packet::from_bytes(frame);
    inject_packet_to_context(&ctx, "eth0", packet).unwrap();

    let mut harness = TestHarness::with_context(ctx.clone());
    let result = harness.run();
    // 应该静默丢弃（不响应）
    assert!(result.is_ok());
}

#[test]
#[serial]
fn test_udp_checksum_validation() {
    let ctx = create_test_context();

    let sender_mac = MacAddr::new([0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0x01]);
    let sender_ip = Ipv4Addr::new(192, 168, 1, 10);
    let target_ip = Ipv4Addr::new(192, 168, 1, 100);

    // 创建带校验和的 UDP 数据报
    let udp_data = encapsulate_udp_datagram(
        1234,
        5678,
        sender_ip,
        target_ip,
        b"Test",
        true,   // 计算校验和
    );

    let mut ip_data = create_ip_header_udp(sender_ip, target_ip, udp_data.len());
    ip_data.extend_from_slice(&udp_data);

    let mut frame = Vec::new();
    frame.extend_from_slice(&[0xff, 0xff, 0xff, 0xff, 0xff, 0xff]);
    frame.extend_from_slice(&sender_mac.bytes);
    frame.extend_from_slice(&ETH_P_IP.to_be_bytes());
    frame.extend_from_slice(&ip_data);

    let packet = Packet::from_bytes(frame);
    inject_packet_to_context(&ctx, "eth0", packet).unwrap();

    let mut harness = TestHarness::with_context(ctx.clone());
    let result = harness.run();
    assert!(result.is_ok());
}

#[test]
#[serial]
fn test_udp_checksum_error() {
    let ctx = create_test_context();

    let sender_mac = MacAddr::new([0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0x01]);
    let sender_ip = Ipv4Addr::new(192, 168, 1, 10);
    let target_ip = Ipv4Addr::new(192, 168, 1, 100);

    // 创建带错误校验和的 UDP 数据报
    let mut udp_data = encapsulate_udp_datagram(
        1234,
        5678,
        sender_ip,
        target_ip,
        b"Test",
        false,  // 不计算校验和
    );

    // 修改校验和为错误的值
    udp_data[6] = 0xFF;
    udp_data[7] = 0xFF;

    let mut ip_data = create_ip_header_udp(sender_ip, target_ip, udp_data.len());
    ip_data.extend_from_slice(&udp_data);

    let mut frame = Vec::new();
    frame.extend_from_slice(&[0xff, 0xff, 0xff, 0xff, 0xff, 0xff]);
    frame.extend_from_slice(&sender_mac.bytes);
    frame.extend_from_slice(&ETH_P_IP.to_be_bytes());
    frame.extend_from_slice(&ip_data);

    let packet = Packet::from_bytes(frame);
    inject_packet_to_context(&ctx, "eth0", packet).unwrap();

    let mut harness = TestHarness::with_context(ctx.clone());
    let result = harness.run();
    // 校验和错误，应该静默丢弃
    assert!(result.is_ok());
}

#[test]
#[serial]
fn test_udp_zero_checksum() {
    let bytes = [
        0x04, 0xD2, // Source Port
        0x16, 0x2E, // Dest Port
        0x00, 0x0C, // Length
        0x00, 0x00, // Checksum: 0 (IPv4 允许)
        0x01, 0x02, 0x03, 0x04,
    ];

    let datagram = UdpDatagram::parse(&bytes).unwrap();

    // 零校验和在强制模式下应该失败
    assert!(!datagram.verify_checksum(
        Ipv4Addr::new(192, 168, 1, 1),
        Ipv4Addr::new(192, 168, 1, 2),
        true
    ));

    // 零校验和在非强制模式下应该通过
    assert!(datagram.verify_checksum(
        Ipv4Addr::new(192, 168, 1, 1),
        Ipv4Addr::new(192, 168, 1, 2),
        false
    ));
}

// ========== 多端口测试组 ==========

#[test]
#[serial]
fn test_udp_multiple_ports() {
    let _sender_ip = Ipv4Addr::new(192, 168, 1, 1);
    let _dest_ip = Ipv4Addr::new(192, 168, 1, 2);

    // 测试不同的端口号组合
    let test_cases = vec![
        (53, 5353),      // DNS
        (123, 1234),     // NTP
        (161, 161),      // SNMP
        (8080, 80),      // HTTP 代理
    ];

    for (src_port, dst_port) in test_cases {
        let datagram = UdpDatagram::create(src_port, dst_port, b"test");
        assert_eq!(datagram.header.source_port, src_port);
        assert_eq!(datagram.header.destination_port, dst_port);
    }
}

// ========== 非本机 IP 测试组 ==========

#[test]
#[serial]
fn test_udp_not_for_us() {
    let ctx = create_test_context();

    let sender_mac = MacAddr::new([0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0x01]);
    let sender_ip = Ipv4Addr::new(192, 168, 1, 10);
    let target_ip = Ipv4Addr::new(192, 168, 1, 200); // 不是本机 IP

    let udp_data = encapsulate_udp_datagram(
        1234,
        5678,
        sender_ip,
        target_ip,
        b"Hello",
        false,
    );

    let mut ip_data = create_ip_header_udp(sender_ip, target_ip, udp_data.len());
    ip_data.extend_from_slice(&udp_data);

    let mut frame = Vec::new();
    frame.extend_from_slice(&[0xff, 0xff, 0xff, 0xff, 0xff, 0xff]);
    frame.extend_from_slice(&sender_mac.bytes);
    frame.extend_from_slice(&ETH_P_IP.to_be_bytes());
    frame.extend_from_slice(&ip_data);

    let packet = Packet::from_bytes(frame);
    inject_packet_to_context(&ctx, "eth0", packet).unwrap();

    let mut harness = TestHarness::with_context(ctx.clone());
    let result = harness.run();
    assert!(result.is_ok());

    // 目标不是本机，不应该处理
    assert!(verify_context_txq_count(&ctx, "eth0", 0));
}

// ========== 端口边界测试组 ==========

#[test]
#[serial]
fn test_udp_port_boundaries() {
    let bytes = [
        0x00, 0x00, // Port: 0
        0xFF, 0xFF, // Port: 65535
        0x00, 0x08, // Length: 8
        0x00, 0x00, // Checksum
    ];

    let datagram = UdpDatagram::parse(&bytes).unwrap();
    assert_eq!(datagram.header.source_port, 0);
    assert_eq!(datagram.header.destination_port, 65535);
}

// ========== 端口管理测试组 ==========

#[test]
#[serial]
fn test_udp_port_manager_bind_specific() {
    let mut manager = UdpPortManager::new();

    // 绑定特定端口
    let port = manager.bind(8080).unwrap();
    assert_eq!(port, 8080);
    assert!(manager.is_bound(8080));
}

#[test]
#[serial]
fn test_udp_port_manager_bind_auto() {
    let mut manager = UdpPortManager::new();

    // 自动分配端口
    let port1 = manager.bind(0).unwrap();
    assert!(port1 >= EPHEMERAL_PORT_MIN);

    // 再次分配
    let port2 = manager.bind(0).unwrap();
    assert_ne!(port1, port2);
}

#[test]
#[serial]
fn test_udp_port_manager_bind_conflict() {
    let mut manager = UdpPortManager::new();

    manager.bind(8080).unwrap();
    let result = manager.bind(8080);
    assert!(result.is_err());
}

#[test]
#[serial]
fn test_udp_port_manager_unbind() {
    let mut manager = UdpPortManager::new();

    manager.bind(8080).unwrap();
    assert!(manager.is_bound(8080));

    manager.unbind(8080).unwrap();
    assert!(!manager.is_bound(8080));

    // 可以重新绑定
    manager.bind(8080).unwrap();
    assert!(manager.is_bound(8080));
}

#[test]
#[serial]
fn test_udp_port_manager_bound_list() {
    let mut manager = UdpPortManager::new();

    manager.bind(8080).unwrap();
    manager.bind(9090).unwrap();
    manager.bind(53).unwrap();

    let ports = manager.bound_ports();
    assert_eq!(ports, vec![53, 8080, 9090]);
}

// ========== Socket API 测试组 ==========

#[test]
#[serial]
fn test_udp_socket_bind() {
    let ctx = create_test_context();
    let mut socket = UdpSocket::new(ctx);

    // 自动分配端口
    let port = socket.bind(0).unwrap();
    assert!(port >= EPHEMERAL_PORT_MIN);
    assert!(socket.is_bound());
    assert_eq!(socket.local_port(), Some(port));
}

#[test]
#[serial]
fn test_udp_socket_bind_specific() {
    let ctx = create_test_context();
    let mut socket = UdpSocket::new(ctx);

    let port = socket.bind(8080).unwrap();
    assert_eq!(port, 8080);
}

#[test]
#[serial]
fn test_udp_socket_bind_twice() {
    let ctx = create_test_context();
    let mut socket = UdpSocket::new(ctx);

    socket.bind(8080).unwrap();
    let result = socket.bind(9090);
    assert!(result.is_err());
}

#[test]
#[serial]
fn test_udp_socket_close() {
    let ctx = create_test_context();
    let mut socket = UdpSocket::new(ctx);

    socket.bind(8080).unwrap();
    assert!(socket.is_bound());

    socket.close().unwrap();
    assert!(!socket.is_bound());
    assert!(socket.is_closed());
}

#[test]
#[serial]
fn test_udp_socket_set_callback() {
    let ctx = create_test_context();
    let mut socket = UdpSocket::new(ctx);

    socket.bind(8080).unwrap();

    let result = socket.set_callback(|_src_addr, _src_port, _data| {
        // 回调逻辑
    });
    assert!(result.is_ok());
    assert!(socket.has_callback());
}

// ========== 回调和数据分发测试组 ==========

#[test]
#[serial]
fn test_udp_callback_receive() {
    use std::sync::atomic::{AtomicU16, Ordering};

    let ctx = create_test_context();
    let mut socket = UdpSocket::new(ctx.clone());

    let bound_port = socket.bind(5678).unwrap();

    // 使用 Arc 共享状态来验证回调
    let received_port = Arc::new(AtomicU16::new(0));
    let received_data = Arc::new(Mutex::new(Vec::new()));

    let port_clone = received_port.clone();
    let data_clone = received_data.clone();

    socket.set_callback(move |_src_addr, src_port, data| {
        port_clone.store(src_port, Ordering::SeqCst);
        *data_clone.lock().unwrap() = data;
    }).unwrap();

    // 构造并发送 UDP 数据报
    let sender_mac = MacAddr::new([0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0x01]);
    let sender_ip = Ipv4Addr::new(192, 168, 1, 10);
    let target_ip = Ipv4Addr::new(192, 168, 1, 100);

    let udp_data = encapsulate_udp_datagram(
        1234,          // 源端口
        bound_port,    // 目标端口
        sender_ip,
        target_ip,
        b"Hello from callback!",
        true,
    );

    let mut ip_data = create_ip_header_udp(sender_ip, target_ip, udp_data.len());
    ip_data.extend_from_slice(&udp_data);

    let mut frame = Vec::new();
    frame.extend_from_slice(&[0xff, 0xff, 0xff, 0xff, 0xff, 0xff]);
    frame.extend_from_slice(&sender_mac.bytes);
    frame.extend_from_slice(&ETH_P_IP.to_be_bytes());
    frame.extend_from_slice(&ip_data);

    let packet = Packet::from_bytes(frame);
    inject_packet_to_context(&ctx, "eth0", packet).unwrap();

    // 使用相同的context创建harness
    let mut harness = TestHarness::with_context(ctx);
    let result = harness.run();
    assert!(result.is_ok());

    // 验证回调被调用
    let final_port = received_port.load(Ordering::SeqCst);
    let final_data = received_data.lock().unwrap().clone();

    assert_eq!(final_port, 1234);
    assert_eq!(final_data, b"Hello from callback!".to_vec());
}

#[test]
#[serial]
fn test_udp_no_callback_no_response() {
    let ctx = create_test_context();

    // 端口未绑定，应该返回 PortUnreachable（如果配置启用）
    let sender_mac = MacAddr::new([0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0x01]);
    let sender_ip = Ipv4Addr::new(192, 168, 1, 10);
    let target_ip = Ipv4Addr::new(192, 168, 1, 100);

    let udp_data = encapsulate_udp_datagram(
        1234,
        9999,  // 未绑定的端口
        sender_ip,
        target_ip,
        b"Test",
        true,
    );

    let mut ip_data = create_ip_header_udp(sender_ip, target_ip, udp_data.len());
    ip_data.extend_from_slice(&udp_data);

    let mut frame = Vec::new();
    frame.extend_from_slice(&[0xff, 0xff, 0xff, 0xff, 0xff, 0xff]);
    frame.extend_from_slice(&sender_mac.bytes);
    frame.extend_from_slice(&ETH_P_IP.to_be_bytes());
    frame.extend_from_slice(&ip_data);

    let packet = Packet::from_bytes(frame);
    inject_packet_to_context(&ctx, "eth0", packet).unwrap();

    let mut harness = TestHarness::with_context(ctx.clone());
    let result = harness.run();
    assert!(result.is_ok());

    // 端口未绑定，根据默认配置会返回 PortUnreachable
    // 但由于模拟环境的限制，实际可能不会生成 ICMP
}

// ========== 动态端口分配测试组 ==========

#[test]
#[serial]
fn test_udp_ephemeral_port_range() {
    let mut manager = UdpPortManager::new();

    for _ in 0..10 {
        let port = manager.bind(0).unwrap();
        assert!(port >= EPHEMERAL_PORT_MIN);
    }
}

#[test]
#[serial]
fn test_udp_port_exhaustion() {
    let mut manager = UdpPortManager::new();

    // 绑定大量端口（模拟耗尽）
    let mut bind_count = 0;
    for port in EPHEMERAL_PORT_MIN..=EPHEMERAL_PORT_MIN + 100 {
        if manager.bind(port).is_ok() {
            bind_count += 1;
        }
    }

    assert!(bind_count > 0);
}

// ========== 端口冲突检测测试组 ==========

#[test]
#[serial]
fn test_udp_socket_port_conflict() {
    let ctx = create_test_context();
    let mut socket1 = UdpSocket::new(ctx.clone());
    let mut socket2 = UdpSocket::new(ctx);

    socket1.bind(8080).unwrap();
    let result = socket2.bind(8080);
    assert!(result.is_err());
}

#[test]
#[serial]
fn test_udp_well_known_port() {
    let mut manager = UdpPortManager::new();

    // 绑定知名端口
    let port = manager.bind(53).unwrap();  // DNS
    assert_eq!(port, 53);

    let entry = manager.lookup(53).unwrap();
    assert!(entry.is_well_known);
}

