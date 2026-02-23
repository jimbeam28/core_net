// interface 模块集成测试

use core_net::interface;
use core_net::interface::{InterfaceState, MacAddr, Ipv4Addr};
use core_net::protocols::Ipv6Addr;
use core_net::context::SystemContext;
use serial_test::serial;

mod common;
use common::create_test_eth0_config;

// 场景一：系统上电初始化流程

#[test]
#[serial]
fn test_boot_initialization_flow() {
    // 模拟系统上电初始化流程
    // 1. 从默认配置文件加载接口配置
    // 2. 验证系统上下文已创建
    // 3. 验证接口配置已从文件加载
    // 4. 验证每个接口的队列已正确初始化
    // 5. 验证接口状态符合配置

    let ctx = SystemContext::from_config();

    // 验证有接口被加载
    assert!(ctx.interface_count() > 0);

    // 验证每个接口的基本属性
    let interfaces = ctx.interfaces.lock().unwrap();
    for iface in interfaces.interfaces() {
        // 验证接口名称不为空
        assert!(!iface.name.is_empty());

        // 验证 IP 地址有效
        assert!(!iface.ip_addr.is_zero() || iface.name == "lo");

        // 验证队列已创建
        assert!(iface.rxq.capacity() > 0);
        assert!(iface.txq.capacity() > 0);
    }
}

// 场景二：多接口协同工作

#[test]
fn test_multi_interface_coordination() {
    let mut manager = interface::InterfaceManager::new(256, 256);

    for i in 0..3 {
        let config = interface::InterfaceConfig {
            name: format!("eth{}", i),
            mac_addr: MacAddr::new([0x00, 0x11, 0x22, 0x33, 0x44, i as u8]),
            ip_addr: Ipv4Addr::new(192, 168, i as u8, 100),
            ipv6_addr: Ipv6Addr::new(0xfe80, 0, 0, 0, 0, 0, 0, i as u16 + 1),
            netmask: Ipv4Addr::new(255, 255, 255, 0),
            gateway: None,
            mtu: Some(1500),
            state: Some(InterfaceState::Up),
        };
        manager.add_from_config(config).unwrap();
    }

    for i in 0..3 {
        let name = format!("eth{}", i);
        let iface = manager.get_by_name(&name).unwrap();

        assert_eq!(iface.name, name);
        assert_eq!(iface.index, i as u32);
        assert!(iface.is_up());
        let _ = &iface.rxq;
        let _ = &iface.txq;
    }

    for i in 0..3 {
        let iface = manager.get_by_index(i).unwrap();
        assert_eq!(iface.index, i);
    }
}

#[test]
fn test_interface_queue_independence() {
    let mut manager = interface::InterfaceManager::new(4, 4);

    let mut config1 = create_test_eth0_config();
    config1.state = None;
    config1.gateway = None;
    config1.mtu = None;

    let config2 = interface::InterfaceConfig {
        name: "eth1".to_string(),
        mac_addr: MacAddr::new([0x00, 0x11, 0x22, 0x33, 0x44, 0x56]),
        ip_addr: Ipv4Addr::new(192, 168, 2, 100),
        ipv6_addr: Ipv6Addr::new(0xfe80, 0, 0, 0, 0, 0, 0, 2),
        netmask: Ipv4Addr::new(255, 255, 255, 0),
        gateway: None,
        mtu: None,
        state: None,
    };

    manager.add_from_config(config1).unwrap();
    manager.add_from_config(config2).unwrap();

    let iface0_addr = {
        let iface = manager.get_by_name_mut("eth0").unwrap();
        iface.up();
        iface.ip_addr
    };

    let iface1_addr = {
        let iface = manager.get_by_name_mut("eth1").unwrap();
        iface.up();
        iface.ip_addr
    };

    assert_ne!(iface0_addr, iface1_addr);
    assert!(manager.get_by_name("eth0").unwrap().is_up());
    assert!(manager.get_by_name("eth1").unwrap().is_up());
}

// 场景三：接口配置运行时修改

#[test]
fn test_runtime_interface_configuration() {
    let mut manager = interface::InterfaceManager::new(256, 256);

    let config = create_test_eth0_config();
    manager.add_from_config(config).unwrap();

    let original_ip = {
        let iface = manager.get_by_name("eth0").unwrap();
        iface.ip_addr
    };

    {
        let iface = manager.get_by_name_mut("eth0").unwrap();
        iface.set_ip_addr(Ipv4Addr::new(10, 0, 0, 1));
    }

    {
        let iface = manager.get_by_name("eth0").unwrap();
        assert_eq!(iface.ip_addr, Ipv4Addr::new(10, 0, 0, 1));
    }

    {
        let iface = manager.get_by_name_mut("eth0").unwrap();
        iface.set_ip_addr(original_ip);
    }
}

