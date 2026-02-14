# 上电启动模块设计

## 概述

上电启动模块（PowerOn）负责 CoreNet 系统资源的初始化和释放。在系统启动时创建收发队列，在系统关闭时释放资源。

---

## 一、架构设计

### 资源所有权模型

```
┌─────────────────────────────────────┐
│         SystemContext              │
│  ┌───────────┐    ┌───────────┐  │
│  │    RxQ    │    │    TxQ    │  │
│  │(RingQueue)│    │(RingQueue)│  │
│  └───────────┘    └───────────┘  │
└─────────────────────────────────────┘
```

### 上电/下电流程

```
上电：boot(config) -> SystemContext
     ↓
  1. 创建 RxQ (RingQueue<Packet>)
  2. 创建 TxQ (RingQueue<Packet>)
  3. 返回 SystemContext

下电：shutdown(context)
     ↓
  1. 清空 RxQ
  2. 清空 TxQ
  3. 释放资源
```

### 扩展性说明

**设计原则：** 保持简单，优先可读性

CoreNet 是教育性质项目，资源数量有限（预计 <10 个）。当需要添加新资源时：

```rust
// 示例：添加 PacketPool
pub struct SystemConfig {
    pub rxq_capacity: usize,
    pub txq_capacity: usize,
    pub pool_capacity: usize,        // 新增
}

pub struct SystemContext {
    pub rxq: RingQueue<Packet>,
    pub txq: RingQueue<Packet>,
    pub pool: PacketPool,              // 新增
}
```

**添加新资源步骤：**
1. 在 `SystemConfig` 添加配置字段
2. 在 `SystemContext` 添加资源字段
3. 在 `boot()` 初始化资源
4. 在 `shutdown()` 释放资源（如需要）

---

## 二、核心数据结构

### SystemConfig

系统配置结构，定义队列创建参数。

```rust
pub struct SystemConfig {
    /// 接收队列容量
    pub rxq_capacity: usize,

    /// 发送队列容量
    pub txq_capacity: usize,
}
```

### SystemContext

系统上下文，持有队列资源的所有权。

```rust
pub struct SystemContext {
    /// 接收队列（注入器 -> 处理线程）
    pub rxq: RingQueue<Packet>,

    /// 发送队列（处理线程 -> 输出）
    pub txq: RingQueue<Packet>,
}
```

---

## 三、接口定义

### 3.1 上电初始化

```rust
pub fn boot(config: SystemConfig) -> SystemContext
```

**功能：** 使用指定配置初始化系统资源

**返回：** 包含队列资源的 SystemContext

### 3.2 下电释放

```rust
pub fn shutdown(context: &mut SystemContext)
```

**功能：** 释放系统资源

**流程：**
1. 清空 RxQ
2. 清空 TxQ

### 3.3 快捷方法

```rust
pub fn boot_default() -> SystemContext

pub fn boot_with_capacity(rxq_cap: usize, txq_cap: usize) -> SystemContext
```

**功能：** 使用默认配置或指定容量快速启动

---

## 四、默认配置

```rust
impl Default for SystemConfig {
    fn default() -> Self {
        Self {
            rxq_capacity: 256,
            txq_capacity: 256,
        }
    }
}
```

---

## 五、辅助接口

### SystemContext 方法

| 方法 | 说明 |
|------|------|
| `rxq_len(&self) -> usize` | 接收队列当前长度 |
| `txq_len(&self) -> usize` | 发送队列当前长度 |
| `rxq_is_empty(&self) -> bool` | 接收队列是否为空 |
| `txq_is_empty(&self) -> bool` | 发送队列是否为空 |
| `rxq_is_full(&self) -> bool` | 接收队列是否已满 |
| `txq_is_full(&self) -> bool` | 发送队列是否已满 |

---

## 六、模块结构

```
src/common/poweron/
├── mod.rs       # 模块入口
├── config.rs    # SystemConfig 定义
├── context.rs   # SystemContext 实现
└── lib.rs      # 公共接口导出
```

### 模块导出

```rust
pub use config::SystemConfig;
pub use context::SystemContext;

pub fn boot(config: SystemConfig) -> SystemContext;
pub fn shutdown(context: &mut SystemContext);
pub fn boot_default() -> SystemContext;
pub fn boot_with_capacity(rxq_cap: usize, txq_cap: usize) -> SystemContext;
```
