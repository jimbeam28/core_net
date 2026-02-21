// src/protocols/ip/mod.rs
//
// IPv4 协议模块
// 实现了 IP 数据报解析、封装、校验和验证
// 当前版本不支持分片和重组，仅支持 ICMP 协议

pub mod checksum;
mod header;
mod protocol;
mod error;
mod config;
mod packet;

pub use checksum::{calculate_checksum, verify_checksum, add_ipv4_pseudo_header, fold_carry};
pub use header::{
    Ipv4Header,
    IP_VERSION,
    IP_MIN_HEADER_LEN,
    IP_PROTO_ICMP,
    IP_PROTO_TCP,
    IP_PROTO_UDP,
    DEFAULT_TTL,
};
pub use protocol::Ipv4Protocol;
pub use error::IpError;
pub use config::{Ipv4Config, IPV4_CONFIG_DEFAULT};
pub use packet::{IpProcessResult, process_ip_packet, encapsulate_ip_datagram};
