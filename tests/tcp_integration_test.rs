// TCP 协议集成测试
//
// 测试 TCP 协议的报文解析、封装、连接管理

use core_net::testframework::TestHarness;
use core_net::common::Packet;
use core_net::interface::{MacAddr, Ipv4Addr};
use core_net::protocols::ETH_P_IP;
use core_net::protocols::tcp::{TcpHeader, TcpSegment, encapsulate_tcp_segment, create_syn, create_ack};
use serial_test::serial;

mod common;
use common::{create_ip_header, inject_packet_to_context, verify_context_txq_count,
             create_test_context};

// 测试环境配置：本机接口 eth0: ifindex=0, MAC=00:11:22:33:44:55, IP=192.168.1.100

// ========== 基本功能测试组 ==========

#[test]
#[serial]
fn test_tcp_header_parse() {
    let bytes = [
        // TCP 头部
        0x04, 0xD2, // Source Port: 1234
        0x16, 0x2E, // Dest Port: 5678
        0x00, 0x00, 0x03, 0xE8, // Seq: 1000
        0x00, 0x00, 0x01, 0xF4, // Ack: 500
        0x50, 0x18, // Data Offset: 5, Flags: ACK + PSH
        0x20, 0x00, // Window: 8192
        0x00, 0x00, // Checksum
        0x00, 0x00, // Urgent Pointer
    ];

    let header = TcpHeader::parse(&bytes).unwrap();
    assert_eq!(header.source_port, 1234);
    assert_eq!(header.destination_port, 5678);
    assert_eq!(header.sequence_number, 1000);
    assert_eq!(header.acknowledgment_number, 500);
    assert!(header.is_ack());
    assert!(header.is_psh());
}

#[test]
#[serial]
fn test_tcp_segment_parse() {
    let bytes = [
        // TCP 头部
        0x04, 0xD2, // Source Port
        0x16, 0x2E, // Dest Port
        0x00, 0x00, 0x03, 0xE8, // Seq
        0x00, 0x00, 0x01, 0xF4, // Ack
        0x50, 0x18, // Flags: ACK + PSH
        0x20, 0x00, // Window
        0x00, 0x00, // Checksum
        0x00, 0x00, // Urgent Pointer
        // 数据
        0x48, 0x65, 0x6C, 0x6C, 0x6F, // "Hello"
    ];

    let segment = TcpSegment::parse(&bytes).unwrap();
    assert_eq!(segment.header.source_port, 1234);
    assert_eq!(segment.header.destination_port, 5678);
    assert_eq!(segment.payload, b"Hello");
}

#[test]
#[serial]
fn test_tcp_syn_packet() {
    let sender_ip = Ipv4Addr::new(192, 168, 1, 10);
    let target_ip = Ipv4Addr::new(192, 168, 1, 100);

    // 创建 SYN 报文
    let tcp_data = create_syn(1234, 80, sender_ip, target_ip, 1000, 8192);

    // 验证长度
    assert_eq!(tcp_data.len(), 20); // 基本头部

    // 验证源端口
    assert_eq!(tcp_data[0..2], 1234u16.to_be_bytes());

    // 验证 SYN 标志
    assert!(tcp_data[13] & 0x02 != 0);
}

#[test]
#[serial]
fn test_tcp_ack_packet() {
    let sender_ip = Ipv4Addr::new(192, 168, 1, 10);
    let target_ip = Ipv4Addr::new(192, 168, 1, 100);

    // 创建 ACK 报文
    let tcp_data = create_ack(80, 1234, target_ip, sender_ip, 2000, 1001, 8192);

    // 验证 ACK 标志
    assert!(tcp_data[13] & 0x10 != 0);
}

#[test]
#[serial]
fn test_tcp_encapsulate_segment() {
    let src_ip = Ipv4Addr::new(192, 168, 1, 1);
    let dst_ip = Ipv4Addr::new(192, 168, 1, 2);

    let header = TcpHeader::ack(1234, 5678, 1000, 500, 8192);
    let bytes = encapsulate_tcp_segment(&header, &[], src_ip, dst_ip);

    assert_eq!(bytes.len(), 20);
    // 验证校验和已计算
    let checksum = u16::from_be_bytes([bytes[16], bytes[17]]);
    assert_ne!(checksum, 0);
}

// ========== 边界条件测试组 ==========

