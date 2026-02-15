// src/common/mod.rs
//
// 通用模块
// 包含错误类型、队列、上电启动等通用功能

// 模块声明
pub mod error;
pub mod queue;
pub mod poweron;
pub mod tables;

// 导出错误类型
pub use error::{CoreError, Result};

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

// 导出表相关类型
pub use tables::Table;

// 重新导出协议模块的公共类型（保持向后兼容）
pub use crate::protocols::{
    Packet,
    MacAddr,
    Ipv4Addr,
    EthernetHeader,
    ETH_P_IP,
    ETH_P_ARP,
    ETH_P_IPV6,
    ETH_P_8021Q,
    ETH_P_8021AD,
    VlanTag,
    VlanFrame,
    VlanError,
    has_vlan_tag,
    is_vlan_tpid,
};

// 重新导出 ARP 表类型（保持向后兼容）
pub use crate::protocols::arp::tables::{
    ArpCache, ArpEntry, ArpState, ArpConfig, ArpKey,
};
