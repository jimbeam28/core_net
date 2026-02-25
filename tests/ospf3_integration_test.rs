// tests/ospf3_integration_test.rs
//
// OSPFv3 集成测试
// 使用 test_framework 框架测试 OSPFv3 协议功能

use serial_test::serial;
use core_net::testframework::*;
use core_net::protocols::ospf3::*;
use core_net::protocols::ospf::{InterfaceState, InterfaceType};
use core_net::common::{Packet, Ipv6Addr};
use core_net::context::SystemContext;

// 通用测试辅助函数
fn create_test_context() -> SystemContext {
    GlobalStateManager::create_context()
}

fn create_ospfv3_hello_packet(
    router_id: u32,
    area_id: u32,
    interface_id: u32,
    hello_interval: u16,
    dead_interval: u32,
    neighbors: Vec<u32>,
) -> Vec<u8> {
    let mut header = Ospfv3Header::new(1, router_id, area_id);

    let mut hello = Ospfv3Hello::new(interface_id, hello_interval, dead_interval, 1);

    for neighbor in neighbors {
        hello.add_neighbor(neighbor);
    }

    let hello_bytes = hello.to_bytes();
    header.length = (Ospfv3Header::LENGTH as u16) + (hello_bytes.len() as u16);
    header.calculate_checksum(&hello_bytes);

    let mut packet_bytes = header.to_bytes();
    packet_bytes.extend_from_slice(&hello_bytes);

    packet_bytes
}

fn create_ospfv3_dd_packet(
    router_id: u32,
    area_id: u32,
    interface_mtu: u16,
    dd_sequence_number: u32,
) -> Vec<u8> {
    let mut header = Ospfv3Header::new(2, router_id, area_id);

    let mut dd = Ospfv3DatabaseDescription::new(interface_mtu, dd_sequence_number);
    dd.i_bit = true;
    dd.m_bit = true;
    dd.ms_bit = true;

    let dd_bytes = dd.to_bytes();
    header.length = (Ospfv3Header::LENGTH as u16) + (dd_bytes.len() as u16);
    header.calculate_checksum(&dd_bytes);

    let mut packet_bytes = header.to_bytes();
    packet_bytes.extend_from_slice(&dd_bytes);

    packet_bytes
}

// ==================== Hello 报文测试 ====================

#[test]
#[serial]
fn test_ospfv3_hello_packet_creation() {
    let router_id: u32 = 0x01020304;
    let area_id: u32 = 0x00000001;

    let packet_bytes = create_ospfv3_hello_packet(
        router_id,
        area_id,
        1,
        10,
        40,
        vec![],
    );

    assert!(packet_bytes.len() >= Ospfv3Header::LENGTH + Ospfv3Hello::MIN_LENGTH);

    // 首先解析 OSPFv3 头部
    let header = Ospfv3Header::from_bytes(&packet_bytes).unwrap();
    assert_eq!(header.version, 3);
    assert_eq!(header.packet_type, 1);
    assert_eq!(header.router_id, router_id);
    assert_eq!(header.area_id, area_id);

    // 然后解析 Hello 报文（从头部后开始）
    let hello_data = &packet_bytes[Ospfv3Header::LENGTH..];
    assert_eq!(hello_data.len(), Ospfv3Hello::MIN_LENGTH);

    let hello = Ospfv3Hello::from_bytes(hello_data).unwrap();
    assert_eq!(hello.interface_id, 1);
    assert_eq!(hello.hello_interval, 10);
    assert_eq!(hello.router_dead_interval, 40);
}

#[test]
#[serial]
fn test_ospfv3_hello_packet_with_neighbors() {
    let router_id: u32 = 0x01020304;
    let area_id: u32 = 0x00000001;
    let neighbor1: u32 = 0x00000002;
    let neighbor2: u32 = 0x00000003;

    let packet_bytes = create_ospfv3_hello_packet(
        router_id,
        area_id,
        1,
        10,
        40,
        vec![neighbor1, neighbor2],
    );

    let hello_data = &packet_bytes[Ospfv3Header::LENGTH..];
    // Hello 报文长度 = 22 (固定部分) + 2 * 4 (2个邻居) = 30
    assert_eq!(hello_data.len(), Ospfv3Hello::MIN_LENGTH + 2 * 4);

    let hello = Ospfv3Hello::from_bytes(hello_data).unwrap();
    assert_eq!(hello.neighbors.len(), 2);
    assert!(hello.neighbors.contains(&neighbor1));
    assert!(hello.neighbors.contains(&neighbor2));
}

