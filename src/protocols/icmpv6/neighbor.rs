// src/protocols/icmpv6/neighbor.rs
//
// ICMPv6 邻居发现 (NDP) 相关数据结构
// RFC 4861: Neighbor Discovery for IPv6

use std::collections::HashMap;
use std::time::Instant;

use crate::protocols::{Ipv6Addr, MacAddr};
use crate::common::CoreError;

use super::types::*;
use super::error::{Icmpv6Error, Icmpv6Result};

// ========== 邻居缓存 ==========

/// 邻居缓存条目
#[derive(Debug, Clone)]
pub struct NeighborCacheEntry {
    /// IPv6 地址
    pub ipv6_addr: Ipv6Addr,
    /// 链路层地址
    pub link_layer_addr: Option<MacAddr>,
    /// 条目状态
    pub state: NeighborCacheState,
    /// 是否为路由器
    pub is_router: bool,
    /// 进入当前状态的时间
    pub state_since: Instant,
    /// 可达时间（毫秒）
    pub reachable_time: Option<u32>,
}

impl NeighborCacheEntry {
    /// 创建新的邻居缓存条目
    pub fn new(
        ipv6_addr: Ipv6Addr,
        link_layer_addr: Option<MacAddr>,
        state: NeighborCacheState,
        is_router: bool,
    ) -> Self {
        NeighborCacheEntry {
            ipv6_addr,
            link_layer_addr,
            state,
            is_router,
            state_since: Instant::now(),
            reachable_time: None,
        }
    }

    /// 检查条目是否过期
    pub fn is_expired(&self) -> bool {
        match self.state {
            NeighborCacheState::Reachable => {
                if let Some(reachable_time) = self.reachable_time {
                    let elapsed = self.state_since.elapsed().as_millis() as u32;
                    elapsed >= reachable_time
                } else {
                    false
                }
            }
            NeighborCacheState::Permanent => false,
            _ => false,
        }
    }

    /// 更新状态
    pub fn update_state(&mut self, new_state: NeighborCacheState) {
        self.state = new_state;
        self.state_since = Instant::now();
    }

    /// 设置链路层地址
    pub fn set_link_layer_addr(&mut self, addr: MacAddr) {
        self.link_layer_addr = Some(addr);
    }

    /// 设置可达时间
    pub fn set_reachable_time(&mut self, time: u32) {
        self.reachable_time = Some(time);
    }
}

/// 邻居缓存
pub struct NeighborCache {
    /// 缓存条目 (IPv6 地址 -> 条目)
    entries: HashMap<Ipv6Addr, NeighborCacheEntry>,
    /// 最大条目数
    max_entries: usize,
    /// 默认可达时间（毫秒）
    default_reachable_time: u32,
}

impl NeighborCache {
    /// 创建新的邻居缓存
    pub fn new(max_entries: usize, default_reachable_time: u32) -> Self {
        Self {
            entries: HashMap::new(),
            max_entries,
            default_reachable_time,
        }
    }

    /// 查询邻居缓存
    pub fn lookup(&self, addr: &Ipv6Addr) -> Option<&NeighborCacheEntry> {
        self.entries.get(addr)
    }

    /// 查询邻居缓存（可变）
    pub fn lookup_mut(&mut self, addr: &Ipv6Addr) -> Option<&mut NeighborCacheEntry> {
        self.entries.get_mut(addr)
    }

    /// 添加或更新邻居条目
    pub fn update(
        &mut self,
        addr: Ipv6Addr,
        link_layer_addr: MacAddr,
        is_router: bool,
        state: NeighborCacheState,
    ) -> Icmpv6Result<()> {
        // 如果缓存已满，删除最旧的 STALE 条目
        if self.entries.len() >= self.max_entries && !self.entries.contains_key(&addr) {
            self.evict_stale();
        }

        let mut entry = NeighborCacheEntry::new(addr, Some(link_layer_addr), state, is_router);
        entry.set_reachable_time(self.default_reachable_time);
        self.entries.insert(addr, entry);

        Ok(())
    }

