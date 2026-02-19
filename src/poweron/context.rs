use crate::interface::InterfaceManager;

/// 系统上下文，持有接口管理器的所有权（每个接口内部有自己的队列）
pub struct SystemContext {
    pub interfaces: InterfaceManager,
}

impl SystemContext {
    /// 创建新的系统上下文：初始化 ARP 缓存并加载接口配置
    pub fn new() -> Self {
        let _ = crate::protocols::arp::get_or_init_global_arp_cache();
        let global_init_result = crate::interface::init_default();
        let interface_manager = match crate::interface::load_default_config() {
            Ok(manager) => manager,
            Err(e) => {
                eprintln!("[警告] 加载接口配置失败: {}, 使用空接口管理器", e);
                InterfaceManager::default()
            }
        };
        if let Err(e) = global_init_result {
            eprintln!("[警告] 初始化全局接口管理器失败: {}", e);
        }
        SystemContext {
            interfaces: interface_manager,
        }
    }

    /// 获取接口数量
    pub fn interface_count(&self) -> usize {
        self.interfaces.len()
    }

    /// 通过名称获取接口
    pub fn get_interface(&self, name: &str) -> Option<&crate::interface::NetworkInterface> {
        self.interfaces.get_by_name(name).ok()
    }

    pub fn get_interface_mut(&mut self, name: &str) -> Option<&mut crate::interface::NetworkInterface> {
        self.interfaces.get_by_name_mut(name).ok()
    }

    /// 通过索引获取接口
    pub fn get_interface_by_index(&self, index: u32) -> Option<&crate::interface::NetworkInterface> {
        self.interfaces.get_by_index(index).ok()
    }

    pub fn get_interface_by_index_mut(&mut self, index: u32) -> Option<&mut crate::interface::NetworkInterface> {
        self.interfaces.get_by_index_mut(index).ok()
    }
}

// ========== 单元测试 ==========

#[cfg(test)]
mod tests {
    use super::*;
    use crate::interface::{InterfaceConfig, InterfaceState, MacAddr, Ipv4Addr};

    // ========== 测试辅助函数 ==========

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

    /// 创建测试用接口管理器
    fn create_test_manager() -> InterfaceManager {
        let mut manager = InterfaceManager::new(256, 256);
        manager.add_from_config(create_eth0_config()).unwrap();
        manager.add_from_config(create_lo_config()).unwrap();
        manager
    }

    /// 创建测试上下文
    fn create_test_context() -> SystemContext {
        SystemContext {
            interfaces: create_test_manager(),
        }
    }

    /// 创建空上下文
    fn create_empty_context() -> SystemContext {
        SystemContext {
            interfaces: InterfaceManager::default(),
        }
    }

    // ========== SystemContext 基础测试组 ==========

    #[test]
    fn test_context_create() {
        let context = create_test_context();
        assert_eq!(context.interface_count(), 2);
    }

    #[test]
    fn test_context_create_empty() {
        let context = create_empty_context();
        assert_eq!(context.interface_count(), 0);
        assert!(context.get_interface("eth0").is_none());
        assert!(context.get_interface_by_index(0).is_none());
    }

    #[test]
    fn test_context_interface_count() {
        let empty = create_empty_context();
        assert_eq!(empty.interface_count(), 0);

        let single = SystemContext {
            interfaces: {
                let mut mgr = InterfaceManager::new(256, 256);
                mgr.add_from_config(create_eth0_config()).unwrap();
                mgr
            },
        };
        assert_eq!(single.interface_count(), 1);

        let multi = create_test_context();
        assert_eq!(multi.interface_count(), 2);
    }

    #[test]
    fn test_context_get_interface_by_name() {
        let context = create_test_context();

        // 获取存在的接口
        let eth0 = context.get_interface("eth0");
        assert!(eth0.is_some());
        assert_eq!(eth0.unwrap().name(), "eth0");

        let lo = context.get_interface("lo");
        assert!(lo.is_some());
        assert_eq!(lo.unwrap().name(), "lo");

        // 获取不存在的接口
        let nonexistent = context.get_interface("eth99");
        assert!(nonexistent.is_none());

        let empty_string = context.get_interface("");
        assert!(empty_string.is_none());
    }

