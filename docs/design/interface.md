# 网络接口模块设计

## 概述

网络接口模块（Interface）负责管理网络接口的配置和状态。每个接口拥有独立的属性（MAC地址、IP地址、MTU等）和状态（启用/禁用）。模块支持从默认配置文件加载接口配置，提供运行时动态修改接口配置的接口，并提供全局访问能力。

**当前阶段目标：** 实现全局接口管理，支持上电时从配置文件加载，所有模块可直接访问接口信息。

**设计原则：**
- 配置文件路径由 interface 模块自己管理
- 队列容量配置在 interface.toml 中
- 只提供默认配置文件加载，不提供指定路径的加载接口

---

## 一、需求介绍

### 1.1 功能需求

- **需求1**：接口需要维护自己的状态（启用/禁用/错误等）
- **需求2**：接口需要存储网络地址信息（MAC地址、IPv4地址）
- **需求3**：接口需要存储网络参数（MTU、接口名称、索引）
- **需求4**：系统上电时从默认配置文件加载接口配置
- **需求5**：提供接口查询接口属性（状态、地址等）
- **需求6**：提供接口修改接口配置（IP、MAC、状态等）
- **需求7**：支持多接口管理
- **需求8**：提供全局接口管理器，所有模块可直接访问

### 1.2 非功能需求

- **零依赖**：仅使用 Rust 标准库
- **纯内存模拟**：无真实网络设备操作
- **可读性优先**：代码结构清晰，便于学习
- **类型安全**：利用 Rust 类型系统保证接口操作的安全性
- **线程安全**：全局接口管理器使用 `OnceLock<Mutex<>>` 保证线程安全，支持运行时修改接口配置

---

## 二、架构设计

### 2.1 模块定位

网络接口模块位于协议栈的底层，为上层协议提供接口信息，同时与上电启动模块集成。通过全局接口管理器，所有模块都可以直接访问接口信息。

```
┌────────────────────────────────────────────────────────────────┐
│                        全局访问层                               │
│  ┌──────────────────────────────────────────────────────────┐  │
│  │       全局接口管理器 (OnceLock<Mutex<InterfaceManager>>) │  │
│  │              init_default() -> &InterfaceManager          │  │
│  └──────────────────────────────────────────────────────────┘  │
└────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                    System Context                           │
│  ┌───────────┐  ┌───────────┐                                │
│  │    RxQ    │  │    TxQ    │                                │
│  │           │  │           │                                │
│  └───────────┘  └───────────┘                                │
└─────────────────────────────────────────────────────────────┘
        │
        ▼
┌─────────────┐
│  Protocol  │
│   Stack    │
└─────────────┘
```

### 2.2 数据流向

```
配置文件 (src/interface/interface.toml)
    │
    │ 上电时：init_default()
    ▼
全局接口管理器 (OnceLock<Mutex<InterfaceManager>>)
    │
    ├─► Interface 0 (eth0) ──► 提供接口信息
    ├─► Interface 1 (lo)   ──► 查询/修改配置
    └─► Interface N         ──► 状态变更通知

任何模块通过 global_manager() 访问：
    - get_by_name("eth0")
    - get_by_index(0)
    - interfaces()
```

### 2.3 处理模型

1. **上电初始化阶段**：
   - 系统上电时调用 `init_default()`
   - 从默认配置文件 `src/interface/interface.toml` 解析配置
   - 创建 `InterfaceManager`
   - 将管理器存储到全局 `OnceLock` 中

2. **协议栈运行阶段**：
   - 任何模块调用 `global_manager()` 获取全局接口管理器引用
   - 通过接口名称或索引查询接口信息
   - 获取接口的 MAC、IP、MTU 等属性用于报文处理

3. **配置变更阶段**：
   - 通过接口的可变引用修改属性
   - 状态变更时更新内部状态

### 2.4 上电集成流程

```
main() or boot_default()
    │
    ├─► init_default()
    │       │
    │       ├─► 读取 src/interface/interface.toml
    │       ├─► 解析 TOML 格式（包含队列配置）
    │       ├─► 创建 InterfaceManager（使用配置的队列容量）
    │       └─► 存储到全局 OnceLock
    │
    └─► 协议栈处理...
            │
            └─► global_manager()?.get_by_name("eth0")
```

---

## 三、核心数据结构

### 3.1 接口状态枚举

表示网络接口的当前状态。

```rust
/// 网络接口状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InterfaceState {
    /// 接口已启用，可以收发数据
    Up,
    /// 接口已禁用
    Down,
    /// 接口处于测试模式
    Testing,
    /// 接口发生错误
    Error,
}
```

