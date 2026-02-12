// src/common/queue.rs
//
// SPSC环形队列实现
// 单生产者单消费者无锁队列

use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

use crate::common::{CoreError, Result};

// ========== 常量定义 ==========

/// 默认队列容量
pub const DEFAULT_QUEUE_CAPACITY: usize = 256;

/// 最小队列容量
pub const MIN_QUEUE_CAPACITY: usize = 2;

/// 最大队列容量
pub const MAX_QUEUE_CAPACITY: usize = 65536;

/// 默认自旋次数
pub const DEFAULT_SPIN_COUNT: usize = 1000;

/// 默认超时时间（毫秒）
pub const DEFAULT_TIMEOUT_MS: u64 = 100;

// ========== 队列错误 ==========

/// 队列错误
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QueueError {
    /// 队列已满
    Full,
    /// 队列为空
    Empty,
    /// 队列已关闭
    Closed,
}

impl std::fmt::Display for QueueError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            QueueError::Full => write!(f, "队列已满"),
            QueueError::Empty => write!(f, "队列为空"),
            QueueError::Closed => write!(f, "队列已关闭"),
        }
    }
}

impl std::error::Error for QueueError {}

impl From<QueueError> for CoreError {
    fn from(err: QueueError) -> Self {
        match err {
            QueueError::Full => CoreError::QueueFull,
            QueueError::Empty => CoreError::QueueEmpty,
            QueueError::Closed => CoreError::Other("队列已关闭".to_string()),
        }
    }
}

// ========== 等待策略 ==========

/// 等待策略
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WaitStrategy {
    /// 自旋等待（CPU忙等待）
    Spin,

    /// 休眠等待（yield让出CPU）
    Yield,

    /// 超时后返回错误
    Timeout(Duration),
}

// ========== 队列配置 ==========

/// 队列配置
#[derive(Debug, Clone, Copy)]
pub struct QueueConfig {
    /// 队列容量（默认：256）
    pub capacity: usize,

    /// 是否启用阻塞模式（默认：true）
    pub blocking: bool,

    /// 等待策略
    pub wait_strategy: WaitStrategy,
}

impl Default for QueueConfig {
    fn default() -> Self {
        QueueConfig {
            capacity: DEFAULT_QUEUE_CAPACITY,
            blocking: true,
            wait_strategy: WaitStrategy::Yield,
        }
    }
}

// ========== RingQueue：非线程安全的环形队列 ==========

/// 环形队列（非线程安全）
///
/// 用于单线程场景下的环形缓冲区实现
pub struct RingQueue<T> {
    /// 环形缓冲区
    buffer: Vec<Option<T>>,

    /// 队列容量
    capacity: usize,

    /// 读指针（消费者位置）
    head: usize,

    /// 写指针（生产者位置）
    tail: usize,

    /// 当前元素数量
    count: usize,
}

impl<T> RingQueue<T> {
    /// 创建新的环形队列
    ///
    /// # 参数
    /// - `capacity`: 队列容量
    pub fn new(capacity: usize) -> Self {
        RingQueue {
            buffer: (0..capacity).map(|_| None).collect(),
            capacity,
            head: 0,
            tail: 0,
            count: 0,
        }
    }

    /// 入队（生产者）
    pub fn enqueue(&mut self, item: T) -> std::result::Result<(), QueueError> {
        if self.count >= self.capacity {
            return Err(QueueError::Full);
        }

        self.buffer[self.tail] = Some(item);
        self.tail = (self.tail + 1) % self.capacity;
        self.count += 1;

        Ok(())
    }

    /// 出队（消费者）
    pub fn dequeue(&mut self) -> std::result::Result<Option<T>, QueueError> {
        if self.count == 0 {
            return Err(QueueError::Empty);
        }

        let item = self.buffer[self.head].take();
        self.head = (self.head + 1) % self.capacity;
        self.count -= 1;

        Ok(item)
    }

