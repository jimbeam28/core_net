# Queue模块实现日志

## 日期
2026-02-12

## 概述

按照 [`../detail/queue.md`](../detail/queue.md) 设计文档，实现了完整的队列模块，包括：
- **RingQueue**: 非线程安全的环形队列（单线程使用）
- **SpscQueue**: 单生产者单消费者的无锁队列（多线程使用）
- **QueueConfig**: 队列配置结构
- **SafeQueue**: Arc包装的类型别名

---

## 一、常量定义

```rust
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
```

---

## 二、队列错误

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QueueError {
    /// 队列已满
    Full,
    /// 队列为空
    Empty,
    /// 队列已关闭
    Closed,
}
```

**说明**: `Closed` 错误用于队列关闭后的操作，确保优雅关闭。

---

## 三、等待策略

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WaitStrategy {
    /// 自旋等待（CPU忙等待）
    Spin,
    /// 休眠等待（yield让出CPU）
    Yield,
    /// 超时后返回错误
    Timeout(Duration),
}
```

**说明**:
- `Spin`: CPU密集型等待，适合短等待
- `Yield`: 让出CPU时间片，适合一般场景
- `Timeout`: 带超时的等待

---

## 四、队列配置

```rust
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
```

**说明**: 配置结构用于在创建队列时指定行为，避免每次调用时传递参数。

---

## 五、RingQueue - 非线程安全环形队列

### 结构定义

```rust
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
```

### 核心方法

| 方法 | 说明 | 时间复杂度 |
|------|------|------------|
| `new(capacity)` | 创建指定容量的空队列 | O(n) |
| `enqueue(&mut self, item)` | 入队（生产者） | O(1) |
| `dequeue(&mut self)` | 出队（消费者） | O(1) |
| `is_empty(&self)` | 判断是否为空 | O(1) |
| `is_full(&self)` | 判断是否已满 | O(1) |
| `len(&self)` | 获取元素数量 | O(1) |
| `capacity(&self)` | 获取队列容量 | O(1) |
| `clear(&mut self)` | 清空队列 | O(n) |

### 使用示例

```rust
// 创建队列
let mut queue = RingQueue::<u32>::new(4);

// 生产数据
queue.enqueue(1)?;
queue.enqueue(2)?;
queue.enqueue(3)?;

// 消费数据
while let Some(item) = queue.dequeue()? {
    println!("消费: {}", item);
}
```

### 环形回绕测试

```rust
let mut queue = RingQueue::<u32>::new(4);

// 填满队列
for i in 0..4 {
    queue.enqueue(i)?;
}

// 消费两个
assert_eq!(queue.dequeue()?, Some(0));
assert_eq!(queue.dequeue()?, Some(1));

// 再添加两个（测试回绕）
queue.enqueue(4)?;
queue.enqueue(5)?;

// 验证所有数据
assert_eq!(queue.dequeue()?, Some(2));
assert_eq!(queue.dequeue()?, Some(3));
assert_eq!(queue.dequeue()?, Some(4));
assert_eq!(queue.dequeue()?, Some(5));
```

---

## 六、SpscQueue - 单生产者单消费者无锁队列

### 结构定义

```rust
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
```

### 无锁设计

使用原子操作实现无锁并发：
- `Acquire`语义：读取时获取最新值
- `Release`语义：写入时确保数据可见
- 位掩码取模：`index & mask` 代替 `index % capacity`

### 构造方法

| 方法 | 说明 |
|------|------|
| `new(capacity)` | 使用默认配置创建队列 |
| `with_config(config)` | 使用指定配置创建队列 |

### 核心操作

| 方法 | 说明 |
|------|------|
| `try_enqueue(&self, item)` | 非阻塞入队 |
| `enqueue(&self, item)` | 入队（根据blocking决定是否等待） |
| `try_dequeue(&self)` | 非阻塞出队 |
| `dequeue(&self)` | 出队（根据blocking决定是否等待） |
| `is_closed(&self)` | 队列是否已关闭 |
| `close(&self)` | 关闭队列 |

### 阻塞模式

当 `blocking: true` 时：
- `enqueue()` 使用配置的等待策略自旋/让出/超时等待
- `dequeue()` 使用配置的等待策略自旋/让出/超时等待

当 `blocking: false` 时：
- `enqueue()` 直接调用 `try_enqueue()`，失败立即返回
- `dequeue()` 直接调用 `try_dequeue()`，失败立即返回

### 使用示例

