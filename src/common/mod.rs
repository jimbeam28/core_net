// src/common/mod.rs
//
// 通用模块
// 包含错误类型、通用类型定义、报文描述符、队列等

// 模块声明
pub mod error;
pub mod types;
pub mod packet;
pub mod queue;
pub mod pool;
pub mod poweron;

// 导出错误类型
pub use error::{CoreError, Result};

// 导出通用类型
pub use types::{
    MacAddr,
    IpAddr,
    IpVersion,
    EtherType,
    IpProtocol,
    Layer,
};

// 导出Packet相关类型
pub use packet::Packet;

// 导出队列相关类型
pub use queue::{
    // 基础队列结构
    RingQueue,
    SpscQueue,
    SafeQueue,

    // 错误和策略（重命名 WaitStrategy 避免与 pool 冲突）
    QueueError,
    WaitStrategy as QueueWaitStrategy,

    // 队列配置
    QueueConfig,

    // 常量定义
    DEFAULT_QUEUE_CAPACITY,
    MIN_QUEUE_CAPACITY,
    MAX_QUEUE_CAPACITY,
    DEFAULT_SPIN_COUNT,
    DEFAULT_TIMEOUT_MS,
};

// 导出对象池相关类型
pub use pool::{
    // 基础池结构
    Pool,
    Pooled,
    Clear,

    // 错误和策略
    PoolError,
    AllocStrategy,
    PoolConfig,
    WaitStrategy as PoolWaitStrategy,

    // 统计和状态
    PoolStats,
    PoolStatus,

    // Packet 适配层 - 暂时禁用，需要根据新 Packet API 重新设计
    // PacketPool,
    // PacketPoolConfig,
    // PacketPoolStats,
    // PacketBuilder,
    // PooledPacket,
};

// 导出上电启动模块类型
pub use poweron::{
    SystemConfig,
    SystemContext,
    SystemState,
    SystemStatus,
    boot,
    boot_default,
    boot_with_capacity,
    shutdown,
    default as poweron_default,
    high_throughput,
    low_latency,
    memory_constrained,
    with_capacity as poweron_with_capacity,
};