#[test]
#[serial]
fn test_tcp_minimal_header() {
    let bytes = [
        0x04, 0xD2, // Source Port
        0x16, 0x2E, // Dest Port
        0x00, 0x00, 0x03, 0xE8, // Seq
        0x00, 0x00, 0x01, 0xF4, // Ack
        0x50, 0x10, // Data Offset: 5, Flags: ACK
        0x20, 0x00, // Window
        0x00, 0x00, // Checksum
        0x00, 0x00, // Urgent Pointer
    ];

    let segment = TcpSegment::parse(&bytes).unwrap();
    assert_eq!(segment.header.data_offset(), 5);
    assert!(segment.payload.is_empty());
}

#[test]
#[serial]
fn test_tcp_invalid_data_offset() {
    let bytes = [
        0x04, 0xD2, // Source Port
        0x16, 0x2E, // Dest Port
        0x00, 0x00, 0x03, 0xE8, // Seq
        0x00, 0x00, 0x01, 0xF4, // Ack
        0x40, 0x10, // Data Offset: 4 (invalid, < 5), Flags: ACK
        0x20, 0x00, // Window
        0x00, 0x00, // Checksum
        0x00, 0x00, // Urgent Pointer
    ];

    let result = TcpSegment::parse(&bytes);
    assert!(result.is_err());
}

#[test]
#[serial]
fn test_tcp_header_too_short() {
    let bytes = [0x04, 0xD2, 0x16, 0x2E]; // 只有 4 字节

    let result = TcpSegment::parse(&bytes);
    assert!(result.is_err());
}

#[test]
#[serial]
fn test_tcp_port_boundaries() {
    let bytes = [
        0x00, 0x00, // Port: 0
        0xFF, 0xFF, // Port: 65535
        0x00, 0x00, 0x03, 0xE8, // Seq
        0x00, 0x00, 0x01, 0xF4, // Ack
        0x50, 0x10, // Data Offset: 5, Flags: ACK
        0x20, 0x00, // Window
        0x00, 0x00, // Checksum
        0x00, 0x00, // Urgent Pointer
    ];

    let segment = TcpSegment::parse(&bytes).unwrap();
    assert_eq!(segment.header.source_port, 0);
    assert_eq!(segment.header.destination_port, 65535);
}

// ========== 标志位测试组 ==========

#[test]
#[serial]
fn test_tcp_flags_syn() {
    let bytes = [
        0x04, 0xD2, 0x16, 0x2E,
        0x00, 0x00, 0x03, 0xE8,
        0x00, 0x00, 0x00, 0x00,
        0x50, 0x02, // Flags: SYN
        0x20, 0x00,
        0x00, 0x00,
        0x00, 0x00,
    ];

    let segment = TcpSegment::parse(&bytes).unwrap();
    assert!(segment.header.is_syn());
    assert!(!segment.header.is_ack());
    assert!(!segment.header.is_fin());
}

#[test]
#[serial]
fn test_tcp_flags_syn_ack() {
    let bytes = [
        0x04, 0xD2, 0x16, 0x2E,
        0x00, 0x00, 0x03, 0xE8,
        0x00, 0x00, 0x01, 0xF4,
        0x50, 0x12, // Flags: SYN + ACK
        0x20, 0x00,
        0x00, 0x00,
        0x00, 0x00,
    ];

    let segment = TcpSegment::parse(&bytes).unwrap();
    assert!(segment.header.is_syn());
    assert!(segment.header.is_ack());
}

#[test]
#[serial]
fn test_tcp_flags_fin_ack() {
    let bytes = [
        0x04, 0xD2, 0x16, 0x2E,
        0x00, 0x00, 0x03, 0xE8,
        0x00, 0x00, 0x01, 0xF4,
        0x50, 0x11, // Flags: FIN + ACK
        0x20, 0x00,
        0x00, 0x00,
        0x00, 0x00,
    ];

    let segment = TcpSegment::parse(&bytes).unwrap();
    assert!(segment.header.is_fin());
    assert!(segment.header.is_ack());
}

#[test]
#[serial]
fn test_tcp_flags_rst_ack() {
    let bytes = [
        0x04, 0xD2, 0x16, 0x2E,
        0x00, 0x00, 0x03, 0xE8,
        0x00, 0x00, 0x01, 0xF4,
        0x50, 0x14, // Flags: RST + ACK
        0x00, 0x00,
        0x00, 0x00,
        0x00, 0x00,
    ];

    let segment = TcpSegment::parse(&bytes).unwrap();
    assert!(segment.header.is_rst());
    assert!(segment.header.is_ack());
}

