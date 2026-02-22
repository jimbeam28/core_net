// 测试公共模块 - 提供各测试文件共用的辅助函数和配置

use core_net::testframework::{HarnessError, HarnessResult, GlobalStateManager};
use core_net::interface::{InterfaceConfig, InterfaceState, MacAddr, Ipv4Addr, NetworkInterface};
use core_net::protocols::Ipv6Addr;
use core_net::protocols::arp::{ArpPacket, ArpOperation, encapsulate_ethernet};
use core_net::protocols::{IP_PROTO_ICMP, IP_PROTO_UDP};
use core_net::protocols::ip::Ipv4Header;
use core_net::protocols::ipv6::{IpProtocol, encapsulate_ipv6_packet};
use core_net::protocols::udp::encapsulate_udp_datagram;
use core_net::protocols::ETH_P_IPV6;
use core_net::common::Packet;
use core_net::context::SystemContext;

// 测试配置

/// 创建测试用 eth0 配置
pub fn create_test_eth0_config() -> InterfaceConfig {
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

/// 创建测试用 lo 配置
pub fn create_test_lo_config() -> InterfaceConfig {
    InterfaceConfig {
        name: "lo".to_string(),
        mac_addr: MacAddr::zero(),
        ip_addr: Ipv4Addr::new(127, 0, 0, 1),
        ipv6_addr: Ipv6Addr::LOOPBACK,
        netmask: Ipv4Addr::new(255, 0, 0, 0),
        gateway: None,
        mtu: Some(65535),
        state: Some(InterfaceState::Up),
    }
}

// 报文创建函数

/// 创建 IP 头部
pub fn create_ip_header(src_ip: Ipv4Addr, dst_ip: Ipv4Addr, payload_len: usize) -> Vec<u8> {
    let ip_header = Ipv4Header::new(src_ip, dst_ip, IP_PROTO_ICMP, payload_len);
    ip_header.to_bytes()
}

/// 创建 ICMP Echo Request 报文（带 IP 和以太网封装）
pub fn create_echo_request_packet(
    src_mac: MacAddr,
    src_ip: Ipv4Addr,
    dst_ip: Ipv4Addr,
    identifier: u16,
    sequence: u16,
) -> Packet {
    use core_net::protocols::icmp::create_echo_request;
    use core_net::protocols::ETH_P_IP;

    let icmp_data = vec![0x42; 32];
    let icmp_packet = create_echo_request(identifier, sequence, icmp_data);

    let mut ip_data = create_ip_header(src_ip, dst_ip, icmp_packet.len());
    ip_data.extend_from_slice(&icmp_packet);

    let mut frame = Vec::new();
    frame.extend_from_slice(&[0xff, 0xff, 0xff, 0xff, 0xff, 0xff]);
    frame.extend_from_slice(&src_mac.bytes);
    frame.extend_from_slice(&ETH_P_IP.to_be_bytes());
    frame.extend_from_slice(&ip_data);

    Packet::from_bytes(frame)
}

/// 创建 ICMP Echo Reply 报文（带 IP 和以太网封装）
pub fn create_echo_reply_packet(
    dst_mac: MacAddr,
    src_mac: MacAddr,
    src_ip: Ipv4Addr,
    dst_ip: Ipv4Addr,
    identifier: u16,
    sequence: u16,
) -> Packet {
    use core_net::protocols::icmp::IcmpEcho;
    use core_net::protocols::ETH_P_IP;

    let icmp_echo = IcmpEcho::new_reply(identifier, sequence, vec![0x42; 32]);
    let icmp_packet = icmp_echo.to_bytes();

    let mut ip_data = create_ip_header(src_ip, dst_ip, icmp_packet.len());
    ip_data.extend_from_slice(&icmp_packet);

    let mut frame = Vec::new();
    frame.extend_from_slice(&dst_mac.bytes);
    frame.extend_from_slice(&src_mac.bytes);
    frame.extend_from_slice(&ETH_P_IP.to_be_bytes());
    frame.extend_from_slice(&ip_data);

    Packet::from_bytes(frame)
}

/// 创建测试用系统上下文（每个测试独立）
pub fn create_test_context() -> SystemContext {
    GlobalStateManager::create_context()
}

/// 创建ARP请求报文（带以太网封装）
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

/// 创建免费ARP报文（带以太网封装），特征：SPA == TPA
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

// 报文注入和验证函数

/// 注入报文到指定接口的 RxQ
pub fn inject_packet_to_context(context: &SystemContext, iface_name: &str, packet: Packet) -> HarnessResult<()> {
    let mut interfaces = context.interfaces.lock()
        .map_err(|e| HarnessError::QueueError(format!("锁定接口管理器失败: {}", e)))?;
    let iface = interfaces.get_by_name_mut(iface_name)?;
    iface.rxq.enqueue(packet).map_err(|e| HarnessError::QueueError(format!("{:?}", e)))?;
    Ok(())
}

/// 验证接口 TxQ 中的报文数量
pub fn verify_context_txq_count(context: &SystemContext, iface_name: &str, expected: usize) -> bool {
    let guard = context.interfaces.lock();
    if guard.is_err() {
        return false;
    }
    guard.unwrap().get_by_name(iface_name)
        .map(|iface| iface.txq.len() == expected)
        .unwrap_or(false)
}

/// 清空指定接口的 TxQ
pub fn clear_context_txq(context: &SystemContext, iface_name: &str) -> HarnessResult<()> {
    let mut interfaces = context.interfaces.lock()
        .map_err(|e| HarnessError::InterfaceError(format!("锁定接口管理器失败: {}", e)))?;
    let iface = interfaces.get_by_name_mut(iface_name)?;
    iface.txq.clear();
    Ok(())
}

/// 创建测试报文（满足以太网最小长度 14 字节）
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

/// 计算所有接口 RxQ 中的报文总数
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
pub trait QueueAccessor {
    type Interface: QueueAccess;
    fn interfaces(&self) -> &[Self::Interface];
}

/// 单个接口的队列访问
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

// ========== IPv6 辅助函数 ==========

/// 创建 IPv6 Echo Request 报文（带以太网封装）
pub fn create_ipv6_echo_request_packet(
    src_mac: MacAddr,
    src_ipv6: Ipv6Addr,
    dst_ipv6: Ipv6Addr,
    identifier: u16,
    sequence: u16,
) -> Packet {
    // ICMPv6 Echo Request 报文
    let icmp_data = vec![0x42; 32]; // 数据负载

    // ICMPv6 Echo Request: Type=128, Code=0
    let mut icmp_packet = Vec::with_capacity(4 + icmp_data.len());
    icmp_packet.push(128); // Type: Echo Request
    icmp_packet.push(0);   // Code: 0
    icmp_packet.extend_from_slice(&identifier.to_be_bytes()); // Identifier
    icmp_packet.extend_from_slice(&sequence.to_be_bytes());   // Sequence
    icmp_packet.extend_from_slice(&icmp_data);

    // 封装 IPv6 头部
    let ipv6_packet = encapsulate_ipv6_packet(
        src_ipv6,
        dst_ipv6,
        IpProtocol::IcmpV6,
        &icmp_packet,
        64, // Hop Limit
    );

    // 封装以太网头部
    let mut frame = Vec::new();
    frame.extend_from_slice(&[0xff, 0xff, 0xff, 0xff, 0xff, 0xff]); // 广播目的 MAC
    frame.extend_from_slice(&src_mac.bytes);
    frame.extend_from_slice(&ETH_P_IPV6.to_be_bytes());
    frame.extend_from_slice(&ipv6_packet);

    Packet::from_bytes(frame)
}

/// 创建 IPv6 数据包（带以太网封装）
pub fn create_ipv6_packet(
    dst_mac: MacAddr,
    src_mac: MacAddr,
    src_ipv6: Ipv6Addr,
    dst_ipv6: Ipv6Addr,
    next_header: IpProtocol,
    payload: Vec<u8>,
) -> Packet {
    let ipv6_packet = encapsulate_ipv6_packet(
        src_ipv6,
        dst_ipv6,
        next_header,
        &payload,
        64, // Hop Limit
    );

    let mut frame = Vec::new();
    frame.extend_from_slice(&dst_mac.bytes);
    frame.extend_from_slice(&src_mac.bytes);
    frame.extend_from_slice(&ETH_P_IPV6.to_be_bytes());
    frame.extend_from_slice(&ipv6_packet);

    Packet::from_bytes(frame)
}

// ========== UDP 辅助函数 ==========

/// 创建 UDP 数据报（带 IP 和以太网封装）
pub fn create_udp_packet(
    src_mac: MacAddr,
    src_ip: Ipv4Addr,
    dst_ip: Ipv4Addr,
    src_port: u16,
    dst_port: u16,
    payload: &[u8],
    calculate_checksum: bool,
) -> Packet {
    use core_net::protocols::ETH_P_IP;

    // 封装 UDP 数据报
    let udp_data = encapsulate_udp_datagram(
        src_port,
        dst_port,
        src_ip,
        dst_ip,
        payload,
        calculate_checksum,
    );

    // 创建 IP 头部
    let mut ip_data = create_ip_header_udp(src_ip, dst_ip, udp_data.len());
    ip_data.extend_from_slice(&udp_data);

    // 封装以太网帧
    let mut frame = Vec::new();
    frame.extend_from_slice(&[0xff, 0xff, 0xff, 0xff, 0xff, 0xff]);
    frame.extend_from_slice(&src_mac.bytes);
    frame.extend_from_slice(&ETH_P_IP.to_be_bytes());
    frame.extend_from_slice(&ip_data);

    Packet::from_bytes(frame)
}

/// 创建 IP 头部（用于 UDP）
pub fn create_ip_header_udp(src_ip: Ipv4Addr, dst_ip: Ipv4Addr, payload_len: usize) -> Vec<u8> {
    let ip_header = Ipv4Header::new(src_ip, dst_ip, IP_PROTO_UDP, payload_len);
    ip_header.to_bytes()
}