### 3.2 接口类型枚举

```rust
/// 接口类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InterfaceType {
    /// 以太网接口
    Ethernet,
    /// 本地回环接口
    Loopback,
    /// 虚拟接口
    Virtual,
}
```

### 3.3 网络接口结构

```rust
/// 网络接口
#[derive(Debug)]
pub struct NetworkInterface {
    /// 接口名称（如 eth0）
    pub name: String,

    /// 接口索引（系统内唯一标识）
    pub index: u32,

    /// MAC地址
    pub mac_addr: MacAddr,

    /// IPv4地址
    pub ip_addr: Ipv4Addr,

    /// 子网掩码
    pub netmask: Ipv4Addr,

    /// 默认网关
    pub gateway: Option<Ipv4Addr>,

    /// 最大传输单元（字节）
    pub mtu: u16,

    /// 接口状态
    pub state: InterfaceState,

    /// 接口类型
    pub if_type: InterfaceType,

    /// 接收队列
    pub rxq: RingQueue<Packet>,

    /// 发送队列
    pub txq: RingQueue<Packet>,
}
```

### 3.4 接口配置（用于配置文件）

```rust
/// 接口配置（用于配置文件解析）
#[derive(Debug, Clone)]
pub struct InterfaceConfig {
    /// 接口名称
    pub name: String,

    /// MAC地址
    pub mac_addr: MacAddr,

    /// IPv4地址
    pub ip_addr: Ipv4Addr,

    /// 子网掩码
    pub netmask: Ipv4Addr,

    /// 默认网关
    pub gateway: Option<Ipv4Addr>,

    /// MTU
    pub mtu: Option<u16>,

    /// 初始状态
    pub state: Option<InterfaceState>,
}
```

### 3.5 接口模块配置（包含队列配置）

```rust
/// 接口模块配置（包含队列配置和接口列表）
#[derive(Debug, Clone)]
pub struct InterfaceModuleConfig {
    /// 接收队列容量
    pub rxq_capacity: usize,
    /// 发送队列容量
    pub txq_capacity: usize,
    /// 接口配置列表
    pub interfaces: Vec<InterfaceConfig>,
}
```

### 3.6 接口管理器

```rust
/// 接口管理器
#[derive(Debug)]
pub struct InterfaceManager {
    /// 接口列表（按索引排序）
    interfaces: Vec<NetworkInterface>,

    /// 名称到索引的映射
    name_to_index: HashMap<String, u32>,

    /// 接收队列容量（用于创建新接口）
    rxq_capacity: usize,

    /// 发送队列容量（用于创建新接口）
    txq_capacity: usize,
}
```

### 3.7 全局接口管理器

```rust
/// 全局接口管理器
///
/// 使用 OnceLock + Mutex 实现线程安全的单例模式，支持运行时修改接口配置
static GLOBAL_MANAGER: OnceLock<Mutex<InterfaceManager>> = OnceLock::new();
```

---

## 四、接口定义

### 4.1 配置文件常量

```rust
/// 接口模块默认配置文件路径
pub const DEFAULT_CONFIG_PATH: &str = "src/interface/interface.toml";
```

### 4.2 NetworkInterface 接口

NetworkInterface 提供网络接口的配置和状态管理功能。

| 方法签名 | 功能说明 |
|---------|---------|
| `pub fn new(name: String, index: u32, mac_addr: MacAddr, ip_addr: Ipv4Addr, rxq_capacity: usize, txq_capacity: usize) -> Self` | 创建新接口 |
| `pub fn from_config(config: InterfaceConfig, index: u32, rxq_capacity: usize, txq_capacity: usize) -> Self` | 从配置创建接口 |
| `pub fn name(&self) -> &str` | 获取接口名称 |
| `pub fn index(&self) -> u32` | 获取接口索引 |
| `pub fn set_ip_addr(&mut self, addr: Ipv4Addr)` | 设置 IP 地址 |
| `pub fn set_mac_addr(&mut self, addr: MacAddr)` | 设置 MAC 地址 |
| `pub fn set_netmask(&mut self, mask: Ipv4Addr)` | 设置子网掩码 |
| `pub fn set_gateway(&mut self, addr: Option<Ipv4Addr>)` | 设置网关 |
| `pub fn set_mtu(&mut self, mtu: u16)` | 设置 MTU |
| `pub fn up(&mut self)` | 启用接口 |
| `pub fn down(&mut self)` | 禁用接口 |
| `pub fn is_up(&self) -> bool` | 检查接口是否启用 |
| `pub fn network_address(&self) -> Ipv4Addr` | 计算网络地址 |
| `pub fn broadcast_address(&self) -> Ipv4Addr` | 计算广播地址 |

