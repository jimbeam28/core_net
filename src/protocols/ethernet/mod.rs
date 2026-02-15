// src/common/protocols/ethernet/mod.rs
//
// 以太网协议定义
// 包含以太网头部结构和常量

mod header;

pub use header::{
    EthernetHeader,
    ETH_P_IP,
    ETH_P_ARP,
    ETH_P_IPV6,
    ETH_P_8021Q,
    ETH_P_8021AD,
};
