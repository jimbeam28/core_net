# 报文处理模块设计

## 概述

报文处理模块（Engine）是 CoreNet 协议栈的**协议分发和处理协调中心**。它的核心职责是作为**薄层**（thin layer）调用各协议模块的接口来完成逐层协议解析，而非包含具体的协议处理逻辑。

**设计原则**：processor 应该是轻量级的，主要职责是协议分发和调用协调，具体的协议解析和处理逻辑由各协议模块自身实现。

**当前阶段目标**：实现链路层协议分发（VLAN、ARP），建立清晰的协议处理调用流程。

---

## 一、需求介绍

### 1.1 功能需求

1. **协议分发**：根据以太网类型字段，将报文分发到对应的协议处理模块
2. **调用协调**：调用各协议模块的解析接口，逐层处理报文
3. **VLAN 解析调用**：调用 VLAN 模块解析 VLAN 标签
4. **ARP 处理调用**：调用 ARP 模块解析和处理 ARP 报文
5. **错误传播**：将协议模块返回的错误向上传播，提供清晰的错误信息

### 1.2 非功能需求

- **薄层设计**：processor 本身不包含具体的协议处理逻辑，只负责调用
- **零外部依赖**：仅使用 Rust 标准库
- **可读性优先**：调用流程清晰，便于理解协议分发机制
- **渐进式实现**：随新协议模块的增加而扩展分发逻辑

---

## 二、架构设计

### 2.1 模块定位

```
┌──────────────┐         ┌──────────────┐         ┌──────────────┐
│  报文注入    │  ───>  │  接收队列    │  ───>  │  报文处理    │
│  (Injector) │  RxQ   │  (RingQueue) │         │ (Processor)  │
└──────────────┘         └──────────────┘         └──────┬───────┘
                                                       │
                           协议分发调用                   v
                       ┌──────────────────────────────┐
                       │                              │
                       v                              v
                ┌──────────────┐              ┌──────────────┐
                │  VLAN模块    │              │  ARP模块     │
                │  (解析标签)   │              │  (处理报文)   │
                └──────────────┘              └──────────────┘
                       │                              │
                       └──────────┬───────────────────┘
                                  v
                         ┌──────────────┐
                         │  发送队列    │
                         │  (TxQ)       │
                         └──────────────┘
```

### 2.2 职责边界

**Processor 的职责**：
- 以太网头解析（调用 `EthernetHeader::from_packet()`）
- 协议类型分发（根据 EtherType 决定调用哪个协议模块）
- 协议模块调用协调（调用各协议模块的接口）
- 错误转换和传播

**协议模块的职责**（不属于 Processor）：
- VLAN 标签解析和验证
- ARP 报文解析、缓存更新、响应生成
- 未来：IPv4/IPv6、TCP/UDP、ICMP 等协议的具体处理

### 2.3 协议处理层次

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

### 2.4 数据流向

**上行（解析）流程**：
```
原始字节流 -> Packet 描述符 -> Processor::process()
                                    |
                                    v
                          EthernetHeader::from_packet()
                                    |
                                    v
                          dispatch_by_ether_type()
                                    |
            +-----------------------+-----------------------+
            |                       |                       |
            v                       v                       v
      VLAN 处理调用            ARP 处理调用            IPv4/IPv6 (规划中)
            |                       |
    vlan::process_vlan_packet()  arp::process_arp()
            |                       |
            +-----------> 内层协议分发 <------------+
                        |
                        v
                    返回处理结果
```

### 2.5 处理模型

