# 上电启动模块设计

## 概述

上电启动模块（PowerOn）负责 CoreNet 系统资源的初始化和释放。在系统启动时调用 interface 模块的默认初始化接口，在系统关闭时释放资源。

**设计原则：**
- `poweron` 模块只负责系统启动和下电
- 接口配置文件路径由 `interface` 模块自己管理
- 队列容量配置由 `interface` 模块管理

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

**职责分离**：
- `interface` 模块管理配置文件路径和队列容量配置
- `poweron` 模块只负责调用 interface 的初始化接口

### 上电/下电流程

```
上电：boot_default() -> SystemContext
     ↓
  1. 调用 interface::init_default() 初始化全局接口管理器
  2. 调用 interface::load_default_config() 加载配置
  3. 从默认配置文件读取队列容量配置
  4. 为每个接口创建独立的 RxQ 和 TxQ
  5. 返回 SystemContext

下电：shutdown(context)
     ↓
  1. 清空所有接口的 RxQ
  2. 清空所有接口的 TxQ
  3. 释放队列资源
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

---

## 二、核心数据结构

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

### 3.1 系统启动

```rust
pub fn boot_default() -> SystemContext
```

**功能：** 使用默认配置初始化系统资源和接口

**返回：** 包含接口管理器和队列资源的 SystemContext

**流程：**
1. 调用 `interface::init_default()` 初始化全局接口管理器
2. 调用 `interface::load_default_config()` 加载配置
3. 从默认配置文件 `src/interface/interface.toml` 读取队列容量配置
4. 为每个接口创建独立的 RxQ 和 TxQ
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

---

## 四、辅助接口

### SystemContext 方法

| 方法 | 说明 |
|------|------|
| `interface_count(&self) -> usize` | 获取接口数量 |
| `get_interface(&self, name: &str) -> Option<&NetworkInterface>` | 通过名称获取接口 |
| `get_interface_mut(&mut self, name: &str) -> Option<&mut NetworkInterface>` | 通过名称获取可变接口 |
| `get_interface_by_index(&self, index: u32) -> Option<&NetworkInterface>` | 通过索引获取接口 |
| `get_interface_by_index_mut(&mut self, index: u32) -> Option<&mut NetworkInterface>` | 通过索引获取可变接口 |

### 全局接口访问

任何模块都可以访问全局接口管理器：

```rust
// 获取全局接口管理器引用
let manager = interface::global_manager()?;

// 查询接口
let eth0 = manager.get_by_name("eth0")?;
let rxq = &eth0.rxq;
let txq = &eth0.txq;

// 操作队列
rxq.enqueue(packet)?;
let packet = txq.dequeue()?;
```

---

## 五、模块结构

```
src/poweron/
├── mod.rs       # 模块入口，导出 boot_default() 和 shutdown()
└── context.rs   # SystemContext 实现

src/interface/
├── mod.rs           # 模块入口，导出公共类型
├── types.rs         # MacAddr, Ipv4Addr, InterfaceState 等类型定义
├── iface.rs         # NetworkInterface 实现（包含队列）
├── manager.rs       # InterfaceManager 实现
├── config.rs        # 接口配置文件加载
├── interface.toml   # 接口配置文件（包含队列配置）
└── global.rs        # 全局接口管理器（OnceLock）
```

### 模块导出

**poweron 模块导出：**
```rust
pub use context::SystemContext;

pub fn boot_default() -> SystemContext;
pub fn shutdown(context: &mut SystemContext);
```

**interface 模块导出：**
```rust
pub use types::{MacAddr, Ipv4Addr, InterfaceState, InterfaceType, InterfaceError};
pub use iface::{NetworkInterface, InterfaceConfig};
pub use manager::InterfaceManager;
pub use config::{load_default_config, save_config, DEFAULT_CONFIG_PATH};
pub use global::{
    init_global_manager,
    init_default,
    global_manager,
    update_interface,
    set_interface_ip,
    set_interface_mac,
    set_interface_netmask,
    set_interface_gateway,
    set_interface_mtu,
    interface_up,
    interface_down,
};
```

---

## 六、设计变更说明

### 变更原因

1. **职责分离**：配置文件路径和队列容量应该由 interface 模块管理，而不是 poweron 模块
2. **简化接口**：poweron 模块只提供唯一的启动方式 `boot_default()`，简化 API
3. **消除冗余**：删除不需要的 `SystemConfig` 和 `boot()` 函数

### 当前设计

- **配置文件路径**：由 `interface` 模块的 `DEFAULT_CONFIG_PATH` 常量定义
- **队列容量配置**：在 `src/interface/interface.toml` 的 `[queue]` 部分配置
- **系统启动**：只需调用 `boot_default()`，无需传递任何参数

### 使用示例

```rust
// 系统启动
let context = core_net::boot_default();

// 使用接口
let eth0 = context.get_interface_by_index(0).unwrap();
println!("接口: {}, 队列容量: {}", eth0.name, eth0.rxq.capacity());