### 4.3 InterfaceManager 接口

InterfaceManager 提供多接口的管理和查询功能。

| 方法签名 | 功能说明 | 返回值 |
|---------|---------|--------|
| `pub fn new(rxq_capacity: usize, txq_capacity: usize) -> Self` | 创建新的接口管理器 | InterfaceManager |
| `pub fn add_interface(&mut self, interface: NetworkInterface)` | 添加接口 | Result<(), InterfaceError> |
| `pub fn add_from_config(&mut self, config: InterfaceConfig)` | 从配置添加接口 | Result<(), InterfaceError> |
| `pub fn get_by_name(&self, name: &str)` | 通过名称获取接口 | Result<&NetworkInterface, InterfaceError> |
| `pub fn get_by_name_mut(&mut self, name: &str)` | 通过名称获取可变接口 | Result<&mut NetworkInterface, InterfaceError> |
| `pub fn get_by_index(&self, index: u32)` | 通过索引获取接口 | Result<&NetworkInterface, InterfaceError> |
| `pub fn get_by_index_mut(&mut self, index: u32)` | 通过索引获取可变接口 | Result<&mut NetworkInterface, InterfaceError> |
| `pub fn interfaces(&self) -> &[NetworkInterface]` | 获取所有接口 | 接口切片 |
| `pub fn interfaces_mut(&mut self) -> &mut [NetworkInterface]` | 获取所有接口的可变引用 | 可变切片 |
| `pub fn len(&self) -> usize` | 获取接口数量 | 数量 |
| `pub fn is_empty(&self) -> bool` | 是否为空 | 布尔值 |

### 4.4 配置文件接口

| 函数签名 | 功能说明 | 返回值 |
|---------|---------|--------|
| `pub fn load_default_config()` | 从默认配置文件加载接口 | Result<InterfaceManager, InterfaceError> |
| `pub fn save_config(manager: &InterfaceManager, path: &str, rxq_capacity: usize, txq_capacity: usize)` | 保存配置到文件 | Result<(), InterfaceError> |

**配置文件格式 (TOML)：**
```toml
# 队列配置
[queue]
rxq_capacity = 256
txq_capacity = 256

# 网络接口配置
[[interfaces]]
name = "eth0"
mac_addr = "00:11:22:33:44:55"
ip_addr = "192.168.1.100"
netmask = "255.255.255.0"
gateway = "192.168.1.1"
mtu = 1500
state = "Up"

[[interfaces]]
name = "lo"
mac_addr = "00:00:00:00:00:00"
ip_addr = "127.0.0.1"
netmask = "255.0.0.0"
state = "Up"
```

### 4.5 全局接口管理器接口

| 函数签名 | 功能说明 | 返回值 |
|---------|---------|--------|
| `pub fn init_global_manager(manager: InterfaceManager)` | 初始化全局接口管理器 | Result<(), InterfaceError> |
| `pub fn init_default()` | 从默认配置文件初始化全局管理器 | Result<(), InterfaceError> |
| `pub fn global_manager() -> Option<&'static Mutex<InterfaceManager>>` | 获取全局接口管理器引用 | Option<&Mutex<InterfaceManager>> |
| `pub fn update_interface<F>(name: &str, f: F)` | 修改指定接口配置（通用） | Result<(), InterfaceError> |
| `pub fn set_interface_ip(name: &str, addr: Ipv4Addr)` | 设置接口 IP 地址 | Result<(), InterfaceError> |
| `pub fn set_interface_mac(name: &str, addr: MacAddr)` | 设置接口 MAC 地址 | Result<(), InterfaceError> |
| `pub fn set_interface_netmask(name: &str, mask: Ipv4Addr)` | 设置接口子网掩码 | Result<(), InterfaceError> |
| `pub fn set_interface_gateway(name: &str, addr: Option<Ipv4Addr>)` | 设置接口网关 | Result<(), InterfaceError> |
| `pub fn set_interface_mtu(name: &str, mtu: u16)` | 设置接口 MTU | Result<(), InterfaceError> |
| `pub fn interface_up(name: &str)` | 启用接口 | Result<(), InterfaceError> |
| `pub fn interface_down(name: &str)` | 禁用接口 | Result<(), InterfaceError> |

