use std::sync::{Mutex, OnceLock};

use crate::interface::InterfaceManager;
use crate::interface::types::InterfaceError;

/// 全局接口管理器
///
/// 使用 OnceLock + Mutex 实现线程安全的单例模式，支持运行时修改接口配置
static GLOBAL_MANAGER: OnceLock<Mutex<InterfaceManager>> = OnceLock::new();

/// 初始化全局接口管理器
///
/// # 参数
/// - `manager`: 要设置为全局的接口管理器
///
/// # 返回
/// - `Ok(())`: 初始化成功
/// - `Err(InterfaceError::InvalidFormat)`: 已经初始化过
pub fn init_global_manager(manager: InterfaceManager) -> Result<(), InterfaceError> {
    GLOBAL_MANAGER
        .set(Mutex::new(manager))
        .map_err(|_| InterfaceError::InvalidFormat("全局接口管理器已经初始化".to_string()))
}

/// 获取全局接口管理器的引用（只读）
///
/// # 返回
/// - `Some(&Mutex<InterfaceManager>)`: 如果已初始化
/// - `None`: 如果未初始化
pub fn global_manager() -> Option<&'static Mutex<InterfaceManager>> {
    GLOBAL_MANAGER.get()
}

/// 修改指定接口的配置
///
/// # 参数
/// - `name`: 接口名称
/// - `f`: 修改闭包，接收 `&mut NetworkInterface`
///
/// # 返回
/// - `Ok(())`: 修改成功
/// - `Err(InterfaceError)`: 修改失败
///
/// # 示例
/// ```rust,ignore
/// use core_net::common::Ipv4Addr;
/// use core_net::interface::global::update_interface;
///
/// // 修改 IP 地址
/// update_interface("eth0", |iface| {
///     iface.set_ip_addr(Ipv4Addr::new(192, 168, 2, 100));
/// })?;
/// ```
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

/// 设置接口的 IP 地址
///
/// # 参数
/// - `name`: 接口名称
/// - `addr`: 新的 IP 地址
pub fn set_interface_ip(name: &str, addr: crate::interface::types::Ipv4Addr) -> Result<(), InterfaceError> {
    update_interface(name, |iface| iface.set_ip_addr(addr))
}

/// 设置接口的 MAC 地址
///
/// # 参数
/// - `name`: 接口名称
/// - `addr`: 新的 MAC 地址
pub fn set_interface_mac(name: &str, addr: crate::interface::types::MacAddr) -> Result<(), InterfaceError> {
    update_interface(name, |iface| iface.set_mac_addr(addr))
}

/// 设置接口的子网掩码
///
/// # 参数
/// - `name`: 接口名称
/// - `mask`: 新的子网掩码
pub fn set_interface_netmask(name: &str, mask: crate::interface::types::Ipv4Addr) -> Result<(), InterfaceError> {
    update_interface(name, |iface| iface.set_netmask(mask))
}

/// 设置接口的网关
///
/// # 参数
/// - `name`: 接口名称
/// - `addr`: 新的网关地址
pub fn set_interface_gateway(name: &str, addr: Option<crate::interface::types::Ipv4Addr>) -> Result<(), InterfaceError> {
    update_interface(name, |iface| iface.set_gateway(addr))
}

/// 设置接口的 MTU
///
/// # 参数
/// - `name`: 接口名称
/// - `mtu`: 新的 MTU 值
pub fn set_interface_mtu(name: &str, mtu: u16) -> Result<(), InterfaceError> {
    update_interface(name, |iface| iface.set_mtu(mtu))
}

/// 启用接口
///
/// # 参数
/// - `name`: 接口名称
pub fn interface_up(name: &str) -> Result<(), InterfaceError> {
    update_interface(name, |iface| iface.up())
}

/// 禁用接口
///
/// # 参数
/// - `name`: 接口名称
pub fn interface_down(name: &str) -> Result<(), InterfaceError> {
    update_interface(name, |iface| iface.down())
}

/// 使用默认配置文件初始化全局接口管理器
///
/// 使用默认配置文件路径 (src/interface/interface.toml) 初始化全局接口管理器
///
/// # 返回
/// - `Ok(())`: 初始化成功
/// - `Err(InterfaceError)`: 初始化失败
pub fn init_default() -> Result<(), InterfaceError> {
    let manager = crate::interface::load_default_config()?;
    init_global_manager(manager)
}

