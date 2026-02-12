# Packet模块实现日志

## 日期
2026-02-12

## 概述

本次实现了CoreNet项目的核心基础模块，包括错误类型、通用类型定义、报文描述符（Packet）、环形队列以及模块导出。这些模块是整个网络协议栈的基础，为后续协议实现提供核心数据结构和工具支持。

---

## 一、错误类型模块 (`src/common/error.rs`)

### 1.1 设计目标

提供统一的错误类型定义，用于整个协议栈的错误处理。采用Rust的`Result<T>`类型别名模式，简化错误传递。

### 1.2 核心定义

```rust
/// CoreNet核心错误类型
#[derive(Debug, Clone, PartialEq)]
pub enum CoreError {
    // Buffer相关错误
    BufferOverflow,      // Buffer溢出：写入数据超过容量
    BufferUnderflow,     // Buffer下溢：读取数据超过实际长度
    InvalidLength,       // 无效长度：长度参数不符合要求

    // 解析相关错误
    ParseError(String),  // 解析错误：无法解析协议数据
    InvalidProtocol(String), // 无效协议：不支持或未知的协议类型

    // 队列相关错误
    QueueFull,          // 队列已满：无法插入更多元素
    QueueEmpty,         // 队列为空：无法获取元素

    // Packet相关错误
    InvalidPacket(String),    // 无效报文：报文格式不正确
    UnsupportedProtocol(String), // 不支持的协议：协议尚未实现

    // 状态错误
    InvalidOffset,        // 位置越界：offset超出有效范围

    // 通用错误
    Other(String),       // 其他错误
}
```

### 1.3 实现特性

1. **派生宏实现**
   - `Debug`: 支持格式化输出
   - `Clone`: 允许错误值的克隆
   - `PartialEq`: 支持错误比较

2. **辅助构造方法**
   ```rust
   CoreError::parse_error("解析失败")      // 快速创建解析错误
   CoreError::invalid_protocol("TCP")      // 快速创建无效协议错误
   CoreError::invalid_packet("长度不足")   // 快速创建无效报文错误
   CoreError::unsupported_protocol("SCTP") // 快速创建不支持协议错误
   ```

3. **Display实现**
   - 提供中文错误描述，符合项目的中文文档风格
   - 包含详细错误信息（如预期长度vs实际长度）

4. **std::error::Error实现**
   - 完全兼容Rust标准错误处理
   - 支持`?`操作符自动转换

### 1.4 Result类型别名

```rust
/// Result类型别名：使用CoreError作为错误类型
pub type Result<T> = std::result::Result<T, CoreError>;
```

使用示例：
```rust
fn parse_packet(data: &[u8]) -> Result<Packet> {
    if data.len() < 20 {
        return Err(CoreError::InvalidLength {
            expected: 20,
            actual: data.len(),
        });
    }
    // ... 解析逻辑
    Ok(packet)
}
```

---

## 二、通用类型模块 (`src/common/types.rs`)

### 2.1 设计目标

定义网络协议中使用的通用数据类型，包括MAC地址、IP地址、以太网类型、IP协议号等。所有类型都实现了标准trait以便于使用。

### 2.2 MAC地址类型 (`MacAddr`)

#### 定义

```rust
/// MAC地址（6字节）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MacAddr([u8; 6]);
```

#### 核心方法

| 方法 | 说明 |
|------|------|
| `new(a,b,c,d,e,f)` | 从6个字节创建MAC地址 |
| `from_bytes([u8;6])` | 从字节数组创建 |
| `bytes()` | 返回字节数组（拷贝） |
| `as_bytes()` | 返回字节数组引用 |
| `is_broadcast()` | 判断是否为广播地址 |
| `is_multicast()` | 判断是否为多播地址 |
| `is_local()` | 判断是否为本地管理地址 |

#### 常量定义

```rust
pub const BROADCAST: MacAddr = MacAddr([0xFF, 0xFF, 0xFF, 0xFF, 0xFF]);
pub const ZERO: MacAddr = MacAddr([0x00, 0x00, 0x00, 0x00, 0x00, 0x00]);
```

#### 格式化输出

