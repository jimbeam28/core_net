// OSPF 协议集成测试
//
// 测试 OSPF 协议的报文解析、封装、状态机、邻居管理

use core_net::testframework::TestHarness;
use core_net::interface::{MacAddr, Ipv4Addr};
use core_net::protocols::ospf2::{OspfProcessResult, OspfInterface, OspfNeighbor, NeighborState, InterfaceState};
use core_net::protocols::ospf::{InterfaceType};

use serial_test::serial;

mod common;
use common::{create_test_context, create_ospf_hello_packet, create_ospf_dd_packet,
             inject_packet_to_context};

// 测试环境配置：
// 本机接口 eth0: ifindex=0, MAC=00:11:22:33:44:55, IP=192.168.1.100
// 本机路由器 ID: 1.1.1.1
// 邻居路由器: Router ID=2.2.2.2, IP=192.168.1.2

// ========== 报文解析和封装测试 ==========

#[test]
#[serial]
fn test_ospf_hello_packet_creation() {
    let src_mac = MacAddr::new([0x00, 0x11, 0x22, 0x33, 0x44, 0x55]);
    let src_ip = Ipv4Addr::new(192, 168, 1, 100);
    let router_id = Ipv4Addr::new(1, 1, 1, 1);
    let network_mask = Ipv4Addr::new(255, 255, 255, 0);

    let packet = create_ospf_hello_packet(
        src_mac,
        src_ip,
        router_id,
        network_mask,
        10, // hello_interval
        40, // dead_interval
        vec![],
    );

    // 验证基本结构
    assert!(packet.len() >= 14 + 20 + 24); // 以太网 + IP + OSPF头部 + Hello
}

#[test]
#[serial]
fn test_ospf_hello_packet_with_neighbors() {
    let src_mac = MacAddr::new([0x00, 0x11, 0x22, 0x33, 0x44, 0x55]);
    let src_ip = Ipv4Addr::new(192, 168, 1, 100);
    let router_id = Ipv4Addr::new(1, 1, 1, 1);
    let network_mask = Ipv4Addr::new(255, 255, 255, 0);

    let neighbors = vec![
        Ipv4Addr::new(2, 2, 2, 2),
        Ipv4Addr::new(3, 3, 3, 3),
    ];

    let packet = create_ospf_hello_packet(
        src_mac,
        src_ip,
        router_id,
        network_mask,
        10,
        40,
        neighbors,
    );

    // 验证包含邻居列表
    assert!(packet.len() >= 14 + 20 + 24 + 8); // 额外的 2 个邻居 = 8 字节
}

// ========== Hello 报文接收测试 ==========

#[test]
#[serial]
fn test_ospf_hello_received_from_new_neighbor() {
    let ctx = create_test_context();

    // 模拟来自邻居路由器的 Hello 报文
    let neighbor_mac = MacAddr::new([0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0x01]);
    let neighbor_ip = Ipv4Addr::new(192, 168, 1, 2);
    let neighbor_router_id = Ipv4Addr::new(2, 2, 2, 2);
    let network_mask = Ipv4Addr::new(255, 255, 255, 0);

    let hello_packet = create_ospf_hello_packet(
        neighbor_mac,
        neighbor_ip,
        neighbor_router_id,
        network_mask,
        10,
        40,
        vec![],
    );

    // 注入报文
    inject_packet_to_context(&ctx, "eth0", hello_packet).unwrap();

    // 运行调度器
    let mut harness = TestHarness::with_context(ctx.clone());
    let result = harness.run();
    assert!(result.is_ok(), "调度器运行失败: {:?}", result.err());

    // 注意：当前 OSPF 实现是简化版本，主要验证报文能被正确解析
    // 完整的邻居状态机测试需要更详细的实现
}

