# ICMP (Internet Control Message Protocol) 详细设计文档

## 1. 协议概述

### 1.1 背景与历史

**协议全称和定义：**
- 协议全称：Internet Control Message Protocol（互联网控制消息协议）
- 在 TCP/IP 协议栈中的层级位置：网络层（与 IP 同层）
- 核心功能概述：用于在 IP 主机、路由器之间传递控制消息，包括错误报告、诊断信息和网络状态查询

**为什么需要该协议？**

IP 协议本身是一种"尽力而为"（best-effort）的交付服务，不保证可靠性。当数据报无法到达目的地、生存时间过期、路由重定向等情况发生时，需要一种机制来通知发送方。ICMP 协议正是为了解决这个问题而设计的：

- **错误报告**：通知发送方数据报传输过程中出现的问题
- **诊断工具**：支持 ping（连通性测试）和 traceroute（路由跟踪）等网络诊断工具
- **网络管理**：提供网络状态信息和重定向功能

**历史背景：**
- **RFC 792**：1981年9月发布，由 J. Postel（ISI）编写
- **协议发展**：随着 IP 协议演进，ICMP 也进行了扩展
- **相关补充 RFC**：
  - RFC 950（子网掩码请求/应答）
  - RFC 1122（主机要求，对 ICMP 的修订）
  - RFC 1191（路径 MTU 发现）
  - RFC 1256（ICMP 路由器发现）
  - RFC 2461（邻居发现，ICMPv6 基础）
  - RFC 4443（ICMPv6 规范）

### 1.2 设计原理

ICMP 的核心设计思想是**轻量级的控制和反馈机制**。它不负责数据传输，而是作为 IP 协议的补充，提供网络层的诊断和错误报告功能。

```
                        IP 协议栈层级
    +------------------------------------------------+
    |              应用层 (HTTP/FTP/...)              |
    +------------------------------------------------+
                         ↓↑
    +------------------------------------------------+
    |         传输层 (TCP/UDP)        |
    +------------------------------------------------+
                         ↓↑
    +---------------+----------------+----------------+
    |   ICMP        |      IP        |    其他协议    |
    +---------------+----------------+----------------+
                         ↓↑
    +------------------------------------------------+
    |         数据链路层 (Ethernet)                   |
    +------------------------------------------------+
```

**ICMP 消息处理流程：**

```
    发送端                    中间路由器                   接收端
       |                          |                         |
       |---- IP 数据报 ---------->|------- IP 数据报 ------->|
       |                          |                         |
       |                          |      [问题发生]          |
       |                          |      例如：TTL=0        |
       |                          |                         |
       |<---- ICMP 错误消息-------|                         |
       |    (Time Exceeded)       |                         |
       |                          |                         |
    [处理错误]                  [继续转发]                 [正常处理]
```

**关键特点：**

1. **错误报告不产生额外错误**：ICMP 错误消息本身不会触发另一个 ICMP 错误消息，避免无限循环
2. **带状态信息**：ICMP 消息携带导致问题的原始 IP 数据报的部分内容，便于诊断
3. **核心协议支持**：ping、traceroute、MTU 发现等关键网络工具依赖 ICMP
4. **类型驱动设计**：通过类型(Type)和代码(Code)字段区分不同消息，便于扩展
5. **无连接通信**：ICMP 消息独立传输，不建立连接，不保证可靠交付

---

## 2. 报文格式

### 2.1 报文结构

ICMP 报文封装在 IP 数据报中，IP 头部的协议字段值为 1。ICMP 报文格式如下：

```
 0                   1                   2                   3
 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|     Type      |     Code     |          Checksum             |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|                       (取决于消息类型)                        |
+                                                               +
|                            ...                                |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|           Internet Header + 64 bits of Original Data          |
+                 (仅在错误消息中包含)                          |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
```

### 2.2 通用字段说明

| 字段 | 大小 | 说明 | 常见值 |
|------|------|------|--------|
| Type | 1 字节 | ICMP 消息类型 | 0-18 (见下表) |
| Code | 1 字节 | 类型子代码，提供详细信息 | 0-15 |
| Checksum | 2 字节 | 整个 ICMP 报文的校验和 | 计算值 |
| Rest | 4+ 字节 | 取决于消息类型 | - |

**最小报文长度：** 8 字节（头部）
**最大报文长度：** 受限于 IP 数据报的 MTU（通常 576 字节）

### 2.3 ICMP 消息类型