    /// 队列是否为空
    pub fn is_empty(&self) -> bool {
        self.count == 0
    }

    /// 队列是否已满
    pub fn is_full(&self) -> bool {
        self.count >= self.capacity
    }

    /// 当前元素数量
    pub fn len(&self) -> usize {
        self.count
    }

    /// 队列容量
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// 清空队列
    pub fn clear(&mut self) {
        self.head = 0;
        self.tail = 0;
        self.count = 0;
        // 清空buffer
        for slot in &mut self.buffer {
            *slot = None;
        }
    }
}

// ========== SpscQueue：线程安全的SPSC队列 ==========

/// SPSC环形队列
///
/// 单生产者单消费者无锁队列，用于在注入器/处理线程、处理线程/输出之间传递Packet
pub struct SpscQueue<T> {
    /// 环形buffer
    buffer: Vec<Option<T>>,

    /// buffer大小（必须是2的幂）
    capacity: usize,

    /// 掩码（用于快速取模）
    mask: usize,

    /// 写入位置（生产者使用）
    write_idx: AtomicUsize,

    /// 读取位置（消费者使用）
    read_idx: AtomicUsize,

    /// 队列关闭标志
    closed: AtomicBool,

    /// 等待策略（配置时决定）
    wait_strategy: WaitStrategy,

    /// 是否启用阻塞模式
    blocking: bool,
}

impl<T> SpscQueue<T> {
    /// 创建新的SPSC队列
    ///
    /// # 参数
    /// - `capacity`: 队列容量（会向上调整为2的幂）
    pub fn new(capacity: usize) -> Self {
        // 限制容量范围
        let capacity = capacity.clamp(MIN_QUEUE_CAPACITY, MAX_QUEUE_CAPACITY);
        // 向上调整为2的幂
        let capacity = capacity.next_power_of_two();
        let mask = capacity - 1;

        SpscQueue {
            buffer: (0..capacity).map(|_| None).collect(),
            capacity,
            mask,
            write_idx: AtomicUsize::new(0),
            read_idx: AtomicUsize::new(0),
            closed: AtomicBool::new(false),
            wait_strategy: WaitStrategy::Yield,
            blocking: true,
        }
    }

    /// 使用配置创建队列
    ///
    /// # 参数
    /// - `config`: 队列配置
    pub fn with_config(config: QueueConfig) -> Self {
        // 限制容量范围
        let capacity = config.capacity.clamp(MIN_QUEUE_CAPACITY, MAX_QUEUE_CAPACITY);
        // 向上调整为2的幂
        let capacity = capacity.next_power_of_two();
        let mask = capacity - 1;

        SpscQueue {
            buffer: (0..capacity).map(|_| None).collect(),
            capacity,
            mask,
            write_idx: AtomicUsize::new(0),
            read_idx: AtomicUsize::new(0),
            closed: AtomicBool::new(false),
            wait_strategy: config.wait_strategy,
            blocking: config.blocking,
        }
    }

    /// 尝试入队（非阻塞）
    pub fn try_enqueue(&self, item: T) -> std::result::Result<(), QueueError> {
        // 检查队列是否已关闭
        if self.is_closed() {
            return Err(QueueError::Closed);
        }

        let write = self.write_idx.load(Ordering::Acquire);
        let read = self.read_idx.load(Ordering::Acquire);

        // 检查队列是否已满
        if write - read >= self.capacity {
            return Err(QueueError::Full);
        }

        // 写入数据
        self.buffer[write & self.mask] = Some(item);

        // 更新写入位置（Release确保数据可见）
        self.write_idx.store(write + 1, Ordering::Release);
        Ok(())
    }

