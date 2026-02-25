// src/protocols/bgp/timer.rs
//
// BGP 定时器管理
// BGP 需要维护多种定时器：Keepalive、Hold、Connect Retry

use std::collections::HashMap;
use std::net::IpAddr;
use std::time::{Duration, Instant};
use super::BgpTimerEvent;

/// 定时器 ID
pub type TimerId = u64;

/// 定时器状态
#[derive(Debug, Clone)]
pub struct BgpTimer {
    /// 定时器 ID
    pub id: TimerId,
    /// 关联的对等体地址
    pub peer_addr: IpAddr,
    /// 定时器类型
    pub timer_type: BgpTimerEvent,
    /// 到期时间
    pub expires_at: Instant,
    /// 是否活跃
    pub active: bool,
}

impl BgpTimer {
    /// 创建新的定时器
    pub fn new(id: TimerId, peer_addr: IpAddr, timer_type: BgpTimerEvent, delay: Duration) -> Self {
        Self {
            id,
            peer_addr,
            timer_type,
            expires_at: Instant::now() + delay,
            active: true,
        }
    }

    /// 检查是否到期
    pub fn is_expired(&self) -> bool {
        self.active && Instant::now() >= self.expires_at
    }

    /// 重置定时器
    pub fn reset(&mut self, delay: Duration) {
        self.expires_at = Instant::now() + delay;
        self.active = true;
    }

    /// 停用定时器
    pub fn deactivate(&mut self) {
        self.active = false;
    }
}

/// BGP 定时器管理器
#[derive(Debug)]
pub struct BgpTimerManager {
    /// 定时器列表（按 ID 索引）
    timers: HashMap<TimerId, BgpTimer>,
    /// 下一个定时器 ID
    next_id: TimerId,
    /// 对等体到定时器 ID 的映射（用于快速查找）
    peer_timers: HashMap<IpAddr, PeerTimers>,
}

/// 对等体的定时器
#[derive(Debug, Clone)]
struct PeerTimers {
    /// Keepalive 定时器 ID
    keepalive_id: Option<TimerId>,
    /// Hold 定时器 ID
    hold_id: Option<TimerId>,
    /// Connect Retry 定时器 ID
    connect_retry_id: Option<TimerId>,
}

impl BgpTimerManager {
    /// 创建新的定时器管理器
    pub fn new() -> Self {
        Self {
            timers: HashMap::new(),
            next_id: 1,
            peer_timers: HashMap::new(),
        }
    }

    /// 添加 Keepalive 定时器
    ///
    /// # 参数
    /// - `peer_addr`: 对等体地址
    /// - `delay`: 延迟时间
    ///
    /// # 返回
    /// 定时器 ID
    pub fn add_keepalive_timer(&mut self, peer_addr: IpAddr, delay: Duration) -> TimerId {
        let id = self.next_id;
        self.next_id += 1;

        let timer = BgpTimer::new(id, peer_addr, BgpTimerEvent::KeepaliveTimer, delay);
        self.timers.insert(id, timer);

        // 更新对等体定时器映射
        let peer_timers = self.peer_timers.entry(peer_addr).or_insert_with(|| PeerTimers {
            keepalive_id: None,
            hold_id: None,
            connect_retry_id: None,
        });
        peer_timers.keepalive_id = Some(id);

        id
    }

    /// 添加 Hold 定时器
    ///
    /// # 参数
    /// - `peer_addr`: 对等体地址
    /// - `delay`: 延迟时间
    ///
    /// # 返回
    /// 定时器 ID
    pub fn add_hold_timer(&mut self, peer_addr: IpAddr, delay: Duration) -> TimerId {
        let id = self.next_id;
        self.next_id += 1;

        let timer = BgpTimer::new(id, peer_addr, BgpTimerEvent::HoldTimer, delay);
        self.timers.insert(id, timer);

        // 更新对等体定时器映射
        let peer_timers = self.peer_timers.entry(peer_addr).or_insert_with(|| PeerTimers {
            keepalive_id: None,
            hold_id: None,
            connect_retry_id: None,
        });
        peer_timers.hold_id = Some(id);

        id
    }

    /// 添加 Connect Retry 定时器
    ///
    /// # 参数
    /// - `peer_addr`: 对等体地址
    /// - `delay`: 延迟时间
    ///
    /// # 返回
    /// 定时器 ID
    pub fn add_connect_retry_timer(&mut self, peer_addr: IpAddr, delay: Duration) -> TimerId {
        let id = self.next_id;
        self.next_id += 1;

        let timer = BgpTimer::new(id, peer_addr, BgpTimerEvent::ConnectRetryTimer, delay);
        self.timers.insert(id, timer);

        // 更新对等体定时器映射
        let peer_timers = self.peer_timers.entry(peer_addr).or_insert_with(|| PeerTimers {
            keepalive_id: None,
            hold_id: None,
            connect_retry_id: None,
        });
        peer_timers.connect_retry_id = Some(id);

        id
    }

    /// 重置 Keepalive 定时器
    ///
    /// # 参数
    /// - `peer_addr`: 对等体地址
    /// - `delay`: 新的延迟时间
    ///
    /// # 返回
    /// 是否成功重置
    pub fn reset_keepalive_timer(&mut self, peer_addr: &IpAddr, delay: Duration) -> bool {
        if let Some(peer_timers) = self.peer_timers.get(peer_addr)
            && let Some(id) = peer_timers.keepalive_id
            && let Some(timer) = self.timers.get_mut(&id)
        {
            timer.reset(delay);
            return true;
        }
        false
    }

