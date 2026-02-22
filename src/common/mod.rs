// 通用模块：错误类型、队列、报文描述符、地址类型等
pub mod error;
pub mod queue;
pub mod tables;
pub mod packet;
pub mod addr;
pub mod timer;

pub use error::{CoreError, Result};
pub use queue::{RingQueue, QueueError, DEFAULT_QUEUE_CAPACITY, MIN_QUEUE_CAPACITY, MAX_QUEUE_CAPACITY};
pub use tables::Table;
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