    /// 标记条目为 INCOMPLETE（开始地址解析）
    pub fn mark_incomplete(&mut self, addr: Ipv6Addr) -> Icmpv6Result<()> {
        if let Some(entry) = self.entries.get_mut(&addr) {
            entry.update_state(NeighborCacheState::Incomplete);
            entry.link_layer_addr = None;
        } else {
            if self.entries.len() >= self.max_entries {
                self.evict_stale();
            }
            let entry = NeighborCacheEntry::new(addr, None, NeighborCacheState::Incomplete, false);
            self.entries.insert(addr, entry);
        }
        Ok(())
    }

    /// 标记条目为 REACHABLE
    pub fn mark_reachable(&mut self, addr: Ipv6Addr, link_layer_addr: MacAddr) -> Icmpv6Result<()> {
        if let Some(entry) = self.entries.get_mut(&addr) {
            entry.set_link_layer_addr(link_layer_addr);
            entry.update_state(NeighborCacheState::Reachable);
            entry.set_reachable_time(self.default_reachable_time);
        } else {
            return Err(Icmpv6Error::NeighborCacheError(format!(
                "邻居条目不存在: {}", addr
            )));
        }
        Ok(())
    }

    /// 标记条目为 STALE
    pub fn mark_stale(&mut self, addr: Ipv6Addr) -> Icmpv6Result<()> {
        if let Some(entry) = self.entries.get_mut(&addr) {
            entry.update_state(NeighborCacheState::Stale);
        }
        Ok(())
    }

    /// 处理可达性超时
    pub fn handle_timeouts(&mut self) {
        let now = Instant::now();
        let mut to_remove = Vec::new();

        for (addr, entry) in &mut self.entries {
            match entry.state {
                NeighborCacheState::Reachable => {
                    if let Some(reachable_time) = entry.reachable_time {
                        let elapsed = now.duration_since(entry.state_since).as_millis() as u32;
                        if elapsed >= reachable_time {
                            entry.update_state(NeighborCacheState::Stale);
                        }
                    }
                }
                NeighborCacheState::Incomplete => {
                    // INCOMPLETE 状态超时后删除
                    let elapsed = now.duration_since(entry.state_since).as_secs();
                    if elapsed > 3 {
                        to_remove.push(*addr);
                    }
                }
                _ => {}
            }
        }

        for addr in to_remove {
            self.entries.remove(&addr);
        }
    }

    /// 淘汰 STALE 条目
    fn evict_stale(&mut self) {
        if let Some(addr) = self.entries
            .iter()
            .filter(|(_, e)| e.state == NeighborCacheState::Stale)
            .min_by_key(|(_, e)| e.state_since)
            .map(|(addr, _)| *addr)
        {
            self.entries.remove(&addr);
        }
    }

    /// 删除条目
    pub fn remove(&mut self, addr: &Ipv6Addr) -> Option<NeighborCacheEntry> {
        self.entries.remove(addr)
    }

    /// 清空缓存
    pub fn clear(&mut self) {
        self.entries.clear();
    }

    /// 获取缓存大小
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// 检查缓存是否为空
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// 获取所有条目
    pub fn entries(&self) -> &HashMap<Ipv6Addr, NeighborCacheEntry> {
        &self.entries
    }
}

impl Default for NeighborCache {
    fn default() -> Self {
        Self::new(256, 30000) // 默认最大256条目，可达时间30秒
    }
}

// ========== 默认路由器列表 ==========

/// 默认路由器条目
#[derive(Debug, Clone)]
pub struct DefaultRouterEntry {
    /// 路由器 IPv6 地址
    pub router_addr: Ipv6Addr,
    /// 路由器链路层地址
    pub link_layer_addr: MacAddr,
    /// 路由器生命周期（秒）
    pub lifetime: u16,
    /// 上次更新时间
    pub last_update: Instant,
}

impl DefaultRouterEntry {
    pub fn new(router_addr: Ipv6Addr, link_layer_addr: MacAddr, lifetime: u16) -> Self {
        DefaultRouterEntry {
            router_addr,
            link_layer_addr,
            lifetime,
            last_update: Instant::now(),
        }
    }

    /// 检查路由器是否过期
    pub fn is_expired(&self) -> bool {
        self.last_update.elapsed().as_secs() >= self.lifetime as u64
    }
}

