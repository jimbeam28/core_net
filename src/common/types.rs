// src/common/types.rs
//
// 通用类型定义
// 定义网络协议中使用的通用类型：MacAddr、IpAddr、EtherType等

use std::fmt;
use std::net::{Ipv4Addr, Ipv6Addr};
use std::str::FromStr;

// ========== MAC地址 ==========

/// MAC地址（6字节）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MacAddr([u8; 6]);

impl MacAddr {
    /// 创建新的MAC地址
    pub const fn new(a: u8, b: u8, c: u8, d: u8, e: u8, f: u8) -> Self {
        MacAddr([a, b, c, d, e, f])
    }

    /// 从字节数组创建MAC地址
    pub const fn from_bytes(bytes: [u8; 6]) -> Self {
        MacAddr(bytes)
    }

    /// 返回字节数组
    pub const fn bytes(&self) -> [u8; 6] {
        self.0
    }

    /// 返回字节数组的引用
    pub fn as_bytes(&self) -> &[u8; 6] {
        &self.0
    }

    /// 广播地址：FF:FF:FF:FF:FF:FF
    pub const BROADCAST: MacAddr = MacAddr([0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF]);

    /// 零地址：00:00:00:00:00:00
    pub const ZERO: MacAddr = MacAddr([0x00, 0x00, 0x00, 0x00, 0x00, 0x00]);

    /// 判断是否为广播地址
    pub fn is_broadcast(&self) -> bool {
        self.0[0] == 0xFF && self.0[1] == 0xFF && self.0[2] == 0xFF
            && self.0[3] == 0xFF && self.0[4] == 0xFF && self.0[5] == 0xFF
    }

    /// 判断是否为多播地址
    pub fn is_multicast(&self) -> bool {
        // 第一字节最低位为1表示多播
        self.0[0] & 0x01 == 0x01
    }

    /// 判断是否为本地管理地址
    pub fn is_local(&self) -> bool {
        // 第二字节最低位为1表示本地管理
        self.0[0] & 0x02 == 0x02
    }
}

impl fmt::Display for MacAddr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{:02X}:{:02X}:{:02X}:{:02X}:{:02X}:{:02X}",
            self.0[0], self.0[1], self.0[2], self.0[3], self.0[4], self.0[5]
        )
    }
}

impl fmt::LowerHex for MacAddr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}",
            self.0[0], self.0[1], self.0[2], self.0[3], self.0[4], self.0[5]
        )
    }
}

impl From<[u8; 6]> for MacAddr {
    fn from(bytes: [u8; 6]) -> Self {
        MacAddr(bytes)
    }
}

impl FromStr for MacAddr {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        let parts: Vec<&str> = s.split(':').collect();
        if parts.len() != 6 {
            return Err(format!("Invalid MAC address format: {}", s));
        }

        let mut bytes = [0u8; 6];
        for (i, part) in parts.iter().enumerate() {
            bytes[i] = u8::from_str_radix(part, 16)
                .map_err(|_| format!("Invalid MAC address octet: {}", part))?;
        }

        Ok(MacAddr(bytes))
    }
}

// ========== IP地址 ==========

/// IP版本
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IpVersion {
    V4,
    V6,
}

impl fmt::Display for IpVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            IpVersion::V4 => write!(f, "IPv4"),
            IpVersion::V6 => write!(f, "IPv6"),
        }
    }
}

/// IP地址枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IpAddr {
    V4([u8; 4]),
    V6([u8; 16]),
}

impl IpAddr {
    /// 创建IPv4地址
    pub const fn v4(a: u8, b: u8, c: u8, d: u8) -> Self {
        IpAddr::V4([a, b, c, d])
    }

    /// 创建IPv6地址
    pub const fn v6(a: u16, b: u16, c: u16, d: u16, e: u16, f: u16, g: u16, h: u16) -> Self {
        IpAddr::V6([
            (a >> 8) as u8, a as u8,
            (b >> 8) as u8, b as u8,
            (c >> 8) as u8, c as u8,
            (d >> 8) as u8, d as u8,
            (e >> 8) as u8, e as u8,
            (f >> 8) as u8, f as u8,
            (g >> 8) as u8, g as u8,
            (h >> 8) as u8, h as u8,
        ])
    }

