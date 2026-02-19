---
name: proto-apply
description: 根据协议设计文档实现协议代码。按照协议规范实现数据结构、报文处理逻辑、状态机，并使用 test_framework 编写集成测试。
allowed-tools: Read, Write, Edit, AskUserQuestion, Glob, Grep, Bash, TodoWrite
---

# 协议代码实现

根据协议设计文档实现完整的协议模块代码。

## 执行流程

### 第一步：确认路径信息

如未指定，使用 AskUserQuestion 询问：
1. 协议设计文档路径（默认：`docs/design/protocols/{protocol}.md`）
2. 代码存放路径（默认：`src/protocols/{protocol}/`）
3. 测试文件路径（默认：`tests/{protocol}_integration_test.rs`）

### 第二步：读取并分析协议设计文档

提取：报文格式、状态机、处理逻辑、数据结构、表项/缓存、与其他模块的交互、测试场景。

### 第三步：检查相关模块的设计文档和代码

查看 `docs/design/` 和 `src/` 中的相关模块，理解接口定义、数据结构和调用关系。

### 第四步：检查目标路径现有代码

如存在，对比设计文档，确定需要新增、修改或补充的内容。

### 第五步：制定实现计划

使用 TodoWrite 创建任务清单：
1. 创建/更新模块结构
2. 实现报文数据结构
3. 实现报文解析函数
4. 实现报文封装函数
5. 实现状态机（如适用）
6. 实现报文处理逻辑
7. 实现表项/缓存管理（如适用）
8. 添加必要的 pub use 导出
9. 实现集成测试用例
10. 在 engine 模块中集成协议
11. 输出实现简报

### 第六步：实现核心协议代码

**模块结构**：`src/protocols/{protocol}/mod.rs` + 各功能文件

**核心接口**：
- `parse(data: &[u8]) -> Result<PacketStruct, ParseError>`
- `encapsulate(packet: &PacketStruct) -> Vec<u8>`
- `process_..._packet(...) -> Result<ProcessResult, Error>`

### 第七步：实现集成测试

测试文件：`tests/{protocol}_integration_test.rs`

使用 test_framework：`TestHarness`、`GlobalStateManager`、`#[serial]`

### 第八步：在 engine 模块中集成协议

在 `src/engine/processor.rs` 中：
1. 添加协议模块引用
2. 添加错误转换（如需要）
3. 在分发函数中添加协议分支
4. 实现协议处理函数，调用协议模块接口

### 第九步：编译验证

```bash
cargo build
```

### 第十步：测试验证

```bash
cargo test --test {protocol}_integration_test
```

### 第十一步：输出实现简报

输出：实现内容、对其他模块的改动、测试结果、编译状态。

## 重要原则

1. **核心逻辑独立实现**：协议逻辑全部在 `src/protocols/{protocol}/`，engine 只调用接口
2. **公共部分和协议相关部分分离**：通用类型用 common，协议特定用本模块
3. **资源申请释放在本模块实现**：init/cleanup 函数在本模块，在上电模块调用
4. **集成测试在 tests/ 目录**：使用 test_framework，不在代码中写单元测试
5. **必须在 engine 中应用**：添加分发逻辑，调用协议模块接口
6. **复用已有模块**：优先使用 common、interface、testframework 的功能

## 完成标准

- 设计文档定义的数据结构和接口已实现
- 代码通过 `cargo build` 编译
- 集成测试全部通过
- engine 模块已完成协议集成
- 复用了已有模块，无重复实现
- 已输出实现简报