/// 路由器列表
pub struct RouterList {
    routers: Vec<DefaultRouterEntry>,
}

impl RouterList {
    pub fn new() -> Self {
        Self {
            routers: Vec::new(),
        }
    }

    /// 添加或更新路由器
    pub fn add_or_update(&mut self, router: DefaultRouterEntry) {
        if let Some(entry) = self.routers.iter_mut().find(|r| r.router_addr == router.router_addr) {
            *entry = router;
        } else {
            self.routers.push(router);
        }
    }

    /// 移除过期的路由器
    pub fn remove_expired(&mut self) {
        let now = Instant::now();
        self.routers.retain(|r| {
            now.duration_since(r.last_update).as_secs() < r.lifetime as u64
        });
    }

    /// 获取最佳路由器
    pub fn get_best_router(&self) -> Option<&DefaultRouterEntry> {
        self.routers.first()
    }

    /// 获取所有路由器
    pub fn routers(&self) -> &[DefaultRouterEntry] {
        &self.routers
    }

    /// 删除路由器
    pub fn remove(&mut self, addr: &Ipv6Addr) {
        self.routers.retain(|r| &r.router_addr != addr);
    }

    /// 清空列表
    pub fn clear(&mut self) {
        self.routers.clear();
    }
}

impl Default for RouterList {
    fn default() -> Self {
        Self::new()
    }
}

// ========== 网络前缀列表 ==========

/// 网络前缀条目
#[derive(Debug, Clone)]
pub struct PrefixEntry {
    /// 前缀
    pub prefix: Ipv6Addr,
    /// 前缀长度
    pub prefix_length: u8,
    /// 有效生命周期（秒）
    pub valid_lifetime: u32,
    /// 优先生命周期（秒）
    pub preferred_lifetime: u32,
    /// 上次更新时间
    pub last_update: Instant,
}

impl PrefixEntry {
    pub fn new(
        prefix: Ipv6Addr,
        prefix_length: u8,
        valid_lifetime: u32,
        preferred_lifetime: u32,
    ) -> Self {
        PrefixEntry {
            prefix,
            prefix_length,
            valid_lifetime,
            preferred_lifetime,
            last_update: Instant::now(),
        }
    }

    /// 检查前缀是否过期
    pub fn is_expired(&self) -> bool {
        self.last_update.elapsed().as_secs() >= self.valid_lifetime as u64
    }

    /// 检查前缀是否已废弃
    pub fn is_deprecated(&self) -> bool {
        self.last_update.elapsed().as_secs() >= self.preferred_lifetime as u64
    }
}

/// 前缀列表
pub struct PrefixList {
    prefixes: Vec<PrefixEntry>,
}

impl PrefixList {
    pub fn new() -> Self {
        Self {
            prefixes: Vec::new(),
        }
    }

    /// 添加或更新前缀
    pub fn add_or_update(&mut self, prefix: PrefixEntry) {
        if let Some(entry) = self.prefixes.iter_mut().find(|p| {
            p.prefix == prefix.prefix && p.prefix_length == prefix.prefix_length
        }) {
            *entry = prefix;
        } else {
            self.prefixes.push(prefix);
        }
    }

    /// 移除过期的前缀
    pub fn remove_expired(&mut self) {
        let now = Instant::now();
        self.prefixes.retain(|p| {
            now.duration_since(p.last_update).as_secs() < p.valid_lifetime as u64
        });
    }

    /// 获取所有前缀
    pub fn prefixes(&self) -> &[PrefixEntry] {
        &self.prefixes
    }

    /// 删除前缀
    pub fn remove(&mut self, prefix: &Ipv6Addr, prefix_length: u8) {
        self.prefixes.retain(|p| {
            &p.prefix != prefix || p.prefix_length != prefix_length
        });
    }

    /// 清空列表
    pub fn clear(&mut self) {
        self.prefixes.clear();
    }
}

impl Default for PrefixList {
    fn default() -> Self {
        Self::new()
    }
}

// ========== PMTU 缓存 ==========

