# 调度模块设计

## 概述

调度模块（Scheduler）是 CoreNet 协议栈的调度中心，负责从接收队列（RxQ）中持续取出报文，并调用协议处理引擎（Engine）进行逐层解析处理。当队列为空时，调度循环自然终止。

**当前阶段目标**：实现基础调度功能和多接口调度功能，连接接收队列与协议处理引擎，完成数据流的闭环。

**调度模式**：
- **单队列模式**：直接处理指定的单个 RingQueue<Packet>
- **多接口模式**：遍历所有网络接口，逐个处理每个接口的接收队列

---

## 一、需求介绍

### 1.1 功能需求

**单队列调度需求**：
- **FR1**：提供调度接口，持续从接收队列（RxQ）中取出报文
- **FR2**：调用 Engine 模块的处理接口对报文进行协议解析
- **FR3**：当接收队列为空时，终止调度循环
- **FR4**：统计并返回已处理的报文数量

**多接口调度需求**：
- **FR5**：提供多接口调度接口，遍历所有网络接口
- **FR6**：对每个接口的接收队列逐个进行报文处理
- **FR7**：支持显示每个接口的处理进度（verbose 模式）
- **FR8**：统计并返回所有接口处理的总报文数量

### 1.2 非功能需求

- **简洁性**：调度逻辑简单清晰，易于理解和维护
- **非阻塞**：队列为空时立即返回，不进行等待或重试
- **错误宽容**：单个报文处理失败不影响后续报文处理

---

## 二、架构设计

### 2.1 模块定位

调度模块位于测试注入器和协议处理引擎之间，是数据流的核心枢纽。支持单队列和多接口两种调度模式。

#### 单队列模式

```
┌──────────────┐         ┌──────────────┐         ┌──────────────┐
│  测试/模拟    │  ───>  │  接收队列    │  ───>  │  调度模块    │
│  注入器      │  RxQ   │  (RingQueue) │         │  (Scheduler) │
└──────────────┘         └──────────────┘         └──────┬───────┘
                                                          │
                                                          v
                                                  ┌──────────────┐
                                                  │  协议处理    │
                                                  │  (Engine)    │
                                                  └──────────────┘
```

#### 多接口模式

```
┌──────────────┐         ┌──────────────────────────────────┐
│  测试/模拟    │  ───>  │        接口管理器                 │
│  注入器      │         │     (InterfaceManager)           │
└──────────────┘         │  ┌────────┐  ┌────────┐         │
                         │  │ eth0   │  │  lo    │  ...   │
                         │  │ RxQ/TxQ│  │ RxQ/TxQ│         │
                         │  └───┬────┘  └───┬────┘         │
                         └──────┼──────────┼──────────────┘
                                │          │
                                v          v
                         ┌──────────────────────────────────┐
                         │       调度模块                    │
                         │      (Scheduler)                 │
                         │  run_all_interfaces()            │
                         └───────────────┬──────────────────┘
                                         │
                                         v
                                 ┌──────────────┐
                                 │  协议处理    │
                                 │  (Engine)    │
                                 └──────────────┘
```

### 2.2 数据流向

#### 单队列模式数据流向

```
测试注入报文
      │
      v
┌───────────┐
│   RxQ     │ <--- 生产者（测试注入器）
│  [P1,P2]  │
└─────┬─────┘
      │ dequeue()
      v
┌───────────┐
│ Scheduler │ ---> 从队列取出 Packet
└─────┬─────┘
      │ process(packet)
      v
┌───────────┐
│  Engine   │ ---> 协议解析处理
└───────────┘
```

#### 多接口模式数据流向