#[test]
#[serial]
fn test_ospf_hello_with_bidirectional_communication() {
    let ctx = create_test_context();

    let neighbor_mac = MacAddr::new([0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0x01]);
    let neighbor_ip = Ipv4Addr::new(192, 168, 1, 2);
    let neighbor_router_id = Ipv4Addr::new(2, 2, 2, 2);
    let local_router_id = Ipv4Addr::new(1, 1, 1, 1);

    // 邻居的 Hello 包含本机路由器 ID（双向通信）
    let hello_packet = create_ospf_hello_packet(
        neighbor_mac,
        neighbor_ip,
        neighbor_router_id,
        Ipv4Addr::new(255, 255, 255, 0),
        10,
        40,
        vec![local_router_id], // 包含本机路由器 ID
    );

    inject_packet_to_context(&ctx, "eth0", hello_packet).unwrap();

    let mut harness = TestHarness::with_context(ctx.clone());
    let result = harness.run();
    assert!(result.is_ok());
}

// ========== Database Description 报文测试 ==========

#[test]
#[serial]
fn test_ospf_dd_packet_creation() {
    let src_mac = MacAddr::new([0x00, 0x11, 0x22, 0x33, 0x44, 0x55]);
    let src_ip = Ipv4Addr::new(192, 168, 1, 100);
    let router_id = Ipv4Addr::new(1, 1, 1, 1);

    let dd_packet = create_ospf_dd_packet(
        src_mac,
        src_ip,
        router_id,
        0x80000001, // DD 序列号
    );

    // 验证基本结构
    assert!(dd_packet.len() >= 14 + 20 + 24 + 8); // 以太网 + IP + OSPF + DD
}

// ========== OSPF 接口状态测试 ==========

#[test]
#[serial]
fn test_ospf_interface_state_down_to_waiting() {
    let mut iface = OspfInterface::new(
        "eth0".to_string(),
        1,
        Ipv4Addr::new(192, 168, 1, 100),
        Ipv4Addr::new(255, 255, 255, 0),
        Ipv4Addr::new(0, 0, 0, 0),
    );

    assert_eq!(iface.state, InterfaceState::Down);

    // 启动接口（广播网络）
    iface.up();

    // 应该进入 Waiting 状态等待 DR/BDR 选举
    assert_eq!(iface.state, InterfaceState::Waiting);
    assert!(iface.hello_timer.is_some());
    assert!(iface.wait_timer.is_some());
}

#[test]
#[serial]
fn test_ospf_interface_state_point_to_point() {
    let mut iface = OspfInterface::new(
        "eth0".to_string(),
        1,
        Ipv4Addr::new(192, 168, 1, 100),
        Ipv4Addr::new(255, 255, 255, 255),
        Ipv4Addr::new(0, 0, 0, 0),
    );

    // 设置为点到点网络类型
    iface.if_type = InterfaceType::PointToPoint;

    iface.up();

    // 点到点网络直接进入 PointToPoint 状态
    assert_eq!(iface.state, InterfaceState::PointToPoint);
    assert!(iface.hello_timer.is_some());
    assert!(iface.wait_timer.is_none()); // 点到点不需要 Wait 定时器
}

#[test]
#[serial]
fn test_ospf_interface_down() {
    let mut iface = OspfInterface::new(
        "eth0".to_string(),
        1,
        Ipv4Addr::new(192, 168, 1, 100),
        Ipv4Addr::new(255, 255, 255, 0),
        Ipv4Addr::new(0, 0, 0, 0),
    );

    iface.up();
    assert_ne!(iface.state, InterfaceState::Down);

    // 关闭接口
    iface.down();

    assert_eq!(iface.state, InterfaceState::Down);
    assert_eq!(iface.dr, Ipv4Addr::unspecified());
    assert_eq!(iface.bdr, Ipv4Addr::unspecified());
    assert!(iface.hello_timer.is_none());
    assert!(iface.wait_timer.is_none());
}

// ========== OSPF 邻居状态测试 ==========

#[test]
#[serial]
fn test_ospf_neighbor_creation() {
    let router_id = Ipv4Addr::new(2, 2, 2, 2);
    let ip_addr = Ipv4Addr::new(192, 168, 1, 2);

    let neighbor = OspfNeighbor::new(router_id, ip_addr, 40);

    assert_eq!(neighbor.router_id, router_id);
    assert_eq!(neighbor.ip_addr, ip_addr);
    assert_eq!(neighbor.state, NeighborState::Down);
    assert_eq!(neighbor.priority, 1);
    assert_eq!(neighbor.dr, Ipv4Addr::unspecified());
    assert_eq!(neighbor.bdr, Ipv4Addr::unspecified());
}