    /// 从IPv4字节数组创建
    pub const fn from_v4_bytes(bytes: [u8; 4]) -> Self {
        IpAddr::V4(bytes)
    }

    /// 从IPv6字节数组创建
    pub const fn from_v6_bytes(bytes: [u8; 16]) -> Self {
        IpAddr::V6(bytes)
    }

    /// 返回IP版本
    pub const fn version(&self) -> IpVersion {
        match self {
            IpAddr::V4(_) => IpVersion::V4,
            IpAddr::V6(_) => IpVersion::V6,
        }
    }

    /// 判断是否为IPv4
    pub const fn is_v4(&self) -> bool {
        matches!(self, IpAddr::V4(_))
    }

    /// 判断是否为IPv6
    pub const fn is_v6(&self) -> bool {
        matches!(self, IpAddr::V6(_))
    }

    /// 判断是否为回环地址
    pub fn is_loopback(&self) -> bool {
        match self {
            IpAddr::V4(bytes) => bytes[0] == 127,
            IpAddr::V6(bytes) => bytes[0..15].iter().all(|&b| b == 0) && bytes[15] == 1,
        }
    }

    /// 判断是否为多播地址
    pub fn is_multicast(&self) -> bool {
        match self {
            IpAddr::V4(bytes) => bytes[0] >= 224 && bytes[0] <= 239,
            IpAddr::V6(bytes) => bytes[0] == 0xFF,
        }
    }

    /// 获取字节数组（按版本返回）
    pub fn bytes(&self) -> Vec<u8> {
        match self {
            IpAddr::V4(b) => b.to_vec(),
            IpAddr::V6(b) => b.to_vec(),
        }
    }
}

impl fmt::Display for IpAddr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            IpAddr::V4(bytes) => {
                write!(f, "{}.{}.{}.{}", bytes[0], bytes[1], bytes[2], bytes[3])
            }
            IpAddr::V6(bytes) => {
                // 简化格式：每16位一组
                for i in 0..8 {
                    let val = u16::from_be_bytes([bytes[i * 2], bytes[i * 2 + 1]]);
                    if i > 0 {
                        write!(f, ":")?;
                    }
                    write!(f, "{:x}", val)?;
                }
                Ok(())
            }
        }
    }
}

impl From<[u8; 4]> for IpAddr {
    fn from(bytes: [u8; 4]) -> Self {
        IpAddr::V4(bytes)
    }
}

impl From<[u8; 16]> for IpAddr {
    fn from(bytes: [u8; 16]) -> Self {
        IpAddr::V6(bytes)
    }
}

impl From<Ipv4Addr> for IpAddr {
    fn from(addr: Ipv4Addr) -> Self {
        IpAddr::V4(addr.octets())
    }
}

impl From<Ipv6Addr> for IpAddr {
    fn from(addr: Ipv6Addr) -> Self {
        IpAddr::V6(addr.octets())
    }
}

// ========== 以太网类型 ==========

/// 以太网类型（EtherType）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u16)]
pub enum EtherType {
    /// IPv4协议 (0x0800)
    IPv4 = 0x0800,

    /// ARP协议 (0x0806)
    ARP = 0x0806,

    /// Wake-on-LAN (0x0842)
    WakeOnLan = 0x0842,

    /// TRILL (0x22F3)
    Trill = 0x22F3,

    /// DECnet (0x6000)
    Decnet = 0x6000,

    /// RARP (0x8035)
    RARP = 0x8035,

    /// AppleTalk (0x809B)
    AppleTalk = 0x809B,

    /// AppleTalk ARP (0x80F3)
    AppleTalkARP = 0x80F3,

    /// VLAN标签 (0x8100)
    Vlan = 0x8100,

    /// IPX (0x8137)
    IPX = 0x8137,

    /// IPX (0x8138)
    IPX2 = 0x8138,

