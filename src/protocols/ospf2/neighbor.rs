// src/protocols/ospf2/neighbor.rs
//
// OSPFv2 邻居状态机（简化版）

use crate::common::Ipv4Addr;
use std::time::Instant;

/// 邻居状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum NeighborState {
    Down,
    Attempt,
    Init,
    TwoWay,
    ExStart,
    Exchange,
    Loading,
    Full,
}

impl NeighborState {
    pub fn name(&self) -> &'static str {
        match self {
            NeighborState::Down => "Down",
            NeighborState::Attempt => "Attempt",
            NeighborState::Init => "Init",
            NeighborState::TwoWay => "2-Way",
            NeighborState::ExStart => "ExStart",
            NeighborState::Exchange => "Exchange",
            NeighborState::Loading => "Loading",
            NeighborState::Full => "Full",
        }
    }

    pub fn is_two_way_established(&self) -> bool {
        matches!(self,
            NeighborState::TwoWay |
            NeighborState::ExStart |
            NeighborState::Exchange |
            NeighborState::Loading |
            NeighborState::Full
        )
    }

    pub fn is_adjacency_established(&self) -> bool {
        matches!(self, NeighborState::Full)
    }
}

/// OSPFv2 邻居（简化版）
#[derive(Debug, Clone)]
pub struct OspfNeighbor {
    /// 邻居路由器 ID
    pub router_id: Ipv4Addr,
    /// 邻居 IP 地址
    pub ip_addr: Ipv4Addr,
    /// 邻居状态
    pub state: NeighborState,
    /// 邻居优先级
    pub priority: u8,
    /// 邻居的 DR
    pub dr: Ipv4Addr,
    /// 邻居的 BDR
    pub bdr: Ipv4Addr,
    /// Database Description 序列号
    pub dd_seq_number: u32,
    /// 最后收到 Hello 的时间
    pub last_hello_time: Instant,
    /// Inactivity Timer
    pub inactivity_timer: Instant,
    /// 是否是 Master
    pub is_master: bool,
    /// DD 交换是否完成
    pub dd_exchange_complete: bool,
}

impl OspfNeighbor {
    /// 创建新的邻居
    pub fn new(router_id: Ipv4Addr, ip_addr: Ipv4Addr, dead_interval: u32) -> Self {
        let now = Instant::now();
        Self {
            router_id,
            ip_addr,
            state: NeighborState::Down,
            priority: 1,
            dr: Ipv4Addr::UNSPECIFIED,
            bdr: Ipv4Addr::UNSPECIFIED,
            dd_seq_number: 0,
            last_hello_time: now,
            inactivity_timer: now,
            is_master: false,
            dd_exchange_complete: false,
        }
    }
}
