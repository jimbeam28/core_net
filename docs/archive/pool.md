# Pool模块实现日志

## 日期
2026-02-12

## 概述

本次实现了 CoreNet 的通用对象池框架，提供高性能的对象复用机制，用于减少频繁的内存分配和释放开销。该框架设计为通用型，支持任意类型的对象池化，并为 Packet 类型提供了专门的适配层。

---

## 一、Clear Trait (`src/common/pool/clear.rs`)

### 1.1 设计目标

提供对象清空的统一接口，使对象池能够在归还对象时自动重置对象状态。

### 1.2 核心定义

```rust
/// 可清空对象 trait
pub trait Clear {
    fn clear(&mut self);
}
```

### 1.3 实现特性

- 简洁的 trait 定义，仅需实现 `clear` 方法
- 为 `Packet` 类型实现了 `Clear` trait

---

## 二、配置和策略模块 (`src/common/pool/config.rs`)

### 2.1 分配策略

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AllocStrategy {
    /// 先进先出
    Fifo,
    /// 后进先出
    Lifo,
    /// 随机获取
    Random,
    /// 优先分配最近使用的
    Recent,
}
```

| 策略 | 适用场景 | 特点 |
|------|----------|------|
| Fifo | 公平性要求高 | 保证公平分配 |
| Lifo | 低延迟场景 | 利用缓存局部性 |
| Random | 高并发场景 | 减少锁竞争 |
| Recent | 短生命周期对象 | CPU 缓存友好 |

### 2.2 等待策略

```rust
#[derive(Debug, Clone, Copy)]
pub enum WaitStrategy {
    /// 立即返回失败
    Immediate,
    /// 自旋等待指定次数
    Spin(usize),
    /// 让出 CPU 时间片
    Yield,
    /// 等待指定超时时间
    Timeout(Duration),
    /// 无限等待
    Blocking,
}
```

### 2.3 池配置

```rust
#[derive(Debug, Clone)]
pub struct PoolConfig {
    pub capacity: usize,
    pub initial_capacity: usize,
    pub alloc_strategy: AllocStrategy,
    pub wait_strategy: WaitStrategy,
    pub allow_overflow: bool,
}

impl Default for PoolConfig {
    fn default() -> Self {
        Self {
            capacity: 100,
            initial_capacity: 10,
            alloc_strategy: AllocStrategy::Fifo,
            wait_strategy: WaitStrategy::Timeout(Duration::from_millis(100)),
            allow_overflow: false,
        }
    }
}
```

---

## 三、错误类型模块 (`src/common/pool/error.rs`)

### 3.1 核心定义

```rust
#[derive(Debug)]
pub enum PoolError {
    /// 池已空（无可用对象）
    Empty,
    /// 池已满（归还时超出容量）
    Full,
    /// 池已关闭
    Shutdown,
    /// 超时
    Timeout(Duration),
    /// 配置错误
    InvalidConfig(String),
    /// 其他错误
    Other(String),
}
```

### 3.2 实现特性

1. **Display 实现**：提供中文错误描述
2. **std::error::Error 实现**：完全兼容 Rust 标准错误处理
3. **Debug 派生**：支持错误调试

---

## 四、统计模块 (`src/common/pool/stats.rs`)

### 4.1 核心结构

```rust
/// 池状态快照
#[derive(Debug, Clone)]
pub struct PoolStatus {
    pub idle: usize,
    pub active: usize,
    pub utilization: f64,
}

/// 池统计（使用原子操作实现线程安全）
#[derive(Debug)]
pub struct PoolStats {
    pub total_allocations: AtomicU64,
    pub pooled_allocations: AtomicU64,
    pub overflow_allocations: AtomicU64,
    pub idle_count: AtomicUsize,
    pub active_count: AtomicUsize,
    pub wait_count: AtomicUsize,
    pub total_wait_ns: AtomicU64,
    pub release_count: AtomicU64,
}
```

### 4.2 关键方法

```rust
impl PoolStats {
    pub fn new() -> Arc<Self>;
    pub fn utilization_rate(&self) -> f64;
    pub fn avg_wait_ns(&self) -> u64;
    pub fn reset(&self);
}