    /// Q-in-Q (0x88A8)
    QinQ = 0x88A8,

    /// IPv6协议 (0x86DD)
    IPv6 = 0x86DD,

    /// EAPOL (0x888E)
    EAPOL = 0x888E,

    /// MPLS单播 (0x8847)
    MPLSUnicast = 0x8847,

    /// MPLS多播 (0x8848)
    MPLSMulticast = 0x8848,

    /// PPPOE发现阶段 (0x8863)
    PPPOEDiscovery = 0x8863,

    /// PPPOE会话阶段 (0x8864)
    PPPOESession = 0x8864,

    /// 802.1X (0x888E)
    Dot1X = 0x888E,

    /// 保留
    Reserved(u16),
}

impl EtherType {
    /// 从u16值创建EtherType
    pub const fn from_u16(value: u16) -> Self {
        match value {
            0x0800 => EtherType::IPv4,
            0x0806 => EtherType::ARP,
            0x0842 => EtherType::WakeOnLan,
            0x22F3 => EtherType::Trill,
            0x6000 => EtherType::Decnet,
            0x8035 => EtherType::RARP,
            0x809B => EtherType::AppleTalk,
            0x80F3 => EtherType::AppleTalkARP,
            0x8100 => EtherType::Vlan,
            0x8137 => EtherType::IPX,
            0x8138 => EtherType::IPX2,
            0x88A8 => EtherType::QinQ,
            0x86DD => EtherType::IPv6,
            0x888E => EtherType::EAPOL,
            0x8847 => EtherType::MPLSUnicast,
            0x8848 => EtherType::MPLSMulticast,
            0x8863 => EtherType::PPPOEDiscovery,
            0x8864 => EtherType::PPPOESession,
            _ => EtherType::Reserved(value),
        }
    }

    /// 转换为u16值
    pub const fn to_u16(self) -> u16 {
        match self {
            EtherType::IPv4 => 0x0800,
            EtherType::ARP => 0x0806,
            EtherType::WakeOnLan => 0x0842,
            EtherType::Trill => 0x22F3,
            EtherType::Decnet => 0x6000,
            EtherType::RARP => 0x8035,
            EtherType::AppleTalk => 0x809B,
            EtherType::AppleTalkARP => 0x80F3,
            EtherType::Vlan => 0x8100,
            EtherType::IPX => 0x8137,
            EtherType::IPX2 => 0x8138,
            EtherType::QinQ => 0x88A8,
            EtherType::IPv6 => 0x86DD,
            EtherType::EAPOL => 0x888E,
            EtherType::MPLSUnicast => 0x8847,
            EtherType::MPLSMulticast => 0x8848,
            EtherType::PPPOEDiscovery => 0x8863,
            EtherType::PPPOESession => 0x8864,
            EtherType::Reserved(v) => v,
        }
    }
}

impl fmt::Display for EtherType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EtherType::IPv4 => write!(f, "IPv4"),
            EtherType::ARP => write!(f, "ARP"),
            EtherType::WakeOnLan => write!(f, "Wake-on-LAN"),
            EtherType::Trill => write!(f, "TRILL"),
            EtherType::Decnet => write!(f, "DECnet"),
            EtherType::RARP => write!(f, "RARP"),
            EtherType::AppleTalk => write!(f, "AppleTalk"),
            EtherType::AppleTalkARP => write!(f, "AppleTalk ARP"),
            EtherType::Vlan => write!(f, "VLAN"),
            EtherType::IPX => write!(f, "IPX"),
            EtherType::IPX2 => write!(f, "IPX2"),
            EtherType::QinQ => write!(f, "Q-in-Q"),
            EtherType::IPv6 => write!(f, "IPv6"),
            EtherType::EAPOL => write!(f, "EAPOL"),
            EtherType::MPLSUnicast => write!(f, "MPLS Unicast"),
            EtherType::MPLSMulticast => write!(f, "MPLS Multicast"),
            EtherType::PPPOEDiscovery => write!(f, "PPPOE Discovery"),
            EtherType::PPPOESession => write!(f, "PPPOE Session"),
            EtherType::Reserved(v) => write!(f, "Reserved(0x{:04X})", v),
        }
    }
}

