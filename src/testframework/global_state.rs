//! 全局状态管理器
//!
//! 提供全局状态的安全访问、Mutex 毒化自动恢复和测试隔离保证。

use std::sync::MutexGuard;
use crate::interface::{InterfaceManager, MacAddr, Ipv4Addr, init_default, global_manager};
use crate::protocols::arp::{ArpCache, global_arp_cache, init_default_arp_cache, get_or_init_global_arp_cache};
use crate::testframework::error::{HarnessError, HarnessResult};
use crate::common::tables::Table;

/// 接口测试配置
///
/// 用于重置接口配置
#[derive(Debug, Clone)]
pub struct InterfaceTestConfig {
    /// 接口名称
    pub name: String,
    /// MAC 地址
    pub mac_addr: MacAddr,
    /// IP 地址
    pub ip_addr: Ipv4Addr,
    /// 子网掩码
    pub netmask: Ipv4Addr,
    /// MTU
    pub mtu: u16,
    /// 网关
    pub gateway: Option<Ipv4Addr>,
}

/// 全局状态管理器
///
/// 提供全局状态的安全访问、Mutex 毒化自动恢复和测试隔离保证。
/// 整合了原 GlobalTestSetup 的功能。
pub struct GlobalStateManager;

impl GlobalStateManager {
    /// 清空全局状态
    ///
    /// 清空所有接口的队列并重置配置，自动恢复毒化的 Mutex。
    ///
    /// # 返回
    /// - Ok(()): 清空成功
    /// - Err(HarnessError): 清空失败
    pub fn clear_global_state() -> HarnessResult<()> {
        // 第一步：清空 ARP 缓存（获取并立即释放 ARP 缓存锁）
        if let Some(_arp_cache) = global_arp_cache() {
            let mut cache = Self::get_or_recover_arp_lock();
            cache.clear();
            // ARP 缓存锁在这里自动释放
        }

        // 第二步：清空接口管理器状态（获取并立即释放接口管理器锁）
        // 确保不持有 ARP 缓存锁的情况下获取接口管理器锁，避免死锁
        if let Some(_global_mgr) = global_manager() {
            let mut guard = Self::get_or_recover_interface_lock();

            let len = guard.len();
            for i in 0..len {
                if let Ok(iface) = guard.get_by_index_mut(i as u32) {
                    // 清空队列
                    iface.rxq.clear();
                    iface.txq.clear();

                    // 重置常见接口的配置
                    match iface.name() {
                        "eth0" => {
                            iface.ip_addr = Ipv4Addr::new(192, 168, 1, 100);
                            iface.mac_addr = MacAddr::new([0x00, 0x11, 0x22, 0x33, 0x44, 0x55]);
                            iface.netmask = Ipv4Addr::new(255, 255, 255, 0);
                            iface.mtu = 1500;
                        }
                        "lo" => {
                            iface.ip_addr = Ipv4Addr::new(127, 0, 0, 1);
                            iface.mac_addr = MacAddr::zero();
                            iface.netmask = Ipv4Addr::new(255, 0, 0, 0);
                            iface.mtu = 65535;
                        }
                        _ => {
                            // 其他接口只清空队列，不修改配置
                        }
                    }
                }
            }
            // 接口管理器锁在这里自动释放
        }

        Ok(())
    }

    /// 初始化全局状态
    ///
    /// 初始化全局接口管理器和 ARP 缓存（如果尚未初始化）。
    /// 注意：不清空现有状态，由测试负责在测试后清理。
    ///
    /// # 返回
    /// - Ok(()): 初始化成功
    /// - Err(HarnessError): 初始化失败
    pub fn setup_global_state() -> HarnessResult<()> {
        // 初始化全局 ARP 缓存（如果尚未初始化）
        let _ = init_default_arp_cache();

        // 初始化全局接口管理器
        let _ = init_default();

        Ok(())
    }

    /// 安全获取 ARP 缓存锁，恢复毒化 Mutex
    ///
    /// # 返回
    /// ARP 缓存的 Mutex 守卫
    pub fn get_or_recover_arp_lock() -> MutexGuard<'static, ArpCache> {
        let arp_cache_ref = get_or_init_global_arp_cache();

        // 使用 lock() 会阻塞等待，但因为是单线程测试，应该很快获取
        match arp_cache_ref.lock() {
            Ok(guard) => guard,
            Err(poisoned) => {
                // 恢复毒化的 Mutex
                poisoned.into_inner()
            }
        }
    }

    /// 安全获取接口管理器锁，恢复毒化 Mutex
    ///
    /// # 返回
    /// 接口管理器的 Mutex 守卫
    pub fn get_or_recover_interface_lock() -> MutexGuard<'static, InterfaceManager> {
        let interface_ref = global_manager()
            .expect("全局接口管理器未初始化，请先调用 init_default()");

        // 使用 lock() 会阻塞等待，但因为是单线程测试，应该很快获取
        match interface_ref.lock() {
            Ok(guard) => guard,
            Err(poisoned) => {
                // 恢复毒化的 Mutex
                poisoned.into_inner()
            }
        }
    }

    /// 重置接口配置
    ///
    /// 根据提供的配置列表重置接口，不重新初始化 OnceLock。
    ///
    /// # 参数
    /// - configs: 接口配置列表
    ///
    /// # 返回
    /// - Ok(()): 重置成功
    /// - Err(HarnessError): 重置失败
    pub fn reset_interface_configs(configs: Vec<InterfaceTestConfig>) -> HarnessResult<()> {
        let mut guard = Self::get_or_recover_interface_lock();

        for config in configs {
            let iface = guard.get_by_name_mut(&config.name)
                .map_err(|e| HarnessError::InterfaceError(format!("获取接口失败: {}", e)))?;

            iface.mac_addr = config.mac_addr;
            iface.ip_addr = config.ip_addr;
            iface.netmask = config.netmask;
            iface.mtu = config.mtu;
            iface.gateway = config.gateway;
        }

        Ok(())
    }
}
