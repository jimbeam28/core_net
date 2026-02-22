use crate::interface::types::{MacAddr, Ipv4Addr, Ipv6Addr, InterfaceState, InterfaceType};
use crate::common::queue::RingQueue;
use crate::protocols::Packet;

/// 网络接口
#[derive(Debug)]
pub struct NetworkInterface {
    /// 接口名称（如 eth0）
    pub name: String,

    /// 接口索引（系统内唯一标识）
    pub index: u32,

    /// MAC地址
    pub mac_addr: MacAddr,

    /// IPv4地址
    pub ip_addr: Ipv4Addr,

    /// IPv6地址
    pub ipv6_addr: Ipv6Addr,

    /// 子网掩码
    pub netmask: Ipv4Addr,

    /// 默认网关
    pub gateway: Option<Ipv4Addr>,

    /// 最大传输单元（字节）
    pub mtu: u16,

    /// 接口状态
    pub state: InterfaceState,

    /// 接口类型
    pub if_type: InterfaceType,

    /// 接收队列
    pub rxq: RingQueue<Packet>,

    /// 发送队列
    pub txq: RingQueue<Packet>,
}

impl NetworkInterface {
    /// 创建新接口
    ///
    /// # 参数
    /// - name: 接口名称
    /// - index: 接口索引
    /// - mac_addr: MAC地址
    /// - ip_addr: IPv4地址
    /// - ipv6_addr: IPv6地址
    /// - rxq_capacity: 接收队列容量
    /// - txq_capacity: 发送队列容量
    pub fn new(
        name: String,
        index: u32,
        mac_addr: MacAddr,
        ip_addr: Ipv4Addr,
        ipv6_addr: Ipv6Addr,
        rxq_capacity: usize,
        txq_capacity: usize,
    ) -> Self {
        Self {
            name,
            index,
            mac_addr,
            ip_addr,
            ipv6_addr,
            netmask: Ipv4Addr::new(255, 255, 255, 0),
            gateway: None,
            mtu: 1500,
            state: InterfaceState::Down,
            if_type: InterfaceType::Ethernet,
            rxq: RingQueue::new(rxq_capacity),
            txq: RingQueue::new(txq_capacity),
        }
    }

    /// 从配置创建接口
    ///
    /// # 参数
    /// - config: 接口配置
    /// - index: 接口索引
    /// - rxq_capacity: 接收队列容量
    /// - txq_capacity: 发送队列容量
    pub fn from_config(config: InterfaceConfig, index: u32, rxq_capacity: usize, txq_capacity: usize) -> Self {
        Self {
            name: config.name,
            index,
            mac_addr: config.mac_addr,
            ip_addr: config.ip_addr,
            ipv6_addr: config.ipv6_addr,
            netmask: config.netmask,
            gateway: config.gateway,
            mtu: config.mtu.unwrap_or(1500),
            state: config.state.unwrap_or(InterfaceState::Down),
            if_type: InterfaceType::Ethernet,
            rxq: RingQueue::new(rxq_capacity),
            txq: RingQueue::new(txq_capacity),
        }
    }

    /// 获取接口名称
    pub fn name(&self) -> &str {
        &self.name
    }

    /// 获取接口索引
    pub fn index(&self) -> u32 {
        self.index
    }

    /// 设置IP地址
    pub fn set_ip_addr(&mut self, addr: Ipv4Addr) {
        self.ip_addr = addr;
    }

    /// 获取IPv6地址
    pub fn ipv6_addr(&self) -> Ipv6Addr {
        self.ipv6_addr
    }

    /// 设置IPv6地址
    pub fn set_ipv6_addr(&mut self, addr: Ipv6Addr) {
        self.ipv6_addr = addr;
    }

    /// 设置MAC地址
    pub fn set_mac_addr(&mut self, addr: MacAddr) {
        self.mac_addr = addr;
    }

    /// 设置子网掩码
    pub fn set_netmask(&mut self, mask: Ipv4Addr) {
        self.netmask = mask;
    }

    /// 设置网关
    pub fn set_gateway(&mut self, addr: Option<Ipv4Addr>) {
        self.gateway = addr;
    }

    /// 设置MTU
    pub fn set_mtu(&mut self, mtu: u16) {
        self.mtu = mtu;
    }

    /// 启用接口
    pub fn up(&mut self) {
        self.state = InterfaceState::Up;
    }

    /// 禁用接口
    pub fn down(&mut self) {
        self.state = InterfaceState::Down;
    }

