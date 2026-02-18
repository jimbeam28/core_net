//! 测试框架模块
//!
//! 提供报文注入和调度执行的通用测试基础设施。
//!
//! # 设计原则
//! - 单一职责：只负责报文注入和调度执行
//! - 协议无关：不涉及具体协议的报文构造和验证
//! - 测试隔离：提供全局状态清理和初始化

mod error;
mod injector;
mod harness;
mod global_state;

// 导出公共接口
pub use error::{HarnessError, HarnessResult};
pub use injector::PacketInjector;
pub use harness::TestHarness;
pub use global_state::{GlobalStateManager, InterfaceTestConfig};
