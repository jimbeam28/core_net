// Poweron 模块集成测试
//
// 测试场景：
// - 场景一：完整的系统生命周期
// - 场景二：与全局管理器集成

use core_net::poweron::{boot_default, shutdown, SystemContext};
use core_net::interface::{InterfaceState, Ipv4Addr, MacAddr, update_interface};

mod common;
use common::create_test_packet;

/// 向接口的 RxQ 注入报文
fn inject_packets(context: &mut SystemContext, iface_name: &str, count: usize) {
    if let Some(iface) = context.get_interface_mut(iface_name) {
        for i in 0..count {
            let packet = create_test_packet(vec![i as u8; 64]);
            let _ = iface.rxq.enqueue(packet);
        }
    }
}

/// 统计所有接口的队列报文数量
fn count_all_packets(context: &SystemContext) -> usize {
    let mut count = 0;
    for iface in context.interfaces.interfaces() {
        count += iface.rxq.len();
        count += iface.txq.len();
    }
    count
}

// 场景一：完整的系统生命周期

#[test]
fn test_complete_lifecycle() {
    // 1. 系统上电
    let mut context = boot_default();

    // 2. 验证系统初始化
    assert!(context.interface_count() > 0, "系统应该至少有一个接口");

    // 3. 验证所有接口正确初始化
    for iface in context.interfaces.interfaces() {
        assert!(!iface.name().is_empty(), "接口名称不应为空");
        assert!(iface.rxq.capacity() > 0, "接收队列容量应大于0");
        assert!(iface.txq.capacity() > 0, "发送队列容量应大于0");
        assert!(iface.rxq.is_empty(), "初始接收队列应为空");
        assert!(iface.txq.is_empty(), "初始发送队列应为空");
    }

    // 4. 验证队列容量符合配置
    if let Some(eth0) = context.get_interface("eth0") {
        // 默认配置文件中 rxq_capacity = 256, txq_capacity = 256
        assert_eq!(eth0.rxq.capacity(), 256);
        assert_eq!(eth0.txq.capacity(), 256);
    }

    // 5. 模拟报文处理
    if context.get_interface("eth0").is_some() {
        inject_packets(&mut context, "eth0", 10);
        assert!(count_all_packets(&context) > 0, "应该有报文在队列中");
    }

    // 6. 系统下电
    shutdown(&mut context);

    // 7. 验证所有队列已清空
    assert_eq!(count_all_packets(&context), 0, "下电后所有队列应为空");
}

#[test]
fn test_lifecycle_reboot() {
    // 测试可以重新启动系统
    let mut context1 = boot_default();
    let count1 = context1.interface_count();
    shutdown(&mut context1);

    // 重新启动
    let context2 = boot_default();
    let count2 = context2.interface_count();

    // 验证接口数量一致
    assert_eq!(count1, count2, "重启后接口数量应保持一致");
}

#[test]
fn test_lifecycle_with_modifications() {
    // 测试在上电后修改接口配置
    let mut context = boot_default();

    // 修改接口配置
    if let Some(iface) = context.get_interface_mut("eth0") {
        iface.set_ip_addr(Ipv4Addr::new(10, 0, 0, 1));
        iface.up();
    }

    // 验证修改生效
    if let Some(iface) = context.get_interface("eth0") {
        assert_eq!(iface.ip_addr, Ipv4Addr::new(10, 0, 0, 1));
        assert_eq!(iface.state, InterfaceState::Up);
    }

    // 下电
    shutdown(&mut context);

    // 验证下电成功
    assert_eq!(count_all_packets(&context), 0);
}

#[test]
fn test_lifecycle_multiple_cycles() {
    // 测试多次上电下电循环
    for i in 0..3 {
        let mut context = boot_default();
        assert!(context.interface_count() > 0, "第 {} 次启动应成功", i + 1);

        if let Some(_) = context.get_interface("eth0") {
            inject_packets(&mut context, "eth0", 5);
        }

        shutdown(&mut context);
        assert_eq!(count_all_packets(&context), 0, "第 {} 次下电应清理所有队列", i + 1);
    }
}

// 场景二：与全局管理器集成

#[test]
fn test_global_manager_integration() {
    // 1. 启动系统（会初始化全局管理器）
    boot_default();

    // 2. 通过全局接口管理器访问接口
    let manager_opt = core_net::interface::global_manager();

    // 3. 验证全局管理器已初始化
    assert!(manager_opt.is_some(), "全局管理器应该已初始化");

    // 4. 通过全局管理器查询接口
    if let Some(manager) = manager_opt {
        let guard = manager.lock().unwrap();
        assert!(guard.len() > 0, "全局管理器应该包含接口");

        // 验证可以通过全局管理器获取接口
        if let Ok(eth0) = guard.get_by_name("eth0") {
            assert_eq!(eth0.name(), "eth0");
        }
    }
}

#[test]
fn test_global_manager_modification() {
    boot_default();

    // 通过全局管理器修改接口配置
    let result = update_interface("eth0", |iface| {
        iface.set_ip_addr(Ipv4Addr::new(10, 0, 0, 1));
    });
    assert!(result.is_ok(), "通过全局管理器修改接口 IP 应该成功");

    // 验证修改生效
    if let Some(manager) = core_net::interface::global_manager() {
        let guard = manager.lock().unwrap();
        if let Ok(eth0) = guard.get_by_name("eth0") {
            assert_eq!(eth0.ip_addr, Ipv4Addr::new(10, 0, 0, 1));
        }
    }
}

