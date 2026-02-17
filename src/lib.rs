// src/lib.rs
//
// CoreNet库入口
// 网络协议栈学习/研究项目

// 公共模块声明
pub mod common;
pub mod poweron;
pub mod engine;
pub mod scheduler;
pub mod protocols;
pub mod interface;

// 重新导出常用类型
pub use common::{
    // 错误类型
    CoreError, Result,

    // Packet相关
    Packet,

    // 地址类型
    MacAddr, Ipv4Addr, AddrError,

    // 队列相关
    RingQueue,
    QueueError,
    QueueConfig,

    // 队列常量
    DEFAULT_QUEUE_CAPACITY,
    MIN_QUEUE_CAPACITY,
    MAX_QUEUE_CAPACITY,
};

// 重新导出上电启动模块
pub use poweron::{
    SystemContext,
    boot_default,
    shutdown,
};

// 导出 interface 模块
pub use interface::{
    NetworkInterface, InterfaceState, InterfaceType,
    InterfaceManager, InterfaceConfig,
    load_default_config, save_config,
    InterfaceError,
    DEFAULT_CONFIG_PATH,
    // 全局接口管理器
    init_global_manager, init_default, global_manager,
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

// 导出 ARP 模块
pub use protocols::arp::{
    init_default_arp_cache, init_global_arp_cache,
    ArpCache, ArpEntry, ArpState, ArpConfig,
};
