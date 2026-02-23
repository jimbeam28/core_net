// src/protocols/icmpv6/mod.rs
//
// ICMPv6 协议模块
// 实现 ICMPv6 错误报告、Echo 请求/响应、邻居发现协议 (NDP)

mod types;
mod packet;
mod neighbor;
mod config;
mod error;
mod process;
mod checksum;

// ========== 公共导出 ==========

pub use types::*;
pub use packet::*;
pub use neighbor::*;
pub use config::*;
pub use error::*;
pub use process::*;
pub use checksum::*;

// 重新导出 ErrorRateLimiter
pub use neighbor::ErrorRateLimiter;
