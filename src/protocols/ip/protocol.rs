// src/protocols/ip/protocol.rs
//
// IPv4 协议号定义

/// IPv4 协议号（上层协议类型）
///
/// 定义了 IP 头部中 Protocol 字段的所有已知值。
/// 当前版本仅支持 ICMP，其他协议返回 UnsupportedProtocol 错误。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Ipv4Protocol {
    /// ICMP (Internet Control Message Protocol)
    Icmp = 1,

    /// TCP (Transmission Control Protocol) - 未实现
    Tcp = 6,

    /// UDP (User Datagram Protocol) - 未实现
    Udp = 17,

    /// IPv6 隧道 - 未实现
    Ipv6 = 41,

    /// OSPF (Open Shortest Path First) - 未实现
    Ospf = 89,

    /// SCTP (Stream Control Transmission Protocol) - 未实现
    Sctp = 132,

    /// 未知协议
    Unknown(u8),
}

impl Ipv4Protocol {
    /// 判断协议是否被支持（当前仅支持 ICMP）
    pub const fn is_supported(&self) -> bool {
        matches!(self, Ipv4Protocol::Icmp)
    }

    /// 获取协议名称（用于调试）
    pub const fn name(&self) -> &'static str {
        match self {
            Ipv4Protocol::Icmp => "ICMP",
            Ipv4Protocol::Tcp => "TCP",
            Ipv4Protocol::Udp => "UDP",
            Ipv4Protocol::Ipv6 => "IPv6",
            Ipv4Protocol::Ospf => "OSPF",
            Ipv4Protocol::Sctp => "SCTP",
            Ipv4Protocol::Unknown(_) => "Unknown",
        }
    }
}

impl From<u8> for Ipv4Protocol {
    fn from(value: u8) -> Self {
        match value {
            1 => Ipv4Protocol::Icmp,
            6 => Ipv4Protocol::Tcp,
            17 => Ipv4Protocol::Udp,
            41 => Ipv4Protocol::Ipv6,
            89 => Ipv4Protocol::Ospf,
            132 => Ipv4Protocol::Sctp,
            v => Ipv4Protocol::Unknown(v),
        }
    }
}

impl From<Ipv4Protocol> for u8 {
    fn from(protocol: Ipv4Protocol) -> Self {
        match protocol {
            Ipv4Protocol::Icmp => 1,
            Ipv4Protocol::Tcp => 6,
            Ipv4Protocol::Udp => 17,
            Ipv4Protocol::Ipv6 => 41,
            Ipv4Protocol::Ospf => 89,
            Ipv4Protocol::Sctp => 132,
            Ipv4Protocol::Unknown(v) => v,
        }
    }
}

impl std::fmt::Display for Ipv4Protocol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Ipv4Protocol::Unknown(v) => write!(f, "Unknown({})", v),
            _ => write!(f, "{}", self.name()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_protocol_from_u8() {
        assert_eq!(Ipv4Protocol::from(1), Ipv4Protocol::Icmp);
        assert_eq!(Ipv4Protocol::from(6), Ipv4Protocol::Tcp);
        assert_eq!(Ipv4Protocol::from(17), Ipv4Protocol::Udp);
        assert_eq!(Ipv4Protocol::from(255), Ipv4Protocol::Unknown(255));
    }

    #[test]
    fn test_protocol_to_u8() {
        assert_eq!(u8::from(Ipv4Protocol::Icmp), 1);
        assert_eq!(u8::from(Ipv4Protocol::Tcp), 6);
        assert_eq!(u8::from(Ipv4Protocol::Udp), 17);
        assert_eq!(u8::from(Ipv4Protocol::Unknown(99)), 99);
    }

    #[test]
    fn test_is_supported() {
        assert!(Ipv4Protocol::Icmp.is_supported());
        assert!(!Ipv4Protocol::Tcp.is_supported());
        assert!(!Ipv4Protocol::Udp.is_supported());
        assert!(!Ipv4Protocol::Unknown(99).is_supported());
    }

    #[test]
    fn test_protocol_display() {
        assert_eq!(format!("{}", Ipv4Protocol::Icmp), "ICMP");
        assert_eq!(format!("{}", Ipv4Protocol::Tcp), "TCP");
        assert_eq!(format!("{}", Ipv4Protocol::Unknown(99)), "Unknown(99)");
    }
}