/// 路径 MTU 缓存条目
#[derive(Debug, Clone)]
pub struct PmtuEntry {
    /// 目标地址
    pub dest_addr: Ipv6Addr,
    /// 路径 MTU
    pub pmtu: u32,
    /// 上次更新时间
    pub last_update: Instant,
}

impl PmtuEntry {
    pub fn new(dest_addr: Ipv6Addr, pmtu: u32) -> Self {
        PmtuEntry {
            dest_addr,
            pmtu,
            last_update: Instant::now(),
        }
    }
}

/// 路径 MTU 缓存
pub struct PmtuCache {
    entries: HashMap<Ipv6Addr, PmtuEntry>,
    /// 缓存超时时间（分钟）
    timeout_minutes: u32,
}

impl PmtuCache {
    pub fn new(timeout_minutes: u32) -> Self {
        Self {
            entries: HashMap::new(),
            timeout_minutes,
        }
    }

    /// 查询 PMTU
    pub fn lookup(&self, dest: &Ipv6Addr) -> Option<u32> {
        self.entries.get(dest).map(|e| e.pmtu)
    }

    /// 更新 PMTU
    pub fn update(&mut self, dest: Ipv6Addr, pmtu: u32) {
        let entry = PmtuEntry::new(dest, pmtu);
        self.entries.insert(dest, entry);
    }

    /// 移除过期的条目
    pub fn remove_expired(&mut self) {
        let timeout = std::time::Duration::from_secs(self.timeout_minutes as u64 * 60);
        self.entries.retain(|_, e| e.last_update.elapsed() < timeout);
    }

    /// 删除条目
    pub fn remove(&mut self, dest: &Ipv6Addr) {
        self.entries.remove(dest);
    }

    /// 清空缓存
    pub fn clear(&mut self) {
        self.entries.clear();
    }
}

impl Default for PmtuCache {
    fn default() -> Self {
        Self::new(10) // 默认10分钟超时
    }
}

// ========== 待处理 Echo 请求 ==========

/// 待处理的 Echo 请求
#[derive(Debug, Clone)]
pub struct PendingEcho {
    /// 标识符
    pub identifier: u16,
    /// 序列号
    pub sequence: u16,
    /// 发送时间
    pub send_time: Instant,
    /// 目标地址
    pub dest_addr: Ipv6Addr,
}

impl PendingEcho {
    pub fn new(identifier: u16, sequence: u16, dest_addr: Ipv6Addr) -> Self {
        PendingEcho {
            identifier,
            sequence,
            send_time: Instant::now(),
            dest_addr,
        }
    }

    /// 计算往返时间
    pub fn rtt_ms(&self) -> u64 {
        self.send_time.elapsed().as_millis() as u64
    }

    /// 检查是否超时
    pub fn is_timeout(&self, timeout_ms: u64) -> bool {
        self.send_time.elapsed().as_millis() as u64 > timeout_ms
    }
}

/// Echo 请求管理器
pub struct EchoManager {
    /// 待处理的 Echo 请求 (identifier, sequence) -> PendingEcho
    pending_echoes: HashMap<(u16, u16), PendingEcho>,
    /// 最大待处理数量
    max_pending: usize,
    /// 默认超时时间（毫秒）
    default_timeout: u32,
}

impl EchoManager {
    pub fn new(max_pending: usize, default_timeout: u32) -> Self {
        Self {
            pending_echoes: HashMap::new(),
            max_pending,
            default_timeout,
        }
    }

    /// 注册 Echo 请求
    pub fn register(&mut self, identifier: u16, sequence: u16, dest_addr: Ipv6Addr) -> Icmpv6Result<()> {
        let key = (identifier, sequence);

        // 清理超时的请求
        self.cleanup_timeouts();

        // 检查是否超过最大数量
        if self.pending_echoes.len() >= self.max_pending {
            return Err(Icmpv6Error::NeighborCacheError(
                "待处理Echo请求数量超过限制".to_string()
            ));
        }

        let pending = PendingEcho::new(identifier, sequence, dest_addr);
        self.pending_echoes.insert(key, pending);

        Ok(())
    }

    /// 匹配 Echo 响应
    pub fn match_reply(&mut self, identifier: u16, sequence: u16) -> Option<PendingEcho> {
        let key = (identifier, sequence);
        self.pending_echoes.remove(&key)
    }

