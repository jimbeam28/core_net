// Scheduler 模块集成测试

use core_net::common::{MacAddr, Ipv4Addr};
use core_net::engine::PacketProcessor;
use core_net::interface::InterfaceManager;
use core_net::poweron::{boot_default, shutdown};
use core_net::scheduler::Scheduler;
use core_net::context::SystemContext;
use serial_test::serial;

mod common;
use common::{
    create_test_eth0_config, create_test_lo_config, create_test_packet,
    create_arp_request_packet, count_all_rxq_packets, count_all_txq_packets,
};

/// 创建多接口管理器
fn create_multi_interface_manager() -> InterfaceManager {
    let mut manager = InterfaceManager::new(256, 256);
    manager.add_from_config(create_test_eth0_config()).unwrap();
    manager.add_from_config(create_test_lo_config()).unwrap();
    manager
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
        let packet = create_arp_request_packet(src_mac, src_ip, dst_ip);
        let _ = iface.rxq.enqueue(packet);
    }
}

// 场景一：完整的报文处理流程

#[test]
#[serial]
fn test_full_packet_processing_flow() {
    let mut manager = create_multi_interface_manager();

    inject_packets_to_rxq(&mut manager, "eth0", 5);
    inject_packets_to_rxq(&mut manager, "lo", 3);

    assert_eq!(count_all_rxq_packets(&manager), 8);

    let scheduler = Scheduler::new("IntegrationTestScheduler".to_string())
        .with_processor(PacketProcessor::with_context(SystemContext::new()));

    let result = scheduler.run_all_interfaces(&mut manager);
    assert!(result.is_ok());

    assert_eq!(count_all_rxq_packets(&manager), 0, "所有报文应被处理");

    let _processed_count = result.unwrap();
}

#[test]
#[serial]
fn test_single_interface_full_flow() {
    let mut manager = InterfaceManager::new(256, 256);
    manager.add_from_config(create_test_eth0_config()).unwrap();

    inject_packets_to_rxq(&mut manager, "eth0", 10);

    let scheduler = Scheduler::new("SingleInterfaceTestScheduler".to_string())
        .with_processor(PacketProcessor::with_context(SystemContext::new()));

    let result = scheduler.run_all_interfaces(&mut manager);
    assert!(result.is_ok());
    assert_eq!(count_all_rxq_packets(&manager), 0);
}

#[test]
#[serial]
fn test_arp_response_flow() {
    let mut manager = create_multi_interface_manager();

    let src_mac = MacAddr::new([0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF]);
    let src_ip = Ipv4Addr::new(192, 168, 1, 1);
    let dst_ip = Ipv4Addr::new(192, 168, 1, 100);
    inject_arp_to_rxq(&mut manager, "eth0", src_mac, src_ip, dst_ip);

    let initial_rxq_count = count_all_rxq_packets(&manager);
    assert_eq!(initial_rxq_count, 1);

    let scheduler = Scheduler::new("ArpFlowTestScheduler".to_string())
        .with_processor(PacketProcessor::with_context(SystemContext::new()));

    let result = scheduler.run_all_interfaces(&mut manager);
    assert!(result.is_ok());

    assert_eq!(count_all_rxq_packets(&manager), 0);

    let _txq_count = count_all_txq_packets(&manager);
}

// 场景二：多接口负载均衡

#[test]
#[serial]
fn test_multi_interface_load_balancing() {
    let mut manager = create_multi_interface_manager();

    inject_packets_to_rxq(&mut manager, "eth0", 10);
    inject_packets_to_rxq(&mut manager, "lo", 5);

    assert_eq!(count_all_rxq_packets(&manager), 15);

    let scheduler = Scheduler::new("LoadBalancingTestScheduler".to_string())
        .with_processor(PacketProcessor::with_context(SystemContext::new()));

    let result = scheduler.run_all_interfaces(&mut manager);
    assert!(result.is_ok());

    let eth0 = manager.get_by_name("eth0").unwrap();
    let lo = manager.get_by_name("lo").unwrap();
    assert!(eth0.rxq.is_empty(), "eth0 的 RxQ 应被清空");
    assert!(lo.rxq.is_empty(), "lo 的 RxQ 应被清空");
}

