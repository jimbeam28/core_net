//! 测试框架主实现
//!
//! 提供报文注入和调度执行的通用测试基础设施。

use crate::interface::InterfaceManager;
use crate::scheduler::Scheduler;
use crate::engine::PacketProcessor;
use crate::common::Packet;
use crate::testframework::{
    error::{HarnessError, HarnessResult},
    injector::PacketInjector,
};

/// 测试框架
///
/// 提供报文注入和调度执行的通用测试基础设施。
pub struct TestHarness {
    /// 接口管理器
    interfaces: InterfaceManager,

    /// 调度器
    scheduler: Scheduler,

    /// 协议处理器
    processor: PacketProcessor,

    /// 是否启用详细输出
    verbose: bool,

    /// 是否使用全局接口管理器
    use_global: bool,
}

impl TestHarness {
    /// 创建新的测试框架
    ///
    /// # 参数
    /// - rxq_capacity: 接收队列容量
    /// - txq_capacity: 发送队列容量
    ///
    /// # 返回
    /// 新的 TestHarness 实例
    pub fn new(rxq_capacity: usize, txq_capacity: usize) -> Self {
        let interfaces = InterfaceManager::new(rxq_capacity, txq_capacity);
        let processor = PacketProcessor::new().with_verbose(false);
        let scheduler = Scheduler::new("TestHarness".to_string())
            .with_processor(PacketProcessor::new().with_verbose(false));

        Self {
            interfaces,
            scheduler,
            processor,
            verbose: false,
            use_global: false,
        }
    }

    /// 创建使用全局接口管理器的测试框架
    ///
    /// # 返回
    /// 新的 TestHarness 实例，使用全局接口管理器
    pub fn with_global_manager() -> Self {
        let processor = PacketProcessor::new().with_verbose(false);
        let scheduler = Scheduler::new("TestHarness".to_string())
            .with_processor(PacketProcessor::new().with_verbose(false));

        Self {
            interfaces: InterfaceManager::new(0, 0), // 不使用，容量设为0
            scheduler,
            processor,
            verbose: false,
            use_global: true,
        }
    }

    /// 启用详细输出
    ///
    /// # 参数
    /// - verbose: 是否启用详细输出
    pub fn with_verbose(mut self, verbose: bool) -> Self {
        self.verbose = verbose;
        self.processor = PacketProcessor::new().with_verbose(verbose);
        self.scheduler = Scheduler::new("TestHarness".to_string())
            .with_processor(PacketProcessor::new().with_verbose(verbose))
            .with_verbose(verbose);
        self
    }

    /// 获取报文注入器
    pub fn injector(&mut self) -> PacketInjector<'_> {
        PacketInjector::new(&mut self.interfaces)
    }

    /// 运行调度器处理所有接口的报文
    ///
    /// # 返回
    /// - Ok(count): 成功处理的报文数量
    /// - Err(HarnessError): 调度失败
    pub fn run(&mut self) -> HarnessResult<usize> {
        if self.use_global {
            // 使用全局接口管理器时，需要避免死锁
            // 方案：先收集所有接口的报文，释放锁，处理报文，再获取锁放回响应
            use crate::interface::global_manager;
            let global_mgr = global_manager()
                .ok_or_else(|| HarnessError::SchedulerError("全局接口管理器未初始化".to_string()))?;

            // 第一步：收集所有接口的报文
            let mut packets_to_process: Vec<(u32, Packet)> = Vec::new();
            {
                let mut guard = global_mgr.lock()
                    .map_err(|e| HarnessError::SchedulerError(format!("锁定接口管理器失败: {}", e)))?;

                let len = guard.len();
                for index in 0..len {
                    if let Ok(iface) = guard.get_by_index_mut(index as u32) {
                        // 使用循环取出所有报文
                        while let Some(packet) = iface.rxq.dequeue() {
                            packets_to_process.push((iface.index, packet));
                        }
                    }
                }
            }

            // 第二步：处理报文（不持有全局接口管理器锁）
            let mut responses: Vec<(u32, Packet)> = Vec::new();
            for (ifindex, packet) in &packets_to_process {
                let mut packet = packet.clone();
                packet.set_ifindex(*ifindex);
                match self.processor.process(packet) {
                    Ok(Some(response_packet)) => {
                        responses.push((*ifindex, response_packet));
                    }
                    Ok(None) => {}
                    Err(_) => {}
                }
            }

            // 第三步：将响应放回队列
            let mut guard = global_mgr.lock()
                .map_err(|e| HarnessError::SchedulerError(format!("锁定接口管理器失败: {}", e)))?;

            for (ifindex, response_packet) in responses {
                if let Ok(iface) = guard.get_by_index_mut(ifindex) {
                    let _ = iface.txq.enqueue(response_packet);
                }
            }

            Ok(packets_to_process.len())
        } else {
            self.scheduler.run_all_interfaces(&mut self.interfaces)
                .map_err(|e| HarnessError::SchedulerError(e.to_string()))
        }
    }

    /// 获取接口管理器的引用
    pub fn interfaces(&self) -> &InterfaceManager {
        &self.interfaces
    }

    /// 获取接口管理器的可变引用
    pub fn interfaces_mut(&mut self) -> &mut InterfaceManager {
        &mut self.interfaces
    }
}
