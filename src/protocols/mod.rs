// src/protocols/mod.rs
//
// 协议模块声明

// 以太网协议
pub mod ethernet;

// VLAN协议
pub mod vlan;

// ARP协议
pub mod arp;

// 从 common 模块重新导出类型
pub use crate::common::{
    Packet,
    MacAddr,
    Ipv4Addr,
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
