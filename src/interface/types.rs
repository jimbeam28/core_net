use std::fmt;

// 从 common 模块导入地址类型
pub use crate::common::{MacAddr, Ipv4Addr};

/// 接口错误类型
#[derive(Debug)]
pub enum InterfaceError {
    /// 接口名称重复
    DuplicateName(String),

    /// 接口未找到
    InterfaceNotFound,

    /// 配置文件读取失败
    ConfigReadFailed(String),

    /// 配置文件解析失败
    ConfigParseFailed(String),

    /// 配置文件写入失败
    ConfigWriteFailed(String),

    /// MAC地址格式无效
    InvalidMacAddr(String),

    /// IP地址格式无效
    InvalidIpAddr(String),

    /// MTU值无效
    InvalidMtu(u16),

    /// 配置文件格式错误
    InvalidFormat(String),

    /// 互斥锁锁定失败
    MutexLockFailed(String),
}

impl fmt::Display for InterfaceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            InterfaceError::DuplicateName(name) => {
                write!(f, "接口名称已存在: {}", name)
            }
            InterfaceError::InterfaceNotFound => {
                write!(f, "接口未找到")
            }
            InterfaceError::ConfigReadFailed(msg) => {
                write!(f, "配置文件读取失败: {}", msg)
            }
            InterfaceError::ConfigParseFailed(msg) => {
                write!(f, "配置文件解析失败: {}", msg)
            }
            InterfaceError::ConfigWriteFailed(msg) => {
                write!(f, "配置文件写入失败: {}", msg)
            }
            InterfaceError::InvalidMacAddr(addr) => {
                write!(f, "无效的MAC地址格式: {}", addr)
            }
            InterfaceError::InvalidIpAddr(addr) => {
                write!(f, "无效的IP地址格式: {}", addr)
            }
            InterfaceError::InvalidMtu(mtu) => {
                write!(f, "无效的MTU值: {}", mtu)
            }
            InterfaceError::InvalidFormat(msg) => {
                write!(f, "配置文件格式错误: {}", msg)
            }
            InterfaceError::MutexLockFailed(msg) => {
                write!(f, "互斥锁锁定失败: {}", msg)
            }
        }
    }
}

impl std::error::Error for InterfaceError {}

/// 网络接口状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InterfaceState {
    /// 接口已启用，可以收发数据
    Up,
    /// 接口已禁用
    Down,
    /// 接口处于测试模式
    Testing,
    /// 接口发生错误
    Error,
}

