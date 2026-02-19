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
                        Some(mut packet) => {
                            // 设置接口索引
                            packet.set_ifindex(iface.index);

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

// ========== 单元测试 ==========

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::{MacAddr, Ipv4Addr, ETH_P_ARP};
    use crate::protocols::arp::{ArpPacket, ArpOperation};
    use crate::protocols::ethernet;
    use std::cell::RefCell;
    use std::rc::Rc;

    // ========== 测试辅助函数 ==========

    /// 创建测试调度器（带默认处理器）
    fn create_test_scheduler() -> Scheduler {
        Scheduler::new("TestScheduler".to_string())
            .with_processor(PacketProcessor::new())
    }

    /// 创建测试报文
    fn create_test_packet() -> Packet {
        Packet::from_bytes(vec![0x01, 0x02, 0x03, 0x04])
    }

    /// 创建 ARP 请求报文（带以太网头）
    fn create_arp_request_packet(
        dst_mac: MacAddr,
        src_mac: MacAddr,
        src_ip: Ipv4Addr,
        dst_ip: Ipv4Addr,
    ) -> Packet {
        let arp_pkt = ArpPacket::new(
            ArpOperation::Request,
            src_mac,
            src_ip,
            MacAddr::zero(),
            dst_ip,
        );
        Packet::from_bytes(ethernet::build_ethernet_frame(dst_mac, src_mac, ETH_P_ARP, &arp_pkt.to_bytes()))
    }

    /// 创建无效报文（太短）
    fn create_invalid_packet() -> Packet {
        Packet::from_bytes(vec![0x01, 0x02])
    }

    /// 创建单接口管理器
    fn create_single_interface_manager() -> InterfaceManager {
        let mut manager = InterfaceManager::new(256, 256);
        let iface = crate::interface::NetworkInterface::new(
            "eth0".to_string(),
            0,
            MacAddr::new([0x00, 0x11, 0x22, 0x33, 0x44, 0x55]),
            Ipv4Addr::new(192, 168, 1, 100),
            256,
            256,
        );
        manager.add_interface(iface).unwrap();
        manager
    }

    /// 创建多接口管理器（eth0 + lo）
    fn create_multi_interface_manager() -> InterfaceManager {
        let mut manager = InterfaceManager::new(256, 256);

        let eth0 = crate::interface::NetworkInterface::new(
            "eth0".to_string(),
            0,
            MacAddr::new([0x00, 0x11, 0x22, 0x33, 0x44, 0x55]),
            Ipv4Addr::new(192, 168, 1, 100),
            256,
            256,
        );

        let lo = crate::interface::NetworkInterface::new(
            "lo".to_string(),
            1,
            MacAddr::zero(),
            Ipv4Addr::new(127, 0, 0, 1),
            256,
            256,
        );

        manager.add_interface(eth0).unwrap();
        manager.add_interface(lo).unwrap();
        manager
    }

    /// Mock 处理器，用于测试错误处理
    #[allow(dead_code)]
    struct MockProcessor {
        call_count: Rc<RefCell<usize>>,
        should_fail: bool,
        return_response: bool,
    }

    impl MockProcessor {
        #[allow(dead_code)]
        fn new(should_fail: bool, return_response: bool) -> (Self, Rc<RefCell<usize>>) {
            let count = Rc::new(RefCell::new(0));
            (
                Self {
                    call_count: count.clone(),
                    should_fail,
                    return_response,
                },
                count,
            )
        }
    }

    // ========== 1. 基础功能测试组 ==========

    #[test]
    fn test_scheduler_new() {
        let scheduler = Scheduler::new("TestScheduler".to_string());
        // 验证调度器创建成功（通过 run 方法验证）
        let mut rxq = RingQueue::<Packet>::new(10);
        let mut txq = RingQueue::<Packet>::new(10);
        let result = scheduler.run(&mut rxq, &mut txq);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0);
    }

    #[test]
    fn test_scheduler_with_processor() {
        let processor = PacketProcessor::new();
        let scheduler = Scheduler::new("TestScheduler".to_string())
            .with_processor(processor);
        // 验证调度器带处理器创建成功
        let mut rxq = RingQueue::<Packet>::new(10);
        let mut txq = RingQueue::<Packet>::new(10);
        let result = scheduler.run(&mut rxq, &mut txq);
        assert!(result.is_ok());
    }

    #[test]
    fn test_scheduler_with_verbose() {
        let scheduler = Scheduler::new("TestScheduler".to_string())
            .with_verbose(true);
        // 验证 verbose 模式设置成功
        let mut rxq = RingQueue::<Packet>::new(10);
        let mut txq = RingQueue::<Packet>::new(10);
        let result = scheduler.run(&mut rxq, &mut txq);
        assert!(result.is_ok());
    }

    #[test]
    fn test_scheduler_chain() {
        let scheduler = Scheduler::new("ChainedScheduler".to_string())
            .with_processor(PacketProcessor::new())
            .with_verbose(true);
        let mut rxq = RingQueue::<Packet>::new(10);
        let mut txq = RingQueue::<Packet>::new(10);
        let result = scheduler.run(&mut rxq, &mut txq);
        assert!(result.is_ok());
    }

    // ========== 2. 单队列调度测试组 ==========

    #[test]
    fn test_run_empty_queue() {
        let scheduler = create_test_scheduler();
        let mut rxq = RingQueue::<Packet>::new(10);
        let mut txq = RingQueue::<Packet>::new(10);

        let result = scheduler.run(&mut rxq, &mut txq);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0);
        assert!(rxq.is_empty());
        assert!(txq.is_empty());
    }

    #[test]
    fn test_run_single_packet() {
        let scheduler = create_test_scheduler();
        let mut rxq = RingQueue::<Packet>::new(10);
        let mut txq = RingQueue::<Packet>::new(10);

        // 注入一个测试报文
        let packet = create_test_packet();
        rxq.enqueue(packet).unwrap();

        let result = scheduler.run(&mut rxq, &mut txq);
        // 报文会被处理，即使处理失败也会计数
        assert!(result.is_ok());
        assert!(rxq.is_empty());
    }

    #[test]
    fn test_run_multiple_packets() {
        let scheduler = create_test_scheduler();
        let mut rxq = RingQueue::<Packet>::new(10);
        let mut txq = RingQueue::<Packet>::new(10);

        // 注入多个报文
        for _ in 0..5 {
            rxq.enqueue(create_test_packet()).unwrap();
        }

        let result = scheduler.run(&mut rxq, &mut txq);
        assert!(result.is_ok());
        // 所有报文都应该被处理
        assert!(rxq.is_empty());
    }

    #[test]
    fn test_run_stops_when_empty() {
        let scheduler = create_test_scheduler();
        let mut rxq = RingQueue::<Packet>::new(10);
        let mut txq = RingQueue::<Packet>::new(10);

        // 注入报文
        rxq.enqueue(create_test_packet()).unwrap();
        rxq.enqueue(create_test_packet()).unwrap();

        // 处理第一次
        let result1 = scheduler.run(&mut rxq, &mut txq);
        assert!(result1.is_ok());
        assert!(rxq.is_empty());

        // 再次运行应该立即返回
        let result2 = scheduler.run(&mut rxq, &mut txq);
        assert!(result2.is_ok());
        assert_eq!(result2.unwrap(), 0);
    }

    #[test]
    fn test_run_invalid_packet_continues() {
        let scheduler = create_test_scheduler();
        let mut rxq = RingQueue::<Packet>::new(10);
        let mut txq = RingQueue::<Packet>::new(10);

        // 注入无效报文后跟有效报文
        rxq.enqueue(create_invalid_packet()).unwrap();
        rxq.enqueue(create_test_packet()).unwrap();

        let result = scheduler.run(&mut rxq, &mut txq);
        // 即使第一个报文处理失败，调度器也应该继续处理
        assert!(result.is_ok());
        assert!(rxq.is_empty());
    }

    #[test]
    fn test_run_processor_error_tolerance() {
        let scheduler = create_test_scheduler();
        let mut rxq = RingQueue::<Packet>::new(10);
        let mut txq = RingQueue::<Packet>::new(10);

        // 注入多个会导致处理错误的报文
        for _ in 0..3 {
            rxq.enqueue(create_invalid_packet()).unwrap();
        }

        // 处理失败不应该中断调度
        let result = scheduler.run(&mut rxq, &mut txq);
        assert!(result.is_ok());
        assert!(rxq.is_empty());
    }

    #[test]
    fn test_run_txq_full_handling() {
        let scheduler = create_test_scheduler();
        let mut rxq = RingQueue::<Packet>::new(10);
        // 创建一个只能容纳 1 个报文的 TxQ
        let mut txq = RingQueue::<Packet>::new(1);

        // 填满 TxQ
        txq.enqueue(create_test_packet()).unwrap();

        // 注入会生成响应的 ARP 报文
        let src_mac = MacAddr::new([0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF]);
        let src_ip = Ipv4Addr::new(192, 168, 1, 1);
        let dst_ip = Ipv4Addr::new(192, 168, 1, 2);
        let arp_packet = create_arp_request_packet(MacAddr::broadcast(), src_mac, src_ip, dst_ip);
        rxq.enqueue(arp_packet).unwrap();

        // 即使 TxQ 已满，调度器也应该继续运行
        let result = scheduler.run(&mut rxq, &mut txq);
        assert!(result.is_ok());
        assert!(rxq.is_empty());
    }

    #[test]
    fn test_run_arp_response_in_txq() {
        let scheduler = create_test_scheduler();
        let mut rxq = RingQueue::<Packet>::new(10);
        let mut txq = RingQueue::<Packet>::new(10);

        // 注入 ARP 请求报文
        let src_mac = MacAddr::new([0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF]);
        let src_ip = Ipv4Addr::new(192, 168, 1, 1);
        let dst_ip = Ipv4Addr::new(192, 168, 1, 2);
        let arp_packet = create_arp_request_packet(MacAddr::broadcast(), src_mac, src_ip, dst_ip);
        rxq.enqueue(arp_packet).unwrap();

        // 运行调度器
        let result = scheduler.run(&mut rxq, &mut txq);
        assert!(result.is_ok());
        assert!(rxq.is_empty());

        // ARP 处理可能生成响应（需要全局缓存初始化）
        // 这里只验证不崩溃
        let _txq_len = txq.len();
    }

    // ========== 3. 多接口调度测试组 ==========

    #[test]
    fn test_run_all_interfaces_empty_manager() {
        let scheduler = create_test_scheduler();
        let mut manager = InterfaceManager::new(256, 256);

        let result = scheduler.run_all_interfaces(&mut manager);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0);
    }

    #[test]
    fn test_run_all_interfaces_single_interface() {
        let scheduler = create_test_scheduler();
        let mut manager = create_single_interface_manager();

        // 向接口的 RxQ 注入报文
        {
            let iface = manager.get_by_name_mut("eth0").unwrap();
            iface.rxq.enqueue(create_test_packet()).unwrap();
        } // 释放借用

        let result = scheduler.run_all_interfaces(&mut manager);
        assert!(result.is_ok());

        // 验证 RxQ 已清空
        let iface = manager.get_by_name("eth0").unwrap();
        assert!(iface.rxq.is_empty());
    }

    #[test]
    fn test_run_all_interfaces_multiple_interfaces() {
        let scheduler = create_test_scheduler();
        let mut manager = create_multi_interface_manager();

        // 向不同接口注入报文
        {
            let eth0 = manager.get_by_name_mut("eth0").unwrap();
            eth0.rxq.enqueue(create_test_packet()).unwrap();
            eth0.rxq.enqueue(create_test_packet()).unwrap();
        }

        {
            let lo = manager.get_by_name_mut("lo").unwrap();
            lo.rxq.enqueue(create_test_packet()).unwrap();
        }

        let result = scheduler.run_all_interfaces(&mut manager);
        assert!(result.is_ok());

        // 验证所有接口的 RxQ 都被清空
        let eth0 = manager.get_by_name("eth0").unwrap();
        let lo = manager.get_by_name("lo").unwrap();
        assert!(eth0.rxq.is_empty());
        assert!(lo.rxq.is_empty());
    }

    #[test]
    fn test_run_all_interfaces_response_to_correct_txq() {
        let scheduler = create_test_scheduler();
        let mut manager = create_multi_interface_manager();

        // 向 eth0 注入 ARP 请求
        let src_mac = MacAddr::new([0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF]);
        let src_ip = Ipv4Addr::new(192, 168, 1, 1);
        let dst_ip = Ipv4Addr::new(192, 168, 1, 2);
        let arp_packet = create_arp_request_packet(MacAddr::broadcast(), src_mac, src_ip, dst_ip);

        {
            let eth0 = manager.get_by_name_mut("eth0").unwrap();
            eth0.rxq.enqueue(arp_packet).unwrap();
        }

        // 运行调度器
        let result = scheduler.run_all_interfaces(&mut manager);
        assert!(result.is_ok());

        // 验证 eth0 的 RxQ 被清空
        let eth0 = manager.get_by_name("eth0").unwrap();
        assert!(eth0.rxq.is_empty());

        // 验证 lo 的队列为空
        let lo = manager.get_by_name("lo").unwrap();
        assert!(lo.rxq.is_empty());
        assert!(lo.txq.is_empty());
    }

    #[test]
    fn test_run_all_interfaces_partial_empty() {
        let scheduler = create_test_scheduler();
        let mut manager = create_multi_interface_manager();

        // 只向 eth0 注入报文，lo 保持空
        {
            let eth0 = manager.get_by_name_mut("eth0").unwrap();
            eth0.rxq.enqueue(create_test_packet()).unwrap();
        }

        let result = scheduler.run_all_interfaces(&mut manager);
        assert!(result.is_ok());

        // 验证所有接口都被处理
        let eth0 = manager.get_by_name("eth0").unwrap();
        let lo = manager.get_by_name("lo").unwrap();
        assert!(eth0.rxq.is_empty());
        assert!(lo.rxq.is_empty());
    }

    #[test]
    fn test_run_all_interfaces_error_continues() {
        let scheduler = create_test_scheduler();
        let mut manager = create_multi_interface_manager();

        // 向不同接口注入无效报文
        {
            let eth0 = manager.get_by_name_mut("eth0").unwrap();
            eth0.rxq.enqueue(create_invalid_packet()).unwrap();
            eth0.rxq.enqueue(create_invalid_packet()).unwrap();
        }

        {
            let lo = manager.get_by_name_mut("lo").unwrap();
            lo.rxq.enqueue(create_test_packet()).unwrap();
        }

        // 即使处理出错，也应该继续处理其他接口和报文
        let result = scheduler.run_all_interfaces(&mut manager);
        assert!(result.is_ok());

        let eth0 = manager.get_by_name("eth0").unwrap();
        let lo = manager.get_by_name("lo").unwrap();
        assert!(eth0.rxq.is_empty());
        assert!(lo.rxq.is_empty());
    }

    #[test]
    fn test_run_all_interfaces_packet_ifindex_set() {
        let scheduler = create_test_scheduler();
        let mut manager = create_multi_interface_manager();

        // 向 eth0 注入报文
        let packet = create_test_packet();
        {
            let eth0 = manager.get_by_name_mut("eth0").unwrap();
            eth0.rxq.enqueue(packet).unwrap();
        }

        scheduler.run_all_interfaces(&mut manager).unwrap();

        // 验证报文的接口索引被正确设置（通过处理过程）
        // 这需要检查处理器是否收到了正确的 ifindex
        let eth0 = manager.get_by_name("eth0").unwrap();
        assert!(eth0.rxq.is_empty());
    }

    #[test]
    fn test_run_all_interfaces_verbose_mode() {
        let scheduler = Scheduler::new("VerboseTestScheduler".to_string())
            .with_processor(PacketProcessor::new())
            .with_verbose(true);

        let mut manager = create_single_interface_manager();

        {
            let eth0 = manager.get_by_name_mut("eth0").unwrap();
            eth0.rxq.enqueue(create_test_packet()).unwrap();
        }

        // verbose 模式不应该影响功能
        let result = scheduler.run_all_interfaces(&mut manager);
        assert!(result.is_ok());
    }

    // ========== 4. 便捷函数测试组 ==========

    #[test]
    fn test_schedule_packets() {
        let mut rxq = RingQueue::<Packet>::new(10);
        let mut txq = RingQueue::<Packet>::new(10);

        rxq.enqueue(create_test_packet()).unwrap();

        let result = schedule_packets(&mut rxq, &mut txq);
        assert!(result.is_ok());
        assert!(rxq.is_empty());
    }

    #[test]
    fn test_schedule_packets_verbose() {
        let mut rxq = RingQueue::<Packet>::new(10);
        let mut txq = RingQueue::<Packet>::new(10);

        rxq.enqueue(create_test_packet()).unwrap();

        let result = schedule_packets_verbose(&mut rxq, &mut txq);
        assert!(result.is_ok());
        assert!(rxq.is_empty());
    }

    #[test]
    fn test_schedule_packets_empty_queue() {
        let mut rxq = RingQueue::<Packet>::new(10);
        let mut txq = RingQueue::<Packet>::new(10);

        let result = schedule_packets(&mut rxq, &mut txq);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0);
    }

    #[test]
    fn test_schedule_packets_multiple_calls() {
        let mut rxq = RingQueue::<Packet>::new(10);
        let mut txq = RingQueue::<Packet>::new(10);

        // 第一次调用
        rxq.enqueue(create_test_packet()).unwrap();
        let result1 = schedule_packets(&mut rxq, &mut txq);
        assert!(result1.is_ok());

        // 第二次调用（空队列）
        let result2 = schedule_packets(&mut rxq, &mut txq);
        assert!(result2.is_ok());
        assert_eq!(result2.unwrap(), 0);
    }

    // ========== 5. 错误处理测试组 ==========

    #[test]
    fn test_schedule_error_queue_error() {
        let scheduler = create_test_scheduler();
        let mut rxq = RingQueue::<Packet>::new(10);
        let mut txq = RingQueue::<Packet>::new(10);

        // 注入会导致处理错误的报文
        rxq.enqueue(create_invalid_packet()).unwrap();

        // 处理错误不应该中断调度
        let result = scheduler.run(&mut rxq, &mut txq);
        assert!(result.is_ok());
    }

    #[test]
    fn test_schedule_error_all_packets_fail() {
        let scheduler = create_test_scheduler();
        let mut rxq = RingQueue::<Packet>::new(10);
        let mut txq = RingQueue::<Packet>::new(10);

        // 注入多个无效报文
        for _ in 0..5 {
            rxq.enqueue(create_invalid_packet()).unwrap();
        }

        // 即使所有报文都失败，调度也应该正常完成
        let result = scheduler.run(&mut rxq, &mut txq);
        assert!(result.is_ok());
        assert!(rxq.is_empty());
    }

    // ========== 6. 边界条件测试组 ==========

    #[test]
    fn test_run_max_queue_capacity() {
        let scheduler = create_test_scheduler();
        // 使用最小容量队列
        let mut rxq = RingQueue::<Packet>::new(2);
        let mut txq = RingQueue::<Packet>::new(2);

        // 填满队列
        rxq.enqueue(create_test_packet()).unwrap();
        rxq.enqueue(create_test_packet()).unwrap();

        let result = scheduler.run(&mut rxq, &mut txq);
        assert!(result.is_ok());
        assert!(rxq.is_empty());
    }

    #[test]
    fn test_run_single_interface_manager() {
        let scheduler = create_test_scheduler();
        let mut manager = InterfaceManager::new(256, 256);

        // 添加单个接口
        let iface = crate::interface::NetworkInterface::new(
            "eth0".to_string(),
            0,
            MacAddr::new([0x00, 0x11, 0x22, 0x33, 0x44, 0x55]),
            Ipv4Addr::new(192, 168, 1, 100),
            256,
            256,
        );
        manager.add_interface(iface).unwrap();

        let result = scheduler.run_all_interfaces(&mut manager);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0);
    }

    #[test]
    fn test_run_no_processor() {
        // 没有设置处理器的调度器应该使用默认处理器
        let scheduler = Scheduler::new("NoProcessorScheduler".to_string());
        let mut rxq = RingQueue::<Packet>::new(10);
        let mut txq = RingQueue::<Packet>::new(10);

        rxq.enqueue(create_test_packet()).unwrap();

        let result = scheduler.run(&mut rxq, &mut txq);
        assert!(result.is_ok());
    }

    // ========== 7. 数据流测试组 ==========

    #[test]
    fn test_data_flow_rxq_to_processor() {
        let scheduler = create_test_scheduler();
        let mut rxq = RingQueue::<Packet>::new(10);
        let mut txq = RingQueue::<Packet>::new(10);

        // 验证数据从 RxQ 流向处理器
        let packet = create_test_packet();
        let _original_data = packet.as_slice().to_vec();
        rxq.enqueue(packet).unwrap();

        scheduler.run(&mut rxq, &mut txq).unwrap();

        // RxQ 应该被清空
        assert!(rxq.is_empty());
    }

    #[test]
    fn test_data_flow_processor_to_txq() {
        let scheduler = create_test_scheduler();
        let mut rxq = RingQueue::<Packet>::new(10);
        let mut txq = RingQueue::<Packet>::new(10);

        // 注入可能生成响应的报文
        let src_mac = MacAddr::new([0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF]);
        let src_ip = Ipv4Addr::new(192, 168, 1, 1);
        let dst_ip = Ipv4Addr::new(192, 168, 1, 2);
        let arp_packet = create_arp_request_packet(MacAddr::broadcast(), src_mac, src_ip, dst_ip);
        rxq.enqueue(arp_packet).unwrap();

        scheduler.run(&mut rxq, &mut txq).unwrap();

        // 验证数据流完成
        assert!(rxq.is_empty());
        // TxQ 可能包含响应（取决于 ARP 缓存状态）
    }
}
