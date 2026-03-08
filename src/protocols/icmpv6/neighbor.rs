// src/protocols/icmpv6/neighbor.rs
//
// ICMPv6 邻居发现 (NDP) 相关数据结构（精简版）
// 简化：只保留基础地址映射，无状态管理

use std::collections::HashMap;

use crate::protocols::{Ipv6Addr, MacAddr};

/// 邻居缓存（精简版）
///
/// 只保留 IPv6 到 MAC 地址的基础映射，无状态机
pub struct NeighborCache {
    /// 缓存条目 (IPv6 地址 -> MAC 地址)
    entries: HashMap<Ipv6Addr, MacAddr>,
    /// 最大条目数
    max_entries: usize,
}

impl NeighborCache {
    /// 创建新的邻居缓存
    pub fn new(max_entries: usize) -> Self {
        Self {
            entries: HashMap::new(),
            max_entries,
        }
    }

    /// 查询邻居缓存
    pub fn lookup(&self, addr: &Ipv6Addr) -> Option<MacAddr> {
        self.entries.get(addr).copied()
    }

    /// 添加或更新邻居条目
    pub fn update(&mut self, addr: Ipv6Addr, mac: MacAddr) {
        // 如果缓存已满，删除任意条目
        if self.entries.len() >= self.max_entries
            && !self.entries.contains_key(&addr)
            && let Some(first_key) = self.entries.keys().next().copied()
        {
            self.entries.remove(&first_key);
        }
        self.entries.insert(addr, mac);
    }

    /// 删除条目
    pub fn remove(&mut self, addr: &Ipv6Addr) {
        self.entries.remove(addr);
    }

    /// 清空缓存
    pub fn clear(&mut self) {
        self.entries.clear();
    }

    /// 获取条目数
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// 是否为空
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

impl Default for NeighborCache {
    fn default() -> Self {
        Self::new(128)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_neighbor_cache_basic() {
        let mut cache = NeighborCache::new(10);
        let ip = Ipv6Addr::new(0xfe80, 0, 0, 0, 0, 0, 0, 1);
        let mac = MacAddr::new([0x00, 0x11, 0x22, 0x33, 0x44, 0x55]);

        cache.update(ip, mac);
        assert_eq!(cache.lookup(&ip), Some(mac));
    }

    #[test]
    fn test_neighbor_cache_update() {
        let mut cache = NeighborCache::new(10);
        let ip = Ipv6Addr::new(0xfe80, 0, 0, 0, 0, 0, 0, 1);
        let mac1 = MacAddr::new([0x00, 0x11, 0x22, 0x33, 0x44, 0x55]);
        let mac2 = MacAddr::new([0x00, 0x11, 0x22, 0x33, 0x44, 0x66]);

        cache.update(ip, mac1);
        cache.update(ip, mac2);
        assert_eq!(cache.lookup(&ip), Some(mac2));
    }
}
