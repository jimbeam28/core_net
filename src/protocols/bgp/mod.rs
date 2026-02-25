// src/protocols/bgp/mod.rs
//
// BGP（边界网关协议）模块
// 实现 RFC 4271 Border Gateway Protocol 4 (BGP-4)

mod config;
mod error;
mod message;
mod packet;
mod peer;
mod rib;

pub use config::{BgpConfig, BgpPeerConfig, BgpPolicy, BgpPeerType};
pub use error::{BgpError, Result};
pub use message::*;
pub use packet::{parse_bgp_message, encapsulate_bgp_message};
pub use peer::{BgpPeer, BgpPeerManager, BgpState};
pub use rib::{BgpRoute, BgpRib};

// BGP 常量
pub const BGP_PORT: u16 = 179;
pub const BGP_VERSION: u8 = 4;
pub const BGP_MARKER_SIZE: usize = 16;
pub const BGP_HEADER_SIZE: usize = 18;
pub const BGP_MIN_MESSAGE_SIZE: usize = 19;
pub const BGP_MAX_MESSAGE_SIZE: usize = 4096;

// 默认定时器值
pub const DEFAULT_HOLD_TIME: u16 = 180;
pub const DEFAULT_CONNECT_RETRY_TIME: u16 = 60;
pub const DEFAULT_KEEPALIVE_TIME: u16 = 60;

// 消息类型
pub const BGP_MSG_OPEN: u8 = 1;
pub const BGP_MSG_UPDATE: u8 = 2;
pub const BGP_MSG_NOTIFICATION: u8 = 3;
pub const BGP_MSG_KEEPALIVE: u8 = 4;
pub const BGP_MSG_ROUTE_REFRESH: u8 = 5;