    /// 入队（可选等待，由spin策略决定）
    pub fn enqueue(&self, item: T) -> std::result::Result<(), QueueError> {
        // 如果不阻塞，直接调用try_enqueue
        if !self.blocking {
            return self.try_enqueue(item);
        }

        match self.wait_strategy {
            WaitStrategy::Spin => {
                loop {
                    match self.try_enqueue(item) {
                        Ok(_) => return Ok(()),
                        Err(QueueError::Full) => {
                            if self.is_closed() {
                                return Err(QueueError::Closed);
                            }
                            // 自旋等待
                            std::hint::spin_loop();
                        }
                        Err(e) => return Err(e),
                    }
                }
            }
            WaitStrategy::Yield => {
                loop {
                    match self.try_enqueue(item) {
                        Ok(_) => return Ok(()),
                        Err(QueueError::Full) => {
                            if self.is_closed() {
                                return Err(QueueError::Closed);
                            }
                            std::thread::yield_now();
                        }
                        Err(e) => return Err(e),
                    }
                }
            }
            WaitStrategy::Timeout(duration) => {
                let start = std::time::Instant::now();
                loop {
                    match self.try_enqueue(item) {
                        Ok(_) => return Ok(()),
                        Err(QueueError::Full) => {
                            if self.is_closed() {
                                return Err(QueueError::Closed);
                            }
                            if start.elapsed() >= duration {
                                return Err(QueueError::Full);
                            }
                            std::thread::yield_now();
                        }
                        Err(e) => return Err(e),
                    }
                }
            }
        }
    }

    /// 尝试出队（非阻塞）
    pub fn try_dequeue(&self) -> std::result::Result<Option<T>, QueueError> {
        // 检查队列是否已关闭
        if self.is_closed() && self.is_empty() {
            return Err(QueueError::Closed);
        }

        let read = self.read_idx.load(Ordering::Acquire);
        let write = self.write_idx.load(Ordering::Acquire);

        // 检查队列是否为空
        if read == write {
            return if self.is_closed() {
                Err(QueueError::Closed)
            } else {
                Err(QueueError::Empty)
            };
        }

        // 读取数据
        let item = self.buffer[read & self.mask].take()
            .ok_or(QueueError::Empty)?;

        // 更新读取位置
        self.read_idx.store(read + 1, Ordering::Release);
        Ok(item)
    }

    /// 出队（可选等待，由spin策略决定）
    pub fn dequeue(&self) -> std::result::Result<Option<T>, QueueError> {
        // 如果不阻塞，直接调用try_dequeue
        if !self.blocking {
            return self.try_dequeue();
        }

        match self.wait_strategy {
            WaitStrategy::Spin => {
                loop {
                    match self.try_dequeue() {
                        Ok(item) => return Ok(item),
                        Err(QueueError::Empty) => {
                            if self.is_closed() {
                                return Err(QueueError::Closed);
                            }
                            std::hint::spin_loop();
                        }
                        Err(e) => return Err(e),
                    }
                }
            }
            WaitStrategy::Yield => {
                loop {
                    match self.try_dequeue() {
                        Ok(item) => return Ok(item),
                        Err(QueueError::Empty) => {
                            if self.is_closed() {
                                return Err(QueueError::Closed);
                            }
                            std::thread::yield_now();
                        }
                        Err(e) => return Err(e),
                    }
                }
            }
            WaitStrategy::Timeout(duration) => {
                let start = std::time::Instant::now();
                loop {
                    match self.try_dequeue() {
                        Ok(item) => return Ok(item),
                        Err(QueueError::Empty) => {
                            if self.is_closed() {
                                return Err(QueueError::Closed);
                            }
                            if start.elapsed() >= duration {
                                return Err(QueueError::Empty);
                            }
                            std::thread::yield_now();
                        }
                        Err(e) => return Err(e),
                    }
                }
            }
        }
    }

    /// 关闭队列
    ///
    /// 关闭后，新的入队操作将返回Closed错误
    /// 已入队的数据仍可被消费
    pub fn close(&self) {
        self.closed.store(true, Ordering::Release);
    }

    /// 队列是否已关闭
    pub fn is_closed(&self) -> bool {
        self.closed.load(Ordering::Acquire)
    }