```
┌─────────────────────────────────────────────────────────┐
│              PacketProcessor (薄层设计)               │
│  ┌─────────────────────────────────────────────────┐  │
│  │        process(packet)                           │  │
│  │                                                 │  │
│  │  1. 打印报文信息（调试）                         │  │
│  │  2. 调用 EthernetHeader::from_packet()           │  │
│  │  3. 调用 dispatch_by_ether_type()                │  │
│  │                                                 │  │
│  │  dispatch_by_ether_type():                       │  │
│  │  ┌─────────────────────────────────────────┐    │  │
│  │  │ match ether_type {                      │    │  │
│  │  │   VLAN => 调用 handle_vlan()            │    │  │
│  │  │   ARP  => 调用 handle_arp()             │    │  │
│  │  │   IP   => 返回未实现错误                │    │  │
│  │  │ }                                       │    │  │
│  │  └─────────────────────────────────────────┘    │  │
│  │                                                 │  │
│  │  handle_vlan():                                 │  │
│  │  ┌─────────────────────────────────────────┐    │  │
│  │  │ 调用 vlan::process_vlan_packet()         │    │  │
│  │  │ 获取 inner_type，递归调用 dispatch       │    │  │
│  │  └─────────────────────────────────────────┘    │  │
│  │                                                 │  │
│  │  handle_arp():                                  │  │
│  │  ┌─────────────────────────────────────────┐    │  │
│  │  │ 调用 ArpPacket::from_packet() 解析       │    │  │
│  │  │ 调用 arp::process_arp() 处理             │    │  │
│  │  │ 返回响应报文（如果有）                   │    │  │
│  │  └─────────────────────────────────────────┘    │  │
│  └─────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────┘
```

### 2.6 接口调用关系

```
┌──────────────────┐         调用          ┌──────────────────┐
│   Processor      │ ──────────────────> │   协议模块        │
├──────────────────┤                      ├──────────────────┤
│ process()        │                      │ VLAN 模块         │
│ dispatch()       │ <────────────────── │ process_vlan_pkt()│
│ handle_vlan()    │         返回         │ VlanTag::parse()  │
│ handle_arp()     │                      ├──────────────────┤
└──────────────────┘                      │ ARP 模块          │
                                          │ ArpPacket::parse()│
                                          │ process_arp()     │
                                          │ encapsulate_eth() │
                                          └──────────────────┘
```

---

## 三、核心数据结构

### 3.1 PacketProcessor

报文处理器，作为**薄层**负责协议分发和调用协调。

```rust
/// 报文处理器
///
/// 职责：
/// - 接收 Packet 并进行协议分发
/// - 调用各协议模块的接口完成具体处理
/// - 转换和传播协议模块的错误
pub struct PacketProcessor {
    /// 处理器名称（用于调试输出）
    name: String,

    /// 是否启用详细输出（调用协议模块时传递）
    verbose: bool,
}
```

### 3.2 ProcessError

处理错误类型，负责转换各协议模块的错误。

```rust
/// 报文处理错误
pub enum ProcessError {
    /// 报文解析错误（来自各协议模块的解析错误）
    ParseError(String),

    /// 报文封装错误（未来用于下行封装）
    EncapError(String),

    /// 不支持的协议（分发时遇到未知协议类型）
    UnsupportedProtocol(String),

    /// 报文格式错误（来自协议模块的格式验证）
    InvalidPacket(String),
}

// Processor 负责实现错误转换
impl From<CoreError> for ProcessError { ... }
impl From<VlanError> for ProcessError { ... }
// 未来添加: From<IpError>, From<TcpError> 等
```

### 3.3 处理结果类型

