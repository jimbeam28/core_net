// Poweron 模块集成测试（精简版）
//
// 核心功能测试：系统启动和关闭

use core_net::context::SystemContext;
use core_net::interface::InterfaceState;
use serial_test::serial;

// 测试1：系统上下文创建
#[test]
#[serial]
fn test_context_creation() {
    let ctx = SystemContext::new();
    assert!(ctx.interface_count() >= 0);
}

// 测试2：从配置加载
#[test]
#[serial]
fn test_context_from_config() {
    let ctx = SystemContext::from_config();
    // 配置文件应该至少加载一个接口
    assert!(ctx.interface_count() > 0);
}

// 测试3：接口状态验证
#[test]
#[serial]
fn test_interface_state() {
    let ctx = SystemContext::from_config();
    let interfaces = ctx.interfaces.lock().unwrap();

    for iface in interfaces.interfaces() {
        assert!(!iface.name.is_empty());
        assert!(iface.rxq.capacity() > 0);
        assert!(iface.txq.capacity() > 0);
    }
}
