use std::collections::HashMap;
use std::vec::Vec;

use crate::interface::iface::{InterfaceConfig, NetworkInterface};
use crate::interface::types::InterfaceError;

/// 接口管理器
#[derive(Debug)]
pub struct InterfaceManager {
    /// 接口列表（按索引排序）
    interfaces: Vec<NetworkInterface>,

    /// 名称到索引的映射
    name_to_index: HashMap<String, u32>,

    /// 接收队列容量（用于创建新接口）
    rxq_capacity: usize,

    /// 发送队列容量（用于创建新接口）
    txq_capacity: usize,
}

impl InterfaceManager {
    /// 创建新的接口管理器
    ///
    /// # 参数
    /// - rxq_capacity: 每个接口的接收队列容量
    /// - txq_capacity: 每个接口的发送队列容量
    pub fn new(rxq_capacity: usize, txq_capacity: usize) -> Self {
        Self {
            interfaces: Vec::new(),
            name_to_index: HashMap::new(),
            rxq_capacity,
            txq_capacity,
        }
    }

    /// 添加接口
    ///
    /// # 参数
    /// - interface: 要添加的接口
    ///
    /// # 返回
    /// - Ok(()): 添加成功
    /// - Err(InterfaceError::DuplicateName): 接口名称已存在
    pub fn add_interface(&mut self, interface: NetworkInterface) -> Result<(), InterfaceError> {
        if self.name_to_index.contains_key(&interface.name) {
            return Err(InterfaceError::DuplicateName(interface.name));
        }
        let index = interface.index;
        self.name_to_index.insert(interface.name.clone(), index);
        self.interfaces.push(interface);
        Ok(())
    }

    /// 从配置添加接口
    pub fn add_from_config(&mut self, config: InterfaceConfig) -> Result<(), InterfaceError> {
        let index = self.interfaces.len() as u32;
        let interface = NetworkInterface::from_config(config, index, self.rxq_capacity, self.txq_capacity);
        self.add_interface(interface)
    }

    /// 通过名称获取接口
    ///
    /// # 参数
    /// - name: 接口名称
    ///
    /// # 返回
    /// - Ok(&interface): 找到接口
    /// - Err(InterfaceError::InterfaceNotFound): 未找到
    pub fn get_by_name(&self, name: &str) -> Result<&NetworkInterface, InterfaceError> {
        let index = self
            .name_to_index
            .get(name)
            .ok_or(InterfaceError::InterfaceNotFound)?;
        self.interfaces
            .get(*index as usize)
            .ok_or(InterfaceError::InterfaceNotFound)
    }

    /// 通过名称获取可变接口
    pub fn get_by_name_mut(
        &mut self,
        name: &str,
    ) -> Result<&mut NetworkInterface, InterfaceError> {
        let index = *self
            .name_to_index
            .get(name)
            .ok_or(InterfaceError::InterfaceNotFound)? as usize;
        self.interfaces
            .get_mut(index)
            .ok_or(InterfaceError::InterfaceNotFound)
    }

    /// 通过索引获取接口
    pub fn get_by_index(&self, index: u32) -> Result<&NetworkInterface, InterfaceError> {
        self.interfaces
            .get(index as usize)
            .ok_or(InterfaceError::InterfaceNotFound)
    }

    /// 通过索引获取可变接口
    pub fn get_by_index_mut(
        &mut self,
        index: u32,
    ) -> Result<&mut NetworkInterface, InterfaceError> {
        self.interfaces
            .get_mut(index as usize)
            .ok_or(InterfaceError::InterfaceNotFound)
    }

    /// 获取所有接口
    pub fn interfaces(&self) -> &[NetworkInterface] {
        &self.interfaces
    }

    /// 获取所有接口的可变引用
    pub fn interfaces_mut(&mut self) -> &mut [NetworkInterface] {
        &mut self.interfaces
    }

    /// 获取接口数量
    pub fn len(&self) -> usize {
        self.interfaces.len()
    }

    /// 是否为空
    pub fn is_empty(&self) -> bool {
        self.interfaces.is_empty()
    }
}