#[test]
#[serial]
fn test_ospf_neighbor_inactivity_timer() {
    let mut neighbor = OspfNeighbor::new(
        Ipv4Addr::new(2, 2, 2, 2),
        Ipv4Addr::new(192, 168, 1, 2),
        40, // dead_interval = 40秒
    );

    // 初始状态下，未超时
    assert!(!neighbor.is_inactivity_timer_expired());

    // 重置定时器
    neighbor.reset_inactivity_timer(40);
    assert!(!neighbor.is_inactivity_timer_expired());
}

#[test]
#[serial]
fn test_ospf_neighbor_state_transitions() {
    let mut neighbor = OspfNeighbor::new(
        Ipv4Addr::new(2, 2, 2, 2),
        Ipv4Addr::new(192, 168, 1, 2),
        40,
    );

    // Down -> Init
    neighbor.set_state(NeighborState::Init);
    assert_eq!(neighbor.state, NeighborState::Init);
    assert!(!neighbor.state.is_two_way_established());

    // Init -> TwoWay
    neighbor.set_state(NeighborState::TwoWay);
    assert_eq!(neighbor.state, NeighborState::TwoWay);
    assert!(neighbor.state.is_two_way_established());

    // TwoWay -> ExStart
    neighbor.set_state(NeighborState::ExStart);
    assert!(neighbor.state.is_two_way_established());

    // ExStart -> Exchange
    neighbor.set_state(NeighborState::Exchange);
    assert!(neighbor.state.is_two_way_established());

    // Exchange -> Loading
    neighbor.set_state(NeighborState::Loading);
    assert!(neighbor.state.is_two_way_established());

    // Loading -> Full
    neighbor.set_state(NeighborState::Full);
    assert_eq!(neighbor.state, NeighborState::Full);
    assert!(neighbor.state.is_two_way_established());
    assert!(neighbor.state.is_adjacency_established());
}

#[test]
#[serial]
fn test_ospf_neighbor_needs_adjacency() {
    let neighbor = OspfNeighbor::new(
        Ipv4Addr::new(2, 2, 2, 2),
        Ipv4Addr::new(192, 168, 1, 2),
        40,
    );

    // DR 和 BDR 之间需要建立邻接关系
    assert!(neighbor.needs_adjacency(true, false, false, true));

    // DR/BDR 与所有非 DR/BDR 路由器建立邻接关系
    assert!(neighbor.needs_adjacency(true, false, false, false));

    // 非 DR/BDR 与 DR 建立邻接关系
    assert!(neighbor.needs_adjacency(false, false, true, false));

    // 两个非 DR/BDR 路由器之间不需要建立邻接关系
    assert!(!neighbor.needs_adjacency(false, false, false, false));
}

#[test]
#[serial]
fn test_ospf_neighbor_dd_sequence_init() {
    let mut neighbor = OspfNeighbor::new(
        Ipv4Addr::new(2, 2, 2, 2),
        Ipv4Addr::new(192, 168, 1, 2),
        40,
    );

    neighbor.init_dd_sequence();

    // DD 序列号应该被初始化为随机值
    assert_ne!(neighbor.dd_seq_number, 0);
}

// ========== OSPF 报文处理结果测试 ==========

#[test]
#[serial]
fn test_ospf_process_result_no_reply() {
    let result = OspfProcessResult::NoReply;
    match result {
        OspfProcessResult::NoReply => {},
        _ => panic!("Expected NoReply"),
    }
}

#[test]
#[serial]
fn test_ospf_process_result_reply() {
    let reply_data = vec![0x01, 0x02, 0x03, 0x04];
    let result = OspfProcessResult::Reply(reply_data.clone());

    match result {
        OspfProcessResult::Reply(data) => {
            assert_eq!(data, reply_data);
        },
        _ => panic!("Expected Reply"),
    }
}

