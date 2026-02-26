// src/context.rs
//
// 系统上下文 - 使用依赖注入替代全局状态
// 提供 Arc<Mutex<T>> 封装的接口、ARP缓存和ICMP Echo管理器

use std::sync::{Arc, Mutex};
use crate::interface::InterfaceManager;
use crate::protocols::arp::ArpCache;
use crate::protocols::icmp::EchoManager;
use crate::protocols::tcp::{TcpConnectionManager, TcpSocketManager, TcpTimerManager};
use crate::protocols::udp::UdpPortManager;
use crate::protocols::bgp::BgpPeerManager;
use crate::protocols::icmpv6::Icmpv6Context;
use crate::protocols::ip::fragment::{ReassemblyTable, DEFAULT_REASSEMBLY_TIMEOUT_SECS, DEFAULT_MAX_REASSEMBLY_ENTRIES};
use crate::protocols::ospf::OspfManager;
use crate::protocols::ipv6::FragmentCache;
use crate::protocols::ipsec::{SadManager, SpdManager};
use crate::protocols::ipsec::ikev2::IkeSaManager;
use crate::common::timer::TimerHandle;
use crate::route::RouteTable;
use crate::socket::SocketManager;

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

    /// TCP Socket 管理器
    pub tcp_sockets: Arc<Mutex<TcpSocketManager>>,

    /// UDP 端口管理器
    pub udp_ports: Arc<Mutex<UdpPortManager>>,

    /// TCP 定时器管理器
    pub tcp_timers: Arc<Mutex<TcpTimerManager>>,

    /// 定时器管理器（用于驱动协议状态机）
    pub timers: Arc<Mutex<TimerHandle>>,

    /// 路由表
    pub route_table: Arc<Mutex<RouteTable>>,

    /// ICMPv6 上下文
    pub icmpv6_context: Arc<Mutex<Icmpv6Context>>,

    /// IPv4 重组表
    pub ip_reassembly: Arc<Mutex<ReassemblyTable>>,

    /// IPv6 分片重组缓存
    pub ipv6_fragment_cache: Arc<Mutex<FragmentCache>>,

    /// Socket 管理器
    pub socket_mgr: Arc<Mutex<SocketManager>>,

    /// BGP 对等体管理器
    pub bgp_manager: Arc<Mutex<BgpPeerManager>>,

    /// OSPF 管理器
    pub ospf_manager: Arc<Mutex<OspfManager>>,

    /// IPsec SAD 管理器
    pub sad_mgr: Arc<Mutex<SadManager>>,

    /// IPsec SPD 管理器
    pub spd_mgr: Arc<Mutex<SpdManager>>,

    /// IKEv2 SA 管理器
    pub ike_manager: Arc<Mutex<IkeSaManager>>,
}

/// Socket 管理器及其依赖组件
struct SocketManagers {
    tcp_sockets: Arc<Mutex<TcpSocketManager>>,
    udp_ports: Arc<Mutex<UdpPortManager>>,
    socket_mgr: Arc<Mutex<SocketManager>>,
}

/// 系统上下文配置组件（用于高级自定义）
///
/// 允许完全自定义所有组件，用于需要精细控制的场景。
pub struct ContextComponents {
    /// 接口管理器
    pub interfaces: Arc<Mutex<InterfaceManager>>,
    /// ARP 缓存
    pub arp_cache: Arc<Mutex<ArpCache>>,
    /// ICMP Echo 管理器
    pub icmp_echo: Arc<Mutex<EchoManager>>,
    /// TCP 连接管理器
    pub tcp_connections: Arc<Mutex<TcpConnectionManager>>,
    /// TCP Socket 管理器
    pub tcp_sockets: Option<Arc<Mutex<TcpSocketManager>>>,
    /// UDP 端口管理器
    pub udp_ports: Option<Arc<Mutex<UdpPortManager>>>,
    /// TCP 定时器管理器
    pub tcp_timers: Option<Arc<Mutex<TcpTimerManager>>>,
    /// 定时器管理器
    pub timers: Option<Arc<Mutex<TimerHandle>>>,
    /// 路由表
    pub route_table: Option<Arc<Mutex<RouteTable>>>,
    /// ICMPv6 上下文
    pub icmpv6_context: Option<Arc<Mutex<Icmpv6Context>>>,
    /// IPv4 重组表
    pub ip_reassembly: Option<Arc<Mutex<ReassemblyTable>>>,
    /// IPv6 分片缓存
    pub ipv6_fragment_cache: Option<Arc<Mutex<FragmentCache>>>,
    /// Socket 管理器
    pub socket_mgr: Option<Arc<Mutex<SocketManager>>>,
    /// BGP 管理器
    pub bgp_manager: Option<Arc<Mutex<BgpPeerManager>>>,
    /// OSPF 管理器
    pub ospf_manager: Option<Arc<Mutex<OspfManager>>>,
    /// IPsec SAD 管理器
    pub sad_mgr: Option<Arc<Mutex<SadManager>>>,
    /// IPsec SPD 管理器
    pub spd_mgr: Option<Arc<Mutex<SpdManager>>>,
    /// IKEv2 SA 管理器
    pub ike_manager: Option<Arc<Mutex<IkeSaManager>>>,
}