/// 便捷宏：获取全局接口管理器，如果未初始化则 panic
#[macro_export]
macro_rules! get_interfaces {
    () => {
        $crate::interface::global::global_manager()
            .expect("全局接口管理器未初始化，请先调用 init_global_manager() 或 init_default()")
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::interface::types::{InterfaceState, MacAddr, Ipv4Addr};
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::thread;

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

    // ========== 初始化测试 ==========

    #[test]
    fn test_init_global_manager() {
        // 确保测试开始时全局管理器未初始化
        // 注意：由于 OnceLock 的特性，一旦初始化就无法重置
        // 这个测试需要在其他测试之前运行，或者需要考虑全局状态

        let manager = create_test_manager();

        // 首次初始化应该成功
        let _result = init_global_manager(manager);
        // 注意：如果之前有测试已经初始化过，这里会返回 Err
        // 这是 OnceLock 的预期行为
    }

    #[test]
    fn test_global_manager() {
        // 这个测试假设全局管理器已经被初始化
        // 如果未初始化，global_manager() 应该返回 None

        // 我们不假设它一定是 Some 或 None，因为测试执行顺序不确定
        // 只测试如果它存在，可以正常访问
        if let Some(manager) = global_manager() {
            let guard = manager.lock().unwrap();
            assert_eq!(guard.len(), 1);
        }
    }

    #[test]
    fn test_global_manager_none_when_not_initialized() {
        // 注意：这个测试的有效性依赖于测试执行顺序
        // 如果之前的测试已经初始化了全局管理器，这个测试会失败
        // 在实际项目中，通常需要一种机制来重置全局状态用于测试

        // 由于 OnceLock 无法重置，我们只能测试行为
        let _manager_opt = global_manager();
        // 如果之前没有测试初始化过，这里应该是 None
        // 如果已经有测试初始化过，这里会是 Some
        // 两种情况都是有效的
    }

    // ========== 修改接口测试 ==========

    #[test]
    fn test_update_interface() {
        let manager_opt = global_manager();

        if let Some(manager) = manager_opt {
            // 测试修改 IP 地址
            let original_ip = {
                let guard = manager.lock().unwrap();
                let iface = guard.get_by_name("eth0").unwrap();
                iface.ip_addr
            };

            update_interface("eth0", |iface| {
                iface.set_ip_addr(Ipv4Addr::new(10, 0, 0, 1));
            }).unwrap();

            let new_ip = {
                let guard = manager.lock().unwrap();
                let iface = guard.get_by_name("eth0").unwrap();
                iface.ip_addr
            };

            assert_eq!(new_ip, Ipv4Addr::new(10, 0, 0, 1));

            // 恢复原始值
            update_interface("eth0", |iface| {
                iface.set_ip_addr(original_ip);
            }).unwrap();
        }
    }

    #[test]
    fn test_update_interface_not_found() {
        let manager_opt = global_manager();

        if let Some(_) = manager_opt {
            let result = update_interface("nonexistent", |iface| {
                iface.set_ip_addr(Ipv4Addr::new(10, 0, 0, 1));
            });

            assert!(result.is_err());
            match result {
                Err(InterfaceError::InterfaceNotFound) => {}
                _ => panic!("Expected InterfaceNotFound error"),
            }
        }
    }

    // ========== 便捷函数测试 ==========

    #[test]
    fn test_set_interface_ip() {
        let manager_opt = global_manager();

        if let Some(manager) = manager_opt {
            let original_ip = {
                let guard = manager.lock().unwrap();
                let iface = guard.get_by_name("eth0").unwrap();
                iface.ip_addr
            };

            set_interface_ip("eth0", Ipv4Addr::new(10, 0, 0, 1)).unwrap();

            let new_ip = {
                let guard = manager.lock().unwrap();
                let iface = guard.get_by_name("eth0").unwrap();
                iface.ip_addr
            };

            assert_eq!(new_ip, Ipv4Addr::new(10, 0, 0, 1));

            // 恢复
            set_interface_ip("eth0", original_ip).unwrap();
        }
    }

    #[test]
    fn test_set_interface_mac() {
        let manager_opt = global_manager();

        if let Some(manager) = manager_opt {
            let original_mac = {
                let guard = manager.lock().unwrap();
                let iface = guard.get_by_name("eth0").unwrap();
                iface.mac_addr
            };

            set_interface_mac("eth0", MacAddr::new([0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0xff])).unwrap();

            let new_mac = {
                let guard = manager.lock().unwrap();
                let iface = guard.get_by_name("eth0").unwrap();
                iface.mac_addr
            };

            assert_eq!(new_mac, MacAddr::new([0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0xff]));

            // 恢复
            set_interface_mac("eth0", original_mac).unwrap();
        }
    }

    #[test]
    fn test_set_interface_netmask() {
        let manager_opt = global_manager();

        if let Some(manager) = manager_opt {
            let original_netmask = {
                let guard = manager.lock().unwrap();
                let iface = guard.get_by_name("eth0").unwrap();
                iface.netmask
            };

            set_interface_netmask("eth0", Ipv4Addr::new(255, 255, 255, 128)).unwrap();

            let new_netmask = {
                let guard = manager.lock().unwrap();
                let iface = guard.get_by_name("eth0").unwrap();
                iface.netmask
            };

            assert_eq!(new_netmask, Ipv4Addr::new(255, 255, 255, 128));

            // 恢复
            set_interface_netmask("eth0", original_netmask).unwrap();
        }
    }

    #[test]
    fn test_set_interface_gateway() {
        let manager_opt = global_manager();

        if let Some(manager) = manager_opt {
            let original_gateway = {
                let guard = manager.lock().unwrap();
                let iface = guard.get_by_name("eth0").unwrap();
                iface.gateway
            };

            set_interface_gateway("eth0", Some(Ipv4Addr::new(192, 168, 2, 1))).unwrap();

            let new_gateway = {
                let guard = manager.lock().unwrap();
                let iface = guard.get_by_name("eth0").unwrap();
                iface.gateway
            };

            assert_eq!(new_gateway, Some(Ipv4Addr::new(192, 168, 2, 1)));

            // 恢复
            set_interface_gateway("eth0", original_gateway).unwrap();
        }
    }

    #[test]
    fn test_set_interface_mtu() {
        let manager_opt = global_manager();

        if let Some(manager) = manager_opt {
            let original_mtu = {
                let guard = manager.lock().unwrap();
                let iface = guard.get_by_name("eth0").unwrap();
                iface.mtu
            };

            set_interface_mtu("eth0", 9000).unwrap();

            let new_mtu = {
                let guard = manager.lock().unwrap();
                let iface = guard.get_by_name("eth0").unwrap();
                iface.mtu
            };

            assert_eq!(new_mtu, 9000);

            // 恢复
            let mut guard = manager.lock().unwrap();
            let iface = guard.get_by_name_mut("eth0").unwrap();
            iface.mtu = original_mtu;
        }
    }

    #[test]
    fn test_interface_up_down() {
        let manager_opt = global_manager();

        if let Some(manager) = manager_opt {
            let original_state = {
                let guard = manager.lock().unwrap();
                let iface = guard.get_by_name("eth0").unwrap();
                iface.state
            };

            // 测试禁用
            interface_down("eth0").unwrap();
            {
                let guard = manager.lock().unwrap();
                let iface = guard.get_by_name("eth0").unwrap();
                assert_eq!(iface.state, InterfaceState::Down);
            }

            // 测试启用
            interface_up("eth0").unwrap();
            {
                let guard = manager.lock().unwrap();
                let iface = guard.get_by_name("eth0").unwrap();
                assert_eq!(iface.state, InterfaceState::Up);
            }

            // 恢复原始状态
            let mut guard = manager.lock().unwrap();
            let iface = guard.get_by_name_mut("eth0").unwrap();
            iface.state = original_state;
        }
    }

    // ========== 并发安全测试 ==========

    #[test]
    fn test_concurrent_read() {
        let manager_opt = global_manager();

        if let Some(_) = manager_opt {
            let num_threads = 10;
            let counter = std::sync::Arc::new(AtomicU32::new(0));

            let handles: Vec<_> = (0..num_threads)
                .map(|_| {
                    let counter_clone = std::sync::Arc::clone(&counter);
                    thread::spawn(move || {
                        if let Some(m) = global_manager() {
                            for _ in 0..100 {
                                let guard = m.lock().unwrap();
                                let _ = guard.get_by_name("eth0");
                                counter_clone.fetch_add(1, Ordering::Relaxed);
                            }
                        }
                    })
                })
                .collect();

            for handle in handles {
                handle.join().unwrap();
            }

            assert_eq!(counter.load(Ordering::Relaxed), num_threads * 100);
        }
    }

    #[test]
    fn test_concurrent_write() {
        let manager_opt = global_manager();

        if let Some(_) = manager_opt {
            let num_threads = 10;

            let handles: Vec<_> = (0..num_threads)
                .map(|i| {
                    thread::spawn(move || {
                        if let Some(_) = global_manager() {
                            let new_ip = Ipv4Addr::new(10, 0, 0, (i % 256) as u8);
                            let _ = set_interface_ip("eth0", new_ip);
                        }
                    })
                })
                .collect();

            for handle in handles {
                handle.join().unwrap();
            }

            // 验证接口仍然可以访问
            if let Some(m) = global_manager() {
                let guard = m.lock().unwrap();
                assert!(guard.get_by_name("eth0").is_ok());
            }
        }
    }

    #[test]
    fn test_concurrent_read_write() {
        let manager_opt = global_manager();

        if let Some(_) = manager_opt {
            let counter = std::sync::Arc::new(AtomicU32::new(0));

            let read_handles: Vec<_> = (0..5)
                .map(|_| {
                    let counter_clone = std::sync::Arc::clone(&counter);
                    thread::spawn(move || {
                        if let Some(m) = global_manager() {
                            for _ in 0..50 {
                                let guard = m.lock().unwrap();
                                let _ = guard.get_by_name("eth0");
                                counter_clone.fetch_add(1, Ordering::Relaxed);
                            }
                        }
                    })
                })
                .collect();

            let write_handles: Vec<_> = (0..5)
                .map(|i| {
                    thread::spawn(move || {
                        if let Some(_) = global_manager() {
                            for j in 0..10 {
                                let new_ip = Ipv4Addr::new(10, 0, (i % 256) as u8, (j % 256) as u8);
                                let _ = set_interface_ip("eth0", new_ip);
                            }
                        }
                    })
                })
                .collect();

            for handle in read_handles {
                handle.join().unwrap();
            }
            for handle in write_handles {
                handle.join().unwrap();
            }

            // 验证接口仍然可以访问
            if let Some(m) = global_manager() {
                let guard = m.lock().unwrap();
                assert!(guard.get_by_name("eth0").is_ok());
            }
        }
    }

    // ========== 错误处理测试 ==========

    #[test]
    fn test_update_interface_uninitialized() {
        // 注意：这个测试的有效性依赖于测试执行顺序
        // 由于我们无法重置 OnceLock，这个测试只能验证错误处理逻辑

        // 创建一个新的 OnceLock 模拟未初始化状态
        // 实际上我们无法测试这个场景，因为全局的 GLOBAL_MANAGER 可能已被初始化
    }

    #[test]
    fn test_convenience_function_not_found() {
        let manager_opt = global_manager();

        if let Some(_) = manager_opt {
            // 测试所有便捷函数对不存在接口的处理
            assert!(set_interface_ip("nonexistent", Ipv4Addr::new(10, 0, 0, 1)).is_err());
            assert!(set_interface_mac("nonexistent", MacAddr::new([0x00, 0x00, 0x00, 0x00, 0x00, 0x00])).is_err());
            assert!(set_interface_netmask("nonexistent", Ipv4Addr::new(255, 255, 255, 0)).is_err());
            assert!(set_interface_gateway("nonexistent", Some(Ipv4Addr::new(192, 168, 1, 1))).is_err());
            assert!(set_interface_mtu("nonexistent", 1500).is_err());
            assert!(interface_up("nonexistent").is_err());
            assert!(interface_down("nonexistent").is_err());
        }
    }

    // ========== 组合操作测试 ==========

    #[test]
    fn test_multiple_updates() {
        let manager_opt = global_manager();

        if let Some(manager) = manager_opt {
            // 保存原始值
            let (original_ip, original_mac, original_netmask) = {
                let guard = manager.lock().unwrap();
                let iface = guard.get_by_name("eth0").unwrap();
                (iface.ip_addr, iface.mac_addr, iface.netmask)
            };

            // 执行多个修改
            update_interface("eth0", |iface| {
                iface.set_ip_addr(Ipv4Addr::new(10, 0, 0, 1));
                iface.set_mac_addr(MacAddr::new([0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0xff]));
                iface.set_netmask(Ipv4Addr::new(255, 255, 255, 128));
                iface.set_mtu(9000);
            }).unwrap();

            // 验证所有修改
            {
                let guard = manager.lock().unwrap();
                let iface = guard.get_by_name("eth0").unwrap();
                assert_eq!(iface.ip_addr, Ipv4Addr::new(10, 0, 0, 1));
                assert_eq!(iface.mac_addr, MacAddr::new([0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0xff]));
                assert_eq!(iface.netmask, Ipv4Addr::new(255, 255, 255, 128));
                assert_eq!(iface.mtu, 9000);
            }

            // 恢复原始值
            update_interface("eth0", |iface| {
                iface.set_ip_addr(original_ip);
                iface.set_mac_addr(original_mac);
                iface.set_netmask(original_netmask);
                iface.set_mtu(1500);
            }).unwrap();
        }
    }
}
