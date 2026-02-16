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