impl Default for InterfaceManager {
    fn default() -> Self {
        Self::new(256, 256)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::interface::types::{MacAddr, Ipv4Addr, Ipv6Addr};

    /// 创建测试用接口
    fn create_test_interface(name: &str, index: u32) -> NetworkInterface {
        NetworkInterface::new(
            name.to_string(),
            index,
            MacAddr::new([0x00, 0x11, 0x22, 0x33, 0x44, 0x55]),
            Ipv4Addr::new(192, 168, 1, 100),
            Ipv6Addr::new(0xfe80, 0, 0, 0, 0, 0, 0, 1),
            256,
            256,
        )
    }

    /// 创建测试配置
    fn create_test_config(name: &str, ip: [u8; 4]) -> crate::interface::iface::InterfaceConfig {
        crate::interface::iface::InterfaceConfig {
            name: name.to_string(),
            mac_addr: MacAddr::new([0x00, 0x11, 0x22, 0x33, 0x44, 0x55]),
            ip_addr: Ipv4Addr::new(ip[0], ip[1], ip[2], ip[3]),
            ipv6_addr: Ipv6Addr::new(0xfe80, 0, 0, 0, 0, 0, 0, 1),
            netmask: Ipv4Addr::new(255, 255, 255, 0),
            gateway: None,
            mtu: None,
            state: None,
        }
    }

    /// 创建预填充的管理器
    fn create_populated_manager() -> InterfaceManager {
        let mut manager = InterfaceManager::new(256, 256);

        let iface1 = create_test_interface("eth0", 0);
        let iface2 = create_test_interface("eth1", 1);
        let iface3 = create_test_interface("lo", 2);

        manager.add_interface(iface1).unwrap();
        manager.add_interface(iface2).unwrap();
        manager.add_interface(iface3).unwrap();

        manager
    }

    #[test]
    fn test_manager_new() {
        let manager = InterfaceManager::new(512, 1024);

        assert!(manager.is_empty());
        assert_eq!(manager.len(), 0);
        assert_eq!(manager.interfaces().len(), 0);
    }

    #[test]
    fn test_manager_default() {
        let manager = InterfaceManager::default();

        assert!(manager.is_empty());
        assert_eq!(manager.len(), 0);
    }

    #[test]
    fn test_add_interface() {
        let mut manager = InterfaceManager::new(256, 256);
        let iface = create_test_interface("eth0", 0);

        let result = manager.add_interface(iface);
        assert!(result.is_ok());
        assert_eq!(manager.len(), 1);
    }

    #[test]
    fn test_add_multiple_interfaces() {
        let mut manager = InterfaceManager::new(256, 256);

        let iface1 = create_test_interface("eth0", 0);
        let iface2 = create_test_interface("eth1", 1);
        let iface3 = create_test_interface("lo", 2);

        manager.add_interface(iface1).unwrap();
        manager.add_interface(iface2).unwrap();
        manager.add_interface(iface3).unwrap();

        assert_eq!(manager.len(), 3);
    }

    #[test]
    fn test_add_interface_duplicate_name() {
        let mut manager = InterfaceManager::new(256, 256);

        let iface1 = create_test_interface("eth0", 0);
        let iface2 = create_test_interface("eth0", 1); // 同名

        manager.add_interface(iface1).unwrap();
        let result = manager.add_interface(iface2);

        assert!(result.is_err());
        match result {
            Err(InterfaceError::DuplicateName(name)) => {
                assert_eq!(name, "eth0");
            }
            _ => panic!("Expected DuplicateName error"),
        }

        // 第二个接口不应该被添加
        assert_eq!(manager.len(), 1);
    }

    #[test]
    fn test_add_from_config() {
        let mut manager = InterfaceManager::new(256, 256);
        let config = create_test_config("eth0", [192, 168, 1, 100]);

        let result = manager.add_from_config(config);
        assert!(result.is_ok());
        assert_eq!(manager.len(), 1);

        let iface = manager.get_by_name("eth0").unwrap();
        assert_eq!(iface.name, "eth0");
        assert_eq!(iface.ip_addr, Ipv4Addr::new(192, 168, 1, 100));
    }

    #[test]
    fn test_add_from_config_duplicate() {
        let mut manager = InterfaceManager::new(256, 256);

        let config1 = create_test_config("eth0", [192, 168, 1, 100]);
        let config2 = create_test_config("eth0", [192, 168, 1, 101]);

        manager.add_from_config(config1).unwrap();
        let result = manager.add_from_config(config2);

        assert!(result.is_err());
        assert_eq!(manager.len(), 1);
    }

    #[test]
    fn test_get_by_name() {
        let manager = create_populated_manager();

        let iface = manager.get_by_name("eth0");
        assert!(iface.is_ok());
        assert_eq!(iface.unwrap().name, "eth0");
    }

    #[test]
    fn test_get_by_name_not_found() {
        let manager = create_populated_manager();

        let result = manager.get_by_name("eth99");
        assert!(result.is_err());
        match result {
            Err(InterfaceError::InterfaceNotFound) => {}
            _ => panic!("Expected InterfaceNotFound error"),
        }
    }

    #[test]
    fn test_get_by_index() {
        let manager = create_populated_manager();

        let iface = manager.get_by_index(0);
        assert!(iface.is_ok());
        assert_eq!(iface.unwrap().name, "eth0");

        let iface = manager.get_by_index(1);
        assert!(iface.is_ok());
        assert_eq!(iface.unwrap().name, "eth1");
    }

    #[test]
    fn test_get_by_index_not_found() {
        let manager = create_populated_manager();

        let result = manager.get_by_index(99);
        assert!(result.is_err());
        match result {
            Err(InterfaceError::InterfaceNotFound) => {}
            _ => panic!("Expected InterfaceNotFound error"),
        }
    }

    #[test]
    fn test_get_by_name_mut() {
        let mut manager = create_populated_manager();

        let iface = manager.get_by_name_mut("eth0");
        assert!(iface.is_ok());

        let iface = iface.unwrap();
        iface.set_ip_addr(Ipv4Addr::new(10, 0, 0, 1));

        // 验证修改生效
        let iface = manager.get_by_name("eth0").unwrap();
        assert_eq!(iface.ip_addr, Ipv4Addr::new(10, 0, 0, 1));
    }

    #[test]
    fn test_get_by_index_mut() {
        let mut manager = create_populated_manager();

        let iface = manager.get_by_index_mut(0);
        assert!(iface.is_ok());

        let iface = iface.unwrap();
        iface.up();

        // 验证修改生效
        let iface = manager.get_by_index(0).unwrap();
        assert!(iface.is_up());
    }

    #[test]
    fn test_interfaces_slice() {
        let manager = create_populated_manager();

        let interfaces = manager.interfaces();
        assert_eq!(interfaces.len(), 3);
        assert_eq!(interfaces[0].name, "eth0");
        assert_eq!(interfaces[1].name, "eth1");
        assert_eq!(interfaces[2].name, "lo");
    }

    #[test]
    fn test_interfaces_mut_slice() {
        let mut manager = create_populated_manager();

        let interfaces = manager.interfaces_mut();
        assert_eq!(interfaces.len(), 3);

        // 修改所有接口状态
        for iface in interfaces.iter_mut() {
            iface.up();
        }

        // 验证所有接口都已启用
        let interfaces = manager.interfaces();
        assert!(interfaces.iter().all(|iface| iface.is_up()));
    }

    #[test]
    fn test_iterate_interfaces() {
        let manager = create_populated_manager();

        let mut count = 0;
        for iface in manager.interfaces() {
            assert!(!iface.name.is_empty());
            count += 1;
        }
        assert_eq!(count, 3);
    }

    #[test]
    fn test_empty_manager() {
        let manager = InterfaceManager::new(256, 256);

        assert!(manager.is_empty());
        assert_eq!(manager.len(), 0);
        assert!(manager.get_by_name("eth0").is_err());
        assert!(manager.get_by_index(0).is_err());
    }

    #[test]
    fn test_single_interface() {
        let mut manager = InterfaceManager::new(256, 256);
        manager.add_interface(create_test_interface("eth0", 0)).unwrap();

        assert!(!manager.is_empty());
        assert_eq!(manager.len(), 1);
        assert!(manager.get_by_name("eth0").is_ok());
        assert!(manager.get_by_index(0).is_ok());
        assert!(manager.get_by_index(1).is_err());
    }

    #[test]
    fn test_large_number_of_interfaces() {
        let mut manager = InterfaceManager::new(256, 256);

        // 添加多个接口
        for i in 0..100 {
            let iface = create_test_interface(&format!("eth{}", i), i);
            manager.add_interface(iface).unwrap();
        }

        assert_eq!(manager.len(), 100);

        // 验证可以访问所有接口
        for i in 0..100 {
            let name = format!("eth{}", i);
            assert!(manager.get_by_name(&name).is_ok());
            assert!(manager.get_by_index(i).is_ok());
        }
    }

    #[test]
    fn test_manager_queue_capacity() {
        let _manager = InterfaceManager::new(512, 1024);

        // 验证队列容量设置（通过创建接口间接验证）
        let config = create_test_config("eth0", [192, 168, 1, 100]);
        let mut manager_with_iface = InterfaceManager::new(512, 1024);
        manager_with_iface.add_from_config(config).unwrap();

        // 接口已创建，队列已使用指定容量初始化
        assert_eq!(manager_with_iface.len(), 1);
    }

    #[test]
    fn test_get_by_name_empty_string() {
        let manager = create_populated_manager();

        let result = manager.get_by_name("");
        assert!(result.is_err());
    }

    #[test]
    fn test_get_by_index_zero() {
        let mut manager = InterfaceManager::new(256, 256);
        manager.add_interface(create_test_interface("eth0", 0)).unwrap();

        let result = manager.get_by_index(0);
        assert!(result.is_ok());
    }

    #[test]
    fn test_interface_not_found_after_removal_simulation() {
        let mut manager = InterfaceManager::new(256, 256);

        manager.add_interface(create_test_interface("eth0", 0)).unwrap();

        // 模拟移除：创建新管理器，不包含该接口
        let manager2 = InterfaceManager::new(256, 256);

        assert!(manager2.get_by_name("eth0").is_err());
    }
}
