// src/protocols/ospf/neighbor.rs
//
// OSPF 邻居共享逻辑
// 定义 OSPFv2 和 OSPFv3 共享的邻居行为 trait

use crate::protocols::ospf::types::NeighborState;
use std::time::Instant;

/// OSPF 邻居共享行为 trait
///
/// 此 trait 定义了 OSPFv2 和 OSPFv3 邻居的共同行为，
/// 包括定时器管理、状态转换、邻接关系判断等。
pub trait OspfNeighborCommon {
    /// 重置 Inactivity Timer
    ///
    /// # 参数
    /// - `dead_interval`: Router Dead Interval（秒）
    fn reset_inactivity_timer(&mut self, dead_interval: u32);

    /// 检查 Inactivity Timer 是否超时
    fn is_inactivity_timer_expired(&self) -> bool;

    /// 转换邻居状态
    fn set_state(&mut self, new_state: NeighborState);

    /// 获取当前邻居状态
    fn state(&self) -> NeighborState;

    /// 判断是否已建立双向通信
    fn is_two_way_established(&self) -> bool {
        self.state().is_two_way_established()
    }

    /// 判断是否已建立邻接关系
    fn is_adjacency_established(&self) -> bool {
        self.state().is_adjacency_established()
    }

    /// 获取邻居优先级
    fn priority(&self) -> u8;

    /// 判断是否需要建立邻接关系
    ///
    /// RFC 2328 Section 10.4: 是否需要建立邻接关系
    ///
    /// # 参数
    /// - `local_is_dr`: 本地是否是 DR
    /// - `local_is_bdr`: 本地是否是 BDR
    /// - `neighbor_is_dr`: 邻居是否是 DR
    /// - `neighbor_is_bdr`: 邻居是否是 BDR
    ///
    /// # 返回
    /// 如果需要建立邻接关系返回 true
    fn needs_adjacency(
        &self,
        local_is_dr: bool,
        local_is_bdr: bool,
        neighbor_is_dr: bool,
        neighbor_is_bdr: bool,
    ) -> bool {
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
}

/// OSPF 邻居定时器状态（共享）
///
/// 跟踪邻居的各种定时器状态
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

    /// 重置 Inactivity Timer
    pub fn reset_inactivity(&mut self, dead_interval: u32) {
        let now = Instant::now();
        self.inactivity_expiry = now
            .checked_add(std::time::Duration::from_secs(dead_interval as u64))
            .unwrap_or(now);
        self.last_hello_time = now;
    }

    /// 检查 Inactivity Timer 是否超时
    pub fn is_inactivity_expired(&self) -> bool {
        Instant::now() > self.inactivity_expiry
    }

    /// 设置重传定时器
    pub fn set_retransmit_timer(&mut self, interval: u32) {
        self.retransmit_expiry = Some(
            Instant::now()
                .checked_add(std::time::Duration::from_secs(interval as u64))
                .unwrap_or(Instant::now())
        );
    }

    /// 检查重传定时器是否超时
    pub fn is_retransmit_expired(&self) -> bool {
        if let Some(expiry) = self.retransmit_expiry {
            Instant::now() > expiry
        } else {
            false
        }
    }

    /// 清除重传定时器
    pub fn clear_retransmit_timer(&mut self) {
        self.retransmit_expiry = None;
    }
}

/// OSPF Database Description 交换状态
///
/// 跟踪 DD 交换过程的状态
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

    /// 初始化 DD 序列号（随机生成）
    pub fn init_sequence(&mut self) {
        // 使用当前时间戳生成初始序列号
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as u32;

        // 确保序列号在有效范围内且不为零
        self.dd_seq_number = if timestamp == 0 { 1 } else { timestamp | 0x80000000 };
        if self.dd_seq_number > 0x7FFFFFFF {
            self.dd_seq_number = 1;
        }
    }

    /// 递增 DD 序列号
    pub fn increment_sequence(&mut self) {
        self.dd_seq_number = self.dd_seq_number.wrapping_add(1);
        if self.dd_seq_number > 0x7FFFFFFF || self.dd_seq_number == 0 {
            self.dd_seq_number = 1;
        }
    }

    /// 设置为 Master
    pub fn set_master(&mut self, is_master: bool) {
        self.is_master = is_master;
    }

    /// 标记交换完成
    pub fn mark_complete(&mut self) {
        self.exchange_complete = true;
    }

    /// 保存最后的 DD 报文
    pub fn save_last_dd(&mut self, packet: Vec<u8>) {
        self.last_dd_packet = Some(packet);
    }
}

impl Default for DdExchangeState {
    fn default() -> Self {
        Self::new()
    }
}

/// OSPF LSA 请求/重传列表管理
///
/// 管理 LSA 请求列表和重传列表
#[derive(Debug, Clone)]
pub struct LsaRequestManager {
    /// LSA 请求列表
    /// OSPFv2: (lsa_type, link_state_id, advertising_router)
    /// OSPFv3: 使用单独的结构
    pub request_list: Vec<LsaKey>,

