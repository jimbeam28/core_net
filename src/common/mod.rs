// src/common/mod.rs
//
// 通用模块
// 包含错误类型、通用类型定义、报文描述符、队列等

// 模块声明
pub mod error;
pub mod packet;
pub mod queue;
pub mod poweron;

// 导出错误类型
pub use error::{CoreError, Result};

// 导出Packet相关类型
pub use packet::Packet;

// 导出队列相关类型
pub use queue::{
    // 基础队列结构
    RingQueue,

    // 错误
    QueueError,

    // 队列配置
    QueueConfig,

    // 常量定义
    DEFAULT_QUEUE_CAPACITY,
    MIN_QUEUE_CAPACITY,
    MAX_QUEUE_CAPACITY,
};

// 导出上电启动模块类型
pub use poweron::{
    SystemConfig,
    SystemContext,
    boot,
    boot_default,
    boot_with_capacity,
    shutdown,
};
