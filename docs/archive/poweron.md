# PowerOn模块实现日志

## 日期
2026-02-12

## 概述

本次实现了 CoreNet 的上电启动模块（PowerOn），负责系统资源的统一初始化和释放。该模块在系统启动时创建 Packet 对象池和收发包队列，在系统关闭时按正确顺序释放所有资源。

---

## 一、模块结构

```
src/common/poweron/
├── mod.rs              # 模块入口，实现 boot() 和 shutdown() 核心接口
├── error.rs            # PowerOnError 错误类型定义
├── state.rs            # SystemState 和 SystemStatus 状态管理
├── config.rs           # SystemConfig 配置结构
├── context.rs          # SystemContext 系统上下文实现
└── presets.rs          # 预设配置（default, high_throughput, low_latency, memory_constrained）
```

---

## 二、错误类型模块 (`src/common/poweron/error.rs`)

### 2.1 核心定义

```rust
#[derive(Debug)]
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

### 2.2 实现特性

- **Display 实现**：提供中文错误描述
- **std::error::Error 实现**：完全兼容 Rust 标准错误处理
- **Debug 派生**：支持错误调试

---

## 三、状态管理模块 (`src/common/poweron/state.rs`)

### 3.1 SystemState

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SystemState {
    /// 运行中
    Running,

    /// 正在关闭
    ShuttingDown,

    /// 已关闭
    Shutdown,
}
```

#### 辅助方法

| 方法 | 说明 |
|------|------|
| `is_running(&self) -> bool` | 判断系统是否运行中 |
| `is_shutdown(&self) -> bool` | 判断系统是否已关闭 |
| `is_shutting_down(&self) -> bool` | 判断系统是否正在关闭 |

### 3.2 SystemStatus

```rust
#[derive(Debug, Clone)]
pub struct SystemStatus {
    /// 系统状态
    pub state: SystemState,

    /// 对象池状态
    pub pool_status: PoolStatus,

    /// 接收队列长度
    pub rxq_len: usize,

    /// 接收队列容量
    pub rxq_capacity: usize,

    /// 发送队列长度
    pub txq_len: usize,

    /// 发送队列容量
    pub txq_capacity: usize,
}
```

#### 辅助方法

| 方法 | 说明 |
|------|------|
| `rxq_utilization(&self) -> f64` | 获取接收队列使用率 |
| `txq_utilization(&self) -> f64` | 获取发送队列使用率 |

---

## 四、配置模块 (`src/common/poweron/config.rs`)

### 4.1 SystemConfig

```rust
#[derive(Debug, Clone)]
pub struct SystemConfig {
    /// Packet 对象池配置
    pub packet_pool: pool::PacketPoolConfig,

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

### 4.2 默认配置

```rust
impl Default for SystemConfig {
    fn default() -> Self {
        Self {
            packet_pool: PacketPoolConfig::default(),
            rxq_capacity: 1024,
            txq_capacity: 1024,
            queue_blocking: true,
            queue_wait_strategy: WaitStrategy::Yield,
            warmup_pool: true,
            warmup_count: 10,
        }
    }
}
```

### 4.3 Builder 方法

| 方法 | 说明 |
|------|------|
| `with_rxq_capacity(capacity)` | 设置接收队列容量 |
| `with_txq_capacity(capacity)` | 设置发送队列容量 |
| `with_queue_capacity(capacity)` | 设置队列容量（RxQ 和 TxQ 相同） |
| `with_packet_pool(config)` | 设置 Packet 对象池配置 |
| `with_blocking(blocking)` | 设置队列阻塞模式 |
| `with_wait_strategy(strategy)` | 设置队列等待策略 |
| `with_warmup(warmup)` | 启用或禁用池预热 |
| `with_warmup_count(count)` | 设置预热对象数量 |

### 4.4 配置验证

```rust
pub fn validate(&self) -> Result<(), String>
```

验证规则：
- 队列容量必须在 `MIN_QUEUE_CAPACITY` 和 `MAX_QUEUE_CAPACITY` 之间
- 预热数量不能超过池容量
- 启用预热时预热数量不能为 0

---

## 五、系统上下文模块 (`src/common/poweron/context.rs`)

### 5.1 SystemContext

```rust
pub struct SystemContext {
    /// Packet 对象池
    pub pool: Arc<PacketPool>,