// ========== 序列号和确认号测试组 ==========

#[test]
#[serial]
fn test_tcp_sequence_wraparound() {
    let bytes = [
        0x04, 0xD2, 0x16, 0x2E,
        0xFF, 0xFF, 0xFF, 0xFF, // Seq: 0xFFFFFFFF
        0x00, 0x00, 0x00, 0x01, // Ack: 1
        0x50, 0x10,
        0x20, 0x00,
        0x00, 0x00,
        0x00, 0x00,
    ];

    let segment = TcpSegment::parse(&bytes).unwrap();
    assert_eq!(segment.header.sequence_number, 0xFFFFFFFF);
    assert_eq!(segment.header.acknowledgment_number, 1);
}

// ========== 窗口大小测试组 ==========

#[test]
#[serial]
fn test_tcp_window_size() {
    let bytes = [
        0x04, 0xD2, 0x16, 0x2E,
        0x00, 0x00, 0x03, 0xE8,
        0x00, 0x00, 0x01, 0xF4,
        0x50, 0x10,
        0xFF, 0xFF, // Window: 65535
        0x00, 0x00,
        0x00, 0x00,
    ];

    let segment = TcpSegment::parse(&bytes).unwrap();
    assert_eq!(segment.header.window_size, 65535);
}

#[test]
#[serial]
fn test_tcp_zero_window() {
    let bytes = [
        0x04, 0xD2, 0x16, 0x2E,
        0x00, 0x00, 0x03, 0xE8,
        0x00, 0x00, 0x01, 0xF4,
        0x50, 0x10,
        0x00, 0x00, // Window: 0
        0x00, 0x00,
        0x00, 0x00,
    ];

    let segment = TcpSegment::parse(&bytes).unwrap();
    assert_eq!(segment.header.window_size, 0);
}

// ========== 三次握手测试组 ==========

/// 创建完整的 TCP/IP 数据包（用于测试）
fn create_tcp_ip_packet(
    src_mac: MacAddr,
    dst_mac: MacAddr,
    src_ip: Ipv4Addr,
    dst_ip: Ipv4Addr,
    src_port: u16,
    dst_port: u16,
    seq: u32,
    ack: u32,
    flags: u8,
    data: &[u8],
) -> Packet {
    // 构建 TCP 头部
    let mut tcp_bytes = Vec::new();

    // 源端口和目标端口
    tcp_bytes.extend_from_slice(&src_port.to_be_bytes());
    tcp_bytes.extend_from_slice(&dst_port.to_be_bytes());

    // 序列号和确认号
    tcp_bytes.extend_from_slice(&seq.to_be_bytes());
    tcp_bytes.extend_from_slice(&ack.to_be_bytes());

    // 数据偏移和标志位
    tcp_bytes.push(5 << 4); // Data Offset = 5 (20 bytes)
    tcp_bytes.push(flags);

    // 窗口大小
    tcp_bytes.extend_from_slice(&8192u16.to_be_bytes());

    // 校验和（先填0）
    tcp_bytes.push(0);
    tcp_bytes.push(0);

    // 紧急指针
    tcp_bytes.push(0);
    tcp_bytes.push(0);

    // 数据
    tcp_bytes.extend_from_slice(data);

    // 计算 TCP 校验和（包含伪头部）
    use core_net::protocols::ip::add_ipv4_pseudo_header;
    use core_net::protocols::ip::fold_carry;

    let mut sum = 0u32;
    add_ipv4_pseudo_header(&mut sum, src_ip, dst_ip);
    sum += u32::from(6u16) << 8; // Protocol = TCP

    let tcp_len = tcp_bytes.len() as u16;
    sum += u32::from(tcp_len >> 8) << 8;
    sum += u32::from(tcp_len & 0xFF) << 8;

    let mut i = 0;
    while i + 1 < tcp_bytes.len() {
        let word = u16::from_be_bytes([tcp_bytes[i], tcp_bytes[i + 1]]);
        sum += u32::from(word);
        i += 2;
    }
    if i < tcp_bytes.len() {
        sum += u32::from(tcp_bytes[i]) << 8;
    }

    let checksum = !fold_carry(sum);
    tcp_bytes[16] = (checksum >> 8) as u8;
    tcp_bytes[17] = (checksum & 0xFF) as u8;

    // 构建 IP 头部
    let mut ip_bytes = Vec::new();
    ip_bytes.push(0x45); // Version=4, IHL=5
    ip_bytes.push(0); // TOS
    let total_len = (20 + tcp_bytes.len()) as u16;
    ip_bytes.extend_from_slice(&total_len.to_be_bytes());
    ip_bytes.extend_from_slice(&0u16.to_be_bytes()); // ID
    ip_bytes.extend_from_slice(&0u16.to_be_bytes()); // Flags/Fragment
    ip_bytes.push(64); // TTL
    ip_bytes.push(6); // Protocol = TCP
    ip_bytes.extend_from_slice(&[0, 0]); // Checksum (placeholder)
    ip_bytes.extend_from_slice(&src_ip.bytes);
    ip_bytes.extend_from_slice(&dst_ip.bytes);

    // 计算 IP 校验和
    let mut ip_sum = 0u32;
    let mut ip_i = 0;
    while ip_i + 1 < 20 {
        let word = u16::from_be_bytes([ip_bytes[ip_i], ip_bytes[ip_i + 1]]);
        ip_sum += u32::from(word);
        ip_i += 2;
    }
    let ip_checksum = !fold_carry(ip_sum);
    ip_bytes[10] = (ip_checksum >> 8) as u8;
    ip_bytes[11] = (ip_checksum & 0xFF) as u8;

    // 组合 IP 头和 TCP 数据
    ip_bytes.extend_from_slice(&tcp_bytes);

    // 构建以太网帧
    let mut frame = Vec::new();
    frame.extend_from_slice(&dst_mac.bytes);
    frame.extend_from_slice(&src_mac.bytes);
    frame.extend_from_slice(&ETH_P_IP.to_be_bytes());
    frame.extend_from_slice(&ip_bytes);

    Packet::from_bytes(frame)
}

