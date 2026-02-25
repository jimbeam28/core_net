// src/protocols/bgp/rib.rs
//
// BGP 路由信息库（RIB）实现

use std::net::IpAddr;
use std::collections::HashMap;
use crate::protocols::bgp::message::IpPrefix;

/// BGP 路由信息库（RIB）
#[derive(Debug, Clone)]
pub struct BgpRib {
    /// 路由条目列表（按前缀索引）
    routes: HashMap<IpPrefix, BgpRoute>,
}

impl Default for BgpRib {
    fn default() -> Self {
        Self::new()
    }
}

impl BgpRib {
    /// 创建新的 RIB
    pub fn new() -> Self {
        Self {
            routes: HashMap::new(),
        }
    }

    /// 添加或更新路由
    pub fn add_or_update(&mut self, route: BgpRoute) {
        let prefix = route.prefix.clone();
        self.routes.insert(prefix, route);
    }

    /// 删除路由
    pub fn remove(&mut self, prefix: &IpPrefix) -> Option<BgpRoute> {
        self.routes.remove(prefix)
    }

    /// 查找路由
    pub fn find(&self, prefix: &IpPrefix) -> Option<&BgpRoute> {
        self.routes.get(prefix)
    }

    /// 最长前缀匹配查找
    pub fn lookup(&self, addr: &IpAddr) -> Option<&BgpRoute> {
        let mut best_match: Option<&BgpRoute> = None;
        let mut best_len = 0;

        for route in self.routes.values() {
            if let IpAddr::V4(route_addr) = route.prefix.prefix
                && let IpAddr::V4(query_addr) = addr
                && self::matches_prefix(*query_addr, route_addr, route.prefix.prefix_len)
                && route.prefix.prefix_len > best_len
            {
                best_match = Some(route);
                best_len = route.prefix.prefix_len;
            }
        }

        best_match
    }

    /// 获取所有路由
    pub fn routes(&self) -> Vec<BgpRoute> {
        self.routes.values().cloned().collect()
    }

    /// 获取路由数量
    pub fn len(&self) -> usize {
        self.routes.len()
    }

    /// 是否为空
    pub fn is_empty(&self) -> bool {
        self.routes.is_empty()
    }

    /// 清空所有路由
    pub fn clear(&mut self) {
        self.routes.clear();
    }
}

/// 检查 IP 地址是否匹配前缀
fn matches_prefix(addr: std::net::Ipv4Addr, prefix_addr: std::net::Ipv4Addr, prefix_len: u8) -> bool {
    let addr_octets = addr.octets();
    let prefix_octets = prefix_addr.octets();

    let full_bytes = (prefix_len / 8) as usize;
    let partial_bits = prefix_len % 8;

    // 比较完整字节
    for i in 0..full_bytes {
        if addr_octets[i] != prefix_octets[i] {
            return false;
        }
    }

    // 比较部分字节
    if partial_bits > 0 && full_bytes < 4 {
        let mask = 0xFF << (8 - partial_bits);
        if (addr_octets[full_bytes] & mask) != (prefix_octets[full_bytes] & mask) {
            return false;
        }
    }

    true
}

/// BGP 路由条目
#[derive(Debug, Clone)]
pub struct BgpRoute {
    /// 网络前缀
    pub prefix: IpPrefix,

    /// 下一跳
    pub next_hop: IpAddr,

    /// 本地优先级（仅 IBGP）
    pub local_pref: Option<u32>,

    /// MED（多出口鉴别器）
    pub med: u32,

    /// AS 路径
    pub as_path: Vec<u32>,

    /// 起源类型（0=IGP, 1=EGP, 2=INCOMPLETE）
    pub origin: u8,

    /// 来自哪个对等体
    pub peer: IpAddr,

    /// 路由是否有效
    pub valid: bool,

    /// 路由年龄（秒）
    pub age: u32,
}

impl BgpRoute {
    /// 创建新的路由条目
    pub fn new(prefix: IpPrefix, next_hop: IpAddr, peer: IpAddr) -> Self {
        Self {
            prefix,
            next_hop,
            local_pref: None,
            med: 0,
            as_path: Vec::new(),
            origin: 0, // IGP
            peer,
            valid: true,
            age: 0,
        }
    }

