/// 上电启动模块
///
/// 负责系统资源的初始化和释放

mod config;
mod context;

pub use config::SystemConfig;
pub use context::SystemContext;

/// 上电初始化
///
/// 使用指定配置初始化系统资源
///
/// # 参数
/// - `config`: 系统配置
///
/// # 返回
/// 包含队列资源的 SystemContext
pub fn boot(config: SystemConfig) -> SystemContext {
    SystemContext::new(config.rxq_capacity, config.txq_capacity)
}

/// 下电释放
///
/// 释放系统资源（清空队列并释放内存）
///
/// # 行为
/// 1. 清空接收队列，丢弃所有未处理的报文
/// 2. 清空发送队列，丢弃所有未发送的报文
/// 3. 每个 Packet 被 drop，释放其持有的 buffer 内存
///
/// # 参数
/// - `context`: 可变引用的系统上下文
pub fn shutdown(context: &mut SystemContext) {
    // 清空队列，遍历 buffer 并将每个 slot 设为 None
    // 这会触发 Packet 的 drop，释放 Vec<u8> 内存
    context.rxq.clear();
    context.txq.clear();
}

/// 快速启动 - 使用默认配置
///
/// 使用默认配置（256容量）快速启动系统
///
/// # 返回
/// 包含默认容量队列的 SystemContext
pub fn boot_default() -> SystemContext {
    boot(SystemConfig::default())
}

/// 快速启动 - 指定容量
///
/// 使用指定的接收和发送队列容量快速启动系统
///
/// # 参数
/// - `rxq_cap`: 接收队列容量
/// - `txq_cap`: 发送队列容量
///
/// # 返回
/// 包含指定容量队列的 SystemContext
pub fn boot_with_capacity(rxq_cap: usize, txq_cap: usize) -> SystemContext {
    boot(SystemConfig {
        rxq_capacity: rxq_cap,
        txq_capacity: txq_cap,
    })
}
