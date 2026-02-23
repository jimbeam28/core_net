// src/protocols/udp/port.rs
//
// UDP 端口管理模块
// 实现端口绑定表、端口分配、释放和查找功能

use crate::common::{CoreError, Result};
use crate::protocols::Ipv4Addr;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// 知名端口范围 (0-1023)
pub const WELL_KNOWN_PORT_MIN: u16 = 0;
pub const WELL_KNOWN_PORT_MAX: u16 = 1023;

/// 注册端口范围 (1024-49151)
pub const REGISTERED_PORT_MIN: u16 = 1024;
pub const REGISTERED_PORT_MAX: u16 = 49151;

/// 动态/私有端口范围 (49152-65535)
pub const EPHEMERAL_PORT_MIN: u16 = 49152;
pub const EPHEMERAL_PORT_MAX: u16 = 65535;

/// 有效端口范围
pub const PORT_MIN: u16 = 1;
pub const PORT_MAX: u16 = 65535;

/// UDP 数据接收回调函数类型
///
/// # 参数
/// - source_addr: 发送方 IP 地址
/// - source_port: 发送方端口号
/// - data: 接收到的数据
pub type UdpReceiveCallback = Box<dyn Fn(Ipv4Addr, u16, Vec<u8>) + Send>;

/// 端口入口结构
///
/// 表示一个绑定的端口及其关联的处理器。
#[derive(Clone)]
pub struct PortEntry {
    /// 端口号
    pub port: u16,
    /// 是否为知名端口（受保护）
    pub is_well_known: bool,
    /// 数据接收回调
    pub callback: Arc<Mutex<Option<UdpReceiveCallback>>>,
}

impl PortEntry {
    /// 创建新的端口入口
    pub fn new(port: u16) -> Self {
        Self {
            port,
            is_well_known: port <= WELL_KNOWN_PORT_MAX,
            callback: Arc::new(Mutex::new(None)),
        }
    }

    /// 设置接收回调
    pub fn set_callback(&self, callback: UdpReceiveCallback) {
        *self.callback.lock().unwrap() = Some(callback);
    }

    /// 移除接收回调
    pub fn clear_callback(&self) {
        *self.callback.lock().unwrap() = None;
    }

    /// 检查是否有回调
    pub fn has_callback(&self) -> bool {
        self.callback.lock().unwrap().is_some()
    }

    /// 调用回调（如果存在）
    pub fn invoke_callback(&self, source_addr: Ipv4Addr, source_port: u16, data: Vec<u8>) {
        if let Some(callback) = self.callback.lock().unwrap().as_ref() {
            callback(source_addr, source_port, data);
        }
    }
}

/// UDP 端口管理器
///
/// 管理所有绑定的 UDP 端口，提供端口分配、释放和查找功能。
pub struct UdpPortManager {
    /// 端口绑定表：port -> PortEntry
    port_table: HashMap<u16, PortEntry>,
    /// 动态端口分配指针
    ephemeral_pointer: u16,
}

impl Default for UdpPortManager {
    fn default() -> Self {
        Self {
            port_table: HashMap::new(),
            ephemeral_pointer: EPHEMERAL_PORT_MIN,
        }
    }
}

impl UdpPortManager {
    /// 创建新的端口管理器
    pub fn new() -> Self {
        Self::default()
    }

    /// 绑定端口
    ///
    /// # 参数
    /// - port: 要绑定的端口号（0 表示自动分配）
    ///
    /// # 返回
    /// - Ok(u16): 绑定的端口号
    /// - Err(CoreError): 绑定失败（端口已被占用或无效）
    pub fn bind(&mut self, port: u16) -> Result<u16> {
        // 端口为 0 表示自动分配
        if port == 0 {
            return self.allocate_ephemeral_port();
        }

        // 验证端口范围
        if !(PORT_MIN..=PORT_MAX).contains(&port) {
            return Err(CoreError::invalid_packet(format!(
                "Invalid port number: {} (must be 1-65535)",
                port
            )));
        }

        // 检查端口是否已被绑定
        if self.port_table.contains_key(&port) {
            return Err(CoreError::invalid_packet(format!(
                "Port {} is already bound",
                port
            )));
        }

        // 创建端口入口
        let entry = PortEntry::new(port);
        self.port_table.insert(port, entry);

        Ok(port)
    }

    /// 解绑端口
    ///
    /// # 参数
    /// - port: 要解绑的端口号
    ///
    /// # 返回
    /// - Ok(()): 解绑成功
    /// - Err(CoreError): 解绑失败（端口未绑定）
    pub fn unbind(&mut self, port: u16) -> Result<()> {
        match self.port_table.remove(&port) {
            Some(_) => Ok(()),
            None => Err(CoreError::invalid_packet(format!(
                "Port {} is not bound",
                port
            ))),
        }
    }

    /// 查找端口入口
    ///
    /// # 参数
    /// - port: 端口号
    ///
    /// # 返回
    /// - Option<&PortEntry>: 端口入口（如果存在）
    pub fn lookup(&self, port: u16) -> Option<&PortEntry> {
        self.port_table.get(&port)
    }

    /// 检查端口是否已绑定
    ///
    /// # 参数
    /// - port: 端口号
    ///
    /// # 返回
    /// - bool: 端口是否已绑定
    pub fn is_bound(&self, port: u16) -> bool {
        self.port_table.contains_key(&port)
    }