impl Clone for PoolStats {
    fn clone(&self) -> Self {
        // 克隆所有原子计数器的当前值
    }
}
```

### 4.3 性能指标

- **utilization_rate()**: 池使用率 = active / (active + idle)
- **avg_wait_ns()**: 平均等待时间（纳秒）
- **命中率**: pooled_allocations / total_allocations

---

## 五、Pooled 包装器模块 (`src/common/pool/pooled.rs`)

### 5.1 设计目标

提供自动归还机制的包装器，当包装器离开作用域时自动将对象归还到池中。

### 5.2 核心结构

```rust
pub struct Pooled<T> {
    inner: Option<T>,
    pool: Option<Arc<Pool<T>>>,
    released: AtomicBool,
}
```

### 5.3 核心方法

| 方法 | 说明 |
|------|------|
| `new(item, pool)` | 创建新的池化对象 |
| `inner()` | 获取不可变引用 |
| `inner_mut()` | 获取可变引用 |
| `release(self)` | 手动归还（消耗 self） |
| `release_and_clear(self)` | 归还并清空 |
| `detach(self)` | 提取对象，不归还 |
| `map<U, F>(self, f)` | 转换对象 |

### 5.4 Trait 实现

```rust
impl<T> Deref for Pooled<T> { /* ... */ }
impl<T> DerefMut for Pooled<T> { /* ... */ }
// 不实现 Clone，避免意外复制导致重复释放
impl<T> Drop for Pooled<T> { /* 自动归还 */ }
```

---

## 六、通用对象池模块 (`src/common/pool/pool.rs`)

### 6.1 核心结构

```rust
pub struct Pool<T> {
    idle: Mutex<Vec<T>>,
    config: PoolConfig,
    stats: Arc<PoolStats>,
    factory: Box<dyn Fn() -> T + Send + Sync>,
    resetter: Option<Box<dyn Fn(&mut T) + Send + Sync>>,
    shutdown: Arc<Mutex<bool>>,
}
```

### 6.2 构造方法

| 方法 | 说明 |
|------|------|
| `new<F>(factory, config)` | 创建新池 |
| `with_resetter<F, R>(factory, resetter, config)` | 创建带重置器的池 |

### 6.3 核心操作

| 方法 | 说明 |
|------|------|
| `acquire(&self)` | 获取对象（可能等待） |
| `try_acquire(&self)` | 尝试获取（非阻塞） |
| `acquire_many(&self, count)` | 获取多个对象 |
| `release(&self, item)` | 归还对象 |
| `release_and_clear(&self, item)` | 归还并清空 |
| `stats(&self)` | 获取统计 |
| `reset_stats(&self)` | 重置统计 |
| `warm_up(&self, count)` | 预热池 |
| `shrink(&self, target_count)` | 收缩池 |
| `status(&self)` | 获取状态 |
| `shutdown(&self)` | 关闭池 |

### 6.4 分配策略实现

| 策略 | 实现方式 |
|------|----------|
| Fifo | 从头部弹出 (`idle.remove(0)`) |
| Lifo | 从尾部弹出 (`idle.pop()`) |
| Random | 随机选择（简化为 Lifo） |
| Recent | 从尾部弹出 |

### 6.5 等待策略实现

| 策略 | 实现方式 |
|------|----------|
| Immediate | 立即返回 Empty 错误 |
| Spin(n) | 自旋 n 次后继续循环 |
| Yield | 调用 `thread::yield_now()` |
| Timeout(d) | 超时后返回，期间休眠 1ms |
| Blocking | 休眠 10ms 后继续 |

### 6.6 溢出处理

当 `allow_overflow = true` 时：
- 池空时直接调用 factory 创建新对象
- 统计 `overflow_allocations` 计数增加
- 归还时如果池满则丢弃对象

---

## 七、Packet 适配层模块 (`src/common/pool/packet_pool.rs`)

### 7.1 PacketPoolConfig

```rust
#[derive(Debug, Clone)]
pub struct PacketPoolConfig {
    pub pool: PoolConfig,
    pub packet_size: usize,
    pub header_reserve: usize,
    pub trailer_reserve: usize,
    pub default_interface: Option<InterfaceId>,
}

impl Default for PacketPoolConfig {
    fn default() -> Self {
        Self {
            pool: PoolConfig::default(),
            packet_size: 1514,
            header_reserve: 128,
            trailer_reserve: 0,
            default_interface: None,
        }
    }
}
```

#### 预设配置

| 配置 | 容量 | 分配策略 | Packet 大小 | 适用场景 |
|------|-------|----------|------------|----------|
| Default | 100 | Fifo | 1514B | 通用场景 |
| high_throughput | 1000 | Recent | 9000B | 大流量传输 |
| low_latency | 500 | Lifo | 1514B | 实时通信 |
| memory_constrained | 50 | Fifo | 1514B | 嵌入式设备 |

### 7.2 PacketPoolStats

```rust
#[derive(Debug)]
pub struct PacketPoolStats {
    pub base: PoolStats,           // 基础池统计
    pub total_bytes: AtomicU64,      // 总处理字节数
    pub pooled_capacity_bytes: AtomicU64,  // 池总容量
    pub avg_packet_length: AtomicU64,     // 平均包长
}