| Type | 名称 | 用途 |
|------|------|------|
| 0 | Echo Reply | Ping 回应（Echo Request 的响应） |
| 3 | Destination Unreachable | 目标不可达 |
| 4 | Source Quench | 源抑制（已废弃） |
| 5 | Redirect | 路由重定向 |
| 8 | Echo Request | Ping 请求 |
| 9 | Router Advertisement | 路由器通告 |
| 10 | Router Solicitation | 路由器请求 |
| 11 | Time Exceeded | 超时 |
| 12 | Parameter Problem | 参数问题 |
| 13 | Timestamp Request | 时间戳请求 |
| 14 | Timestamp Reply | 时间戳应答 |
| 15 | Information Request | 信息请求（已废弃） |
| 16 | Information Reply | 信息应答（已废弃） |
| 17 | Address Mask Request | 地址掩码请求 |
| 18 | Address Mask Reply | 地址掩码应答 |

### 2.4 封装格式

**ICMP 封装在 IP 中：**

```
+------------------+
|    IP Header     |  Protocol = 1
+------------------+
|   ICMP Header    |  Type, Code, Checksum
+------------------+
|  ICMP Payload    |  消息特定数据
+------------------+
```

**错误消息额外包含：**

```
+------------------+
|    IP Header     |  Protocol = 1
+------------------+
|   ICMP Header    |  Type=Error, Code
+------------------+
|     Unused       |  4 字节（部分消息）
+------------------+
| Original IP Hdr  |  原始数据报的 IP 头
+------------------+
| Original Data    |  原始数据报的前 64 位
+------------------+
```

---

## 3. 状态机设计

### 3.0 状态变量

ICMP 本质上是一个**无状态协议**，不维护会话状态。但为了实现某些功能（如 Echo 请求匹配），需要临时存储信息：

| 变量名称 | 类型 | 用途 | 初始值 |
|---------|------|------|--------|
| echo_identifier | u16 | Echo 请求的标识符，用于匹配请求和响应 | 随机生成 |
| echo_sequence | u16 | Echo 请求的序列号 | 0 或随机 |
| pending_echoes | HashMap | 未完成的 Echo 请求 (identifier+sequence -> timestamp) | 空 |

### 3.1 状态定义

ICMP 没有传统意义上的连接状态机。但对于 Echo（ping）功能，可以定义一个简化的请求-响应模型：

```
                       发送 Echo Request
                              |
                              v
    +------------------------------------------------+
    |               Waiting for Echo Reply          |
    |          (定时器运行，等待响应或超时)           |
    +------------------------------------------------+
         |                                    |
    [收到 Echo Reply]                   [超时]
         |                                    |
         v                                    v
    [计算 RTT，完成]                    [请求失败/重试]
```

### 3.2 Echo 请求-响应流程

#### 3.2.1 发送 Echo Request

**描述：** 发起一个 Echo 请求（ping）

**进入条件：** 应用层或用户触发 ping 操作

**行为：**
1. 生成 identifier 和 sequence number
2. 记录发送时间戳
3. 构建 Echo Request ICMP 消息
4. 通过 IP 层发送

**状态变量更新：**
- `pending_echoes[(identifier, sequence)] = 当前时间戳`

#### 3.2.2 等待 Echo Reply

**描述：** 等待 Echo Request 的响应

**超时设置：** 默认 1-5 秒（可配置）

**转换条件：**
- 收到匹配的 Echo Reply → 完成状态
- 超时 → 失败状态（可选择重试）

#### 3.2.3 收到 Echo Reply

**描述：** 收到 Echo Reply 消息

**匹配条件：** identifier 和 sequence 与 pending_echoes 中的条目匹配

**处理步骤：**
1. 从 `pending_echoes` 查找对应的请求时间戳
2. 计算往返时间 (RTT) = 当前时间 - 请求时间戳
3. 从 `pending_echoes` 删除该条目
4. 向应用层报告结果

---

## 4. 报文处理逻辑

### 4.0 定时器

ICMP 使用的定时器（主要用于 Echo 请求）：

| 定时器名称 | 启动条件 | 超时时间 | 超时动作 |
|-----------|---------|---------|---------|
| echo_timeout | 发送 Echo Request 后 | 默认 1-5 秒 | 标记请求失败，可选重试或移除 pending 条目 |

### 4.1 接收处理总流程

```
                    收到 IP 数据报
                         |
                         v
                 [Protocol == 1?] ---否---> 丢弃
                         |
                        是
                         |
                         v
                 [校验和正确?] ---否----> 丢弃
                         |
                        是
                         |
                         v
                 +----------------+
                 |  提取 Type/Code|
                 +----------------+
                         |
         +---------------+---------------+
         |               |               |
         v               v               v
   [Error Msg]     [Echo Request]  [Echo Reply]
         |               |               |
         v               v               v
    [4.2 节]        [4.3 节]         [4.4 节]
         |               |               |
         v               v               v
    [可选应答]      [发送 Reply]    [应用通知]
```

### 4.2 Destination Unreachable (Type 3)

**用途：** 通知发送方数据报无法到达最终目的地

**Code 定义：**

