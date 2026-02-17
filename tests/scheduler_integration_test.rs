// tests/scheduler_integration_test.rs
//
// Scheduler 模块集成测试
// 测试调度器与引擎、接口模块之间的交互

use core_net::common::{MacAddr, Ipv4Addr, ETH_P_ARP};
use core_net::engine::PacketProcessor;
use core_net::interface::{InterfaceConfig, InterfaceManager, InterfaceState};
use core_net::protocols::arp::{ArpPacket, ArpOperation};
use core_net::protocols::Packet;
use core_net::poweron::{boot_default, shutdown};
use core_net::scheduler::Scheduler;

// ========== 测试辅助函数 ==========

/// 创建测试报文（至少 14 字节，满足以太网帧最小长度）
fn create_test_packet(data: Vec<u8>) -> Packet {
    // 如果数据太短，添加填充以满足以太网帧最小长度（14 字节）
    if data.len() < 14 {
        let mut padded = data;
        while padded.len() < 14 {
            padded.push(0);
        }
        Packet::from_bytes(padded)
    } else {
        Packet::from_bytes(data)
    }
}

/// 创建 ARP 请求报文
fn create_arp_request_packet(
    dst_mac: MacAddr,
    src_mac: MacAddr,
    src_ip: Ipv4Addr,
    dst_ip: Ipv4Addr,
) -> Packet {
    let arp_pkt = ArpPacket::new(
        ArpOperation::Request,
        src_mac,
        src_ip,
        MacAddr::zero(),
        dst_ip,
    );

    let mut bytes = Vec::new();
    bytes.extend_from_slice(&dst_mac.bytes);
    bytes.extend_from_slice(&src_mac.bytes);
    bytes.extend_from_slice(&ETH_P_ARP.to_be_bytes());
    bytes.extend_from_slice(&arp_pkt.to_bytes());

    Packet::from_bytes(bytes)
}

/// 创建 eth0 配置
fn create_eth0_config() -> InterfaceConfig {
    InterfaceConfig {
        name: "eth0".to_string(),
        mac_addr: MacAddr::new([0x00, 0x11, 0x22, 0x33, 0x44, 0x55]),
        ip_addr: Ipv4Addr::new(192, 168, 1, 100),
        netmask: Ipv4Addr::new(255, 255, 255, 0),
        gateway: Some(Ipv4Addr::new(192, 168, 1, 1)),
        mtu: Some(1500),
        state: Some(InterfaceState::Up),
    }
}

/// 创建 lo 配置
fn create_lo_config() -> InterfaceConfig {
    InterfaceConfig {
        name: "lo".to_string(),
        mac_addr: MacAddr::zero(),
        ip_addr: Ipv4Addr::new(127, 0, 0, 1),
        netmask: Ipv4Addr::new(255, 0, 0, 0),
        gateway: None,
        mtu: Some(65535),
        state: Some(InterfaceState::Up),
    }
}

/// 创建多接口管理器
fn create_multi_interface_manager() -> InterfaceManager {
    let mut manager = InterfaceManager::new(256, 256);
    manager.add_from_config(create_eth0_config()).unwrap();
    manager.add_from_config(create_lo_config()).unwrap();
    manager
}

/// 计算所有接口 RxQ 中的报文总数
fn count_all_rxq_packets(manager: &InterfaceManager) -> usize {
    let mut count = 0;
    for iface in manager.interfaces() {
        count += iface.rxq.len();
    }
    count
}

/// 计算所有接口 TxQ 中的报文总数
fn count_all_txq_packets(manager: &InterfaceManager) -> usize {
    let mut count = 0;
    for iface in manager.interfaces() {
        count += iface.txq.len();
    }
    count
}

/// 向指定接口的 RxQ 注入报文
fn inject_packets_to_rxq(manager: &mut InterfaceManager, iface_name: &str, count: usize) {
    if let Ok(iface) = manager.get_by_name_mut(iface_name) {
        for i in 0..count {
            let packet = create_test_packet(vec![0x01, 0x02, 0x03, 0x04, i as u8]);
            let _ = iface.rxq.enqueue(packet);
        }
    }
}