impl ContextComponents {
    /// 创建新的配置组件
    pub fn new(
        interfaces: Arc<Mutex<InterfaceManager>>,
        arp_cache: Arc<Mutex<ArpCache>>,
        icmp_echo: Arc<Mutex<EchoManager>>,
        tcp_connections: Arc<Mutex<TcpConnectionManager>>,
    ) -> Self {
        Self {
            interfaces,
            arp_cache,
            icmp_echo,
            tcp_connections,
            tcp_sockets: None,
            udp_ports: None,
            tcp_timers: None,
            timers: None,
            route_table: None,
            icmpv6_context: None,
            ip_reassembly: None,
            ipv6_fragment_cache: None,
            socket_mgr: None,
            bgp_manager: None,
            ospf_manager: None,
            sad_mgr: None,
            spd_mgr: None,
            ike_manager: None,
        }
    }

    /// 设置 TCP Socket 管理器
    pub fn with_tcp_sockets(mut self, tcp_sockets: Arc<Mutex<TcpSocketManager>>) -> Self {
        self.tcp_sockets = Some(tcp_sockets);
        self
    }

    /// 设置 UDP 端口管理器
    pub fn with_udp_ports(mut self, udp_ports: Arc<Mutex<UdpPortManager>>) -> Self {
        self.udp_ports = Some(udp_ports);
        self
    }

    /// 设置 TCP 定时器管理器
    pub fn with_tcp_timers(mut self, tcp_timers: Arc<Mutex<TcpTimerManager>>) -> Self {
        self.tcp_timers = Some(tcp_timers);
        self
    }

    /// 设置定时器管理器
    pub fn with_timers(mut self, timers: Arc<Mutex<TimerHandle>>) -> Self {
        self.timers = Some(timers);
        self
    }

    /// 设置路由表
    pub fn with_route_table(mut self, route_table: Arc<Mutex<RouteTable>>) -> Self {
        self.route_table = Some(route_table);
        self
    }

    /// 设置 ICMPv6 上下文
    pub fn with_icmpv6_context(mut self, icmpv6_context: Arc<Mutex<Icmpv6Context>>) -> Self {
        self.icmpv6_context = Some(icmpv6_context);
        self
    }

    /// 设置 IPv4 重组表
    pub fn with_ip_reassembly(mut self, ip_reassembly: Arc<Mutex<ReassemblyTable>>) -> Self {
        self.ip_reassembly = Some(ip_reassembly);
        self
    }

    /// 设置 IPv6 分片缓存
    pub fn with_ipv6_fragment_cache(mut self, ipv6_fragment_cache: Arc<Mutex<FragmentCache>>) -> Self {
        self.ipv6_fragment_cache = Some(ipv6_fragment_cache);
        self
    }

    /// 设置 Socket 管理器
    pub fn with_socket_mgr(mut self, socket_mgr: Arc<Mutex<SocketManager>>) -> Self {
        self.socket_mgr = Some(socket_mgr);
        self
    }

    /// 设置 BGP 管理器
    pub fn with_bgp_manager(mut self, bgp_manager: Arc<Mutex<BgpPeerManager>>) -> Self {
        self.bgp_manager = Some(bgp_manager);
        self
    }

    /// 设置 OSPF 管理器
    pub fn with_ospf_manager(mut self, ospf_manager: Arc<Mutex<OspfManager>>) -> Self {
        self.ospf_manager = Some(ospf_manager);
        self
    }

