// src/engine/mod.rs
//
// 协议处理引擎模块
// 负责报文的协议解析和封装处理

mod processor;

// 导出报文处理器
pub use processor::{
    PacketProcessor,
    ProcessError,
    ProcessResult,
    process_packet,
    process_packet_verbose,
};
