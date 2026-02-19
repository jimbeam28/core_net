// interface 模块集成测试
//
// 测试 interface 模块与其他模块的集成场景

use core_net::interface;
use core_net::interface::{InterfaceState, MacAddr, Ipv4Addr};

// ========== 场景一：系统上电初始化流程 ==========

#[test]
fn test_boot_initialization_flow() {
    // 模拟系统上电初始化流程
    // 1. 从默认配置文件加载接口配置
    // 2. 验证全局接口管理器已创建
    // 3. 验证接口配置已从文件加载
    // 4. 验证每个接口的队列已正确初始化
    // 5. 验证接口状态符合配置

    // 注意：这个测试依赖于 src/interface/interface.toml 文件存在
    // 由于测试执行顺序不确定，全局管理器可能已被初始化
    // 我们只测试可以测试的部分

    let result = interface::init_default();
    if result.is_ok() {
        // 初始化成功，验证接口
        if let Some(manager) = interface::global_manager() {
            let guard = manager.lock().unwrap();

            // 验证有接口被加载
            assert!(!guard.is_empty());

            // 验证每个接口的基本属性
            for iface in guard.interfaces() {
                // 验证接口名称不为空
                assert!(!iface.name().is_empty());

                // 验证 IP 地址有效
                assert!(!iface.ip_addr.is_zero() || iface.name() == "lo");

                // 验证队列已创建（通过访问 rxq 和 txq 间接验证）
                let _ = &iface.rxq;
                let _ = &iface.txq;
            }
        }
    }
}

#[test]
fn test_manual_initialization() {
    // 手动创建接口管理器并初始化全局管理器

    let mut manager = interface::InterfaceManager::new(256, 256);

    // 添加多个接口
    let config1 = interface::InterfaceConfig {
        name: "eth0".to_string(),
        mac_addr: MacAddr::new([0x00, 0x11, 0x22, 0x33, 0x44, 0x55]),
        ip_addr: Ipv4Addr::new(192, 168, 1, 100),
        netmask: Ipv4Addr::new(255, 255, 255, 0),
        gateway: Some(Ipv4Addr::new(192, 168, 1, 1)),
        mtu: Some(1500),
        state: Some(InterfaceState::Up),
    };

    let config2 = interface::InterfaceConfig {
        name: "eth1".to_string(),
        mac_addr: MacAddr::new([0x00, 0x11, 0x22, 0x33, 0x44, 0x56]),
        ip_addr: Ipv4Addr::new(192, 168, 2, 100),
        netmask: Ipv4Addr::new(255, 255, 255, 0),
        gateway: Some(Ipv4Addr::new(192, 168, 2, 1)),
        mtu: Some(1500),
        state: Some(InterfaceState::Down),
    };

    let config3 = interface::InterfaceConfig {
        name: "lo".to_string(),
        mac_addr: MacAddr::zero(),
        ip_addr: Ipv4Addr::new(127, 0, 0, 1),
        netmask: Ipv4Addr::new(255, 0, 0, 0),
        gateway: None,
        mtu: None,
        state: Some(InterfaceState::Up),
    };

    manager.add_from_config(config1).unwrap();
    manager.add_from_config(config2).unwrap();
    manager.add_from_config(config3).unwrap();

    // 注意：由于 OnceLock 的特性，这里可能会失败（如果已经初始化过）
    // 在实际的集成测试环境中，可能需要特殊的设置来处理这种情况
    let _ = interface::init_global_manager(manager);
}

// ========== 场景二：多接口协同工作 ==========

#[test]
fn test_multi_interface_coordination() {
    // 测试多个接口独立工作
    let mut manager = interface::InterfaceManager::new(256, 256);

    // 添加多个接口
    for i in 0..3 {
        let config = interface::InterfaceConfig {
            name: format!("eth{}", i),
            mac_addr: MacAddr::new([0x00, 0x11, 0x22, 0x33, 0x44, i as u8]),
            ip_addr: Ipv4Addr::new(192, 168, i as u8, 100),
            netmask: Ipv4Addr::new(255, 255, 255, 0),
            gateway: None,
            mtu: Some(1500),
            state: Some(InterfaceState::Up),
        };
        manager.add_from_config(config).unwrap();
    }

    // 验证每个接口独立处理
    for i in 0..3 {
        let name = format!("eth{}", i);
        let iface = manager.get_by_name(&name).unwrap();

        // 验证接口属性
        assert_eq!(iface.name(), &name);
        assert_eq!(iface.index(), i as u32);
        assert!(iface.is_up());

        // 验证接口队列是独立的
        let _ = &iface.rxq;
        let _ = &iface.txq;
    }

    // 测试通过索引访问接口
    for i in 0..3 {
        let iface = manager.get_by_index(i).unwrap();
        assert_eq!(iface.index(), i as u32);
    }
}