```rust
// Display格式: XX:XX:XX:XX（大写）
MacAddr::new(0x00, 0x11, 0x22, 0x33, 0x44, 0x55).to_string()
// 输出: "00:11:22:33:44:55"

// LowerHex格式: xx:xx:xx:xx:xx（小写）
format!("{:x}", MacAddr::BROADCAST)
// 输出: "ff:ff:ff:ff:ff:ff:ff"
```

#### 字符串解析

```rust
impl FromStr for MacAddr {
    type Err = String;

    // 支持格式: "00:11:22:33:44:55"
    "00:11:22:33:44:55".parse::<MacAddr>()?;
}
```

### 2.3 IP地址类型 (`IpAddr`)

#### 定义

```rust
/// IP地址枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IpAddr {
    V4([u8; 4]),  // IPv4地址（4字节）
    V6([u8; 16]), // IPv6地址（16字节）
}
```

#### 核心方法

| 方法 | 说明 |
|------|------|
| `v4(a,b,c,d)` | 创建IPv4地址 |
| `v6(a,b,c,d,e,f,g,h)` | 创建IPv6地址（16位段） |
| `from_v4_bytes([u8;4])` | 从IPv4字节数组创建 |
| `from_v6_bytes([u8;16])` | 从IPv6字节数组创建 |
| `version()` | 返回IP版本（V4/V6） |
| `is_v4()` / `is_v6()` | 判断IP版本 |
| `is_loopback()` | 判断是否为回环地址 |
| `is_multicast()` | 判断是否为多播地址 |
| `bytes()` | 获取字节数组 |

#### 回环地址检测

```rust
// IPv4回环: 127.0.0.0/8
IpAddr::v4(127, 0, 0, 1).is_loopback(); // true

// IPv6回环: ::1
IpAddr::v6(0, 0, 0, 0, 0, 1).is_loopback(); // true
```

#### 多播地址检测

```rust
// IPv4多播: 224.0.0.0/4
IpAddr::v4(224, 0, 0, 1).is_multicast(); // true

// IPv6多播: ff00::/8
IpAddr::from_v6_bytes([0xFF, ...]).is_multicast(); // true
```

### 2.4 以太网类型 (`EtherType`)

#### 定义

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u16)]
pub enum EtherType {
    IPv4 = 0x0800,      // IPv4协议
    ARP = 0x0806,        // ARP协议
    WakeOnLan = 0x0842,   // Wake-on-LAN
    RARP = 0x8035,        // RARP协议
    AppleTalk = 0x809B,   // AppleTalk
    Vlan = 0x8100,       // VLAN标签 (802.1Q)
    IPv6 = 0x86DD,        // IPv6协议
    EAPOL = 0x888E,       // EAPOL (802.1X)
    // ...
    Reserved(u16),        // 保留/未知类型
}
```

#### 类型转换

```rust
// 从u16创建
EtherType::from_u16(0x0800) // => EtherType::IPv4
EtherType::from_u16(0xFFFF) // => EtherType::Reserved(0xFFFF)

// 转换为u16
EtherType::IPv4.to_u16() // => 0x0800
```

### 2.5 IP协议号 (`IpProtocol`)

#### 定义

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum IpProtocol {
    ICMP = 1,           // ICMP协议
    IGMP = 2,           // IGMP协议
    IPv4Encap = 4,      // IPv4封装
    TCP = 6,            // TCP协议
    UDP = 17,           // UDP协议
    IPv6Encap = 41,     // IPv6封装
    ICMPv6 = 58,        // ICMPv6协议
    IPv6NoNextHeader = 59, // IPv6无next头
    OSPF = 89,          // OSPF协议
    SCTP = 132,         // SCTP协议
    Reserved(u8),        // 保留/未知协议
}
```

### 2.6 协议层标识 (`Layer`)

