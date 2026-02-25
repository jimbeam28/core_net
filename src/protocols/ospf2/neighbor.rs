// src/protocols/ospf2/neighbor.rs
//
// OSPFv2 邻居状态机

use crate::common::Ipv4Addr;
use std::time::Instant;

/// 邻居状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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

/// OSPFv2 邻居
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

    /// 请求的 LSA 列表
    pub lsa_request_list: Vec<(u8, Ipv4Addr, Ipv4Addr)>,

    /// 最后的 DD 报文
    pub last_dd_packet: Option<Vec<u8>>,
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
            inactivity_timer: now
                .checked_add(std::time::Duration::from_secs(dead_interval as u64))
                .unwrap_or(now),
            is_master: false,
            dd_exchange_complete: false,
            lsa_request_list: Vec::new(),
            last_dd_packet: None,
        }
    }

    /// 重置 Inactivity Timer
    pub fn reset_inactivity_timer(&mut self, dead_interval: u32) {
        self.inactivity_timer = Instant::now()
            .checked_add(std::time::Duration::from_secs(dead_interval as u64))
            .unwrap_or(self.inactivity_timer);
        self.last_hello_time = Instant::now();
    }

    /// 检查 Inactivity Timer 是否超时
    pub fn is_inactivity_timer_expired(&self) -> bool {
        Instant::now() > self.inactivity_timer
    }

    /// 转换邻居状态
    pub fn set_state(&mut self, new_state: NeighborState) {
        self.state = new_state;
    }

    /// 是否需要建立邻接关系
    pub fn needs_adjacency(&self, local_is_dr: bool, local_is_bdr: bool, neighbor_is_dr: bool, neighbor_is_bdr: bool) -> bool {
        // DR 与 BDR 之间必须建立邻接关系
        if (local_is_dr && neighbor_is_bdr) || (local_is_bdr && neighbor_is_dr) {
            return true;
        }

        // DR/BDR 与所有非 DR/BDR 路由器建立邻接关系
        if local_is_dr || local_is_bdr {
            return !neighbor_is_dr && !neighbor_is_bdr;
        }

        // 非 DR/BDR 路由器与 DR/BDR 建立邻接关系
        if neighbor_is_dr || neighbor_is_bdr {
            return true;
        }

        false
    }

    /// 初始化 DD 序列号
    pub fn init_dd_sequence(&mut self) {
        self.dd_seq_number = rand::random::<u32>() & 0x7FFFFFFF;
        if self.dd_seq_number == 0 {
            self.dd_seq_number = 1;
        }
    }

    /// 递增 DD 序列号
    pub fn increment_dd_sequence(&mut self) {
        self.dd_seq_number = self.dd_seq_number.wrapping_add(1);
        if self.dd_seq_number > 0x7FFFFFFF {
            self.dd_seq_number = 1;
        }
    }

    /// 添加 LSA 请求
    pub fn add_lsa_request(&mut self, lsa_type: u8, link_state_id: Ipv4Addr, advertising_router: Ipv4Addr) {
        let key = (lsa_type, link_state_id, advertising_router);
        if !self.lsa_request_list.contains(&key) {
            self.lsa_request_list.push(key);
        }
    }

    /// 移除 LSA 请求
    pub fn remove_lsa_request(&mut self, lsa_type: u8, link_state_id: Ipv4Addr, advertising_router: Ipv4Addr) {
        self.lsa_request_list.retain(|&(t, id, ar)| {
            t != lsa_type || id != link_state_id || ar != advertising_router
        });
    }

    /// 清空 LSA 请求列表
    pub fn clear_lsa_requests(&mut self) {
        self.lsa_request_list.clear();
    }

    /// 是否有未完成的 LSA 请求
    pub fn has_pending_requests(&self) -> bool {
        !self.lsa_request_list.is_empty()
    }
}

// 简化的随机数生成（实际应使用更复杂的实现）
mod rand {
    pub fn random<T>() -> T {
        // 简化实现：返回一个固定值
        // 实际实现应使用操作系统随机数生成器
        let val: u32 = 0x12345678;
        unsafe { std::mem::transmute_copy(&val) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_neighbor_state_is_two_way_established() {
        assert!(!NeighborState::Down.is_two_way_established());
        assert!(!NeighborState::Init.is_two_way_established());
        assert!(NeighborState::TwoWay.is_two_way_established());
        assert!(NeighborState::Full.is_two_way_established());
    }

    #[test]
    fn test_ospf_neighbor_new() {
        let neighbor = OspfNeighbor::new(
            Ipv4Addr::new(1, 1, 1, 2),
            Ipv4Addr::new(192, 168, 1, 2),
            40,
        );

        assert_eq!(neighbor.router_id, Ipv4Addr::new(1, 1, 1, 2));
        assert_eq!(neighbor.state, NeighborState::Down);
    }

    #[test]
    fn test_ospf_neighbor_reset_inactivity_timer() {
        let mut neighbor = OspfNeighbor::new(
            Ipv4Addr::new(1, 1, 1, 2),
            Ipv4Addr::new(192, 168, 1, 2),
            40,
        );

        let original_timer = neighbor.inactivity_timer;
        std::thread::sleep(std::time::Duration::from_millis(10));
        neighbor.reset_inactivity_timer(40);

        assert!(neighbor.inactivity_timer > original_timer);
    }

    #[test]
    fn test_ospf_neighbor_set_state() {
        let mut neighbor = OspfNeighbor::new(
            Ipv4Addr::new(1, 1, 1, 2),
            Ipv4Addr::new(192, 168, 1, 2),
            40,
        );

        neighbor.set_state(NeighborState::Init);
        assert_eq!(neighbor.state, NeighborState::Init);

        neighbor.set_state(NeighborState::Full);
        assert_eq!(neighbor.state, NeighborState::Full);
    }

    #[test]
    fn test_ospf_neighbor_lsa_requests() {
        let mut neighbor = OspfNeighbor::new(
            Ipv4Addr::new(1, 1, 1, 2),
            Ipv4Addr::new(192, 168, 1, 2),
            40,
        );

        neighbor.add_lsa_request(1, Ipv4Addr::new(1, 1, 1, 1), Ipv4Addr::new(1, 1, 1, 1));
        assert!(neighbor.has_pending_requests());

        neighbor.remove_lsa_request(1, Ipv4Addr::new(1, 1, 1, 1), Ipv4Addr::new(1, 1, 1, 1));
        assert!(!neighbor.has_pending_requests());
    }
}
