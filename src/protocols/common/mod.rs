// src/protocols/common/mod.rs
//
// 协议公共模块
// 包含报文描述符、地址类型等协议栈各层共享的基础类型

pub mod packet;
pub mod addr;

// 导出Packet类型
pub use packet::Packet;

// 导出地址类型
pub use addr::{MacAddr, Ipv4Addr};