| Code | 名称 | 说明 |
|------|------|------|
| 0 | Network Unreachable | 网络不可达（路由失败） |
| 1 | Host Unreachable | 主机不可达（ARP 失败等） |
| 2 | Protocol Unreachable | 协议不可达（上层协议未实现） |
| 3 | Port Unreachable | 端口不可达（端口未监听） |
| 4 | Fragmentation Needed | 需要分片但 DF 标志设置 |
| 5 | Source Route Failed | 源路由失败 |
| 6 | Destination Network Unknown | 目的网络未知 |
| 7 | Destination Host Unknown | 目的主机未知 |
| 8 | Source Host Isolated | 源主机被隔离 |
| 9 | Destination Network Administratively Prohibited | 目的网络被管理禁止 |
| 10 | Destination Host Administratively Prohibited | 目的主机被管理禁止 |
| 11 | Network Unreachable for TOS | 对指定服务类型网络不可达 |
| 12 | Host Unreachable for TOS | 对指定服务类型主机不可达 |
| 13 | Communication Administratively Prohibited | 通信被管理禁止 |
| 14 | Host Precedence Violation | 主机优先级违规 |
| 15 | Precedence Cutoff in Effect | 优先级截断生效 |

**处理流程：**

1. **提取信息：**
   - Type = 3, Code → 具体的不可达原因
   - Next-Hop MTU (Code=4 时) → 路径 MTU 发现

2. **处理步骤：**
   - 解析原始 IP 数据报头部（包含在 ICMP 负载中）
   - 提取源地址，确定应该通知谁
   - 根据 Code 确定不可达的具体原因

3. **资源更新：**
   - 无需更新 ICMP 相关表项（无状态）

4. **响应动作：**
   - 不发送 ICMP 响应（避免循环）
   - 可选：通知上层协议或应用层

### 4.3 Echo Request (Type 8)

**用途：** ping 请求，用于测试目的地的可达性和往返时间

**报文格式：**

```
 0                   1                   2                   3
 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|     Type=8    |     Code=0   |          Checksum             |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|           Identifier          |        Sequence Number       |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|                             Data                              |
+                                                               +
|                            ...                                |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
```

**处理流程：**

1. **提取信息：**
   - Identifier → 标识符（通常由进程 ID 生成）
   - Sequence Number → 序列号
   - Data → 可选的负载数据（时间戳、填充模式等）

2. **处理步骤：**
   - 验证校验和
   - 提取 Identifier 和 Sequence Number

3. **资源更新：**
   - 无需更新表项（无状态）

4. **响应动作：**
   - **必须**发送 Echo Reply (Type 0, Code 0)
   - Echo Reply 包含相同的 Identifier、Sequence Number 和 Data

### 4.4 Echo Reply (Type 0)

**用途：** ping 响应，确认目的地可达并返回负载数据

**报文格式：** 与 Echo Request 相同，Type = 0

**处理流程：**

1. **提取信息：**
   - Identifier → 标识符
   - Sequence Number → 序列号
   - Data → 原始请求数据（可能包含时间戳）

2. **处理步骤：**
   - 在 `pending_echoes` 中查找匹配的请求
   - 如果找到，计算 RTT
   - 如果未找到，可能是重复或延迟的响应

3. **资源更新：**
   - 表项：`pending_echoes[(identifier, sequence)]` - **删除**
   - 状态变量：记录 RTT 统计信息

4. **响应动作：**
   - 不发送 ICMP 响应
   - 向应用层报告 ping 结果

### 4.5 Time Exceeded (Type 11)

**用途：** 通知发送方数据报的 TTL（生存时间）已过期

**Code 定义：**

| Code | 名称 | 说明 |
|------|------|------|
| 0 | Time to Live Exceeded | TTL 在传输中过期 |
| 1 | Fragment Reassembly Time Exceeded | 分片重组超时 |

**处理流程：**

1. **提取信息：**
   - Type = 11, Code → TTL 过期或分片重组超时
   - 原始 IP 头和数据

2. **处理步骤：**
   - 解析原始数据报，确定源地址
   - 对于 Code=0，这是 traceroute 工作的基础

3. **资源更新：**
   - 无需更新表项

4. **响应动作：**
   - 不发送 ICMP 响应
   - 可选：通知上层协议

### 4.6 Redirect (Type 5)

**用途：** 通知主机有更好的路由路径

**Code 定义：**

| Code | 名称 | 说明 |
|------|------|------|
| 0 | Redirect for Network | 重定向到网络 |
| 1 | Redirect for Host | 重定向到主机 |
| 2 | Redirect for TOS and Network | 基于服务类型的网络重定向 |
| 3 | Redirect for TOS and Host | 基于服务类型的主机重定向 |

**报文格式：**

