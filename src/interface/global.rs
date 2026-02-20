use std::sync::{Mutex, OnceLock};
use crate::interface::InterfaceManager;
use crate::interface::types::InterfaceError;

/// 全局接口管理器，使用 OnceLock + Mutex 实现线程安全的单例模式（已弃用）
///
/// **弃用警告**: 推荐使用 `SystemContext` 替代全局状态。
/// 全局状态无法在测试间重置，导致 `cargo test` 挂起。
static GLOBAL_MANAGER: OnceLock<Mutex<InterfaceManager>> = OnceLock::new();

/// 初始化全局接口管理器（已弃用）
///
/// # 弃用警告
/// 此函数已弃用，推荐使用 `SystemContext` 进行依赖注入。
#[deprecated(note = "使用 SystemContext 替代全局状态")]
pub fn init_global_manager(manager: InterfaceManager) -> Result<(), InterfaceError> {
    GLOBAL_MANAGER
        .set(Mutex::new(manager))
        .map_err(|_| InterfaceError::InvalidFormat("全局接口管理器已经初始化".to_string()))
}

/// 获取全局接口管理器的引用（已弃用）
///
/// # 弃用警告
/// 此函数已弃用，推荐使用 `SystemContext` 进行依赖注入。
#[deprecated(note = "使用 SystemContext 替代全局状态")]
pub fn global_manager() -> Option<&'static Mutex<InterfaceManager>> {
    GLOBAL_MANAGER.get()
}

/// 修改指定接口的配置，通过闭包批量修改接口属性（已弃用）
///
/// # 弃用警告
/// 此函数已弃用，推荐使用 `SystemContext` 进行依赖注入。
#[deprecated(note = "使用 SystemContext 替代全局状态")]
pub fn update_interface<F>(name: &str, f: F) -> Result<(), InterfaceError>
where
    F: FnOnce(&mut crate::interface::iface::NetworkInterface),
{
    let manager = GLOBAL_MANAGER
        .get()
        .ok_or_else(|| InterfaceError::InvalidFormat("全局接口管理器未初始化".to_string()))?;
    let mut guard = manager.lock().map_err(|e| {
        InterfaceError::MutexLockFailed(format!("锁定互斥锁失败: {}", e.to_string()))
    })?;
    let iface = guard.get_by_name_mut(name)?;
    f(iface);
    Ok(())
}