// 系统下电
core_net::shutdown(&mut context);
```

---

## 七、测试策略

### 7.1 单元测试

#### 7.1.1 测试范围

**SystemContext 基础功能测试**：
- **正常路径**：创建上下文、获取接口数量、通过名称/索引获取接口
- **边界条件**：空上下文、单接口上下文、多接口上下文
- **错误路径**：不存在的接口名称、越界的接口索引

**boot_default() 测试**：
- **正常路径**：默认配置文件存在、正确加载所有接口、队列容量正确
- **边界条件**：配置文件为空（无接口）、单个接口、最大接口数
- **错误路径**：配置文件不存在、配置文件格式错误、配置值无效

**shutdown() 测试**：
- **正常路径**：正常关闭、所有队列被清空、资源正确释放
- **边界条件**：空上下文的关闭、已关闭的上下文再次关闭
- **错误路径**：队列清空失败（理论上不应发生）

#### 7.1.2 测试组织

测试代码按以下类别组织：

- **SystemContext 基础测试组**：创建、查询、空管理器
- **boot_default() 测试组**：成功加载、队列容量、接口状态、多接口
- **shutdown() 测试组**：队列清空、空上下文、多次关闭、资源清理
- **集成测试组**：完整上电下电循环、修改后关闭

测试辅助函数：
- `create_test_context()`: 创建包含 eth0 和 lo 的测试上下文
- `create_test_context_with_packets()`: 创建带报文的上下文
- `create_empty_context()`: 创建空上下文
- `create_test_manager()`: 创建测试用接口管理器
- `create_eth0_config()`: 创建 eth0 配置
- `create_lo_config()`: 创建 lo 配置

#### 7.1.3 测试覆盖要点

| 测试维度 | 覆盖要点 |
|---------|---------|
| **公共接口** | `boot_default()`, `shutdown()`<br>`SystemContext` 的所有查询方法 |
| **内部逻辑** | 接口管理器的创建和传递<br>队列资源的初始化<br>资源清理逻辑 |
| **边界条件** | 空配置、单接口、多接口<br>队列容量最小/最大值 |
| **错误处理** | 配置文件读取失败<br>配置文件解析失败<br>接口名称不存在 |
| **资源管理** | 队列的清空<br>内存的正确释放 |

### 7.2 集成测试

#### 7.2.1 测试场景

**场景一：完整的系统生命周期**
- **涉及模块**：poweron、interface
- **测试内容**：
  - 调用 `boot_default()` 启动系统
  - 验证所有接口正确初始化
  - 验证队列容量符合配置
  - 通过全局接口管理器访问接口
  - 调用 `shutdown()` 释放资源
  - 验证可以重新启动系统

**场景二：与调度器集成**
- **涉及模块**：poweron、scheduler、engine
- **测试内容**：
  - 启动系统获取上下文
  - 创建调度器
  - 向接口 RxQ 注入报文
  - 运行调度器处理报文
  - 验证 TxQ 中的响应报文
  - 正确关闭系统

**场景三：与全局管理器集成**
- **涉及模块**：poweron、interface（全局管理器）
- **测试内容**：
  - 启动系统
  - 通过 `global_manager()` 访问接口
  - 修改接口配置
  - 验证修改生效
  - 关闭系统

#### 7.2.2 测试依赖

- **配置文件**：`src/interface/interface.toml`
- **interface 模块**：正确实现接口管理和全局访问
- **测试数据**：辅助函数创建的测试配置

### 7.3 测试数据设计

#### 7.3.1 测试数据来源

- **默认配置文件**：使用真实的 `interface.toml`
- **辅助构造函数**：`create_eth0_config()`, `create_lo_config()` 等
- **测试专用配置**：边界值配置（空接口、最大接口数等）

#### 7.3.2 测试数据管理

提供辅助函数用于创建测试数据：

- `create_test_context()`: 创建包含 eth0 和 lo 的测试上下文
- `create_empty_context()`: 创建空上下文
- `create_test_manager()`: 创建预配置的接口管理器
- `create_eth0_config()`: 创建 eth0 配置对象
- `create_lo_config()`: 创建 lo 配置对象

### 7.4 测试执行计划

```bash
# 运行 poweron 模块所有测试
cargo test poweron

# 运行特定测试
cargo test test_boot_default_success
cargo test test_shutdown_clears_queues

# 显示测试输出
cargo test -- --nocapture

# 运行文档测试
cargo test --doc
```

---

## 八、设计原则

1. **职责单一**：poweron 模块只负责系统启动和下电
2. **配置自治**：interface 模块管理自己的配置文件和队列配置
3. **接口简洁**：只提供 `boot_default()` 和 `shutdown()` 两个公共函数
4. **零依赖**：仅使用 Rust 标准库
5. **可读性优先**：代码结构清晰，便于学习
