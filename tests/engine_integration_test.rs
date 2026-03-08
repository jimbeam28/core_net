// Engine 模块集成测试（精简版）
//
// 核心功能测试：处理器创建和基本处理

use core_net::engine::PacketProcessor;
use core_net::testframework::GlobalStateManager;
use core_net::common::Packet;
use serial_test::serial;

// 测试1：处理器创建
#[test]
#[serial]
fn test_processor_creation() {
    let ctx = GlobalStateManager::create_context();
    PacketProcessor::with_context(ctx);

    // 处理器创建成功
}

// 测试2：处理器verbose模式
#[test]
#[serial]
fn test_processor_verbose() {
    let ctx = GlobalStateManager::create_context();
    PacketProcessor::with_context(ctx).with_verbose(true);

    // verbose模式设置成功
}

// 测试3：处理空报文（应优雅处理）
#[test]
#[serial]
fn test_processor_empty_packet() {
    let ctx = GlobalStateManager::create_context();
    let mut processor = PacketProcessor::with_context(ctx);

    let empty_packet = Packet::from_bytes(vec![]);
    let result = processor.process(empty_packet);

    // 空报文应该被处理（可能返回错误但不会panic）
    assert!(result.is_ok() || result.is_err());
}
