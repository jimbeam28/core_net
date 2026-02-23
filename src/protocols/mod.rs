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

// ICMPv6 协议
pub mod icmpv6;

// UDP 协议
pub mod udp;

// TCP 协议
pub mod tcp;

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

// ICMPv6 模块导出
pub use icmpv6::{
    Icmpv6Type,
    Icmpv6Packet,
    Icmpv6Echo,
    Icmpv6Error,
    Icmpv6Result,
    Icmpv6Config,
    ICMPV6_CONFIG_DEFAULT,
    NeighborCache,
    NeighborCacheEntry,
    NeighborCacheState,
    RouterList,
    DefaultRouterEntry,
    PrefixList,
    PrefixEntry,
    PmtuCache,
    EchoManager,
    PendingEcho,
    process_icmpv6_packet,
    create_icmpv6_echo_request,
    create_icmpv6_echo_reply,
    IPPROTO_ICMPV6,
};

// UDP 模块导出
pub use udp::{
    UdpHeader,
    UdpDatagram,
    UdpConfig,
    UdpProcessResult,
    process_udp_packet,
    encapsulate_udp_datagram,
    create_port_unreachable,
    UDP_HEADER_SIZE,
    UDP_MIN_LENGTH,
    UDP_CONFIG_DEFAULT,
};

// TCP 模块导出
pub use tcp::{
    TcpHeader,
    TcpSegment,
    TcpConfig,
    TcpProcessResult,
    TcpError,
    TcpConnectionId,
    TcpState,
    TcpConnectionManager,
    Tcb,
    IP_PROTO_TCP,
    TCP_MIN_HEADER_LEN,
    TCP_MAX_HEADER_LEN,
    TCP_CONFIG_DEFAULT,
    process_tcp_packet,
    encapsulate_tcp_segment,
    encapsulate_tcp_header,
    create_syn,
    create_ack,
    create_fin,
    create_rst,
};