/// 接口类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InterfaceType {
    /// 以太网接口
    Ethernet,
    /// 本地回环接口
    Loopback,
    /// 虚拟接口
    Virtual,
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========== MacAddr 测试 ==========

    #[test]
    fn test_mac_addr_creation() {
        // 测试通过字节数组创建
        let mac = MacAddr::new([0x00, 0x11, 0x22, 0x33, 0x44, 0x55]);
        assert_eq!(mac.bytes, [0x00, 0x11, 0x22, 0x33, 0x44, 0x55]);
    }

    #[test]
    fn test_mac_addr_parse() {
        // 测试解析正确的 MAC 地址
        let mac: MacAddr = "00:11:22:33:44:55".parse().unwrap();
        assert_eq!(mac.bytes, [0x00, 0x11, 0x22, 0x33, 0x44, 0x55]);

        // 测试小写十六进制
        let mac: MacAddr = "aa:bb:cc:dd:ee:ff".parse().unwrap();
        assert_eq!(mac.bytes, [0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0xff]);
    }

    #[test]
    fn test_mac_addr_parse_invalid() {
        // 测试解析无效的 MAC 地址
        let result: Result<MacAddr, _> = "invalid".parse();
        assert!(result.is_err());

        let result: Result<MacAddr, _> = "00:11:22:33:44".parse();
        assert!(result.is_err());

        let result: Result<MacAddr, _> = "00:11:22:33:44:55:66".parse();
        assert!(result.is_err());
    }

    #[test]
    fn test_mac_addr_broadcast() {
        let mac = MacAddr::broadcast();
        assert!(mac.is_broadcast());
        assert_eq!(mac.bytes, [0xff, 0xff, 0xff, 0xff, 0xff, 0xff]);
    }

    #[test]
    fn test_mac_addr_zero() {
        let mac = MacAddr::zero();
        assert!(mac.is_zero());
        assert_eq!(mac.bytes, [0x00, 0x00, 0x00, 0x00, 0x00, 0x00]);
    }

    #[test]
    fn test_mac_addr_multicast() {
        // 单播地址（最低位为 0）
        let unicast = MacAddr::new([0x00, 0x11, 0x22, 0x33, 0x44, 0x55]);
        assert!(!unicast.is_multicast());

        // 多播地址（最低位为 1）
        let multicast = MacAddr::new([0x01, 0x00, 0x5e, 0x00, 0x00, 0x01]);
        assert!(multicast.is_multicast());

        // 广播地址也是多播
        assert!(MacAddr::broadcast().is_multicast());
    }

    #[test]
    fn test_mac_addr_equality() {
        let mac1 = MacAddr::new([0x00, 0x11, 0x22, 0x33, 0x44, 0x55]);
        let mac2 = MacAddr::new([0x00, 0x11, 0x22, 0x33, 0x44, 0x55]);
        let mac3 = MacAddr::new([0x00, 0x11, 0x22, 0x33, 0x44, 0x56]);

        assert_eq!(mac1, mac2);
        assert_ne!(mac1, mac3);
    }

    #[test]
    fn test_mac_addr_display() {
        let mac = MacAddr::new([0x00, 0x11, 0x22, 0x33, 0x44, 0x55]);
        assert_eq!(format!("{}", mac), "00:11:22:33:44:55");
    }

    // ========== Ipv4Addr 测试 ==========

    #[test]
    fn test_ipv4_addr_creation() {
        let ip = Ipv4Addr::new(192, 168, 1, 1);
        assert_eq!(ip.bytes, [192, 168, 1, 1]);
    }

    #[test]
    fn test_ipv4_addr_parse() {
        let ip: Ipv4Addr = "192.168.1.1".parse().unwrap();
        assert_eq!(ip.bytes, [192, 168, 1, 1]);

        let ip: Ipv4Addr = "0.0.0.0".parse().unwrap();
        assert_eq!(ip.bytes, [0, 0, 0, 0]);

        let ip: Ipv4Addr = "255.255.255.255".parse().unwrap();
        assert_eq!(ip.bytes, [255, 255, 255, 255]);
    }

    #[test]
    fn test_ipv4_addr_parse_invalid() {
        let result: Result<Ipv4Addr, _> = "invalid".parse();
        assert!(result.is_err());

        let result: Result<Ipv4Addr, _> = "192.168.1".parse();
        assert!(result.is_err());

        let result: Result<Ipv4Addr, _> = "192.168.1.1.1".parse();
        assert!(result.is_err());

        let result: Result<Ipv4Addr, _> = "256.0.0.1".parse();
        assert!(result.is_err());
    }

    #[test]
    fn test_ipv4_addr_localhost() {
        let ip = Ipv4Addr::localhost();
        assert_eq!(ip.bytes, [127, 0, 0, 1]);
        assert!(ip.is_loopback());
    }

    #[test]
    fn test_ipv4_addr_unspecified() {
        let ip = Ipv4Addr::unspecified();
        assert_eq!(ip.bytes, [0, 0, 0, 0]);
        assert!(ip.is_unspecified());
        assert!(ip.is_zero());
    }

    #[test]
    fn test_ipv4_addr_broadcast() {
        let ip = Ipv4Addr::broadcast();
        assert_eq!(ip.bytes, [255, 255, 255, 255]);
        assert!(ip.is_broadcast());
    }

    #[test]
    fn test_ipv4_addr_loopback_range() {
        // 127.0.0.0/8 都是回环地址
        assert!(Ipv4Addr::new(127, 0, 0, 1).is_loopback());
        assert!(Ipv4Addr::new(127, 255, 255, 255).is_loopback());
        assert!(!Ipv4Addr::new(128, 0, 0, 1).is_loopback());
    }

    #[test]
    fn test_ipv4_addr_to_u32() {
        let ip = Ipv4Addr::new(192, 168, 1, 1);
        let u32_val = ip.to_u32();
        assert_eq!(u32_val, 0xC0A80101);

        let ip2 = Ipv4Addr::from_u32(u32_val);
        assert_eq!(ip2, ip);
    }

    #[test]
    fn test_ipv4_addr_equality() {
        let ip1 = Ipv4Addr::new(192, 168, 1, 1);
        let ip2 = Ipv4Addr::new(192, 168, 1, 1);
        let ip3 = Ipv4Addr::new(192, 168, 1, 2);

        assert_eq!(ip1, ip2);
        assert_ne!(ip1, ip3);
    }

    #[test]
    fn test_ipv4_addr_display() {
        let ip = Ipv4Addr::new(192, 168, 1, 1);
        assert_eq!(format!("{}", ip), "192.168.1.1");
    }

    #[test]
    fn test_ipv4_addr_ord() {
        let ip1 = Ipv4Addr::new(192, 168, 1, 1);
        let ip2 = Ipv4Addr::new(192, 168, 1, 2);
        assert!(ip1 < ip2);
        assert!(ip2 > ip1);
    }

    // ========== InterfaceState 测试 ==========

    #[test]
    fn test_interface_state_equality() {
        assert_eq!(InterfaceState::Up, InterfaceState::Up);
        assert_ne!(InterfaceState::Up, InterfaceState::Down);
    }

    #[test]
    fn test_interface_state_copy() {
        let state1 = InterfaceState::Up;
        let state2 = state1;
        assert_eq!(state1, InterfaceState::Up);
        assert_eq!(state2, InterfaceState::Up);
    }

    // ========== InterfaceType 测试 ==========

    #[test]
    fn test_interface_type_equality() {
        assert_eq!(InterfaceType::Ethernet, InterfaceType::Ethernet);
        assert_ne!(InterfaceType::Ethernet, InterfaceType::Loopback);
    }

    #[test]
    fn test_interface_type_copy() {
        let if_type1 = InterfaceType::Ethernet;
        let if_type2 = if_type1;
        assert_eq!(if_type1, InterfaceType::Ethernet);
        assert_eq!(if_type2, InterfaceType::Ethernet);
    }

    // ========== InterfaceError 测试 ==========

    #[test]
    fn test_interface_error_display() {
        let err = InterfaceError::DuplicateName("eth0".to_string());
        let msg = format!("{}", err);
        assert!(msg.contains("eth0"));
        assert!(msg.contains("已存在"));
    }

    #[test]
    fn test_interface_error_not_found() {
        let err = InterfaceError::InterfaceNotFound;
        let msg = format!("{}", err);
        assert!(msg.contains("未找到"));
    }

    #[test]
    fn test_interface_error_invalid_mac() {
        let err = InterfaceError::InvalidMacAddr("invalid".to_string());
        let msg = format!("{}", err);
        assert!(msg.contains("invalid"));
        assert!(msg.contains("MAC"));
    }

    #[test]
    fn test_interface_error_invalid_ip() {
        let err = InterfaceError::InvalidIpAddr("invalid".to_string());
        let msg = format!("{}", err);
        assert!(msg.contains("invalid"));
        assert!(msg.contains("IP"));
    }

    #[test]
    fn test_interface_error_invalid_mtu() {
        let err = InterfaceError::InvalidMtu(65535);
        let msg = format!("{}", err);
        assert!(msg.contains("65535"));
        assert!(msg.contains("MTU"));
    }
}
