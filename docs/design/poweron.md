# 上电启动模块设计

## 概述

上电启动模块（PowerOn）负责 CoreNet 系统的资源初始化和释放。该模块在系统启动时统一创建 Packet 对象池和收发包队列，在系统关闭时按正确顺序释放所有资源。

---

## 一、架构设计

### 1.1 资源所有权模型

```
┌─────────────────────────────────────────────────────────────┐
│                    SystemContext                          │
│  ┌───────────────┐  ┌───────────────┐  ┌───────────────┐ │
│  │  PacketPool   │  │      RxQ      │  │      TxQ      │ │
│  │  (Arc<Pool>)  │  │  (Arc<SpscQ>) │  │  (Arc<SpscQ>) │ │
│  └───────────────┘  └───────────────┘  └───────────────┘ │
└─────────────────────────────────────────────────────────────┘
                          │
          Drop trait 自动清理（按声明逆序）
```

**设计原则：**
- SystemContext 持有所有资源的所有权
- 使用 Rust 的 Drop trait 保证资源自动释放
- 外部模块通过引用访问资源，不持有所有权

### 1.2 上电/下电流程

```
上电流程：
  boot(config) -> SystemContext { pool, rxq, txq }
       ↓
  1. 创建 PacketPool
  2. 创建 RxQ (接收队列)
  3. 创建 TxQ (发送队列)
  4. 预热对象池（可选）
  5. 返回 SystemContext

下电流程：
  context.drop() 或 shutdown(&mut context)
       ↓
  1. 关闭 TxQ (停止接收新报文)
  2. 关闭 RxQ (停止注入新报文)
  3. 等待队列排空（可选）
  4. 关闭 PacketPool
```

---

## 二、核心数据结构

### 2.1 SystemConfig

系统配置结构，定义资源创建参数。

```rust
pub struct SystemConfig {
    /// Packet 对象池配置
    pub packet_pool: PacketPoolConfig,

    /// 接收队列容量
    pub rxq_capacity: usize,

    /// 发送队列容量
    pub txq_capacity: usize,

    /// 是否阻塞模式
    pub queue_blocking: bool,

    /// 队列等待策略
    pub queue_wait_strategy: queue::WaitStrategy,

    /// 是否预热对象池
    pub warmup_pool: bool,

    /// 预热对象数量
    pub warmup_count: usize,
}
```

### 2.2 SystemContext

系统上下文，持有所有资源的所有权。

```rust
pub struct SystemContext {
    /// Packet 对象池
    pub pool: Arc<PacketPool>,

    /// 接收队列（注入器 -> 处理线程）
    pub rxq: Arc<SpscQueue<Packet>>,

    /// 发送队列（处理线程 -> 输出）
    pub txq: Arc<SpscQueue<Packet>>,

    /// 系统状态
    state: SystemState,
}

pub enum SystemState {
    /// 运行中
    Running,
    /// 正在关闭
    ShuttingDown,
    /// 已关闭
    Shutdown,
}
```

### 2.3 SystemState

系统运行状态。

```rust
pub enum SystemState {
    /// 运行中
    Running,

    /// 正在关闭
    ShuttingDown,

    /// 已关闭
    Shutdown,
}
```

---

## 三、接口设计

### 3.1 上电初始化

```rust
pub fn boot(config: SystemConfig) -> Result<SystemContext>
```

**功能：** 使用指定配置初始化系统资源

**返回：** 包含所有资源的 SystemContext

**错误：** 资源创建失败时返回错误

### 3.2 下电释放

```rust
pub fn shutdown(context: &mut SystemContext) -> Result<()>
```

**功能：** 优雅关闭系统，释放所有资源

**流程：**
1. 标记系统为 ShuttingDown 状态
2. 关闭队列（停止接收新数据）
3. 等待队列排空（可选超时）
4. 标记系统为 Shutdown 状态

### 3.3 快速启动

```rust
pub fn boot_default() -> Result<SystemContext>

pub fn boot_with_capacity(pool_cap: usize, rxq_cap: usize, txq_cap: usize) -> Result<SystemContext>
```

**功能：** 使用默认配置或指定容量快速启动系统

---

## 四、资源访问接口

### 4.1 SystemContext 方法

| 方法 | 说明 |
|------|------|
| `pool(&self) -> &Arc<PacketPool>` | 获取对象池引用 |
| `rxq(&self) -> &Arc<SpscQueue<Packet>>` | 获取接收队列引用 |
| `txq(&self) -> &Arc<SpscQueue<Packet>>` | 获取发送队列引用 |
| `state(&self) -> SystemState` | 获取系统状态 |
| `is_running(&self) -> bool` | 判断系统是否运行中 |
| `is_shutdown(&self) -> bool` | 判断系统是否已关闭 |

### 4.2 统计信息