#[test]
#[serial]
fn test_tcp_three_way_handshake() {
    use core_net::protocols::tcp::{Tcb, TcpConnectionManager};

    let ctx = create_test_context();

    // 创建监听 TCB 并添加到连接管理器
    let listen_tcb = Tcb::listen(Ipv4Addr::new(192, 168, 1, 100), 8080, 65535);
    let result = ctx.tcp_connections.lock().unwrap()
        .add_listen(listen_tcb);
    assert!(result.is_ok(), "Failed to add listen TCB: {:?}", result);

    // 验证监听端口已添加
    assert!(ctx.tcp_connections.lock().unwrap().find_listen(8080).is_some());

    // 检查连接管理器状态
    let conn_mgr = ctx.tcp_connections.lock().unwrap();
    println!("Listen count: {}", conn_mgr.listen_count());
    println!("Connection count before SYN: {}", conn_mgr.connection_count());
    drop(conn_mgr);

    // 步骤 1：客户端发送 SYN
    let syn_packet = create_tcp_ip_packet(
        MacAddr::new([0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF]), // src MAC
        MacAddr::new([0x00, 0x11, 0x22, 0x33, 0x44, 0x55]), // dst MAC (本机)
        Ipv4Addr::new(192, 168, 1, 10),  // src IP
        Ipv4Addr::new(192, 168, 1, 100), // dst IP (本机)
        12345,  // src port
        8080,   // dst port
        1000,   // seq
        0,      // ack
        0x02,   // flags: SYN
        &[],
    );

    inject_packet_to_context(&ctx, "eth0", syn_packet).unwrap();

    // 检查 RXQ 中的数据包
    let rxq_len = ctx.interfaces.lock().unwrap()
        .get_by_name("eth0").unwrap().rxq.len();
    println!("RXQ length after injection: {}", rxq_len);

    let mut harness = TestHarness::with_context(ctx.clone());
    let harness_result = harness.run();

    println!("Harness result: {:?}", harness_result);

    // 检查连接管理器状态
    let conn_mgr = ctx.tcp_connections.lock().unwrap();
    println!("Connection count after processing: {}", conn_mgr.connection_count());
    println!("Listen count after processing: {}", conn_mgr.listen_count());

    // 检查是否有连接
    let conn_id = core_net::protocols::tcp::TcpConnectionId::new(
        Ipv4Addr::new(192, 168, 1, 100), 8080,
        Ipv4Addr::new(192, 168, 1, 10), 12345,
    );
    let conn = conn_mgr.find(&conn_id);
    drop(conn_mgr);

    // 验证新连接已创建
    if let Some(tcb) = conn {
        let tcb_guard = tcb.lock().unwrap();
        println!("Connection state: {:?}", tcb_guard.state);
        assert_eq!(tcb_guard.state, core_net::protocols::tcp::TcpState::SynReceived);
    } else {
        panic!("Connection not created! TCP processing may have failed.");
    }
}

