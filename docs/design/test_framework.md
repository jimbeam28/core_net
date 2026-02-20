# 报文测试框架设计

## 概述

报文测试框架（Test Harness）是 CoreNet 协议栈的通用测试基础设施，提供**报文注入**和**调度执行**的核心测试能力。

**设计目标**：
- 提供协议无关的通用测试框架
- 支持单接口和多接口测试场景
- **解决全局状态管理问题，确保测试隔离**
- **避免 Mutex 毒化和死锁问题**

**核心职责**：
1. 报文注入：将报文注入到接口的接收队列
2. 调度执行：运行调度器处理报文
3. 全局状态管理：提供测试隔离和 Mutex 毒化恢复

**不属于本模块**：
- ❌ 报文构造：由协议模块或测试用例实现
- ❌ 结果验证：由测试用例直接断言

---

## 一、架构设计

### 1.1 模块定位

```
测试用例层 → 测试框架层 → 协议栈层
   ↓              ↓              ↓
 创建报文     注入+调度      处理报文
 验证结果    GlobalStateManager   Engine/Interface
```

### 1.2 数据流向

```
测试用例创建报文 → 注入RxQ → 调度执行 → 测试用例验证TxQ/资源
                          │
                  三步处理模式（避免死锁）
                  1. 收集报文（持锁）
                  2. 处理报文（无锁）★核心★
                  3. 放回响应（持锁）
```

### 1.3 模块结构

```
src/testframework/
├── mod.rs                # 模块入口
├── harness.rs            # TestHarness 主实现
├── injector.rs           # PacketInjector
├── global_state.rs       # 全局状态管理器
└── error.rs              # HarnessError
```

---

## 二、核心数据结构

### 2.1 TestHarness

```rust
pub struct TestHarness {
    interfaces: InterfaceManager,
    scheduler: Scheduler,
    processor: PacketProcessor,
    verbose: bool,
    use_global: bool,
}
```

**职责**：
- 管理接口和调度器
- 提供报文注入器
- 运行调度器处理报文

### 2.2 PacketInjector

```rust
pub struct PacketInjector<'a> {
    interfaces: &'a mut InterfaceManager,
}
```

**职责**：
- 将报文注入到指定接口的接收队列

### 2.3 GlobalStateManager

```rust
pub struct GlobalStateManager;

impl GlobalStateManager {
    pub fn clear_global_state() -> HarnessResult<()>;
    pub fn setup_global_state() -> HarnessResult<()>;
    pub fn get_or_recover_arp_lock() -> MutexGuard<'static, ArpCache>;
    pub fn get_or_recover_interface_lock() -> MutexGuard<'static, InterfaceManager>;
    pub fn reset_interface_configs(configs: Vec<InterfaceTestConfig>) -> HarnessResult<()>;
}
```

**职责**：
- 全局状态初始化和清理
- Mutex 毒化自动恢复
- 测试隔离保证

---

## 三、接口定义

### 3.1 TestHarness

| 方法 | 说明 |
|------|------|
| `new(rxq_capacity, txq_capacity)` | 创建独立测试框架 |
| `with_global_manager()` | 创建使用全局管理器的测试框架 |
| `with_verbose(bool)` | 启用详细输出 |
| `injector()` | 获取报文注入器 |
| `run()` | 运行调度器处理报文 |
| `interfaces()` | 获取接口管理器引用 |
| `interfaces_mut()` | 获取接口管理器可变引用 |

### 3.2 PacketInjector

| 方法 | 说明 |
|------|------|
| `inject(interface_name, packet)` | 注入单个报文 |
| `inject_multiple(interface_name, packets)` | 注入多个报文 |

### 3.3 GlobalStateManager

| 方法 | 说明 |
|------|------|
| `clear_global_state()` | 清空全局状态，自动恢复毒化 Mutex |
| `setup_global_state()` | 初始化或重置全局状态 |
| `get_or_recover_arp_lock()` | 安全获取 ARP 缓存锁 |
| `get_or_recover_interface_lock()` | 安全获取接口管理器锁 |
| `reset_interface_configs()` | 重置接口配置 |

---

## 四、错误处理

```rust
pub enum HarnessError {
    InterfaceError(String),
    QueueError(String),
    SchedulerError(String),
    GlobalStateError(String),
    MutexPoisonedError(String),
}
```

**Mutex 毒化恢复策略**：循环检测 `lock()` 结果，毒化时使用 `into_inner()` 恢复。

---

## 五、测试隔离模板

### 5.1 使用全局状态管理器的测试

