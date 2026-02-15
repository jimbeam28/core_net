// src/protocols/mod.rs
//
// 协议模块声明

// 公共模块（Packet、地址类型等）
pub mod common;

// 以太网协议
pub mod ethernet;

// VLAN协议
pub mod vlan;

// ARP协议
pub mod arp;

// 导出常用类型
pub use common::{
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
