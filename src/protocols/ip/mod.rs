// src/protocols/ip/mod.rs
//
// IPv4 协议模块
// 实现了 IP 数据报解析、封装、校验和验证、分片和重组

pub mod checksum;
mod header;
mod protocol;
mod error;
mod config;
mod packet;
pub mod fragment;

pub use checksum::{
    calculate_checksum,
    verify_checksum,
    add_ipv4_pseudo_header,
    add_ipv6_pseudo_header,
    fold_carry,
    calculate_icmpv6_checksum,
    verify_icmpv6_checksum,
};
pub use header::{
    Ipv4Header,
    IP_VERSION,
    IP_MIN_HEADER_LEN,
    IP_PROTO_ICMP,
    IP_PROTO_TCP,
    IP_PROTO_UDP,
    IP_PROTO_OSPF,
    IP_PROTO_ESP,
    IP_PROTO_AH,
    DEFAULT_TTL,
};
pub use protocol::Ipv4Protocol;
pub use error::IpError;
pub use config::{Ipv4Config, IPV4_CONFIG_DEFAULT};
pub use packet::{IpProcessResult, process_ip_packet, encapsulate_ip_datagram, fragment_datagram};
pub use fragment::{
    FragmentInfo,
    ReassemblyKey,
    ReassemblyEntry,
    ReassemblyTable,
    FragmentOverlapPolicy,
    ReassemblyStats,
    DEFAULT_REASSEMBLY_TIMEOUT_SECS,
    DEFAULT_MAX_REASSEMBLY_ENTRIES,
    DEFAULT_MAX_FRAGMENTS_PER_DATAGRAM,
};
