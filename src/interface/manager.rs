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