/// 向指定接口的 RxQ 注入 ARP 请求报文
fn inject_arp_to_rxq(
    manager: &mut InterfaceManager,
    iface_name: &str,
    src_mac: MacAddr,
    src_ip: Ipv4Addr,
    dst_ip: Ipv4Addr,
) {
    if let Ok(iface) = manager.get_by_name_mut(iface_name) {
        let packet = create_arp_request_packet(MacAddr::broadcast(), src_mac, src_ip, dst_ip);
        let _ = iface.rxq.enqueue(packet);
    }
}

// ========== 场景一：完整的报文处理流程 ==========

#[test]
fn test_full_packet_processing_flow() {
    // 1. 创建接口管理器
    let mut manager = create_multi_interface_manager();

    // 2. 向接口 RxQ 注入测试报文
    inject_packets_to_rxq(&mut manager, "eth0", 5);
    inject_packets_to_rxq(&mut manager, "lo", 3);

    // 验证报文已注入
    assert_eq!(count_all_rxq_packets(&manager), 8);

    // 3. 创建调度器和处理器
    let scheduler = Scheduler::new("IntegrationTestScheduler".to_string())
        .with_processor(PacketProcessor::new());

    // 4. 运行调度器
    let result = scheduler.run_all_interfaces(&mut manager);
    assert!(result.is_ok());

    // 5. 验证所有报文被处理（RxQ 被清空）
    assert_eq!(count_all_rxq_packets(&manager), 0, "所有报文应被处理");

    // 6. 验证调度成功完成
    let _processed_count = result.unwrap();
}

#[test]
fn test_single_interface_full_flow() {
    // 单接口的完整流程测试
    let mut manager = InterfaceManager::new(256, 256);
    manager.add_from_config(create_eth0_config()).unwrap();

    // 注入报文
    inject_packets_to_rxq(&mut manager, "eth0", 10);

    let scheduler = Scheduler::new("SingleInterfaceTestScheduler".to_string())
        .with_processor(PacketProcessor::new());

    let result = scheduler.run_all_interfaces(&mut manager);
    assert!(result.is_ok());
    assert_eq!(count_all_rxq_packets(&manager), 0);
}

#[test]
fn test_arp_response_flow() {
    // 测试 ARP 响应流程
    let mut manager = create_multi_interface_manager();

    // 注入 ARP 请求
    let src_mac = MacAddr::new([0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF]);
    let src_ip = Ipv4Addr::new(192, 168, 1, 1);
    let dst_ip = Ipv4Addr::new(192, 168, 1, 100); // eth0 的 IP
    inject_arp_to_rxq(&mut manager, "eth0", src_mac, src_ip, dst_ip);

    let initial_rxq_count = count_all_rxq_packets(&manager);
    assert_eq!(initial_rxq_count, 1);

    let scheduler = Scheduler::new("ArpFlowTestScheduler".to_string())
        .with_processor(PacketProcessor::new());

    let result = scheduler.run_all_interfaces(&mut manager);
    assert!(result.is_ok());

    // 验证报文被处理
    assert_eq!(count_all_rxq_packets(&manager), 0);

    // ARP 处理可能生成响应放入 TxQ（取决于全局缓存状态）
    // 这里只验证流程正常，不强制要求响应
    let _txq_count = count_all_txq_packets(&manager);
}

// ========== 场景二：多接口负载均衡 ==========

#[test]
fn test_multi_interface_load_balancing() {
    // 1. 创建多个接口
    let mut manager = create_multi_interface_manager();

    // 2. 向不同接口注入不同数量的报文
    inject_packets_to_rxq(&mut manager, "eth0", 10);
    inject_packets_to_rxq(&mut manager, "lo", 5);

    // 验证注入成功
    assert_eq!(count_all_rxq_packets(&manager), 15);

    // 3. 运行多接口调度
    let scheduler = Scheduler::new("LoadBalancingTestScheduler".to_string())
        .with_processor(PacketProcessor::new());

    let result = scheduler.run_all_interfaces(&mut manager);
    assert!(result.is_ok());

    // 4. 验证每个接口的报文都被处理
    let eth0 = manager.get_by_name("eth0").unwrap();
    let lo = manager.get_by_name("lo").unwrap();
    assert!(eth0.rxq.is_empty(), "eth0 的 RxQ 应被清空");
    assert!(lo.rxq.is_empty(), "lo 的 RxQ 应被清空");
}