    /// 获取已绑定端口列表
    ///
    /// # 返回
    /// - Vec<u16>: 已绑定的端口号列表
    pub fn bound_ports(&self) -> Vec<u16> {
        let mut ports: Vec<u16> = self.port_table.keys().copied().collect();
        ports.sort();
        ports
    }

    /// 获取绑定端口数量
    ///
    /// # 返回
    /// - usize: 已绑定的端口数量
    pub fn bound_count(&self) -> usize {
        self.port_table.len()
    }

    /// 分配动态端口
    ///
    /// 在 49152-65535 范围内分配一个未使用的端口。
    ///
    /// # 返回
    /// - Ok(u16): 分配的端口号
    /// - Err(CoreError): 分配失败（没有可用端口）
    fn allocate_ephemeral_port(&mut self) -> Result<u16> {
        let start = self.ephemeral_pointer;
        let mut port = start;

        loop {
            // 检查端口是否可用
            if !self.is_bound(port) {
                self.ephemeral_pointer = if port == EPHEMERAL_PORT_MAX {
                    EPHEMERAL_PORT_MIN
                } else {
                    port + 1
                };

                let entry = PortEntry::new(port);
                self.port_table.insert(port, entry);
                return Ok(port);
            }

            // 移动到下一个端口
            port = if port == EPHEMERAL_PORT_MAX {
                EPHEMERAL_PORT_MIN
            } else {
                port + 1
            };

            // 检查是否遍历了一圈
            if port == start {
                return Err(CoreError::invalid_packet(
                    "No ephemeral ports available"
                ));
            }
        }
    }

    /// 清空所有端口绑定
    pub fn clear(&mut self) {
        self.port_table.clear();
        self.ephemeral_pointer = EPHEMERAL_PORT_MIN;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bind_specific_port() {
        let mut manager = UdpPortManager::new();

        // 绑定端口 8080
        let port = manager.bind(8080).unwrap();
        assert_eq!(port, 8080);
        assert!(manager.is_bound(8080));
    }

    #[test]
    fn test_bind_already_bound() {
        let mut manager = UdpPortManager::new();

        manager.bind(8080).unwrap();
        let result = manager.bind(8080);
        assert!(result.is_err());
    }

    #[test]
    fn test_bind_invalid_port() {
        let mut manager = UdpPortManager::new();

        // 端口 0 表示自动分配
        assert!(manager.bind(0).is_ok()); // 自动分配

        // 端口号 0-65535 都是有效的 u16 范围
        // 无效端口号无法通过类型系统传递给 bind 函数
    }

    #[test]
    fn test_bind_auto_allocate() {
        let mut manager = UdpPortManager::new();

        // 端口 0 自动分配
        let port1 = manager.bind(0).unwrap();
        assert!(port1 >= EPHEMERAL_PORT_MIN);

        // 再次分配应该得到不同的端口
        let port2 = manager.bind(0).unwrap();
        assert_ne!(port1, port2);
    }

    #[test]
    fn test_unbind_port() {
        let mut manager = UdpPortManager::new();

        manager.bind(8080).unwrap();
        assert!(manager.is_bound(8080));

        manager.unbind(8080).unwrap();
        assert!(!manager.is_bound(8080));
    }

    #[test]
    fn test_unbind_not_bound() {
        let mut manager = UdpPortManager::new();

        let result = manager.unbind(8080);
        assert!(result.is_err());
    }

    #[test]
    fn test_lookup_port() {
        let mut manager = UdpPortManager::new();

        manager.bind(8080).unwrap();
        let entry = manager.lookup(8080);
        assert!(entry.is_some());
        assert_eq!(entry.unwrap().port, 8080);

        assert!(manager.lookup(9090).is_none());
    }

    #[test]
    fn test_bound_ports_list() {
        let mut manager = UdpPortManager::new();

        manager.bind(8080).unwrap();
        manager.bind(9090).unwrap();
        manager.bind(53).unwrap();

        let ports = manager.bound_ports();
        assert_eq!(ports, vec![53, 8080, 9090]);
    }

    #[test]
    fn test_bound_count() {
        let mut manager = UdpPortManager::new();

        assert_eq!(manager.bound_count(), 0);

        manager.bind(8080).unwrap();
        assert_eq!(manager.bound_count(), 1);

        manager.bind(9090).unwrap();
        assert_eq!(manager.bound_count(), 2);

        manager.unbind(8080).unwrap();
        assert_eq!(manager.bound_count(), 1);
    }

    #[test]
    fn test_ephemeral_port_exhaustion() {
        let mut manager = UdpPortManager::new();

        // 绑定所有动态端口
        for port in EPHEMERAL_PORT_MIN..=EPHEMERAL_PORT_MAX {
            manager.bind(port).unwrap();
        }

        // 应该无法分配更多
        let result = manager.bind(0);
        assert!(result.is_err());
    }

    #[test]
    fn test_port_entry_callback() {
        let entry = PortEntry::new(8080);

        assert!(!entry.has_callback());

        // 设置回调
        entry.set_callback(Box::new(|_src_addr, _src_port, _data| {
            // 回调逻辑
        }));

        assert!(entry.has_callback());

        // 清除回调
        entry.clear_callback();
        assert!(!entry.has_callback());
    }

    #[test]
    fn test_clear() {
        let mut manager = UdpPortManager::new();

        manager.bind(8080).unwrap();
        manager.bind(9090).unwrap();
        assert_eq!(manager.bound_count(), 2);

        manager.clear();
        assert_eq!(manager.bound_count(), 0);
        assert!(!manager.is_bound(8080));
    }
}
