// Scheduler 模块集成测试（精简版）
//
// 核心功能测试：调度器基本运行

use core_net::scheduler::Scheduler;
use core_net::engine::PacketProcessor;
use core_net::testframework::GlobalStateManager;
use serial_test::serial;

// 测试1：调度器创建
#[test]
#[serial]
fn test_scheduler_creation() {
    Scheduler::new("test".to_string());

    // 调度器创建成功
}

// 测试2：调度器设置上下文
#[test]
#[serial]
fn test_scheduler_with_context() {
    let ctx = GlobalStateManager::create_context();
    Scheduler::new("test".to_string()).with_context(ctx);

    // 上下文设置成功
}

// 测试3：调度器设置处理器
#[test]
#[serial]
fn test_scheduler_with_processor() {
    let ctx = GlobalStateManager::create_context();
    let processor = PacketProcessor::with_context(ctx);
    Scheduler::new("test".to_string()).with_processor(processor);

    // 处理器设置成功
}
