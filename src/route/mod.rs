// src/route/mod.rs
//
// 路由模块 - 管理系统的路由表，提供路由查找功能
// 支持 IPv4 和 IPv6 路由，使用最长前缀匹配（LPM）算法

mod ipv4;
mod ipv6;
mod table;

pub use ipv4::Ipv4Route;
pub use ipv6::Ipv6Route;
pub use table::{RouteTable, RouteLookup};

// ==================== error.rs 内容 ====================

use std::fmt;
use crate::common::addr::Ipv4Addr;

/// 路由模块错误类型
#[derive(Debug)]
pub enum RouteError {
    /// 路由已存在
    RouteAlreadyExists {
        destination: String,
    },

    /// 路由不存在
    RouteNotFound {
        destination: String,
    },

    /// 接口不存在
    InterfaceNotFound {
        interface: String,
    },

    /// 无效的前缀长度
    InvalidPrefixLength {
        prefix_len: u8,
    },

    /// 无效的子网掩码
    InvalidNetmask {
        netmask: Ipv4Addr,
    },

    /// 路由表已满
    RouteTableFull,
}

impl fmt::Display for RouteError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::RouteAlreadyExists { destination } => {
                write!(f, "Route already exists: {}", destination)
            }
            Self::RouteNotFound { destination } => {
                write!(f, "Route not found: {}", destination)
            }
            Self::InterfaceNotFound { interface } => {
                write!(f, "Interface not found: {}", interface)
            }
            Self::InvalidPrefixLength { prefix_len } => {
                write!(f, "Invalid prefix length: {}", prefix_len)
            }
            Self::InvalidNetmask { netmask } => {
                write!(f, "Invalid netmask: {}", netmask)
            }
            Self::RouteTableFull => {
                write!(f, "Route table is full")
            }
        }
    }
}

impl std::error::Error for RouteError {}
