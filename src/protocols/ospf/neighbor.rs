// src/protocols/ospf/neighbor.rs
//
// OSPF 邻居共享逻辑接口定义（简化版）

use crate::protocols::ospf::types::NeighborState;
use std::time::Instant;

/// OSPF 邻居共享行为 trait
pub trait OspfNeighborCommon {
    /// 重置 Inactivity Timer
    fn reset_inactivity_timer(&mut self, dead_interval: u32);
    /// 检查 Inactivity Timer 是否超时
    fn is_inactivity_timer_expired(&self) -> bool;
    /// 转换邻居状态
    fn set_state(&mut self, new_state: NeighborState);
    /// 获取当前邻居状态
    fn state(&self) -> NeighborState;
    /// 判断是否已建立双向通信
    fn is_two_way_established(&self) -> bool;
    /// 判断是否已建立邻接关系
    fn is_adjacency_established(&self) -> bool;
    /// 获取邻居优先级
    fn priority(&self) -> u8;
    /// 判断是否需要建立邻接关系
    fn needs_adjacency(
        &self,
        local_is_dr: bool,
        local_is_bdr: bool,
        neighbor_is_dr: bool,
        neighbor_is_bdr: bool,
    ) -> bool;
}

/// OSPF 邻居定时器状态（共享）
#[derive(Debug, Clone)]
pub struct SharedNeighborTimers {
    /// Inactivity Timer 到期时间
    pub inactivity_expiry: Instant,
    /// 最后收到 Hello 的时间
    pub last_hello_time: Instant,
    /// 重传定时器到期时间（可选）
    pub retransmit_expiry: Option<Instant>,
}

impl SharedNeighborTimers {
    /// 创建新的定时器状态
    pub fn new(dead_interval: u32) -> Self {
        let now = Instant::now();
        Self {
            inactivity_expiry: now
                .checked_add(std::time::Duration::from_secs(dead_interval as u64))
                .unwrap_or(now),
            last_hello_time: now,
            retransmit_expiry: None,
        }
    }
}

/// OSPF Database Description 交换状态
#[derive(Debug, Clone)]
pub struct DdExchangeState {
    /// DD 序列号
    pub dd_seq_number: u32,
    /// 是否是 Master
    pub is_master: bool,
    /// DD 交换是否完成
    pub exchange_complete: bool,
    /// 最后收到的 DD 报文（可选）
    pub last_dd_packet: Option<Vec<u8>>,
}

impl DdExchangeState {
    /// 创建新的 DD 交换状态
    pub fn new() -> Self {
        Self {
            dd_seq_number: 0,
            is_master: false,
            exchange_complete: false,
            last_dd_packet: None,
        }
    }
}

impl Default for DdExchangeState {
    fn default() -> Self {
        Self::new()
    }
}

/// LSA 键值（用于请求和重传列表）
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct LsaKey {
    /// LSA 类型（OSPFv2: u8, OSPFv3: u16）
    pub lsa_type: u16,
    /// 链路状态 ID（OSPFv2: Ipv4Addr, OSPFv3: u32）
    pub link_state_id: u32,
    /// 通告路由器（OSPFv2: Ipv4Addr, OSPFv3: u32）
    pub advertising_router: u32,
}

impl LsaKey {
    /// 创建 OSPFv2 LSA 键值
    pub fn v2(lsa_type: u8, link_state_id: u32, advertising_router: u32) -> Self {
        Self {
            lsa_type: lsa_type as u16,
            link_state_id,
            advertising_router,
        }
    }

    /// 创建 OSPFv3 LSA 键值
    pub fn v3(lsa_type: u16, link_state_id: u32, advertising_router: u32) -> Self {
        Self {
            lsa_type,
            link_state_id,
            advertising_router,
        }
    }
}

/// OSPF LSA 请求/重传列表管理
#[derive(Debug, Clone)]
pub struct LsaRequestManager {
    /// LSA 请求列表
    pub request_list: Vec<LsaKey>,
    /// LSA 重传列表
    pub retransmit_list: Vec<LsaKey>,
}

impl LsaRequestManager {
    /// 创建新的请求管理器
    pub fn new() -> Self {
        Self {
            request_list: Vec::new(),
            retransmit_list: Vec::new(),
        }
    }
}

impl Default for LsaRequestManager {
    fn default() -> Self {
        Self::new()
    }
}
