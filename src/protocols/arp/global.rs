// src/protocols/arp/global.rs
//
// 全局 ARP 缓存管理
// 使用 OnceLock + Mutex 实现线程安全的单例模式

use std::sync::{Mutex, OnceLock};

use crate::protocols::arp::tables::ArpCache;

#[cfg(test)]
use std::sync::atomic::AtomicBool;

/// 全局 ARP 缓存
///
/// 使用 OnceLock + Mutex 实现线程安全的单例模式
static GLOBAL_ARP_CACHE: OnceLock<Mutex<ArpCache>> = OnceLock::new();

/// 标记是否需要重置全局缓存（用于测试）
#[cfg(test)]
static RESET_FLAG: AtomicBool = AtomicBool::new(false);

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

/// 获取或初始化全局 ARP 缓存
///
/// 使用 `get_or_init()` 确保线程安全的懒初始化。
/// 首次调用时自动初始化，后续调用直接返回已初始化的引用。
///
/// # 返回
/// - 全局 ARP 缓存的 Mutex 引用
///
/// # 优势
/// - 线程安全：`get_or_init()` 保证原子操作
/// - 永不失败：无需错误处理
/// - 测试友好：每次测试可以清空缓存，重新初始化
///
/// # 示例
/// ```ignore
/// use core_net::protocols::arp::get_or_init_global_arp_cache;
///
/// // 直接获取缓存，无需检查是否已初始化
/// let cache = get_or_init_global_arp_cache();
/// let mut guard = cache.lock().unwrap();
/// guard.update_arp(...);
/// ```
pub fn get_or_init_global_arp_cache() -> &'static Mutex<ArpCache> {
    GLOBAL_ARP_CACHE.get_or_init(|| {
        Mutex::new(ArpCache::default())
    })
}

/// 重置全局ARP缓存（仅用于测试）
///
/// # Safety
/// 此函数仅用于测试，确保没有其他线程正在使用全局缓存
#[cfg(test)]
pub unsafe fn reset_global_arp_cache_for_test() {
    use std::sync::atomic::Ordering;
    RESET_FLAG.store(true, Ordering::SeqCst);
    // 注意：OnceLock 无法真正重置，这个函数只是设置标志
    // 实际的重置需要在测试中重新初始化
}

/// 检查是否需要重置（测试辅助函数）
#[cfg(test)]
pub fn should_reset_global_cache() -> bool {
    use std::sync::atomic::Ordering;
    RESET_FLAG.load(Ordering::SeqCst)
}

/// 清除重置标志（测试辅助函数）
#[cfg(test)]
pub fn clear_reset_flag() {
    use std::sync::atomic::Ordering;
    RESET_FLAG.store(false, Ordering::SeqCst);
}
