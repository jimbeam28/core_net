// src/scheduler/scheduler.rs
//
// 调度器实现
// 负责从接收队列中取出报文并调度给协议处理引擎

use crate::common::queue::RingQueue;
use crate::common::Packet;
use crate::engine::PacketProcessor;
use crate::interface::InterfaceManager;

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

// ========== 错误转换 ==========

/// 从 CoreError 转换
impl From<crate::common::CoreError> for ScheduleError {
    fn from(err: crate::common::CoreError) -> Self {
        match err {
            crate::common::CoreError::QueueFull => {
                ScheduleError::QueueError("队列已满".to_string())
            }
            crate::common::CoreError::QueueEmpty => {
                ScheduleError::QueueError("队列为空".to_string())
            }
            _ => ScheduleError::Other(format!("{:?}", err)),
        }
    }
}

/// 从 ProcessError 转换
impl From<crate::engine::ProcessError> for ScheduleError {
    fn from(err: crate::engine::ProcessError) -> Self {
        ScheduleError::ProcessorError(err.to_string())
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
    /// 如果协议处理返回响应报文（如 ARP Reply），将其放入发送队列。
    ///
    /// # 参数
    /// - `rxq`: 接收队列的可变引用
    /// - `txq`: 发送队列的可变引用（用于接收响应报文）
    ///
    /// # 行为
    /// 1. 循环从 rxq 中尝试出队
    /// 2. 若队列为空（QueueError::Empty），退出循环
    /// 3. 若成功取出报文，调用 processor.process() 处理
    /// 4. 若返回响应报文，将其放入 txq
    /// 5. 处理失败仅记录，不中断调度
    ///
    /// # 返回
    /// - `Ok(count)`: 成功处理的报文数量
    /// - `Err(ScheduleError)`: 调度过程中发生严重错误
    pub fn run(&self, rxq: &mut RingQueue<Packet>, txq: &mut RingQueue<Packet>) -> ScheduleResult<usize> {
        let mut count = 0;

        if self.verbose {
            println!("=== 调度器 [{}] 开始运行 ===", self.name);
        }

        loop {
            match rxq.dequeue() {
                Some(packet) => {
                    // 根据是否有自定义处理器选择处理方式
                    let result = match &self.processor {
                        Some(processor) => processor.process(packet),
                        None => PacketProcessor::new().process(packet),
                    };

                    // 处理结果
                    match result {
                        Ok(response) => {
                            count += 1;
                            // 如果返回响应报文，放入 TxQ
                            if let Some(response_packet) = response {
                                if let Err(_) = txq.enqueue(response_packet) {
                                    if self.verbose {
                                        println!("警告: TxQ 已满，响应报文丢失");
                                    }
                                } else if self.verbose {
                                    println!("响应报文已放入 TxQ");
                                }
                            }
                        }
                        Err(e) => {
                            // 处理失败，记录但继续处理后续报文
                            if self.verbose {
                                println!("报文处理失败: {}", e);
                            }
                        }
                    }
                }
                None => {
                    // 队列为空，正常退出循环
                    break;
                }
            }
        }

        if self.verbose {
            println!("=== 调度器 [{}] 完成，处理了 {} 个报文 ===", self.name, count);
        }

        Ok(count)
    }

    /// 运行调度循环，遍历所有接口的接收队列
    ///
    /// 从所有接口的接收队列中取出报文进行处理，直到所有队列为空。
    /// 如果协议处理返回响应报文，将其放入对应接口的发送队列。
    ///
    /// # 参数
    /// - `interfaces`: 接口管理器的可变引用
    ///
    /// # 行为
    /// 1. 遍历所有接口
    /// 2. 对每个接口的接收队列循环出队
    /// 3. 若队列为空，继续处理下一个接口
    /// 4. 若成功取出报文，调用 processor.process() 处理
    /// 5. 若返回响应报文，将其放入该接口的 txq
    /// 6. 处理失败仅记录，不中断调度
    ///
    /// # 返回
    /// - `Ok(count)`: 成功处理的报文总数
    /// - `Err(ScheduleError)`: 调度过程中发生严重错误
    pub fn run_all_interfaces(&self, interfaces: &mut InterfaceManager) -> ScheduleResult<usize> {
        let mut total_count = 0;

        if self.verbose {
            println!("=== 调度器 [{}] 开始运行（多接口模式）===", self.name);
            println!("接口数量: {}", interfaces.len());
        }

        // 遍历所有接口
        for index in 0..interfaces.len() {
            if let Ok(iface) = interfaces.get_by_index_mut(index as u32) {
                if self.verbose {
                    println!("--- 处理接口 [{}] ({}) ---", iface.index, iface.name);
                }

                let mut iface_count = 0;
                loop {
                    match iface.rxq.dequeue() {
                        Some(packet) => {
                            // 根据是否有自定义处理器选择处理方式
                            let result = match &self.processor {
                                Some(processor) => processor.process(packet),
                                None => PacketProcessor::new().process(packet),
                            };

                            // 处理结果
                            match result {
                                Ok(response) => {
                                    iface_count += 1;
                                    // 如果返回响应报文，放入该接口的 TxQ
                                    if let Some(response_packet) = response {
                                        if let Err(_) = iface.txq.enqueue(response_packet) {
                                            if self.verbose {
                                                println!("  警告: 接口 [{}] TxQ 已满，响应报文丢失", iface.name);
                                            }
                                        } else if self.verbose {
                                            println!("  响应报文已放入接口 [{}] TxQ", iface.name);
                                        }
                                    }
                                }
                                Err(e) => {
                                    // 处理失败，记录但继续处理后续报文
                                    if self.verbose {
                                        println!("  报文处理失败: {}", e);
                                    }
                                }
                            }
                        }
                        None => {
                            // 队列为空，处理下一个接口
                            break;
                        }
                    }
                }

                if self.verbose {
                    println!("--- 接口 [{}] 处理完成，处理了 {} 个报文 ---", iface.name, iface_count);
                }

                total_count += iface_count;
            }
        }

        if self.verbose {
            println!("=== 调度器 [{}] 完成，共处理了 {} 个报文 ===", self.name, total_count);
        }

        Ok(total_count)
    }
}

// ========== 便捷函数 ==========

/// 使用默认调度器处理接收队列
///
/// # 参数
/// - `rxq`: 接收队列的可变引用
/// - `txq`: 发送队列的可变引用（用于接收响应报文）
///
/// # 返回
/// - `Ok(count)`: 成功处理的报文数量
/// - `Err(ScheduleError)`: 调度失败
pub fn schedule_packets(rxq: &mut RingQueue<Packet>, txq: &mut RingQueue<Packet>) -> ScheduleResult<usize> {
    Scheduler::new("DefaultScheduler".to_string()).run(rxq, txq)
}

/// 使用详细输出模式调度
///
/// # 参数
/// - `rxq`: 接收队列的可变引用
/// - `txq`: 发送队列的可变引用（用于接收响应报文）
///
/// # 返回
/// - `Ok(count)`: 成功处理的报文数量
/// - `Err(ScheduleError)`: 调度失败
pub fn schedule_packets_verbose(rxq: &mut RingQueue<Packet>, txq: &mut RingQueue<Packet>) -> ScheduleResult<usize> {
    Scheduler::new("VerboseScheduler".to_string())
        .with_verbose(true)
        .run(rxq, txq)
}