#[test]
#[serial]
fn test_ospfv3_hello_empty_neighbor_list() {
    let router_id: u32 = 0x01020304;
    let area_id: u32 = 0x00000001;

    let packet_bytes = create_ospfv3_hello_packet(
        router_id,
        area_id,
        1,
        10,
        40,
        vec![],
    );

    let hello_data = &packet_bytes[Ospfv3Header::LENGTH..];
    // Hello 报文长度 = 22 (固定部分，无邻居)
    assert_eq!(hello_data.len(), Ospfv3Hello::MIN_LENGTH);

    let hello = Ospfv3Hello::from_bytes(hello_data).unwrap();
    assert_eq!(hello.neighbors.len(), 0);
}

#[test]
#[serial]
fn test_ospfv3_hello_max_neighbors() {
    let router_id: u32 = 0x01020304;
    let area_id: u32 = 0x00000001;

    let mut neighbors = Vec::new();
    for i in 1..=10 {
        neighbors.push(i as u32);
    }

    let packet_bytes = create_ospfv3_hello_packet(
        router_id,
        area_id,
        1,
        10,
        40,
        neighbors.clone(),
    );

    let hello_data = &packet_bytes[Ospfv3Header::LENGTH..];
    // Hello 报文长度 = 22 + 10 * 4 = 62
    assert_eq!(hello_data.len(), Ospfv3Hello::MIN_LENGTH + 10 * 4);

    let hello = Ospfv3Hello::from_bytes(hello_data).unwrap();
    assert_eq!(hello.neighbors.len(), 10);

    for neighbor in &neighbors {
        assert!(hello.neighbors.contains(neighbor));
    }
}

// ==================== Database Description 报文测试 ====================

#[test]
#[serial]
fn test_ospfv3_dd_packet_creation() {
    let router_id: u32 = 0x01020304;
    let area_id: u32 = 0x00000001;

    let packet_bytes = create_ospfv3_dd_packet(
        router_id,
        area_id,
        1500,
        0x80000001,
    );

    let header = Ospfv3Header::from_bytes(&packet_bytes).unwrap();
    assert_eq!(header.version, 3);
    assert_eq!(header.packet_type, 2);

    let dd_data = &packet_bytes[Ospfv3Header::LENGTH..];
    // DD 报文固定部分 = 12 字节 (MTU 2 + Options 2 + Flags 1 + Reserved 1 + DD Seq 4)
    // 加上标志位后的实际数据
    assert!(dd_data.len() >= Ospfv3DatabaseDescription::MIN_LENGTH);
    assert_eq!(header.router_id, router_id);
}

// ==================== 邻居状态测试 ====================

#[test]
#[serial]
fn test_ospfv3_neighbor_creation() {
    let router_id: u32 = 0x01020304;
    let link_local_addr = Ipv6Addr::new(0xfe80, 0, 0, 0, 0, 0, 0, 2);

    let neighbor = Ospfv3Neighbor::new(router_id, link_local_addr, 40);

    assert_eq!(neighbor.router_id, router_id);
    assert_eq!(neighbor.link_local_addr, link_local_addr);
    assert_eq!(neighbor.priority, 1);
    assert!(!neighbor.is_inactivity_timer_expired());
}

#[test]
#[serial]
fn test_ospfv3_neighbor_dd_sequence_init() {
    let router_id: u32 = 0x01020304;
    let link_local_addr = Ipv6Addr::new(0xfe80, 0, 0, 0, 0, 0, 0, 2);

    let mut neighbor = Ospfv3Neighbor::new(router_id, link_local_addr, 40);
    neighbor.init_dd_sequence();

    // DD 序列号应该设置了最高位
    assert!(neighbor.dd_seq_number & 0x80000000 != 0);
}

#[test]
#[serial]
fn test_ospfv3_neighbor_inactivity_timer() {
    let router_id: u32 = 0x01020304;
    let link_local_addr = Ipv6Addr::new(0xfe80, 0, 0, 0, 0, 0, 0, 2);

    let mut neighbor = Ospfv3Neighbor::new(router_id, link_local_addr, 40);
    assert!(!neighbor.is_inactivity_timer_expired());

    // 重置定时器
    neighbor.reset_inactivity_timer(40);
    assert!(!neighbor.is_inactivity_timer_expired());
}