// ========== IP协议号 ==========

/// IP协议号（IP Protocol Number）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum IpProtocol {
    /// ICMP (1)
    ICMP = 1,

    /// IGMP (2)
    IGMP = 2,

    /// IPv4封装 (4)
    IPv4Encap = 4,

    /// TCP (6)
    TCP = 6,

    /// UDP (17)
    UDP = 17,

    /// IPv6封装 (41)
    IPv6Encap = 41,

    /// IPv6无_next头 (59)
    IPv6NoNextHeader = 59,

    /// ICMPv6 (58)
    ICMPv6 = 58,

    /// OSPF (89)
    OSPF = 89,

    /// SCTP (132)
    SCTP = 132,

    /// 保留
    Reserved(u8),
}

impl IpProtocol {
    /// 从u8值创建IpProtocol
    pub const fn from_u8(value: u8) -> Self {
        match value {
            1 => IpProtocol::ICMP,
            2 => IpProtocol::IGMP,
            4 => IpProtocol::IPv4Encap,
            6 => IpProtocol::TCP,
            17 => IpProtocol::UDP,
            41 => IpProtocol::IPv6Encap,
            58 => IpProtocol::ICMPv6,
            59 => IpProtocol::IPv6NoNextHeader,
            89 => IpProtocol::OSPF,
            132 => IpProtocol::SCTP,
            _ => IpProtocol::Reserved(value),
        }
    }

    /// 转换为u8值
    pub const fn to_u8(self) -> u8 {
        match self {
            IpProtocol::ICMP => 1,
            IpProtocol::IGMP => 2,
            IpProtocol::IPv4Encap => 4,
            IpProtocol::TCP => 6,
            IpProtocol::UDP => 17,
            IpProtocol::IPv6Encap => 41,
            IpProtocol::ICMPv6 => 58,
            IpProtocol::IPv6NoNextHeader => 59,
            IpProtocol::OSPF => 89,
            IpProtocol::SCTP => 132,
            IpProtocol::Reserved(v) => v,
        }
    }
}

impl fmt::Display for IpProtocol {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            IpProtocol::ICMP => write!(f, "ICMP"),
            IpProtocol::IGMP => write!(f, "IGMP"),
            IpProtocol::IPv4Encap => write!(f, "IPv4 Encapsulation"),
            IpProtocol::TCP => write!(f, "TCP"),
            IpProtocol::UDP => write!(f, "UDP"),
            IpProtocol::IPv6Encap => write!(f, "IPv6 Encapsulation"),
            IpProtocol::ICMPv6 => write!(f, "ICMPv6"),
            IpProtocol::IPv6NoNextHeader => write!(f, "IPv6 No Next Header"),
            IpProtocol::OSPF => write!(f, "OSPF"),
            IpProtocol::SCTP => write!(f, "SCTP"),
            IpProtocol::Reserved(v) => write!(f, "Reserved({})", v),
        }
    }
}

// ========== 协议层标识 ==========

/// 协议层标识（用于Packet解析状态跟踪）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Layer {
    /// 以太网层
    Ethernet,

    /// ARP协议
    Arp,

    /// IPv4协议
    IPv4,

    /// IPv6协议
    IPv6,

    /// ICMP协议
    ICMP,

    /// ICMPv6协议
    ICMPv6,

    /// TCP协议
    TCP,

    /// UDP协议
    UDP,
}

impl fmt::Display for Layer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Layer::Ethernet => write!(f, "Ethernet"),
            Layer::Arp => write!(f, "ARP"),
            Layer::IPv4 => write!(f, "IPv4"),
            Layer::IPv6 => write!(f, "IPv6"),
            Layer::ICMP => write!(f, "ICMP"),
            Layer::ICMPv6 => write!(f, "ICMPv6"),
            Layer::TCP => write!(f, "TCP"),
            Layer::UDP => write!(f, "UDP"),
        }
    }
}

// ========== 单元测试 ==========

