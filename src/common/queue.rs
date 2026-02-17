// src/common/queue/mod.rs
//
// 环形队列实现
// 单线程场景下的环形缓冲区

use crate::common::CoreError;

// ========== 常量定义 ==========

/// 默认队列容量
pub const DEFAULT_QUEUE_CAPACITY: usize = 256;

/// 最小队列容量
pub const MIN_QUEUE_CAPACITY: usize = 2;

/// 最大队列容量
pub const MAX_QUEUE_CAPACITY: usize = 65536;

// ========== 队列错误 ==========

/// 队列错误
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QueueError {
    /// 队列已满
    Full,
    /// 队列为空
    Empty,
}

impl std::fmt::Display for QueueError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            QueueError::Full => write!(f, "队列已满"),
            QueueError::Empty => write!(f, "队列为空"),
        }
    }
}

impl std::error::Error for QueueError {}

impl From<QueueError> for CoreError {
    fn from(err: QueueError) -> Self {
        match err {
            QueueError::Full => CoreError::QueueFull,
            QueueError::Empty => CoreError::QueueEmpty,
        }
    }
}

// ========== 队列配置 ==========

/// 队列配置
#[derive(Debug, Clone, Copy)]
pub struct QueueConfig {
    /// 队列容量（默认：256）
    pub capacity: usize,
}

impl Default for QueueConfig {
    fn default() -> Self {
        QueueConfig {
            capacity: DEFAULT_QUEUE_CAPACITY,
        }
    }
}

// ========== RingQueue：环形队列 ==========

/// 环形队列
///
/// 单线程场景下的环形缓冲区实现
pub struct RingQueue<T> {
    /// 环形缓冲区
    buffer: Vec<Option<T>>,

    /// 队列容量
    capacity: usize,

    /// 读指针
    head: usize,

    /// 写指针
    tail: usize,

    /// 当前元素数量
    count: usize,
}

impl<T: std::fmt::Debug> std::fmt::Debug for RingQueue<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RingQueue")
            .field("capacity", &self.capacity)
            .field("count", &self.count)
            .field("head", &self.head)
            .field("tail", &self.tail)
            .finish()
    }
}

impl<T> RingQueue<T> {
    /// 创建新的环形队列
    ///
    /// # 参数
    /// - `capacity`: 队列容量
    pub fn new(capacity: usize) -> Self {
        let capacity = capacity.clamp(MIN_QUEUE_CAPACITY, MAX_QUEUE_CAPACITY);
        RingQueue {
            buffer: (0..capacity).map(|_| None).collect(),
            capacity,
            head: 0,
            tail: 0,
            count: 0,
        }
    }

    /// 使用配置创建队列
    ///
    /// # 参数
    /// - `config`: 队列配置
    pub fn with_config(config: QueueConfig) -> Self {
        Self::new(config.capacity)
    }

    /// 入队
    ///
    /// # 参数
    /// - `item`: 要入队的元素
    ///
    /// # 返回
    /// - `Ok(())`: 入队成功
    /// - `Err(QueueError::Full)`: 队列已满
    pub fn enqueue(&mut self, item: T) -> std::result::Result<(), QueueError> {
        if self.count >= self.capacity {
            return Err(QueueError::Full);
        }

        self.buffer[self.tail] = Some(item);
        self.tail = (self.tail + 1) % self.capacity;
        self.count += 1;

        Ok(())
    }

