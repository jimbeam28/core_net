// src/protocols/mod.rs
//
// 协议模块声明

// 以太网协议
pub mod ethernet;

// VLAN协议
pub mod vlan;

// ARP协议
pub mod arp;

// IP 协议（最小化实现，支持 ICMP）
pub mod ip;

// IPv6 协议
pub mod ipv6;

// ICMP 协议
pub mod icmp;

// 从 common 模块重新导出类型
pub use crate::common::{
    Packet,
    MacAddr,
    Ipv4Addr,
    Ipv6Addr,
};

pub use ethernet::{
    EthernetHeader,
    ETH_P_IP,
    ETH_P_ARP,
    ETH_P_IPV6,
    ETH_P_8021Q,
    ETH_P_8021AD,
};

pub use vlan::{
    VlanTag,
    VlanFrame,
    VlanError,
    has_vlan_tag,
    is_vlan_tpid,
};

// IP 模块导出
pub use ip::{
    Ipv4Header,
    IP_PROTO_ICMP,
    IP_PROTO_TCP,
    IP_PROTO_UDP,
};

// IPv6 模块导出
pub use ipv6::{
    Ipv6Header,
    Ipv6Error,
    Ipv6ProcessResult,
    IpProtocol,
    IPV6_VERSION,
    IPV6_HEADER_LEN,
    IPV6_MIN_MTU,
    DEFAULT_HOP_LIMIT,
    process_ipv6_packet,
    encapsulate_ipv6_packet,
};

// ICMP 模块导出
pub use icmp::{
    IcmpPacket,
    IcmpEcho,
    IcmpProcessResult,
    process_icmp_packet,
    create_echo_request,
    create_echo_reply,
};