```rust
/// 处理结果类型
///
/// - Ok(Some(packet)): 处理成功，有响应报文需要发送
/// - Ok(None): 处理成功，无需响应
/// - Err(ProcessError): 处理失败
pub type ProcessResult = Result<Option<Packet>, ProcessError>;
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
    /// # 处理流程
    /// 1. 打印报文信息（如果 verbose）
    /// 2. 调用 `EthernetHeader::from_packet()` 解析以太网头
    /// 3. 调用 `dispatch_by_ether_type()` 进行协议分发
    ///
    /// # 参数
    /// - packet: 要处理的报文（按值传递，取得所有权）
    ///
    /// # 返回
    /// - Ok(Some(response)): 有响应报文需要发送
    /// - Ok(None): 处理成功，无需响应
    /// - Err(ProcessError): 处理失败
    pub fn process(&self, packet: Packet) -> ProcessResult;

    /// 根据以太网类型分发报文
    ///
    /// # 调用关系
    /// - VLAN: 调用 `handle_vlan()`
    /// - ARP: 调用 `handle_arp()`
    /// - IPv4/IPv6: 返回未实现错误
    fn dispatch_by_ether_type(&self, eth_hdr: EthernetHeader, packet: Packet) -> ProcessResult;

    /// 处理 VLAN 报文
    ///
    /// # 调用关系
    /// - 调用 `vlan::process_vlan_packet()` 解析 VLAN 标签
    /// - 调用 `dispatch_inner_vlan()` 分发内层协议
    fn handle_vlan(&self, eth_hdr: EthernetHeader, packet: Packet) -> ProcessResult;

    /// 处理内层协议（去除 VLAN 标签后）
    ///
    /// # 调用关系
    /// - ARP: 调用 `handle_arp_packet()`
    /// - IPv4/IPv6: 返回未实现错误
    fn dispatch_inner_vlan(
        &self,
        eth_hdr: EthernetHeader,
        outer_vlan: Option<VlanTag>,
        inner_vlan: Option<VlanTag>,
        inner_type: u16,
        packet: Packet,
    ) -> ProcessResult;

    /// 处理普通以太网帧中的 ARP 报文
    ///
    /// # 调用关系
    /// - 验证目标 MAC 地址
    /// - 调用 `handle_arp_packet()`
    fn handle_arp(&self, eth_hdr: EthernetHeader, packet: Packet) -> ProcessResult;

    /// 处理 ARP 报文（统一入口）
    ///
    /// # 调用关系
    /// - 调用 `ArpPacket::from_packet()` 解析
    /// - 调用 `arp::process_arp()` 处理并生成响应
    ///
    /// # 参数（未来需要传递）
    /// - cache: ARP 缓存的可变引用
    /// - local_mac: 本接口 MAC 地址
    /// - local_ip: 本接口 IP 地址
    fn handle_arp_packet(&self, packet: Packet) -> ProcessResult;
}
```

### 4.2 协议模块接口（由各协议模块提供）

以下是 Processor 调用的协议模块接口定义，这些接口由各协议模块实现：

#### VLAN 模块接口

```rust
// 位置: src/protocols/vlan/parse.rs

/// VLAN 处理结果
pub struct VlanProcessResult {
    pub inner_type: u16,           // 内层协议类型
    pub outer_vlan: Option<VlanTag>, // 外层 VLAN 标签
    pub inner_vlan: Option<VlanTag>, // 内层 VLAN 标签 (QinQ)
}

/// 处理 VLAN 报文
pub fn process_vlan_packet(packet: &mut Packet) -> Result<VlanProcessResult, VlanError>;
```

#### ARP 模块接口

```rust
// 位置: src/protocols/arp/mod.rs

/// ARP 报文结构
pub struct ArpPacket {
    pub hardware_type: u16,
    pub protocol_type: u16,
    pub hardware_addr_len: u8,
    pub protocol_addr_len: u8,
    pub operation: ArpOperation,
    pub sender_hardware_addr: MacAddr,
    pub sender_protocol_addr: Ipv4Addr,
    pub target_hardware_addr: MacAddr,
    pub target_protocol_addr: Ipv4Addr,
}

impl ArpPacket {
    /// 从 Packet 解析 ARP 报文
    pub fn from_packet(packet: &mut Packet) -> Result<Self, CoreError>;
}

/// ARP 处理结果
pub enum ArpProcessResult {
    NoReply,              // 不需要响应
    Reply(Vec<u8>),       // 需要发送的响应报文（已封装以太网帧）
}

/// 处理 ARP 报文（统一入口）
///
/// # 参数
/// - packet: 已去除以太网头部的 Packet
/// - local_mac: 本接口 MAC 地址
/// - local_ip: 本接口 IP 地址
/// - src_mac: 原始帧源 MAC（用于响应）
/// - cache: ARP 缓存可变引用
/// - ifindex: 接口索引
/// - verbose: 详细输出标志
pub fn process_arp(
    packet: &mut Packet,
    local_mac: MacAddr,
    local_ip: Ipv4Addr,
    src_mac: MacAddr,
    cache: &mut ArpCache,
    ifindex: u32,
    verbose: bool,
) -> Result<ArpProcessResult, CoreError>;
```

