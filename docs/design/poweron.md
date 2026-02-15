# 上电启动模块设计

## 概述

上电启动模块（PowerOn）负责 CoreNet 系统资源的初始化和释放。在系统启动时创建接口管理器并为每个接口创建收发队列，在系统关闭时释放资源。

---

## 一、架构设计

### 资源所有权模型

```
┌─────────────────────────────────────────────────────────────────┐
│                      SystemContext                              │
│                                                                 │
│  ┌─────────────────────────────────────────────────────────┐  │
│  │              InterfaceManager                            │  │
│  │                                                          │  │
│  │  ┌─────────────────┐  ┌─────────────────┐              │  │
│  │  │  Interface 0    │  │  Interface 1    │              │  │
│  │  │  (eth0)         │  │  (lo)           │  ...         │  │
│  │  │                 │  │                 │              │  │
│  │  │  ┌───────────┐  │  │  ┌───────────┐  │              │  │
│  │  │  │   RxQ0    │  │  │  │   RxQ1    │  │              │  │
│  │  │  │(RingQueue)│  │  │  │(RingQueue)│  │              │  │
│  │  │  └───────────┘  │  │  └───────────┘  │              │  │
│  │  │                 │  │                 │              │  │
│  │  │  ┌───────────┐  │  │  ┌───────────┐  │              │  │
│  │  │  │   TxQ0    │  │  │  │   TxQ1    │  │              │  │
│  │  │  │(RingQueue)│  │  │  │(RingQueue)│  │              │  │
│  │  │  └───────────┘  │  │  └───────────┘  │              │  │
│  │  └─────────────────┘  └─────────────────┘              │  │
│  └─────────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────┘
```

### 设计原则

**每个接口独立队列**：
- 每个网络接口拥有独立的收包队列（RxQ）和发包队列（TxQ）
- 不同接口的报文流相互隔离，提高并发处理能力
- 符合真实网络设备的驱动模型

**全局接口管理器**：
- 通过 `OnceLock` 提供全局访问接口
- 所有协议模块可以直接查询接口信息和队列
- 线程安全的单例模式

### 上电/下电流程

```
上电：boot(config) -> SystemContext
     ↓
  1. 读取接口配置文件（interface.toml）
  2. 解析接口配置，创建 InterfaceManager
  3. 为每个接口创建独立的 RxQ 和 TxQ
  4. 初始化全局接口管理器（OnceLock）
  5. 返回 SystemContext

下电：shutdown(context)
     ↓
  1. 清空所有接口的 RxQ
  2. 清空所有接口的 TxQ
  3. 释放队列资源
  4. 释放接口管理器资源
```

### 报文流向

```
外部注入 -> 接口 RxQ -> 协议处理 -> 接口 TxQ -> 输出
   │            │           │            │
   │            ▼           ▼            │
   │        ┌─────────────────────────┐  │
   │        │   协议栈处理引擎         │  │
   │        │  (Engine/Processor)     │  │
   │        └─────────────────────────┘  │
   │                                     │
   └─────> 接口选择（根据路由） ────────┘
```

### 接口与队列绑定

**绑定关系**：
- 接口索引（Interface Index）与队列一一对应
- `Interface[0]` 拥有 `RxQ[0]` 和 `TxQ[0]`
- 接口通过索引直接访问自己的队列

**队列分配策略**：
- 所有接口使用相同的队列容量配置
- 队列容量在 SystemConfig 中统一指定
- 添加新接口时自动分配新队列

---

## 二、核心数据结构

### SystemConfig

系统配置结构，定义队列创建参数和接口配置路径。

```rust
pub struct SystemConfig {
    /// 接口配置文件路径
    pub interface_config_path: String,

    /// 每个接口的接收队列容量
    pub rxq_capacity: usize,

    /// 每个接口的发送队列容量
    pub txq_capacity: usize,
}
```

### SystemContext

系统上下文，持有接口管理器和所有接口队列资源的所有权。

```rust
pub struct SystemContext {
    /// 接口管理器（包含所有接口及其队列）
    pub interfaces: InterfaceManager,
}
```

**注意**：队列现在由接口管理器内部的每个接口持有，SystemContext 只持有接口管理器的所有权。

---

## 三、接口定义

### 3.1 上电初始化

```rust
pub fn boot(config: SystemConfig) -> SystemContext
```

**功能：** 使用指定配置初始化系统资源和接口

**参数：**
- `config`: 系统配置（接口配置路径、队列容量）

**返回：** 包含接口管理器和队列资源的 SystemContext

**流程：**
1. 读取接口配置文件
2. 解析 TOML 格式的接口配置
3. 为每个接口创建独立的 RxQ 和 TxQ
4. 初始化全局接口管理器（存储到 OnceLock）
5. 返回 SystemContext

### 3.2 下电释放

```rust
pub fn shutdown(context: &mut SystemContext)
```

**功能：** 释放系统资源

**流程：**
1. 清空所有接口的接收队列
2. 清空所有接口的发送队列
3. 释放队列内存
4. （可选）保存接口配置到文件

