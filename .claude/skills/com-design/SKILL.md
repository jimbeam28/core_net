---
name: com-design
description: Generate or update detailed design documents for Rust modules. Use when user asks to create design docs, writes technical specifications, or needs architecture documentation for implementation.
allowed-tools: Read, Write, Edit, AskUserQuestion, Glob, Grep
---

# 方案设计

根据需求描述生成或完善详细设计文档。

## 执行流程

### 第一步：确认文档路径

如果未指定文档路径，使用 AskUserQuestion 询问用户。

### 第二步：收集需求信息

根据用户的需求描述，判断信息是否充分。如果信息不足，通过追问明确以下方面：

**必需信息**：
- 模块/功能名称
- 功能目标和核心职责
- 主要使用场景

**可选追问**（视情况）：
- 与现有哪些模块有交互关系？
- 有哪些关键操作或处理流程？
- 有什么性能或可靠性要求？
- 是否有特殊约束（如零依赖、线程安全、no_std 等）？
- 错误处理策略是什么？
- 是否需要配置或状态管理？

### 第三步：分析现有设计文档（如果存在）

如果文档已存在，先读取现有内容，了解：
- 当前已设计的部分
- 需要新增或修改的部分
- 与现有架构的一致性

### 第四步：分析依赖模块的设计文档

根据需求描述，识别涉及的已有模块。如果存在依赖模块：

1. 使用 Glob 工具列出 `docs/design/*.md` 中的所有设计文档
2. 根据模块名称匹配并读取相关文档，了解：
   - 依赖模块的接口定义
   - 依赖模块的数据结构
   - 依赖模块的错误类型
   - 与当前模块的交互约定

**常见依赖模块映射**：
| 模块名 | 设计文档路径 | 主要内容 |
|--------|------------|---------|
| queue | docs/design/queue.md | RingQueue 接口、QueueError |
| packet | docs/design/packet.md | Packet 结构、操作方法 |
| engine | docs/design/engine.md | PacketProcessor、process() 接口 |
| poweron | docs/design/poweron.md | PowerOnContext、资源初始化 |

### 第五步：生成设计文档

按照以下章节结构生成或更新文档：

```markdown
# [模块名称]设计

## 概述

简要描述模块的定位、职责和目标。说明当前阶段目标。

## 一、需求介绍

### 1.1 功能需求

- 需求1：描述
- 需求2：描述
- ...

### 1.2 非功能需求

- 性能要求（如适用）
- 可靠性要求（如适用）
- 约束条件（零依赖、纯内存模拟等）

## 二、架构设计

### 2.1 模块定位

描述模块在整个系统中的位置，用 ASCII 图展示与其他模块的关系。

### 2.2 数据流向

描述数据如何流经此模块，使用箭头图展示。

### 2.3 处理模型

描述核心处理逻辑的结构和流程。

## 三、核心数据结构

### 3.1 [结构体名称1]

结构体用途说明。

```rust
/// 结构体文档注释
pub struct StructName {
    /// 字段说明
    field_name: Type,
}
```

### 3.2 [结构体名称2]

...

## 四、接口定义

### 4.1 [ Trait/Impl 名称]

接口用途说明。

```rust
impl StructName {
    /// 函数文档注释
    ///
    /// # 参数
    /// - param1: 参数说明
    ///
    /// # 返回
    /// - Ok(成功值): 成功时返回
    /// - Err(ErrorType): 错误情况
    pub fn function_name(&self, param1: Type) -> Result<SuccessType, ErrorType>;
}
```

### 4.2 [其他接口]

...

## 五、模块结构

```
src/[module_name]/
├── mod.rs           # 模块入口
├── file1.rs         # 文件说明
└── file2.rs         # 文件说明
```

### 模块导出

```rust
mod file1;
mod file2;

pub use file1::{PublicType1, PublicType2};
pub use file2::{PublicType3};
```

## 六、错误处理

### 6.1 错误类型定义

```rust
/// 错误类型说明
pub enum ErrorName {
    /// 错误变体说明
    ErrorVariant(String),
}
```

### 6.2 错误处理策略

描述错误的传播和处理策略。

## 七、测试策略

### 7.1 单元测试

列出需要测试的关键功能点。

### 7.2 集成测试（如适用）

描述集成测试场景。

## 八、实现路线图

| 阶段 | 内容 | 状态 |
|------|------|------|
| Phase 1 | 基础框架 | 待实现 |
| Phase 2 | 核心功能 | 待规划 |
| ... | ... | ... |

## 九、设计原则

列出遵循的核心设计原则。
```

### 第六步：向用户确认

使用 AskUserQuestion 询问用户设计文档是否满足需求，是否需要修改。

## 不包含的内容

- 代码实现细节（函数体逻辑）
- 使用示例代码
- 测试用例代码
- 运行输出示例

## 项目特定约束

CoreNet 项目的设计文档应遵循以下约定：
1. 使用中文编写
2. 遵循零外部依赖原则（仅使用 Rust 标准库）
3. 采用纯模拟模型（无真实网络接口）
4. 优先考虑可读性和学习价值
