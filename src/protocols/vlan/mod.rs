// src/common/protocols/vlan/mod.rs
//
// VLAN模块入口
// 负责802.1Q VLAN标签的解析和封装

mod tag;
mod frame;
mod error;
mod parse;

pub use tag::VlanTag;
pub use frame::VlanFrame;
pub use error::VlanError;

pub use parse::{
    has_vlan_tag,
    is_vlan_tpid,
    process_vlan_packet,
    VlanProcessResult,
};