    /// 设置 IPsec SAD 管理器
    pub fn with_sad_mgr(mut self, sad_mgr: Arc<Mutex<SadManager>>) -> Self {
        self.sad_mgr = Some(sad_mgr);
        self
    }

    /// 设置 IPsec SPD 管理器
    pub fn with_spd_mgr(mut self, spd_mgr: Arc<Mutex<SpdManager>>) -> Self {
        self.spd_mgr = Some(spd_mgr);
        self
    }

    /// 设置 IKEv2 SA 管理器
    pub fn with_ike_manager(mut self, ike_manager: Arc<Mutex<IkeSaManager>>) -> Self {
        self.ike_manager = Some(ike_manager);
        self
    }

    /// 构建系统上下文
    pub fn build(self) -> SystemContext {
        let tcp_sockets = self.tcp_sockets.unwrap_or_else(|| Arc::new(Mutex::new(TcpSocketManager::new())));
        let udp_ports = self.udp_ports.unwrap_or_else(|| Arc::new(Mutex::new(UdpPortManager::new())));

        let socket_mgr = self.socket_mgr.unwrap_or_else(|| {
            Arc::new(Mutex::new(SocketManager::new(tcp_sockets.clone(), udp_ports.clone())))
        });

        let bgp_manager = self.bgp_manager.unwrap_or_else(|| {
            Arc::new(Mutex::new(BgpPeerManager::new(
                0,
                crate::protocols::Ipv4Addr::new(127, 0, 0, 1),
            )))
        });

        let ospf_manager = self.ospf_manager.unwrap_or_else(|| {
            Arc::new(Mutex::new(OspfManager::new(0)))
        });

        let sad_mgr = self.sad_mgr.unwrap_or_else(|| {
            Arc::new(Mutex::new(SadManager::new()))
        });

        let spd_mgr = self.spd_mgr.unwrap_or_else(|| {
            Arc::new(Mutex::new(SpdManager::new()))
        });

        let ike_manager = self.ike_manager.unwrap_or_else(|| {
            Arc::new(Mutex::new(IkeSaManager::new()))
        });

        let tcp_timers = self.tcp_timers.unwrap_or_else(|| Arc::new(Mutex::new(TcpTimerManager::new())));
        let timers = self.timers.unwrap_or_else(|| Arc::new(Mutex::new(TimerHandle::new())));

        SystemContext {
            interfaces: self.interfaces,
            arp_cache: self.arp_cache,
            icmp_echo: self.icmp_echo,
            tcp_connections: self.tcp_connections,
            tcp_sockets,
            udp_ports,
            tcp_timers,
            timers,
            route_table: self.route_table.unwrap_or_else(|| Arc::new(Mutex::new(RouteTable::new()))),
            icmpv6_context: self.icmpv6_context.unwrap_or_else(|| Arc::new(Mutex::new(Icmpv6Context::default()))),
            ip_reassembly: self.ip_reassembly.unwrap_or_else(SystemContext::create_default_reassembly),
            ipv6_fragment_cache: self.ipv6_fragment_cache.unwrap_or_else(SystemContext::create_default_ipv6_fragment_cache),
            socket_mgr,
            bgp_manager,
            ospf_manager,
            sad_mgr,
            spd_mgr,
            ike_manager,
        }
    }
}

impl SystemContext {
    /// 创建默认的 Socket 管理器及其依赖组件
    fn create_default_socket_managers() -> SocketManagers {
        let tcp_sockets = Arc::new(Mutex::new(TcpSocketManager::new()));
        let udp_ports = Arc::new(Mutex::new(UdpPortManager::new()));
        let socket_mgr = Arc::new(Mutex::new(SocketManager::new(
            tcp_sockets.clone(),
            udp_ports.clone(),
        )));
        SocketManagers {
            tcp_sockets,
            udp_ports,
            socket_mgr,
        }
    }

    /// 创建默认的重组表
    fn create_default_reassembly() -> Arc<Mutex<ReassemblyTable>> {
        Arc::new(Mutex::new(ReassemblyTable::new(
            DEFAULT_MAX_REASSEMBLY_ENTRIES,
            DEFAULT_REASSEMBLY_TIMEOUT_SECS,
        )))
    }

