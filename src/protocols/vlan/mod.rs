// src/protocols/vlan/mod.rs
//
// VLAN模块入口
// 负责802.1Q VLAN标签的解析和封装

mod tag;
mod frame;
mod error;
mod parse;
mod filter;

pub use tag::VlanTag;
pub use frame::VlanFrame;
pub use error::VlanError;
pub use filter::VlanFilter;

pub use parse::{
    has_vlan_tag,
    is_vlan_tpid,
    process_vlan_packet,
    VlanProcessResult,
    // 封装相关
    TPID_8021Q,
    TPID_QINQ,
    TPID_8021AD,
    VlanEncapParams,
    QinQEncapParams,
    encapsulate_vlan_frame_with_params,
    encapsulate_qinq_frame_with_params,
    add_vlan_tag,
    remove_vlan_tag,
};
