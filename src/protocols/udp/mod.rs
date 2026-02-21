// src/protocols/udp/mod.rs
//
// UDP 协议模块
// 实现 RFC 768 User Datagram Protocol

mod header;
mod packet;
mod process;
mod config;

pub use header::UdpHeader;
pub use packet::UdpDatagram;
pub use config::UdpConfig;
pub use process::{
    UdpProcessResult,
    process_udp_packet,
    encapsulate_udp_datagram,
    create_port_unreachable,
};

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
