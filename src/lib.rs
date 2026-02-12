// src/lib.rs
//
// CoreNet库入口
// 网络协议栈学习/研究项目

// 公共模块声明
pub mod common;

// 重新导出常用类型
pub use common::{
    // 错误类型
    CoreError, Result,

    // 网络类型
    MacAddr, IpAddr, IpVersion, EtherType, IpProtocol, Layer,

    // Packet相关
    Packet,

    // 队列相关
    RingQueue, SpscQueue, SafeQueue,
    QueueError, WaitStrategy, QueueConfig,

    // 队列常量
    DEFAULT_QUEUE_CAPACITY,
    MIN_QUEUE_CAPACITY,
    MAX_QUEUE_CAPACITY,
    DEFAULT_SPIN_COUNT,
    DEFAULT_TIMEOUT_MS,
};