#[test]
#[serial]
fn test_ospf_process_result_schedule_spf() {
    let result = OspfProcessResult::ScheduleSpfCalculation;
    match result {
        OspfProcessResult::ScheduleSpfCalculation => {},
        _ => panic!("Expected ScheduleSpfCalculation"),
    }
}

// ========== 边界条件测试 ==========

#[test]
#[serial]
fn test_ospf_hello_empty_neighbor_list() {
    let src_mac = MacAddr::new([0x00, 0x11, 0x22, 0x33, 0x44, 0x55]);
    let src_ip = Ipv4Addr::new(192, 168, 1, 100);

    // 空邻居列表
    let packet = create_ospf_hello_packet(
        src_mac,
        src_ip,
        Ipv4Addr::new(1, 1, 1, 1),
        Ipv4Addr::new(255, 255, 255, 0),
        10,
        40,
        vec![],
    );

    // 至少应该有以太网 + IP + OSPF 头部 + Hello 基础部分
    assert!(packet.len() >= 14 + 20 + 24 + 20);
}

#[test]
#[serial]
fn test_ospf_hello_max_neighbors() {
    let src_mac = MacAddr::new([0x00, 0x11, 0x22, 0x33, 0x44, 0x55]);
    let src_ip = Ipv4Addr::new(192, 168, 1, 100);

    // 创建大量邻居
    let neighbors: Vec<Ipv4Addr> = (0..100)
        .map(|i| Ipv4Addr::new(192, 168, 1, i as u8))
        .collect();

    let packet = create_ospf_hello_packet(
        src_mac,
        src_ip,
        Ipv4Addr::new(1, 1, 1, 1),
        Ipv4Addr::new(255, 255, 255, 0),
        10,
        40,
        neighbors,
    );

    // 验证报文大小合理增长
    assert!(packet.len() >= 14 + 20 + 24 + 20 + 100 * 4);
}

#[test]
#[serial]
fn test_ospf_interface_timers() {
    let mut iface = OspfInterface::new(
        "eth0".to_string(),
        1,
        Ipv4Addr::new(192, 168, 1, 100),
        Ipv4Addr::new(255, 255, 255, 0),
        Ipv4Addr::new(0, 0, 0, 0),
    );

    iface.up();

    // Hello 定时器应该被设置
    assert!(iface.hello_timer.is_some());

    // 重置 Hello 定时器
    iface.reset_hello_timer();
    assert!(iface.hello_timer.is_some());

    // 接口关闭后定时器应该被清除
    iface.down();
    assert!(iface.hello_timer.is_none());
}

#[test]
#[serial]
fn test_ospf_interface_priority() {
    let mut iface = OspfInterface::new(
        "eth0".to_string(),
        1,
        Ipv4Addr::new(192, 168, 1, 100),
        Ipv4Addr::new(255, 255, 255, 0),
        Ipv4Addr::new(0, 0, 0, 0),
    );

    // 默认优先级为 1
    assert_eq!(iface.priority, 1);

    // 设置高优先级
    iface.priority = 255;
    assert_eq!(iface.priority, 255);
    assert!(iface.is_eligible_for_dr());

    // 优先级为 0 不参与 DR 选举
    iface.priority = 0;
    assert!(!iface.is_eligible_for_dr());
}

#[test]
#[serial]
fn test_ospf_interface_dr_bdr() {
    let mut iface = OspfInterface::new(
        "eth0".to_string(),
        1,
        Ipv4Addr::new(192, 168, 1, 100),
        Ipv4Addr::new(255, 255, 255, 0),
        Ipv4Addr::new(0, 0, 0, 0),
    );

    let router_id = Ipv4Addr::new(1, 1, 1, 1);

    // 设置 DR 和 BDR
    iface.set_dr(router_id);
    iface.set_bdr(Ipv4Addr::new(2, 2, 2, 2));

    assert_eq!(iface.dr, router_id);
    assert!(iface.is_dr(router_id));
    assert_eq!(iface.bdr, Ipv4Addr::new(2, 2, 2, 2));
    assert!(iface.is_bdr(Ipv4Addr::new(2, 2, 2, 2)));
}