    #[test]
    fn test_context_get_interface_by_index() {
        let context = create_test_context();

        // 通过索引获取接口
        let iface0 = context.get_interface_by_index(0);
        assert!(iface0.is_some());
        assert_eq!(iface0.unwrap().name(), "eth0");

        let iface1 = context.get_interface_by_index(1);
        assert!(iface1.is_some());
        assert_eq!(iface1.unwrap().name(), "lo");

        // 越界索引
        let out_of_range = context.get_interface_by_index(99);
        assert!(out_of_range.is_none());

        // 空上下文
        let empty = create_empty_context();
        assert!(empty.get_interface_by_index(0).is_none());
    }

    #[test]
    fn test_context_get_interface_mut_by_name() {
        let mut context = create_test_context();

        // 修改接口 IP 地址
        if let Some(iface) = context.get_interface_mut("eth0") {
            iface.set_ip_addr(Ipv4Addr::new(10, 0, 0, 1));
        }

        // 验证修改生效
        let eth0 = context.get_interface("eth0").unwrap();
        assert_eq!(eth0.ip_addr, Ipv4Addr::new(10, 0, 0, 1));

        // 不存在的接口
        assert!(context.get_interface_mut("nonexistent").is_none());
    }

    #[test]
    fn test_context_get_interface_mut_by_index() {
        let mut context = create_test_context();

        // 修改接口状态
        if let Some(iface) = context.get_interface_by_index_mut(0) {
            iface.down();
        }

        // 验证修改生效
        let eth0 = context.get_interface_by_index(0).unwrap();
        assert!(!eth0.is_up());

        // 越界索引
        assert!(context.get_interface_by_index_mut(99).is_none());
    }

    #[test]
    fn test_context_with_single_interface() {
        let context = SystemContext {
            interfaces: {
                let mut mgr = InterfaceManager::new(256, 256);
                mgr.add_from_config(create_eth0_config()).unwrap();
                mgr
            },
        };

        assert_eq!(context.interface_count(), 1);
        assert!(context.get_interface("eth0").is_some());
        assert!(context.get_interface("lo").is_none());
        assert!(context.get_interface_by_index(0).is_some());
        assert!(context.get_interface_by_index(1).is_none());
    }

    #[test]
    fn test_context_with_multiple_interfaces() {
        let mut context = create_test_context();

        // 添加更多接口
        let config3 = InterfaceConfig {
            name: "eth1".to_string(),
            mac_addr: MacAddr::new([0x00, 0x11, 0x22, 0x33, 0x44, 0x56]),
            ip_addr: Ipv4Addr::new(192, 168, 2, 100),
            netmask: Ipv4Addr::new(255, 255, 255, 0),
            gateway: Some(Ipv4Addr::new(192, 168, 2, 1)),
            mtu: Some(1500),
            state: Some(InterfaceState::Down),
        };
        context.interfaces.add_from_config(config3).unwrap();

        assert_eq!(context.interface_count(), 3);
        assert!(context.get_interface("eth0").is_some());
        assert!(context.get_interface("lo").is_some());
        assert!(context.get_interface("eth1").is_some());
    }

    #[test]
    fn test_context_interface_properties() {
        let context = create_test_context();

        let eth0 = context.get_interface("eth0").unwrap();
        assert_eq!(eth0.name(), "eth0");
        assert_eq!(eth0.index(), 0);
        assert_eq!(eth0.mac_addr, MacAddr::new([0x00, 0x11, 0x22, 0x33, 0x44, 0x55]));
        assert_eq!(eth0.ip_addr, Ipv4Addr::new(192, 168, 1, 100));
        assert_eq!(eth0.mtu, 1500);
        assert_eq!(eth0.state, InterfaceState::Up);

        let lo = context.get_interface("lo").unwrap();
        assert_eq!(lo.name(), "lo");
        assert_eq!(lo.index(), 1);
        assert_eq!(lo.ip_addr, Ipv4Addr::new(127, 0, 0, 1));
    }

    #[test]
    fn test_context_queue_access() {
        let context = create_test_context();

        let eth0 = context.get_interface("eth0").unwrap();
        // 验证队列存在（通过检查其方法）
        assert!(eth0.rxq.is_empty());
        assert!(eth0.txq.is_empty());
    }

    #[test]
    fn test_context_manager_direct_access() {
        let context = create_test_context();

        // 直接访问接口管理器
        let interfaces = context.interfaces.interfaces();
        assert_eq!(interfaces.len(), 2);
        assert_eq!(interfaces[0].name(), "eth0");
        assert_eq!(interfaces[1].name(), "lo");
    }
}