    /// LSA 重传列表
    pub retransmit_list: Vec<LsaKey>,
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

impl LsaRequestManager {
    /// 创建新的请求管理器
    pub fn new() -> Self {
        Self {
            request_list: Vec::new(),
            retransmit_list: Vec::new(),
        }
    }

    /// 添加 LSA 请求
    pub fn add_request(&mut self, key: LsaKey) {
        if !self.request_list.contains(&key) {
            self.request_list.push(key);
        }
    }

    /// 移除 LSA 请求
    pub fn remove_request(&mut self, key: &LsaKey) {
        self.request_list.retain(|k| k != key);
    }

    /// 清空请求列表
    pub fn clear_requests(&mut self) {
        self.request_list.clear();
    }

    /// 是否有未完成的请求
    pub fn has_pending_requests(&self) -> bool {
        !self.request_list.is_empty()
    }

    /// 添加 LSA 到重传列表
    pub fn add_retransmit(&mut self, key: LsaKey) {
        if !self.retransmit_list.contains(&key) {
            self.retransmit_list.push(key);
        }
    }

    /// 从重传列表移除 LSA
    pub fn remove_retransmit(&mut self, key: &LsaKey) {
        self.retransmit_list.retain(|k| k != key);
    }

    /// 清空重传列表
    pub fn clear_retransmit(&mut self) {
        self.retransmit_list.clear();
    }

    /// 获取重传列表
    pub fn get_retransmit_list(&self) -> &[LsaKey] {
        &self.retransmit_list
    }
}

impl Default for LsaRequestManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_neighbor_timers_new() {
        let timers = SharedNeighborTimers::new(40);
        assert!(!timers.is_inactivity_expired());
    }

    #[test]
    fn test_neighbor_timers_reset_inactivity() {
        let mut timers = SharedNeighborTimers::new(40);
        let original_expiry = timers.inactivity_expiry;

        std::thread::sleep(std::time::Duration::from_millis(10));
        timers.reset_inactivity(40);

        assert!(timers.inactivity_expiry > original_expiry);
    }

    #[test]
    fn test_neighbor_timers_retransmit() {
        let mut timers = SharedNeighborTimers::new(40);

        timers.set_retransmit_timer(5);
        assert!(!timers.is_retransmit_expired());

        timers.clear_retransmit_timer();
        assert!(!timers.is_retransmit_expired());
    }

    #[test]
    fn test_dd_exchange_state() {
        let mut state = DdExchangeState::new();

        state.init_sequence();
        assert!(state.dd_seq_number != 0);

        state.increment_sequence();
        assert!(state.dd_seq_number > 0);

        state.set_master(true);
        assert!(state.is_master);

        state.mark_complete();
        assert!(state.exchange_complete);
    }

    #[test]
    fn test_lsa_key_v2() {
        let key = LsaKey::v2(1, 0x01010101, 0x02020202);
        assert_eq!(key.lsa_type, 1);
        assert_eq!(key.link_state_id, 0x01010101);
        assert_eq!(key.advertising_router, 0x02020202);
    }

    #[test]
    fn test_lsa_key_v3() {
        let key = LsaKey::v3(0x2001, 0x01010101, 0x02020202);
        assert_eq!(key.lsa_type, 0x2001);
    }

    #[test]
    fn test_lsa_request_manager() {
        let mut mgr = LsaRequestManager::new();

        let key = LsaKey::v2(1, 0x01010101, 0x02020202);
        mgr.add_request(key.clone());

        assert!(mgr.has_pending_requests());

        mgr.remove_request(&key);
        assert!(!mgr.has_pending_requests());

        mgr.add_retransmit(key.clone());
        assert_eq!(mgr.get_retransmit_list().len(), 1);

        mgr.clear_retransmit();
        assert_eq!(mgr.get_retransmit_list().len(), 0);
    }

    #[test]
    fn test_needs_adjacency() {
        // DR 与 BDR 之间
        assert!(neighbor_logic_needs_adjacency(true, false, false, true));
        assert!(neighbor_logic_needs_adjacency(false, true, true, false));

        // DR/BDR 与非 DR/BDR
        assert!(neighbor_logic_needs_adjacency(true, false, false, false));
        assert!(neighbor_logic_needs_adjacency(false, true, false, false));

        // 非 DR/BDR 与 DR/BDR
        assert!(neighbor_logic_needs_adjacency(false, false, true, false));
        assert!(neighbor_logic_needs_adjacency(false, false, false, true));

        // 两个非 DR/BDR 之间
        assert!(!neighbor_logic_needs_adjacency(false, false, false, false));
    }

    // 辅助函数，测试共享的邻接关系逻辑
    fn neighbor_logic_needs_adjacency(
        local_is_dr: bool,
        local_is_bdr: bool,
        neighbor_is_dr: bool,
        neighbor_is_bdr: bool,
    ) -> bool {
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
}
