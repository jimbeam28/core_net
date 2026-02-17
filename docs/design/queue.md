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

## 8. 测试策略

### 8.1 测试框架

CoreNet使用Rust标准库内置测试框架，无需额外依赖：

```rust
// 单元测试 - 放在源文件内的 #[cfg(test)] 模块
#[cfg(test)]
mod tests {
    #[test]
    fn test_example() {
        // 测试代码
    }
}

// 集成测试 - 放在 tests/ 目录下的独立文件
// tests/queue_integration_test.rs
#[test]
fn integration_test_example() {
    // 测试代码
}
```

### 8.2 单元测试用例

#### 8.2.1 基础功能测试

| 测试用例 | 描述 | 验证点 |
|---------|------|--------|
| `test_new_queue` | 创建新队列 | capacity正确，初始为空，count=0 |
| `test_default_config` | 默认配置 | capacity=DEFAULT_QUEUE_CAPACITY |
| `test_custom_config` | 自定义配置 | with_config正确应用配置 |
| `test_capacity_clamp_min` | 容量下限限制 | capacity<MIN时自动限制 |
| `test_capacity_clamp_max` | 容量上限限制 | capacity>MAX时自动限制 |

#### 8.2.2 入队出队测试

| 测试用例 | 描述 | 验证点 |
|---------|------|--------|
| `test_enqueue_single` | 单元素入队 | enqueue返回Ok，len=1 |
| `test_dequeue_single` | 单元素出队 | dequeue返回Some，元素值正确 |
| `test_enqueue_dequeue_roundtrip` | 入队后出队 | 数据一致性保持 |
| `test_fifo_order` | FIFO顺序 | 多元素按入队顺序出队 |
| `test_enqueue_until_full` | 填满队列 | 填满后is_full返回true |
| `test_dequeue_until_empty` | 清空队列 | 清空后is_empty返回true |

#### 8.2.3 边界条件测试

| 测试用例 | 描述 | 验证点 |
|---------|------|--------|
| `test_enqueue_full` | 队列满时入队 | 返回Err(QueueError::Full) |
| `test_dequeue_empty` | 队列空时出队 | 返回None |
| `test_min_capacity_queue` | 最小容量队列 | capacity=2时正常工作 |
| `test_max_capacity_queue` | 最大容量队列 | capacity=65536时正常工作 |

#### 8.2.4 环形特性测试

| 测试用例 | 描述 | 验证点 |
|---------|------|--------|
| `test_wrap_around` | 指针环绕 | head/tail正确绕回buffer开头 |
| `test_multiple_wrap` | 多次环绕 | 多次循环后仍正常工作 |
| `test_enumerate_all` | 遍历所有位置 | 覆盖buffer所有索引 |

#### 8.2.5 状态查询测试

| 测试用例 | 描述 | 验证点 |
|---------|------|--------|
| `test_is_empty` | 空状态检测 | 初始及清空后返回true |
| `test_is_full` | 满状态检测 | 填满后返回true |
| `test_len` | 元素计数 | len()与实际元素数一致 |
| `test_capacity` | 容量查询 | capacity()返回正确值 |

#### 8.2.6 清空操作测试

| 测试用例 | 描述 | 验证点 |
|---------|------|--------|
| `test_clear_empty` | 清空队列 | 清空后is_empty返回true |
| `test_clear_with_data` | 有数据时清空 | 数据被丢弃，状态重置 |
| `test_clear_memory_release` | 内存释放 | buffer元素被drop |

#### 8.2.7 泛型类型测试

| 测试用例 | 描述 | 验证点 |
|---------|------|--------|
| `test_with_u8` | u8类型队列 | 基本类型正常工作 |
| `test_with_string` | String类型队列 | 有所有权类型正常工作 |
| `test_with_packet` | Packet类型队列 | 复杂类型正常工作 |
| `test_with_option` | Option类型队列 | 嵌套泛型正常工作 |

### 8.3 集成测试用例

#### 8.3.1 与Packet模块集成

| 测试用例 | 描述 | 验证点 |
|---------|------|--------|
| `test_packet_queue_flow` | Packet在队列中流动 | Packet所有权正确转移 |
| `test_multiple_packets` | 多个Packet入队出队 | 多个报文独立处理 |
| `test_large_packet` | 大Packet入队 | 大报文正常处理 |

#### 8.3.2 并发场景模拟（单线程）

| 测试用例 | 描述 | 验证点 |
|---------|------|--------|
| `test_producer_consumer_pattern` | 生产者消费者模式 | 交替入队出队正常 |
| `test_burst_enqueue` | 突发入队 | 快速连续入队正常 |
| `test_burst_dequeue` | 突发出队 | 快速连续出队正常 |
| `test_alternating_ops` | 交替操作 | 入队出队混合操作正常 |

#### 8.3.3 边界压力测试

| 测试用例 | 描述 | 验证点 |
|---------|------|--------|
| `test_stress_fill_drain` | 反复填充清空 | 多次循环无泄漏 |
| `test_random_ops` | 随机操作序列 | 随机入队出队正常 |
| `test_zero_capacity_handling` | 零容量处理 | 容量为0时自动修正 |

#### 8.3.4 与CoreError集成

| 测试用例 | 描述 | 验证点 |
|---------|------|--------|
| `test_queue_error_conversion` | 错误类型转换 | QueueError正确转换为CoreError |
| `test_error_propagation` | 错误传播 | 错误正确向上传播 |

### 8.4 测试目录结构

```
core_net/
├── src/
│   └── common/
│       └── queue.rs          # 单元测试在文件末尾的 #[cfg(test)] 模块
└── tests/
    └── queue_integration_test.rs  # 集成测试
```

### 8.5 运行测试

```bash
# 运行所有测试
cargo test

# 运行队列模块单元测试
cargo test queue::tests

# 运行集成测试
cargo test --test queue_integration_test

# 显示测试输出
cargo test -- --nocapture

# 运行特定测试
cargo test test_wrap_around
```
