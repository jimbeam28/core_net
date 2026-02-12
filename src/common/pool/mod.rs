// src/common/pool/mod.rs
//
// 通用对象池模块导出

mod clear;
mod config;
mod error;
mod packet_pool;
mod pooled;
mod pool;
mod stats;

pub use clear::Clear;
pub use config::{AllocStrategy, PoolConfig, WaitStrategy};
pub use error::PoolError;
pub use packet_pool::{PacketPool, PacketPoolConfig, PacketPoolStats, PacketBuilder, PooledPacket};
pub use pooled::Pooled;
pub use pool::Pool;
pub use stats::{PoolStats, PoolStatus};

// ========== 单元测试 ==========

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_module_exports() {
        // 测试类型是否正确导出
        let _config = PoolConfig::default();
        let _strategy: AllocStrategy = AllocStrategy::Fifo;
        let _wait: WaitStrategy = WaitStrategy::Immediate;
    }
}
