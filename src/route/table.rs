// src/route/table.rs
//
// 路由表实现
// 管理 IPv4 和 IPv6 路由条目，提供路由查找功能

use crate::common::addr::{IpAddr, Ipv4Addr};
use crate::protocols::Ipv6Addr;
use crate::route::ipv4::Ipv4Route;
use crate::route::ipv6::Ipv6Route;
use crate::route::RouteError;

/// 路由查找结果
///
/// 包含查找到的路由信息：下一跳和出接口
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RouteLookup {
    /// 下一跳地址（None 表示直连）
    pub next_hop: Option<IpAddr>,

    /// 出接口名称
    pub interface: String,

    /// 路由优先级
    pub metric: u32,
}

/// 路由表
///
/// 管理 IPv4 和 IPv6 路由条目，提供路由查找功能
#[derive(Debug)]
pub struct RouteTable {
    /// IPv4 路由列表
    ipv4_routes: Vec<Ipv4Route>,

    /// IPv6 路由列表
    ipv6_routes: Vec<Ipv6Route>,
}

impl RouteTable {
    /// 创建新的空路由表
    pub fn new() -> Self {
        Self {
            ipv4_routes: Vec::new(),
            ipv6_routes: Vec::new(),
        }
    }

    // ==================== IPv4 路由管理 ====================

    /// 添加 IPv4 路由
    ///
    /// # 参数
    ///
    /// - `route`: 要添加的路由条目
    ///
    /// # 返回
    ///
    /// - `Ok(())`: 路由添加成功
    /// - `Err(RouteError)`: 添加失败（如重复路由）
    pub fn add_ipv4_route(&mut self, route: Ipv4Route) -> Result<(), RouteError> {
        // 检查是否已存在相同的路由（目标网络和子网掩码相同）
        for existing in &self.ipv4_routes {
            if existing.destination == route.destination && existing.netmask == route.netmask {
                return Err(RouteError::RouteAlreadyExists {
                    destination: format!("{}/{}", route.destination, route.prefix_len()),
                });
            }
        }

        self.ipv4_routes.push(route);
        Ok(())
    }

    /// 删除 IPv4 路由
    ///
    /// # 参数
    ///
    /// - `destination`: 目标网络地址
    /// - `netmask`: 子网掩码
    ///
    /// # 返回
    ///
    /// - `Ok(())`: 路由删除成功
    /// - `Err(RouteError)`: 删除失败（如路由不存在）
    pub fn remove_ipv4_route(
        &mut self,
        destination: Ipv4Addr,
        netmask: Ipv4Addr,
    ) -> Result<(), RouteError> {
        let original_len = self.ipv4_routes.len();

        self.ipv4_routes.retain(|route| {
            !(route.destination == destination && route.netmask == netmask)
        });

        if self.ipv4_routes.len() == original_len {
            Err(RouteError::RouteNotFound {
                destination: format!("{}/{}", destination, Self::calc_prefix_len(netmask)),
            })
        } else {
            Ok(())
        }
    }

    /// 查找 IPv4 路由（最长前缀匹配）
    ///
    /// 根据目标地址查找最佳匹配路由，按照最长前缀匹配原则。
    ///
    /// # 参数
    ///
    /// - `dest`: 目标 IP 地址
    ///
    /// # 返回
    ///
    /// - `Some(RouteLookup)`: 找到匹配路由，包含下一跳和出接口
    /// - `None`: 没有找到匹配路由
    ///
    /// # 查找算法
    ///
    /// 1. 遍历所有 IPv4 路由条目
    /// 2. 筛选出与目标地址匹配的条目（目标地址 & 子网掩码 == 目标网络）
    /// 3. 在匹配条目中选择前缀长度最长的
    /// 4. 返回路由信息
    pub fn lookup_ipv4(&self, dest: Ipv4Addr) -> Option<RouteLookup> {
        self.ipv4_routes
            .iter()
            .filter(|route| route.matches(dest))
            .max_by_key(|route| route.prefix_len())
            .map(|route| RouteLookup {
                next_hop: route.gateway.map(IpAddr::V4),
                interface: route.interface.clone(),
                metric: route.metric.unwrap_or(0),
            })
    }

    /// 获取所有 IPv4 路由
    pub fn ipv4_routes(&self) -> &[Ipv4Route] {
        &self.ipv4_routes
    }

    // ==================== IPv6 路由管理 ====================