```rust
#[test]
fn test_example() {
    // 清空和初始化全局状态
    GlobalStateManager::clear_global_state().unwrap();
    GlobalStateManager::setup_global_state().unwrap();

    // 创建测试报文（由测试用例或协议模块实现）
    let packet = create_test_packet();

    // 注入报文
    GlobalStateManager::inject_to_global_interface("eth0", packet).unwrap();

    // 运行调度器
    let mut harness = TestHarness::with_global_manager();
    let count = harness.run().unwrap();

    // 验证结果（由测试用例直接断言）
    let guard = GlobalStateManager::get_or_recover_interface_lock();
    let iface = guard.get_by_name("eth0").unwrap();
    assert_eq!(iface.txq.len(), 1);
}
```

### 5.2 使用独立接口管理器的测试

```rust
#[test]
fn test_example() {
    // 创建测试框架
    let mut harness = TestHarness::new(16, 16);

    // 添加接口
    let iface = NetworkInterface::new("eth0", ...);
    harness.add_interface(iface).unwrap();

    // 创建测试报文
    let packet = create_test_packet();

    // 注入报文
    harness.injector().inject("eth0", packet).unwrap();

    // 运行调度器
    let count = harness.run().unwrap();

    // 验证结果
    assert_eq!(count, 1);
    let iface = harness.interfaces().get_by_name("eth0").unwrap();
    assert_eq!(iface.txq.len(), 1);
}
```

---

## 六、设计原则

1. **单一职责**：只负责报文注入和调度执行
2. **协议无关**：不涉及具体协议的报文构造和验证
3. **测试隔离**：提供全局状态清理和初始化
4. **Mutex 安全**：自动恢复毒化 Mutex，避免死锁
5. **简洁易用**：最小化 API，减少学习成本

---

## 七、依赖注入模式下的测试

### 7.1 简化的测试流程

CoreNet 使用依赖注入模式后，测试不再需要复杂的全局状态管理：

```rust
#[test]
fn test_with_context() {
    // 直接创建测试上下文（包含所有必要组件）
    let ctx = SystemContext::new();

    // 添加测试接口
    ctx.interfaces.lock().unwrap()
        .add_from_config(test_config()).unwrap();

    // 创建测试报文
    let packet = create_test_packet();

    // 注入报文
    ctx.interfaces.lock().unwrap()
        .get_by_name_mut("eth0").unwrap()
        .rxq.enqueue(packet).unwrap();

    // 运行调度器（传递上下文引用）
    let mut scheduler = Scheduler::with_context(&ctx);
    let count = scheduler.run_all_interfaces().unwrap();

    // 验证结果
    assert_eq!(count, 1);
}
```

### 7.2 与全局状态模式的对比

| 特性 | 全局状态模式 | 依赖注入模式 |
|------|-------------|-------------|
| 初始化 | `GlobalStateManager::setup_global_state()` | `SystemContext::new()` |
| 资源访问 | `GlobalStateManager::get_or_recover_xxx_lock()` | `ctx.xxx.lock().unwrap()` |
| 测试隔离 | 需要手动清理全局状态 | 每个测试独立创建上下文 |
| Mutex 毒化 | 需要特殊处理 | 由 Arc<Mutex<T>> 自动处理 |
| 并发测试 | 需要 `#[serial]` 序列化 | 天然隔离，可并行 |

### 7.3 TestHarness 更新

新的 TestHarness 支持接收 SystemContext：

```rust
pub struct TestHarness {
    /// 使用依赖注入模式
    context: SystemContext,
    scheduler: Scheduler,
    processor: PacketProcessor,
}

impl TestHarness {
    /// 从 SystemContext 创建测试框架
    pub fn with_context(context: SystemContext) -> Self {
        // ...
    }

    /// 便捷方法：创建空上下文的测试框架
    pub fn new() -> Self {
        Self::with_context(SystemContext::new())
    }
}
```

### 7.4 推荐的测试模式

```rust
#[test]
fn test_protocol_with_di() {
    // 1. 创建上下文
    let ctx = SystemContext::new();

    // 2. 配置测试环境
    setup_test_interfaces(&ctx);
    setup_test_arp_cache(&ctx);

    // 3. 创建测试框架
    let mut harness = TestHarness::with_context(ctx.clone());

    // 4. 注入报文并运行
    harness.injector().inject("eth0", test_packet()).unwrap();
    harness.run().unwrap();

    // 5. 验证结果（使用原始上下文引用）
    let iface = ctx.interfaces.lock().unwrap()
        .get_by_name("eth0").unwrap();
    assert_eq!(iface.txq.len(), 1);
}
```