    /// 接收队列（注入器 -> 处理线程）
    pub rxq: Arc<SpscQueue<Packet>>,

    /// 发送队列（处理线程 -> 输出）
    pub txq: Arc<SpscQueue<Packet>>,

    /// 系统状态
    state: Arc<RwLock<SystemState>>,
}
```

### 5.2 核心方法

| 方法 | 说明 |
|------|------|
| `pool(&self) -> &Arc<PacketPool>` | 获取对象池引用 |
| `rxq(&self) -> &Arc<SpscQueue<Packet>>` | 获取接收队列引用 |
| `txq(&self) -> &Arc<SpscQueue<Packet>>` | 获取发送队列引用 |
| `state(&self) -> SystemState` | 获取系统状态 |
| `is_running(&self) -> bool` | 判断系统是否运行中 |
| `is_shutdown(&self) -> bool` | 判断系统是否已关闭 |
| `status(&self) -> SystemStatus` | 获取系统状态快照 |
| `print_status(&self)` | 打印系统状态到控制台 |
| `set_state(&self, new_state)` | 设置系统状态（内部使用） |

---

## 六、预设配置模块 (`src/common/poweron/presets.rs`)

### 6.1 预设配置

| 配置函数 | Pool容量 | RxQ容量 | TxQ容量 | Packet大小 | 适用场景 |
|----------|----------|---------|---------|-----------|----------|
| `default()` | 100 | 1024 | 1024 | 1514B | 通用场景 |
| `high_throughput()` | 1000 | 8192 | 8192 | 9000B | 大流量传输 |
| `low_latency()` | 500 | 256 | 256 | 1514B | 实时通信 |
| `memory_constrained()` | 50 | 256 | 256 | 1514B | 嵌入式设备 |
| `with_capacity(pool, rxq, txq)` | 自定义 | 自定义 | 自定义 | 1514B | 自定义场景 |

### 6.2 高吞吐量配置特点

- 对象池容量 1000，支持大流量
- 队列容量 8192，减少阻塞
- Packet 大小 9000，支持巨型帧
- 使用 Recent 分配策略，CPU 缓存友好
- 允许溢出，流量高峰时临时扩容

### 6.3 低延迟配置特点

- 使用 LIFO 分配策略，提高缓存命中率
- 队列容量 256，减少排队延迟
- 使用 Spin 等待策略，低延迟场景

### 6.4 内存受限配置特点

- 对象池容量仅 50
- 预留头部空间减少到 64
- 非阻塞模式，避免等待
- 禁用预热，节省初始化内存

---

## 七、核心接口 (`src/common/poweron/mod.rs`)

### 7.1 上电初始化

```rust
pub fn boot(config: SystemConfig) -> Result<SystemContext>
```

**功能：** 使用指定配置初始化系统资源

**流程：**
1. 验证配置
2. 创建 Packet 对象池
3. 预热对象池（可选）
4. 创建接收队列
5. 创建发送队列
6. 创建并返回系统上下文

**错误处理：** 资源创建失败时返回 `CoreError::InvalidConfig` 或 `CoreError::Other`

### 7.2 快速启动

```rust
pub fn boot_default() -> Result<SystemContext>

pub fn boot_with_capacity(pool_cap: usize, rxq_cap: usize, txq_cap: usize) -> Result<SystemContext>
```

**功能：** 使用默认配置或指定容量快速启动系统

### 7.3 下电释放

```rust
pub fn shutdown(context: &mut SystemContext) -> Result<()>
```

**功能：** 优雅关闭系统，释放所有资源

**流程：**
1. 标记系统为 ShuttingDown 状态
2. 关闭发送队列
3. 关闭接收队列
4. 标记系统为 Shutdown 状态

**注意：** 实际的资源释放由 Drop trait 自动处理

---

## 八、模块导出 (`src/common/mod.rs`)

```rust
pub mod poweron;