#### 定义

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Layer {
    Ethernet,  // 以太网层
    Arp,       // ARP协议
    IPv4,      // IPv4协议
    IPv6,      // IPv6协议
    ICMP,      // ICMP协议
    ICMPv6,    // ICMPv6协议
    TCP,       // TCP协议
    UDP,       // UDP协议
}
```

用于Packet解析状态跟踪，记录报文经过了哪些协议层。

---

## 三、报文描述符模块 (`src/common/packet.rs`)

### 3.1 设计目标

Packet是CoreNet的核心数据结构，用于在协议栈各层之间传递报文数据。它封装了：
1. **原始buffer管理**：数据存储、容量、长度
2. **协议元数据**：各层解析出的地址、端口等信息
3. **解析状态跟踪**：当前offset、已解析的层

### 3.2 结构定义

```rust
pub struct Packet {
    // === Buffer管理 ===
    buffer: Vec<u8>,        // 报文数据缓冲区
    length: usize,          // 报文实际数据长度
    capacity: usize,        // buffer总容量

    // === 元数据 ===
    timestamp: Instant,         // 接收时间戳
    interface: InterfaceId,     // 接收接口ID

    // === 协议信息（逐步填充） ===
    eth_src: Option<MacAddr>,          // 以太网源地址
    eth_dst: Option<MacAddr>,          // 以太网目的地址
    eth_type: Option<EtherType>,      // 以太网类型
    ip_version: Option<IpVersion>,    // IP版本 (4/6)
    ip_src: Option<IpAddr>,          // IP源地址
    ip_dst: Option<IpAddr>,          // IP目的地址
    ip_ttl: Option<u8>,             // TTL/Hop Limit
    protocol: Option<IpProtocol>,      // 传输层协议
    transport_src: Option<u16>,       // 传输层源端口
    transport_dst: Option<u16>,       // 传输层目的端口

    // === 解析状态 ===
    offset: usize,                     // 当前解析位置
    layers: Vec<Layer>,                // 已解析的协议层
}
```

### 3.3 构造方法

| 方法 | 说明 |
|------|------|
| `new(capacity)` | 创建指定容量的空Packet |
| `from_bytes(data)` | 从已有数据创建Packet |
| `from_slice(data)` | 从字节切片创建Packet |

### 3.4 Buffer访问方法

| 方法 | 说明 |
|------|------|
| `as_bytes()` | 获取不可变数据引用 |
| `as_mut_bytes()` | 获取可变数据引用 |
| `slice(start, end)` | 获取指定范围切片 |
| `slice_from(start)` | 获取从start开始的切片 |
| `len()` | 获取数据长度 |
| `capacity()` | 获取buffer容量 |
| `is_empty()` | 判断是否为空 |

### 3.5 数据写入方法

| 方法 | 说明 |
|------|------|
| `extend_from_slice(data)` | 追加字节数组到末尾 |
| `write_u8(value)` | 写入单字节 |
| `write_u16(value)` | 写入u16（大端序） |
| `write_u32(value)` | 写入u32（大端序） |

### 3.6 数据解析方法

| 方法 | 说明 |
|------|------|
| `remaining()` | 获取剩余可读字节数 |
| `has_remaining(len)` | 检查是否有足够字节可读 |
| `peek(len)` | 读取指定字节但不移动offset |
| `read(len)` | 读取指定字节并移动offset |
| `skip(len)` | 跳过指定字节数 |
| `seek(offset)` | 重置offset到指定位置 |
| `read_u8()` | 读取u8并移动offset |
| `read_u16()` | 读取u16（大端序）并移动offset |
| `read_u32()` | 读取u32（大端序）并移动offset |
| `peek_u8()` | 读取u8但不移动offset |
| `peek_u16()` | 读取u16（大端序）但不移动offset |

### 3.7 协议层管理

| 方法 | 说明 |
|------|------|
| `push_layer(layer)` | 添加协议层 |
| `pop_layer()` | 移除最后一个协议层 |
| `current_layer()` | 获取当前（最后）协议层 |
| `layers()` | 获取所有协议层 |
| `has_layer(layer)` | 检查是否包含指定层 |

### 3.8 封装操作

| 方法 | 说明 |
|------|------|
| `reserve_header(len)` | 在头部预留空间（数据后移） |
| `insert_space(len)` | 在当前位置插入空间 |
| `write_header(data)` | 写入协议头（先插入再写入） |

### 3.9 状态管理

| 方法 | 说明 |
|------|------|
| `clear()` | 清空数据和元数据，保留buffer |
| `reset_offset()` | 重置offset到0 |
| `get_offset()` | 获取当前offset值 |
| `set_timestamp(t)` | 设置时间戳 |
| `set_interface(id)` | 设置接口ID |
| `metadata()` | 获取元数据快照 |

### 3.10 元数据快照

```rust
#[derive(Debug, Clone)]
pub struct PacketMetadata {
    pub eth_src: Option<MacAddr>,
    pub eth_dst: Option<MacAddr>,
    pub eth_type: Option<EtherType>,
    pub ip_version: Option<IpVersion>,
    pub ip_src: Option<IpAddr>,
    pub ip_dst: Option<IpAddr>,
    pub ip_ttl: Option<u8>,
    pub protocol: Option<IpProtocol>,
    pub transport_src: Option<u16>,
    pub transport_dst: Option<u16>,
}
```

用于获取Packet的协议元数据，不包含buffer。

### 3.11 Clone实现

实现深度克隆，支持独立的Packet实例。

---

## 四、环形队列模块 (`src/common/queue.rs`)

队列模块已完全按照 [`../detail/queue.md`](../detail/queue.md) 设计文档实现。

---

## 五、模块导出 (`src/common/mod.rs`)

### 5.1 模块声明

```rust
pub mod error;   // 错误类型
pub mod types;   // 通用类型
pub mod packet;  // 报文描述符
pub mod queue;   // 环形队列
```

### 5.2 便捷导出

```rust
// 错误类型
pub use error::{CoreError, Result};