### 4.3 便捷函数

```rust
/// 使用默认处理器处理报文
pub fn process_packet(packet: Packet) -> ProcessResult;

/// 使用详细输出模式处理报文
pub fn process_packet_verbose(packet: Packet) -> ProcessResult;
```

---

## 五、协议处理流程

### 5.1 以太网头解析

Processor 调用 `EthernetHeader::from_packet()` 解析以太网头部。

**调用接口**：
```rust
// 位置: src/common/packet.rs (或 common 模块)
let eth_hdr = EthernetHeader::from_packet(&mut packet)?;
```

**返回值**：解析后的以太网头部结构，包含：
- `dst_mac`: 目标 MAC 地址
- `src_mac`: 源 MAC 地址
- `ether_type`: 以太网类型字段

### 5.2 VLAN 处理

**VLAN 模块位置**：[src/protocols/vlan/](src/protocols/vlan/)

**Processor 调用流程**：
```
handle_vlan() 被调用
    |
    v
调用 vlan::process_vlan_packet(&mut packet)
    |
    v
获得 VlanProcessResult { inner_type, outer_vlan, inner_vlan }
    |
    v
根据 inner_type 递归调用 dispatch_inner_vlan()
```

**调用的接口**：
```rust
// 位置: src/protocols/vlan/parse.rs
use crate::protocols::vlan::{process_vlan_packet, VlanProcessResult};

let result = process_vlan_packet(&mut packet)?;
// result.inner_type: 内层协议类型
// result.outer_vlan: 外层 VLAN 标签
// result.inner_vlan: 内层 VLAN 标签 (QinQ)
```

**返回值**：`VlanProcessResult` 结构，包含：
- `inner_type`: 去除 VLAN 标签后的内层协议类型
- `outer_vlan`: 外层 VLAN 标签（Option）
- `inner_vlan`: 内层 VLAN 标签（Option，QinQ 场景）

### 5.3 ARP 处理

**ARP 模块位置**：[src/protocols/arp/](src/protocols/arp/)

**Processor 调用流程**：
```
handle_arp() 被调用
    |
    v
验证目标 MAC 地址（广播或本机）
    |
    v
调用 handle_arp_packet()
    |
    v
调用 ArpPacket::from_packet(&mut packet) 解析
    |
    v
调用 arp::process_arp() 处理并生成响应
    |
    v
返回 ArpProcessResult (NoReply 或 Reply)
```

**调用的接口**：
```rust
// 位置: src/protocols/arp/mod.rs
use crate::protocols::arp::{
    ArpPacket, ArpOperation, process_arp, ArpProcessResult
};

// 解析 ARP 报文
let arp_pkt = ArpPacket::from_packet(&mut packet)?;

// 处理 ARP 报文（需要提供接口上下文）
let result = process_arp(
    &mut packet,
    local_mac,      // 本接口 MAC 地址
    local_ip,       // 本接口 IP 地址
    src_mac,        // 原始帧源 MAC
    &mut cache,     // ARP 缓存
    ifindex,        // 接口索引
    verbose,        // 详细输出标志
)?;

// 返回值: ArpProcessResult
// - NoReply: 不需要发送响应
// - Reply(frame_bytes): 需要发送的响应报文（已封装以太网帧）
```