    /// 添加 IPv6 路由
    ///
    /// # 参数
    ///
    /// - `route`: 要添加的路由条目
    ///
    /// # 返回
    ///
    /// - `Ok(())`: 路由添加成功
    /// - `Err(RouteError)`: 添加失败（如重复路由）
    pub fn add_ipv6_route(&mut self, route: Ipv6Route) -> Result<(), RouteError> {
        // 验证前缀长度
        if route.prefix_len > 128 {
            return Err(RouteError::InvalidPrefixLength {
                prefix_len: route.prefix_len,
            });
        }

        // 检查是否已存在相同的路由（目标前缀和前缀长度相同）
        for existing in &self.ipv6_routes {
            if existing.destination == route.destination && existing.prefix_len == route.prefix_len {
                return Err(RouteError::RouteAlreadyExists {
                    destination: format!("{}/{}", existing.destination, existing.prefix_len),
                });
            }
        }

        self.ipv6_routes.push(route);
        Ok(())
    }

    /// 删除 IPv6 路由
    ///
    /// # 参数
    ///
    /// - `destination`: 目标前缀
    /// - `prefix_len`: 前缀长度
    ///
    /// # 返回
    ///
    /// - `Ok(())`: 路由删除成功
    /// - `Err(RouteError)`: 删除失败（如路由不存在）
    pub fn remove_ipv6_route(
        &mut self,
        destination: Ipv6Addr,
        prefix_len: u8,
    ) -> Result<(), RouteError> {
        let original_len = self.ipv6_routes.len();

        self.ipv6_routes.retain(|route| {
            !(route.destination == destination && route.prefix_len == prefix_len)
        });

        if self.ipv6_routes.len() == original_len {
            Err(RouteError::RouteNotFound {
                destination: format!("{}/{}", destination, prefix_len),
            })
        } else {
            Ok(())
        }
    }

    /// 查找 IPv6 路由（最长前缀匹配）
    ///
    /// 根据目标地址查找最佳匹配路由，按照最长前缀匹配原则。
    ///
    /// # 参数
    ///
    /// - `dest`: 目标 IPv6 地址
    ///
    /// # 返回
    ///
    /// - `Some(RouteLookup)`: 找到匹配路由，包含下一跳和出接口
    /// - `None`: 没有找到匹配路由
    ///
    /// # 查找算法
    ///
    /// 1. 遍历所有 IPv6 路由条目
    /// 2. 筛选出与目标地址匹配的条目（前缀匹配）
    /// 3. 在匹配条目中选择前缀长度最长的
    /// 4. 返回路由信息
    pub fn lookup_ipv6(&self, dest: Ipv6Addr) -> Option<RouteLookup> {
        self.ipv6_routes
            .iter()
            .filter(|route| route.matches(dest))
            .max_by_key(|route| route.prefix_len)
            .map(|route| RouteLookup {
                next_hop: route.next_hop.map(IpAddr::V6),
                interface: route.interface.clone(),
                metric: route.metric.unwrap_or(0),
            })
    }

    /// 获取所有 IPv6 路由
    pub fn ipv6_routes(&self) -> &[Ipv6Route] {
        &self.ipv6_routes
    }

    // ==================== 通用方法 ====================

    /// 清空路由表
    pub fn clear(&mut self) {
        self.ipv4_routes.clear();
        self.ipv6_routes.clear();
    }

    /// 获取路由总数
    pub fn len(&self) -> usize {
        self.ipv4_routes.len() + self.ipv6_routes.len()
    }

    /// 路由表是否为空
    pub fn is_empty(&self) -> bool {
        self.ipv4_routes.is_empty() && self.ipv6_routes.is_empty()
    }

    // ==================== 辅助方法 ====================

    /// 计算子网掩码的前缀长度
    fn calc_prefix_len(netmask: Ipv4Addr) -> u8 {
        32 - netmask.to_u32().leading_zeros() as u8
    }
}

impl Default for RouteTable {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ==================== IPv4 路由测试 ====================

    #[test]
    fn test_add_ipv4_route() {
        let mut table = RouteTable::new();

        let route = Ipv4Route::new(
            Ipv4Addr::new(192, 168, 0, 0),
            Ipv4Addr::new(255, 255, 0, 0),
            Some(Ipv4Addr::new(192, 168, 1, 1)),
            "eth0".to_string(),
        );

        assert!(table.add_ipv4_route(route.clone()).is_ok());
        assert_eq!(table.ipv4_routes().len(), 1);
    }