```
 0                   1                   2                   3
 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|     Type=5    |     Code     |          Checksum             |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|                 Gateway Address (Internet Address)           |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|      Internet Header + 64 bits of Original Data              |
+                                                               +
|                            ...                                |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
```

**处理流程：**

1. **提取信息：**
   - Code → 重定向类型
   - Gateway Address → 建议的新网关地址

2. **处理步骤：**
   - 验证 Redirect 消息来自当前第一跳路由器
   - 更新路由表，添加/更新到目标网络/主机的新路由

3. **资源更新：**
   - 路由表：添加/更新路由条目

4. **响应动作：**
   - 不发送 ICMP 响应
   - 更新内部路由缓存

### 4.7 Parameter Problem (Type 12)

**用途：** 指示 IP 头部存在参数错误

**Code 定义：**

| Code | 名称 | 说明 |
|------|------|------|
| 0 | Pointer indicates the error | 指针指向错误位置 |
| 1 | Missing a Required Option | 缺少必需选项 |
| 2 | Bad Length | 长度错误 |

**处理流程：**

1. **提取信息：**
   - Pointer → 指向 IP 头部中错误字节的偏移量

2. **处理步骤：**
   - 解析原始 IP 头
   - 根据 Pointer 定位错误字段

3. **资源更新：**
   - 无需更新表项

4. **响应动作：**
   - 不发送 ICMP 响应
   - 记录错误信息

---

## 5. 核心数据结构

### 5.0 表项/缓存

ICMP 本身是无状态的，不需要持久化的表项。但为了支持 Echo 功能，需要临时存储：

| 资源名称 | 用途 | 最大容量 | 淘汰策略 |
|---------|------|---------|---------|
| pending_echoes | 匹配 Echo Request/Reply | 由配置决定 | 超时自动删除 |

### 5.1 报文结构

```rust
/// ICMP 报文头部（通用格式）
#[repr(C, packed)]
pub struct IcmpHeader {
    /// ICMP 消息类型
    pub type_: u8,
    /// 类型子代码
    pub code: u8,
    /// 校验和
    pub checksum: u16,
}

/// Echo Request/Reply 报文
#[repr(C, packed)]
pub struct IcmpEcho {
    /// 类型 (0=Reply, 8=Request)
    pub type_: u8,
    /// 代码 (始终为 0)
    pub code: u8,
    /// 校验和
    pub checksum: u16,
    /// 标识符（用于匹配请求和响应）
    pub identifier: u16,
    /// 序列号
    pub sequence: u16,
}

/// Destination Unreachable 报文
#[repr(C, packed)]
pub struct IcmpDestUnreachable {
    /// 类型 (3)
    pub type_: u8,
    /// 不可达代码
    pub code: u8,
    /// 校验和
    pub checksum: u16,
    /// 未使用（填充为 0）
    pub unused: u32,
    /// 原始 IP 头部 + 8 字节数据紧随其后
}

/// Time Exceeded 报文
#[repr(C, packed)]
pub struct IcmpTimeExceeded {
    /// 类型 (11)
    pub type_: u8,
    /// 超时代码 (0=TTL, 1=分片重组)
    pub code: u8,
    /// 校验和
    pub checksum: u16,
    /// 未使用（填充为 0）
    pub unused: u32,
    /// 原始 IP 头部 + 8 字节数据紧随其后
}

/// Redirect 报文
#[repr(C, packed)]
pub struct IcmpRedirect {
    /// 类型 (5)
    pub type_: u8,
    /// 重定向代码
    pub code: u8,
    /// 校验和
    pub checksum: u16,
    /// 建议的网关 IP 地址
    pub gateway_address: u32,
}

/// Parameter Problem 报文
#[repr(C, packed)]
pub struct IcmpParameterProblem {
    /// 类型 (12)
    pub type_: u8,
    /// 参数问题代码
    pub code: u8,
    /// 校验和
    pub checksum: u16,
    /// 指向错误字节的指针
    pub pointer: u8,
    /// 未使用（填充为 0）
    pub unused: [u8; 3],
}
```

### 5.2 枚举类型