**ARP 模块职责**（由 ARP 模块实现，Processor 不包含）：
- 解析 ARP 报文格式
- 更新 ARP 缓存表（自动学习）
- 判断是否需要响应（检查目标 IP）
- 生成 ARP 响应报文
- 封装以太网帧

**ARP 缓存状态**（由 ARP 模块管理）：
```
NONE -> INCOMPLETE -> REACHABLE -> STALE -> DELAY -> PROBE -> NONE
```

### 5.4 协议类型分发

**分发逻辑**（Processor 职责）：
```rust
// 伪代码：dispatch_by_ether_type()
match ether_type {
    ETH_P_8021Q | ETH_P_8021AD => {
        // 调用 VLAN 模块处理
        self.handle_vlan(eth_hdr, packet)?;
    }
    ETH_P_ARP => {
        // 调用 ARP 模块处理
        self.handle_arp(eth_hdr, packet)?;
    }
    ETH_P_IP => {
        return Err(ProcessError::UnsupportedProtocol(
            String::from("IPv4 protocol not implemented")
        ));
    }
    ETH_P_IPV6 => {
        return Err(ProcessError::UnsupportedProtocol(
            String::from("IPv6 protocol not implemented")
        ));
    }
    _ => {
        return Err(ProcessError::UnsupportedProtocol(
            format!("Unknown ethernet type: 0x{:04x}", ether_type)
        ));
    }
}
```

### 5.5 模块职责清单

| 功能 | 负责模块 | Processor 职责 |
|------|---------|---------------|
| 以太网头解析 | Ethernet 模块 | 调用 `from_packet()` |
| VLAN 标签解析 | VLAN 模块 | 调用 `process_vlan_packet()` |
| VLAN 信息打印 | Processor | 根据 verbose 标志打印 |
| ARP 报文解析 | ARP 模块 | 调用 `ArpPacket::from_packet()` |
| ARP 缓存更新 | ARP 模块 | 调用 `process_arp()` 内部处理 |
| ARP 响应生成 | ARP 模块 | 调用 `process_arp()` 返回响应 |
| 协议类型分发 | Processor | 根据 EtherType 调用对应模块 |
| 错误转换 | Processor | 实现From trait，转换各模块错误 |
```

---

## 六、错误处理

### 6.1 错误转换（Processor 职责）

作为薄层，Processor 负责将各协议模块的错误转换为 `ProcessError`。

```rust
/// 从 CoreError 转换（通用错误）
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

/// 从 VlanError 转换
impl From<crate::protocols::vlan::VlanError> for ProcessError {
    fn from(err: crate::protocols::vlan::VlanError) -> Self {
        ProcessError::ParseError(format!("VLAN错误: {}", err))
    }
}

// 未来添加：
// impl From<crate::protocols::ipv4::IpError> for ProcessError { ... }
// impl From<crate::protocols::tcp::TcpError> for ProcessError { ... }
```

### 6.2 错误处理策略

1. **解析失败**：协议模块返回解析错误，Processor 转换后向上传播
2. **不支持的协议**：Processor 在分发时检测并返回 `UnsupportedProtocol`
3. **格式错误**：协议模块验证后返回格式错误，Processor 转换后传播
4. **错误不吞没**：所有错误都向上传播，由调用者决定如何处理

### 6.3 错误流向

```
┌──────────────┐    From trait    ┌──────────────┐
│ 协议模块错误  │ ───────────────> │ ProcessError │
│ VlanError    │                  │              │
│ ArpError     │ <─────────────── │ (统一类型)   │
│ IpError      │    转换逻辑      │              │
└──────────────┘                  └──────────────┘
                                          │
                                          v
                                    ┌──────────────┐
                                    │  调用者      │
                                    │ (Scheduler)  │
                                    └──────────────┘