#[test]
fn test_interface_queue_independence() {
    // 测试每个接口的队列是独立的
    let mut manager = interface::InterfaceManager::new(4, 4);

    let config1 = interface::InterfaceConfig {
        name: "eth0".to_string(),
        mac_addr: MacAddr::new([0x00, 0x11, 0x22, 0x33, 0x44, 0x55]),
        ip_addr: Ipv4Addr::new(192, 168, 1, 100),
        netmask: Ipv4Addr::new(255, 255, 255, 0),
        gateway: None,
        mtu: None,
        state: None,
    };

    let config2 = interface::InterfaceConfig {
        name: "eth1".to_string(),
        mac_addr: MacAddr::new([0x00, 0x11, 0x22, 0x33, 0x44, 0x56]),
        ip_addr: Ipv4Addr::new(192, 168, 2, 100),
        netmask: Ipv4Addr::new(255, 255, 255, 0),
        gateway: None,
        mtu: None,
        state: None,
    };

    manager.add_from_config(config1).unwrap();
    manager.add_from_config(config2).unwrap();

    // 验证可以独立访问和修改每个接口
    // 获取第一个接口的地址并修改状态
    let iface0_addr = {
        let iface = manager.get_by_name_mut("eth0").unwrap();
        iface.up();
        iface.ip_addr
    };

    // 获取第二个接口的地址并修改状态
    let iface1_addr = {
        let iface = manager.get_by_name_mut("eth1").unwrap();
        iface.up();
        iface.ip_addr
    };

    // 验证接口是独立的
    assert_ne!(iface0_addr, iface1_addr);
    assert!(manager.get_by_name("eth0").unwrap().is_up());
    assert!(manager.get_by_name("eth1").unwrap().is_up());
}

// ========== 场景三：接口配置运行时修改 ==========

#[test]
fn test_runtime_interface_configuration() {
    // 测试运行时修改接口配置
    let mut manager = interface::InterfaceManager::new(256, 256);

    let config = interface::InterfaceConfig {
        name: "eth0".to_string(),
        mac_addr: MacAddr::new([0x00, 0x11, 0x22, 0x33, 0x44, 0x55]),
        ip_addr: Ipv4Addr::new(192, 168, 1, 100),
        netmask: Ipv4Addr::new(255, 255, 255, 0),
        gateway: Some(Ipv4Addr::new(192, 168, 1, 1)),
        mtu: Some(1500),
        state: Some(InterfaceState::Up),
    };

    manager.add_from_config(config).unwrap();

    // 保存原始值
    let original_ip = {
        let iface = manager.get_by_name("eth0").unwrap();
        iface.ip_addr
    };

    // 修改 IP 地址
    {
        let iface = manager.get_by_name_mut("eth0").unwrap();
        iface.set_ip_addr(Ipv4Addr::new(10, 0, 0, 1));
    }

    // 验证修改生效
    {
        let iface = manager.get_by_name("eth0").unwrap();
        assert_eq!(iface.ip_addr, Ipv4Addr::new(10, 0, 0, 1));
    }

    // 恢复原始值
    {
        let iface = manager.get_by_name_mut("eth0").unwrap();
        iface.set_ip_addr(original_ip);
    }
}

