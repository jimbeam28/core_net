// src/protocols/icmp/mod.rs
//
// ICMP 协议模块
// 实现 Echo Request/Reply、Destination Unreachable、Time Exceeded

mod types;
mod packet;
mod echo;
mod process;
mod global;

pub use types::{
    IcmpType,
    DestUnreachableCode,
    TimeExceededCode,
    IcmpV6Type,
    IcmpV6DestUnreachableCode,
    IcmpV6TimeExceededCode,
    ICMP_TYPE_ECHO_REPLY,
    ICMP_TYPE_ECHO_REQUEST,
    ICMP_TYPE_DEST_UNREACHABLE,
    ICMP_TYPE_TIME_EXCEEDED,
    ICMPV6_TYPE_DEST_UNREACHABLE,
    ICMPV6_TYPE_PACKET_TOO_BIG,
    ICMPV6_TYPE_TIME_EXCEEDED,
    ICMPV6_TYPE_PARAMETER_PROBLEM,
    ICMPV6_TYPE_ECHO_REQUEST,
    ICMPV6_TYPE_ECHO_REPLY,
};
pub use packet::{
    IcmpEcho,
    IcmpDestUnreachable,
    IcmpTimeExceeded,
    IcmpPacket,
    IcmpV6Echo,
    IcmpV6Packet,
    extract_ip_header_plus_data,
    validate_original_datagram,
    is_broadcast_addr,
    is_multicast_addr,
    ICMP_ORIGINAL_DATAGRAM_MIN_LEN,
};
pub use echo::{
    EchoProcessResult,
    handle_echo_request,
    handle_echo_reply,
    register_echo_request,
    cleanup_echo_timeouts,
};
pub use process::{
    IcmpProcessResult,
    process_icmp_packet,
    process_icmpv6_packet,
    create_echo_request,
    create_echo_reply,
    create_dest_unreachable,
    create_time_exceeded,
    create_icmpv6_echo_request,
    create_icmpv6_echo_reply,
};
pub use global::{
    PendingEcho,
    EchoManager,
    IcmpConfig,
};
