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

// BGP 协议
pub mod bgp;

// OSPF 协议（共享核心模块）
pub mod ospf;

// OSPFv2 协议（IPv4）
pub mod ospf2;

// OSPFv3 协议（IPv6）
pub mod ospf3;

// IPsec 协议（AH 和 ESP）
pub mod ipsec;

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

// ICMPv6 模块导出（精简版）
pub use icmpv6::{
    Icmpv6Type,
    Icmpv6Packet,
    Icmpv6Echo,
    Icmpv6Error,
    Icmpv6Result,
    Icmpv6Config,
    ICMPV6_CONFIG_DEFAULT,
    NeighborCache,
    process_icmpv6_packet,
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
    create_syn,
    create_ack,
    create_fin,
    create_rst,
};

// BGP 模块导出（精简版）
pub use bgp::{
    BgpConfig, BgpPeerConfig, BgpPolicy, BgpPeerType,
    BgpError, BgpState,
    BgpHeader, BgpOpen, BgpUpdate, BgpNotification, BgpKeepalive, BgpRouteRefresh,
    BgpMessage, BgpCapability, IpPrefix,
    BgpPeer, BgpPeerManager,
    parse_bgp_message, encapsulate_bgp_message,
    BGP_PORT, BGP_VERSION, DEFAULT_HOLD_TIME, DEFAULT_CONNECT_RETRY_TIME,
    BGP_MSG_OPEN, BGP_MSG_UPDATE, BGP_MSG_NOTIFICATION, BGP_MSG_KEEPALIVE, BGP_MSG_ROUTE_REFRESH,
};

// OSPF 共享模块导出（精简版）
pub use ospf::{
    OspfType, OspfOptions,
    OspfConfig, OspfInterfaceConfig, AuthAlgorithm, CryptoAuthConfig,
    IP_PROTO_OSPF, OSPF_ALL_SPF_ROUTERS, OSPF_ALL_D_ROUTERS,
    HELLO_INTERVAL_DEFAULT, DEAD_INTERVAL_DEFAULT, PRIORITY_DEFAULT,
    RETRANSMIT_INTERVAL_DEFAULT, TRANSMIT_DELAY_DEFAULT,
    OspfManager,
};

// OSPFv2 模块导出（精简版）
pub use ospf2::{
    OspfHeader, OspfHello, OspfDatabaseDescription, OspfLinkStateRequest,
    OspfLinkStateUpdate, OspfLinkStateAck, OspfType as OspfV2Type,
    OspfError, OspfResult,
    OspfV2Config,
    process_ospfv2_packet,
    OSPFV2_VERSION, IP_PROTO_OSPF as OSPFV2_PROTO,
    OSPF_ALL_SPF_ROUTERS as OSPFV2_ALL_SPF_ROUTERS,
    OSPF_ALL_D_ROUTERS as OSPFV2_ALL_D_ROUTERS,
};

// IPsec 模块导出
pub use ipsec::{
    AhHeader, AhPacket, IP_PROTO_AH,
    EspHeader, EspTrailer, EspPacket, IP_PROTO_ESP,
    SecurityAssociation, SaDirection, IpsecMode, IpsecProtocol,
    SecurityPolicy, PolicyAction, TrafficSelector,
    CipherTransform, AuthTransform,
    SadManager, SpdManager,
    IpsecError, IpsecResult,
    DEFAULT_REPLAY_WINDOW_SIZE,
};
