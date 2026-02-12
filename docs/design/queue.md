# 环形队列设计

## 1. 概述

CoreNet使用自定义的环形缓冲区（Ring Buffer）实现收发队列，用于在注入器、处理线程和结果输出之间传递报文。

## 2. 队列模型

```
┌──────────────┐         ┌──────────────┐         ┌──────────────┐
│ 报文注入器   │  ───>  │  接收队列    │  ───>  │  处理线程    │
│ (生产者)     │  RxQ   │  (SPSC)      │         │  (消费者)     │
└──────────────┘         └──────────────┘         └──────────────┘

┌──────────────┐         ┌──────────────┐         ┌──────────────┐
│ 处理线程     │  ───>  │  发送队列    │  ───>  │  结果输出     │
│ (生产者)     │  TxQ   │  (SPSC)      │         │  (消费者)     │
└──────────────┘         └──────────────┘         └──────────────┘
```

**SPSC**: Single Producer Single Consumer（单生产者单消费者）

## 3. 数据结构

### 3.1 核心结构

```rust
/// 环形队列
pub struct RingQueue<T> {
    buffer: Vec<Option<T>>,    // 环形缓冲区
    capacity: usize,           // 队列容量
    head: usize,              // 读指针（消费者位置）
    tail: usize,              // 写指针（生产者位置）
    count: usize,             // 当前元素数量
}

/// 队列错误
#[derive(Debug, Clone, PartialEq)]
pub enum QueueError {
    Full,       // 队列已满
    Empty,      // 队列为空
    Closed,     // 队列已关闭
}
```

### 3.2 线程安全封装

```rust
/// 线程安全的SPSC队列
pub struct SpscQueue<T> {
    // 使用原子操作实现无锁SPSC
    buffer: Vec<Option<T>>,
    capacity: usize,
    write_idx: AtomicUsize,     // 写索引（仅生产者修改）
    read_idx: AtomicUsize,      // 读索引（仅消费者修改）
    closed: AtomicBool,        // 队列关闭标志
}

/// Arc包装，用于跨线程共享
pub type SafeQueue<T> = Arc<SpscQueue<T>>;
```

## 4. 接口定义

### 4.1 RingQueue API

```rust
impl<T> RingQueue<T> {
    /// 创建新队列
    pub fn new(capacity: usize) -> Self;

    /// 入队（生产者）
    pub fn enqueue(&mut self, item: T) -> Result<(), QueueError>;

    /// 出队（消费者）
    pub fn dequeue(&mut self) -> Result<Option<T>, QueueError>;

    /// 队列是否为空
    pub fn is_empty(&self) -> bool;

    /// 队列是否已满
    pub fn is_full(&self) -> bool;

    /// 当前元素数量
    pub fn len(&self) -> usize;

    /// 队列容量
    pub fn capacity(&self) -> usize;

    /// 清空队列
    pub fn clear(&mut self);
}
```

### 4.2 SpscQueue API

```rust
impl<T> SpscQueue<T> {
    /// 创建新的SPSC队列
    pub fn new(capacity: usize) -> Self;

    /// 尝试入队（非阻塞）
    pub fn try_enqueue(&self, item: T) -> Result<(), QueueError>;

    /// 入队（可选等待，由spin策略决定）
    pub fn enqueue(&self, item: T) -> Result<(), QueueError>;

    /// 尝试出队（非阻塞）
    pub fn try_dequeue(&self) -> Result<Option<T>, QueueError>;

    /// 出队（可选等待，由spin策略决定）
    pub fn dequeue(&self) -> Result<Option<T>, QueueError>;

    /// 关闭队列
    pub fn close(&self);

    /// 队列是否已关闭
    pub fn is_closed(&self) -> bool;

    /// 队列是否为空
    pub fn is_empty(&self) -> bool;

    /// 队列是否已满
    pub fn is_full(&self) -> bool;
}
```

## 5. 配置参数

```rust
/// 队列配置
pub struct QueueConfig {
    /// 队列容量（默认：256）
    pub capacity: usize,

    /// 是否启用阻塞模式（默认：true）
    pub blocking: bool,

    /// 等待策略
    pub wait_strategy: WaitStrategy,
}

/// 等待策略
pub enum WaitStrategy {
    /// 自旋等待（CPU忙等待）
    Spin,

    /// 休眠等待（yield让出CPU）
    Yield,

    /// 超时后返回错误
    Timeout(Duration),
}
```

## 6. 内存布局

```
环形缓冲区内存布局：
┌────┬────┬────┬────┬────┬────┬────┬────┬────┐
│ 0  │ 1  │ 2  │ 3  │ 4  │ 5  │ 6  │ 7  │ ... │
└────┴────┴────┴────┴────┴────┴────┴────┴────┴────┘
  ↑                                               ↑
  tail(写)                                   head(读)

当 tail 追上 head 时，队列满
当 head == tail 时，队列空
```

## 7. 常量定义

```rust
/// 默认队列容量
pub const DEFAULT_QUEUE_CAPACITY: usize = 256;

/// 最小队列容量
pub const MIN_QUEUE_CAPACITY: usize = 2;

/// 最大队列容量
pub const MAX_QUEUE_CAPACITY: usize = 65536;

/// 默认自旋次数
pub const DEFAULT_SPIN_COUNT: usize = 1000;

/// 默认超时时间
pub const DEFAULT_TIMEOUT_MS: u64 = 100;
```
