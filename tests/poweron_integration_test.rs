// Poweron 模块集成测试

use core_net::poweron::shutdown;
use core_net::context::SystemContext;
use core_net::interface::{InterfaceState, Ipv4Addr, MacAddr};
use serial_test::serial;

mod common;
use common::create_test_packet;

/// 向接口的 RxQ 注入报文
fn inject_packets(context: &SystemContext, iface_name: &str, count: usize) {
    let mut guard = match context.interfaces.lock() {
        Ok(g) => g,
        Err(_) => return,
    };

    if let Ok(iface) = guard.get_by_name_mut(iface_name) {
        for i in 0..count {
            let packet = create_test_packet(vec![i as u8; 64]);
            let _ = iface.rxq.enqueue(packet);
        }
    }
}

/// 统计所有接口的队列报文数量
fn count_all_packets(context: &SystemContext) -> usize {
    let guard = match context.interfaces.lock() {
        Ok(g) => g,
        Err(_) => return 0,
    };

    let mut count = 0;
    for iface in guard.interfaces() {
        count += iface.rxq.len();
        count += iface.txq.len();
    }
    count
}

// 场景一：完整的系统生命周期

#[test]
#[serial]
fn test_complete_lifecycle() {
    let context = SystemContext::from_config();

    assert!(context.interface_count() > 0, "系统应该至少有一个接口");

    let guard = context.interfaces.lock().unwrap();
    for iface in guard.interfaces() {
        assert!(!iface.name.is_empty(), "接口名称不应为空");
        assert!(iface.rxq.capacity() > 0, "接收队列容量应大于0");
        assert!(iface.txq.capacity() > 0, "发送队列容量应大于0");
        assert!(iface.rxq.is_empty(), "初始接收队列应为空");
        assert!(iface.txq.is_empty(), "初始发送队列应为空");
    }
    drop(guard);

    if let Ok(eth0) = context.interfaces.lock().unwrap().get_by_name("eth0") {
        assert_eq!(eth0.rxq.capacity(), 256);
        assert_eq!(eth0.txq.capacity(), 256);
    }

    if context.interfaces.lock().unwrap().get_by_name("eth0").is_ok() {
        inject_packets(&context, "eth0", 10);
        assert_eq!(count_all_packets(&context), 10, "注入10个报文后应有10个报文");
    }

    shutdown(&context);
    assert_eq!(count_all_packets(&context), 0, "下电后应清理所有队列");
}

#[test]
#[serial]
fn test_from_config_creates_context() {
    let context = SystemContext::from_config();
    assert!(context.interface_count() > 0);
}

#[test]
#[serial]
fn test_shutdown_clears_queues() {
    let context = SystemContext::from_config();
    inject_packets(&context, "eth0", 5);
    assert!(count_all_packets(&context) > 0);
    shutdown(&context);
    assert_eq!(count_all_packets(&context), 0);
}

#[test]
#[serial]
fn test_multiple_boot_shutdown_cycles() {
    for i in 0..3 {
        let context = SystemContext::from_config();
        assert!(context.interface_count() > 0, "第 {} 次上电应该成功", i + 1);
        shutdown(&context);
        assert_eq!(count_all_packets(&context), 0, "第 {} 次下电应清理所有队列", i + 1);
    }
}

// 场景二：SystemContext 接口操作

#[test]
#[serial]
fn test_context_get_interface() {
    let context = SystemContext::from_config();

    let guard = context.interfaces.lock().unwrap();
    let eth0 = guard.get_by_name("eth0");
    assert!(eth0.is_ok(), "应该能获取到 eth0 接口");

    if let Ok(eth0) = eth0 {
        assert_eq!(eth0.name, "eth0");
        assert!(eth0.rxq.capacity() > 0);
    }

    let nonexistent = guard.get_by_name("nonexistent");
    assert!(nonexistent.is_err(), "不存在的接口应返回 Err");
}

#[test]
#[serial]
fn test_context_get_interface_mut() {
    let context = SystemContext::from_config();

    let mut guard = context.interfaces.lock().unwrap();
    if let Ok(iface) = guard.get_by_name_mut("eth0") {
        let original_ip = iface.ip_addr;
        iface.set_ip_addr(Ipv4Addr::new(10, 0, 0, 1));

        // 验证修改生效
        assert_eq!(iface.ip_addr, Ipv4Addr::new(10, 0, 0, 1));

        // 恢复原始值
        iface.set_ip_addr(original_ip);
    }
}