```rust
/// ICMP 消息类型
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IcmpType {
    /// Echo Reply
    EchoReply = 0,
    /// Destination Unreachable
    DestinationUnreachable = 3,
    /// Source Quench（已废弃）
    SourceQuench = 4,
    /// Redirect
    Redirect = 5,
    /// Echo Request
    EchoRequest = 8,
    /// Router Advertisement
    RouterAdvertisement = 9,
    /// Router Solicitation
    RouterSolicitation = 10,
    /// Time Exceeded
    TimeExceeded = 11,
    /// Parameter Problem
    ParameterProblem = 12,
    /// Timestamp Request
    TimestampRequest = 13,
    /// Timestamp Reply
    TimestampReply = 14,
    /// Information Request（已废弃）
    InformationRequest = 15,
    /// Information Reply（已废弃）
    InformationReply = 16,
    /// Address Mask Request
    AddressMaskRequest = 17,
    /// Address Mask Reply
    AddressMaskReply = 18,
}

impl IcmpType {
    /// 从 u8 解析 ICMP 类型
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0 => Some(IcmpType::EchoReply),
            3 => Some(IcmpType::DestinationUnreachable),
            4 => Some(IcmpType::SourceQuench),
            5 => Some(IcmpType::Redirect),
            8 => Some(IcmpType::EchoRequest),
            9 => Some(IcmpType::RouterAdvertisement),
            10 => Some(IcmpType::RouterSolicitation),
            11 => Some(IcmpType::TimeExceeded),
            12 => Some(IcmpType::ParameterProblem),
            13 => Some(IcmpType::TimestampRequest),
            14 => Some(IcmpType::TimestampReply),
            15 => Some(IcmpType::InformationRequest),
            16 => Some(IcmpType::InformationReply),
            17 => Some(IcmpType::AddressMaskRequest),
            18 => Some(IcmpType::AddressMaskReply),
            _ => None,
        }
    }
}

/// Destination Unreachable 代码
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DestUnreachableCode {
    /// 网络不可达
    NetworkUnreachable = 0,
    /// 主机不可达
    HostUnreachable = 1,
    /// 协议不可达
    ProtocolUnreachable = 2,
    /// 端口不可达
    PortUnreachable = 3,
    /// 需要分片但 DF 设置
    FragmentationNeeded = 4,
    /// 源路由失败
    SourceRouteFailed = 5,
    /// 目的网络未知
    DestinationNetworkUnknown = 6,
    /// 目的主机未知
    DestinationHostUnknown = 7,
    /// 源主机被隔离
    SourceHostIsolated = 8,
    /// 目的网络被管理禁止
    DestinationNetworkProhibited = 9,
    /// 目的主机被管理禁止
    DestinationHostProhibited = 10,
    /// 对指定 TOS 网络不可达
    NetworkUnreachableForTos = 11,
    /// 对指定 TOS 主机不可达
    HostUnreachableForTos = 12,
    /// 通信被管理禁止
    CommunicationProhibited = 13,
    /// 主机优先级违规
    HostPrecedenceViolation = 14,
    /// 优先级截断生效
    PrecedenceCutoff = 15,
}

/// Time Exceeded 代码
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimeExceededCode {
    /// TTL 过期
    TtlExpired = 0,
    /// 分片重组超时
    FragmentReassemblyTimeout = 1,
}

/// Redirect 代码
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RedirectCode {
    /// 重定向到网络
    RedirectNetwork = 0,
    /// 重定向到主机
    RedirectHost = 1,
    /// 基于服务类型的网络重定向
    RedirectTosNetwork = 2,
    /// 基于服务类型的主机重定向
    RedirectTosHost = 3,
}
```

### 5.3 待处理 Echo 请求管理

```rust
use std::collections::HashMap;
use std::time::Instant;

/// 待处理的 Echo 请求条目
#[derive(Debug, Clone)]
pub struct PendingEcho {
    /// 标识符
    pub identifier: u16,
    /// 序列号
    pub sequence: u16,
    /// 发送时间戳
    pub sent_at: Instant,
    /// 目标地址
    pub destination: Ipv4Addr,
}

/// Echo 请求管理器
pub struct EchoManager {
    /// 待处理的 Echo 请求
    pending: HashMap<(u16, u16), PendingEcho>,
    /// 默认超时时间
    default_timeout: Duration,
}

impl EchoManager {
    /// 创建新的 Echo 管理器
    pub fn new(default_timeout: Duration) -> Self {
        Self {
            pending: HashMap::new(),
            default_timeout,
        }
    }

    /// 添加待处理的 Echo 请求
    pub fn add_pending(&mut self, echo: PendingEcho) {
        let key = (echo.identifier, echo.sequence);
        self.pending.insert(key, echo);
    }

    /// 查找并移除待处理的 Echo 请求
    pub fn remove_pending(&mut self, identifier: u16, sequence: u16) -> Option<PendingEcho> {
        let key = (identifier, sequence);
        self.pending.remove(&key)
    }

    /// 清理超时的请求
    pub fn cleanup_timeouts(&mut self) {
        let now = Instant::now();
        self.pending.retain(|_, echo| {
            now.duration_since(echo.sent_at) < self.default_timeout
        });
    }
}
```

---

## 6. 与其他模块的交互

### 6.1 模块依赖关系图