#[test]
#[serial]
fn test_multi_interface_asymmetric_load() {
    let mut manager = create_multi_interface_manager();

    inject_packets_to_rxq(&mut manager, "eth0", 100);
    inject_packets_to_rxq(&mut manager, "lo", 5);

    let scheduler = Scheduler::new("AsymmetricLoadTestScheduler".to_string())
        .with_processor(PacketProcessor::with_context(SystemContext::new()));

    let result = scheduler.run_all_interfaces(&mut manager);
    assert!(result.is_ok());

    assert_eq!(count_all_rxq_packets(&manager), 0);
}

#[test]
#[serial]
fn test_multi_interface_empty_interfaces() {
    let mut manager = create_multi_interface_manager();

    inject_packets_to_rxq(&mut manager, "eth0", 7);

    let scheduler = Scheduler::new("PartialEmptyTestScheduler".to_string())
        .with_processor(PacketProcessor::with_context(SystemContext::new()));

    let result = scheduler.run_all_interfaces(&mut manager);
    assert!(result.is_ok());

    assert_eq!(count_all_rxq_packets(&manager), 0);
}

#[test]
#[serial]
fn test_multi_interface_all_empty() {
    let mut manager = create_multi_interface_manager();

    let scheduler = Scheduler::new("AllEmptyTestScheduler".to_string())
        .with_processor(PacketProcessor::with_context(SystemContext::new()));

    let result = scheduler.run_all_interfaces(&mut manager);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 0);
}

// 场景三：系统上电后的调度

#[test]
#[serial]
fn test_boot_schedule_shutdown_cycle() {
    let context = boot_default();

    assert!(context.interface_count() > 0, "系统应启动并加载接口");

    let scheduler = Scheduler::new("BootCycleTestScheduler".to_string())
        .with_processor(PacketProcessor::with_context(context.clone()));

    // 注入报文到 eth0
    {
        let mut guard = context.interfaces.lock().unwrap();
        if let Ok(iface) = guard.get_by_name_mut("eth0") {
            for i in 0..5 {
                let packet = create_test_packet(vec![0x01, 0x02, 0x03, 0x04, i as u8]);
                let _ = iface.rxq.enqueue(packet);
            }
        }
    }

    // 注入报文到 lo
    {
        let mut guard = context.interfaces.lock().unwrap();
        if let Ok(iface) = guard.get_by_name_mut("lo") {
            for i in 0..3 {
                let packet = create_test_packet(vec![0x05, 0x06, 0x07, 0x08, i as u8]);
                let _ = iface.rxq.enqueue(packet);
            }
        }
    }

    // 使用 run_all_interfaces_context 避免死锁
    let result = scheduler.run_all_interfaces_context(&context);
    assert!(result.is_ok(), "调度应成功");

    {
        let guard = context.interfaces.lock().unwrap();
        for iface in guard.interfaces() {
            assert!(iface.rxq.is_empty(), "所有接口的 RxQ 应被清空");
        }
    }

    shutdown(&context);

    {
        let guard = context.interfaces.lock().unwrap();
        for iface in guard.interfaces() {
            assert!(iface.rxq.is_empty(), "shutdown 后 RxQ 应为空");
            assert!(iface.txq.is_empty(), "shutdown 后 TxQ 应为空");
        }
    }
}

#[test]
#[serial]
fn test_boot_with_arp_schedule_shutdown() {
    let context = boot_default();

    let scheduler = Scheduler::new("BootArpTestScheduler".to_string())
        .with_processor(PacketProcessor::with_context(context.clone()));

    {
        let mut guard = context.interfaces.lock().unwrap();
        if let Ok(iface) = guard.get_by_name_mut("eth0") {
            let src_mac = MacAddr::new([0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF]);
            let src_ip = Ipv4Addr::new(192, 168, 1, 1);
            let dst_ip = Ipv4Addr::new(192, 168, 1, 100);
            let arp_packet = create_arp_request_packet(src_mac, src_ip, dst_ip);
            let _ = iface.rxq.enqueue(arp_packet);
        }
    }

    // 使用 run_all_interfaces_context 避免死锁
    let result = scheduler.run_all_interfaces_context(&context);
    assert!(result.is_ok());

    shutdown(&context);
}

