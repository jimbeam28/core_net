/// 上电启动模块
///
/// 负责系统资源的初始化和释放
///
/// 注意：接口配置文件路径由 interface 模块自己管理

mod context;

pub use context::SystemContext;

/// 下电释放
///
/// 释放系统资源（清空所有接口的队列并释放内存）
///
/// # 行为
/// 1. 清空所有接口的接收队列，丢弃所有未处理的报文
/// 2. 清空所有接口的发送队列，丢弃所有未发送的报文
/// 3. 每个 Packet 被 drop，释放其持有的 buffer 内存
///
/// # 参数
/// - `context`: 可变引用的系统上下文
pub fn shutdown(context: &mut SystemContext) {
    // 清空所有接口的队列
    for iface in context.interfaces.interfaces_mut() {
        iface.rxq.clear();
        iface.txq.clear();
    }
}

/// 系统启动
///
/// 使用默认配置启动系统
///
/// # 返回
/// 包含接口管理器的 SystemContext
///
/// # 行为
/// 1. 从默认配置文件加载接口配置（由 interface 模块管理）
/// 2. 初始化全局接口管理器
pub fn boot_default() -> SystemContext {
    SystemContext::new()
}

// ========== 单元测试 ==========

#[cfg(test)]
mod tests {
    use super::*;
    use crate::interface::{MacAddr, Ipv4Addr};
    use crate::protocols::Packet;

    // ========== 测试辅助函数 ==========

    /// 创建测试用报文
    fn create_test_packet() -> Packet {
        Packet::from_bytes(vec![0x01, 0x02, 0x03, 0x04, 0x05, 0x06])
    }

    /// 向接口的 RxQ 注入报文
    fn inject_packets_to_rxq(context: &mut SystemContext, iface_name: &str, count: usize) {
        if let Some(iface) = context.get_interface_mut(iface_name) {
            for _ in 0..count {
                let packet = create_test_packet();
                let _ = iface.rxq.enqueue(packet);
            }
        }
    }

    /// 向接口的 TxQ 注入报文
    fn inject_packets_to_txq(context: &mut SystemContext, iface_name: &str, count: usize) {
        if let Some(iface) = context.get_interface_mut(iface_name) {
            for _ in 0..count {
                let packet = create_test_packet();
                let _ = iface.txq.enqueue(packet);
            }
        }
    }

    /// 计算所有接口的队列中报文总数
    fn count_all_packets(context: &SystemContext) -> usize {
        let mut count = 0;
        for iface in context.interfaces.interfaces() {
            count += iface.rxq.len();
            count += iface.txq.len();
        }
        count
    }

    // ========== boot_default() 测试组 ==========

    #[test]
    fn test_boot_default_success() {
        let context = boot_default();

        // 验证上下文创建成功
        assert!(context.interface_count() > 0);

        // 验证接口具有预期的属性
        if let Some(eth0) = context.get_interface("eth0") {
            assert_eq!(eth0.name(), "eth0");
            assert!(eth0.rxq.is_empty());
            assert!(eth0.txq.is_empty());
        }
    }

    #[test]
    fn test_boot_default_loads_interfaces() {
        let context = boot_default();

        // 根据默认配置文件，应该至少有一个接口
        assert!(context.interface_count() >= 1);

        // 验证每个接口都有独立的队列
        for iface in context.interfaces.interfaces() {
            // 验证队列存在且为空
            assert!(iface.rxq.is_empty());
            assert!(iface.txq.is_empty());
        }
    }

    #[test]
    fn test_boot_default_interface_properties() {
        let context = boot_default();

        // 验证 eth0 接口属性（如果存在）
        if let Some(eth0) = context.get_interface("eth0") {
            assert_eq!(eth0.name(), "eth0");
            assert_eq!(eth0.mac_addr, MacAddr::new([0x00, 0x11, 0x22, 0x33, 0x44, 0x55]));
            assert_eq!(eth0.ip_addr, Ipv4Addr::new(192, 168, 1, 100));
            assert_eq!(eth0.netmask, Ipv4Addr::new(255, 255, 255, 0));
            assert_eq!(eth0.mtu, 1500);
        }

        // 验证 lo 接口属性（如果存在）
        if let Some(lo) = context.get_interface("lo") {
            assert_eq!(lo.name(), "lo");
            assert_eq!(lo.mac_addr, MacAddr::zero());
            assert_eq!(lo.ip_addr, Ipv4Addr::new(127, 0, 0, 1));
        }
    }

