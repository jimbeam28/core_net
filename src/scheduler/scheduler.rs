// src/scheduler/scheduler.rs
//
// 调度器实现
// 负责从接收队列中取出报文并调度给协议处理引擎

use crate::common::queue::{RingQueue, QueueError};
use crate::common::Packet;
use crate::engine::PacketProcessor;

// ========== 错误类型定义 ==========

/// 调度错误
#[derive(Debug)]
pub enum ScheduleError {
    /// 队列操作失败
    QueueError(String),

    /// 处理器错误
    ProcessorError(String),

    /// 其他错误
    Other(String),
}

impl std::fmt::Display for ScheduleError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ScheduleError::QueueError(msg) => write!(f, "队列错误: {}", msg),
            ScheduleError::ProcessorError(msg) => write!(f, "处理器错误: {}", msg),
            ScheduleError::Other(msg) => write!(f, "其他错误: {}", msg),
        }
    }
}

impl std::error::Error for ScheduleError {}

/// 从 QueueError 转换
impl From<QueueError> for ScheduleError {
    fn from(err: QueueError) -> Self {
        match err {
            QueueError::Empty => {
                // 队列空是正常退出条件，不应该转换为错误
                ScheduleError::Other("意外的队列空错误".to_string())
            }
            QueueError::Full => ScheduleError::QueueError("队列已满".to_string()),
        }
    }
}

/// 调度结果类型
pub type ScheduleResult<T> = Result<T, ScheduleError>;

// ========== Scheduler 调度器 ==========

/// 调度器
///
/// 负责从接收队列持续取出报文并调度给协议处理引擎。
pub struct Scheduler {
    /// 调度器名称
    name: String,

    /// 协议处理器
    processor: Option<PacketProcessor>,

    /// 是否启用详细输出
    verbose: bool,
}

impl Scheduler {
    /// 创建新的调度器
    ///
    /// # 参数
    /// - `name`: 调度器名称
    ///
    /// # 返回
    /// 新的 Scheduler 实例
    pub fn new(name: String) -> Self {
        Self {
            name,
            processor: None,
            verbose: false,
        }
    }

    /// 设置协议处理器
    ///
    /// # 参数
    /// - `processor`: 协议处理器实例
    pub fn with_processor(mut self, processor: PacketProcessor) -> Self {
        self.processor = Some(processor);
        self
    }

    /// 启用详细输出
    ///
    /// # 参数
    /// - `verbose`: 是否启用详细输出
    pub fn with_verbose(mut self, verbose: bool) -> Self {
        self.verbose = verbose;
        self
    }

    /// 运行调度循环
    ///
    /// 从接收队列中持续取出报文进行处理，直到队列为空。
    ///
    /// # 参数
    /// - `rxq`: 接收队列的可变引用
    ///
    /// # 行为
    /// 1. 循环从 rxq 中尝试出队
    /// 2. 若队列为空（QueueError::Empty），退出循环
    /// 3. 若成功取出报文，调用 processor.process() 处理
    /// 4. 处理结果仅记录，不中断调度
    ///
    /// # 返回
    /// - `Ok(count)`: 成功处理的报文数量
    /// - `Err(ScheduleError)`: 调度过程中发生严重错误
    pub fn run(&self, rxq: &mut RingQueue<Packet>) -> ScheduleResult<usize> {
        let mut count = 0;

        if self.verbose {
            println!("=== 调度器 [{}] 开始运行 ===", self.name);
        }

        loop {
            match rxq.dequeue() {
                Ok(Some(packet)) => {
                    // 根据是否有自定义处理器选择处理方式
                    let result = match &self.processor {
                        Some(processor) => processor.process(packet),
                        None => PacketProcessor::new().process(packet),
                    };

                    // 处理失败，记录但继续处理后续报文
                    if let Err(e) = result {
                        if self.verbose {
                            println!("报文处理失败: {}", e);
                        }
                    } else {
                        count += 1;
                    }
                }
                Ok(None) => {
                    // 不应该出现这种情况，dequeue 返回 None 只在 buffer 有 None 占位时
                    // 这属于内部错误
                    return Err(ScheduleError::Other(
                        "队列内部状态错误: dequeue 返回 None".to_string()
                    ));
                }
                Err(QueueError::Empty) => {
                    // 队列为空，正常退出循环
                    break;
                }
                Err(QueueError::Full) => {
                    // dequeue 不应该返回 Full 错误
                    return Err(ScheduleError::Other(
                        "队列内部状态错误: dequeue 返回 Full".to_string()
                    ));
                }
            }
        }

        if self.verbose {
            println!("=== 调度器 [{}] 完成，处理了 {} 个报文 ===", self.name, count);
        }

        Ok(count)
    }
}

// ========== 便捷函数 ==========

/// 使用默认调度器处理接收队列
///
/// # 参数
/// - `rxq`: 接收队列的可变引用
///
/// # 返回
/// - `Ok(count)`: 成功处理的报文数量
/// - `Err(ScheduleError)`: 调度失败
pub fn schedule_packets(rxq: &mut RingQueue<Packet>) -> ScheduleResult<usize> {
    Scheduler::new("DefaultScheduler".to_string()).run(rxq)
}

/// 使用详细输出模式调度
///
/// # 参数
/// - `rxq`: 接收队列的可变引用
///
/// # 返回
/// - `Ok(count)`: 成功处理的报文数量
/// - `Err(ScheduleError)`: 调度失败
pub fn schedule_packets_verbose(rxq: &mut RingQueue<Packet>) -> ScheduleResult<usize> {
    Scheduler::new("VerboseScheduler".to_string())
        .with_verbose(true)
        .run(rxq)
}
