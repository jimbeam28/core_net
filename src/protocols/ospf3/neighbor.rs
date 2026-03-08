// src/protocols/ospf3/neighbor.rs
//
// OSPFv3 邻居管理（简化版）

use crate::common::Ipv6Addr;
use crate::protocols::ospf::NeighborState;
use std::time::Instant;

/// OSPFv3 邻居
#[derive(Debug, Clone)]
pub struct Ospfv3Neighbor {
    /// 邻居路由器 ID (32-bit)
    pub router_id: u32,
    /// 邻居链路本地地址
    pub link_local_addr: Ipv6Addr,
    /// 邻居状态
    pub state: NeighborState,
    /// 邻居优先级
    pub priority: u8,
    /// 邻居的 DR (32-bit)
    pub dr: u32,
    /// 邻居的 BDR (32-bit)
    pub bdr: u32,
    /// Database Description 序列号
    pub dd_seq_number: u32,
    /// 最后收到 Hello 的时间
    pub last_hello_time: Instant,
    /// Inactivity Timer
    pub inactivity_timer: Instant,
    /// 是否是 Master
    pub is_master: bool,
    /// Database Description 交换是否完成
    pub dd_exchange_complete: bool,
}

impl Ospfv3Neighbor {
    pub fn new(router_id: u32, link_local_addr: Ipv6Addr, dead_interval: u32) -> Self {
        let now = Instant::now();
        Self {
            router_id,
            link_local_addr,
            state: NeighborState::Down,
            priority: 1,
            dr: 0,
            bdr: 0,
            dd_seq_number: 0,
            last_hello_time: now,
            inactivity_timer: now,
            is_master: false,
            dd_exchange_complete: false,
        }
    }
}
