/// 上电启动模块
///
/// 负责系统资源的初始化和释放
///
/// 注意：接口配置文件路径由 interface 模块自己管理
use crate::context::SystemContext;

/// 下电释放 - 清空所有接口的队列并释放内存
pub fn shutdown(context: &SystemContext) {
    let mut guard = match context.interfaces.lock() {
        Ok(g) => g,
        Err(_) => return,
    };

    for iface in guard.interfaces_mut() {
        iface.rxq.clear();
        iface.txq.clear();
    }
}

/// 使用默认配置启动系统
pub fn boot_default() -> SystemContext {
    SystemContext::from_config()
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
    fn inject_packets_to_rxq(context: &SystemContext, iface_name: &str, count: usize) {
        let mut guard = match context.interfaces.lock() {
            Ok(g) => g,
            Err(_) => return,
        };

        if let Ok(iface) = guard.get_by_name_mut(iface_name) {
            for _ in 0..count {
                let packet = create_test_packet();
                let _ = iface.rxq.enqueue(packet);
            }
        }
    }

    /// 向接口的 TxQ 注入报文
    fn inject_packets_to_txq(context: &SystemContext, iface_name: &str, count: usize) {
        let mut guard = match context.interfaces.lock() {
            Ok(g) => g,
            Err(_) => return,
        };

        if let Ok(iface) = guard.get_by_name_mut(iface_name) {
            for _ in 0..count {
                let packet = create_test_packet();
                let _ = iface.txq.enqueue(packet);
            }
        }
    }

    /// 计算所有接口的队列中报文总数
    fn count_all_packets(context: &SystemContext) -> usize {
        let guard = match context.interfaces.lock() {
            Ok(g) => g,
            Err(_) => return 0,
        };

        let mut count = 0;
        for iface in guard.interfaces() {
            count += iface.rxq.len();
            count += iface.txq.len();
        }
        count
    }

    /// 获取接口属性
    fn get_interface_properties(context: &SystemContext, name: &str) -> Option<(String, MacAddr, Ipv4Addr, u16)> {
        let guard = context.interfaces.lock().ok()?;
        let iface = guard.get_by_name(name).ok()?;
        Some((
            iface.name().to_string(),
            iface.mac_addr,
            iface.ip_addr,
            iface.mtu,
        ))
    }

    /// 检查接口队列是否为空
    fn is_interface_queue_empty(context: &SystemContext, name: &str) -> bool {
        let guard = match context.interfaces.lock() {
            Ok(g) => g,
            Err(_) => return true,
        };

        if let Ok(iface) = guard.get_by_name(name) {
            iface.rxq.is_empty() && iface.txq.is_empty()
        } else {
            true
        }
    }

    // ========== boot_default() 测试组 ==========

    #[test]
    fn test_boot_default_success() {
        let context = boot_default();

        // 验证上下文创建成功
        assert!(context.interface_count() > 0);

        // 验证接口队列为空
        assert!(is_interface_queue_empty(&context, "eth0"));
    }

    #[test]
    fn test_boot_default_loads_interfaces() {
        let context = boot_default();

        // 根据默认配置文件，应该至少有一个接口
        assert!(context.interface_count() >= 1);

        // 验证每个接口都有独立的队列
        let guard = context.interfaces.lock().unwrap();
        for iface in guard.interfaces() {
            // 验证队列存在且为空
            assert!(iface.rxq.is_empty());
            assert!(iface.txq.is_empty());
        }
    }

    #[test]
    fn test_boot_default_interface_properties() {
        let context = boot_default();

        // 验证 eth0 接口属性（如果存在）
        if let Some((name, mac, ip, mtu)) = get_interface_properties(&context, "eth0") {
            assert_eq!(name, "eth0");
            assert_eq!(mac, MacAddr::new([0x00, 0x11, 0x22, 0x33, 0x44, 0x55]));
            assert_eq!(ip, Ipv4Addr::new(192, 168, 1, 100));
            assert_eq!(mtu, 1500);
        }

        // 验证 lo 接口属性（如果存在）
        if let Some((name, mac, ip, _)) = get_interface_properties(&context, "lo") {
            assert_eq!(name, "lo");
            assert_eq!(mac, MacAddr::zero());
            assert_eq!(ip, Ipv4Addr::new(127, 0, 0, 1));
        }
    }

    #[test]
    fn test_boot_default_initializes_queues() {
        let context = boot_default();

        // 验证所有接口的队列都已初始化
        let guard = context.interfaces.lock().unwrap();
        for iface in guard.interfaces() {
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
        let context = boot_default();

        // 向队列中注入报文
        inject_packets_to_rxq(&context, "eth0", 10);
        inject_packets_to_txq(&context, "eth0", 5);

        // 验证报文已注入
        assert!(count_all_packets(&context) > 0);

        // 调用 shutdown
        shutdown(&context);

        // 验证所有队列已清空
        assert_eq!(count_all_packets(&context), 0);
    }

    #[test]
    fn test_shutdown_empty_context() {
        let context = SystemContext::new();

        // 空上下文的 shutdown 不应 panic
        shutdown(&context);

        // 验证仍然为空
        assert_eq!(context.interface_count(), 0);
    }

    #[test]
    fn test_shutdown_multiple_interfaces() {
        let context = boot_default();

        // 向多个接口的队列注入报文
        if context.interface_count() >= 2 {
            inject_packets_to_rxq(&context, "eth0", 5);
            inject_packets_to_txq(&context, "eth0", 3);
            inject_packets_to_rxq(&context, "lo", 7);
            inject_packets_to_txq(&context, "lo", 2);
        }

        // 调用 shutdown
        shutdown(&context);

        // 验证所有队列已清空
        let guard = context.interfaces.lock().unwrap();
        for iface in guard.interfaces() {
            assert!(iface.rxq.is_empty());
            assert!(iface.txq.is_empty());
        }
    }

    #[test]
    fn test_shutdown_idempotent() {
        let context = boot_default();

        // 第一次 shutdown
        shutdown(&context);

        // 第二次 shutdown 不应 panic
        shutdown(&context);

        // 验证队列仍然为空
        assert_eq!(count_all_packets(&context), 0);
    }

    #[test]
    fn test_shutdown_with_full_queue() {
        let context = boot_default();

        // 填满一个队列
        {
            let mut guard = context.interfaces.lock().unwrap();
            if let Ok(iface) = guard.get_by_name_mut("eth0") {
                let capacity = iface.rxq.capacity();
                for _ in 0..capacity {
                    let packet = create_test_packet();
                    let _ = iface.rxq.enqueue(packet);
                }
                assert!(iface.rxq.is_full());
            }
        }

        // 调用 shutdown
        shutdown(&context);

        // 验证队列已清空
        assert!(is_interface_queue_empty(&context, "eth0"));
    }

    #[test]
    fn test_shutdown_memory_release() {
        let context = boot_default();

        // 注入大量报文
        inject_packets_to_rxq(&context, "eth0", 100);
        inject_packets_to_txq(&context, "eth0", 100);

        // 调用 shutdown
        shutdown(&context);

        // 验证队列已清空（内存已释放）
        assert_eq!(count_all_packets(&context), 0);
    }

    // ========== 集成测试组 ==========

    #[test]
    fn test_boot_shutdown_cycle() {
        // 完整的上电下电循环
        let context = boot_default();

        // 验证系统正常运行
        assert!(context.interface_count() > 0);

        // 注入一些报文
        inject_packets_to_rxq(&context, "eth0", 5);

        // 下电
        shutdown(&context);

        // 验证资源已清理
        assert_eq!(count_all_packets(&context), 0);
    }

    #[test]
    fn test_boot_modify_shutdown() {
        let context = boot_default();

        // 修改接口配置
        {
            let mut guard = context.interfaces.lock().unwrap();
            if let Ok(iface) = guard.get_by_name_mut("eth0") {
                iface.set_ip_addr(Ipv4Addr::new(10, 0, 0, 1));
            }
        }

        // 验证修改生效
        if let Some((_, _, ip, _)) = get_interface_properties(&context, "eth0") {
            assert_eq!(ip, Ipv4Addr::new(10, 0, 0, 1));
        }

        // 下电
        shutdown(&context);
    }

    #[test]
    fn test_multiple_boot_cycles() {
        // 多次上电下电循环
        for _ in 0..3 {
            let context = boot_default();
            assert!(context.interface_count() > 0);
            shutdown(&context);
        }
    }
}