    /// 判断队列是否为空
    pub fn is_empty(&self) -> bool {
        let read = self.read_idx.load(Ordering::Acquire);
        let write = self.write_idx.load(Ordering::Acquire);
        read == write
    }

    /// 判断队列是否已满
    pub fn is_full(&self) -> bool {
        let read = self.read_idx.load(Ordering::Acquire);
        let write = self.write_idx.load(Ordering::Acquire);
        write - read >= self.capacity
    }

    /// 获取队列中的元素数量
    pub fn len(&self) -> usize {
        let read = self.read_idx.load(Ordering::Acquire);
        let write = self.write_idx.load(Ordering::Acquire);
        write - read
    }

    /// 获取队列容量
    pub fn capacity(&self) -> usize {
        self.capacity
    }
}

// ========== SafeQueue类型别名 ==========

/// Arc包装，用于跨线程共享
///
/// 使用示例：
/// ```rust
/// let queue = Arc::new(SpscQueue::<Packet>::new(1024));
/// let queue_clone = Arc::clone(&queue);
/// ```
pub type SafeQueue<T> = Arc<SpscQueue<T>>;

// ========== 单元测试 ==========

#[cfg(test)]
mod tests {
    use super::*;

    // ========== RingQueue测试 ==========

    #[test]
    fn test_ring_queue_basic() {
        let mut queue = RingQueue::<u32>::new(4);

        assert!(queue.is_empty());
        assert_eq!(queue.len(), 0);

        queue.enqueue(1).unwrap();
        queue.enqueue(2).unwrap();
        queue.enqueue(3).unwrap();

        assert_eq!(queue.len(), 3);
        assert!(!queue.is_empty());

        assert_eq!(queue.dequeue().unwrap(), Some(1));
        assert_eq!(queue.dequeue().unwrap(), Some(2));
        assert_eq!(queue.dequeue().unwrap(), Some(3));

        assert!(queue.is_empty());
    }

    #[test]
    fn test_ring_queue_full() {
        let mut queue = RingQueue::<u32>::new(2);

        queue.enqueue(1).unwrap();
        queue.enqueue(2).unwrap();

        // 队列已满
        let result = queue.enqueue(3);
        assert!(matches!(result, Err(QueueError::Full)));
    }

    #[test]
    fn test_ring_queue_empty() {
        let mut queue = RingQueue::<u32>::new(2);

        let result = queue.dequeue();
        assert!(matches!(result, Err(QueueError::Empty)));
    }

    #[test]
    fn test_ring_queue_clear() {
        let mut queue = RingQueue::<u32>::new(4);
        queue.enqueue(1).unwrap();
        queue.enqueue(2).unwrap();
        assert_eq!(queue.len(), 2);

        queue.clear();

        assert_eq!(queue.len(), 0);
        assert!(queue.is_empty());
    }

    #[test]
    fn test_ring_queue_wraparound() {
        let mut queue = RingQueue::<u32>::new(4);

        // 填满队列
        for i in 0..4 {
            queue.enqueue(i).unwrap();
        }

        // 消费两个
        assert_eq!(queue.dequeue().unwrap(), Some(0));
        assert_eq!(queue.dequeue().unwrap(), Some(1));

        // 再添加两个（测试环绕）
        queue.enqueue(4).unwrap();
        queue.enqueue(5).unwrap();

        // 验证所有数据
        assert_eq!(queue.dequeue().unwrap(), Some(2));
        assert_eq!(queue.dequeue().unwrap(), Some(3));
        assert_eq!(queue.dequeue().unwrap(), Some(4));
        assert_eq!(queue.dequeue().unwrap(), Some(5));
    }

    // ========== SpscQueue测试 ==========