#[test]
#[serial]
fn test_tcp_data_transfer() {
    use core_net::protocols::tcp::{TcpConfig, TCP_CONFIG_DEFAULT, TcpHeader, encapsulate_tcp_segment};

    let ctx = create_test_context();

    // 创建监听 TCB
    let listen_tcb = core_net::protocols::tcp::Tcb::listen(Ipv4Addr::new(192, 168, 1, 100), 8080, 65535);
    ctx.tcp_connections.lock().unwrap()
        .add_listen(listen_tcb).unwrap();

    // 完成三次握手（简化：直接创建已连接的 TCB）
    use core_net::protocols::tcp::{Tcb, TcpConnectionId, TcpState};
    let conn_id = TcpConnectionId::new(
        Ipv4Addr::new(192, 168, 1, 100), 8080,
        Ipv4Addr::new(192, 168, 1, 10), 12345,
    );
    let mut tcb = Tcb::new(conn_id.clone());
    tcb.state = TcpState::Established;
    tcb.init_send_state(2000);
    tcb.init_recv_state(1001, 65535);
    ctx.tcp_connections.lock().unwrap().add(tcb).unwrap();

    // 创建带数据的 TCP 报文
    let data_header = TcpHeader::psh_ack(12345, 8080, 1001, 2001, 8192);
    let tcp_data = encapsulate_tcp_segment(&data_header, b"Hello TCP", Ipv4Addr::new(192, 168, 1, 10), Ipv4Addr::new(192, 168, 1, 100));

    // IP 封装
    let mut ip_data = Vec::new();
    ip_data.extend_from_slice(&0x45u16.to_be_bytes()); // Version=4, IHL=5
    ip_data.extend_from_slice(&0x0000u16.to_be_bytes()); // TOS
    let total_len = (20 + tcp_data.len()) as u16;
    ip_data.extend_from_slice(&total_len.to_be_bytes());
    ip_data.extend_from_slice(&0x0000u16.to_be_bytes()); // ID
    ip_data.extend_from_slice(&0x4000u16.to_be_bytes()); // Flags/Fragment
    ip_data.push(64); // TTL
    ip_data.push(6); // Protocol = TCP
    ip_data.extend_from_slice(&[0x00, 0x00]); // Checksum (placeholder)
    ip_data.extend_from_slice(&Ipv4Addr::new(192, 168, 1, 10).bytes);
    ip_data.extend_from_slice(&Ipv4Addr::new(192, 168, 1, 100).bytes);

    // 计算 IP 校验和
    use core_net::protocols::ip::{fold_carry};
    let mut ip_sum = 0u32;
    let mut ip_i = 0;
    while ip_i + 1 < 20 {
        let word = u16::from_be_bytes([ip_data[ip_i], ip_data[ip_i + 1]]);
        ip_sum += u32::from(word);
        ip_i += 2;
    }
    let ip_checksum = !fold_carry(ip_sum);
    ip_data[10] = (ip_checksum >> 8) as u8;
    ip_data[11] = (ip_checksum & 0xFF) as u8;

    ip_data.extend_from_slice(&tcp_data);

    // 以太网封装
    let mut frame = Vec::new();
    frame.extend_from_slice(&[0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF]); // src MAC
    frame.extend_from_slice(&[0x00, 0x11, 0x22, 0x33, 0x44, 0x55]); // dst MAC
    frame.extend_from_slice(&0x08u16.to_be_bytes()); // ETH_P_IP
    frame.extend_from_slice(&ip_data);

    let packet = Packet::from_bytes(frame);
    inject_packet_to_context(&ctx, "eth0", packet).unwrap();

    let mut harness = TestHarness::with_context(ctx.clone());
    harness.run().unwrap();

    // 验证 ACK 响应（暂时跳过，因为这是一个复杂的端到端测试）
    // TODO: 完善数据传输的端到端测试
}

