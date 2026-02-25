// src/protocols/ospf3/neighbor.rs
//
// OSPFv3 邻居管理

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
    /// LSA 请求列表 (LSA 类型+链路状态 ID+通告路由器的组合)
    pub lsa_request_list: Vec<(u16, u32, u32)>,
    /// 最后收到的 DD 报文
    pub last_dd_packet: Option<Vec<u8>>,
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

        // 两个非 DR/BDR 路由器之间不需要建立邻接关系
        false
    }

    /// 初始化 DD 序列号
    pub fn init_dd_sequence(&mut self) {
        // 简化实现：使用当前时间戳的低32位
        self.dd_seq_number = (std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as u32) | 0x80000000;
    }
}