    #[test]
    fn test_spsc_queue_basic() {
        let queue = SpscQueue::<u32>::new(4);

        assert!(queue.is_empty());
        assert_eq!(queue.len(), 0);

        queue.try_enqueue(1).unwrap();
        queue.try_enqueue(2).unwrap();
        queue.try_enqueue(3).unwrap();

        assert_eq!(queue.len(), 3);
        assert!(!queue.is_empty());

        assert_eq!(queue.try_dequeue().unwrap(), Some(1));
        assert_eq!(queue.try_dequeue().unwrap(), Some(2));
        assert_eq!(queue.try_dequeue().unwrap(), Some(3));

        assert!(queue.is_empty());
    }

    #[test]
    fn test_spsc_queue_full() {
        let queue = SpscQueue::<u32>::new(2);

        queue.try_enqueue(1).unwrap();
        queue.try_enqueue(2).unwrap();

        // 队列已满
        let result = queue.try_enqueue(3);
        assert!(matches!(result, Err(QueueError::Full)));
    }

    #[test]
    fn test_spsc_queue_empty() {
        let queue = SpscQueue::<u32>::new(2);

        let result = queue.try_dequeue();
        assert!(matches!(result, Err(QueueError::Empty)));
    }

    #[test]
    fn test_spsc_queue_closed() {
        let queue = SpscQueue::<u32>::new(4);

        // 关闭队列
        queue.close();
        assert!(queue.is_closed());

        // 关闭后无法入队
        let result = queue.try_enqueue(1);
        assert!(matches!(result, Err(QueueError::Closed)));

        // 但可以消费剩余数据
        queue.try_enqueue(2).unwrap(); // 关闭前添加
        queue.close();
        assert_eq!(queue.try_dequeue().unwrap(), Some(2));
        // 空队列关闭后返回Closed
        assert!(matches!(queue.try_dequeue(), Err(QueueError::Closed)));
    }

    #[test]
    fn test_spsc_queue_roundtrip() {
        let queue = SpscQueue::<String>::new(4);

        queue.try_enqueue("hello".to_string()).unwrap();
        queue.try_enqueue("world".to_string()).unwrap();

        assert_eq!(queue.try_dequeue().unwrap(), Some("hello".to_string()));
        assert_eq!(queue.try_dequeue().unwrap(), Some("world".to_string()));
    }

    #[test]
    fn test_spsc_queue_capacity_power_of_two() {
        let queue = SpscQueue::<u32>::new(5);
        // 应该向上调整为8（2的幂）
        assert_eq!(queue.capacity(), 8);
    }

    #[test]
    fn test_spsc_queue_capacity_limits() {
        // 测试容量限制
        let queue = SpscQueue::<u32>::new(100000);
        assert_eq!(queue.capacity(), MAX_QUEUE_CAPACITY);

        let queue = SpscQueue::<u32>::new(1);
        assert_eq!(queue.capacity(), MIN_QUEUE_CAPACITY);
    }

    #[test]
    fn test_spsc_queue_with_config() {
        let config = QueueConfig {
            capacity: 128,
            blocking: true,
            wait_strategy: WaitStrategy::Spin,
        };

        let queue = SpscQueue::<u32>::with_config(config);
        assert_eq!(queue.capacity(), 128);
    }

    #[test]
    fn test_spsc_queue_blocking() {
        // 非阻塞模式：enqueue/dequeue直接返回
        let config = QueueConfig {
            capacity: 2,
            blocking: false,
            wait_strategy: WaitStrategy::Yield,
        };

        let queue = SpscQueue::<u32>::with_config(config);

        queue.try_enqueue(1).unwrap();
        queue.try_enqueue(2).unwrap();

        // 队列满时enqueue立即返回错误（不等待）
        let result = std::panic::catch_unwind(|| {
            queue.enqueue(3)
        });
        assert!(result.is_err()); // 会panic因为无限循环
    }

    #[test]
    fn test_queue_config_default() {
        let config = QueueConfig::default();
        assert_eq!(config.capacity, DEFAULT_QUEUE_CAPACITY);
        assert!(config.blocking);
        assert!(matches!(config.wait_strategy, WaitStrategy::Yield));
    }
}