**使用示例：**
```rust
// 使用通用修改接口
update_interface("eth0", |iface| {
    iface.set_ip_addr(Ipv4Addr::new(192, 168, 2, 100));
    iface.set_mac_addr(MacAddr::new([0x00, 0x11, 0x22, 0x33, 0x44, 0x56]));
})?;

// 使用专用便捷函数
set_interface_ip("eth0", Ipv4Addr::new(192, 168, 2, 100))?;
interface_up("eth0")?;
```

---

## 五、模块结构

```
src/interface/
├── mod.rs           # 模块入口，导出公共类型
├── types.rs         # InterfaceState, InterfaceType, InterfaceError 定义
├── iface.rs         # NetworkInterface 结构定义（包含队列）
├── manager.rs       # InterfaceManager 实现
├── config.rs        # 接口配置文件加载（包含队列配置解析）
├── global.rs        # 全局接口管理器
└── interface.toml   # 接口配置文件（包含队列配置）
```

### 模块导出

```rust
mod types;
mod iface;
mod manager;
mod config;
mod global;

pub use types::{MacAddr, Ipv4Addr, InterfaceState, InterfaceType, InterfaceError};
pub use iface::{NetworkInterface, InterfaceConfig};
pub use manager::InterfaceManager;
pub use config::{load_default_config, save_config, InterfaceModuleConfig, DEFAULT_CONFIG_PATH};
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

## 六、错误处理

### 6.1 错误类型定义

| 变体 | 说明 | 数据 |
|------|------|------|
| `DuplicateName` | 接口名称重复 | `String` (接口名称) |
| `InterfaceNotFound` | 接口未找到 | - |
| `ConfigReadFailed` | 配置文件读取失败 | `String` (错误信息) |
| `ConfigParseFailed` | 配置文件解析失败 | `String` (错误信息) |
| `ConfigWriteFailed` | 配置文件写入失败 | `String` (错误信息) |
| `InvalidMacAddr` | MAC地址格式无效 | `String` (地址值) |
| `InvalidIpAddr` | IP地址格式无效 | `String` (地址值) |
| `InvalidMtu` | MTU值无效（过小或过大） | `u16` (MTU值) |
| `InvalidFormat` | 配置文件格式错误 | `String` (错误信息) |
| `MutexLockFailed` | 互斥锁锁定失败 | `String` (错误信息) |

**Trait 实现：**
- `Display`: 提供错误信息描述
- `Error`: 实现 Rust 标准错误 trait

### 6.2 错误处理策略

- **解析错误**：配置文件解析失败时返回具体错误信息
- **查找错误**：未找到接口时返回 `InterfaceNotFound`
- **验证错误**：无效的地址格式、MTU值等返回对应错误
- **IO错误**：文件读写失败时包装为配置错误

---

## 七、设计原则

1. **单一职责**：接口模块只负责接口配置和状态管理，不涉及报文收发
2. **零依赖**：仅使用 Rust 标准库，配置文件解析使用简单的 TOML 解析
3. **类型安全**：使用 Rust 类型系统防止无效操作（如无效地址格式）
4. **可扩展性**：设计支持多接口，方便后续添加 IPv6、虚拟接口等
5. **可读性**：代码和文档使用中文，便于学习理解
6. **全局访问**：通过 `OnceLock<Mutex<>>` 提供线程安全的全局访问接口，支持运行时修改接口配置
7. **配置自治**：配置文件路径和队列容量由 interface 模块自己管理

---

## 七、测试策略

### 7.1 单元测试

#### 7.1.1 测试范围

**基础类型测试**：
- **MacAddr**：MAC 地址创建、解析、比较、广播地址判断
- **Ipv4Addr**：IP 地址创建、解析、网络地址计算、广播地址计算
- **InterfaceState**：状态转换和判断
- **InterfaceType**：类型判断

**NetworkInterface 测试**：
- **正常路径**：创建新接口、从配置创建接口、属性设置、状态管理
- **边界条件**：MTU 最小/最大值、空地址处理
- **错误路径**：无效 MAC/IP 格式、状态转换限制
- **状态转换**：Up <-> Down、Testing、Error 状态转换

**InterfaceManager 测试**：
- **正常路径**：添加接口、查询接口、遍历接口
- **边界条件**：空管理器、单接口、多接口
- **错误路径**：重复名称、未找到接口、索引越界

**配置文件测试**：
- **正常路径**：加载正确配置、保存配置
- **错误路径**：文件不存在、格式错误、无效值、重复接口名

**全局管理器测试**：
- **正常路径**：初始化、获取引用、修改接口
- **并发安全**：多线程访问全局管理器
- **错误路径**：未初始化访问、重复初始化

#### 7.1.2 测试组织

测试代码使用 Rust 标准的内嵌测试模式，在各源文件中通过 `#[cfg(test)]` 定义测试模块。