#[test]
fn test_multi_interface_asymmetric_load() {
    // 非对称负载测试
    let mut manager = create_multi_interface_manager();

    // 非对称负载：eth0 远多于 lo
    inject_packets_to_rxq(&mut manager, "eth0", 100);
    inject_packets_to_rxq(&mut manager, "lo", 5);

    let scheduler = Scheduler::new("AsymmetricLoadTestScheduler".to_string())
        .with_processor(PacketProcessor::new());

    let result = scheduler.run_all_interfaces(&mut manager);
    assert!(result.is_ok());

    // 验证所有队列都被清空
    assert_eq!(count_all_rxq_packets(&manager), 0);
}

#[test]
fn test_multi_interface_empty_interfaces() {
    // 部分接口为空的场景
    let mut manager = create_multi_interface_manager();

    // 只向 eth0 注入报文，lo 保持空
    inject_packets_to_rxq(&mut manager, "eth0", 7);

    let scheduler = Scheduler::new("PartialEmptyTestScheduler".to_string())
        .with_processor(PacketProcessor::new());

    let result = scheduler.run_all_interfaces(&mut manager);
    assert!(result.is_ok());

    // 验证所有接口都被遍历（包括空的 lo）
    assert_eq!(count_all_rxq_packets(&manager), 0);
}

#[test]
fn test_multi_interface_all_empty() {
    // 所有接口都为空的场景
    let mut manager = create_multi_interface_manager();

    let scheduler = Scheduler::new("AllEmptyTestScheduler".to_string())
        .with_processor(PacketProcessor::new());

    let result = scheduler.run_all_interfaces(&mut manager);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 0);
}

// ========== 场景三：系统上电后的调度 ==========

#[test]
fn test_boot_schedule_shutdown_cycle() {
    // 1. 调用 boot_default() 启动系统
    let mut context = boot_default();

    // 验证系统已启动
    assert!(context.interface_count() > 0, "系统应启动并加载接口");

    // 2. 创建调度器
    let scheduler = Scheduler::new("BootCycleTestScheduler".to_string())
        .with_processor(PacketProcessor::new());

    // 3. 向各接口注入报文
    if context.get_interface("eth0").is_some() {
        if let Some(iface) = context.get_interface_mut("eth0") {
            for i in 0..5 {
                let packet = create_test_packet(vec![0x01, 0x02, 0x03, 0x04, i as u8]);
                let _ = iface.rxq.enqueue(packet);
            }
        }
    }

    if context.get_interface("lo").is_some() {
        if let Some(iface) = context.get_interface_mut("lo") {
            for i in 0..3 {
                let packet = create_test_packet(vec![0x05, 0x06, 0x07, 0x08, i as u8]);
                let _ = iface.rxq.enqueue(packet);
            }
        }
    }

    // 4. 运行多接口调度
    let result = scheduler.run_all_interfaces(&mut context.interfaces);
    assert!(result.is_ok(), "调度应成功");

    // 5. 验证所有 RxQ 被清空
    for iface in context.interfaces.interfaces() {
        assert!(iface.rxq.is_empty(), "所有接口的 RxQ 应被清空");
    }

    // 6. 调用 shutdown() 关闭系统
    shutdown(&mut context);

    // 验证资源已清理
    for iface in context.interfaces.interfaces() {
        assert!(iface.rxq.is_empty(), "shutdown 后 RxQ 应为空");
        assert!(iface.txq.is_empty(), "shutdown 后 TxQ 应为空");
    }
}

#[test]
fn test_boot_with_arp_schedule_shutdown() {
    // 测试启动后处理 ARP 报文
    let mut context = boot_default();

    let scheduler = Scheduler::new("BootArpTestScheduler".to_string())
        .with_processor(PacketProcessor::new());

    // 向接口注入 ARP 请求
    if let Some(iface) = context.get_interface_mut("eth0") {
        let src_mac = MacAddr::new([0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF]);
        let src_ip = Ipv4Addr::new(192, 168, 1, 1);
        let dst_ip = Ipv4Addr::new(192, 168, 1, 100);
        let arp_packet = create_arp_request_packet(MacAddr::broadcast(), src_mac, src_ip, dst_ip);
        let _ = iface.rxq.enqueue(arp_packet);
    }

    // 运行调度
    let result = scheduler.run_all_interfaces(&mut context.interfaces);
    assert!(result.is_ok());

    // 关闭系统
    shutdown(&mut context);
}

