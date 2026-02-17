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

如果信息不足，通过追问明确：

**必需信息**：
- 模块/功能名称
- 功能目标和核心职责
- 主要使用场景

**可选追问**（视情况）：
- 与现有哪些模块有交互关系？
- 有哪些关键操作或处理流程？
- 有什么特殊约束（零依赖、线程安全等）？

### 第三步：分析现有设计文档

如果文档已存在，先读取了解当前设计状态和需要新增/修改的部分。

### 第四步：分析依赖模块

使用 Glob 列出 `docs/design/*.md`，根据模块名称匹配并读取相关依赖文档，了解接口定义、数据结构和交互约定。

**常见依赖映射**：
| 模块名 | 设计文档 | 主要内容 |
|--------|---------|---------|
| queue | docs/design/queue.md | RingQueue 接口、QueueError |
| packet | docs/design/packet.md | Packet 结构、操作方法 |
| engine | docs/design/engine.md | PacketProcessor、process() |
| poweron | docs/design/poweron.md | PowerOnContext、资源初始化 |

### 第五步：生成设计文档

按以下章节结构生成（**保持简洁，重点突出**）：

```markdown
# [模块名称]设计

## 概述

简要描述模块的定位、职责和目标。

## 一、需求介绍

### 1.1 功能需求

- 需求1：描述
- 需求2：描述

### 1.2 约束条件

零依赖、纯内存模拟等特殊约束。

## 二、架构设计

### 2.1 模块定位

描述模块在系统中的位置，用 ASCII 图展示与其他模块的关系。

### 2.2 数据流向

描述数据流经此模块的方式，使用箭头图展示。

## 三、核心数据结构

### 3.1 [结构体名称]

结构体用途说明。

```rust
/// 结构体文档注释
pub struct StructName {
    /// 字段说明
    field_name: Type,
}
```

## 四、接口定义

### 4.1 [接口名称]

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

## 五、模块结构

```
src/[module_name]/
├── mod.rs           # 模块入口
├── file1.rs         # 文件说明
└── file2.rs         # 文件说明
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

## 七、测试策略

### 7.1 单元测试范围

| 测试维度 | 覆盖要点 |
|---------|---------|
| 正常路径 | 正常输入下的预期行为 |
| 边界条件 | 空值、最大值、边界值等 |
| 错误处理 | 各种 Error 分支的验证 |
| 状态转换 |（如适用）状态机转换场景 |

### 7.2 集成测试（如适用）

描述模块间协作的测试场景。

## 八、实现路线图

| 阶段 | 内容 | 状态 |
|------|------|------|
| Phase 1 | 基础框架 | 待实现 |
| Phase 2 | 核心功能 | 待规划 |
```

### 第六步：向用户确认

使用 AskUserQuestion 询问用户设计文档是否满足需求，是否需要修改。

## 不包含的内容

- 代码实现细节（函数体逻辑）
- 使用示例代码
- 测试用例的具体实现代码
- 运行输出示例

## 项目特定约束

CoreNet 项目设计文档约定：
1. 使用中文编写
2. 零外部依赖（仅 Rust 标准库）
3. 纯模拟模型（无真实网络接口）
4. 优先可读性和学习价值