```
                        +---------------------------+
                        |       应用层/API          |
                        |    (ping/traceroute)      |
                        +---------------------------+
                                 |        ^
                                 v        |
    +----------------+    +---------------------------+    +------------------+
    | Engine/        |    |       ICMP 模块           |    |  IP 模块 (IPv4)  |
    | Processor      |<-->|   (protocols/icmp/)      |<-->| (protocols/ip/)  |
    +----------------+    +---------------------------+    +------------------+
         |                        ^        |                      ^
         v                        |        v                      |
    +----------------+    +---------------------------+    +------------------+
    |  Scheduler     |    |      Interface 模块       |    |  Ethernet 模块   |
    +----------------+    |   (interface/)            |    |(protocols/eth)   |
                          +---------------------------+    +------------------+
                                    ^        |
                                    |        v
                          +---------------------------+
                          |      Common 模块          |
                          | (packet/error/addr/queue)|
                          +---------------------------+
```

### 6.2 与 Common 模块的交互

**依赖的 Common 子模块：**

| 子模块 | 使用内容 | 用途 |
|-------|---------|------|
| `common/packet.rs` | `Packet` 结构体 | ICMP 报文的封装和解析 |
| `common/error.rs` | `CoreError` | 错误处理和返回 |
| `common/addr.rs` | `Ipv4Addr`, `MacAddr` | IP 地址和 MAC 地址类型 |
| `common/queue.rs` | `RxQueue`, `TxQueue` | 数据包的收发队列 |

**具体使用方式：**

```rust
// 从 common/packet.rs
use crate::common::packet::Packet;
// 用途：接收 IP 数据报，构造 ICMP 响应报文

// 从 common/error.rs
use crate::common::error::CoreError;
// 用途：返回解析错误、校验和错误等

// 从 common/addr.rs
use crate::common::addr::Ipv4Addr;
// 用途：源/目的 IP 地址的解析和设置
```

### 6.3 与 Interface 模块的交互

**依赖的 Interface 子模块：**

| 子模块 | 使用内容 | 用途 |
|-------|---------|------|
| `interface/global.rs` | `INTERFACE_GLOBAL` | 获取接口信息 |
| `interface/iface.rs` | `Interface` 结构体 | 接口状态和配置 |
| `interface/types.rs` | 接口相关类型 | MTU、地址等 |

**具体交互：**

```rust
// 获取接收接口的 IP 地址（用于 ICMP 响应的源地址）
use crate::interface::global::INTERFACE_GLOBAL;

let iface = INTERFACE_GLOBAL.lock()
    .get_interface_by_name(received_iface_name)?;

let source_addr = iface.ipv4_addr;  // 用作 ICMP 响应的源地址
let mtu = iface.mtu;                // 检查 ICMP 报文长度
```

**交互场景：**
- **发送 ICMP 响应时**：查询接收接口的 IP 地址作为源地址
- **处理 Redirect 时**：可能更新接口的路由表
- **MTU 检查**：确保 ICMP 报文不超过接口 MTU

### 6.4 与 IP 模块的交互

**依赖内容：**

| IP 模块组件 | 用途 |
|-----------|------|
| IP 头部解析 | 提取源/目的地址、协议字段 |
| IP 报文封装 | 将 ICMP 报文封装为 IP 数据报 |
| 路由查询 | 确定 ICMP 响应的发送接口 |

**具体交互：**

```rust
// IP 层接收处理 (在 IP 模块中)
// 当 Protocol = 1 时，分发给 ICMP 模块
match ip_header.protocol {
    1 => {
        // ICMP 协议
        let icmp_packet = parse_icmp(&packet.payload())?;
        icmp_module.handle_icmp(icmp_packet, &ip_header)?;
    }
    // ... 其他协议
}

// ICMP 模块发送 (通过 IP 模块)
use crate::protocols::ip::Ipv4Header;

let ip_header = Ipv4Header {
    source_addr: my_ip,           // 接收接口的 IP
    dest_addr: original_source,   // 原始数据报的源地址
    protocol: 1,                  // ICMP
    ttl: 64,
    // ...
};
ip_module.send(ip_header, icmp_packet)?;
```

**接收流程：**
1. IP 模块解析 IP 头部，发现 Protocol = 1
2. IP 模块调用 ICMP 模块的 `handle_icmp()` 函数
3. 传递 IP 头部和 ICMP 负载

**发送流程：**
1. ICMP 模块构造 ICMP 报文
2. 调用 IP 模块的封装函数
3. IP 模块添加 IP 头部，查找路由，发送到下层

### 6.5 与 Ethernet 模块的交互

**间接交互（通过 IP 层）：**

ICMP 不直接与 Ethernet 模块交互，而是通过 IP 层间接访问：

```rust
// ICMP 报文最终封装为 Ethernet 帧
// ICMP -> IP -> Ethernet -> Queue
```

**需要 Ethernet 模块提供的：**
- **目的 MAC 地址解析**（通过 ARP）：发送 ICMP 响应时需要知道目标的 MAC 地址
- **帧封装**：IP 数据报（包含 ICMP）需要封装为 Ethernet 帧