#[test]
#[serial]
fn test_ospfv3_neighbor_needs_adjacency() {
    let router_id: u32 = 0x01020304;
    let link_local_addr = Ipv6Addr::new(0xfe80, 0, 0, 0, 0, 0, 0, 2);

    let neighbor = Ospfv3Neighbor::new(router_id, link_local_addr, 40);

    // DR 和 BDR 之间需要建立邻接关系
    assert!(neighbor.needs_adjacency(true, false, false, true));
    assert!(neighbor.needs_adjacency(false, true, true, false));

    // DR/BDR 与非 DR/BDR 路由器建立邻接关系
    assert!(neighbor.needs_adjacency(true, false, false, false));
    assert!(neighbor.needs_adjacency(false, true, false, false));

    // 两个非 DR/BDR 路由器之间不需要建立邻接关系
    assert!(!neighbor.needs_adjacency(false, false, false, false));
}

// ==================== 接口状态测试 ====================

#[test]
#[serial]
fn test_ospfv3_interface_creation() {
    let ip_addr = Ipv6Addr::new(0xfe80, 0, 0, 0, 0, 0, 0, 1);
    let area_id: u32 = 0x00000001;
    let iface = Ospfv3Interface::new("eth0".to_string(), 1, ip_addr, area_id);

    assert_eq!(iface.ifindex, 1);
    assert_eq!(iface.if_type, InterfaceType::Broadcast);
    assert_eq!(iface.state, InterfaceState::Down);
    assert_eq!(iface.hello_interval, 10);
    assert_eq!(iface.dead_interval, 40);
}

#[test]
#[serial]
fn test_ospfv3_interface_state_down_to_waiting() {
    let ip_addr = Ipv6Addr::new(0xfe80, 0, 0, 0, 0, 0, 0, 1);
    let area_id: u32 = 0x00000001;
    let mut iface = Ospfv3Interface::new("eth0".to_string(), 1, ip_addr, area_id);

    assert_eq!(iface.state, InterfaceState::Down);

    iface.up();

    assert_eq!(iface.state, InterfaceState::Waiting);
}

#[test]
#[serial]
fn test_ospfv3_interface_state_point_to_point() {
    let ip_addr = Ipv6Addr::new(0xfe80, 0, 0, 0, 0, 0, 0, 1);
    let area_id: u32 = 0x00000001;
    let mut iface = Ospfv3Interface::new("eth0".to_string(), 1, ip_addr, area_id);
    iface.if_type = InterfaceType::PointToPoint;

    assert_eq!(iface.state, InterfaceState::Down);

    iface.up();

    assert_eq!(iface.state, InterfaceState::PointToPoint);
}

#[test]
#[serial]
fn test_ospfv3_interface_priority() {
    let ip_addr = Ipv6Addr::new(0xfe80, 0, 0, 0, 0, 0, 0, 1);
    let area_id: u32 = 0x00000001;
    let mut iface = Ospfv3Interface::new("eth0".to_string(), 1, ip_addr, area_id);

    assert_eq!(iface.priority, 1);

    iface.priority = 10;
    assert_eq!(iface.priority, 10);
}

#[test]
#[serial]
fn test_ospfv3_interface_timers() {
    let ip_addr = Ipv6Addr::new(0xfe80, 0, 0, 0, 0, 0, 0, 1);
    let area_id: u32 = 0x00000001;
    let mut iface = Ospfv3Interface::new("eth0".to_string(), 1, ip_addr, area_id);

    iface.up();

    // Hello 定时器应该已设置
    assert!(iface.hello_timer.is_some());

    // 等待定时器应该已设置（广播网络）
    assert!(iface.wait_timer.is_some());

    // 检查 Hello 定时器是否超时（应该还未超时）
    assert!(!iface.is_hello_timer_expired());
}