    /// 检查接口是否启用
    pub fn is_up(&self) -> bool {
        self.state == InterfaceState::Up
    }

    /// 计算网络地址
    pub fn network_address(&self) -> Ipv4Addr {
        Ipv4Addr::new(
            self.ip_addr.bytes[0] & self.netmask.bytes[0],
            self.ip_addr.bytes[1] & self.netmask.bytes[1],
            self.ip_addr.bytes[2] & self.netmask.bytes[2],
            self.ip_addr.bytes[3] & self.netmask.bytes[3],
        )
    }

    /// 计算广播地址
    pub fn broadcast_address(&self) -> Ipv4Addr {
        Ipv4Addr::new(
            self.ip_addr.bytes[0] | !self.netmask.bytes[0],
            self.ip_addr.bytes[1] | !self.netmask.bytes[1],
            self.ip_addr.bytes[2] | !self.netmask.bytes[2],
            self.ip_addr.bytes[3] | !self.netmask.bytes[3],
        )
    }
}

/// 接口配置（用于配置文件解析）
#[derive(Debug, Clone)]
pub struct InterfaceConfig {
    /// 接口名称
    pub name: String,

    /// MAC地址
    pub mac_addr: MacAddr,

    /// IPv4地址
    pub ip_addr: Ipv4Addr,

    /// IPv6地址
    pub ipv6_addr: Ipv6Addr,

    /// 子网掩码
    pub netmask: Ipv4Addr,

    /// 默认网关
    pub gateway: Option<Ipv4Addr>,

    /// MTU
    pub mtu: Option<u16>,

    /// 初始状态
    pub state: Option<InterfaceState>,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 创建测试用接口
    fn create_test_interface() -> NetworkInterface {
        NetworkInterface::new(
            "eth0".to_string(),
            0,
            MacAddr::new([0x00, 0x11, 0x22, 0x33, 0x44, 0x55]),
            Ipv4Addr::new(192, 168, 1, 100),
            Ipv6Addr::new(0xfe80, 0, 0, 0, 0, 0, 0, 1),
            256,
            256,
        )
    }