    #[test]
    fn test_add_duplicate_ipv4_route() {
        let mut table = RouteTable::new();

        let route = Ipv4Route::new(
            Ipv4Addr::new(192, 168, 0, 0),
            Ipv4Addr::new(255, 255, 0, 0),
            Some(Ipv4Addr::new(192, 168, 1, 1)),
            "eth0".to_string(),
        );

        assert!(table.add_ipv4_route(route.clone()).is_ok());
        assert!(table.add_ipv4_route(route).is_err());
    }

    #[test]
    fn test_remove_ipv4_route() {
        let mut table = RouteTable::new();

        let route = Ipv4Route::new(
            Ipv4Addr::new(192, 168, 0, 0),
            Ipv4Addr::new(255, 255, 0, 0),
            Some(Ipv4Addr::new(192, 168, 1, 1)),
            "eth0".to_string(),
        );

        table.add_ipv4_route(route).unwrap();
        assert_eq!(table.ipv4_routes().len(), 1);

        assert!(table
            .remove_ipv4_route(Ipv4Addr::new(192, 168, 0, 0), Ipv4Addr::new(255, 255, 0, 0))
            .is_ok());
        assert_eq!(table.ipv4_routes().len(), 0);
    }

    #[test]
    fn test_remove_nonexistent_ipv4_route() {
        let mut table = RouteTable::new();

        assert!(table
            .remove_ipv4_route(Ipv4Addr::new(192, 168, 0, 0), Ipv4Addr::new(255, 255, 0, 0))
            .is_err());
    }