```
测试注入报文
      │
      v
┌──────────────────────────────────┐
│       InterfaceManager           │
│  ┌─────────┐    ┌─────────┐     │
│  │ eth0    │    │  lo     │ ... │
│  │ RxQ     │    │ RxQ     │     │
│  │ [P1,P2] │    │ [P3]    │     │
│  └────┬────┘    └────┬────┘     │
└───────┼──────────────┼──────────┘
        │              │
        │ dequeue()    │ dequeue()
        v              v
┌──────────────────────────────────┐
│       Scheduler                  │
│  run_all_interfaces()            │
│  │                              │
│  ├─> eth0: process(P1, P2)      │
│  └─> lo:   process(P3)          │
└───────────────┬──────────────────┘
                │
                v
        ┌───────────┐
        │  Engine   │ ---> 协议解析处理
        └───────────┘
```

### 2.3 处理模型

#### 单队列处理模型

```
┌─────────────────────────────────────────────────────────┐
│                   Scheduler                           │
│  ┌─────────────────────────────────────────────────┐  │
│  │            run(rxq, processor)                  │  │
│  │                                                 │  │
│  │         循环:                                     │  │
│  │           1. 从 RxQ 尝试取出报文                    │  │
│  │           2. 若队列为空 -> 退出循环                  │  │
│  │           3. 若取出报文 -> 调用 Engine::process()  │  │
│  │           4. 重复                                  │  │
│  └─────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────┘
```

#### 多接口处理模型

```
┌─────────────────────────────────────────────────────────┐
│                   Scheduler                           │
│  ┌─────────────────────────────────────────────────┐  │
│  │   run_all_interfaces(interfaces, processor)     │  │
│  │                                                 │  │
│  │   遍历所有接口:                                   │  │
│  │     for each interface in interfaces:           │  │
│  │       1. 从该接口的 RxQ 尝试取出报文               │  │
│  │       2. 若队列为空 -> 处理下一个接口              │  │
│  │       3. 若取出报文 -> 调用 Engine::process()    │  │
│  │       4. 重复直到该接口队列为空                   │  │
│  │       5. 统计该接口处理数量                        │  │
│  │     返回所有接口处理总数                          │  │
│  └─────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────┘
```

---

## 三、核心数据结构

### 3.1 Scheduler

调度器，负责从接收队列中取出报文并调度给协议处理引擎。

```rust
use crate::common::queue::RingQueue;
use crate::common::Packet;
use crate::engine::PacketProcessor;

/// 调度器
///
/// 负责从接收队列持续取出报文并调度给协议处理引擎。
pub struct Scheduler {
    /// 调度器名称
    name: String,

    /// 协议处理器
    processor: PacketProcessor,

    /// 是否启用详细输出
    verbose: bool,
}
```

### 3.2 ScheduleError

调度错误类型。

```rust
/// 调度错误
#[derive(Debug)]
pub enum ScheduleError {
    /// 队列操作失败
    QueueError(String),

    /// 处理器错误
    ProcessorError(String),

    /// 其他错误
    Other(String),
}

impl std::fmt::Display for ScheduleError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ScheduleError::QueueError(msg) => write!(f, "队列错误: {}", msg),
            ScheduleError::ProcessorError(msg) => write!(f, "处理器错误: {}", msg),
            ScheduleError::Other(msg) => write!(f, "其他错误: {}", msg),
        }
    }
}

impl std::error::Error for ScheduleError {}

/// 调度结果类型
pub type ScheduleResult<T> = Result<T, ScheduleError>;
```

---

## 四、接口定义

### 4.1 Scheduler 核心 API

接口用途说明：创建调度器并运行调度循环。