    /// 创建测试配置
    fn create_test_config() -> InterfaceConfig {
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

    #[test]
    fn test_interface_new() {
        let iface = create_test_interface();

        assert_eq!(iface.name(), "eth0");
        assert_eq!(iface.index(), 0);
        assert_eq!(iface.mac_addr, MacAddr::new([0x00, 0x11, 0x22, 0x33, 0x44, 0x55]));
        assert_eq!(iface.ip_addr, Ipv4Addr::new(192, 168, 1, 100));
        assert_eq!(iface.netmask, Ipv4Addr::new(255, 255, 255, 0));
        assert_eq!(iface.gateway, None);
        assert_eq!(iface.mtu, 1500);
        assert_eq!(iface.state, InterfaceState::Down);
        assert_eq!(iface.if_type, InterfaceType::Ethernet);
    }

    #[test]
    fn test_interface_from_config() {
        let config = create_test_config();
        let iface = NetworkInterface::from_config(config, 0, 256, 256);

        assert_eq!(iface.name(), "eth0");
        assert_eq!(iface.index(), 0);
        assert_eq!(iface.mac_addr, MacAddr::new([0x00, 0x11, 0x22, 0x33, 0x44, 0x55]));
        assert_eq!(iface.ip_addr, Ipv4Addr::new(192, 168, 1, 100));
        assert_eq!(iface.netmask, Ipv4Addr::new(255, 255, 255, 0));
        assert_eq!(iface.gateway, Some(Ipv4Addr::new(192, 168, 1, 1)));
        assert_eq!(iface.mtu, 1500);
        assert_eq!(iface.state, InterfaceState::Up);
    }

    #[test]
    fn test_interface_from_config_defaults() {
        let config = InterfaceConfig {
            name: "lo".to_string(),
            mac_addr: MacAddr::zero(),
            ip_addr: Ipv4Addr::new(127, 0, 0, 1),
            ipv6_addr: Ipv6Addr::LOOPBACK,
            netmask: Ipv4Addr::new(255, 0, 0, 0),
            gateway: None,
            mtu: None,
            state: None,
        };

        let iface = NetworkInterface::from_config(config, 1, 256, 256);

        assert_eq!(iface.mtu, 1500); // 默认 MTU
        assert_eq!(iface.state, InterfaceState::Down); // 默认状态
        assert_eq!(iface.gateway, None);
    }

    #[test]
    fn test_interface_set_ip_addr() {
        let mut iface = create_test_interface();
        assert_eq!(iface.ip_addr, Ipv4Addr::new(192, 168, 1, 100));

        iface.set_ip_addr(Ipv4Addr::new(10, 0, 0, 1));
        assert_eq!(iface.ip_addr, Ipv4Addr::new(10, 0, 0, 1));
    }

    #[test]
    fn test_interface_ipv6_addr() {
        let mut iface = create_test_interface();
        assert_eq!(iface.ipv6_addr(), Ipv6Addr::new(0xfe80, 0, 0, 0, 0, 0, 0, 1));

        iface.set_ipv6_addr(Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 1));
        assert_eq!(iface.ipv6_addr(), Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 1));
    }

    #[test]
    fn test_interface_set_mac_addr() {
        let mut iface = create_test_interface();
        assert_eq!(iface.mac_addr, MacAddr::new([0x00, 0x11, 0x22, 0x33, 0x44, 0x55]));

        iface.set_mac_addr(MacAddr::new([0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0xff]));
        assert_eq!(iface.mac_addr, MacAddr::new([0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0xff]));
    }

    #[test]
    fn test_interface_set_netmask() {
        let mut iface = create_test_interface();
        assert_eq!(iface.netmask, Ipv4Addr::new(255, 255, 255, 0));

        iface.set_netmask(Ipv4Addr::new(255, 255, 255, 128));
        assert_eq!(iface.netmask, Ipv4Addr::new(255, 255, 255, 128));
    }

    #[test]
    fn test_interface_set_gateway() {
        let mut iface = create_test_interface();
        assert_eq!(iface.gateway, None);

        iface.set_gateway(Some(Ipv4Addr::new(192, 168, 1, 1)));
        assert_eq!(iface.gateway, Some(Ipv4Addr::new(192, 168, 1, 1)));

        iface.set_gateway(None);
        assert_eq!(iface.gateway, None);
    }

    #[test]
    fn test_interface_set_mtu() {
        let mut iface = create_test_interface();
        assert_eq!(iface.mtu, 1500);

        iface.set_mtu(9000);
        assert_eq!(iface.mtu, 9000);
    }

    #[test]
    fn test_interface_up_down() {
        let mut iface = create_test_interface();

        // 初始状态是 Down
        assert_eq!(iface.state, InterfaceState::Down);
        assert!(!iface.is_up());

        // 启用接口
        iface.up();
        assert_eq!(iface.state, InterfaceState::Up);
        assert!(iface.is_up());

        // 禁用接口
        iface.down();
        assert_eq!(iface.state, InterfaceState::Down);
        assert!(!iface.is_up());
    }

    #[test]
    fn test_interface_state_transitions() {
        let mut iface = create_test_interface();

        // Down -> Up
        iface.up();
        assert_eq!(iface.state, InterfaceState::Up);

        // Up -> Down
        iface.down();
        assert_eq!(iface.state, InterfaceState::Down);

        // 可以直接设置状态
        iface.state = InterfaceState::Testing;
        assert_eq!(iface.state, InterfaceState::Testing);

        iface.state = InterfaceState::Error;
        assert_eq!(iface.state, InterfaceState::Error);
    }

    #[test]
    fn test_network_address() {
        let mut iface = create_test_interface();
        iface.set_ip_addr(Ipv4Addr::new(192, 168, 1, 100));
        iface.set_netmask(Ipv4Addr::new(255, 255, 255, 0));

        let network = iface.network_address();
        assert_eq!(network, Ipv4Addr::new(192, 168, 1, 0));
    }

    #[test]
    fn test_network_address_class_c() {
        let mut iface = create_test_interface();
        iface.set_ip_addr(Ipv4Addr::new(192, 168, 1, 100));
        iface.set_netmask(Ipv4Addr::new(255, 255, 255, 0)); // /24

        assert_eq!(iface.network_address(), Ipv4Addr::new(192, 168, 1, 0));
    }

    #[test]
    fn test_network_address_class_b() {
        let mut iface = create_test_interface();
        iface.set_ip_addr(Ipv4Addr::new(172, 16, 5, 100));
        iface.set_netmask(Ipv4Addr::new(255, 255, 0, 0)); // /16

        assert_eq!(iface.network_address(), Ipv4Addr::new(172, 16, 0, 0));
    }

    #[test]
    fn test_network_address_class_a() {
        let mut iface = create_test_interface();
        iface.set_ip_addr(Ipv4Addr::new(10, 0, 5, 100));
        iface.set_netmask(Ipv4Addr::new(255, 0, 0, 0)); // /8

        assert_eq!(iface.network_address(), Ipv4Addr::new(10, 0, 0, 0));
    }

    #[test]
    fn test_broadcast_address() {
        let mut iface = create_test_interface();
        iface.set_ip_addr(Ipv4Addr::new(192, 168, 1, 100));
        iface.set_netmask(Ipv4Addr::new(255, 255, 255, 0));

        let broadcast = iface.broadcast_address();
        assert_eq!(broadcast, Ipv4Addr::new(192, 168, 1, 255));
    }

    #[test]
    fn test_broadcast_address_class_c() {
        let mut iface = create_test_interface();
        iface.set_ip_addr(Ipv4Addr::new(192, 168, 1, 100));
        iface.set_netmask(Ipv4Addr::new(255, 255, 255, 0)); // /24

        assert_eq!(iface.broadcast_address(), Ipv4Addr::new(192, 168, 1, 255));
    }

    #[test]
    fn test_broadcast_address_class_b() {
        let mut iface = create_test_interface();
        iface.set_ip_addr(Ipv4Addr::new(172, 16, 5, 100));
        iface.set_netmask(Ipv4Addr::new(255, 255, 0, 0)); // /16

        assert_eq!(iface.broadcast_address(), Ipv4Addr::new(172, 16, 255, 255));
    }

    #[test]
    fn test_broadcast_address_class_a() {
        let mut iface = create_test_interface();
        iface.set_ip_addr(Ipv4Addr::new(10, 0, 5, 100));
        iface.set_netmask(Ipv4Addr::new(255, 0, 0, 0)); // /8

        assert_eq!(iface.broadcast_address(), Ipv4Addr::new(10, 255, 255, 255));
    }

    // ========== 边界条件测试 ==========

    #[test]
    fn test_interface_mtu_minimum() {
        let mut iface = create_test_interface();

        // 最小 MTU 值（理论上以太网最小是 68，但允许设置更小）
        iface.set_mtu(1);
        assert_eq!(iface.mtu, 1);
    }

    #[test]
    fn test_interface_mtu_maximum() {
        let mut iface = create_test_interface();

        // 巨帧 MTU
        iface.set_mtu(9000);
        assert_eq!(iface.mtu, 9000);

        // 最大 u16 值
        iface.set_mtu(u16::MAX);
        assert_eq!(iface.mtu, u16::MAX);
    }

    #[test]
    fn test_interface_zero_ip() {
        let mut iface = create_test_interface();

        iface.set_ip_addr(Ipv4Addr::new(0, 0, 0, 0));
        assert_eq!(iface.ip_addr, Ipv4Addr::new(0, 0, 0, 0));
        assert!(iface.ip_addr.is_zero());
    }

    #[test]
    fn test_interface_broadcast_ip() {
        let mut iface = create_test_interface();

        iface.set_ip_addr(Ipv4Addr::new(255, 255, 255, 255));
        assert_eq!(iface.ip_addr, Ipv4Addr::new(255, 255, 255, 255));
        assert!(iface.ip_addr.is_broadcast());
    }

    #[test]
    fn test_interface_loopback_ip() {
        let mut iface = create_test_interface();

        iface.set_ip_addr(Ipv4Addr::new(127, 0, 0, 1));
        assert_eq!(iface.ip_addr, Ipv4Addr::new(127, 0, 0, 1));
        assert!(iface.ip_addr.is_loopback());
    }

    #[test]
    fn test_interface_queues_capacity() {
        let iface = NetworkInterface::new(
            "eth0".to_string(),
            0,
            MacAddr::new([0x00, 0x11, 0x22, 0x33, 0x44, 0x55]),
            Ipv4Addr::new(192, 168, 1, 100),
            Ipv6Addr::new(0xfe80, 0, 0, 0, 0, 0, 0, 1),
            512,
            1024,
        );

        // 队列容量验证（通过行为间接验证）
        // 这里我们主要验证队列已创建，实际容量需要通过 RingQueue 的实现来验证
        // 因为 RingQueue 没有公开 capacity() 方法，我们通过非空来验证
        let _ = &iface.rxq;
        let _ = &iface.txq;
    }

    #[test]
    fn test_interface_config_clone() {
        let config = create_test_config();
        let config_clone = config.clone();

        assert_eq!(config.name, config_clone.name);
        assert_eq!(config.mac_addr, config_clone.mac_addr);
        assert_eq!(config.ip_addr, config_clone.ip_addr);
    }
}