    #[test]
    fn test_lookup_ipv4_exact_match() {
        let mut table = RouteTable::new();

        table
            .add_ipv4_route(Ipv4Route::new(
                Ipv4Addr::new(192, 168, 0, 0),
                Ipv4Addr::new(255, 255, 0, 0),
                Some(Ipv4Addr::new(192, 168, 1, 1)),
                "eth0".to_string(),
            ))
            .unwrap();

        let result = table.lookup_ipv4(Ipv4Addr::new(192, 168, 1, 100));
        assert!(result.is_some());
        let lookup = result.unwrap();
        assert_eq!(
            lookup.next_hop,
            Some(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1)))
        );
        assert_eq!(lookup.interface, "eth0");
    }

    #[test]
    fn test_lookup_ipv4_no_match() {
        let mut table = RouteTable::new();

        table
            .add_ipv4_route(Ipv4Route::new(
                Ipv4Addr::new(192, 168, 0, 0),
                Ipv4Addr::new(255, 255, 0, 0),
                None,
                "eth0".to_string(),
            ))
            .unwrap();

        let result = table.lookup_ipv4(Ipv4Addr::new(10, 0, 0, 1));
        assert!(result.is_none());
    }

    #[test]
    fn test_longest_prefix_match() {
        let mut table = RouteTable::new();

        // 添加两条路由：/16 和 /24
        table
            .add_ipv4_route(Ipv4Route::new(
                Ipv4Addr::new(192, 168, 0, 0),
                Ipv4Addr::new(255, 255, 0, 0),
                Some(Ipv4Addr::new(192, 168, 1, 1)),
                "eth0".to_string(),
            ))
            .unwrap();

        table
            .add_ipv4_route(Ipv4Route::new(
                Ipv4Addr::new(192, 168, 1, 0),
                Ipv4Addr::new(255, 255, 255, 0),
                Some(Ipv4Addr::new(192, 168, 1, 254)),
                "eth0".to_string(),
            ))
            .unwrap();

        // 测试地址 192.168.1.100，应该选择 /24 路由
        let result = table.lookup_ipv4(Ipv4Addr::new(192, 168, 1, 100));
        assert!(result.is_some());
        let lookup = result.unwrap();
        assert_eq!(
            lookup.next_hop,
            Some(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 254)))
        );
    }

    #[test]
    fn test_default_route() {
        let mut table = RouteTable::new();

        // 添加默认路由
        table
            .add_ipv4_route(Ipv4Route::new(
                Ipv4Addr::new(0, 0, 0, 0),
                Ipv4Addr::new(0, 0, 0, 0),
                Some(Ipv4Addr::new(192, 168, 1, 1)),
                "eth0".to_string(),
            ))
            .unwrap();

        // 任意地址都应该匹配默认路由
        let result = table.lookup_ipv4(Ipv4Addr::new(8, 8, 8, 8));
        assert!(result.is_some());
        let lookup = result.unwrap();
        assert_eq!(
            lookup.next_hop,
            Some(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1)))
        );
    }

    #[test]
    fn test_specific_route_preferred_over_default() {
        let mut table = RouteTable::new();

        // 添加具体路由
        table
            .add_ipv4_route(Ipv4Route::new(
                Ipv4Addr::new(192, 168, 0, 0),
                Ipv4Addr::new(255, 255, 0, 0),
                None,
                "eth0".to_string(),
            ))
            .unwrap();

        // 添加默认路由
        table
            .add_ipv4_route(Ipv4Route::new(
                Ipv4Addr::new(0, 0, 0, 0),
                Ipv4Addr::new(0, 0, 0, 0),
                Some(Ipv4Addr::new(192, 168, 1, 1)),
                "eth1".to_string(),
            ))
            .unwrap();

        // 匹配具体路由的地址应该选择具体路由（更长前缀）
        let result = table.lookup_ipv4(Ipv4Addr::new(192, 168, 1, 100));
        assert!(result.is_some());
        let lookup = result.unwrap();
        assert_eq!(lookup.interface, "eth0");
        assert!(lookup.next_hop.is_none()); // 直连网络
    }

    // ==================== IPv6 路由测试 ====================

    #[test]
    fn test_add_ipv6_route() {
        let mut table = RouteTable::new();

        let route = Ipv6Route::new(
            Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 0),
            32,
            None,
            "eth0".to_string(),
        );

        assert!(table.add_ipv6_route(route.clone()).is_ok());
        assert_eq!(table.ipv6_routes().len(), 1);
    }

    #[test]
    fn test_add_invalid_prefix_length() {
        let mut table = RouteTable::new();

        let route = Ipv6Route::new(
            Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 0),
            200, // 无效：超过 128
            None,
            "eth0".to_string(),
        );

        assert!(table.add_ipv6_route(route).is_err());
    }

    #[test]
    fn test_add_duplicate_ipv6_route() {
        let mut table = RouteTable::new();

        let route = Ipv6Route::new(
            Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 0),
            32,
            None,
            "eth0".to_string(),
        );

        assert!(table.add_ipv6_route(route.clone()).is_ok());
        assert!(table.add_ipv6_route(route).is_err());
    }

    #[test]
    fn test_lookup_ipv6() {
        let mut table = RouteTable::new();

        table
            .add_ipv6_route(Ipv6Route::new(
                Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 0),
                32,
                Some(Ipv6Addr::new(0xfe80, 0, 0, 0, 0, 0, 0, 1)),
                "eth0".to_string(),
            ))
            .unwrap();

        let result = table.lookup_ipv6(Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 1));
        assert!(result.is_some());
        let lookup = result.unwrap();
        assert_eq!(
            lookup.next_hop,
            Some(IpAddr::V6(Ipv6Addr::new(0xfe80, 0, 0, 0, 0, 0, 0, 1)))
        );
        assert_eq!(lookup.interface, "eth0");
    }

    #[test]
    fn test_lookup_ipv6_longest_prefix_match() {
        let mut table = RouteTable::new();

        // 添加 /32 路由
        table
            .add_ipv6_route(Ipv6Route::new(
                Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 0),
                32,
                None,
                "eth0".to_string(),
            ))
            .unwrap();

        // 添加 /48 路由
        table
            .add_ipv6_route(Ipv6Route::new(
                Ipv6Addr::new(0x2001, 0xdb8, 0x1, 0, 0, 0, 0, 0),
                48,
                None,
                "eth1".to_string(),
            ))
            .unwrap();

        // 测试地址匹配 /48 路由
        let result = table.lookup_ipv6(Ipv6Addr::new(0x2001, 0xdb8, 0x1, 0, 0, 0, 0, 1));
        assert!(result.is_some());
        let lookup = result.unwrap();
        assert_eq!(lookup.interface, "eth1");
    }

    // ==================== 通用方法测试 ====================

    #[test]
    fn test_clear() {
        let mut table = RouteTable::new();

        table
            .add_ipv4_route(Ipv4Route::new(
                Ipv4Addr::new(192, 168, 0, 0),
                Ipv4Addr::new(255, 255, 0, 0),
                None,
                "eth0".to_string(),
            ))
            .unwrap();

        table
            .add_ipv6_route(Ipv6Route::new(
                Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 0),
                32,
                None,
                "eth0".to_string(),
            ))
            .unwrap();

        assert_eq!(table.len(), 2);

        table.clear();
        assert_eq!(table.len(), 0);
        assert!(table.is_empty());
    }

    #[test]
    fn test_default() {
        let table = RouteTable::default();
        assert!(table.is_empty());
        assert_eq!(table.len(), 0);
    }
}
