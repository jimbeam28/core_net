# 报文处理模块设计

## 概述

报文处理模块（Engine）是 CoreNet 协议栈的核心处理单元，负责对接收到的报文进行逐层协议解析（上行）或对发送数据进行逐层协议封装（下行）。

**当前阶段目标**：实现基础框架，提供报文处理接口。处理函数内部暂时简化为打印报文内容，用于验证数据流的正确性。

---

## 一、架构设计

### 模块定位

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

### 数据流向

**上行（解析）流程**：
```
测试报文 -> RxQ -> Engine::process() -> 协议解析 -> TxQ（响应）
```

**下行（封装）流程**：
```
应用数据 -> Engine::encap() -> 协议封装 -> TxQ
```

### 处理模型

```
┌─────────────────────────────────────────────┐
│            PacketProcessor                 │
│  ┌─────────────────────────────────────┐  │
│  │        process(packet)               │  │
│  │                                     │  │
│  │  阶段一：打印报文信息（当前实现）    │  │
│  │  阶段二：以太网层解析（后续）        │  │
│  │  阶段三：IP层解析（后续）            │  │
│  │  阶段四：传输层解析（后续）          │  │
│  └─────────────────────────────────────┘  │
└─────────────────────────────────────────────┘
```

---

## 二、核心数据结构

### PacketProcessor

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

### ProcessError

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

## 三、接口定义

### 3.1 PacketProcessor 核心 API

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

### 3.2 便捷函数

```rust
/// 使用默认处理器处理报文
pub fn process_packet(packet: Packet) -> ProcessResult;

/// 使用详细输出模式处理报文
pub fn process_packet_verbose(packet: Packet) -> ProcessResult;
```

---

## 四、当前实现（Phase 1）

### 4.1 处理行为

**简化版本（当前）**：
1. 接收 Packet 入参
2. 打印报文基本信息（长度、偏移量）
3. 如启用 verbose，打印十六进制 dump
4. 返回 Ok(())

### 4.2 输出格式

**普通模式**：
```
报文处理 [DefaultProcessor]: 长度=64 字节
```

**详细模式**：
```
=== 报文处理 [DefaultProcessor] ===
报文长度: 64 字节
当前偏移: 0 字节
剩余数据: 64 字节
报文内容:
0000: ff ff ff ff ff ff aa bb cc dd ee ff 08 00 45 00  |..............E.
0010: 00 3c 00 00 40 00 40 11 xx xx xx xx xx xx xx xx  |.<..@.@.........
0020: xx xx xx xx xx xx 00 14 00 35 00 28 xx xx 61 62  |.........5.(..ab
0030: 63 64 65 66 67 68 69 6a 6b 6c 6d 6e 6f 70 71 72  |cdefghijklmnopqr
0040: 73 74 75 76 77 61 62 63 64 65 66 67 68 69        |stuvwabcdefghi
====================
```

---

## 五、后续扩展（Phase 2+）

### 5.1 协议解析流程

```
process(packet) {
    // 1. 以太网层解析
    let eth_hdr = parse_ethernet(packet)?;
    match eth_hdr.ether_type {
        ETH_P_IP => {
            // 2. IPv4 层解析
            let ip_hdr = parse_ipv4(packet)?;
            match ip_hdr.protocol {
                IP_PROTO_ICMP => {
                    // 3. ICMP 处理
                    handle_icmp(packet)?;
                }
                IP_PROTO_TCP => {
                    // 3. TCP 处理
                    handle_tcp(packet)?;
                }
                IP_PROTO_UDP => {
                    // 3. UDP 处理
                    handle_udp(packet)?;
                }
                _ => { /* 忽略 */ }
            }
        }
        ETH_P_ARP => {
            // 2. ARP 处理
            handle_arp(packet)?;
        }
        ETH_P_IPV6 => {
            // 2. IPv6 层解析
            parse_ipv6(packet)?;
        }
        _ => { /* 忽略 */ }
    }
}
```

### 5.2 协议封装流程

```
encap(data, dest) -> Packet {
    // 1. 传输层封装
    add_transport_header(data)?;

    // 2. 网络层封装
    add_network_header(dest)?;

    // 3. 链路层封装
    add_ethernet_header(dest)?;

    // 4. 返回完整报文
    packet
}
```

### 5.3 错误处理增强

```rust
pub enum ProcessError {
    // 当前
    ParseError(String),
    EncapError(String),
    UnsupportedProtocol(String),
    InvalidPacket(String),

    // 后续扩展
    ChecksumError { expected: u16, calculated: u16 },
    RoutingError(String),
    FragmentationError(String),
    TimeoutError(Duration),
}
```

---

## 六、模块结构

```
src/engine/
├── mod.rs           # 模块入口
├── processor.rs     # PacketProcessor 实现
├── engine.rs       # 报文处理引擎核心逻辑（后续）
└── context.rs      # 处理上下文（后续）
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

## 七、测试策略

### 7.1 单元测试

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

### 7.2 集成测试（后续）

```
测试场景：
- 注入完整以太网帧 -> 验证解析结果
- 注入 IP 报文 -> 验证协议识别
- 注入 TCP 报文 -> 验证端口解析
- 注入非法报文 -> 验证错误处理
```

---

## 八、实现路线图

| 阶段 | 内容 | 状态 |
|------|------|------|
| Phase 1 | 基础框架 + 打印功能 | 待实现 |
| Phase 2 | 以太网层解析 | 待规划 |
| Phase 3 | ARP 协议 | 待规划 |
| Phase 4 | IPv4 基础 | 待规划 |
| Phase 5 | ICMP（ping） | 待规划 |

---

## 九、设计原则

1. **简化优先**：当前阶段仅实现基础接口，验证数据流
2. **渐进完善**：协议解析按层级逐步实现
3. **错误透明**：处理错误向上传播，不吞没错误信息
4. **测试驱动**：每阶段添加对应测试用例
