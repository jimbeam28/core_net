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
}

impl<T> RingQueue<T> {
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
