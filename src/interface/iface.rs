use crate::interface::types::{MacAddr, Ipv4Addr, InterfaceState, InterfaceType};
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
    /// - ip_addr: IP地址
    /// - rxq_capacity: 接收队列容量
    /// - txq_capacity: 发送队列容量
    pub fn new(
        name: String,
        index: u32,
        mac_addr: MacAddr,
        ip_addr: Ipv4Addr,
        rxq_capacity: usize,
        txq_capacity: usize,
    ) -> Self {
        Self {
            name,
            index,
            mac_addr,
            ip_addr,
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

    /// 子网掩码
    pub netmask: Ipv4Addr,

    /// 默认网关
    pub gateway: Option<Ipv4Addr>,

    /// MTU
    pub mtu: Option<u16>,

    /// 初始状态
    pub state: Option<InterfaceState>,
}