    /// 创建默认的 IPv6 分片缓存
    fn create_default_ipv6_fragment_cache() -> Arc<Mutex<FragmentCache>> {
        Arc::new(Mutex::new(FragmentCache::new(DEFAULT_MAX_REASSEMBLY_ENTRIES)))
    }

    /// 创建默认的协议管理器
    #[expect(clippy::type_complexity)]
    fn create_default_protocol_managers() -> (
        Arc<Mutex<TimerHandle>>,
        Arc<Mutex<TcpTimerManager>>,
        Arc<Mutex<BgpPeerManager>>,
        Arc<Mutex<OspfManager>>,
        (Arc<Mutex<SadManager>>, Arc<Mutex<SpdManager>>),
        Arc<Mutex<IkeSaManager>>,
    ) {
        let timers = Arc::new(Mutex::new(TimerHandle::new()));
        let tcp_timers = Arc::new(Mutex::new(TcpTimerManager::new()));
        let bgp_manager = Arc::new(Mutex::new(BgpPeerManager::new(
            0,
            crate::protocols::Ipv4Addr::new(127, 0, 0, 1),
        )));
        let ospf_manager = Arc::new(Mutex::new(OspfManager::new(0)));
        let sad_mgr = Arc::new(Mutex::new(SadManager::new()));
        let spd_mgr = Arc::new(Mutex::new(SpdManager::new()));
        let ike_manager = Arc::new(Mutex::new(IkeSaManager::new()));
        (timers, tcp_timers, bgp_manager, ospf_manager, (sad_mgr, spd_mgr), ike_manager)
    }

    /// 构建系统上下文的核心组件
    ///
    /// 内部辅助函数，用于减少 `new()` 和 `from_config()` 之间的重复代码。
    #[expect(clippy::too_many_arguments)]
    fn build_core(
        interfaces: InterfaceManager,
        socket_mgrs: SocketManagers,
        timers: Arc<Mutex<TimerHandle>>,
        tcp_timers: Arc<Mutex<TcpTimerManager>>,
        bgp_manager: Arc<Mutex<BgpPeerManager>>,
        ospf_manager: Arc<Mutex<OspfManager>>,
        sad_mgr: Arc<Mutex<SadManager>>,
        spd_mgr: Arc<Mutex<SpdManager>>,
        ike_manager: Arc<Mutex<IkeSaManager>>,
    ) -> Self {
        Self {
            interfaces: Arc::new(Mutex::new(interfaces)),
            arp_cache: Arc::new(Mutex::new(ArpCache::default())),
            icmp_echo: Arc::new(Mutex::new(EchoManager::default())),
            tcp_connections: Arc::new(Mutex::new(TcpConnectionManager::default())),
            tcp_sockets: socket_mgrs.tcp_sockets,
            udp_ports: socket_mgrs.udp_ports,
            tcp_timers,
            timers,
            route_table: Arc::new(Mutex::new(RouteTable::new())),
            icmpv6_context: Arc::new(Mutex::new(Icmpv6Context::default())),
            ip_reassembly: Self::create_default_reassembly(),
            ipv6_fragment_cache: Self::create_default_ipv6_fragment_cache(),
            socket_mgr: socket_mgrs.socket_mgr,
            bgp_manager,
            ospf_manager,
            sad_mgr,
            spd_mgr,
            ike_manager,
        }
    }

    /// 创建新的系统上下文（用于测试）
    ///
    /// 创建一个空的系统上下文，所有组件使用默认值。
    pub fn new() -> Self {
        let socket_mgrs = Self::create_default_socket_managers();
        let (timers, tcp_timers, bgp_manager, ospf_manager, (sad_mgr, spd_mgr), ike_manager) =
            Self::create_default_protocol_managers();

        Self::build_core(
            InterfaceManager::default(),
            socket_mgrs,
            timers,
            tcp_timers,
            bgp_manager,
            ospf_manager,
            sad_mgr,
            spd_mgr,
            ike_manager,
        )
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

        let socket_mgrs = Self::create_default_socket_managers();
        let (timers, tcp_timers, bgp_manager, ospf_manager, (sad_mgr, spd_mgr), ike_manager) =
            Self::create_default_protocol_managers();

        Self::build_core(
            interface_manager,
            socket_mgrs,
            timers,
            tcp_timers,
            bgp_manager,
            ospf_manager,
            sad_mgr,
            spd_mgr,
            ike_manager,
        )
    }

