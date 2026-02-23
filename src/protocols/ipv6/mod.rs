// src/protocols/ipv6/mod.rs
//
// IPv6 协议模块
// 实现 IPv6 数据包解析、封装、验证
// 支持扩展头、分片重组等功能

mod header;
mod protocol;
mod error;
mod config;
mod packet;
mod extension;
mod fragment;
mod options;

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

// 扩展头相关导出
pub use extension::{
    ExtensionHeader,
    ExtensionHeaderType,
    ExtensionChainResult,
    ExtensionConfig,
    // 逐跳选项
    HopByHopHeader,
    // 路由头
    RoutingHeader,
    RoutingHeaderType2,
    // 分片头
    FragmentHeader,
    // 目的选项
    DestinationOptionsHeader,
    // 扩展头解析
    parse_extension_chain,
    EXTENSION_HEADER_MIN_LEN,
    DEFAULT_MAX_EXTENSION_HEADERS,
    DEFAULT_MAX_EXTENSION_HEADERS_LENGTH,
};

// 分片重组相关导出
pub use fragment::{
    ReassemblyKey,
    FragmentInfo,
    ReassemblyEntry,
    FragmentCache,
    ReassemblyError,
    create_fragments,
    DEFAULT_MAX_REASSEMBLY_ENTRIES,
    DEFAULT_REASSEMBLY_TIMEOUT,
    DEFAULT_MAX_FRAGMENTS_PER_PACKET,
};

// 选项处理相关导出
pub use options::{
    OptionType,
    Option as ExtensionOption,
    OptionsParseResult,
    RouterAlertOption,
    JumboPayloadOption,
    parse_options,
    create_padn,
    OPTION_TYPE_PAD1,
    OPTION_TYPE_PADN,
    OPTION_TYPE_ROUTER_ALERT,
    OPTION_TYPE_JUMBO_PAYLOAD,
};