    #[test]
    fn test_boot_default_initializes_queues() {
        let context = boot_default();

        // 验证所有接口的队列都已初始化
        for iface in context.interfaces.interfaces() {
            // 队列容量应该非零
            assert!(iface.rxq.capacity() > 0);
            assert!(iface.txq.capacity() > 0);

            // 队列初始为空
            assert!(iface.rxq.is_empty());
            assert!(iface.txq.is_empty());
        }
    }

    #[test]
    fn test_boot_default_multiple_calls() {
        // 多次调用 boot_default() 应该都能成功
        let context1 = boot_default();
        let context2 = boot_default();

        // 两个上下文应该有相同数量的接口
        assert_eq!(context1.interface_count(), context2.interface_count());
    }

    // ========== shutdown() 测试组 ==========

    #[test]
    fn test_shutdown_clears_queues() {
        let mut context = boot_default();

        // 向队列中注入报文
        inject_packets_to_rxq(&mut context, "eth0", 10);
        inject_packets_to_txq(&mut context, "eth0", 5);

        // 验证报文已注入
        assert!(count_all_packets(&context) > 0);

        // 调用 shutdown
        shutdown(&mut context);

        // 验证所有队列已清空
        assert_eq!(count_all_packets(&context), 0);
    }

    #[test]
    fn test_shutdown_empty_context() {
        let mut context = SystemContext {
            interfaces: crate::interface::InterfaceManager::default(),
        };

        // 空上下文的 shutdown 不应 panic
        shutdown(&mut context);

        // 验证仍然为空
        assert_eq!(context.interface_count(), 0);
    }

    #[test]
    fn test_shutdown_multiple_interfaces() {
        let mut context = boot_default();

        // 向多个接口的队列注入报文
        if context.interface_count() >= 2 {
            inject_packets_to_rxq(&mut context, "eth0", 5);
            inject_packets_to_txq(&mut context, "eth0", 3);
            inject_packets_to_rxq(&mut context, "lo", 7);
            inject_packets_to_txq(&mut context, "lo", 2);
        }

        // 调用 shutdown
        shutdown(&mut context);

        // 验证所有队列已清空
        for iface in context.interfaces.interfaces() {
            assert!(iface.rxq.is_empty());
            assert!(iface.txq.is_empty());
        }
    }

    #[test]
    fn test_shutdown_idempotent() {
        let mut context = boot_default();

        // 第一次 shutdown
        shutdown(&mut context);

        // 第二次 shutdown 不应 panic
        shutdown(&mut context);

        // 验证队列仍然为空
        assert_eq!(count_all_packets(&context), 0);
    }

    #[test]
    fn test_shutdown_with_full_queue() {
        let mut context = boot_default();

        // 填满一个队列
        if let Some(iface) = context.get_interface_mut("eth0") {
            let capacity = iface.rxq.capacity();
            for _ in 0..capacity {
                let packet = create_test_packet();
                let _ = iface.rxq.enqueue(packet);
            }
            assert!(iface.rxq.is_full());
        }

        // 调用 shutdown
        shutdown(&mut context);

        // 验证队列已清空
        if let Some(iface) = context.get_interface("eth0") {
            assert!(iface.rxq.is_empty());
        }
    }

    #[test]
    fn test_shutdown_memory_release() {
        let mut context = boot_default();

        // 注入大量报文
        inject_packets_to_rxq(&mut context, "eth0", 100);
        inject_packets_to_txq(&mut context, "eth0", 100);

        // 调用 shutdown
        shutdown(&mut context);

        // 验证队列已清空（内存已释放）
        assert_eq!(count_all_packets(&context), 0);
    }

    // ========== 集成测试组 ==========

    #[test]
    fn test_boot_shutdown_cycle() {
        // 完整的上电下电循环
        let mut context = boot_default();

        // 验证系统正常运行
        assert!(context.interface_count() > 0);

        // 注入一些报文
        inject_packets_to_rxq(&mut context, "eth0", 5);

        // 下电
        shutdown(&mut context);

        // 验证资源已清理
        assert_eq!(count_all_packets(&context), 0);
    }

    #[test]
    fn test_boot_modify_shutdown() {
        let mut context = boot_default();

        // 修改接口配置
        if let Some(iface) = context.get_interface_mut("eth0") {
            iface.set_ip_addr(Ipv4Addr::new(10, 0, 0, 1));
        }

        // 验证修改生效
        if let Some(iface) = context.get_interface("eth0") {
            assert_eq!(iface.ip_addr, Ipv4Addr::new(10, 0, 0, 1));
        }

        // 下电
        shutdown(&mut context);
    }

    #[test]
    fn test_multiple_boot_cycles() {
        // 多次上电下电循环
        for _ in 0..3 {
            let mut context = boot_default();
            assert!(context.interface_count() > 0);
            shutdown(&mut context);
        }
    }
}