### 6.6 与 Engine/Processor 模块的交互

**依赖的 Engine 组件：**

| 组件 | 用途 |
|------|------|
| `engine/processor.rs` | 协议分发器 |
| `scheduler/scheduler.rs` | 数据包调度 |

**交互方式：**

```rust
// 在 Processor 中注册 ICMP 处理
// src/engine/processor.rs

impl Processor {
    pub fn process_ipv4(&mut self, packet: &mut Packet) -> Result<(), ProcessError> {
        // ... 解析 IP 头部

        match ip_header.protocol {
            1 => {
                // 分发到 ICMP 模块
                self.icmp_handler.handle(packet, &ip_header)?;
            }
            6 => { /* TCP */ }
            17 => { /* UDP */ }
            // ...
        }
    }
}
```

### 6.7 与 Scheduler 模块的交互

**交互场景：**
- ICMP 报文通过 Scheduler 从 RxQ 获取
- ICMP 响应通过 Scheduler 发送到 TxQ

```rust
// ICMP 模块不需要直接访问 Scheduler
// 由 Processor 统一处理调度逻辑

// Processor 从 Scheduler 获取数据包
let packet = self.scheduler.recv_packet(iface_name)?;

// 处理后（可能生成 ICMP 响应）
let response = self.icmp_handler.build_echo_reply(...)?;

// 发送到 Scheduler
self.scheduler.send_packet(response, iface_name)?;
```

### 6.8 与 ARP 模块的交互

**间接交互：**

ICMP 发送响应时，需要知道目标的 MAC 地址。这通常由 IP 层通过 ARP 模块解析：

```
ICMP 模块 -> "发送到 IP 地址 X.X.X.X"
     |
     v
IP 模块 -> "需要 MAC 地址，查询 ARP"
     |
     v
ARP 模块 -> "返回 MAC 地址 YY:YY:YY:YY:YY:YY"
     |
     v
Ethernet 模块 -> "封装帧，发送"
```

### 6.9 应用层交互（未来扩展）

**计划中的 API：**

```rust
// 未来可能提供的应用层接口
impl IcmpModule {
    /// 发送 Echo Request (ping)
    pub fn ping(&mut self, dest: Ipv4Addr, count: u32) -> Result<Vec<PingResult>, Error> {
        // ...
    }

    /// 注册 Echo Reply 回调
    pub fn register_echo_callback<F>(&mut self, callback: F)
    where
        F: Fn(Ipv4Addr, Duration, u16) + 'static
    {
        // ...
    }
}

// 使用示例
icmp.ping("192.168.1.1".parse()?, 4)?;
```

### 6.10 模块初始化顺序

```
1. Common 模块 (packet, error, addr, queue)
   ↓
2. Interface 模块 (iface, global)
   ↓
3. Ethernet 模块
   ↓
4. ARP 模块
   ↓
5. IP 模块 (依赖 Ethernet, ARP)
   ↓
6. ICMP 模块 (依赖 IP, Interface, Common)
   ↓
7. Processor/Engine (依赖所有协议模块)
   ↓
8. Scheduler (依赖 Processor)
```

### 6.11 数据流示例

**发送 Echo Request：**

```
应用层
  │
  │ ping("192.168.1.1")
  ↓
ICMP 模块
  │
  │ 构建 Echo Request (Type=8)
  │ 调用 IP 模块封装
  ↓
IP 模块
  │
  │ 添加 IP 头部 (Protocol=1)
  │ 查询路由 (选择出口接口)
  │ 调用 ARP 解析 MAC
  ↓
Ethernet 模块
  │
  │ 封装为 Ethernet 帧
  │
  ↓
Scheduler -> TxQ -> 输出
```

**接收 Echo Request，发送 Echo Reply：**

```
Scheduler -> RxQ
  │
  ↓
Processor 解析 Ethernet -> IP (Protocol=1)
  │
  ↓
ICMP 模块处理 Echo Request
  │
  │ 查询 Interface 获取源 IP
  │ 构建 Echo Reply (Type=0)
  │ 调用 IP 模块封装
  ↓
IP 模块
  │
  │ 添加 IP 头部 (Protocol=1)
  │ 调用 ARP 解析 MAC
  ↓
Ethernet 模块
  │
  │ 封装为 Ethernet 帧
  │
  ↓
Scheduler -> TxQ -> 输出
```

---

## 7. 安全考虑

### 7.1 ICMP 攻击

**攻击方式：**
- **Ping Flood（Smurf 攻击）**：向广播地址发送 Echo Request，导致大量主机响应，淹没目标网络
- **ICMP Flood**：大量发送任意类型的 ICMP 消息，消耗目标资源
- **Ping of Death**：发送超大或格式错误的 ICMP 包，导致系统崩溃（历史漏洞）
- **ICMP Redirect 伪造**：伪造 Redirect 消息，劫持流量