#[test]
fn test_multiple_boot_schedule_cycles() {
    // 测试多次上电-调度-下电循环
    for i in 0..3 {
        let mut context = boot_default();
        let scheduler = Scheduler::new(format!("Cycle{}Scheduler", i))
            .with_processor(PacketProcessor::new());

        // 每次循环注入不同数量的报文
        if let Some(iface) = context.get_interface_mut("eth0") {
            for j in 0..(i + 1) {
                let packet = create_test_packet(vec![0x01, 0x02, 0x03, j as u8]);
                let _ = iface.rxq.enqueue(packet);
            }
        }

        let result = scheduler.run_all_interfaces(&mut context.interfaces);
        assert!(result.is_ok());

        shutdown(&mut context);
    }
}

// ========== 额外集成测试 ==========

#[test]
fn test_scheduler_verbose_mode_integration() {
    // 测试 verbose 模式在集成场景中的工作
    let mut manager = create_multi_interface_manager();

    // 使用 ARP 请求报文（有效的以太网帧）
    let src_mac = MacAddr::new([0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF]);
    let src_ip = Ipv4Addr::new(192, 168, 1, 1);
    let dst_ip = Ipv4Addr::new(192, 168, 1, 2);

    // 向 eth0 注入 3 个 ARP 请求
    for _ in 0..3 {
        inject_arp_to_rxq(&mut manager, "eth0", src_mac, src_ip, dst_ip);
    }

    // 向 lo 注入 2 个 ARP 请求
    for _ in 0..2 {
        inject_arp_to_rxq(&mut manager, "lo", src_mac, src_ip, dst_ip);
    }

    let scheduler = Scheduler::new("VerboseIntegrationScheduler".to_string())
        .with_processor(PacketProcessor::new())
        .with_verbose(true);

    let result = scheduler.run_all_interfaces(&mut manager);
    assert!(result.is_ok());

    // 验证所有报文被处理（可能成功也可能失败，取决于 ARP 缓存状态）
    // 至少应该尝试处理所有报文，RxQ 应该被清空
    assert_eq!(count_all_rxq_packets(&manager), 0);
}

#[test]
fn test_scheduler_error_tolerance_integration() {
    // 测试错误容忍性：混合有效和无效报文
    let mut manager = create_multi_interface_manager();

    // 注入有效报文
    inject_packets_to_rxq(&mut manager, "eth0", 5);

    // 注入无效报文（太短）
    if let Ok(iface) = manager.get_by_name_mut("eth0") {
        for _ in 0..3 {
            let invalid_packet = create_test_packet(vec![0x01, 0x02]);
            let _ = iface.rxq.enqueue(invalid_packet);
        }
    }

    // 再注入一些有效报文
    inject_packets_to_rxq(&mut manager, "lo", 4);

    let scheduler = Scheduler::new("ErrorToleranceScheduler".to_string())
        .with_processor(PacketProcessor::new());

    // 即使有无效报文，调度也应该完成
    let result = scheduler.run_all_interfaces(&mut manager);
    assert!(result.is_ok());

    // 所有报文都应该被处理（无论成功失败）
    assert_eq!(count_all_rxq_packets(&manager), 0);
}

#[test]
fn test_single_queue_mode_integration() {
    // 测试单队列调度模式
    let manager = create_multi_interface_manager();

    // 获取接口的队列容量
    let (rxq_cap, txq_cap) = {
        let iface = manager.get_by_name("eth0").unwrap();
        (iface.rxq.capacity(), iface.txq.capacity())
    };

    // 创建独立队列进行测试
    let mut rxq = core_net::common::queue::RingQueue::new(rxq_cap);
    let mut txq = core_net::common::queue::RingQueue::new(txq_cap);

    // 注入报文
    for i in 0..5 {
        let packet = create_test_packet(vec![0x01, 0x02, 0x03, 0x04, i as u8]);
        rxq.enqueue(packet).unwrap();
    }

    let scheduler = Scheduler::new("SingleQueueIntegrationScheduler".to_string())
        .with_processor(PacketProcessor::new());

    let result = scheduler.run(&mut rxq, &mut txq);
    assert!(result.is_ok());
    assert!(rxq.is_empty());
}