    /// 重置 Hold 定时器
    ///
    /// # 参数
    /// - `peer_addr`: 对等体地址
    /// - `delay`: 新的延迟时间
    ///
    /// # 返回
    /// 是否成功重置
    pub fn reset_hold_timer(&mut self, peer_addr: &IpAddr, delay: Duration) -> bool {
        if let Some(peer_timers) = self.peer_timers.get(peer_addr)
            && let Some(id) = peer_timers.hold_id
            && let Some(timer) = self.timers.get_mut(&id)
        {
            timer.reset(delay);
            return true;
        }
        false
    }

    /// 停用所有对等体定时器
    ///
    /// # 参数
    /// - `peer_addr`: 对等体地址
    pub fn deactivate_peer_timers(&mut self, peer_addr: &IpAddr) {
        if let Some(peer_timers) = self.peer_timers.get(peer_addr) {
            if let Some(id) = peer_timers.keepalive_id
                && let Some(timer) = self.timers.get_mut(&id)
            {
                timer.deactivate();
            }
            if let Some(id) = peer_timers.hold_id
                && let Some(timer) = self.timers.get_mut(&id)
            {
                timer.deactivate();
            }
            if let Some(id) = peer_timers.connect_retry_id
                && let Some(timer) = self.timers.get_mut(&id)
            {
                timer.deactivate();
            }
        }
    }

    /// 启用 Established 状态下的定时器（Keepalive + Hold）
    ///
    /// # 参数
    /// - `peer_addr`: 对等体地址
    /// - `keepalive_delay`: Keepalive 延迟
    /// - `hold_delay`: Hold 延迟
    pub fn enable_established_timers(&mut self, peer_addr: IpAddr, keepalive_delay: Duration, hold_delay: Duration) {
        // 重置或创建 Keepalive 定时器
        if !self.reset_keepalive_timer(&peer_addr, keepalive_delay) {
            self.add_keepalive_timer(peer_addr, keepalive_delay);
        }

        // 重置或创建 Hold 定时器
        if !self.reset_hold_timer(&peer_addr, hold_delay) {
            self.add_hold_timer(peer_addr, hold_delay);
        }

        // 停用 Connect Retry 定时器
        if let Some(peer_timers) = self.peer_timers.get(&peer_addr)
            && let Some(id) = peer_timers.connect_retry_id
            && let Some(timer) = self.timers.get_mut(&id)
        {
            timer.deactivate();
        }
    }

    /// 获取所有到期的定时器
    ///
    /// # 返回
    /// 到期的定时器列表
    pub fn get_expired_timers(&self) -> Vec<&BgpTimer> {
        self.timers.values()
            .filter(|t| t.is_expired())
            .collect()
    }

    /// 移除对等体的所有定时器
    ///
    /// # 参数
    /// - `peer_addr`: 对等体地址
    pub fn remove_peer_timers(&mut self, peer_addr: &IpAddr) {
        if let Some(peer_timers) = self.peer_timers.remove(peer_addr) {
            if let Some(id) = peer_timers.keepalive_id {
                self.timers.remove(&id);
            }
            if let Some(id) = peer_timers.hold_id {
                self.timers.remove(&id);
            }
            if let Some(id) = peer_timers.connect_retry_id {
                self.timers.remove(&id);
            }
        }
    }

    /// 获取定时器数量
    pub fn len(&self) -> usize {
        self.timers.len()
    }

    /// 是否为空
    pub fn is_empty(&self) -> bool {
        self.timers.is_empty()
    }
}

impl Default for BgpTimerManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::Ipv4Addr;

    #[test]
    fn test_timer_creation() {
        let mut mgr = BgpTimerManager::new();
        let addr = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1));

        let id = mgr.add_keepalive_timer(addr, Duration::from_secs(60));
        assert_eq!(id, 1);
        assert_eq!(mgr.len(), 1);
    }

    #[test]
    fn test_timer_expiry() {
        let mut mgr = BgpTimerManager::new();
        let addr = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1));

        // 添加一个短延迟定时器
        mgr.add_keepalive_timer(addr, Duration::from_millis(10));

        // 等待定时器到期
        std::thread::sleep(Duration::from_millis(20));

        let expired = mgr.get_expired_timers();
        assert_eq!(expired.len(), 1);
    }

    #[test]
    fn test_reset_timer() {
        let mut mgr = BgpTimerManager::new();
        let addr = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1));

        mgr.add_keepalive_timer(addr, Duration::from_millis(10));

        // 等待一点时间
        std::thread::sleep(Duration::from_millis(5));

        // 重置定时器
        mgr.reset_keepalive_timer(&addr, Duration::from_millis(100));

        // 等待最初的到期时间
        std::thread::sleep(Duration::from_millis(10));

        // 定时器应该还没到期
        let expired = mgr.get_expired_timers();
        assert_eq!(expired.len(), 0);
    }

    #[test]
    fn test_remove_peer_timers() {
        let mut mgr = BgpTimerManager::new();
        let addr = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1));

        mgr.add_keepalive_timer(addr, Duration::from_secs(60));
        mgr.add_hold_timer(addr, Duration::from_secs(180));
        mgr.add_connect_retry_timer(addr, Duration::from_secs(60));

        assert_eq!(mgr.len(), 3);

        mgr.remove_peer_timers(&addr);

        assert_eq!(mgr.len(), 0);
    }

    #[test]
    fn test_deactivate_timers() {
        let mut mgr = BgpTimerManager::new();
        let addr = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1));

        mgr.add_keepalive_timer(addr, Duration::from_millis(10));

        mgr.deactivate_peer_timers(&addr);

        // 等待定时器到期
        std::thread::sleep(Duration::from_millis(20));

        let expired = mgr.get_expired_timers();
        // 定时器已被停用，不应该出现在到期列表中
        assert_eq!(expired.len(), 0);
    }
}
