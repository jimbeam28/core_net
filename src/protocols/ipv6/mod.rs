// src/protocols/ipv6/mod.rs
//
// IPv6 协议模块（精简版）
// 实现 IPv6 数据包解析、封装、验证
// 不支持扩展头链处理

mod header;
mod protocol;
mod error;
mod config;
mod packet;
mod fragment;

pub use header::{
    Ipv6Header,
    IPV6_VERSION,
    IPV6_HEADER_LEN,
    IPV6_MIN_MTU,
    DEFAULT_HOP_LIMIT,
};

pub use protocol::IpProtocol;

pub use error::Ipv6Error;

pub use config::Ipv6Config;

pub use packet::{
    Ipv6ProcessResult,
    Ipv6Result,
    process_ipv6_packet,
    encapsulate_ipv6_packet,
};

// 分片重组相关导出（保留基础功能）
pub use fragment::{
    ReassemblyKey,
    FragmentInfo,
    ReassemblyEntry,
    FragmentCache,
    ReassemblyError,
    FragmentPacket,
    create_fragments_simple,
    DEFAULT_MAX_REASSEMBLY_ENTRIES,
    DEFAULT_REASSEMBLY_TIMEOUT,
};