    /// 清理超时的请求
    pub fn cleanup_timeouts(&mut self) {
        let timeout = self.default_timeout as u64;
        self.pending_echoes.retain(|_, pending| !pending.is_timeout(timeout));
    }

    /// 获取待处理数量
    pub fn pending_count(&self) -> usize {
        self.pending_echoes.len()
    }

    /// 清空所有待处理请求
    pub fn clear(&mut self) {
        self.pending_echoes.clear();
    }
}

impl Default for EchoManager {
    fn default() -> Self {
        Self::new(100, 1000) // 默认最大100个，超时1秒
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_neighbor_cache() {
        let mut cache = NeighborCache::new(10, 30000);
        let addr = Ipv6Addr::new(0xfe80, 0, 0, 0, 0, 0, 0, 1);
        let mac = MacAddr::new([0x00, 0x11, 0x22, 0x33, 0x44, 0x55]);

        cache.update(addr, mac, false, NeighborCacheState::Reachable).unwrap();
        assert_eq!(cache.len(), 1);

        let entry = cache.lookup(&addr).unwrap();
        assert_eq!(entry.ipv6_addr, addr);
        assert_eq!(entry.link_layer_addr, Some(mac));
        assert_eq!(entry.state, NeighborCacheState::Reachable);
    }

    #[test]
    fn test_neighbor_cache_state_transition() {
        let mut cache = NeighborCache::new(10, 30000);
        let addr = Ipv6Addr::new(0xfe80, 0, 0, 0, 0, 0, 0, 1);
        let mac = MacAddr::new([0x00, 0x11, 0x22, 0x33, 0x44, 0x55]);

        cache.mark_incomplete(addr).unwrap();
        let entry = cache.lookup(&addr).unwrap();
        assert_eq!(entry.state, NeighborCacheState::Incomplete);
        assert!(entry.link_layer_addr.is_none());

        cache.mark_reachable(addr, mac).unwrap();
        let entry = cache.lookup(&addr).unwrap();
        assert_eq!(entry.state, NeighborCacheState::Reachable);
        assert_eq!(entry.link_layer_addr, Some(mac));
    }

    #[test]
    fn test_router_list() {
        let mut list = RouterList::new();
        let addr = Ipv6Addr::new(0xfe80, 0, 0, 0, 0, 0, 0, 1);
        let mac = MacAddr::new([0x00, 0x11, 0x22, 0x33, 0x44, 0x55]);

        let router = DefaultRouterEntry::new(addr, mac, 1800);
        list.add_or_update(router);

        assert_eq!(list.routers().len(), 1);
        let best = list.get_best_router().unwrap();
        assert_eq!(best.router_addr, addr);
    }

    #[test]
    fn test_prefix_list() {
        let mut list = PrefixList::new();
        let prefix = Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 0);

        let entry = PrefixEntry::new(prefix, 64, 2592000, 604800);
        list.add_or_update(entry);

        assert_eq!(list.prefixes().len(), 1);
        let p = &list.prefixes()[0];
        assert_eq!(p.prefix, prefix);
        assert_eq!(p.prefix_length, 64);
    }

    #[test]
    fn test_echo_manager() {
        let mut manager = EchoManager::new(10, 1000);
        let addr = Ipv6Addr::new(0xfe80, 0, 0, 0, 0, 0, 0, 1);

        manager.register(1234, 1, addr).unwrap();
        assert_eq!(manager.pending_count(), 1);

        let pending = manager.match_reply(1234, 1).unwrap();
        assert_eq!(pending.identifier, 1234);
        assert_eq!(pending.sequence, 1);
        assert_eq!(pending.dest_addr, addr);

        assert_eq!(manager.pending_count(), 0);
    }

    #[test]
    fn test_pending_echo_rtt() {
        let pending = PendingEcho::new(1234, 1, Ipv6Addr::new(0xfe80, 0, 0, 0, 0, 0, 0, 1));
        std::thread::sleep(std::time::Duration::from_millis(10));
        let rtt = pending.rtt_ms();
        assert!(rtt >= 10);
    }
}
