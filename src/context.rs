// src/context.rs
//
// 系统上下文 - 使用依赖注入替代全局状态
// 提供 Arc<Mutex<T>> 封装的接口、ARP缓存和ICMP Echo管理器

use std::sync::{Arc, Mutex};
use crate::interface::InterfaceManager;
use crate::protocols::arp::ArpCache;
use crate::protocols::icmp::EchoManager;
use crate::protocols::tcp::TcpConnectionManager;
use crate::protocols::udp::UdpPortManager;
use crate::common::timer::TimerHandle;

/// 系统上下文，持有所有全局状态的所有权
///
/// 使用依赖注入模式替代全局状态，便于测试和并发控制。
/// 所有字段都使用 Arc<Mutex<T>> 封装以支持多线程访问。
#[derive(Clone)]
pub struct SystemContext {
    /// 接口管理器
    pub interfaces: Arc<Mutex<InterfaceManager>>,

    /// ARP 缓存
    pub arp_cache: Arc<Mutex<ArpCache>>,

    /// ICMP Echo 管理器
    pub icmp_echo: Arc<Mutex<EchoManager>>,

    /// TCP 连接管理器
    pub tcp_connections: Arc<Mutex<TcpConnectionManager>>,

    /// UDP 端口管理器
    pub udp_ports: Arc<Mutex<UdpPortManager>>,

    /// 定时器管理器（用于驱动协议状态机）
    pub timers: Arc<Mutex<TimerHandle>>,
}

impl SystemContext {
    /// 创建新的系统上下文（用于测试）
    ///
    /// 创建一个空的系统上下文，所有组件使用默认值。
    pub fn new() -> Self {
        Self {
            interfaces: Arc::new(Mutex::new(InterfaceManager::default())),
            arp_cache: Arc::new(Mutex::new(ArpCache::default())),
            icmp_echo: Arc::new(Mutex::new(EchoManager::default())),
            tcp_connections: Arc::new(Mutex::new(TcpConnectionManager::default())),
            udp_ports: Arc::new(Mutex::new(UdpPortManager::new())),
            timers: Arc::new(Mutex::new(TimerHandle::new())),
        }
    }

    /// 从配置文件创建系统上下文（生产环境使用）
    ///
    /// 加载默认配置文件初始化接口管理器，其他组件使用默认值。
    ///
    /// # 返回
    ///
    /// 返回初始化完成的 SystemContext，如果加载配置失败则使用默认值。
    pub fn from_config() -> Self {
        let interface_manager = match crate::interface::load_default_config() {
            Ok(manager) => manager,
            Err(e) => {
                eprintln!("[警告] 加载接口配置失败: {}, 使用空接口管理器", e);
                InterfaceManager::default()
            }
        };

        Self {
            interfaces: Arc::new(Mutex::new(interface_manager)),
            arp_cache: Arc::new(Mutex::new(ArpCache::default())),
            icmp_echo: Arc::new(Mutex::new(EchoManager::default())),
            tcp_connections: Arc::new(Mutex::new(TcpConnectionManager::default())),
            udp_ports: Arc::new(Mutex::new(UdpPortManager::new())),
            timers: Arc::new(Mutex::new(TimerHandle::new())),
        }
    }

    /// 使用指定组件创建系统上下文（高级用法）
    ///
    /// 允许完全自定义所有组件，用于需要精细控制的场景。
    ///
    /// # 参数
    ///
    /// - `interfaces`: 接口管理器
    /// - `arp_cache`: ARP 缓存
    /// - `icmp_echo`: ICMP Echo 管理器
    /// - `tcp_connections`: TCP 连接管理器
    /// - `udp_ports`: UDP 端口管理器（可选，默认为空）
    /// - `timers`: 定时器管理器（可选，默认为空）
    pub fn with_components(
        interfaces: Arc<Mutex<InterfaceManager>>,
        arp_cache: Arc<Mutex<ArpCache>>,
        icmp_echo: Arc<Mutex<EchoManager>>,
        tcp_connections: Arc<Mutex<TcpConnectionManager>>,
        udp_ports: Option<Arc<Mutex<UdpPortManager>>>,
        timers: Option<Arc<Mutex<TimerHandle>>>,
    ) -> Self {
        Self {
            interfaces,
            arp_cache,
            icmp_echo,
            tcp_connections,
            udp_ports: udp_ports.unwrap_or_else(|| Arc::new(Mutex::new(UdpPortManager::new()))),
            timers: timers.unwrap_or_else(|| Arc::new(Mutex::new(TimerHandle::new()))),
        }
    }

    /// 获取接口数量
    pub fn interface_count(&self) -> usize {
        self.interfaces.lock().map(|g| g.len()).unwrap_or(0)
    }
}

impl Default for SystemContext {
    fn default() -> Self {
        Self::new()
    }
}

// ========== 单元测试 ==========

#[cfg(test)]
mod tests {
    use super::*;
    use crate::interface::{InterfaceConfig, InterfaceState, MacAddr, Ipv4Addr};
    use crate::protocols::Ipv6Addr;

    // ========== 测试辅助函数 ==========