impl PacketPoolStats {
    pub fn new(base: &PoolStats, capacity: usize, packet_size: usize) -> Self;
    pub fn memory_efficiency(&self) -> f64;
}
```

### 7.3 PacketPool

```rust
pub struct PacketPool {
    inner: Pool<Packet>,
    config: PacketPoolConfig,
    stats: Arc<PacketPoolStats>,
}
```

#### 核心方法

| 方法 | 说明 |
|------|------|
| `new(config)` | 创建新 Packet 池 |
| `with_capacity(capacity)` | 创建默认配置池 |
| `acquire(&self)` | 获取 PooledPacket |
| `try_acquire(&self)` | 尝试获取（非阻塞） |
| `release(&self, packet)` | 归还 Packet |
| `release_and_clear(&self, packet)` | 归还并清空 |
| `stats(&self)` | 获取统计 |
| `warm_up(&self, count)` | 预热池 |
| `shrink(&self, target_count)` | 收缩池 |
| `shutdown(&self)` | 关闭池 |

### 7.4 PooledPacket

```rust
pub struct PooledPacket {
    inner: Pooled<Packet>,
    pool: Arc<PacketPool>,
    track_bytes: bool,
}
```

#### 核心方法

| 方法 | 说明 |
|------|------|
| `new(inner, pool)` | 创建新的池化 Packet |
| `release(self)` | 手动归还 |
| `release_and_clear(self)` | 归还并清空 |
| `detach(self)` | 提取 Packet，不归还 |
| `pool(&self)` | 获取所属池引用 |

#### Trait 实现

```rust
impl Deref for PooledPacket { /* 自动解引用 */ }
impl DerefMut for PooledPacket { /* 自动可变解引用 */ }
impl Drop for PooledPacket { /* 统计字节数并归还 */ }
```

### 7.5 PacketBuilder

```rust
pub struct PacketBuilder {
    pool: Arc<PacketPool>,
    packet: Option<PooledPacket>,
}
```

#### 流式 API

| 方法 | 说明 |
|------|------|
| `new(pool)` | 创建构建器 |
| `acquire(&mut self)` | 获取 Packet |
| `with_timestamp(self, ts)` | 设置时间戳 |
| `with_interface(self, id)` | 设置接口 ID |
| `with_data(self, data)` | 填充数据 |
| `with_header_reserve(self, len)` | 预留头部空间 |
| `build(self)` | 构建，返回 PooledPacket |

---

## 八、模块导出 (`src/common/pool/mod.rs`)

```rust
mod clear;
mod config;
mod error;
mod packet_pool;
mod pooled;
mod pool;
mod stats;

pub use clear::Clear;
pub use config::{AllocStrategy, PoolConfig, WaitStrategy};
pub use error::PoolError;
pub use packet_pool::{PacketPool, PacketPoolConfig, PacketPoolStats, PacketBuilder, PooledPacket};
pub use pooled::Pooled;
pub use pool::Pool;
pub use stats::{PoolStats, PoolStatus};
```

---

## 九、与现有模块集成

### 9.1 Packet 的 Clear 实现

在 `src/common/packet.rs` 中为 Packet 实现了 Clear trait：

```rust
impl Clear for Packet {
    fn clear(&mut self) {
        self.length = 0;
        self.offset = 0;
        self.layers.clear();

        // 清空协议元数据
        self.eth_src = None;
        self.eth_dst = None;
        // ... 其他字段
    }
}
```

### 9.2 common/mod.rs 导出

```rust
// 导出对象池相关类型
pub use pool::{
    Pool, Pooled, Clear,
    PoolError, AllocStrategy, PoolConfig,
    WaitStrategy as PoolWaitStrategy,  // 避免与 queue 冲突
    PoolStats, PoolStatus,
    PacketPool, PacketPoolConfig, PacketPoolStats,
    PacketBuilder, PooledPacket,
};
```

---

## 十、验证编译

```bash
# 检查编译
cargo check

# 运行测试
cargo test

# 格式化代码
cargo fmt

# 静态检查
cargo clippy
```

---

## 十一、后续工作

当前完成的是对象池基础框架。后续任务：

### 11.1 性能优化
- 无锁队列实现
- 分层池（按对象大小）
- 线程本地池

### 11.2 功能扩展
- 对象池监控接口
- 动态调整池大小
- 对象生命周期追踪

### 11.3 协议层集成
- 在协议处理中使用 PacketPool
- 实现报文接收/发送的池化管理
- 性能测试和基准测试

---

## 十二、设计亮点

1. **零外部依赖**: 仅使用 Rust 标准库
2. **通用性**: Pool<T> 支持任意类型
3. **类型安全**: 充分利用 Rust 类型系统
4. **自动归还**: Drop trait 自动归还对象
5. **中文文档**: 符合学习型项目定位
6. **完整测试**: 每个模块都有单元测试覆盖
