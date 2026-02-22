// src/route/ipv4.rs
//
// IPv4 路由条目定义

use crate::common::addr::Ipv4Addr;

/// IPv4 路由条目
///
/// 包含目标网络、子网掩码、网关和出接口信息
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ipv4Route {
    /// 目标网络地址
    pub destination: Ipv4Addr,

    /// 子网掩码
    pub netmask: Ipv4Addr,

    /// 网关地址（None 表示直连网络）
    pub gateway: Option<Ipv4Addr>,

    /// 出接口名称
    pub interface: String,

    /// 路由优先级（管理距离，可选）
    pub metric: Option<u32>,
}

impl Ipv4Route {
    /// 创建新的 IPv4 路由条目
    pub fn new(
        destination: Ipv4Addr,
        netmask: Ipv4Addr,
        gateway: Option<Ipv4Addr>,
        interface: String,
    ) -> Self {
        Self {
            destination,
            netmask,
            gateway,
            interface,
            metric: None,
        }
    }

    /// 创建新的 IPv4 路由条目（带优先级）
    pub fn with_metric(
        destination: Ipv4Addr,
        netmask: Ipv4Addr,
        gateway: Option<Ipv4Addr>,
        interface: String,
        metric: u32,
    ) -> Self {
        Self {
            destination,
            netmask,
            gateway,
            interface,
            metric: Some(metric),
        }
    }

    /// 计算前缀长度
    ///
    /// 从子网掩码计算前缀长度（连续的 1 的位数）
    pub fn prefix_len(&self) -> u8 {
        let mask = self.netmask.to_u32();
        // 转换为网络序（大端序）后计算前导零
        // Ipv4Addr 内部使用大端序存储，所以需要转换后才能正确计算
        32 - mask.to_be().leading_zeros() as u8
    }

    /// 判断是否为默认路由
    ///
    /// 默认路由是 0.0.0.0/0
    pub fn is_default_route(&self) -> bool {
        self.destination.is_unspecified() && self.netmask.is_unspecified()
    }

    /// 判断目标地址是否匹配此路由
    ///
    /// 通过目标地址与子网掩码的 AND 运算结果与目标网络比较来判断
    pub fn matches(&self, addr: Ipv4Addr) -> bool {
        let addr_bits = addr.to_u32();
        let dest_bits = self.destination.to_u32();
        let mask_bits = self.netmask.to_u32();

        (addr_bits & mask_bits) == (dest_bits & mask_bits)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prefix_len() {
        let route = Ipv4Route::new(
            Ipv4Addr::new(192, 168, 0, 0),
            Ipv4Addr::new(255, 255, 0, 0),
            None,
            "eth0".to_string(),
        );
        assert_eq!(route.prefix_len(), 16);

        let route = Ipv4Route::new(
            Ipv4Addr::new(192, 168, 1, 0),
            Ipv4Addr::new(255, 255, 255, 0),
            None,
            "eth0".to_string(),
        );
        assert_eq!(route.prefix_len(), 24);

        let route = Ipv4Route::new(
            Ipv4Addr::new(10, 0, 0, 0),
            Ipv4Addr::new(255, 0, 0, 0),
            None,
            "eth0".to_string(),
        );
        assert_eq!(route.prefix_len(), 8);
    }

    #[test]
    fn test_is_default_route() {
        let route = Ipv4Route::new(
            Ipv4Addr::new(0, 0, 0, 0),
            Ipv4Addr::new(0, 0, 0, 0),
            Some(Ipv4Addr::new(192, 168, 1, 1)),
            "eth0".to_string(),
        );
        assert!(route.is_default_route());

        let route = Ipv4Route::new(
            Ipv4Addr::new(192, 168, 0, 0),
            Ipv4Addr::new(255, 255, 0, 0),
            None,
            "eth0".to_string(),
        );
        assert!(!route.is_default_route());
    }

    #[test]
    fn test_matches() {
        let route = Ipv4Route::new(
            Ipv4Addr::new(192, 168, 0, 0),
            Ipv4Addr::new(255, 255, 0, 0),
            None,
            "eth0".to_string(),
        );

        // 匹配的地址
        assert!(route.matches(Ipv4Addr::new(192, 168, 0, 1)));
        assert!(route.matches(Ipv4Addr::new(192, 168, 1, 100)));
        assert!(route.matches(Ipv4Addr::new(192, 168, 255, 255)));

        // 不匹配的地址
        assert!(!route.matches(Ipv4Addr::new(192, 169, 0, 1)));
        assert!(!route.matches(Ipv4Addr::new(10, 0, 0, 1)));
        assert!(!route.matches(Ipv4Addr::new(172, 16, 0, 1)));
    }

    #[test]
    fn test_default_route_matches_anything() {
        let route = Ipv4Route::new(
            Ipv4Addr::new(0, 0, 0, 0),
            Ipv4Addr::new(0, 0, 0, 0),
            Some(Ipv4Addr::new(192, 168, 1, 1)),
            "eth0".to_string(),
        );

        // 默认路由匹配任何地址
        assert!(route.matches(Ipv4Addr::new(8, 8, 8, 8)));
        assert!(route.matches(Ipv4Addr::new(192, 168, 1, 100)));
        assert!(route.matches(Ipv4Addr::new(10, 0, 0, 1)));
    }
}
