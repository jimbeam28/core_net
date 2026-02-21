//! 全局状态管理器
//!
//! 提供测试用的系统上下文创建功能。
//!
//! ## 用法：使用 SystemContext
//!
//! 推荐使用 `SystemContext` 进行依赖注入：
//!
//! ```ignore
//! use core_net::testframework::GlobalStateManager;
//! use core_net::engine::PacketProcessor;
//!
//! let ctx = GlobalStateManager::create_context();
//! let processor = PacketProcessor::with_context(ctx);
//! ```

use std::sync::Arc;
use crate::interface::InterfaceManager;
use crate::protocols::arp::ArpCache;
use crate::protocols::icmp::EchoManager;
use crate::context::SystemContext;
use crate::common::tables::Table;

/// 全局状态管理器
///
/// 提供测试用的系统上下文创建功能。
/// 使用依赖注入模式，避免全局状态。
pub struct GlobalStateManager;

impl GlobalStateManager {
    /// 创建用于测试的系统上下文
    ///
    /// 返回包含默认初始化组件的 `SystemContext`。
    ///
    /// # 返回
    ///
    /// 返回包含默认初始化组件的 `SystemContext`
    ///
    /// # 示例
    ///
    /// ```ignore
    /// use core_net::testframework::GlobalStateManager;
    /// use core_net::engine::PacketProcessor;
    ///
    /// let ctx = GlobalStateManager::create_context();
    /// let processor = PacketProcessor::with_context(ctx);
    /// ```
    pub fn create_context() -> SystemContext {
        // 加载默认配置创建接口管理器
        let interface_manager = match crate::interface::load_default_config() {
            Ok(manager) => manager,
            Err(e) => {
                eprintln!("[警告] 加载接口配置失败: {}, 使用空接口管理器", e);
                InterfaceManager::default()
            }
        };

        // 使用指定组件创建 SystemContext
        SystemContext::with_components(
            Arc::new(std::sync::Mutex::new(interface_manager)),
            Arc::new(std::sync::Mutex::new(ArpCache::default())),
            Arc::new(std::sync::Mutex::new(EchoManager::default())),
            Arc::new(std::sync::Mutex::new(crate::protocols::tcp::TcpConnectionManager::default())),
            None,
        )
    }

    /// 创建空的系统上下文
    ///
    /// 返回一个空的系统上下文，所有组件使用默认值。
    pub fn create_empty_context() -> SystemContext {
        SystemContext::new()
    }

    /// 清空上下文中的所有队列
    ///
    /// # 参数
    /// - context: 要清空的系统上下文
    pub fn clear_context_queues(context: &mut SystemContext) {
        let mut interfaces = context.interfaces.lock().unwrap();
        for iface in interfaces.interfaces_mut() {
            iface.rxq.clear();
            iface.txq.clear();
        }
    }

    /// 清空上下文中的 ARP 缓存
    ///
    /// # 参数
    /// - context: 要清空的系统上下文
    pub fn clear_context_arp_cache(context: &SystemContext) {
        let mut arp_cache = context.arp_cache.lock().unwrap();
        arp_cache.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_context() {
        let ctx = GlobalStateManager::create_context();
        assert!(ctx.interface_count() > 0);
    }

    #[test]
    fn test_create_empty_context() {
        let ctx = GlobalStateManager::create_empty_context();
        assert_eq!(ctx.interface_count(), 0);
    }

    #[test]
    fn test_clear_context_queues() {
        let mut ctx = GlobalStateManager::create_context();
        // 注入一些测试报文（这里只是演示，实际注入需要更多代码）
        GlobalStateManager::clear_context_queues(&mut ctx);
        // 验证队列已清空
        let interfaces = ctx.interfaces.lock().unwrap();
        for iface in interfaces.interfaces() {
            assert!(iface.rxq.is_empty());
            assert!(iface.txq.is_empty());
        }
    }

    #[test]
    fn test_clear_context_arp_cache() {
        let ctx = GlobalStateManager::create_context();
        // 先添加一些 ARP 条目
        {
            let mut arp_cache = ctx.arp_cache.lock().unwrap();
            arp_cache.update_arp(
                0,
                crate::interface::Ipv4Addr::new(192, 168, 1, 1),
                crate::interface::MacAddr::new([0x00, 0x11, 0x22, 0x33, 0x44, 0x55]),
                crate::protocols::arp::ArpState::Reachable,
            );
        }

        // 清空 ARP 缓存
        GlobalStateManager::clear_context_arp_cache(&ctx);

        // 验证已清空
        let arp_cache = ctx.arp_cache.lock().unwrap();
        assert!(arp_cache.is_empty());
    }
}