/// 使用默认配置文件初始化全局接口管理器（已弃用）
///
/// # 弃用警告
/// 此函数已弃用，推荐使用 `SystemContext::from_config()`。
#[deprecated(note = "使用 SystemContext::from_config() 替代")]
pub fn init_default() -> Result<(), InterfaceError> {
    let manager = crate::interface::load_default_config()?;
    init_global_manager(manager)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::interface::types::{InterfaceState, MacAddr, Ipv4Addr};

    // ========== 测试辅助函数 ==========

    /// 创建测试用管理器
    fn create_test_manager() -> InterfaceManager {
        let mut manager = InterfaceManager::new(256, 256);

        let config = crate::interface::iface::InterfaceConfig {
            name: "eth0".to_string(),
            mac_addr: MacAddr::new([0x00, 0x11, 0x22, 0x33, 0x44, 0x55]),
            ip_addr: Ipv4Addr::new(192, 168, 1, 100),
            netmask: Ipv4Addr::new(255, 255, 255, 0),
            gateway: Some(Ipv4Addr::new(192, 168, 1, 1)),
            mtu: Some(1500),
            state: Some(InterfaceState::Up),
        };
        manager.add_from_config(config).unwrap();

        manager
    }

    // ========== 基础功能测试（使用本地实例） ==========

    #[test]
    fn test_create_manager() {
        let manager = create_test_manager();
        assert_eq!(manager.len(), 1);
        assert!(manager.get_by_name("eth0").is_ok());
    }

    #[test]
    fn test_interface_ip_addr() {
        let manager = create_test_manager();
        let iface = manager.get_by_name("eth0").unwrap();
        assert_eq!(iface.ip_addr, Ipv4Addr::new(192, 168, 1, 100));
    }

    #[test]
    fn test_interface_mac_addr() {
        let manager = create_test_manager();
        let iface = manager.get_by_name("eth0").unwrap();
        assert_eq!(iface.mac_addr, MacAddr::new([0x00, 0x11, 0x22, 0x33, 0x44, 0x55]));
    }

    #[test]
    fn test_interface_state() {
        let manager = create_test_manager();
        let iface = manager.get_by_name("eth0").unwrap();
        assert_eq!(iface.state, InterfaceState::Up);
    }

    #[test]
    fn test_interface_mtu() {
        let manager = create_test_manager();
        let iface = manager.get_by_name("eth0").unwrap();
        assert_eq!(iface.mtu, 1500);
    }

    #[test]
    fn test_interface_netmask() {
        let manager = create_test_manager();
        let iface = manager.get_by_name("eth0").unwrap();
        assert_eq!(iface.netmask, Ipv4Addr::new(255, 255, 255, 0));
    }

    #[test]
    fn test_modify_interface_ip() {
        let mut manager = create_test_manager();

        // 获取原始 IP
        let original_ip = {
            let iface = manager.get_by_name("eth0").unwrap();
            iface.ip_addr
        };

        // 修改 IP
        {
            let iface = manager.get_by_name_mut("eth0").unwrap();
            iface.set_ip_addr(Ipv4Addr::new(10, 0, 0, 1));
        }

        // 验证修改
        {
            let iface = manager.get_by_name("eth0").unwrap();
            assert_eq!(iface.ip_addr, Ipv4Addr::new(10, 0, 0, 1));
        }

        // 恢复原始值
        {
            let iface = manager.get_by_name_mut("eth0").unwrap();
            iface.set_ip_addr(original_ip);
        }
    }

    #[test]
    fn test_interface_not_found() {
        let manager = create_test_manager();
        let result = manager.get_by_name("nonexistent");
        assert!(result.is_err());
        match result {
            Err(InterfaceError::InterfaceNotFound) => {}
            _ => panic!("Expected InterfaceNotFound error"),
        }
    }

    #[test]
    fn test_interface_state_control() {
        let mut manager = create_test_manager();

        // 测试禁用
        {
            let iface = manager.get_by_name_mut("eth0").unwrap();
            iface.down();
        }
        {
            let iface = manager.get_by_name("eth0").unwrap();
            assert_eq!(iface.state, InterfaceState::Down);
        }

        // 测试启用
        {
            let iface = manager.get_by_name_mut("eth0").unwrap();
            iface.up();
        }
        {
            let iface = manager.get_by_name("eth0").unwrap();
            assert_eq!(iface.state, InterfaceState::Up);
        }
    }

    #[test]
    fn test_multiple_interface_modifications() {
        let mut manager = create_test_manager();

        // 保存原始值
        let (original_ip, original_mac, original_netmask, original_mtu) = {
            let iface = manager.get_by_name("eth0").unwrap();
            (iface.ip_addr, iface.mac_addr, iface.netmask, iface.mtu)
        };

        // 执行多个修改
        {
            let iface = manager.get_by_name_mut("eth0").unwrap();
            iface.set_ip_addr(Ipv4Addr::new(10, 0, 0, 1));
            iface.set_mac_addr(MacAddr::new([0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0xff]));
            iface.set_netmask(Ipv4Addr::new(255, 255, 255, 128));
            iface.set_mtu(9000);
        }

        // 验证所有修改
        {
            let iface = manager.get_by_name("eth0").unwrap();
            assert_eq!(iface.ip_addr, Ipv4Addr::new(10, 0, 0, 1));
            assert_eq!(iface.mac_addr, MacAddr::new([0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0xff]));
            assert_eq!(iface.netmask, Ipv4Addr::new(255, 255, 255, 128));
            assert_eq!(iface.mtu, 9000);
        }

        // 恢复原始值
        {
            let iface = manager.get_by_name_mut("eth0").unwrap();
            iface.set_ip_addr(original_ip);
            iface.set_mac_addr(original_mac);
            iface.set_netmask(original_netmask);
            iface.set_mtu(original_mtu);
        }
    }
}