#[test]
fn test_global_manager_up_down() {
    boot_default();

    // 通过全局管理器禁用接口
    let result = update_interface("eth0", |iface| iface.down());
    assert!(result.is_ok(), "通过全局管理器禁用接口应该成功");

    // 验证接口已禁用
    if let Some(manager) = core_net::interface::global_manager() {
        let guard = manager.lock().unwrap();
        if let Ok(eth0) = guard.get_by_name("eth0") {
            assert_eq!(eth0.state, InterfaceState::Down);
        }
    }

    // 通过全局管理器启用接口
    let result = update_interface("eth0", |iface| iface.up());
    assert!(result.is_ok(), "通过全局管理器启用接口应该成功");

    // 验证接口已启用
    if let Some(manager) = core_net::interface::global_manager() {
        let guard = manager.lock().unwrap();
        if let Ok(eth0) = guard.get_by_name("eth0") {
            assert_eq!(eth0.state, InterfaceState::Up);
        }
    }
}

#[test]
fn test_global_manager_multiple_modifications() {
    boot_default();

    // 保存原始配置
    let (original_ip, _original_mac, original_netmask, original_mtu) = if let Some(manager) = core_net::interface::global_manager() {
        let guard = manager.lock().unwrap();
        if let Ok(eth0) = guard.get_by_name("eth0") {
            (eth0.ip_addr, eth0.mac_addr, eth0.netmask, eth0.mtu)
        } else {
            return; // 如果没有 eth0，跳过测试
        }
    } else {
        return;
    };

    // 执行多个修改
    update_interface("eth0", |iface| {
        iface.set_ip_addr(Ipv4Addr::new(10, 0, 0, 1));
        iface.set_netmask(Ipv4Addr::new(255, 255, 255, 128));
        iface.set_mtu(9000);
    }).unwrap();

    // 验证所有修改生效
    if let Some(manager) = core_net::interface::global_manager() {
        let guard = manager.lock().unwrap();
        if let Ok(eth0) = guard.get_by_name("eth0") {
            assert_eq!(eth0.ip_addr, Ipv4Addr::new(10, 0, 0, 1));
            assert_eq!(eth0.netmask, Ipv4Addr::new(255, 255, 255, 128));
            assert_eq!(eth0.mtu, 9000);
        }
    }

    // 恢复原始配置
    update_interface("eth0", |iface| {
        iface.set_ip_addr(original_ip);
        iface.set_netmask(original_netmask);
        iface.set_mtu(original_mtu);
    }).unwrap();
}

#[test]
fn test_global_manager_error_handling() {
    boot_default();

    // 测试操作不存在的接口
    let result = update_interface("nonexistent", |iface| {
        iface.set_ip_addr(Ipv4Addr::new(10, 0, 0, 1));
    });
    assert!(result.is_err(), "操作不存在的接口应该返回错误");

    // 3. 测试获取不存在的接口
    if let Some(manager) = core_net::interface::global_manager() {
        let guard = manager.lock().unwrap();
        let result = guard.get_by_name("nonexistent");
        assert!(result.is_err(), "获取不存在的接口应该返回错误");
    }
}

// 场景三：完整集成测试

#[test]
fn test_full_integration_scenario() {
    // 完整的集成场景：启动 -> 注入报文 -> 下电

    // 1. 启动系统
    let mut context = boot_default();
    assert!(context.interface_count() > 0);

    // 2. 注入报文
    if context.get_interface("eth0").is_some() {
        inject_packets(&mut context, "eth0", 10);
        assert!(count_all_packets(&context) > 0);
    }

    // 3. 通过上下文修改接口
    if let Some(iface) = context.get_interface_mut("eth0") {
        iface.down();
    }

    // 4. 验证修改在上下文中生效
    if let Some(iface) = context.get_interface("eth0") {
        assert_eq!(iface.state, InterfaceState::Down);
    }

    // 5. 下电
    shutdown(&mut context);
    assert_eq!(count_all_packets(&context), 0);

    // 6. 恢复原始状态（通过全局管理器）
    if let Some(_manager) = core_net::interface::global_manager() {
        let _ = update_interface("eth0", |iface| iface.up());
    }
}

#[test]
fn test_integration_with_config_file() {
    // 测试默认配置文件被正确加载

    let context = boot_default();

    // 验证配置文件中的接口被正确加载
    if let Some(eth0) = context.get_interface("eth0") {
        assert_eq!(eth0.mac_addr, MacAddr::new([0x00, 0x11, 0x22, 0x33, 0x44, 0x55]));
        assert_eq!(eth0.ip_addr, Ipv4Addr::new(192, 168, 1, 100));
        assert_eq!(eth0.netmask, Ipv4Addr::new(255, 255, 255, 0));
        assert_eq!(eth0.mtu, 1500);
        assert_eq!(eth0.state, InterfaceState::Up);
    }

    if let Some(lo) = context.get_interface("lo") {
        assert_eq!(lo.mac_addr, MacAddr::zero());
        assert_eq!(lo.ip_addr, Ipv4Addr::new(127, 0, 0, 1));
        assert_eq!(lo.netmask, Ipv4Addr::new(255, 0, 0, 0));
    }
}

#[test]
fn test_integration_queue_capacity() {
    // 测试队列容量配置被正确应用

    let context = boot_default();

    // 默认配置文件中 rxq_capacity = 256, txq_capacity = 256
    for iface in context.interfaces.interfaces() {
        assert_eq!(iface.rxq.capacity(), 256, "接收队列容量应为 256");
        assert_eq!(iface.txq.capacity(), 256, "发送队列容量应为 256");
    }
}