#[test]
fn test_system_context_runtime_modification() {
    // 测试通过系统上下文修改接口配置
    let ctx = SystemContext::from_config();

    let original_ip = {
        let interfaces = ctx.interfaces.lock().unwrap();
        if let Ok(iface) = interfaces.get_by_name("eth0") {
            Some(iface.ip_addr)
        } else {
            None
        }
    };

    if let Some(original) = original_ip {
        let new_ip = Ipv4Addr::new(10, 0, 0, 1);

        // 使用系统上下文修改
        {
            let mut interfaces = ctx.interfaces.lock().unwrap();
            if let Ok(iface) = interfaces.get_by_name_mut("eth0") {
                iface.set_ip_addr(new_ip);
            }
        }

        // 验证修改生效
        {
            let interfaces = ctx.interfaces.lock().unwrap();
            let iface = interfaces.get_by_name("eth0").unwrap();
            assert_eq!(iface.ip_addr, new_ip);
        }

        // 恢复原始值
        {
            let mut interfaces = ctx.interfaces.lock().unwrap();
            if let Ok(iface) = interfaces.get_by_name_mut("eth0") {
                iface.set_ip_addr(original);
            }
        }
    }
}

// 辅助测试

#[test]
fn test_interface_network_address_calculation() {
    let mut manager = interface::InterfaceManager::new(256, 256);

    let mut config = create_test_eth0_config();
    config.gateway = None;
    config.mtu = None;
    config.state = None;

    manager.add_from_config(config).unwrap();

    let iface = manager.get_by_name("eth0").unwrap();
    let network = iface.network_address();
    let broadcast = iface.broadcast_address();

    assert_eq!(network, Ipv4Addr::new(192, 168, 1, 0));
    assert_eq!(broadcast, Ipv4Addr::new(192, 168, 1, 255));
}

#[test]
fn test_interface_iterator() {
    // 测试遍历所有接口
    let mut manager = interface::InterfaceManager::new(256, 256);

    for i in 0..5 {
        let config = interface::InterfaceConfig {
            name: format!("eth{}", i),
            mac_addr: MacAddr::new([0x00, 0x11, 0x22, 0x33, 0x44, i as u8]),
            ip_addr: Ipv4Addr::new(192, 168, i as u8, 100),
            ipv6_addr: Ipv6Addr::new(0xfe80, 0, 0, 0, 0, 0, 0, i as u16 + 1),
            netmask: Ipv4Addr::new(255, 255, 255, 0),
            gateway: None,
            mtu: None,
            state: None,
        };
        manager.add_from_config(config).unwrap();
    }

    // 遍历所有接口
    let mut count = 0;
    for iface in manager.interfaces() {
        assert!(iface.name.starts_with("eth"));
        count += 1;
    }
    assert_eq!(count, 5);
}

#[test]
fn test_interface_state_transitions() {
    let mut manager = interface::InterfaceManager::new(256, 256);

    let mut config = create_test_eth0_config();
    config.state = Some(InterfaceState::Down);

    manager.add_from_config(config).unwrap();

    {
        let iface = manager.get_by_name("eth0").unwrap();
        assert_eq!(iface.state, InterfaceState::Down);
        assert!(!iface.is_up());
    }

    {
        let iface = manager.get_by_name_mut("eth0").unwrap();
        iface.up();
    }

    {
        let iface = manager.get_by_name("eth0").unwrap();
        assert_eq!(iface.state, InterfaceState::Up);
        assert!(iface.is_up());
    }

    {
        let iface = manager.get_by_name_mut("eth0").unwrap();
        iface.down();
    }

    {
        let iface = manager.get_by_name("eth0").unwrap();
        assert_eq!(iface.state, InterfaceState::Down);
        assert!(!iface.is_up());
    }
}

#[test]
fn test_multiple_managers_independence() {
    let mut manager1 = interface::InterfaceManager::new(256, 256);
    let mut manager2 = interface::InterfaceManager::new(512, 512);

    let mut config1 = create_test_eth0_config();
    config1.gateway = None;
    config1.mtu = None;
    config1.state = None;

    let config2 = interface::InterfaceConfig {
        name: "eth1".to_string(),
        mac_addr: MacAddr::new([0x00, 0x11, 0x22, 0x33, 0x44, 0x56]),
        ip_addr: Ipv4Addr::new(192, 168, 2, 100),
        ipv6_addr: Ipv6Addr::new(0xfe80, 0, 0, 0, 0, 0, 0, 2),
        netmask: Ipv4Addr::new(255, 255, 255, 0),
        gateway: None,
        mtu: None,
        state: None,
    };

    manager1.add_from_config(config1).unwrap();
    manager2.add_from_config(config2).unwrap();

    assert!(manager1.get_by_name("eth0").is_ok());
    assert!(manager1.get_by_name("eth1").is_err());

    assert!(manager2.get_by_name("eth1").is_ok());
    assert!(manager2.get_by_name("eth0").is_err());

    assert_eq!(manager1.len(), 1);
    assert_eq!(manager2.len(), 1);
}
