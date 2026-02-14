// src/scheduler/mod.rs
//
// 调度模块
// 负责从接收队列中取出报文并调度给协议处理引擎

mod scheduler;

// 导出公共接口
pub use scheduler::{
    Scheduler,
    ScheduleError,
    ScheduleResult,
    schedule_packets,
    schedule_packets_verbose,
};
