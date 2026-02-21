// src/protocols/ipv6/protocol.rs
//
// IPv6 协议号定义（Next Header 字段值）

/// IPv6 协议号（Next Header 字段值）
///
/// 定义了 IPv6 头部中 Next Header 字段的所有已知值。
/// 当前版本仅支持 ICMPv6，其他协议返回 UnsupportedProtocol 错误。
/// 扩展头部类型当前版本不支持。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum IpProtocol {
    /// 逐跳选项 (Hop-by-Hop Options) - 未来支持
    HopByHopOptions = 0,

    /// ICMP
    Icmp = 1,

    /// IGMP
    Igmp = 2,

    /// TCP
    Tcp = 6,

    /// UDP
    Udp = 17,

    /// IPv6 封装
    Ipv6Encap = 41,

    /// IPv6 路由 (Routing) - 未来支持
    Ipv6Route = 43,

    /// IPv6 分片 (Fragment) - 未来支持
    Ipv6Fragment = 44,

    /// IPSec ESP - 未来支持
    Esp = 50,

    /// IPSec AH - 未来支持
    Ah = 51,

    /// ICMPv6
    IcmpV6 = 58,

    /// 无下一头部
    NoNextHeader = 59,

    /// IPv6 目的选项 (Destination Options) - 未来支持
    Ipv6DestOptions = 60,

    /// 内部主机协议
    Ihip = 139,

    /// 未知协议
    Unknown(u8),
}

impl IpProtocol {
    /// 判断是否为扩展头类型（当前版本不支持）
    pub fn is_extension_header(&self) -> bool {
        matches!(
            self,
            Self::HopByHopOptions
                | Self::Ipv6Route
                | Self::Ipv6Fragment
                | Self::Esp
                | Self::Ah
                | Self::Ipv6DestOptions
        )
    }

    /// 判断是否为上层协议类型
    pub fn is_upper_layer(&self) -> bool {
        matches!(self, Self::Tcp | Self::Udp | Self::IcmpV6 | Self::Icmp)
    }

    /// 判断协议是否被支持（当前仅支持 ICMPv6）
    pub fn is_supported(&self) -> bool {
        matches!(self, Self::IcmpV6)
    }

    /// 获取协议名称（用于调试）
    pub fn name(&self) -> &'static str {
        match self {
            Self::HopByHopOptions => "HopByHopOptions",
            Self::Icmp => "ICMP",
            Self::Igmp => "IGMP",
            Self::Tcp => "TCP",
            Self::Udp => "UDP",
            Self::Ipv6Encap => "IPv6Encap",
            Self::Ipv6Route => "IPv6Route",
            Self::Ipv6Fragment => "IPv6Fragment",
            Self::Esp => "ESP",
            Self::Ah => "AH",
            Self::IcmpV6 => "ICMPv6",
            Self::NoNextHeader => "NoNextHeader",
            Self::Ipv6DestOptions => "IPv6DestOptions",
            Self::Ihip => "IHIP",
            Self::Unknown(_) => "Unknown",
        }
    }
}

impl From<u8> for IpProtocol {
    fn from(value: u8) -> Self {
        match value {
            0 => IpProtocol::HopByHopOptions,
            1 => IpProtocol::Icmp,
            2 => IpProtocol::Igmp,
            6 => IpProtocol::Tcp,
            17 => IpProtocol::Udp,
            41 => IpProtocol::Ipv6Encap,
            43 => IpProtocol::Ipv6Route,
            44 => IpProtocol::Ipv6Fragment,
            50 => IpProtocol::Esp,
            51 => IpProtocol::Ah,
            58 => IpProtocol::IcmpV6,
            59 => IpProtocol::NoNextHeader,
            60 => IpProtocol::Ipv6DestOptions,
            139 => IpProtocol::Ihip,
            v => IpProtocol::Unknown(v),
        }
    }
}

impl From<IpProtocol> for u8 {
    fn from(protocol: IpProtocol) -> Self {
        match protocol {
            IpProtocol::HopByHopOptions => 0,
            IpProtocol::Icmp => 1,
            IpProtocol::Igmp => 2,
            IpProtocol::Tcp => 6,
            IpProtocol::Udp => 17,
            IpProtocol::Ipv6Encap => 41,
            IpProtocol::Ipv6Route => 43,
            IpProtocol::Ipv6Fragment => 44,
            IpProtocol::Esp => 50,
            IpProtocol::Ah => 51,
            IpProtocol::IcmpV6 => 58,
            IpProtocol::NoNextHeader => 59,
            IpProtocol::Ipv6DestOptions => 60,
            IpProtocol::Ihip => 139,
            IpProtocol::Unknown(v) => v,
        }
    }
}

impl std::fmt::Display for IpProtocol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IpProtocol::Unknown(v) => write!(f, "Unknown({})", v),
            _ => write!(f, "{}", self.name()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_protocol_from_u8() {
        assert_eq!(IpProtocol::from(58), IpProtocol::IcmpV6);
        assert_eq!(IpProtocol::from(6), IpProtocol::Tcp);
        assert_eq!(IpProtocol::from(17), IpProtocol::Udp);
        assert_eq!(IpProtocol::from(255), IpProtocol::Unknown(255));
    }

    #[test]
    fn test_protocol_to_u8() {
        assert_eq!(u8::from(IpProtocol::IcmpV6), 58);
        assert_eq!(u8::from(IpProtocol::Tcp), 6);
        assert_eq!(u8::from(IpProtocol::Udp), 17);
        assert_eq!(u8::from(IpProtocol::Unknown(99)), 99);
    }

    #[test]
    fn test_is_extension_header() {
        assert!(IpProtocol::HopByHopOptions.is_extension_header());
        assert!(IpProtocol::Ipv6Route.is_extension_header());
        assert!(IpProtocol::Ipv6Fragment.is_extension_header());
        assert!(IpProtocol::Esp.is_extension_header());
        assert!(IpProtocol::Ah.is_extension_header());
        assert!(IpProtocol::Ipv6DestOptions.is_extension_header());

        assert!(!IpProtocol::IcmpV6.is_extension_header());
        assert!(!IpProtocol::Tcp.is_extension_header());
        assert!(!IpProtocol::Udp.is_extension_header());
    }

    #[test]
    fn test_is_upper_layer() {
        assert!(IpProtocol::Tcp.is_upper_layer());
        assert!(IpProtocol::Udp.is_upper_layer());
        assert!(IpProtocol::IcmpV6.is_upper_layer());
        assert!(IpProtocol::Icmp.is_upper_layer());

        assert!(!IpProtocol::HopByHopOptions.is_upper_layer());
        assert!(!IpProtocol::Ipv6Route.is_upper_layer());
    }

    #[test]
    fn test_is_supported() {
        assert!(IpProtocol::IcmpV6.is_supported());
        assert!(!IpProtocol::Tcp.is_supported());
        assert!(!IpProtocol::Udp.is_supported());
    }
}