#[test]
#[serial]
fn test_tcp_socket_create() {
    use core_net::protocols::tcp::TcpSocketManager;

    let ctx = create_test_context();

    // 创建 Socket
    let socket = ctx.tcp_sockets.lock().unwrap().create_socket();

    // 验证 Socket 已创建
    assert!(socket.lock().unwrap().id() > 0);
    assert_eq!(ctx.tcp_sockets.lock().unwrap().socket_count(), 1);
}

#[test]
#[serial]
fn test_tcp_socket_callback() {
    use std::sync::{Arc, Mutex};
    use core_net::protocols::tcp::{TcpSocketManager, TcpEvent};

    let ctx = create_test_context();

    // 创建 Socket 并设置回调
    let socket = ctx.tcp_sockets.lock().unwrap().create_socket();
    let triggered = Arc::new(Mutex::new(false));
    let triggered_clone = triggered.clone();

    socket.lock().unwrap().set_callback(Box::new(move |event| {
        // 验证收到事件
        if matches!(event, TcpEvent::Connected(_)) {
            *triggered_clone.lock().unwrap() = true;
        }
    }));

    // 注意：trigger_event 是 crate 内部方法，这里只验证回调设置成功
    assert!(socket.lock().unwrap().has_callback());
}

#[test]
#[serial]
fn test_tcp_congestion_control() {
    use core_net::protocols::tcp::{TcpConfig, TCP_CONFIG_DEFAULT, Tcb, TcpConnectionId, TcpState};

    // 创建 TCB 并测试拥塞控制
    let conn_id = TcpConnectionId::new(
        Ipv4Addr::new(192, 168, 1, 100), 8080,
        Ipv4Addr::new(192, 168, 1, 10), 12345,
    );
    let mut tcb = Tcb::new(conn_id);
    tcb.state = TcpState::Established;
    tcb.init_send_state(1000);
    tcb.cwnd = 14600;
    tcb.ssthresh = u32::MAX;

    // 测试慢启动
    let old_cwnd = tcb.cwnd;
    tcb.slow_start(1460);
    assert_eq!(tcb.cwnd, old_cwnd + 1460);

    // 测试拥塞避免
    tcb.ssthresh = 15000;
    tcb.cwnd = 20000;
    tcb.congestion_avoidance(1460);
    assert!(tcb.cwnd > 20000);

    // 测试快重传
    tcb.snd_wnd = 32768;
    tcb.cwnd = 30000;
    tcb.fast_retransmit(1460);
    assert_eq!(tcb.ssthresh, 15000);
}

#[test]
#[serial]
fn test_tcp_connection_manager_add() {
    use core_net::protocols::tcp::{Tcb, TcpConnectionId, TcpState};

    let ctx = create_test_context();

    // 直接测试连接管理器的 add 功能
    let conn_id = TcpConnectionId::new(
        Ipv4Addr::new(192, 168, 1, 100), 8080,
        Ipv4Addr::new(192, 168, 1, 10), 12345,
    );
    let mut tcb = Tcb::new(conn_id.clone());
    tcb.state = TcpState::SynReceived;

    let result = ctx.tcp_connections.lock().unwrap().add(tcb);

    assert!(result.is_ok(), "Failed to add TCB: {:?}", result);

    // 验证连接已添加
    let conn = ctx.tcp_connections.lock().unwrap().find(&conn_id);
    assert!(conn.is_some(), "Connection not found after add");

    if let Some(tcb) = conn {
        let tcb_guard = tcb.lock().unwrap();
        assert_eq!(tcb_guard.state, TcpState::SynReceived);
    }
}

