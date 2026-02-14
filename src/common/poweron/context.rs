/// 系统上下文，持有队列资源的所有权
use crate::common::queue::RingQueue;
use crate::common::packet::Packet;

/// 系统上下文
///
/// 持有接收队列和发送队列的所有权
pub struct SystemContext {
    /// 接收队列（注入器 -> 处理线程）
    pub rxq: RingQueue<Packet>,

    /// 发送队列（处理线程 -> 输出）
    pub txq: RingQueue<Packet>,
}

impl SystemContext {
    /// 创建新的系统上下文
    ///
    /// # 参数
    /// - `rxq_capacity`: 接收队列容量
    /// - `txq_capacity`: 发送队列容量
    pub fn new(rxq_capacity: usize, txq_capacity: usize) -> Self {
        SystemContext {
            rxq: RingQueue::new(rxq_capacity),
            txq: RingQueue::new(txq_capacity),
        }
    }

    // ========== 辅助接口 ==========

    /// 接收队列当前长度
    pub fn rxq_len(&self) -> usize {
        self.rxq.len()
    }

    /// 发送队列当前长度
    pub fn txq_len(&self) -> usize {
        self.txq.len()
    }

    /// 接收队列是否为空
    pub fn rxq_is_empty(&self) -> bool {
        self.rxq.is_empty()
    }

    /// 发送队列是否为空
    pub fn txq_is_empty(&self) -> bool {
        self.txq.is_empty()
    }

    /// 接收队列是否已满
    pub fn rxq_is_full(&self) -> bool {
        self.rxq.is_full()
    }

    /// 发送队列是否已满
    pub fn txq_is_full(&self) -> bool {
        self.txq.is_full()
    }
}
