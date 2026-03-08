// src/protocols/udp/mod.rs
//
// UDP 协议模块
// 实现 RFC 768 User Datagram Protocol

mod header;
mod packet;
mod process;
mod port;
mod socket;

pub use header::UdpHeader;
pub use packet::UdpDatagram;
pub use process::{
    UdpProcessResult,
    process_udp_packet,
    encapsulate_udp_datagram,
    create_port_unreachable,
};
pub use port::{
    PortEntry,
    UdpPortManager,
    UdpReceiveCallback,
    WELL_KNOWN_PORT_MIN,
    WELL_KNOWN_PORT_MAX,
    REGISTERED_PORT_MIN,
    REGISTERED_PORT_MAX,
    EPHEMERAL_PORT_MIN,
    EPHEMERAL_PORT_MAX,
    PORT_MIN,
    PORT_MAX,
};
pub use socket::UdpSocket;

// ==================== config.rs 内容 ====================

/// UDP 协议配置
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UdpConfig {
    /// 是否强制验证校验和（IPv4 中可选，默认启用）
    pub enforce_checksum: bool,
    /// 是否在端口不可达时发送 ICMP 消息
    pub send_icmp_unreachable: bool,
    /// 最大 UDP 数据报大小（受 MTU 限制）
    pub max_datagram_size: u16,
}

impl Default for UdpConfig {
    fn default() -> Self {
        Self {
            enforce_checksum: true,
            send_icmp_unreachable: true,
            max_datagram_size: 1472, // 1500 (以太网 MTU) - 20 (IP 头部) - 8 (UDP 头部)
        }
    }
}

impl UdpConfig {
    /// 创建新的 UDP 配置
    pub fn new() -> Self {
        Self::default()
    }

    /// 设置是否强制验证校验和
    pub fn with_enforce_checksum(mut self, enforce: bool) -> Self {
        self.enforce_checksum = enforce;
        self
    }

    /// 设置是否发送 ICMP 不可达消息
    pub fn with_send_icmp_unreachable(mut self, send: bool) -> Self {
        self.send_icmp_unreachable = send;
        self
    }

    /// 设置最大数据报大小
    pub fn with_max_datagram_size(mut self, size: u16) -> Self {
        self.max_datagram_size = size;
        self
    }
}

// UDP 协议常量

/// UDP 协议号（在 IP 协议字段中的值）
pub const IP_PROTO_UDP: u8 = 17;

/// UDP 头部大小
pub const UDP_HEADER_SIZE: usize = 8;

/// UDP 最小数据报长度
pub const UDP_MIN_LENGTH: u16 = 8;

/// 知名端口号
pub mod well_known_ports {
    /// DNS
    pub const DNS: u16 = 53;
    /// TFTP
    pub const TFTP: u16 = 69;
    /// NTP
    pub const NTP: u16 = 123;
    /// SNMP
    pub const SNMP: u16 = 161;
}

/// 默认 UDP 配置
pub const UDP_CONFIG_DEFAULT: UdpConfig = UdpConfig {
    enforce_checksum: true,
    send_icmp_unreachable: true,
    max_datagram_size: 1472,
};