pub use poweron::{
    SystemConfig,
    SystemContext,
    SystemState,
    SystemStatus,
    boot,
    boot_default,
    boot_with_capacity,
    shutdown,
    default as poweron_default,
    high_throughput,
    low_latency,
    memory_constrained,
    with_capacity as poweron_with_capacity,
};
```

---

## 九、资源所有权模型

```
┌─────────────────────────────────────────────────────┐
│                    SystemContext                  │
│  ┌───────────────┐  ┌───────────────┐  ┌─────┐ │
│  │  PacketPool   │  │      RxQ      │  │ TxQ │ │
│  │  (Arc<Pool>)  │  │  (Arc<SpscQ>) │  │     │ │
│  └───────────────┘  └───────────────┘  └─────┘ │
└─────────────────────────────────────────────────────┘
                          │
          Drop trait 自动清理（按声明逆序）
```

**设计原则：**
- SystemContext 持有所有资源的所有权
- 使用 Rust 的 Drop trait 保证资源自动释放
- 外部模块通过引用访问资源，不持有所有权

---

## 十、单元测试

### 10.1 error.rs 测试

| 测试名称 | 说明 |
|----------|------|
| `test_error_display` | 验证错误类型的 Display 实现 |

### 10.2 state.rs 测试

| 测试名称 | 说明 |
|----------|------|
| `test_system_state` | 验证系统状态判断方法 |
| `test_system_status` | 验证状态快照和使用率计算 |

### 10.3 config.rs 测试

| 测试名称 | 说明 |
|----------|------|
| `test_default_config` | 验证默认配置 |
| `test_config_builders` | 验证 Builder 方法 |
| `test_config_validate` | 验证配置验证逻辑 |

### 10.4 context.rs 测试

| 测试名称 | 说明 |
|----------|------|
| `test_context_creation` | 验证上下文创建 |
| `test_context_status` | 验证状态获取 |

### 10.5 presets.rs 测试

| 测试名称 | 说明 |
|----------|------|
| `test_preset_default` | 验证默认预设 |
| `test_preset_high_throughput` | 验证高吞吐量预设 |
| `test_preset_low_latency` | 验证低延迟预设 |
| `test_preset_memory_constrained` | 验证内存受限预设 |
| `test_preset_with_capacity` | 验证自定义容量预设 |

### 10.6 mod.rs 测试

| 测试名称 | 说明 |
|----------|------|
| `test_boot_default` | 验证默认启动 |
| `test_boot_with_capacity` | 验证自定义容量启动 |
| `test_boot_shutdown` | 验证启动和关闭流程 |
| `test_boot_invalid_config` | 验证无效配置处理 |
| `test_context_status` | 验证状态获取 |
| `test_preset_*` | 验证各预设配置 |

---

## 十一、使用示例

### 11.1 基本使用

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

### 11.2 与处理线程集成

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

### 11.3 使用预设配置

```rust
// 高吞吐量场景
let context = poweron::boot(poweron::high_throughput())?;

// 低延迟场景
let context = poweron::boot(poweron::low_latency())?;

// 内存受限场景
let context = poweron::boot(poweron::memory_constrained())?;
```

---

## 十二、设计亮点

1. **统一入口**：所有资源通过 `boot()` 函数统一创建
2. **自动清理**：利用 Drop trait 确保资源释放
3. **配置驱动**：通过 SystemConfig 灵活控制资源参数
4. **状态可见**：提供 SystemStatus 实时监控系统状态
5. **线程安全**：Arc 包装支持多线程共享访问
6. **优雅关闭**：shutdown() 支持优雅关闭和资源回收
7. **预设配置**：提供多种场景的预设配置，简化使用
8. **完整测试**：每个模块都有单元测试覆盖

---

## 十三、后续工作

当前完成的是上电启动模块基础框架。后续任务：

### 13.1 功能扩展
- 支持动态调整资源参数
- 添加更多监控指标
- 支持热重载配置

### 13.2 协议层集成
- 在协议处理引擎中使用 SystemContext
- 实现报文接收/发送的池化管理
- 性能测试和基准测试

### 13.3 错误处理增强
- 更详细的错误信息
- 支持错误恢复策略
