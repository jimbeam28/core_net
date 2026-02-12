# 环形队列设计

## 1. 概述

CoreNet使用自定义的环形缓冲区（Ring Buffer）实现收发队列，用于在各个协议处理阶段之间传递报文。

## 2. 队列模型

```
┌──────────────┐         ┌──────────────┐         ┌──────────────┐
│ 报文注入     │  ───>  │  接收队列    │  ───>  │  协议处理    │
│             │  RxQ   │  (RingQueue) │         │             │
└──────────────┘         └──────────────┘         └──────────────┘

┌──────────────┐         ┌──────────────┐         ┌──────────────┐
│ 协议处理     │  ───>  │  发送队列    │  ───>  │  结果输出     │
│             │  TxQ   │  (RingQueue) │         │             │
└──────────────┘         └──────────────┘         └──────────────┘
```

## 3. 数据结构

### 核心结构

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
}
```

## 4. 接口定义

### RingQueue API

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

## 5. 配置参数

```rust
/// 队列配置
pub struct QueueConfig {
    /// 队列容量（默认：256）
    pub capacity: usize,
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
```