#[test]
#[serial]
fn test_ospfv3_interface_dr_bdr() {
    let ip_addr = Ipv6Addr::new(0xfe80, 0, 0, 0, 0, 0, 0, 1);
    let area_id: u32 = 0x00000001;
    let mut iface = Ospfv3Interface::new("eth0".to_string(), 1, ip_addr, area_id);

    let dr: u32 = 0x01020304;
    let bdr: u32 = 0x01020305;

    iface.set_dr(dr);
    iface.set_bdr(bdr);

    assert_eq!(iface.dr, dr);
    assert_eq!(iface.bdr, bdr);
}

#[test]
#[serial]
fn test_ospfv3_interface_down() {
    let ip_addr = Ipv6Addr::new(0xfe80, 0, 0, 0, 0, 0, 0, 1);
    let area_id: u32 = 0x00000001;
    let mut iface = Ospfv3Interface::new("eth0".to_string(), 1, ip_addr, area_id);

    iface.up();
    assert_ne!(iface.state, InterfaceState::Down);

    iface.down();
    assert_eq!(iface.state, InterfaceState::Down);
    assert!(iface.hello_timer.is_none());
    assert!(iface.wait_timer.is_none());
}

// ==================== 处理结果测试 ====================

#[test]
#[serial]
fn test_ospfv3_process_result_no_reply() {
    let ctx = create_test_context();
    let _harness = TestHarness::with_context(ctx);

    // 测试 NoReply 结果
    let result = Ospfv3ProcessResult::NoReply;
    match result {
        Ospfv3ProcessResult::NoReply => {}
        _ => panic!("Expected NoReply"),
    }
}

#[test]
#[serial]
fn test_ospfv3_process_result_reply() {
    let ctx = create_test_context();
    let _harness = TestHarness::with_context(ctx);

    // 测试 Reply 结果
    let data = vec![1u8, 2, 3, 4];
    let result = Ospfv3ProcessResult::Reply(data.clone());
    match result {
        Ospfv3ProcessResult::Reply(d) => {
            assert_eq!(d, data);
        }
        _ => panic!("Expected Reply"),
    }
}

#[test]
#[serial]
fn test_ospfv3_process_result_schedule_spf() {
    let ctx = create_test_context();
    let _harness = TestHarness::with_context(ctx);

    // 测试 ScheduleSpfCalculation 结果
    let result = Ospfv3ProcessResult::ScheduleSpfCalculation;
    match result {
        Ospfv3ProcessResult::ScheduleSpfCalculation => {}
        _ => panic!("Expected ScheduleSpfCalculation"),
    }
}

// ==================== Hello 报文接收处理测试 ====================

#[test]
#[serial]
fn test_ospfv3_hello_received_from_new_neighbor() {
    let ctx = create_test_context();
    let mut harness = TestHarness::with_context(ctx);

    let source_ip = Ipv6Addr::new(0xfe80, 0, 0, 0, 0, 0, 0, 2);
    let router_id: u32 = 0x01020304;
    let area_id: u32 = 0x00000001;

    let hello_packet = create_ospfv3_hello_packet(
        router_id,
        area_id,
        1,
        10,
        40,
        vec![],
    );

    // 注入 Hello 报文到接口 eth0
    let packet = Packet::from_bytes(hello_packet);
    let ctx = harness.context().expect("Context should be set");
    let mut injector = PacketInjector::with_context(ctx);
    let result = injector.inject("eth0", packet);
    assert!(result.is_ok());

    // 处理报文
    let run_result = harness.run();
    assert!(run_result.is_ok());
}

#[test]
#[serial]
fn test_ospfv3_hello_with_bidirectional_communication() {
    let ctx = create_test_context();
    let mut harness = TestHarness::with_context(ctx);

    let source_ip = Ipv6Addr::new(0xfe80, 0, 0, 0, 0, 0, 0, 2);
    let router_id: u32 = 0x01020304;
    let area_id: u32 = 0x00000001;

    // Hello 报文包含我们自己的 Router ID，表示双向通信已建立
    let our_router_id: u32 = 0x09080706;
    let hello_packet = create_ospfv3_hello_packet(
        router_id,
        area_id,
        1,
        10,
        40,
        vec![our_router_id],
    );

    let packet = Packet::from_bytes(hello_packet);
    let ctx = harness.context().expect("Context should be set");
    let mut injector = PacketInjector::with_context(ctx);
    let result = injector.inject("eth0", packet);
    assert!(result.is_ok());

    let run_result = harness.run();
    assert!(run_result.is_ok());
}
