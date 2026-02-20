//! 测试框架主实现
//!
//! 提供报文注入和调度执行的通用测试基础设施。

use crate::interface::InterfaceManager;
use crate::scheduler::Scheduler;
use crate::engine::PacketProcessor;
use crate::context::SystemContext;
use crate::testframework::{
    error::{HarnessError, HarnessResult},
    injector::PacketInjector,
};

/// 测试框架
///
/// 提供报文注入和调度执行的通用测试基础设施。
pub struct TestHarness {
    /// 系统上下文（包含接口管理器、ARP缓存等）
    context: Option<SystemContext>,

    /// 接口管理器（直接使用时，不通过context）
    interfaces: InterfaceManager,

    /// 调度器
    scheduler: Scheduler,

    /// 协议处理器
    processor: PacketProcessor,

    /// 是否启用详细输出
    verbose: bool,

    /// 是否使用系统上下文
    use_context: bool,
}

impl TestHarness {
    /// 创建新的测试框架（推荐方式：使用 with_context）
    ///
    /// # 参数
    /// - rxq_capacity: 接收队列容量
    /// - txq_capacity: 发送队列容量
    ///
    /// # 返回
    /// 新的 TestHarness 实例
    ///
    /// # 注意
    /// 此方法保留用于向后兼容，推荐使用 `with_context` 方法
    pub fn new(rxq_capacity: usize, txq_capacity: usize) -> Self {
        let interfaces = InterfaceManager::new(rxq_capacity, txq_capacity);
        let ctx = SystemContext::new();
        let processor = PacketProcessor::with_context(ctx.clone()).with_verbose(false);
        let scheduler = Scheduler::new("TestHarness".to_string())
            .with_processor(PacketProcessor::with_context(ctx.clone()).with_verbose(false))
            .with_context(ctx);

        Self {
            context: None,
            interfaces,
            scheduler,
            processor,
            verbose: false,
            use_context: false,
        }
    }

    /// 使用系统上下文创建测试框架（推荐方式）
    ///
    /// # 参数
    /// - context: 系统上下文
    ///
    /// # 返回
    /// 新的 TestHarness 实例
    ///
    /// # 示例
    /// ```ignore
    /// use core_net::testframework::{TestHarness, GlobalStateManager};
    ///
    /// let ctx = GlobalStateManager::create_context();
    /// let harness = TestHarness::with_context(ctx);
    /// ```
    pub fn with_context(context: SystemContext) -> Self {
        let processor = PacketProcessor::with_context(context.clone()).with_verbose(false);
        let scheduler = Scheduler::new("TestHarness".to_string())
            .with_processor(PacketProcessor::with_context(context.clone()).with_verbose(false))
            .with_context(context.clone())
            .with_verbose(false);

        Self {
            context: Some(context),
            interfaces: InterfaceManager::new(0, 0), // 不使用，容量设为0
            scheduler,
            processor,
            verbose: false,
            use_context: true,
        }
    }

    /// 启用详细输出
    ///
    /// # 参数
    /// - verbose: 是否启用详细输出
    pub fn with_verbose(mut self, verbose: bool) -> Self {
        self.verbose = verbose;
        if self.use_context {
            if let Some(ref ctx) = self.context {
                self.processor = PacketProcessor::with_context(ctx.clone()).with_verbose(verbose);
                self.scheduler = Scheduler::new("TestHarness".to_string())
                    .with_processor(PacketProcessor::with_context(ctx.clone()).with_verbose(verbose))
                    .with_context(ctx.clone())
                    .with_verbose(verbose);
            }
        } else {
            // 使用默认 SystemContext
            let ctx = SystemContext::new();
            self.processor = PacketProcessor::with_context(ctx.clone()).with_verbose(verbose);
            self.scheduler = Scheduler::new("TestHarness".to_string())
                .with_processor(PacketProcessor::with_context(ctx.clone()).with_verbose(verbose))
                .with_context(ctx)
                .with_verbose(verbose);
        }
        self
    }

    /// 获取报文注入器
    pub fn injector(&mut self) -> PacketInjector<'_> {
        if self.use_context {
            // 使用系统上下文创建注入器
            PacketInjector::with_context(self.context.as_ref().expect("context should be set"))
        } else {
            // 使用直接接口管理器创建注入器
            PacketInjector::new(&mut self.interfaces)
        }
    }

    /// 获取系统上下文的引用
    pub fn context(&self) -> Option<&SystemContext> {
        self.context.as_ref()
    }

    /// 获取系统上下文的可变引用
    pub fn context_mut(&mut self) -> Option<&mut SystemContext> {
        self.context.as_mut()
    }

    /// 运行调度器处理所有接口的报文
    ///
    /// # 返回
    /// - Ok(count): 成功处理的报文数量
    /// - Err(HarnessError): 调度失败
    pub fn run(&mut self) -> HarnessResult<usize> {
        if self.use_context {
            // 使用系统上下文：通过 scheduler 处理
            if let Some(ref ctx) = self.context {
                self.scheduler.run_all_interfaces_context(ctx)
                    .map_err(|e| HarnessError::SchedulerError(e.to_string()))
            } else {
                Err(HarnessError::SchedulerError("系统上下文未设置".to_string()))
            }
        } else {
            // 使用直接接口管理器
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