```rust
use crate::interface::InterfaceManager;

impl Scheduler {
    /// 创建新的调度器
    ///
    /// # 参数
    /// - name: 调度器名称
    ///
    /// # 返回
    /// 新的 Scheduler 实例
    pub fn new(name: String) -> Self;

    /// 设置协议处理器
    ///
    /// # 参数
    /// - processor: 协议处理器实例
    pub fn with_processor(mut self, processor: PacketProcessor) -> Self;

    /// 启用详细输出
    ///
    /// # 参数
    /// - verbose: 是否启用详细输出
    pub fn with_verbose(mut self, verbose: bool) -> Self;

    /// 运行调度循环（单队列模式）
    ///
    /// 从接收队列中持续取出报文进行处理，直到队列为空。
    ///
    /// # 参数
    /// - rxq: 接收队列的可变引用
    ///
    /// # 行为
    /// 1. 循环从 rxq 中尝试出队
    /// 2. 若队列为空（QueueError::Empty），退出循环
    /// 3. 若成功取出报文，调用 processor.process() 处理
    /// 4. 处理结果仅记录，不中断调度
    ///
    /// # 返回
    /// - Ok(count): 成功处理的报文数量
    /// - Err(ScheduleError): 调度过程中发生严重错误
    pub fn run(&self, rxq: &mut RingQueue<Packet>) -> Result<usize, ScheduleError>;

    /// 运行调度循环（多接口模式）
    ///
    /// 从所有接口的接收队列中取出报文进行处理，直到所有队列为空。
    ///
    /// # 参数
    /// - interfaces: 接口管理器的可变引用
    ///
    /// # 行为
    /// 1. 遍历所有接口
    /// 2. 对每个接口的接收队列循环出队
    /// 3. 若队列为空，继续处理下一个接口
    /// 4. 若成功取出报文，调用 processor.process() 处理
    /// 5. 处理结果仅记录，不中断调度
    ///
    /// # 返回
    /// - Ok(count): 成功处理的报文总数
    /// - Err(ScheduleError): 调度过程中发生严重错误
    pub fn run_all_interfaces(&self, interfaces: &mut InterfaceManager) -> Result<usize, ScheduleError>;
}
```

### 4.2 便捷函数

```rust
/// 使用默认调度器处理接收队列
///
/// # 参数
/// - rxq: 接收队列的可变引用
///
/// # 返回
/// - Ok(count): 成功处理的报文数量
/// - Err(ScheduleError): 调度失败
pub fn schedule_packets(rxq: &mut RingQueue<Packet>) -> Result<usize, ScheduleError>;

/// 使用详细输出模式调度
///
/// # 参数
/// - rxq: 接收队列的可变引用
///
/// # 返回
/// - Ok(count): 成功处理的报文数量
/// - Err(ScheduleError): 调度失败
pub fn schedule_packets_verbose(rxq: &mut RingQueue<Packet>) -> Result<usize, ScheduleError>;
```

---

## 五、模块结构

```
src/schedule/
├── mod.rs           # 模块入口，导出公共接口
├── scheduler.rs     # Scheduler 核心实现
└── error.rs        # ScheduleError 定义（可选，可在 scheduler.rs 中定义）
```

### 模块导出

```rust
mod scheduler;

pub use scheduler::{
    Scheduler,
    ScheduleError,
    ScheduleResult,
    schedule_packets,
    schedule_packets_verbose,
};
```

---

## 六、错误处理

### 6.1 错误类型定义

已在章节 3.2 中定义 `ScheduleError` 枚举。

### 6.2 错误处理策略

- **队列空错误**：作为正常退出条件，不返回错误
- **队列其他错误**：包装为 `ScheduleError::QueueError` 向上传播
- **处理器错误**：记录日志但继续处理后续报文，不中断调度

---

## 七、测试策略

### 7.1 单元测试

- 创建调度器测试
- 空队列调度测试
- 单报文调度测试
- 多报文调度测试

### 7.2 集成测试

- 注入多个报文 -> 验证全部处理完成
- 处理器返回错误 -> 验证调度继续处理后续报文

---

## 八、实现路线图

| 阶段 | 内容 | 状态 |
|------|------|------|
| Phase 1 | 基础调度器结构 + run() 方法 | 待实现 |
| Phase 2 | 错误处理完善 + 单元测试 | 待规划 |
| Phase 3 | 统计功能 | 待规划 |
| Phase 4 | 限流控制 | 待规划 |

---

## 九、设计原则

1. **职责单一**：调度器只负责队列到处理器的连接，不涉及协议处理逻辑
2. **非阻塞**：队列为空时立即返回，不进行等待
3. **错误宽容**：单个报文处理失败不影响后续报文处理
4. **简洁优先**：核心接口精简，便于理解和使用