```

---

## 七、接口依赖图

### 7.1 Processor 依赖的协议模块

```
┌─────────────────────────────────────────────────────────┐
│                    PacketProcessor                      │
│  ┌─────────────────────────────────────────────────┐   │
│  │                                                 │   │
│  │  依赖的协议模块接口:                             │   │
│  │                                                 │   │
│  │  ┌─────────────────────────────────────────┐   │   │
│  │  │ Ethernet 模块                            │   │   │
│  │  │ - EthernetHeader::from_packet()         │   │   │
│  │  └─────────────────────────────────────────┘   │   │
│  │                                                 │   │
│  │  ┌─────────────────────────────────────────┐   │   │
│  │  │ VLAN 模块                                │   │   │
│  │  │ - process_vlan_packet()                 │   │   │
│  │  │ - VlanTag::parse_from_packet()          │   │   │
│  │  │ - VlanError                             │   │   │
│  │  └─────────────────────────────────────────┘   │   │
│  │                                                 │   │
│  │  ┌─────────────────────────────────────────┐   │   │
│  │  │ ARP 模块                                 │   │   │
│  │  │ - ArpPacket::from_packet()              │   │   │
│  │  │ - process_arp()                         │   │   │
│  │  │ - ArpProcessResult                      │   │   │
│  │  │ - encapsulate_ethernet()                │   │   │
│  │  └─────────────────────────────────────────┘   │   │
│  │                                                 │   │
│  │  未来依赖:                                      │   │
│  │  - IPv4/IPv6 模块                              │   │
│  │  - ICMP 模块                                   │   │
│  │  - TCP/UDP 模块                               │   │
│  │                                                 │   │
│  └─────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────┘
```

### 7.2 新增协议模块的集成方式

当需要新增协议模块时，按以下步骤集成：

1. **协议模块实现**：
   - 实现 `XXXPacket::from_packet()` 解析接口
   - 实现 `process_xxx()` 处理接口
   - 定义 `XXXError` 错误类型

2. **Processor 添加分发**：
   - 在 `dispatch_by_ether_type()` 中添加新的 match 分支
   - 实现 `handle_xxx()` 方法调用协议模块接口
   - 实现 `From<XXXError> for ProcessError` 错误转换

3. **示例**（新增 IPv4 支持）：
```rust
// 1. 在 dispatch_by_ether_type() 中添加
ETH_P_IP => {
    self.handle_ipv4(eth_hdr, packet)?;
}

// 2. 实现 handle_ipv4()
fn handle_ipv4(&self, eth_hdr: EthernetHeader, packet: Packet) -> ProcessResult {
    // 调用 IPv4 模块接口
    let result = ipv4::process_ipv4_packet(&mut packet, ...)?;
    // 处理结果...
}

// 3. 实现错误转换
impl From<ipv4::IpError> for ProcessError {
    fn from(err: ipv4::IpError) -> Self {
        ProcessError::ParseError(format!("IP错误: {}", err))
    }
}
```

---

## 八、模块结构

```
src/engine/
├── mod.rs           # 模块入口，导出公共接口
├── processor.rs     # PacketProcessor 实现（薄层设计） ✅ 已实现
├── context.rs      # 处理上下文（规划中，用于传递接口信息）
└── dispatcher.rs   # 协议分发器（未来可选，将分发逻辑独立）
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

## 九、测试策略

### 9.1 单元测试

#### 9.1.1 测试范围

**PacketProcessor 基础功能测试**：
- **正常路径**：创建处理器、命名处理器、设置 verbose 模式、获取处理器名称
- **边界条件**：空报文、最小报文、最大报文
- **错误路径**：无效报文格式、不支持的协议类型

**协议分发测试**：
- **正常路径**：VLAN 报文分发、ARP 报文分发、IPv4/IPv6 报文分发（未来）
- **边界条件**：未知 EtherType、无效 VLAN 标签
- **错误路径**：协议解析失败、格式错误

