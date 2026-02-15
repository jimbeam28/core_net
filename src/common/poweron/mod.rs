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
/// 包含队列资源和接口管理器的 SystemContext
///
/// # 行为
/// 1. 创建接收和发送队列
/// 2. 从配置文件加载接口配置
/// 3. 初始化全局接口管理器
pub fn boot(config: SystemConfig) -> SystemContext {
    SystemContext::new(
        config.rxq_capacity,
        config.txq_capacity,
        &config.interface_config_path,
    )
}

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

/// 快速启动 - 使用默认配置
///
/// 使用默认配置（256容量，默认接口配置路径）快速启动系统
///
/// # 返回
/// 包含默认容量队列和接口管理器的 SystemContext
pub fn boot_default() -> SystemContext {
    boot(SystemConfig::default())
}

/// 快速启动 - 指定接口配置路径
///
/// 使用指定的接口配置路径和默认队列容量快速启动系统
///
/// # 参数
/// - `config_path`: 接口配置文件路径
///
/// # 返回
/// 包含接口管理器和默认容量队列的 SystemContext
pub fn boot_with_config(config_path: &str) -> SystemContext {
    boot(SystemConfig {
        interface_config_path: config_path.to_string(),
        rxq_capacity: 256,
        txq_capacity: 256,
    })
}

/// 快速启动 - 指定容量和接口配置路径
///
/// 使用指定的接口配置路径和队列容量快速启动系统
///
/// # 参数
/// - `config_path`: 接口配置文件路径
/// - `rxq_cap`: 每个接口的接收队列容量
/// - `txq_cap`: 每个接口的发送队列容量
///
/// # 返回
/// 包含接口管理器和指定容量队列的 SystemContext
pub fn boot_with_capacity(config_path: &str, rxq_cap: usize, txq_cap: usize) -> SystemContext {
    boot(SystemConfig {
        interface_config_path: config_path.to_string(),
        rxq_capacity: rxq_cap,
        txq_capacity: txq_cap,
    })
}