#[cfg(test)]
mod tests {
    use super::*;

    // MacAddr测试
    #[test]
    fn test_mac_addr_display() {
        let mac = MacAddr::new(0x00, 0x11, 0x22, 0x33, 0x44, 0x55);
        assert_eq!(mac.to_string(), "00:11:22:33:44:55");

        assert_eq!(MacAddr::BROADCAST.to_string(), "FF:FF:FF:FF:FF:FF");
        assert_eq!(MacAddr::ZERO.to_string(), "00:00:00:00:00:00");
    }

    #[test]
    fn test_mac_addr_properties() {
        assert!(MacAddr::BROADCAST.is_broadcast());
        assert!(!MacAddr::ZERO.is_broadcast());

        let multicast = MacAddr::new(0x01, 0x00, 0x5E, 0x00, 0x00, 0x01);
        assert!(multicast.is_multicast());

        let local = MacAddr::new(0x02, 0x00, 0x00, 0x00, 0x00, 0x01);
        assert!(local.is_local());
    }

    #[test]
    fn test_mac_addr_from_str() {
        let mac: MacAddr = "00:11:22:33:44:55".parse().unwrap();
        assert_eq!(mac, MacAddr::new(0x00, 0x11, 0x22, 0x33, 0x44, 0x55));

        let err = "invalid".parse::<MacAddr>();
        assert!(err.is_err());
    }

    // IpAddr测试
    #[test]
    fn test_ip_addr_v4() {
        let ip = IpAddr::v4(192, 168, 1, 1);
        assert!(ip.is_v4());
        assert!(!ip.is_v6());
        assert_eq!(ip.to_string(), "192.168.1.1");
        assert_eq!(ip.version(), IpVersion::V4);
    }

    #[test]
    fn test_ip_addr_v6() {
        let ip = IpAddr::v6(0, 0, 0, 0, 0, 0, 0, 1);
        assert!(ip.is_v6());
        assert!(!ip.is_v4());
        assert!(ip.to_string().contains("1"));
        assert_eq!(ip.version(), IpVersion::V6);
    }

    #[test]
    fn test_ip_addr_loopback() {
        let v4_loopback = IpAddr::v4(127, 0, 0, 1);
        assert!(v4_loopback.is_loopback());

        let v6_loopback = IpAddr::v6(0, 0, 0, 0, 0, 0, 0, 1);
        assert!(v6_loopback.is_loopback());
    }

    #[test]
    fn test_ip_addr_multicast() {
        let v4_multi = IpAddr::v4(224, 0, 0, 1);
        assert!(v4_multi.is_multicast());

        let v6_multi = IpAddr::from_v6_bytes([0xFF, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1]);
        assert!(v6_multi.is_multicast());
    }

    // EtherType测试
    #[test]
    fn test_ether_type() {
        assert_eq!(EtherType::from_u16(0x0800), EtherType::IPv4);
        assert_eq!(EtherType::from_u16(0x0806), EtherType::ARP);
        assert_eq!(EtherType::from_u16(0x86DD), EtherType::IPv6);

        assert_eq!(EtherType::IPv4.to_u16(), 0x0800);
        assert_eq!(EtherType::ARP.to_u16(), 0x0806);

        let unknown = EtherType::from_u16(0xFFFF);
        assert!(matches!(unknown, EtherType::Reserved(0xFFFF)));
    }

    // IpProtocol测试
    #[test]
    fn test_ip_protocol() {
        assert_eq!(IpProtocol::from_u8(1), IpProtocol::ICMP);
        assert_eq!(IpProtocol::from_u8(6), IpProtocol::TCP);
        assert_eq!(IpProtocol::from_u8(17), IpProtocol::UDP);

        assert_eq!(IpProtocol::TCP.to_u8(), 6);
        assert_eq!(IpProtocol::UDP.to_u8(), 17);
    }

    // Layer测试
    #[test]
    fn test_layer_display() {
        assert_eq!(Layer::Ethernet.to_string(), "Ethernet");
        assert_eq!(Layer::TCP.to_string(), "TCP");
    }
}