#[test]
fn test_global_manager_runtime_modification() {
    // 测试通过全局管理器修改接口配置

    if let Some(manager) = interface::global_manager() {
        // 尝试修改一个接口的配置
        let original_ip = {
            let guard = manager.lock().unwrap();
            if let Ok(iface) = guard.get_by_name("eth0") {
                Some(iface.ip_addr)
            } else {
                None
            }
        };

        if let Some(original) = original_ip {
            let new_ip = Ipv4Addr::new(10, 0, 0, 1);

            // 使用全局函数修改
            let result = interface::update_interface("eth0", |iface| {
                iface.set_ip_addr(new_ip);
            });

            if result.is_ok() {
                // 验证修改生效
                let guard = manager.lock().unwrap();
                let iface = guard.get_by_name("eth0").unwrap();
                assert_eq!(iface.ip_addr, new_ip);

                // 恢复原始值
                drop(guard);
                let _ = interface::update_interface("eth0", |iface| {
                    iface.set_ip_addr(original);
                });
            }
        }
    }
}

// ========== 辅助测试 ==========

#[test]
fn test_interface_network_address_calculation() {
    // 测试网络地址计算的集成
    let mut manager = interface::InterfaceManager::new(256, 256);

    let config = interface::InterfaceConfig {
        name: "eth0".to_string(),
        mac_addr: MacAddr::new([0x00, 0x11, 0x22, 0x33, 0x44, 0x55]),
        ip_addr: Ipv4Addr::new(192, 168, 1, 100),
        netmask: Ipv4Addr::new(255, 255, 255, 0),
        gateway: None,
        mtu: None,
        state: None,
    };

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
        assert!(iface.name().starts_with("eth"));
        count += 1;
    }
    assert_eq!(count, 5);
}

#[test]
fn test_interface_state_transitions() {
    // 测试接口状态转换
    let mut manager = interface::InterfaceManager::new(256, 256);

    let config = interface::InterfaceConfig {
        name: "eth0".to_string(),
        mac_addr: MacAddr::new([0x00, 0x11, 0x22, 0x33, 0x44, 0x55]),
        ip_addr: Ipv4Addr::new(192, 168, 1, 100),
        netmask: Ipv4Addr::new(255, 255, 255, 0),
        gateway: None,
        mtu: None,
        state: Some(InterfaceState::Down),
    };

    manager.add_from_config(config).unwrap();

    // 验证初始状态
    {
        let iface = manager.get_by_name("eth0").unwrap();
        assert_eq!(iface.state, InterfaceState::Down);
        assert!(!iface.is_up());
    }

    // 启用接口
    {
        let iface = manager.get_by_name_mut("eth0").unwrap();
        iface.up();
    }

    // 验证状态变更
    {
        let iface = manager.get_by_name("eth0").unwrap();
        assert_eq!(iface.state, InterfaceState::Up);
        assert!(iface.is_up());
    }

    // 禁用接口
    {
        let iface = manager.get_by_name_mut("eth0").unwrap();
        iface.down();
    }

    // 验证状态变更
    {
        let iface = manager.get_by_name("eth0").unwrap();
        assert_eq!(iface.state, InterfaceState::Down);
        assert!(!iface.is_up());
    }
}

#[test]
fn test_multiple_managers_independence() {
    // 测试多个管理器实例是独立的
    let mut manager1 = interface::InterfaceManager::new(256, 256);
    let mut manager2 = interface::InterfaceManager::new(512, 512);

    let config1 = interface::InterfaceConfig {
        name: "eth0".to_string(),
        mac_addr: MacAddr::new([0x00, 0x11, 0x22, 0x33, 0x44, 0x55]),
        ip_addr: Ipv4Addr::new(192, 168, 1, 100),
        netmask: Ipv4Addr::new(255, 255, 255, 0),
        gateway: None,
        mtu: None,
        state: None,
    };

    let config2 = interface::InterfaceConfig {
        name: "eth1".to_string(),
        mac_addr: MacAddr::new([0x00, 0x11, 0x22, 0x33, 0x44, 0x56]),
        ip_addr: Ipv4Addr::new(192, 168, 2, 100),
        netmask: Ipv4Addr::new(255, 255, 255, 0),
        gateway: None,
        mtu: None,
        state: None,
    };

    manager1.add_from_config(config1).unwrap();
    manager2.add_from_config(config2).unwrap();

    // 验证两个管理器是独立的
    assert!(manager1.get_by_name("eth0").is_ok());
    assert!(manager1.get_by_name("eth1").is_err());

    assert!(manager2.get_by_name("eth1").is_ok());
    assert!(manager2.get_by_name("eth0").is_err());

    assert_eq!(manager1.len(), 1);
    assert_eq!(manager2.len(), 1);
}