```rust
impl SystemContext {
    /// 获取系统状态快照
    pub fn status(&self) -> SystemStatus;

    /// 打印系统状态到控制台
    pub fn print_status(&self);
}
```

### 4.3 SystemStatus

系统状态快照。

```rust
pub struct SystemStatus {
    /// 系统状态
    pub state: SystemState,

    /// 对象池状态
    pub pool_status: PoolStatus,

    /// 接收队列长度
    pub rxq_len: usize,

    /// 发送队列长度
    pub txq_len: usize,
}
```

---

## 五、默认配置

### 5.1 SystemContext::default()

```rust
impl Default for SystemConfig {
    fn default() -> Self {
        Self {
            packet_pool: PacketPoolConfig::default(),
            rxq_capacity: 1024,
            txq_capacity: 1024,
            queue_blocking: true,
            queue_wait_strategy: queue::WaitStrategy::Yield,
            warmup_pool: true,
            warmup_count: 10,
        }
    }
}
```

### 5.2 预设配置

| 配置名称 | Pool容量 | RxQ容量 | TxQ容量 | 适用场景 |
|----------|----------|---------|---------|----------|
| `default()` | 100 | 1024 | 1024 | 通用场景 |
| `high_throughput()` | 1000 | 8192 | 8192 | 大流量场景 |
| `low_latency()` | 500 | 256 | 256 | 低延迟场景 |
| `memory_constrained()` | 50 | 256 | 256 | 内存受限场景 |

---

## 六、错误处理

### 6.1 PowerOnError

```rust
pub enum PowerOnError {
    /// 配置无效
    InvalidConfig(String),

    /// 对象池创建失败
    PoolCreationFailed(String),

    /// 队列创建失败
    QueueCreationFailed(String),

    /// 系统已关闭
    SystemShutdown,

    /// 其他错误
    Other(String),
}
```

### 6.2 错误处理原则

1. **创建时失败**：立即返回错误，不释放已创建资源（由 Drop 处理）
2. **运行时错误**：记录日志，尝试继续运行
3. **关闭时错误**：记录警告，强制释放资源

---

## 七、线程安全

### 7.1 共享访问

所有资源通过 `Arc` 包装，可安全地在多线程间共享：

```rust
let context = boot(SystemConfig::default())?;

// 处理线程获取资源引用
let rxq = context.rxq.clone();
let txq = context.txq.clone();
let pool = context.pool.clone();

// 在新线程中使用
thread::spawn(move || {
    // 使用 rxq, txq, pool
});
```

### 7.2 状态管理

SystemState 使用 `Mutex<RwLock<SystemState>>` 保护：
- 多个读者可同时读取
- 写者独占访问
- 保证状态变更的原子性

---

## 八、模块结构

```
src/common/poweron/
├── mod.rs              # 模块入口，导出公共接口
├── config.rs           # SystemConfig 定义
├── context.rs          # SystemContext 实现
├── error.rs            # PowerOnError 定义
├── state.rs            # SystemState 和 SystemStatus
└── presets.rs          # 预设配置
```

### 8.1 模块导出

```rust
// src/common/poweron/mod.rs
mod config;
mod context;
mod error;
mod state;
mod presets;

pub use config::SystemConfig;
pub use context::SystemContext;
pub use error::PowerOnError;
pub use state::{SystemState, SystemStatus};

// 核心接口
pub use presets::{default, high_throughput, low_latency, memory_constrained};
pub fn boot(config: SystemConfig) -> Result<SystemContext>;
pub fn shutdown(context: &mut SystemContext) -> Result<()>;
```

---

## 九、使用示例

### 9.1 基本使用

```rust
use core_net::common::poweron;

// 上电初始化
let config = poweron::SystemConfig::default();
let mut context = poweron::boot(config)?;

// 使用资源
let packet = context.pool.acquire()?;
context.rxq.enqueue(packet)?;

// 检查状态
context.print_status();

// 下电释放
poweron::shutdown(&mut context)?;
```

### 9.2 与处理线程集成

```rust
let context = poweron::boot_default()?;

// 处理线程
let rxq = context.rxq.clone();
let txq = context.txq.clone();
let pool = context.pool.clone();

thread::spawn(move || {
    while context.is_running() {
        if let Ok(Some(packet)) = rxq.dequeue() {
            // 处理报文...
            txq.enqueue(packet)?;
        }
    }
});
```

---

## 十、设计亮点

1. **统一入口**：所有资源通过 `boot()` 函数统一创建
2. **自动清理**：利用 Drop trait 确保资源释放
3. **配置驱动**：通过 SystemConfig 灵活控制资源参数
4. **状态可见**：提供 SystemStatus 实时监控系统状态
5. **线程安全**：Arc 包装支持多线程共享访问
6. **优雅关闭**：shutdown() 支持优雅关闭和资源回收
