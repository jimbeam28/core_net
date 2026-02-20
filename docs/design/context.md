# SystemContext 设计

## 概述

SystemContext 是 CoreNet 的系统上下文结构，使用依赖注入模式替代全局状态架构。它持有所有需要共享的资源（接口管理器、ARP 缓存、ICMP Echo 管理器）的所有权，通过 `Arc<Mutex<T>>` 封装以支持多线程访问和并发控制。

**核心职责**：
- 统一管理系统资源
- 通过依赖注入传递给各个模块
- 支持测试环境的灵活配置

---

## 一、需求介绍

### 1.1 功能需求

- **需求1**：持有接口管理器的所有权，提供对网络接口的访问
- **需求2**：持有 ARP 缓存，供协议模块共享
- **需求3**：持有 ICMP Echo 管理器，追踪待处理的 Echo 请求
- **需求4**：支持 Clone，多个模块可以共享同一底层状态
- **需求5**：支持测试环境的灵活配置（空上下文、自定义组件）

### 1.2 约束条件

- **零外部依赖**：仅使用 Rust 标准库（std::sync::Arc、std::sync::Mutex）
- **线程安全**：所有资源通过 Arc<Mutex<T>> 封装，支持多线程访问
- **可测试性**：提供多种构造方式，便于测试环境使用

---

## 二、架构设计

### 2.1 模块定位

SystemContext 位于系统的入口层，作为依赖注入的容器：

```
┌─────────────────────────────────────────────────────────────────┐
│                        应用/测试层                               │
│  ┌─────────────────────────────────────────────────────────────┐ │
│  │                      SystemContext                          │ │
│  │  ┌───────────────────────────────────────────────────────┐ │ │
│  │  │  Arc<Mutex<InterfaceManager>>  (网络接口)              │ │ │
│  │  │  Arc<Mutex<ArpCache>>         (ARP 缓存)              │ │ │
│  │  │  Arc<Mutex<EchoManager>>      (ICMP Echo)             │ │ │
│  │  └───────────────────────────────────────────────────────┘ │ │
│  └─────────────────────────────────────────────────────────────┘ │
│                              │                                  │
│                              ▼                                  │
│  ┌─────────────────────────────────────────────────────────────┐ │
│  │                    协议处理层                                │ │
│  │  ┌─────────┐  ┌─────────┐  ┌─────────┐  ┌─────────┐      │ │
│  │  │ Engine  │  │  ARP    │  │  ICMP   │  │  IP     │      │ │
│  │  └─────────┘  └─────────┘  └─────────┘  └─────────┘      │ │
│  └─────────────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────────┘
```

### 2.2 数据流向

```
┌──────────────┐      传递引用      ┌──────────────────────────────────┐
│  创建上下文   │ ─────────────────> │         SystemContext            │
│              │                    │  ┌────────────────────────────┐  │
│ new()        │                    │  │ Arc<Mutex<InterfaceManager>>│  │
│ from_config()│                    │  │ Arc<Mutex<ArpCache>>        │  │
│ with_components()                  │  │ Arc<Mutex<EchoManager>>     │  │
└──────────────┘                    │  └────────────────────────────┘  │
                                     └──────────────────────────────────┘
                                                  │
                                                  │ 通过参数传递
                                                  ▼
                                     ┌──────────────────────────────────┐
                                     │         各协议模块               │
                                     │  lock() -> 获取可变引用 -> 操作  │
                                     └──────────────────────────────────┘
```

---

## 三、核心数据结构

### 3.1 SystemContext

```rust
/// 系统上下文，持有所有全局状态的所有权
///
/// 使用依赖注入模式替代全局状态，便于测试和并发控制。
/// 所有字段都使用 Arc<Mutex<T>> 封装以支持多线程访问。
#[derive(Clone)]
pub struct SystemContext {
    /// 接口管理器
    pub interfaces: Arc<Mutex<InterfaceManager>>,

    /// ARP 缓存
    pub arp_cache: Arc<Mutex<ArpCache>>,

    /// ICMP Echo 管理器
    pub icmp_echo: Arc<Mutex<EchoManager>>,
}
```

**字段说明**：

| 字段 | 类型 | 说明 |
|------|------|------|
| `interfaces` | `Arc<Mutex<InterfaceManager>>` | 网络接口管理器，包含所有接口的配置和队列 |
| `arp_cache` | `Arc<Mutex<ArpCache>>` | ARP 缓存表，存储 IP 到 MAC 的映射 |
| `icmp_echo` | `Arc<Mutex<EchoManager>>` | ICMP Echo 请求管理器，追踪待处理的请求 |

**Arc<Mutex<T>> 的作用**：
- `Arc`：允许多个所有者共享同一数据（引用计数）
- `Mutex`：保证线程安全的互斥访问
- 组合使用：多个线程可以安全地访问和修改共享状态

---

## 四、接口定义

### 4.1 构造方法

#### 4.1.1 new() - 创建空上下文

```rust
impl SystemContext {
    /// 创建新的系统上下文（用于测试）
    ///
    /// 创建一个空的系统上下文，所有组件使用默认值。
    ///
    /// # 返回
    ///
    /// 返回包含默认组件的 SystemContext
    ///
    /// # 使用场景
    ///
    /// 适用于单元测试，需要完全控制所有组件的状态
    pub fn new() -> Self;
}
```

**使用示例**：
```rust
// 测试环境
let ctx = SystemContext::new();
assert_eq!(ctx.interface_count(), 0);
```

#### 4.1.2 from_config() - 从配置加载

```rust
impl SystemContext {
    /// 从配置文件创建系统上下文（生产环境使用）
    ///
    /// 加载默认配置文件初始化接口管理器，其他组件使用默认值。
    ///
    /// # 返回
    ///
    /// 返回初始化完成的 SystemContext
    ///
    /// # 使用场景
    ///
    /// 适用于生产环境，从配置文件加载接口配置
    pub fn from_config() -> Self;
}
```

