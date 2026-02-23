// src/lib.rs
//
// CoreNet库入口
// 网络协议栈学习/研究项目

// 公共模块声明
pub mod common;
pub mod context;
pub mod poweron;
pub mod engine;
pub mod scheduler;
pub mod protocols;
pub mod interface;
pub mod route;
pub mod socket;
pub mod testframework;

// 重新导出常用类型
pub use common::{
    // 错误类型
    CoreError, Result,

    // Packet相关
    Packet,

    // 地址类型
    MacAddr, Ipv4Addr, Ipv6Addr, IpAddr, AddrError,

    // 队列相关
    RingQueue,
    QueueError,

    // 队列常量
    DEFAULT_QUEUE_CAPACITY,
    MIN_QUEUE_CAPACITY,
    MAX_QUEUE_CAPACITY,
};

// 重新导出上电启动模块
pub use poweron::{
    boot_default,
    shutdown,
};

// 重新导出系统上下文（新的依赖注入方式）
pub use context::SystemContext as Context;

// 导出 interface 模块
pub use interface::{
    NetworkInterface, InterfaceState, InterfaceType,
    InterfaceManager, InterfaceConfig,
    load_default_config, save_config,
    InterfaceError,
    DEFAULT_CONFIG_PATH,
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
};

// 导出 ARP 模块
pub use protocols::arp::{
    ArpCache, ArpEntry, ArpState, ArpConfig,
};

// 导出 testframework 模块
pub use testframework::{
    TestHarness, PacketInjector,
    HarnessError, HarnessResult,
    GlobalStateManager,
};

// 导出路由模块
pub use route::{
    RouteTable,
    Ipv4Route,
    Ipv6Route,
    RouteLookup,
    RouteError,
};

// 导出 Socket 模块
pub use socket::{
    SocketManager, SocketError, SocketConfig,
    SocketFd, SocketAddr, SocketAddrV4, SocketAddrV6,
    AddressFamily, SocketType, SocketProtocol,
    SendFlags, RecvFlags, TcpState,
};