测试按功能类别组织：
- 基础类型测试组：MacAddr、Ipv4Addr、InterfaceState、InterfaceType
- NetworkInterface 测试组：创建、配置、状态管理、地址计算
- InterfaceManager 测试组：添加、查询、遍历接口
- 配置文件测试组：加载、保存、错误处理
- 全局管理器测试组：初始化、访问、修改、并发安全

测试辅助函数：
- `create_test_interface()`: 创建测试用接口
- `create_test_config()`: 创建测试配置
- `create_test_manager()`: 创建测试用管理器
- `setup_global_manager()`: 初始化全局管理器
- `cleanup_global_manager()`: 清理全局状态

#### 7.1.3 测试覆盖要点

| 测试维度 | 覆盖要点 |
|---------|---------|
| **公共接口** | NetworkInterface::new(), from_config(), 所有 setter/getter<br>InterfaceManager::add_interface(), get_by_name(), get_by_index()<br>全局管理器的所有便捷函数 |
| **内部逻辑** | 名称到索引的映射维护<br>队列容量初始化<br>网络地址/广播地址计算算法 |
| **边界条件** | MTU: 0, 576, 1500, 9000<br>接口数量: 0, 1, 最大值<br>队列容量: 最小值, 常用值 |
| **错误处理** | InterfaceError 所有变体<br>配置文件解析所有错误分支 |
| **并发安全** | 全局管理器的 Mutex 访问<br>多线程同时修改接口 |

### 7.2 集成测试

#### 7.2.1 测试场景

**场景一：系统上电初始化流程**
- **涉及模块**：poweron、interface
- **测试内容**：
  - 调用 `boot_default()` 初始化系统
  - 验证全局接口管理器已创建
  - 验证接口配置已从文件加载
  - 验证每个接口的队列已正确初始化
  - 验证接口状态符合配置

**场景二：多接口协同工作**
- **涉及模块**：interface、scheduler
- **测试内容**：
  - 创建多个接口（eth0, eth1, lo）
  - 向不同接口的 RxQ 注入报文
  - 验证每个接口独立处理报文
  - 验证响应报文放入正确接口的 TxQ

**场景三：接口配置运行时修改**
- **涉及模块**：interface（全局管理器）、protocols
- **测试内容**：
  - 通过全局管理器修改接口 IP
  - 验证 ARP 缓存更新
  - 验证后续报文使用新 IP

#### 7.2.2 测试依赖

- **测试数据文件**：`src/interface/interface.toml`（测试配置）
- **模拟模块**：无（使用真实模块）
- **环境准备**：每个测试前清理全局状态

### 7.3 测试数据设计

#### 7.3.1 测试数据来源

- **配置文件**：使用真实的 `interface.toml` 作为基础
- **辅助构造函数**：提供 `create_test_interface()`, `create_test_config()` 等
- **边界值数据**：手工构造的边界 MAC/IP 地址

#### 7.3.2 测试数据管理

提供辅助函数用于创建测试数据：

- `create_test_interface()`: 创建默认测试接口
- `create_test_config()`: 创建测试配置对象
- `create_test_manager()`: 创建预配置的管理器
- `setup_global_manager()`: 初始化全局状态
- `cleanup_global_manager()`: 清理全局状态

### 7.4 测试执行计划

```bash
# 运行 interface 模块所有测试
cargo test interface

# 运行特定文件测试
cargo test --lib interface::tests

# 运行特定测试用例
cargo test test_interface_creation

# 显示测试输出
cargo test -- --nocapture

# 运行文档测试
cargo test --doc
```

---

## 八、实现路线图

| 阶段 | 内容 | 状态 |
|------|------|------|
| Phase 1 | 基础类型（MacAddr, Ipv4Addr） | ✅ 已完成 |
| Phase 2 | NetworkInterface 结构和方法 | ✅ 已完成 |
| Phase 3 | InterfaceManager 实现 | ✅ 已完成 |
| Phase 4 | 配置文件加载（TOML 解析） | ✅ 已完成 |
| Phase 5 | 全局接口管理器 | ✅ 已完成 |
| Phase 6 | 与 poweron 模块集成 | ✅ 已完成 |
| Phase 7 | 队列配置迁移到 interface 模块 | ✅ 已完成 |
| Phase 8 | 简化 API（移除路径参数） | ✅ 已完成 |
| Phase 9 | 与协议层集成测试 | 待规划 |
