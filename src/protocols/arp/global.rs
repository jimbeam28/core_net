// src/protocols/arp/global.rs
//
// 全局 ARP 缓存管理
// 使用 OnceLock + Mutex 实现线程安全的单例模式

use std::sync::{Mutex, OnceLock};

use crate::protocols::arp::tables::ArpCache;

/// 全局 ARP 缓存
///
/// 使用 OnceLock + Mutex 实现线程安全的单例模式
static GLOBAL_ARP_CACHE: OnceLock<Mutex<ArpCache>> = OnceLock::new();

/// 初始化全局 ARP 缓存
///
/// # 参数
/// - `cache`: 要设置为全局的 ARP 缓存
///
/// # 返回
/// - `Ok(())`: 初始化成功
/// - `Err(&'static str)`: 已经初始化过
pub fn init_global_arp_cache(cache: ArpCache) -> Result<(), &'static str> {
    GLOBAL_ARP_CACHE
        .set(Mutex::new(cache))
        .map_err(|_| "全局 ARP 缓存已经初始化")
}

/// 获取全局 ARP 缓存的引用
///
/// # 返回
/// - `Some(&Mutex<ArpCache>)`: 如果已初始化
/// - `None`: 如果未初始化
pub fn global_arp_cache() -> Option<&'static Mutex<ArpCache>> {
    GLOBAL_ARP_CACHE.get()
}

/// 使用默认配置初始化全局 ARP 缓存
///
/// # 返回
/// - `Ok(())`: 初始化成功
/// - `Err(&'static str)`: 已经初始化过
pub fn init_default_arp_cache() -> Result<(), &'static str> {
    init_global_arp_cache(ArpCache::default())
}
