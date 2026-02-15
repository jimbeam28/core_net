// src/lib.rs
//
// CoreNet库入口
// 网络协议栈学习/研究项目

// 公共模块声明
pub mod common;
pub mod engine;
pub mod scheduler;
pub mod protocols;

// 重新导出常用类型
pub use common::{
    // 错误类型
    CoreError, Result,

    // Packet相关
    Packet,

    // 地址类型
    MacAddr, Ipv4Addr,

    // 队列相关
    RingQueue,
    QueueError,
    QueueConfig,

    // 队列常量
    DEFAULT_QUEUE_CAPACITY,
    MIN_QUEUE_CAPACITY,
    MAX_QUEUE_CAPACITY,

    // 上电启动模块
    SystemConfig,
    SystemContext,
    boot,
    boot_default,
    boot_with_capacity,
    shutdown,
};

// 导出 engine 模块
pub use engine::{
    PacketProcessor,
    ProcessResult,
    ProcessError,
    process_packet,
    process_packet_verbose,
};

// 导出 scheduler 模块
pub use scheduler::{
    Scheduler,
    ScheduleError,
    ScheduleResult,
    schedule_packets,
    schedule_packets_verbose,
};