### 3.3 快捷方法

```rust
pub fn boot_default() -> SystemContext

pub fn boot_with_config(config_path: &str) -> SystemContext

pub fn boot_with_capacity(config_path: &str, rxq_cap: usize, txq_cap: usize) -> SystemContext
```

**功能：** 使用默认配置或指定参数快速启动

---

## 四、默认配置

```rust
impl Default for SystemConfig {
    fn default() -> Self {
        Self {
            interface_config_path: "src/config/interface.toml".to_string(),
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
| `interface_count(&self) -> usize` | 获取接口数量 |
| `get_interface(&self, name: &str) -> Option<&NetworkInterface>` | 通过名称获取接口 |
| `get_interface_mut(&mut self, name: &str) -> Option<&mut NetworkInterface>` | 通过名称获取可变接口 |
| `get_interface_by_index(&self, index: u32) -> Option<&NetworkInterface>` | 通过索引获取接口 |
| `get_interface_by_index_mut(&mut self, index: u32) -> Option<&mut NetworkInterface>` | 通过索引获取可变接口 |

### 接口队列访问接口

每个接口提供队列访问方法：

| 方法 | 说明 |
|------|------|
| `rxq(&self) -> &RingQueue<Packet>` | 获取接收队列引用 |
| `rxq_mut(&mut self) -> &mut RingQueue<Packet>` | 获取接收队列可变引用 |
| `txq(&self) -> &RingQueue<Packet>` | 获取发送队列引用 |
| `txq_mut(&mut self) -> &mut RingQueue<Packet>` | 获取发送队列可变引用 |
| `rxq_len(&self) -> usize` | 接收队列当前长度 |
| `txq_len(&self) -> usize` | 发送队列当前长度 |
| `rxq_enqueue(&mut self, packet: Packet) -> Result<(), QueueError>` | 向接收队列加入报文 |
| `rxq_dequeue(&mut self) -> Option<Packet>` | 从接收队列取出报文 |
| `txq_enqueue(&mut self, packet: Packet) -> Result<(), QueueError>` | 向发送队列加入报文 |
| `txq_dequeue(&mut self) -> Option<Packet>` | 从发送队列取出报文 |

### 全局接口访问

任何模块都可以访问全局接口管理器：

```rust
// 获取全局接口管理器引用
let manager = interface::global_manager()?;

// 查询接口
let eth0 = manager.get_by_name("eth0")?;
let rxq = eth0.rxq();
let txq = eth0.txq();

// 操作队列
rxq.enqueue(packet)?;
let packet = txq.dequeue()?;
```

---

## 六、模块结构

```
src/common/poweron/
├── mod.rs       # 模块入口，公共接口导出
├── config.rs    # SystemConfig 定义
└── context.rs   # SystemContext 实现

src/interface/
├── mod.rs           # 模块入口，公共类型导出
├── types.rs         # MacAddr, Ipv4Addr, InterfaceState 等类型定义
├── iface.rs         # NetworkInterface 实现（包含队列）
├── manager.rs       # InterfaceManager 实现
├── config.rs        # 接口配置文件加载
└── global.rs        # 全局接口管理器（OnceLock）
```

### 模块导出

**poweron 模块导出：**
```rust
pub use config::SystemConfig;
pub use context::SystemContext;

pub fn boot(config: SystemConfig) -> SystemContext;
pub fn shutdown(context: &mut SystemContext);
pub fn boot_default() -> SystemContext;
pub fn boot_with_config(config_path: &str) -> SystemContext;
pub fn boot_with_capacity(config_path: &str, rxq_cap: usize, txq_cap: usize) -> SystemContext;
```

**interface 模块导出：**
```rust
pub use types::{MacAddr, Ipv4Addr, InterfaceState, InterfaceType, InterfaceError};
pub use iface::{NetworkInterface, InterfaceConfig};
pub use manager::InterfaceManager;
pub use config::{load_config, save_config};
pub use global::{init_global_manager, init_from_config, global_manager};
```

---

## 七、设计变更说明

### 变更原因

1. **支持多接口**：真实网络设备有多个网络接口，每个接口需要独立的队列
2. **隔离报文流**：不同接口的报文流相互隔离，提高安全性和可维护性
3. **符合实际模型**：真实网络接口卡的驱动程序为每个接口维护独立的收发队列

### 兼容性说明

- 旧版本的 SystemContext 中的全局 `rxq` 和 `txq` 已移除
- 队列现在由接口管理器内部的每个接口持有
- 需要通过接口名称或索引来访问特定接口的队列
- 全局接口管理器提供线程安全的访问接口

### 迁移指南

**旧代码：**
```rust
let context = boot_default();
context.rxq.enqueue(packet)?;
let output = context.txq.dequeue()?;
```

**新代码：**
```rust
let context = boot_default();
let manager = interface::global_manager()?;
let eth0 = manager.get_by_name("eth0")?;
eth0.rxq_mut().enqueue(packet)?;
let output = eth0.txq_mut().dequeue()?;
```
