// src/route/mod.rs
//
// 路由模块 - 管理系统的路由表，提供路由查找功能
// 支持 IPv4 和 IPv6 路由，使用最长前缀匹配（LPM）算法

mod ipv4;
mod ipv6;
mod table;
mod error;

pub use ipv4::Ipv4Route;
pub use ipv6::Ipv6Route;
pub use table::{RouteTable, RouteLookup};
pub use error::RouteError;
