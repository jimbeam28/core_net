# 报文处理模块设计

## 概述

报文处理模块（Engine）是 CoreNet 协议栈的核心处理单元，负责对接收到的报文进行逐层协议解析（上行）或对发送数据进行逐层协议封装（下行）。

**当前阶段目标**：实现链路层协议处理（VLAN、ARP），建立完整的报文处理流程。

---

## 一、需求介绍

### 1.1 功能需求

1. **协议解析（上行）**：从以太网帧开始，逐层解析协议头，提取有效载荷
2. **协议封装（下行）**：从应用数据开始，逐层添加协议头，生成完整报文
3. **VLAN 支持**：处理 802.1Q VLAN 标签，支持单层和双层标签
4. **ARP 处理**：处理地址解析协议，维护 ARP 缓存表
5. **错误处理**：提供清晰的错误信息和传播机制

### 1.2 非功能需求

- **零外部依赖**：仅使用 Rust 标准库
- **纯内存模拟**：所有数据通过内存队列传递
- **可读性优先**：代码结构清晰，便于学习理解
- **渐进式实现**：协议处理按层级逐步完善

---

## 二、架构设计

### 2.1 模块定位

```
┌──────────────┐         ┌──────────────┐         ┌──────────────┐
│  报文注入    │  ───>  │  接收队列    │  ───>  │  报文处理    │
│  (Injector) │  RxQ   │  (RingQueue) │         │  (Engine)     │
└──────────────┘         └──────────────┘         └──────┬───────┘
                                                       │
                                                       v
┌──────────────┐         ┌──────────────┐         ┌──────────────┐
│  结果输出    │  <──  │  发送队列    │  <──  │  协议栈      │
│  (Output)   │  TxQ   │  (RingQueue) │         │  (Protocols) │
└──────────────┘         └──────────────┘         └──────────────┘
```

### 2.2 协议处理层次

```
┌─────────────────────────────────────────────────┐
│           Application Layer (规划中)          │
├─────────────────────────────────────────────────┤
│      Transport Layer (TCP/UDP/ICMP 规划中)   │
├─────────────────────────────────────────────────┤
│         Network Layer (IPv4/IPv6 规划中)      │
├─────────────────────────────────────────────────┤
│    Link Layer (Ethernet/VLAN/ARP ✅ 已实现)   │
└─────────────────────────────────────────────────┘
```

### 2.3 数据流向

**上行（解析）流程**：
```
原始字节流 -> Packet 描述符 -> 链路层解析 -> 协议分发 -> TxQ
    |
    +-> VLAN 解析 (可选)
    +-> ARP 处理
    +-> 以太网类型判断
```

**下行（封装）流程**（规划中）：
```
应用数据 -> 传输层封装 -> 网络层封装 -> 链路层封装 -> TxQ
```

### 2.4 处理模型

```
┌─────────────────────────────────────────────────────┐
│              PacketProcessor                    │
│  ┌───────────────────────────────────────────┐  │
│  │        process(packet)                     │  │
│  │                                         │  │
│  │  1. 打印报文信息（调试）                 │  │
│  │  2. VLAN 解析（如存在）                  │  │
│  │  3. 协议类型分发                         │  │
│  │     - ARP: 处理 ARP 报文                  │  │
│  │     - IPv4: 网络层处理（规划中）         │  │
│  │     - 其他: 不支持的协议                  │  │
│  └───────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────┘
```

---

## 三、核心数据结构

### 3.1 PacketProcessor

报文处理器，负责报文的协议处理。

```rust
/// 报文处理器
pub struct PacketProcessor {
    /// 处理器名称
    name: String,

    /// 是否启用详细输出
    verbose: bool,
}
```

### 3.2 ProcessError

处理错误类型。

```rust
/// 报文处理错误
pub enum ProcessError {
    /// 报文解析错误
    ParseError(String),

    /// 报文封装错误
    EncapError(String),

    /// 不支持的协议
    UnsupportedProtocol(String),

    /// 报文格式错误
    InvalidPacket(String),
}
```

---

## 四、接口定义

### 4.1 PacketProcessor 核心 API

```rust
impl PacketProcessor {
    /// 创建新的报文处理器
    pub fn new() -> Self;

    /// 创建命名处理器
    pub fn with_name(name: String) -> Self;

    /// 启用详细输出
    pub fn with_verbose(self, verbose: bool) -> Self;

    /// 获取处理器名称
    pub fn name(&self) -> &str;

    /// 处理报文（上行解析）
    ///
    /// # 参数
    /// - packet: 要处理的报文（按值传递，取得所有权）
    ///
    /// # 返回
    /// - Ok(()): 处理成功
    /// - Err(ProcessError): 处理失败
    pub fn process(&self, packet: Packet) -> ProcessResult;
}
```

### 4.2 便捷函数

```rust
/// 使用默认处理器处理报文
pub fn process_packet(packet: Packet) -> ProcessResult;

/// 使用详细输出模式处理报文
pub fn process_packet_verbose(packet: Packet) -> ProcessResult;
```

---

## 五、协议处理流程

### 5.1 VLAN 处理

**VLAN 模块位置**：[src/common/protocols/vlan/](src/common/protocols/vlan/)

**处理逻辑**：
```
1. 检查以太网类型字段
2. 如果是 VLAN TPID (0x8100 或 0x88A8)
   - 解析 VLAN 标签 (TCI, TCI, DEI, PCP, VID)
   - 继续解析内层类型
3. 支持 QinQ (双层标签)
```

