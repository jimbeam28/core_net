/// 系统上下文，持有接口管理器的所有权
use crate::interface::InterfaceManager;

/// 系统上下文
///
/// 持有接口管理器的所有权（每个接口内部有自己的队列）
pub struct SystemContext {
    /// 接口管理器（包含所有接口及其队列）
    pub interfaces: InterfaceManager,
}

impl SystemContext {
    /// 创建新的系统上下文
    ///
    /// # 返回
    /// 包含接口管理器和队列资源的 SystemContext
    ///
    /// # 行为
    /// 1. 从默认配置文件加载接口配置（由 interface 模块管理）
    /// 2. 为每个接口创建独立的 RxQ 和 TxQ
    /// 3. 初始化全局接口管理器
    pub fn new() -> Self {
        // 初始化全局 ARP 缓存
        let arp_init_result = crate::protocols::arp::init_default_arp_cache();
        if let Err(e) = arp_init_result {
            eprintln!("[警告] 初始化全局 ARP 缓存失败: {}", e);
        }

        // 尝试从默认配置文件加载接口配置并初始化全局接口管理器
        let global_init_result = crate::interface::init_default();

        // 为 SystemContext 创建独立的接口管理器
        let interface_manager = match crate::interface::load_default_config() {
            Ok(manager) => manager,
            Err(e) => {
                eprintln!("[警告] 加载接口配置失败: {}, 使用空接口管理器", e);
                InterfaceManager::default()
            }
        };

        // 如果全局初始化失败，打印警告
        if let Err(e) = global_init_result {
            eprintln!("[警告] 初始化全局接口管理器失败: {}", e);
        }

        SystemContext {
            interfaces: interface_manager,
        }
    }

    // ========== 辅助接口 ==========

    /// 获取接口数量
    pub fn interface_count(&self) -> usize {
        self.interfaces.len()
    }

    /// 通过名称获取接口
    pub fn get_interface(&self, name: &str) -> Option<&crate::interface::NetworkInterface> {
        self.interfaces.get_by_name(name).ok()
    }

    /// 通过名称获取可变接口
    pub fn get_interface_mut(&mut self, name: &str) -> Option<&mut crate::interface::NetworkInterface> {
        self.interfaces.get_by_name_mut(name).ok()
    }

    /// 通过索引获取接口
    pub fn get_interface_by_index(&self, index: u32) -> Option<&crate::interface::NetworkInterface> {
        self.interfaces.get_by_index(index).ok()
    }

    /// 通过索引获取可变接口
    pub fn get_interface_by_index_mut(&mut self, index: u32) -> Option<&mut crate::interface::NetworkInterface> {
        self.interfaces.get_by_index_mut(index).ok()
    }
}