    /// 出队
    ///
    /// # 返回
    /// - `Some(item)`: 出队成功，返回元素
    /// - `None`: 队列为空
    pub fn dequeue(&mut self) -> Option<T> {
        if self.count == 0 {
            return None;
        }

        let item = self.buffer[self.head].take();
        self.head = (self.head + 1) % self.capacity;
        self.count -= 1;

        item
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

// ========== 单元测试 ==========

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::packet::Packet;

    // ========== 8.2.1 基础功能测试 ==========

    #[test]
    fn test_new_queue() {
        let q: RingQueue<u8> = RingQueue::new(10);
        assert_eq!(q.capacity(), 10);
        assert!(q.is_empty());
        assert!(!q.is_full());
        assert_eq!(q.len(), 0);
    }

    #[test]
    fn test_default_config() {
        let config = QueueConfig::default();
        let q: RingQueue<u8> = RingQueue::with_config(config);
        assert_eq!(q.capacity(), DEFAULT_QUEUE_CAPACITY);
    }

    #[test]
    fn test_custom_config() {
        let config = QueueConfig { capacity: 100 };
        let q: RingQueue<u8> = RingQueue::with_config(config);
        assert_eq!(q.capacity(), 100);
    }

    #[test]
    fn test_capacity_clamp_min() {
        let q: RingQueue<u8> = RingQueue::new(1);
        assert_eq!(q.capacity(), MIN_QUEUE_CAPACITY);
    }

    #[test]
    fn test_capacity_clamp_max() {
        let q: RingQueue<u8> = RingQueue::new(100000);
        assert_eq!(q.capacity(), MAX_QUEUE_CAPACITY);
    }

    // ========== 8.2.2 入队出队测试 ==========

    #[test]
    fn test_enqueue_single() {
        let mut q: RingQueue<u32> = RingQueue::new(5);
        assert!(q.enqueue(42).is_ok());
        assert_eq!(q.len(), 1);
    }

    #[test]
    fn test_dequeue_single() {
        let mut q: RingQueue<u32> = RingQueue::new(5);
        assert!(q.enqueue(42).is_ok());
        assert_eq!(q.dequeue(), Some(42));
    }

    #[test]
    fn test_enqueue_dequeue_roundtrip() {
        let mut q: RingQueue<u32> = RingQueue::new(5);
        let value = 12345;
        assert!(q.enqueue(value).is_ok());
        assert_eq!(q.dequeue(), Some(value));
    }

    #[test]
    fn test_fifo_order() {
        let mut q: RingQueue<u8> = RingQueue::new(5);
        assert!(q.enqueue(1).is_ok());
        assert!(q.enqueue(2).is_ok());
        assert!(q.enqueue(3).is_ok());
        assert_eq!(q.dequeue(), Some(1));
        assert_eq!(q.dequeue(), Some(2));
        assert_eq!(q.dequeue(), Some(3));
    }

    #[test]
    fn test_enqueue_until_full() {
        let mut q: RingQueue<u8> = RingQueue::new(3);
        assert!(q.enqueue(1).is_ok());
        assert!(q.enqueue(2).is_ok());
        assert!(q.enqueue(3).is_ok());
        assert!(q.is_full());
    }

    #[test]
    fn test_dequeue_until_empty() {
        let mut q: RingQueue<u8> = RingQueue::new(3);
        assert!(q.enqueue(1).is_ok());
        assert!(q.enqueue(2).is_ok());
        q.dequeue();
        q.dequeue();
        assert!(q.is_empty());
    }

    // ========== 8.2.3 边界条件测试 ==========

    #[test]
    fn test_enqueue_full() {
        let mut q: RingQueue<u8> = RingQueue::new(2);
        assert!(q.enqueue(1).is_ok());
        assert!(q.enqueue(2).is_ok());
        assert_eq!(q.enqueue(3), Err(QueueError::Full));
    }

    #[test]
    fn test_dequeue_empty() {
        let mut q: RingQueue<u8> = RingQueue::new(5);
        assert_eq!(q.dequeue(), None);
    }

    #[test]
    fn test_min_capacity_queue() {
        let mut q: RingQueue<u8> = RingQueue::new(MIN_QUEUE_CAPACITY);
        assert!(q.enqueue(1).is_ok());
        assert!(q.enqueue(2).is_ok());
        assert_eq!(q.dequeue(), Some(1));
        assert_eq!(q.dequeue(), Some(2));
    }

    #[test]
    fn test_max_capacity_queue() {
        let mut q: RingQueue<u8> = RingQueue::new(MAX_QUEUE_CAPACITY);
        assert!(q.enqueue(1).is_ok());
        assert_eq!(q.dequeue(), Some(1));
    }

    // ========== 8.2.4 环形特性测试 ==========

    #[test]
    fn test_wrap_around() {
        let mut q: RingQueue<u8> = RingQueue::new(3);
        // 填满队列
        assert!(q.enqueue(1).is_ok());
        assert!(q.enqueue(2).is_ok());
        assert!(q.enqueue(3).is_ok());
        // 清空两个
        assert_eq!(q.dequeue(), Some(1));
        assert_eq!(q.dequeue(), Some(2));
        // 重新入队触发环绕
        assert!(q.enqueue(4).is_ok());
        assert!(q.enqueue(5).is_ok());
        assert_eq!(q.dequeue(), Some(3));
        assert_eq!(q.dequeue(), Some(4));
        assert_eq!(q.dequeue(), Some(5));
    }

    #[test]
    fn test_multiple_wrap() {
        let mut q: RingQueue<u8> = RingQueue::new(4);
        for i in 0..20 {
            assert!(q.enqueue(i).is_ok());
            assert_eq!(q.dequeue(), Some(i));
        }
    }

    #[test]
    fn test_enumerate_all() {
        let mut q: RingQueue<u8> = RingQueue::new(5);
        // 覆盖所有buffer位置
        for i in 0..10 {
            assert!(q.enqueue(i).is_ok());
            assert_eq!(q.dequeue(), Some(i));
        }
    }

    // ========== 8.2.5 状态查询测试 ==========

    #[test]
    fn test_is_empty() {
        let mut q: RingQueue<u8> = RingQueue::new(5);
        assert!(q.is_empty());
        q.enqueue(1).unwrap();
        assert!(!q.is_empty());
        q.clear();
        assert!(q.is_empty());
    }

    #[test]
    fn test_is_full() {
        let mut q: RingQueue<u8> = RingQueue::new(3);
        assert!(!q.is_full());
        q.enqueue(1).unwrap();
        q.enqueue(2).unwrap();
        q.enqueue(3).unwrap();
        assert!(q.is_full());
    }

    #[test]
    fn test_len() {
        let mut q: RingQueue<u8> = RingQueue::new(10);
        assert_eq!(q.len(), 0);
        q.enqueue(1).unwrap();
        assert_eq!(q.len(), 1);
        q.enqueue(2).unwrap();
        assert_eq!(q.len(), 2);
        q.dequeue();
        assert_eq!(q.len(), 1);
    }

    #[test]
    fn test_capacity() {
        let q1: RingQueue<u8> = RingQueue::new(5);
        assert_eq!(q1.capacity(), 5);
        let q2: RingQueue<u8> = RingQueue::new(100);
        assert_eq!(q2.capacity(), 100);
    }

    // ========== 8.2.6 清空操作测试 ==========

    #[test]
    fn test_clear_empty() {
        let mut q: RingQueue<u8> = RingQueue::new(5);
        q.clear();
        assert!(q.is_empty());
        assert_eq!(q.len(), 0);
    }

    #[test]
    fn test_clear_with_data() {
        let mut q: RingQueue<u8> = RingQueue::new(5);
        q.enqueue(1).unwrap();
        q.enqueue(2).unwrap();
        q.clear();
        assert!(q.is_empty());
        assert_eq!(q.len(), 0);
        assert_eq!(q.dequeue(), None);
    }

    #[test]
    fn test_clear_memory_release() {
        use std::sync::atomic::{AtomicU32, Ordering};
        static DROP_COUNT: AtomicU32 = AtomicU32::new(0);

        struct DropCounter;

        impl Drop for DropCounter {
            fn drop(&mut self) {
                DROP_COUNT.fetch_add(1, Ordering::SeqCst);
            }
        }

        DROP_COUNT.store(0, Ordering::SeqCst);
        {
            let mut q: RingQueue<DropCounter> = RingQueue::new(5);
            q.enqueue(DropCounter).unwrap();
            q.enqueue(DropCounter).unwrap();
            q.enqueue(DropCounter).unwrap();
            assert_eq!(DROP_COUNT.load(Ordering::SeqCst), 0);
            q.clear();
            assert_eq!(DROP_COUNT.load(Ordering::SeqCst), 3);
        }
        assert_eq!(DROP_COUNT.load(Ordering::SeqCst), 3);
    }

    // ========== 8.2.7 泛型类型测试 ==========

    #[test]
    fn test_with_u8() {
        let mut q: RingQueue<u8> = RingQueue::new(5);
        assert!(q.enqueue(255).is_ok());
        assert_eq!(q.dequeue(), Some(255));
    }

    #[test]
    fn test_with_string() {
        let mut q: RingQueue<String> = RingQueue::new(3);
        assert!(q.enqueue("hello".to_string()).is_ok());
        assert!(q.enqueue("world".to_string()).is_ok());
        assert_eq!(q.dequeue(), Some("hello".to_string()));
        assert_eq!(q.dequeue(), Some("world".to_string()));
    }

    #[test]
    fn test_with_packet() {
        let mut q: RingQueue<Packet> = RingQueue::new(5);
        let packet = Packet::from_bytes(vec![0x01, 0x02, 0x03, 0x04]);
        assert!(q.enqueue(packet).is_ok());
        let received = q.dequeue();
        assert!(received.is_some());
        assert_eq!(received.unwrap().as_slice(), &[0x01, 0x02, 0x03, 0x04]);
    }

    #[test]
    fn test_with_option() {
        let mut q: RingQueue<Option<u32>> = RingQueue::new(5);
        assert!(q.enqueue(Some(42)).is_ok());
        assert!(q.enqueue(None).is_ok());
        assert_eq!(q.dequeue(), Some(Some(42)));
        assert_eq!(q.dequeue(), Some(None));
    }
}