#[test]
#[serial]
fn test_multiple_boot_schedule_cycles() {
    for i in 0..3 {
        let context = boot_default();
        let scheduler = Scheduler::new(format!("Cycle{}Scheduler", i))
            .with_processor(PacketProcessor::with_context(context.clone()));

        {
            let mut guard = context.interfaces.lock().unwrap();
            if let Ok(iface) = guard.get_by_name_mut("eth0") {
                for j in 0..(i + 1) {
                    let packet = create_test_packet(vec![0x01, 0x02, 0x03, j as u8]);
                    let _ = iface.rxq.enqueue(packet);
                }
            }
        }

        // 使用 run_all_interfaces_context 避免死锁
        let result = scheduler.run_all_interfaces_context(&context);
        assert!(result.is_ok());

        shutdown(&context);
    }
}

// 额外集成测试

#[test]
#[serial]
fn test_scheduler_verbose_mode_integration() {
    let mut manager = create_multi_interface_manager();

    let src_mac = MacAddr::new([0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF]);
    let src_ip = Ipv4Addr::new(192, 168, 1, 1);
    let dst_ip = Ipv4Addr::new(192, 168, 1, 2);

    for _ in 0..3 {
        inject_arp_to_rxq(&mut manager, "eth0", src_mac, src_ip, dst_ip);
    }

    for _ in 0..2 {
        inject_arp_to_rxq(&mut manager, "lo", src_mac, src_ip, dst_ip);
    }

    let scheduler = Scheduler::new("VerboseIntegrationScheduler".to_string())
        .with_processor(PacketProcessor::with_context(SystemContext::new()))
        .with_verbose(true);

    let result = scheduler.run_all_interfaces(&mut manager);
    assert!(result.is_ok());

    assert_eq!(count_all_rxq_packets(&manager), 0);
}

#[test]
#[serial]
fn test_scheduler_error_tolerance_integration() {
    let mut manager = create_multi_interface_manager();

    inject_packets_to_rxq(&mut manager, "eth0", 5);

    if let Ok(iface) = manager.get_by_name_mut("eth0") {
        for _ in 0..3 {
            let invalid_packet = create_test_packet(vec![0x01, 0x02]);
            let _ = iface.rxq.enqueue(invalid_packet);
        }
    }

    inject_packets_to_rxq(&mut manager, "lo", 4);

    let scheduler = Scheduler::new("ErrorToleranceScheduler".to_string())
        .with_processor(PacketProcessor::with_context(SystemContext::new()));

    let result = scheduler.run_all_interfaces(&mut manager);
    assert!(result.is_ok());

    assert_eq!(count_all_rxq_packets(&manager), 0);
}

#[test]
#[serial]
fn test_single_queue_mode_integration() {
    let manager = create_multi_interface_manager();

    let (rxq_cap, txq_cap) = {
        let iface = manager.get_by_name("eth0").unwrap();
        (iface.rxq.capacity(), iface.txq.capacity())
    };

    let mut rxq = core_net::common::queue::RingQueue::new(rxq_cap);
    let mut txq = core_net::common::queue::RingQueue::new(txq_cap);

    for i in 0..5 {
        let packet = create_test_packet(vec![0x01, 0x02, 0x03, 0x04, i as u8]);
        rxq.enqueue(packet).unwrap();
    }

    let scheduler = Scheduler::new("SingleQueueIntegrationScheduler".to_string())
        .with_processor(PacketProcessor::with_context(SystemContext::new()));

    let result = scheduler.run(&mut rxq, &mut txq);
    assert!(result.is_ok());
    assert!(rxq.is_empty());
}