    /// 获取 AS 路径长度
    pub fn as_path_length(&self) -> usize {
        self.as_path.len()
    }

    /// 计算路由优先级（用于选路）
    ///
    /// 返回值越小优先级越高
    pub fn preference(&self) -> u32 {
        // 简化的选路算法
        // 1. Local Pref（越大越好）
        // 2. AS Path 长度（越短越好）
        // 3. MED（越小越好）

        let local_pref = self.local_pref.unwrap_or(100);
        let as_path_len = self.as_path_length() as u32;

        // 组合优先级：Local Pref * 1000000 - as_path_len * 1000 - med
        // 这样 Local Pref 优先级最高，其次是 AS Path 长度，最后是 MED
        local_pref * 1_000_000 - as_path_len * 1_000 - self.med
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::Ipv4Addr;

    #[test]
    fn test_rib_add_and_find() {
        let mut rib = BgpRib::new();
        let prefix = IpPrefix::new(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 0)), 24);
        let route = BgpRoute::new(
            prefix.clone(),
            IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1)),
            IpAddr::V4(Ipv4Addr::new(10, 0, 0, 2)),
        );

        rib.add_or_update(route);
        assert_eq!(rib.len(), 1);

        let found = rib.find(&prefix);
        assert!(found.is_some());
        assert_eq!(found.unwrap().next_hop, IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1)));
    }

    #[test]
    fn test_rib_remove() {
        let mut rib = BgpRib::new();
        let prefix = IpPrefix::new(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 0)), 24);
        let route = BgpRoute::new(
            prefix.clone(),
            IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1)),
            IpAddr::V4(Ipv4Addr::new(10, 0, 0, 2)),
        );

        rib.add_or_update(route);
        assert_eq!(rib.len(), 1);

        rib.remove(&prefix);
        assert_eq!(rib.len(), 0);
        assert!(rib.find(&prefix).is_none());
    }

    #[test]
    fn test_rib_longest_prefix_match() {
        let mut rib = BgpRib::new();

        // 添加 10.0.0.0/8
        rib.add_or_update(BgpRoute::new(
            IpPrefix::new(IpAddr::V4(Ipv4Addr::new(10, 0, 0, 0)), 8),
            IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1)),
            IpAddr::V4(Ipv4Addr::new(10, 0, 0, 2)),
        ));

        // 添加 10.0.1.0/24
        rib.add_or_update(BgpRoute::new(
            IpPrefix::new(IpAddr::V4(Ipv4Addr::new(10, 0, 1, 0)), 24),
            IpAddr::V4(Ipv4Addr::new(10, 0, 1, 1)),
            IpAddr::V4(Ipv4Addr::new(10, 0, 1, 2)),
        ));

        // 查询 10.0.1.100 应该匹配 /24
        let result = rib.lookup(&IpAddr::V4(Ipv4Addr::new(10, 0, 1, 100)));
        assert!(result.is_some());
        assert_eq!(
            result.unwrap().next_hop,
            IpAddr::V4(Ipv4Addr::new(10, 0, 1, 1))
        );
    }

    #[test]
    fn test_route_preference() {
        let route1 = BgpRoute {
            prefix: IpPrefix::new(IpAddr::V4(Ipv4Addr::new(10, 0, 0, 0)), 24),
            next_hop: IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1)),
            local_pref: Some(200),
            med: 0,
            as_path: vec![100, 200],
            origin: 0,
            peer: IpAddr::V4(Ipv4Addr::new(10, 0, 0, 2)),
            valid: true,
            age: 0,
        };

        let route2 = BgpRoute {
            prefix: IpPrefix::new(IpAddr::V4(Ipv4Addr::new(10, 0, 0, 0)), 24),
            next_hop: IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1)),
            local_pref: Some(100),
            med: 0,
            as_path: vec![100],
            origin: 0,
            peer: IpAddr::V4(Ipv4Addr::new(10, 0, 0, 3)),
            valid: true,
            age: 0,
        };

        // route1 有更高的 Local Pref，应该优先
        assert!(route1.preference() > route2.preference());
    }
}