**使用示例**：
```rust
// 生产环境
let ctx = SystemContext::from_config();
// 已加载 src/interface/interface.toml 中的接口配置
```

#### 4.1.3 with_components() - 自定义组件

```rust
impl SystemContext {
    /// 使用指定组件创建系统上下文（高级用法）
    ///
    /// 允许完全自定义所有组件，用于需要精细控制的场景。
    ///
    /// # 参数
    ///
    /// - `interfaces`: 接口管理器
    /// - `arp_cache`: ARP 缓存
    /// - `icmp_echo`: ICMP Echo 管理器
    ///
    /// # 使用场景
    ///
    /// 适用于需要完全自定义组件的测试场景
    pub fn with_components(
        interfaces: Arc<Mutex<InterfaceManager>>,
        arp_cache: Arc<Mutex<ArpCache>>,
        icmp_echo: Arc<Mutex<EchoManager>>,
    ) -> Self;
}
```

**使用示例**：
```rust
// 高级用法：自定义组件
let interfaces = Arc::new(Mutex::new(custom_manager));
let arp_cache = Arc::new(Mutex::new(custom_cache));
let icmp_echo = Arc::new(Mutex::new(custom_echo));
let ctx = SystemContext::with_components(interfaces, arp_cache, icmp_echo);
```

### 4.2 查询方法

```rust
impl SystemContext {
    /// 获取接口数量
    pub fn interface_count(&self) -> usize;

    /// 检查上下文是否为空（无接口）
    pub fn is_empty(&self) -> bool;
}
```

### 4.3 Trait 实现

```rust
impl Default for SystemContext {
    fn default() -> Self {
        Self::new()
    }
}
```

---

## 五、模块结构

```
src/
├── context.rs      # SystemContext 实现
├── lib.rs          # 导出 Context 别名
└── main.rs         # 程序入口
```

### 模块导出

```rust
// src/lib.rs
pub use context::SystemContext as Context;
```

**导出说明**：
- `SystemContext` 作为主要类型导出
- `Context` 作为别名导出，提供更简洁的命名

---

## 六、错误处理

SystemContext 本身不定义新的错误类型，依赖子模块的错误类型：

| 错误类型 | 来源 | 说明 |
|---------|------|------|
| `InterfaceError` | `interface` 模块 | 接口操作错误 |
| `ArpError` | `protocols/arp` 模块 | ARP 操作错误 |
| `IcmpError` | `protocols/icmp` 模块 | ICMP 操作错误 |

---

## 七、使用场景

### 7.1 生产环境使用

```rust
use core_net::Context;

fn main() {
    // 从配置文件创建上下文
    let ctx = Context::from_config();

    // 传递给调度器
    let mut scheduler = Scheduler::new(&ctx);

    // 运行
    scheduler.run_all_interfaces();
}
```

### 7.2 测试环境使用

```rust
use core_net::Context;

#[test]
fn test_protocol_processing() {
    // 创建空上下文
    let ctx = Context::new();

    // 添加测试接口
    ctx.interfaces.lock().unwrap()
        .add_from_config(test_config()).unwrap();

    // 创建测试报文
    let packet = create_test_packet();

    // 传递给处理器
    let result = process_packet(&ctx, packet);

    // 验证结果
    assert!(result.is_ok());
}
```

### 7.3 Clone 使用

```rust
// 多个模块可以共享同一上下文
let ctx1 = Context::from_config();
let ctx2 = ctx1.clone();  // 共享底层 Arc，不复制数据

// ctx1 和 ctx2 指向相同的底层资源
assert!(Arc::ptr_eq(&ctx1.interfaces, &ctx2.interfaces));
```

---

## 八、设计原则

1. **依赖注入**：通过参数传递而非全局访问，提高可测试性
2. **线程安全**：使用 Arc<Mutex<T>> 保证并发安全
3. **简洁 API**：提供三种构造方式，覆盖不同使用场景
4. **零依赖**：仅使用 Rust 标准库
5. **Clone 友好**：通过 Arc 实现低成本克隆

---

## 九、测试策略

### 9.1 单元测试范围

| 测试维度 | 覆盖要点 |
|---------|---------|
| 构造方法 | new()、from_config()、with_components() |
| Clone 行为 | 验证克隆后共享相同底层 Arc |
| 查询方法 | interface_count()、is_empty() |
| Default 实现 | 验证 default() 等价于 new() |

### 9.2 集成测试场景

**场景一：与调度器集成**
- 创建 SystemContext
- 传递给 Scheduler
- 验证调度器可以正常访问接口

**场景二：多线程访问**
- 创建 SystemContext
- 多个线程同时访问 interfaces
- 验证 Mutex 正确工作

**场景三：Clone 后的一致性**
- 克隆 SystemContext
- 通过克隆修改状态
- 验证原对象能看到修改

---

## 十、实现路线图

| 阶段 | 内容 | 状态 |
|------|------|------|
| Phase 1 | 基础结构定义 | ✅ 已完成 |
| Phase 2 | 构造方法实现 | ✅ 已完成 |
| Phase 3 | Clone 和 Default 实现 | ✅ 已完成 |
| Phase 4 | 单元测试 | ✅ 已完成 |
| Phase 5 | 集成测试 | ✅ 已完成 |

---

## 十一、参考资料

- [架构设计](architecture.md) - 整体架构说明
- [网络接口模块](interface.md) - InterfaceManager 设计
- [ARP 协议设计](protocols/arp.md) - ArpCache 设计
- [ICMP 协议设计](protocols/icmp.md) - EchoManager 设计
