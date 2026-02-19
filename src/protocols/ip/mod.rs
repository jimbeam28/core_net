// src/protocols/ip/mod.rs
//
// 最小化 IPv4 协议模块
// 仅实现 ICMP 必需的 IP 功能：头部解析、封装、校验和

mod checksum;
mod header;

pub use checksum::{calculate_checksum, verify_checksum};
pub use header::{
    Ipv4Header,
    IP_VERSION,
    IP_MIN_HEADER_LEN,
    IP_PROTO_ICMP,
    IP_PROTO_TCP,
    IP_PROTO_UDP,
    DEFAULT_TTL,
};