**VLAN 处理调用测试**：
- **正常路径**：单层 VLAN 标签、QinQ 双层标签、内层协议分发
- **边界条件**：VLAN ID=0、VLAN ID=4095
- **错误路径**：无效 VLAN TCI、截断的 VLAN 报文

**ARP 处理调用测试**：
- **正常路径**：ARP 请求处理、ARP 响应处理、ARP 缓存更新
- **边界条件**：广播目标 MAC、单播目标 MAC
- **错误路径**：无效 ARP 操作码、格式错误

**错误转换测试**：
- **CoreError 转换**：解析错误、格式错误、不支持的协议
- **VlanError 转换**：VLAN 解析错误
- **未来扩展**：IpError、TcpError 等

#### 9.1.2 测试组织

测试代码按以下类别组织：

- **基础功能测试组**：处理器创建、命名、verbose 模式
- **协议分发测试组**：VLAN、ARP、IPv4/IPv6 分发
- **VLAN 处理测试组**：单层标签、QinQ、边界 TCI、截断报文
- **ARP 处理测试组**：请求处理、响应处理、无效操作码
- **完整流程测试组**：多层协议解析、错误传播
- **错误转换测试组**：CoreError、VlanError 等

测试辅助函数：
- 报文构造函数：`create_vlan_packet()`, `create_arp_packet()`, `create_qinq_packet()` 等
- 以太网头构造：`create_eth_header()`, `create_eth_header_with_mac()`
- 边界测试数据：`create_truncated_packet()`, `create_malformed_packet()`

#### 9.1.3 测试覆盖要点

| 测试维度 | 覆盖要点 |
|---------|---------|
| **公共接口** | `PacketProcessor::new()`, `with_name()`, `with_verbose()`, `process()`<br>`process_packet()`, `process_packet_verbose()` 便捷函数 |
| **内部逻辑** | `dispatch_by_ether_type()` 的所有 match 分支<br>`handle_vlan()` 的 VLAN 解析和内层分发<br>`handle_arp()` 的验证和处理流程 |
| **边界条件** | 空 Packet、最小/最大报文长度<br>VLAN ID=0/4095 边界<br>广播 MAC 地址处理 |
| **错误处理** | `ProcessError` 所有变体<br>各协议错误到 ProcessError 的转换<br>截断/畸形报文的处理 |
| **协议调用** | 正确调用 `vlan::process_vlan_packet()`<br>正确调用 `arp::process_arp()`<br>验证接口参数传递 |

### 9.2 集成测试

#### 9.2.1 测试场景

**场景一：VLAN + ARP 完整流程**
- **涉及模块**：ethernet、vlan、arp、processor
- **测试内容**：
  - 注入完整的以太网帧（带 VLAN 标签 + ARP 报文）
  - 验证逐层解析流程
  - 验证 VLAN 模块正确解析标签
  - 验证 ARP 模块正确处理并生成响应
  - 验证返回的响应报文格式正确

**场景二：多标签 VLAN 报文处理**
- **涉及模块**：vlan、processor
- **测试内容**：
  - 注入 QinQ 双层标签报文
  - 验证外层和内层标签都被正确解析
  - 验证内层协议被正确分发

**场景三：处理器与调度器集成**
- **涉及模块**：processor、scheduler
- **测试内容**：
  - 调度器从队列取出报文
  - 调用 processor.process() 处理
  - 验证响应报文放入发送队列

#### 9.2.2 测试依赖

- **协议模块**：VLAN、ARP 模块的正确实现
- **测试数据**：预构造的各种协议报文（字节数组）
- **接口上下文**：需要模拟本地接口信息（MAC/IP）

### 9.3 测试数据设计

#### 9.3.1 测试数据来源

- **手工构造报文**：使用字节数组构造各种协议报文
- **辅助函数**：提供 `create_xxx_packet()` 系列函数
- **真实抓包数据**：从实际网络捕获的报文样本（未来）

#### 9.3.2 测试数据管理

使用辅助函数构造测试报文和以太网头：