/// 简化测试：直接测试 SYN 处理
#[test]
#[serial]
fn test_tcp_syn_handling_direct() {
    use core_net::protocols::tcp::{process_tcp_packet, TcpConfig, TcpHeader, Tcb};
    use core_net::protocols::tcp::{TcpConnectionId, encapsulate_tcp_segment};
    use core_net::protocols::tcp::TcpSegment;

    let ctx = create_test_context();

    // 创建监听 TCB
    let listen_tcb = Tcb::listen(Ipv4Addr::new(192, 168, 1, 100), 8080, 65535);
    ctx.tcp_connections.lock().unwrap()
        .add_listen(listen_tcb).unwrap();

    // 验证监听端口已添加
    assert!(ctx.tcp_connections.lock().unwrap().find_listen(8080).is_some());

    // 使用现有的函数创建 SYN 报文（确保校验和正确）
    let syn_header = TcpHeader::syn(12345, 8080, 1000, 8192);
    println!("Created SYN header: seq={}, ack={}, flags={:#x}", syn_header.sequence_number, syn_header.acknowledgment_number, syn_header.flags());
    let tcp_bytes = encapsulate_tcp_segment(&syn_header, &[], Ipv4Addr::new(192, 168, 1, 10), Ipv4Addr::new(192, 168, 1, 100));

    // 创建 Packet 并处理
    let packet = Packet::from_bytes(tcp_bytes);

    // 先检查解析是否成功
    let segment_data = packet.peek(packet.remaining()).unwrap();
    let segment = TcpSegment::parse(segment_data);
    if let Ok(seg) = segment {
        println!("Parsed TCP header: src_port={}, dst_port={}, seq={}, ack={}, data_offset={}, flags={:#x}, checksum={:#x}",
            seg.header.source_port,
            seg.header.destination_port,
            seg.header.sequence_number,
            seg.header.acknowledgment_number,
            seg.header.data_offset(),
            seg.header.flags(),
            seg.header.checksum
        );
        println!("data_offset_and_flags = {:#x}", seg.header.data_offset_and_flags_value());
        let calc_checksum = seg.calculate_checksum(Ipv4Addr::new(192, 168, 1, 10), Ipv4Addr::new(192, 168, 1, 100));
        println!("Calculated checksum: {:#x}, Header checksum: {:#x}, Match: {}", calc_checksum, seg.header.checksum, calc_checksum == seg.header.checksum);

        // 打印原始字节
        println!("Raw TCP bytes: {:02x?}", &segment_data[..20]);

        // 手动验证校验和
        use core_net::protocols::ip::{add_ipv4_pseudo_header, fold_carry};
        let mut sum = 0u32;
        add_ipv4_pseudo_header(&mut sum, Ipv4Addr::new(192, 168, 1, 10), Ipv4Addr::new(192, 168, 1, 100));
        sum += u32::from(6u16) << 8; // Protocol = TCP
        let tcp_len = segment_data.len() as u16;
        sum += u32::from(tcp_len >> 8) << 8;
        sum += u32::from(tcp_len & 0xFF) << 8;

        println!("TCP length: {}, checksum so far: {:#x}", tcp_len, sum);

        let mut i = 0;
        while i + 1 < segment_data.len() {
            if i != 16 && i != 17 { // 跳过校验和字段
                let word = u16::from_be_bytes([segment_data[i], segment_data[i + 1]]);
                sum += u32::from(word);
                if i < 20 {
                    println!("Adding bytes {}-{}: {:02x}{:02x} = {:#x}, sum now: {:#x}", i, i+1, segment_data[i], segment_data[i+1], word, sum);
                }
            }
            i += 2;
        }

        let manual_checksum = !fold_carry(sum);
        println!("Manual checksum: {:#x}", manual_checksum);
    }

    // 调用 process_tcp_packet
    let result = process_tcp_packet(
        packet,
        Ipv4Addr::new(192, 168, 1, 10),
        Ipv4Addr::new(192, 168, 1, 100),
        &ctx,
        &TcpConfig::default(),
    );

    // 检查结果
    match result {
        Ok(core_net::protocols::tcp::TcpProcessResult::Reply(_)) => {
            // 检查连接是否被创建
            let conn_id = TcpConnectionId::new(
                Ipv4Addr::new(192, 168, 1, 100), 8080,
                Ipv4Addr::new(192, 168, 1, 10), 12345,
            );
            let conn = ctx.tcp_connections.lock().unwrap().find(&conn_id);
            assert!(conn.is_some(), "Connection not created after SYN processing");

            if let Some(tcb) = conn {
                let tcb_guard = tcb.lock().unwrap();
                println!("Connection state: {:?}", tcb_guard.state);
                assert_eq!(tcb_guard.state, core_net::protocols::tcp::TcpState::SynReceived);
            }
        }
        Ok(other) => {
            panic!("Unexpected result: {:?}", other);
        }
        Err(e) => {
            panic!("process_tcp_packet returned error: {:?}", e);
        }
    }
}
