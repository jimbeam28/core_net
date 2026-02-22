// src/route/ipv6.rs
//
// IPv6 路由条目定义

use crate::protocols::Ipv6Addr;

/// IPv6 路由条目
///
/// 包含目标前缀、下一跳和出接口信息
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ipv6Route {
    /// 目标前缀
    pub destination: Ipv6Addr,

    /// 前缀长度 (0-128)
    pub prefix_len: u8,

    /// 下一跳地址（None 表示直连网络）
    pub next_hop: Option<Ipv6Addr>,

    /// 出接口名称
    pub interface: String,

    /// 路由优先级（可选）
    pub metric: Option<u32>,
}

impl Ipv6Route {
    /// 创建新的 IPv6 路由条目
    pub fn new(
        destination: Ipv6Addr,
        prefix_len: u8,
        next_hop: Option<Ipv6Addr>,
        interface: String,
    ) -> Self {
        Self {
            destination,
            prefix_len,
            next_hop,
            interface,
            metric: None,
        }
    }

    /// 创建新的 IPv6 路由条目（带优先级）
    pub fn with_metric(
        destination: Ipv6Addr,
        prefix_len: u8,
        next_hop: Option<Ipv6Addr>,
        interface: String,
        metric: u32,
    ) -> Self {
        Self {
            destination,
            prefix_len,
            next_hop,
            interface,
            metric: Some(metric),
        }
    }

    /// 判断是否为默认路由
    ///
    /// 默认路由是 ::/0
    pub fn is_default_route(&self) -> bool {
        self.destination.is_unspecified() && self.prefix_len == 0
    }

    /// 判断目标地址是否匹配此路由
    ///
    /// 比较地址的前 N 位（N 为前缀长度）
    pub fn matches(&self, addr: Ipv6Addr) -> bool {
        if self.prefix_len == 0 {
            // 前缀长度为 0，匹配所有地址
            return true;
        }

        // 按字节比较前缀
        let full_bytes = (self.prefix_len / 8) as usize;
        let remaining_bits = (self.prefix_len % 8) as u32;

        // 比较完整字节
        for i in 0..full_bytes {
            if addr.bytes[i] != self.destination.bytes[i] {
                return false;
            }
        }

        // 如果有剩余位，比较部分字节
        if remaining_bits > 0 && full_bytes < 16 {
            let mask = !(0xFF >> remaining_bits);
            let addr_byte = addr.bytes[full_bytes];
            let dest_byte = self.destination.bytes[full_bytes];
            if (addr_byte & mask) != (dest_byte & mask) {
                return false;
            }
        }

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_default_route() {
        let route = Ipv6Route::new(
            Ipv6Addr::UNSPECIFIED,
            0,
            Some(Ipv6Addr::new(0xfe80, 0, 0, 0, 0, 0, 0, 1)),
            "eth0".to_string(),
        );
        assert!(route.is_default_route());

        let route = Ipv6Route::new(
            Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 0),
            32,
            None,
            "eth0".to_string(),
        );
        assert!(!route.is_default_route());
    }

    #[test]
    fn test_matches() {
        // 测试 /64 前缀匹配
        let route = Ipv6Route::new(
            Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 0),
            32,
            None,
            "eth0".to_string(),
        );

        // 匹配的地址
        assert!(route.matches(Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 1)));
        assert!(route.matches(Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0xffff, 0xffff, 0xffff, 0xffff)));

        // 不匹配的地址
        assert!(!route.matches(Ipv6Addr::new(0x2001, 0xdb9, 0, 0, 0, 0, 0, 1)));
        assert!(!route.matches(Ipv6Addr::new(0xfe80, 0, 0, 0, 0, 0, 0, 1)));
    }

    #[test]
    fn test_matches_partial_byte() {
        // 测试跨字节边界的前缀（比如 /40）
        let route = Ipv6Route::new(
            Ipv6Addr::new(0x2001, 0xdb8, 0x1000, 0, 0, 0, 0, 0),
            40,
            None,
            "eth0".to_string(),
        );

        // /40 意味着前 5 个字节完全匹配，第 6 字节的前 4 位匹配
        // 0x1000 = 0001 0000 0000 0000
        // 前 40 位 = 0001 0000 0000 = 0x100
        // 所以地址应该匹配 2001:db8:10xx::/32

        // 匹配：第 40 位是 0
        assert!(route.matches(Ipv6Addr::new(0x2001, 0xdb8, 0x1000, 0, 0, 0, 0, 1)));

        // 不匹配：第 41 位是 1
        assert!(!route.matches(Ipv6Addr::new(0x2001, 0xdb8, 0x1800, 0, 0, 0, 0, 1)));
    }

    #[test]
    fn test_default_route_matches_anything() {
        let route = Ipv6Route::new(
            Ipv6Addr::UNSPECIFIED,
            0,
            Some(Ipv6Addr::new(0xfe80, 0, 0, 0, 0, 0, 0, 1)),
            "eth0".to_string(),
        );

        // 默认路由匹配任何地址
        assert!(route.matches(Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 1)));
        assert!(route.matches(Ipv6Addr::new(0xfe80, 0, 0, 0, 0, 0, 0, 1)));
        assert!(route.matches(Ipv6Addr::UNSPECIFIED));
    }

    #[test]
    fn test_matches_128_prefix() {
        let route = Ipv6Route::new(
            Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 1),
            128,
            None,
            "eth0".to_string(),
        );

        // 只匹配完全相同的地址
        assert!(route.matches(Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 1)));
        assert!(!route.matches(Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 2)));
    }
}