- 报文构造：`create_vlan_packet()`, `create_arp_packet()`, `create_qinq_packet()`
- 以太网头：`create_eth_header()`, `create_eth_header_with_mac()`
- 边界测试：`create_truncated_packet()`, `create_malformed_packet()`

协议常量：
- `ETH_P_ARP: 0x0806`
- `ETH_P_8021Q: 0x8100`
- `ETH_P_8021AD: 0x88A8`

### 9.4 Mock 和桩设计

#### 9.4.1 需要模拟的组件

- **接口信息**：模拟本地接口的 MAC/IP 地址
- **ARP 缓存**：使用测试专用的 ARP 缓存实例
- **协议模块**：对于 processor 测试，可以使用真实的协议模块

#### 9.4.2 测试替身策略

使用测试专用的接口上下文结构：

```text
TestInterfaceContext {
    local_mac: MacAddr       // 本接口 MAC 地址
    local_ip: Ipv4Addr        // 本接口 IP 地址
}
```

提供默认测试值用于 ARP 处理测试。

### 9.5 测试执行计划

```bash
# 运行 engine 模块所有测试
cargo test engine

# 运行 processor 测试
cargo test processor

# 运行特定测试
cargo test test_handle_arp_request

# 显示测试输出（包括 verbose 输出）
cargo test -- --nocapture

# 运行文档测试
cargo test --doc
```

---

## 十、实现路线图

### 9.1 单元测试

```rust
#[cfg(test)]
mod tests {
    // 基础功能测试
    - test_processor_creation()        // 测试创建处理器
    - test_processor_with_name()       // 测试命名处理器
    - test_processor_verbose()         // 测试详细输出模式
    - test_dispatch_vlan()             // 测试 VLAN 分发
    - test_dispatch_arp()              // 测试 ARP 分发
    - test_error_conversion()          // 测试错误转换
}
```

### 9.2 集成测试

**VLAN 测试**：
- 注入带 VLAN 标签的以太网帧
- 验证调用 `process_vlan_packet()` 成功
- 测试 QinQ 双层标签的分发

**ARP 测试**：
- 注入 ARP 请求报文
- 验证调用 `process_arp()` 成功
- 验证返回的响应报文格式

**完整流程测试**：
- 注入完整报文（以太网头 + VLAN + ARP）
- 验证逐层解析和调用流程
- 验证响应报文正确返回

---

## 十、实现路线图

| 阶段 | 内容 | 状态 |
|------|------|------|
| Phase 1 | 基础框架 + 薄层调用设计 | ✅ 已完成 |
| Phase 2 | VLAN 协议调用 | ✅ 已完成 |
| Phase 3 | ARP 协议调用 | ✅ 已完成 |
| Phase 4 | 以太网类型分发 | ✅ 已完成 |
| Phase 5 | 接口上下文传递（传递 MAC/IP） | 🔄 进行中 |
| Phase 6 | IPv4 协议调用 | 📋 待规划 |
| Phase 7 | ICMP 协议调用 | 📋 待规划 |
| Phase 8 | 传输层协议调用（TCP/UDP） | 📋 待规划 |

---

## 十一、设计原则

1. **薄层设计**：processor 只负责协议分发和调用协调，不包含具体处理逻辑
2. **职责清晰**：每个协议模块负责自己的解析和处理，processor 只做分发
3. **错误透明**：通过 From trait 转换各模块错误，向上传播不吞没信息
4. **渐进扩展**：新增协议只需添加分发分支和错误转换，不影响现有代码
5. **接口驱动**：协议模块提供标准化接口，processor 通过接口调用
6. **可测试性**：薄层设计便于 mock 协议模块进行单元测试

---

## 十二、相关文档

- [ARP 协议设计](protocols/arp.md)
- [VLAN 协议设计](common/protocols/vlan.md)
- [数据包描述符](packet.md)
- [环形队列](queue.md)