**相关类型**：
- `VlanTag`: VLAN 标签结构
- `VlanFrame`: 带标签的帧结构
- `VlanError`: VLAN 处理错误

### 5.2 ARP 处理

**ARP 模块位置**：
- 协议定义：[src/protocols/arp/mod.rs](src/protocols/arp/mod.rs)
- 缓存表：[src/common/tables/arp.rs](src/common/tables/arp.rs)

**处理逻辑**：
```
收到 ARP 报文：
1. 解析 ARP 头部
2. 更新 ARP 缓存（自动学习）
3. 判断操作类型：
   - 请求 (Operation=1): 检查目标IP，匹配则发送响应
   - 响应 (Operation=2): 处理等待队列
4. 更新缓存条目状态
```

**ARP 缓存状态**：
```
NONE -> INCOMPLETE -> REACHABLE -> STALE -> DELAY -> PROBE -> NONE
```

**相关类型**：
- `ArpPacket`: ARP 报文结构
- `ArpOperation`: ARP 操作码
- `ArpCache`: ARP 缓存表
- `ArpEntry`: ARP 缓存条目
- `ArpState`: ARP 条目状态
- `ArpConfig`: ARP 配置参数

### 5.3 协议类型分发

```rust
// 伪代码示例
match ether_type {
    ETHER_TYPE_ARP => {
        // 处理 ARP
        let arp_pkt = ArpPacket::from_packet(&mut packet)?;
        handle_arp(arp_pkt);
    }
    ETHER_TYPE_IP => {
        // IPv4 处理（规划中）
        handle_ipv4(packet)?;
    }
    ETHER_TYPE_IPV6 => {
        // IPv6 处理（规划中）
        handle_ipv6(packet)?;
    }
    ETHER_TYPE_VLAN | ETHER_TYPE_VLAN_QINQ => {
        // VLAN 处理
        let vlan_tag = VlanTag::parse(&mut packet)?;
        handle_vlan_inner(vlan_tag, packet)?;
    }
    _ => {
        return Err(ProcessError::UnsupportedProtocol(
            format!("未知的以太网类型: 0x{:04x}", ether_type)
        ));
    }
}
```

---

## 六、错误处理

### 6.1 错误转换

```rust
/// 从 CoreError 转换
impl From<crate::common::CoreError> for ProcessError {
    fn from(err: crate::common::CoreError) -> Self {
        match err {
            CoreError::ParseError(msg) => ProcessError::ParseError(msg),
            CoreError::InvalidPacket(msg) => ProcessError::InvalidPacket(msg),
            CoreError::UnsupportedProtocol(proto) => {
                ProcessError::UnsupportedProtocol(proto)
            }
            _ => ProcessError::EncapError(format!("{:?}", err)),
        }
    }
}
```

### 6.2 错误处理策略

1. **解析失败**：返回 `ParseError`，终止处理
2. **不支持的协议**：返回 `UnsupportedProtocol`，记录日志
3. **格式错误**：返回 `InvalidPacket`，丢弃报文
4. **错误传播**：使用 `?` 操作符向上传播

---

## 七、模块结构

```
src/engine/
├── mod.rs           # 模块入口
├── processor.rs     # PacketProcessor 实现 ✅ 已实现
├── engine.rs       # 报文处理引擎核心逻辑（规划中）
└── context.rs      # 处理上下文（规划中）
```

### 模块导出

```rust
mod processor;

pub use processor::{
    PacketProcessor,
    ProcessError,
    ProcessResult,
    process_packet,
    process_packet_verbose,
};
```

---

## 八、测试策略

### 8.1 单元测试

```rust
#[cfg(test)]
mod tests {
    // 基础功能测试
    - test_processor_creation()
    - test_process_empty_packet()
    - test_process_data_packet()
    - test_convenience_function()
}
```

### 8.2 集成测试

**VLAN 测试**：
- 注入带 VLAN 标签的以太网帧
- 验证 VLAN 解析结果
- 测试 QinQ 双层标签

**ARP 测试**：
- 注入 ARP 请求报文
- 验证缓存更新和响应生成
- 测试 Gratuitous ARP

---

## 九、实现路线图

| 阶段 | 内容 | 状态 |
|------|------|------|
| Phase 1 | 基础框架 + 打印功能 | ✅ 已完成 |
| Phase 2 | VLAN 协议支持 | ✅ 已完成 |
| Phase 3 | ARP 协议处理 | ✅ 已完成 |
| Phase 4 | 以太网类型分发 | 🔄 进行中 |
| Phase 5 | IPv4 协议 | 📋 待规划 |
| Phase 6 | ICMP（ping） | 📋 待规划 |
| Phase 7 | 传输层（TCP/UDP） | 📋 待规划 |

---

## 十、设计原则

1. **简化优先**：当前阶段仅实现必要功能，验证数据流
2. **渐进完善**：协议处理按层级逐步实现
3. **错误透明**：处理错误向上传播，不吞没错误信息
4. **测试驱动**：每阶段添加对应测试用例
5. **模块独立**：各协议模块职责清晰，相互独立

---

## 十一、相关文档

- [ARP 协议设计](protocols/arp.md)
- [VLAN 协议设计](common/protocols/vlan.md)
- [数据包描述符](packet.md)
- [环形队列](queue.md)