**攻击影响：**
- 网络拥塞
- 拒绝服务
- 流量劫持

**防御措施：**
- **速率限制**：限制 ICMP 消息的发送和接收速率
- **严格验证**：验证 ICMP Error 消息中的原始 IP 头部
- **禁用特定消息**：在生产环境中禁用 Redirect 和 Router Advertisement
- **广播 Ping 限制**：不响应广播地址的 Echo Request
- **防火墙规则**：选择性过滤 ICMP 消息类型

### 7.2 实现建议

1. **不响应所有消息**：只响应必要的 ICMP 消息类型，如 Echo Request
2. **验证源地址**：确保 ICMP Error 消息来自合法的下一跳路由器
3. **限制频率**：对 ICMP 响应实施速率限制
4. **最小化信息泄露**：不在 ICMP Error 中泄露过多内部信息
5. **校验和验证**：严格验证校验和，拒绝格式错误的 ICMP 消息

---

## 8. 配置参数

```rust
/// ICMP 配置参数
#[derive(Debug, Clone)]
pub struct IcmpConfig {
    /// 是否响应 Echo Request (ping)
    pub enable_echo_reply: bool,  // 默认: true

    /// Echo 请求超时时间
    pub echo_timeout: Duration,   // 默认: 1秒

    /// 最大待处理 Echo 请求数量
    pub max_pending_echoes: usize, // 默认: 100

    /// 是否响应 Timestamp Request
    pub enable_timestamp_reply: bool,  // 默认: false (安全考虑)

    /// 是否响应 Address Mask Request
    pub enable_address_mask_reply: bool,  // 默认: false (已废弃)

    /// 是否接受 Redirect 消息
    pub accept_redirects: bool,    // 默认: false (安全考虑)

    /// 是否接受 Router Advertisement
    pub accept_router_advertisements: bool,  // 默认: false

    /// ICMP Error 消息发送速率限制（每秒）
    pub error_rate_limit: u32,     // 默认: 10

    /// 是否响应广播 Echo Request
    pub reply_broadcast_echo: bool,  // 默认: false
}

impl Default for IcmpConfig {
    fn default() -> Self {
        Self {
            enable_echo_reply: true,
            echo_timeout: Duration::from_secs(1),
            max_pending_echoes: 100,
            enable_timestamp_reply: false,
            enable_address_mask_reply: false,
            accept_redirects: false,
            accept_router_advertisements: false,
            error_rate_limit: 10,
            reply_broadcast_echo: false,
        }
    }
}
```

---

## 9. 测试场景

### 9.1 基本功能测试

1. **Echo Request/Reply 测试**
   - 发送 Echo Request，验证收到 Echo Reply
   - 验证 Identifier 和 Sequence Number 匹配
   - 验证负载数据完整返回

2. **Destination Unreachable 测试**
   - 发送到不可达网络，收到 Network Unreachable
   - 发送到不可达主机，收到 Host Unreachable
   - 发送到未监听端口，收到 Port Unreachable

3. **Time Exceeded 测试**
   - TTL=1 的数据报，收到 Time Exceeded (TTL)
   - 验证消息中包含原始数据报头部

### 9.2 边界情况测试

1. **最小/最大 ICMP 包**
   - 8 字节的最小 ICMP 头部
   - MTU 限制下的最大 ICMP 包

2. **校验和测试**
   - 错误的校验和，验证包被丢弃
   - 边界值的校验和计算

3. **序列号回绕**
   - Sequence Number 从 65535 到 0 的过渡

### 9.3 异常情况测试

1. **ICMP Error 循环防护**
   - 确保不会为 ICMP Error 消息发送另一个 ICMP Error

2. **未知的 Type/Code**
   - 收到未知类型时，应该静默丢弃

3. **格式错误的 ICMP**
   - 长度不足的 ICMP 包
   - 非法字段值的处理

4. **速率限制测试**
   - 快速连续发送 Echo Request，验证响应速率受限

5. **安全测试**
   - 伪造的 Redirect 消息
   - 广播地址的 Echo Request

---

## 10. 参考资料

1. **RFC 792** - Internet Control Message Protocol (ICMP)
2. **RFC 1122** - Requirements for Internet Hosts -- Communication Layers (对 ICMP 的修订)
3. **RFC 950** - Internet Standard Subnetting Procedure
4. **RFC 1191** - Path MTU Discovery
5. **RFC 1256** - ICMP Router Discovery Messages
6. **RFC 1812** - Requirements for IP Version 4 Routers
7. **RFC 4443** - ICMPv6 Specifications (ICMP for IPv6)
8. **RFC 5508** - ICMP and ICMPv6 for NAT
9. **Wikipedia - Internet Control Message Protocol**
