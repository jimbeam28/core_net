// 通用模块：错误类型、队列、报文描述符、地址类型等
pub mod error;
pub mod queue;
pub mod packet;
pub mod addr;
pub mod timer;

pub use error::{CoreError, Result};
pub use queue::{RingQueue, QueueError, DEFAULT_QUEUE_CAPACITY, MIN_QUEUE_CAPACITY, MAX_QUEUE_CAPACITY};
pub use packet::Packet;
pub use addr::{MacAddr, Ipv4Addr, Ipv6Addr, IpAddr, AddrError};
pub use timer::{TimerManager, TimerHandle, TimerId, TimerType, ProtocolTimer};

// 重新导出协议模块的类型（保持向后兼容）
pub use crate::protocols::{
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
pub use crate::protocols::arp::tables::{ArpCache, ArpEntry, ArpState, ArpConfig, ArpKey};

// ==================== tables.rs 内容 ====================

/// 通用表接口 trait
/// 所有表类型都应实现此 trait 以提供统一的操作接口
pub trait Table<K, V> {
    /// 查找表项
    fn lookup(&self, key: &K) -> Option<&V>;

    /// 查找并返回可变引用
    fn lookup_mut(&mut self, key: &K) -> Option<&mut V>;

    /// 插入或更新表项
    fn insert(&mut self, key: K, value: V) -> Option<V>;

    /// 删除表项
    fn remove(&mut self, key: &K) -> Option<V>;

    /// 清空所有表项
    fn clear(&mut self);

    /// 获取表项数量
    fn len(&self) -> usize;

    /// 检查是否为空
    fn is_empty(&self) -> bool;

    /// 清理过期表项
    fn cleanup(&mut self);
}