    /// 使用指定组件创建系统上下文（高级用法）
    ///
    /// 允许完全自定义所有组件，用于需要精细控制的场景。
    ///
    /// # 参数
    ///
    /// - `components`: 配置组件
    pub fn with_components(components: ContextComponents) -> Self {
        components.build()
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
        assert!(Arc::ptr_eq(&ctx1.tcp_sockets, &ctx2.tcp_sockets));
        assert!(Arc::ptr_eq(&ctx1.udp_ports, &ctx2.udp_ports));
        assert!(Arc::ptr_eq(&ctx1.timers, &ctx2.timers));
        assert!(Arc::ptr_eq(&ctx1.icmpv6_context, &ctx2.icmpv6_context));
        assert!(Arc::ptr_eq(&ctx1.ip_reassembly, &ctx2.ip_reassembly));
        assert!(Arc::ptr_eq(&ctx1.ipv6_fragment_cache, &ctx2.ipv6_fragment_cache));
        assert!(Arc::ptr_eq(&ctx1.socket_mgr, &ctx2.socket_mgr));
        assert!(Arc::ptr_eq(&ctx1.bgp_manager, &ctx2.bgp_manager));
        assert!(Arc::ptr_eq(&ctx1.sad_mgr, &ctx2.sad_mgr));
        assert!(Arc::ptr_eq(&ctx1.spd_mgr, &ctx2.spd_mgr));
    }

    #[test]
    fn test_context_with_components() {
        let manager = create_test_manager();
        let tcp_sockets = TcpSocketManager::new();
        let udp_mgr = UdpPortManager::new();

        let components = ContextComponents::new(
            Arc::new(Mutex::new(manager)),
            Arc::new(Mutex::new(ArpCache::default())),
            Arc::new(Mutex::new(EchoManager::default())),
            Arc::new(Mutex::new(TcpConnectionManager::default())),
        )
        .with_tcp_sockets(Arc::new(Mutex::new(tcp_sockets)))
        .with_udp_ports(Arc::new(Mutex::new(udp_mgr)));

        let ctx = SystemContext::with_components(components);

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

        let tcp_sockets_guard = ctx.tcp_sockets.lock();
        assert!(tcp_sockets_guard.is_ok());
        drop(tcp_sockets_guard);

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

        // 验证 tcp_sockets 也共享相同的 Arc
        assert!(Arc::ptr_eq(&ctx.tcp_sockets, &ctx_clone1.tcp_sockets));
        assert!(Arc::ptr_eq(&ctx_clone1.tcp_sockets, &ctx_clone2.tcp_sockets));

        // 验证 ip_reassembly 也共享相同的 Arc
        assert!(Arc::ptr_eq(&ctx.ip_reassembly, &ctx_clone1.ip_reassembly));
        assert!(Arc::ptr_eq(&ctx_clone1.ip_reassembly, &ctx_clone2.ip_reassembly));

        // 验证 ipv6_fragment_cache 也共享相同的 Arc
        assert!(Arc::ptr_eq(&ctx.ipv6_fragment_cache, &ctx_clone1.ipv6_fragment_cache));
        assert!(Arc::ptr_eq(&ctx_clone1.ipv6_fragment_cache, &ctx_clone2.ipv6_fragment_cache));

        // 验证 ospf_manager 也共享相同的 Arc
        assert!(Arc::ptr_eq(&ctx.ospf_manager, &ctx_clone1.ospf_manager));
        assert!(Arc::ptr_eq(&ctx_clone1.ospf_manager, &ctx_clone2.ospf_manager));

        // 验证 sad_mgr 和 spd_mgr 也共享相同的 Arc
        assert!(Arc::ptr_eq(&ctx.sad_mgr, &ctx_clone1.sad_mgr));
        assert!(Arc::ptr_eq(&ctx_clone1.sad_mgr, &ctx_clone2.sad_mgr));
        assert!(Arc::ptr_eq(&ctx.spd_mgr, &ctx_clone1.spd_mgr));
        assert!(Arc::ptr_eq(&ctx_clone1.spd_mgr, &ctx_clone2.spd_mgr));

        // 强度计数
        assert_eq!(Arc::strong_count(&ctx.interfaces), 3);
        drop(ctx_clone2);
        assert_eq!(Arc::strong_count(&ctx.interfaces), 2);
    }
}