    /// 创建 eth0 配置
    fn create_eth0_config() -> InterfaceConfig {
        InterfaceConfig {
            name: "eth0".to_string(),
            mac_addr: MacAddr::new([0x00, 0x11, 0x22, 0x33, 0x44, 0x55]),
            ip_addr: Ipv4Addr::new(192, 168, 1, 100),
            ipv6_addr: Ipv6Addr::new(0xfe80, 0, 0, 0, 0, 0, 0, 1),
            netmask: Ipv4Addr::new(255, 255, 255, 0),
            gateway: Some(Ipv4Addr::new(192, 168, 1, 1)),
            mtu: Some(1500),
            state: Some(InterfaceState::Up),
        }
    }

    /// 创建测试用接口管理器
    fn create_test_manager() -> InterfaceManager {
        let mut manager = InterfaceManager::new(256, 256);
        manager.add_from_config(create_eth0_config()).unwrap();
        manager
    }

    // ========== SystemContext 基础测试组 ==========

    #[test]
    fn test_context_new() {
        let ctx = SystemContext::new();
        assert_eq!(ctx.interface_count(), 0);
    }

    #[test]
    fn test_context_default() {
        let ctx = SystemContext::default();
        assert_eq!(ctx.interface_count(), 0);
    }

    #[test]
    fn test_context_clone() {
        let ctx1 = SystemContext::new();
        let ctx2 = ctx1.clone();

        // 两个上下文共享相同的底层 Arc
        assert!(Arc::ptr_eq(&ctx1.interfaces, &ctx2.interfaces));
        assert!(Arc::ptr_eq(&ctx1.arp_cache, &ctx2.arp_cache));
        assert!(Arc::ptr_eq(&ctx1.icmp_echo, &ctx2.icmp_echo));
        assert!(Arc::ptr_eq(&ctx1.tcp_connections, &ctx2.tcp_connections));
        assert!(Arc::ptr_eq(&ctx1.udp_ports, &ctx2.udp_ports));
        assert!(Arc::ptr_eq(&ctx1.timers, &ctx2.timers));
    }

    #[test]
    fn test_context_with_components() {
        let manager = create_test_manager();
        let arp_cache = ArpCache::default();
        let echo_mgr = EchoManager::default();
        let tcp_mgr = TcpConnectionManager::default();
        let udp_mgr = UdpPortManager::new();

        let ctx = SystemContext::with_components(
            Arc::new(Mutex::new(manager)),
            Arc::new(Mutex::new(arp_cache)),
            Arc::new(Mutex::new(echo_mgr)),
            Arc::new(Mutex::new(tcp_mgr)),
            Some(Arc::new(Mutex::new(udp_mgr))),
            None,
        );

        assert_eq!(ctx.interface_count(), 1);
    }

    #[test]
    fn test_context_interface_count() {
        let ctx = SystemContext::new();
        assert_eq!(ctx.interface_count(), 0);

        // 添加接口
        ctx.interfaces.lock().unwrap()
            .add_from_config(create_eth0_config()).unwrap();

        assert_eq!(ctx.interface_count(), 1);
    }

    #[test]
    fn test_context_is_empty() {
        let ctx = SystemContext::new();
        assert_eq!(ctx.interface_count(), 0);

        ctx.interfaces.lock().unwrap()
            .add_from_config(create_eth0_config()).unwrap();

        assert_eq!(ctx.interface_count(), 1);
    }

    #[test]
    fn test_context_shared_state() {
        let ctx1 = SystemContext::new();
        let ctx2 = ctx1.clone();

        // 通过 ctx1 修改
        ctx1.interfaces.lock().unwrap()
            .add_from_config(create_eth0_config()).unwrap();

        // 通过 ctx2 可以看到修改
        assert_eq!(ctx2.interface_count(), 1);
    }

    #[test]
    fn test_context_arc_mutex_access() {
        let ctx = SystemContext::new();

        // 验证可以正常获取锁
        let iface_guard = ctx.interfaces.lock();
        assert!(iface_guard.is_ok());
        drop(iface_guard);

        let arp_guard = ctx.arp_cache.lock();
        assert!(arp_guard.is_ok());
        drop(arp_guard);

        let echo_guard = ctx.icmp_echo.lock();
        assert!(echo_guard.is_ok());
        drop(echo_guard);

        let tcp_guard = ctx.tcp_connections.lock();
        assert!(tcp_guard.is_ok());
        drop(tcp_guard);

        let udp_guard = ctx.udp_ports.lock();
        assert!(udp_guard.is_ok());
    }

    #[test]
    fn test_context_multiple_owners() {
        let ctx = SystemContext::new();
        let ctx_clone1 = ctx.clone();
        let ctx_clone2 = ctx.clone();

        // 所有克隆指向相同的底层状态
        assert!(Arc::ptr_eq(&ctx.interfaces, &ctx_clone1.interfaces));
        assert!(Arc::ptr_eq(&ctx_clone1.interfaces, &ctx_clone2.interfaces));

        // 强度计数
        assert_eq!(Arc::strong_count(&ctx.interfaces), 3);
        drop(ctx_clone2);
        assert_eq!(Arc::strong_count(&ctx.interfaces), 2);
    }
}