#[test]
#[serial]
fn test_context_interface_up_down() {
    let context = SystemContext::from_config();

    let mut guard = context.interfaces.lock().unwrap();
    if let Ok(iface) = guard.get_by_name_mut("eth0") {
        iface.down();
        assert_eq!(iface.state, InterfaceState::Down);

        iface.up();
        assert_eq!(iface.state, InterfaceState::Up);
    }
}

#[test]
#[serial]
fn test_context_multiple_modifications() {
    let context = SystemContext::from_config();

    let (original_ip, original_netmask, original_mtu) = {
        let guard = context.interfaces.lock().unwrap();
        if let Ok(eth0) = guard.get_by_name("eth0") {
            (eth0.ip_addr, eth0.netmask, eth0.mtu)
        } else {
            return;
        }
    };

    {
        let mut guard = context.interfaces.lock().unwrap();
        if let Ok(iface) = guard.get_by_name_mut("eth0") {
            iface.set_ip_addr(Ipv4Addr::new(10, 0, 0, 1));
            iface.set_netmask(Ipv4Addr::new(255, 255, 255, 128));
            iface.set_mtu(9000);
        }
    }

    // 验证修改
    {
        let guard = context.interfaces.lock().unwrap();
        if let Ok(eth0) = guard.get_by_name("eth0") {
            assert_eq!(eth0.ip_addr, Ipv4Addr::new(10, 0, 0, 1));
            assert_eq!(eth0.netmask, Ipv4Addr::new(255, 255, 255, 128));
            assert_eq!(eth0.mtu, 9000);
        }
    }

    // 恢复原始值
    {
        let mut guard = context.interfaces.lock().unwrap();
        if let Ok(iface) = guard.get_by_name_mut("eth0") {
            iface.set_ip_addr(original_ip);
            iface.set_netmask(original_netmask);
            iface.set_mtu(original_mtu);
        }
    }

    // 验证恢复
    {
        let guard = context.interfaces.lock().unwrap();
        if let Ok(eth0) = guard.get_by_name("eth0") {
            assert_eq!(eth0.ip_addr, original_ip);
            assert_eq!(eth0.netmask, original_netmask);
            assert_eq!(eth0.mtu, original_mtu);
        }
    }
}

#[test]
#[serial]
fn test_context_interface_properties() {
    let context = SystemContext::from_config();

    let guard = context.interfaces.lock().unwrap();

    if let Ok(eth0) = guard.get_by_name("eth0") {
        assert_eq!(eth0.name, "eth0");
        assert_eq!(eth0.mac_addr, MacAddr::new([0x00, 0x11, 0x22, 0x33, 0x44, 0x55]));
        assert_eq!(eth0.ip_addr, Ipv4Addr::new(192, 168, 1, 100));
        assert_eq!(eth0.netmask, Ipv4Addr::new(255, 255, 255, 0));
        assert_eq!(eth0.mtu, 1500);
        assert_eq!(eth0.state, InterfaceState::Up);
    }

    if let Ok(lo) = guard.get_by_name("lo") {
        assert_eq!(lo.name, "lo");
        assert_eq!(lo.mac_addr, MacAddr::zero());
        assert_eq!(lo.ip_addr, Ipv4Addr::new(127, 0, 0, 1));
    }
}

#[test]
#[serial]
fn test_context_get_interface_by_index() {
    let context = SystemContext::from_config();

    let guard = context.interfaces.lock().unwrap();
    let iface0 = guard.get_by_index(0);
    assert!(iface0.is_ok(), "应该能通过索引0获取接口");

    if let Ok(iface) = iface0 {
        assert_eq!(iface.index, 0);
    }

    let out_of_range = guard.get_by_index(999);
    assert!(out_of_range.is_err(), "越界索引应返回 Err");
}

#[test]
#[serial]
fn test_context_queue_operations() {
    let context = SystemContext::from_config();

    // 初始队列为空
    {
        let guard = context.interfaces.lock().unwrap();
        if let Ok(eth0) = guard.get_by_name("eth0") {
            assert!(eth0.rxq.is_empty());
            assert!(eth0.txq.is_empty());
        }
    }

    // 注入报文
    inject_packets(&context, "eth0", 5);

    // 验证报文已注入
    {
        let guard = context.interfaces.lock().unwrap();
        if let Ok(eth0) = guard.get_by_name("eth0") {
            assert_eq!(eth0.rxq.len(), 5);
        }
    }

    // 下电清空
    shutdown(&context);

    // 验证已清空
    {
        let guard = context.interfaces.lock().unwrap();
        if let Ok(eth0) = guard.get_by_name("eth0") {
            assert!(eth0.rxq.is_empty());
            assert!(eth0.txq.is_empty());
        }
    }
}
