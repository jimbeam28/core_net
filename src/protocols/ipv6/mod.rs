// src/protocols/ipv6/mod.rs
//
// IPv6 协议模块
// 实现 IPv6 数据包解析、封装、验证
// 当前版本不支持分片、扩展头和转发

mod header;
mod protocol;
mod error;
mod config;
mod packet;

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
