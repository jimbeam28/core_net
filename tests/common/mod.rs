// 测试公共模块 - 提供各测试文件共用的辅助函数

use core_net::testframework::GlobalStateManager;
use core_net::context::SystemContext;
use core_net::interface::{InterfaceConfig, InterfaceState, MacAddr, Ipv4Addr};
use core_net::protocols::Ipv6Addr;

/// 创建测试用 eth0 配置
pub fn create_test_eth0_config() -> InterfaceConfig {
    InterfaceConfig {
        name: "eth0".to_string(),
        mac_addr: MacAddr::new([0x00, 0x11, 0x22, 0x33, 0x44, 0x55]),
        ip_addr: Ipv4Addr::new(192, 168, 1, 100),
        ipv6_addr: Ipv6Addr::new(0xfe80, 0, 0, 0, 0, 0, 0, 1),
        netmask: Ipv4Addr::new(255, 255, 255, 0),
        gateway: Some(Ipv4Addr::new(192, 168, 1, 1)),
        mtu: Some(1500),
        state: Some(InterfaceState::Up),
    }
}

/// 创建测试用系统上下文
pub fn create_test_context() -> SystemContext {
    GlobalStateManager::create_context()
}