// 通用类型
pub use types::{
    MacAddr, IpAddr, IpVersion,
    EtherType, IpProtocol, Layer,
};

// Packet相关
pub use packet::{
    Packet, PacketMetadata, InterfaceId,
};

// 队列相关
pub use queue::{
    RingQueue, SpscQueue, SafeQueue,
    QueueError, WaitStrategy, QueueConfig,

    // 队列常量
    DEFAULT_QUEUE_CAPACITY,
    MIN_QUEUE_CAPACITY,
    MAX_QUEUE_CAPACITY,
    DEFAULT_SPIN_COUNT,
    DEFAULT_TIMEOUT_MS,
};
```

---

## 六、库入口 (`src/lib.rs`)

### 6.1 重新导出常用类型

```rust
pub mod common;

pub use common::{
    // 错误类型
    CoreError, Result,

    // 网络类型
    MacAddr, IpAddr, IpVersion,
    EtherType, IpProtocol, Layer,

    // Packet相关
    Packet, PacketMetadata, InterfaceId,

    // 队列相关
    RingQueue, SpscQueue, SafeQueue,
    QueueError, WaitStrategy, QueueConfig,

    // 队列常量
    DEFAULT_QUEUE_CAPACITY,
    MIN_QUEUE_CAPACITY,
    MAX_QUEUE_CAPACITY,
    DEFAULT_SPIN_COUNT,
    DEFAULT_TIMEOUT_MS,
};
```

---

## 七、验证编译

```bash
# 检查编译
cargo check

# 构建项目
cargo build

# 运行测试
cargo test

# 格式化代码
cargo fmt

# 静态检查
cargo clippy
```

---

## 八、后续工作

当前完成的是**阶段一：基础框架**的核心部分。后续任务：

### 8.1 协议实现（阶段二）
- 以太网层解析/封装
- ARP协议
- IPv4基础
- ICMP协议（ping功能）

**目标**: 能够ping通网关

### 8.2 传输层（阶段三）
- UDP协议
- TCP基础实现

**目标**: 能够建立TCP连接并传输数据

### 8.3 IPv6支持（阶段四）
- IPv6基础
- ICMPv6
- 邻居发现

### 8.4 应用接口（阶段五）
- Socket API封装
- 测试工具

---

## 九、设计亮点

1. **零外部依赖**: 仅使用Rust标准库
2. **类型安全**: 充分利用Rust类型系统
3. **零拷贝**: 使用切片引用避免数据拷贝
4. **无锁队列**: 原子操作实现SPSC队列
5. **中文文档**: 符合学习型项目定位
6. **完整测试**: 每个模块都有单元测试覆盖
