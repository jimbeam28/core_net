// src/protocols/arp/global.rs
//
// 全局 ARP 缓存管理（已弃用）
// 使用 OnceLock + Mutex 实现线程安全的单例模式
//
// **弃用警告**: 此模块已弃用，推荐使用 `SystemContext` 进行依赖注入。
// 全局状态无法在测试间重置，导致 `cargo test` 挂起。

use std::sync::{Mutex, OnceLock};

use crate::protocols::arp::tables::ArpCache;

/// 全局 ARP 缓存（已弃用）
///
/// 使用 OnceLock + Mutex 实现线程安全的单例模式
///
/// **弃用警告**: 推荐使用 `SystemContext` 替代全局状态。
static GLOBAL_ARP_CACHE: OnceLock<Mutex<ArpCache>> = OnceLock::new();

/// 初始化全局 ARP 缓存（已弃用）
///
/// # 参数
/// - `cache`: 要设置为全局的 ARP 缓存
///
/// # 返回
/// - `Ok(())`: 初始化成功
/// - `Err(&'static str)`: 已经初始化过
///
/// # 弃用警告
/// 此函数已弃用，推荐使用 `SystemContext` 进行依赖注入。
#[deprecated(note = "使用 SystemContext 替代全局状态")]
pub fn init_global_arp_cache(cache: ArpCache) -> Result<(), &'static str> {
    GLOBAL_ARP_CACHE
        .set(Mutex::new(cache))
        .map_err(|_| "全局 ARP 缓存已经初始化")
}

/// 获取全局 ARP 缓存的引用（已弃用）
///
/// # 返回
/// - `Some(&Mutex<ArpCache>)`: 如果已初始化
/// - `None`: 如果未初始化
///
/// # 弃用警告
/// 此函数已弃用，推荐使用 `SystemContext` 进行依赖注入。
#[deprecated(note = "使用 SystemContext 替代全局状态")]
pub fn global_arp_cache() -> Option<&'static Mutex<ArpCache>> {
    GLOBAL_ARP_CACHE.get()
}

/// 使用默认配置初始化全局 ARP 缓存（已弃用）
///
/// # 返回
/// - `Ok(())`: 初始化成功
/// - `Err(&'static str)`: 已经初始化过
///
/// # 弃用警告
/// 此函数已弃用，推荐使用 `SystemContext` 进行依赖注入。
#[deprecated(note = "使用 SystemContext 替代全局状态")]
pub fn init_default_arp_cache() -> Result<(), &'static str> {
    init_global_arp_cache(ArpCache::default())
}

/// 获取或初始化全局 ARP 缓存（已弃用）
///
/// 使用 `get_or_init()` 确保线程安全的懒初始化。
/// 首次调用时自动初始化，后续调用直接返回已初始化的引用。
///
/// # 返回
/// - 全局 ARP 缓存的 Mutex 引用
///
/// # 弃用警告
/// **此函数已弃用，不应再使用！**
///
/// 全局 ARP 缓存使用 `OnceLock`，一旦初始化就无法重置。
/// 这会导致 `cargo test` 挂起，因为全局状态在测试间持久化。
///
/// 推荐使用 `SystemContext` 进行依赖注入：
/// ```ignore
/// use core_net::context::SystemContext;
/// use core_net::testframework::GlobalStateManager;
///
/// let ctx = GlobalStateManager::create_context();
/// let cache = ctx.arp_cache.lock().unwrap();
/// ```
#[deprecated(note = "使用 SystemContext 替代全局状态")]
pub fn get_or_init_global_arp_cache() -> &'static Mutex<ArpCache> {
    GLOBAL_ARP_CACHE.get_or_init(|| {
        Mutex::new(ArpCache::default())
    })
}
