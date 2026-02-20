// 测试公共模块
//
// 提供各测试文件共用的辅助函数和配置

use core_net::testframework::{GlobalStateManager, HarnessError, HarnessResult};
use core_net::interface::{InterfaceConfig, InterfaceState, MacAddr, Ipv4Addr, NetworkInterface};
use core_net::protocols::arp::{ArpPacket, ArpOperation, encapsulate_ethernet};
use core_net::protocols::IP_PROTO_ICMP;
use core_net::protocols::ip::Ipv4Header;
use core_net::common::Packet;

// ========== 测试配置 ==========

/// 创建测试用 eth0 配置
#[allow(dead_code)]
pub fn create_test_eth0_config() -> InterfaceConfig {
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

/// 创建测试用 lo 配置
#[allow(dead_code)]
pub fn create_test_lo_config() -> InterfaceConfig {
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

// ========== 报文创建函数 ==========

/// 创建 IP 头部
#[allow(dead_code)]
pub fn create_ip_header(src_ip: Ipv4Addr, dst_ip: Ipv4Addr, payload_len: usize) -> Vec<u8> {
    let ip_header = Ipv4Header::new(src_ip, dst_ip, IP_PROTO_ICMP, payload_len);
    ip_header.to_bytes()
}

/// 创建ARP请求报文（带以太网封装）
/// src_mac: 源MAC地址, src_ip: 源IP地址, dst_ip: 目标IP地址
#[allow(dead_code)]
pub fn create_arp_request_packet(
    src_mac: MacAddr,
    src_ip: Ipv4Addr,
    dst_ip: Ipv4Addr,
) -> Packet {
    let arp_packet = ArpPacket::new(
        ArpOperation::Request,
        src_mac,
        src_ip,
        MacAddr::broadcast(),
        dst_ip,
    );

    let frame = encapsulate_ethernet(&arp_packet, MacAddr::broadcast(), src_mac);
    Packet::from_bytes(frame)
}

/// 创建ARP响应报文（带以太网封装）
/// src_mac: 响应者的MAC, src_ip: 响应者的IP
/// dst_mac: 请求者的MAC, dst_ip: 请求者的IP
#[allow(dead_code)]
pub fn create_arp_reply_packet(
    src_mac: MacAddr,
    src_ip: Ipv4Addr,
    dst_mac: MacAddr,
    dst_ip: Ipv4Addr,
) -> Packet {
    let arp_packet = ArpPacket::new(
        ArpOperation::Reply,
        src_mac,
        src_ip,
        dst_mac,
        dst_ip,
    );

    let frame = encapsulate_ethernet(&arp_packet, dst_mac, src_mac);
    Packet::from_bytes(frame)
}

/// 创建免费ARP报文（带以太网封装）
/// 特征：SPA == TPA
#[allow(dead_code)]
pub fn create_gratuitous_arp_packet(mac: MacAddr, ip: Ipv4Addr) -> Packet {
    let arp_packet = ArpPacket::new(
        ArpOperation::Request,
        mac,
        ip,
        MacAddr::zero(),
        ip, // TPA = SPA（免费ARP特征）
    );

    let frame = encapsulate_ethernet(&arp_packet, MacAddr::broadcast(), mac);
    Packet::from_bytes(frame)
}

// ========== 报文注入和验证函数 ==========

/// 注入报文到全局接口的 RxQ
#[allow(dead_code)]
pub fn inject_packet_to_interface(iface_name: &str, packet: Packet) -> HarnessResult<()> {
    let mut guard = GlobalStateManager::get_or_recover_interface_lock();
    let iface = guard.get_by_name_mut(iface_name)?;
    iface.rxq.enqueue(packet).map_err(|e| HarnessError::QueueError(format!("{:?}", e)))?;
    Ok(())
}

/// 验证 TxQ 中的报文数量
#[allow(dead_code)]
pub fn verify_txq_count(iface_name: &str, expected: usize) -> bool {
    let guard = GlobalStateManager::get_or_recover_interface_lock();
    guard.get_by_name(iface_name)
        .map(|iface| iface.txq.len() == expected)
        .unwrap_or(false)
}

/// 创建测试报文（满足以太网最小长度 14 字节）
#[allow(dead_code)]
pub fn create_test_packet(data: Vec<u8>) -> Packet {
    if data.len() < 14 {
        let mut padded = data;
        while padded.len() < 14 {
            padded.push(0);
        }
        Packet::from_bytes(padded)
    } else {
        Packet::from_bytes(data)
    }
}

// ========== 队列计数函数 ==========

/// 计算所有接口 RxQ 中的报文总数
#[allow(dead_code)]
pub fn count_all_rxq_packets<T>(manager: &T) -> usize
where
    T: QueueAccessor,
{
    let mut count = 0;
    for iface in manager.interfaces() {
        count += iface.rxq().len();
    }
    count
}

/// 计算所有接口 TxQ 中的报文总数
#[allow(dead_code)]
pub fn count_all_txq_packets<T>(manager: &T) -> usize
where
    T: QueueAccessor,
{
    let mut count = 0;
    for iface in manager.interfaces() {
        count += iface.txq().len();
    }
    count
}

/// 队列访问 trait，用于抽象不同类型的接口管理器
#[allow(dead_code)]
pub trait QueueAccessor {
    type Interface: QueueAccess;
    fn interfaces(&self) -> &[Self::Interface];
}

/// 单个接口的队列访问
#[allow(dead_code)]
pub trait QueueAccess {
    fn rxq(&self) -> &core_net::common::queue::RingQueue<Packet>;
    fn txq(&self) -> &core_net::common::queue::RingQueue<Packet>;
}

// 为现有类型实现 trait
impl QueueAccess for NetworkInterface {
    fn rxq(&self) -> &core_net::common::queue::RingQueue<Packet> {
        &self.rxq
    }

    fn txq(&self) -> &core_net::common::queue::RingQueue<Packet> {
        &self.txq
    }
}

impl QueueAccessor for core_net::interface::InterfaceManager {
    type Interface = NetworkInterface;

    fn interfaces(&self) -> &[Self::Interface] {
        self.interfaces()
    }
}
