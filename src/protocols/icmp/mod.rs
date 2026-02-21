// src/protocols/icmp/mod.rs
//
// ICMP 协议模块
// 实现 Echo Request/Reply、Destination Unreachable、Time Exceeded

mod types;
mod packet;
mod echo;
mod process;
mod global;

pub use types::{
    IcmpType,
    DestUnreachableCode,
    TimeExceededCode,
    ICMP_TYPE_ECHO_REPLY,
    ICMP_TYPE_ECHO_REQUEST,
    ICMP_TYPE_DEST_UNREACHABLE,
    ICMP_TYPE_TIME_EXCEEDED,
};
pub use packet::{
    IcmpEcho,
    IcmpDestUnreachable,
    IcmpTimeExceeded,
    IcmpPacket,
};
pub use echo::{
    EchoProcessResult,
    handle_echo_request,
    handle_echo_reply,
    register_echo_request,
    cleanup_echo_timeouts,
};
pub use process::{
    IcmpProcessResult,
    process_icmp_packet,
    create_echo_request,
    create_echo_reply,
    create_dest_unreachable,
    create_time_exceeded,
};
pub use global::{
    PendingEcho,
    EchoManager,
};

// ====== 全局状态 API（已弃用，请使用 SystemContext）======

/// 获取全局 Echo 管理器
///
/// # 已弃用
///
/// 请使用 `SystemContext.icmp_echo` 替代。全局状态模式已被依赖注入架构取代。
#[deprecated(note = "使用 SystemContext.icmp_echo 替代")]
pub use global::get_or_init_global_echo_manager;

/// 初始化全局 Echo 管理器
///
/// # 已弃用
///
/// 请使用 `SystemContext::with_components()` 创建自定义上下文。
#[deprecated(note = "使用 SystemContext::with_components() 替代")]
pub use global::init_global_echo_manager;