```rust
use std::sync::Arc;
use std::thread;
use std::time::Duration;

// 使用默认配置创建
let queue = SpscQueue::<Packet>::new(1024);

// 使用自定义配置创建
let config = QueueConfig {
    capacity: 2048,
    blocking: true,
    wait_strategy: WaitStrategy::Spin,
};
let queue = SpscQueue::<Packet>::with_config(config);

// 生产者线程
let producer = Arc::new(&queue);
let producer_clone = Arc::clone(&producer);
thread::spawn(move || {
    for i in 0..100 {
        match producer_clone.enqueue(Packet::new(1500)) {
            Ok(_) => println!("入队成功: {}", i),
            Err(QueueError::Closed) => break,
            Err(QueueError::Full) => {
                // 队列满，继续等待
            }
            _ => {}
        }
    }
});

// 消费者线程
let consumer = Arc::new(&queue);
let consumer_clone = Arc::clone(&consumer);
thread::spawn(move || {
    loop {
        match consumer_clone.dequeue() {
            Ok(Some(packet)) => {
                // 处理报文
                println!("出队: {:?}", packet);
            }
            Err(QueueError::Closed) => {
                println!("队列已关闭，退出");
                break;
            }
            Err(QueueError::Empty) => {
                // 队列空，继续等待
                thread::yield_now();
            }
            _ => {}
        }
    }
});

// 关闭队列
queue.close();
```

---

## 七、SafeQueue类型别名

```rust
/// Arc包装，用于跨线程共享
pub type SafeQueue<T> = Arc<SpscQueue<T>>;
```

**说明**:
- 用于在多线程间共享队列所有权
- 生产者和消费者各持一个Arc克隆
- 队列生命周期由Arc管理

---

## 八、单元测试

### RingQueue测试

| 测试名称 | 说明 |
|----------|------|
| `test_ring_queue_basic` | 基本入队出队 |
| `test_ring_queue_full` | 队列满检测 |
| `test_ring_queue_empty` | 队列空检测 |
| `test_ring_queue_clear` | 清空队列 |
| `test_ring_queue_wraparound` | 环形回绕 |

### SpscQueue测试

| 测试名称 | 说明 |
|----------|------|
| `test_spsc_queue_basic` | 基本入队出队 |
| `test_spsc_queue_full` | 队列满检测 |
| `test_spsc_queue_empty` | 队列空检测 |
| `test_spsc_queue_closed` | 关闭队列机制 |
| `test_spsc_queue_roundtrip` | 往返数据传输 |
| `test_spsc_queue_capacity_power_of_two` | 2的幂调整 |
| `test_spsc_queue_capacity_limits` | 容量范围限制 |
| `test_spsc_queue_with_config` | 自定义配置 |
| `test_spsc_queue_blocking` | 非阻塞模式 |

---

## 九、性能特点

1. **无锁并发**: SPSC队列使用原子操作，无需互斥锁
2. **零拷贝**: 元素在队列中移动所有权
3. **缓存友好**: 连续内存访问模式
4. **快速取模**: 使用位掩码代替取模运算

---

## 十、使用场景

### 场景一：注入器 → 处理线程

```rust
let rxq: SpscQueue<Packet> = SpscQueue::new(1024);

// 注入器（生产者）
rxq.try_enqueue(packet)?;

// 处理线程（消费者）
while let Some(packet) = rxq.dequeue()? {
    // 处理报文
}
```

### 场景二：处理线程 → 结果输出

```rust
let txq: SpscQueue<Packet> = SpscQueue::new(1024);

// 处理线程（生产者）
txq.enqueue(processed_packet)?;

// 结果输出（消费者）
while let Some(packet) = txq.dequeue()? {
    // 输出结果
}
```

---

## 十一、设计符合性

| 设计要求 | 实现状态 |
|----------|----------|
| RingQueue 结构 | ✅ 完成 |
| SpscQueue 结构 | ✅ 完成 |
| QueueError::Closed | ✅ 完成 |
| QueueConfig 结构 | ✅ 完成 |
| enqueue/dequeue 签名 | ✅ 无strategy参数 |
| try_enqueue/try_dequeue | ✅ 完成 |
| close/is_closed | ✅ 完成 |
| 常量定义 | ✅ 完成 |
| SafeQueue 类型别名 | ✅ 完成 |
| 完整单元测试 | ✅ 完成 |

---

## 十二、验证编译

```bash
# 检查编译
cargo check

# 构建项目
cargo build

# 运行测试
cargo test

# 格式化代码
cargo fmt

# 静态检查
cargo clippy
```
