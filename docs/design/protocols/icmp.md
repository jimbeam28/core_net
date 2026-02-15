# Internet Control Message Protocol (ICMP) 详细设计文档

## 1. 协议概述

### 1.1 背景与历史

**协议全称和定义：**
- Internet Control Message Protocol（互联网控制消息协议）
- 在 TCP/IP 协议栈中属于网络层（与 IP 同层）
- 核心功能：为 IP 层提供错误报告、诊断和网络控制消息传递机制

**为什么需要 ICMP 协议？**

IP 协议本身是一种不可靠、无连接的数据报传输协议，它不保证数据报的交付，也缺乏报告错误的机制。当网络中出现问题时（如目标不可达、超时、重定向等），IP 协议无法通知发送方。ICMP 协议正是为了弥补这个缺陷而设计：

1. **错误报告**：当路由器或主机在处理 IP 数据报时遇到问题，通过 ICMP 消息通知发送方
2. **网络诊断**：提供 ping（回显请求/回复）等工具用于网络连通性测试
3. **流量控制**：通过源抑制消息控制发送速率
4. **路由优化**：通过重定向消息告知主机更优的路由路径

**历史背景：**
- **RFC 792**：发布于 1981 年 9 月，由 Jon Postel 编写
- 协议演进：
  - RFC 792 定义了原始 ICMP 协议（用于 IPv4）
  - RFC 950（1985）定义了子网掩码请求/回复
  - RFC 1191（1990）定义了路径 MTU 发现机制
  - RFC 4443（2006）为 IPv6 定义了 ICMPv6（ICMP for IPv6）
- 相关补充 RFC：
  - RFC 1122（主机要求）
  - RFC 1812（路由器要求）
  - RFC 0791（IP 协议）

### 1.2 设计原理

ICMP 协议的核心设计思想是提供一种轻量级的控制消息传递机制，用于网络层的信息交换，而不引入连接的复杂性。

ICMP 消息作为 IP 数据报的数据部分进行传输，本质上仍是网络层协议：

```
+-------------------+
|   IP 数据报头部    |
+-------------------+
|   ICMP 消息        |
+-------------------+
|   ICMP 数据部分    |
+-------------------+
```

**关键特点：**

1. **无状态性**：ICMP 本身不维护连接状态，每个消息独立处理
2. **错误报告限制**：ICMP 消息本身不再产生 ICMP 错误消息（避免无限循环）
3. **对原始数据报的引用**：大多数 ICMP 错误消息包含触发错误的原始 IP 数据报的头部和前 8 字节数据
4. **双向通信**：既支持错误消息（由路由器/主机主动发送），也支持请求/回复消息（如 ping）
5. **受信任但有限**：ICMP 消息用于诊断，但不能完全依赖（可能被防火墙阻断）

---

## 2. 报文格式

### 2.1 报文结构

ICMP 报文分为**报头**和**数据**两部分，所有 ICMP 消息共享相同的基本报头格式：

```
 0                   1                   2                   3
 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|     Type      |     Code     |          Checksum             |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|                             ...                              |
~                      Message-specific Data                   ~
|                             ...                              |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|               Original IP Header (error messages only)       |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|               First 8 bytes of original datagram             |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
```

**对于回显请求/回复消息（Echo Request/Reply）：**

```
 0                   1                   2                   3
 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|     Type      |     Code     |          Checksum             |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|           Identifier          |        Sequence Number       |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|                             Data                              |
+                                                             +
|                                                             +
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
```

**对于目标不可达消息（Destination Unreachable）：**

```
 0                   1                   2                   3
 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|     Type      |     Code     |          Checksum             |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|                             unused                            |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|      Original IP Header + First 8 bytes of datagram data      |
~                                                             ~
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
```

### 2.2 字段说明

#### 通用头部字段

| 字段 | 大小 | 说明 | 常见值 |
|------|------|------|--------|
| Type | 1 字节 | ICMP 消息类型，标识消息的类别 | 0-18（见类型表） |
| Code | 1 字节 | 消息子类型代码，提供更具体的信息 | 0-15（取决于 Type） |
| Checksum | 2 字节 | 整个 ICMP 消息的校验和 | 计算值 |
| Message-specific Data | 可变 | 特定消息类型的附加数据 | 取决于消息类型 |

#### Type 字段值定义

| Type 值 | 消息名称 | 类别 |
|---------|----------|------|
| 0 | Echo Reply（回显回复） | 查询消息 |
| 3 | Destination Unreachable（目标不可达） | 错误消息 |
| 4 | Source Quench（源抑制，已废弃） | 错误消息 |
| 5 | Redirect（重定向） | 错误消息 |
| 8 | Echo Request（回显请求） | 查询消息 |
| 9 | Router Advertisement（路由器通告） | 查询消息 |
| 10 | Router Solicitation（路由器请求） | 查询消息 |
| 11 | Time Exceeded（超时） | 错误消息 |
| 12 | Parameter Problem（参数问题） | 错误消息 |
| 13 | Timestamp Request（时间戳请求） | 查询消息 |
| 14 | Timestamp Reply（时间戳回复） | 查询消息 |
| 15 | Information Request（信息请求，已废弃） | 查询消息 |
| 16 | Information Reply（信息回复，已废弃） | 查询消息 |
| 17 | Address Mask Request（地址掩码请求） | 查询消息 |
| 18 | Address Mask Reply（地址掩码回复） | 查询消息 |

#### Code 字段定义（针对主要消息类型）

**Type = 3 (Destination Unreachable)：**

| Code 值 | 含义 |
|---------|------|
| 0 | Network Unreachable（网络不可达） |
| 1 | Host Unreachable（主机不可达） |
| 2 | Protocol Unreachable（协议不可达） |
| 3 | Port Unreachable（端口不可达） |
| 4 | Fragmentation Needed and DF Set（需要分片但 DF 标志已设置） |
| 5 | Source Route Failed（源路由失败） |

**Type = 5 (Redirect)：**

| Code 值 | 含义 |
|---------|------|
| 0 | Redirect for Network（对网络重定向） |
| 1 | Redirect for Host（对主机重定向） |
| 2 | Redirect for Type-of-Service and Network（对 TOS 和网络重定向） |
| 3 | Redirect for Type-of-Service and Host（对 TOS 和主机重定向） |

**Type = 11 (Time Exceeded)：**

| Code 值 | 含义 |
|---------|------|
| 0 | Time to Live Exceeded in Transit（传输中 TTL 超时） |
| 1 | Fragment Reassembly Time Exceeded（分片重组超时） |

**Type = 12 (Parameter Problem)：**

| Code 值 | 含义 |
|---------|------|
| 0 | Pointer indicates the error（指针指向错误位置） |

**最小/最大报文长度：**
- 最小 ICMP 报文：8 字节（仅包含 Type、Code、Checksum）
- Echo Request/Reply：最少 8 字节，数据部分可变
- 错误消息：最少 8 字节 + 原始 IP 头部（20 字节）+ 原始数据前 8 字节 = 36 字节

### 2.3 封装格式

ICMP 消息封装在 IP 数据报中传输：

```
+-------------------+
|   Ethernet 头部    |  （如果以太网传输）
+-------------------+
|   IP 数据报头部    |  Protocol 字段 = 1
+-------------------+
|   ICMP 报文       |
+-------------------+
|   ICMP 数据       |
+-------------------+
```

**IP 封装要点：**
- IP 头部的 Protocol 字段设置为 1（表示 ICMP）
- IP 头部中的 TTL 需要正常设置
- ICMP 错误消息不发送关于其他 ICMP 错误消息的报文（避免广播风暴）
- ICMP 错误消息不发送关于多播或广播数据报的报文

---

## 3. 消息类型详解

ICMP 消息分为两大类：**错误消息**（Error Messages）和**查询消息**（Query Messages）。

### 3.1 错误消息

错误消息由路由器或主机在处理 IP 数据报遇到问题时发送。

#### 3.1.1 Destination Unreachable（Type 3）

**触发条件：**
- 路由器无法到达目标网络
- 目标主机不可达
- 目标端口没有进程监听
- 需要分片但 DF 标志已设置

**处理流程：**
1. IP 层检测到不可达原因
2. 构造 ICMP 不可达消息
3. 附加原始 IP 头部 + 前 8 字节数据
4. 发回源地址

#### 3.1.2 Time Exceeded（Type 11）

**触发条件：**
- IP 数据报的 TTL 字段减为 0（Code 0）
- 分片重组超时（Code 1）

**处理流程：**
1. 路由器检测到 TTL=0 或重组超时
2. 丢弃原始数据报
3. 构造 ICMP 超时消息
4. 发回源地址

#### 3.1.3 Redirect（Type 5）

**触发条件：**
- 路由器发现主机使用了非最优路由
- 主机和目标主机在同一网络中

**处理流程：**
1. 路由器检测到更优路径
2. 向主机发送 Redirect 消息，告知正确的网关地址
3. 主机应更新路由缓存

#### 3.1.4 Parameter Problem（Type 12）

**触发条件：**
- IP 头部参数无效或缺失
- 可选部分出现错误

**处理流程：**
1. 检测到参数错误
2. 使用 Pointer 字段指向错误位置
3. 构造 ICMP 参数问题消息
4. 发回源地址

### 3.2 查询消息

查询消息用于诊断和信息收集，通常成对出现（请求/回复）。

#### 3.2.1 Echo Request/Reply（Type 8/0）

**用途：**
- 实现 ping 工具，测试网络连通性
- 测量往返时间（RTT）

**请求处理流程：**
1. 主机发送 Echo Request（Type 8）
2. 目标主机收到后发送 Echo Reply（Type 0）
3. Identifier 和 Sequence Number 用于匹配请求和回复

**字段说明：**
- **Identifier**：用于标识 Echo 会话（通常使用进程 ID）
- **Sequence Number**：序列号，用于匹配请求和回复
- **Data**：发送方填充的任意数据，原样返回

#### 3.2.2 Timestamp Request/Reply（Type 13/14）

**用途：**
- 测量网络延迟
- 估算时钟偏差

**处理流程：**
1. 发送方发送 Timestamp Request，包含 Originate Timestamp
2. 接收方添加 Receive Timestamp 和 Transmit Timestamp
3. 发送方计算往返时间和时钟偏差

**字段说明：**
- **Originate Timestamp**：发送方发送请求的时间
- **Receive Timestamp**：接收方收到请求的时间
- **Transmit Timestamp**：接收方发送回复的时间

#### 3.2.3 Address Mask Request/Reply（Type 17/18）

**用途：**
- 主机查询子网掩码
- 适用于无盘工作站等场景

**处理流程：**
1. 主机发送 Address Mask Request
2. 网关或配置服务器回复 Address Mask，包含子网掩码

---

## 4. 报文处理逻辑

### 4.1 接收处理流程

```
收到 IP 数据报
    |
    v
检查 Protocol 字段 == 1？
    |
    +-- No --> 传递给其他协议层
    |
    v
 Yes
    |
    v
提取 ICMP 消息
    |
    v
计算并验证 Checksum
    |
    +-- 失败 --> 丢弃
    |
    v
 成功
    |
    v
根据 Type 字段分发
    |
    v
+-----------------------+
|   Type 判断分支        |
+-----------------------+
    |           |           |
    v           v           v
错误消息      Echo 请求   Echo 回复
    |           |           |
    v           v           v
检查原报文    生成回复    匹配请求
有效性        并发送       处理
```

### 4.2 错误消息处理

**处理流程：**

1. **提取信息：**
   - Type/Code → 错误类型和具体原因
   - Original IP Header → 原始数据报头部信息
   - First 8 bytes → 原始数据报的传输层头部

2. **处理步骤：**
   - 验证原报文有效性（长度、格式）
   - 提取原报文的源地址、协议类型
   - 根据错误类型通知上层协议（如 TCP、UDP）
   - 记录错误统计信息

3. **响应动作：**
   - 通知相关协议层（TCP、UDP、IP）
   - 更新路径 MTU（对于 Fragmentation Needed）
   - 更新路由缓存（对于 Redirect）

### 4.3 Echo Request（Ping）处理

**处理流程：**

1. **提取信息：**
   - Identifier → Echo 会话标识
   - Sequence Number → 序列号
   - Data → 请求携带的数据

2. **处理步骤：**
   - 验证消息长度和格式
   - 检查目标地址是否为本机

3. **响应动作：**
   - 构造 Echo Reply（Type = 0）
   - 保持 Identifier 和 Sequence Number 不变
   - 原样返回 Data
   - 计算新的 Checksum
   - 发送回复

### 4.4 其他查询消息处理

**Timestamp Request（Type 13）：**
1. 提取 Originate Timestamp
2. 记录当前时间作为 Receive Timestamp
3. 准备发送时间作为 Transmit Timestamp
4. 发送 Timestamp Reply

**Address Mask Request（Type 17）：**
1. 检查是否配置了子网掩码
2. 如果有，回复 Address Mask Reply
3. 否则丢弃（或转发给配置服务器）

---

## 5. 核心数据结构

### 5.1 ICMP 消息类型枚举

```rust
/// ICMP 消息类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum IcmpType {
    // 错误消息
    DestinationUnreachable = 3,
    SourceQuench = 4,          // 已废弃
    Redirect = 5,
    TimeExceeded = 11,
    ParameterProblem = 12,

    // 查询消息
    EchoRequest = 8,
    EchoReply = 0,
    TimestampRequest = 13,
    TimestampReply = 14,
    InformationRequest = 15,   // 已废弃
    InformationReply = 16,     // 已废弃
    AddressMaskRequest = 17,
    AddressMaskReply = 18,

    // 未知类型
    Unknown(u8),
}
```

### 5.2 ICMP 代码枚举

```rust
/// Destination Unreachable 代码
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum DestUnreachableCode {
    NetworkUnreachable = 0,
    HostUnreachable = 1,
    ProtocolUnreachable = 2,
    PortUnreachable = 3,
    FragmentationNeeded = 4,
    SourceRouteFailed = 5,
}

/// Time Exceeded 代码
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum TimeExceededCode {
    TtlExceeded = 0,
    FragmentReassemblyTimeExceeded = 1,
}

/// Redirect 代码
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum RedirectCode {
    Network = 0,
    Host = 1,
    TypeOfServiceAndNetwork = 2,
    TypeOfServiceAndHost = 3,
}
```

### 5.3 ICMP 报头结构

```rust
/// ICMP 通用报头
#[derive(Debug, Clone)]
pub struct IcmpHeader {
    /// 消息类型
    pub type_: IcmpType,
    /// 代码
    pub code: u8,
    /// 校验和
    pub checksum: u16,
}

impl IcmpHeader {
    /// 报头固定大小
    pub const SIZE: usize = 4;

    /// 从字节流解析 ICMP 报头
    pub fn parse(packet: &Packet) -> Result<Self, ProcessError>;

    /// 序列化为字节
    pub fn serialize(&self) -> Vec<u8>;
}
```

### 5.4 Echo 消息结构

```rust
/// Echo 请求/回复消息
#[derive(Debug, Clone)]
pub struct EchoMessage {
    /// 标识符
    pub identifier: u16,
    /// 序列号
    pub sequence: u16,
    /// 数据
    pub data: Vec<u8>,
}

impl EchoMessage {
    /// 从字节流解析 Echo 消息
    pub fn parse(packet: &mut Packet) -> Result<Self, ProcessError>;

    /// 序列化为字节
    pub fn serialize(&self) -> Vec<u8>;

    /// 计算校验和
    pub fn compute_checksum(&self) -> u16;
}
```

### 5.5 错误消息结构

```rust
/// ICMP 错误消息（包含原始数据报引用）
#[derive(Debug, Clone)]
pub struct IcmpErrorMessage {
    /// ICMP 报头
    pub header: IcmpHeader,
    /// 原始 IP 头部
    pub original_ip_header: Vec<u8>,
    /// 原始数据报前 8 字节
    pub original_data: Vec<u8>,
    /// 重定向消息的目标地址（仅 Redirect 消息）
    pub gateway_address: Option<Ipv4Addr>,
    /// 参数问题的指针位置（仅 Parameter Problem 消息）
    pub pointer: Option<u8>,
}

impl IcmpErrorMessage {
    /// 从字节流解析错误消息
    pub fn parse(packet: &mut Packet) -> Result<Self, ProcessError>;

    /// 序列化为字节
    pub fn serialize(&self) -> Vec<u8>;
}
```

### 5.6 时间戳消息结构

```rust
/// 时间戳消息
#[derive(Debug, Clone)]
pub struct TimestampMessage {
    /// 发起时间戳
    pub originate_timestamp: u32,
    /// 接收时间戳
    pub receive_timestamp: u32,
    /// 发送时间戳
    pub transmit_timestamp: u32,
}

impl TimestampMessage {
    /// 从字节流解析
    pub fn parse(packet: &mut Packet) -> Result<Self, ProcessError>;

    /// 序列化为字节
    pub fn serialize(&self) -> Vec<u8>;
}
```

### 5.7 ICMP 处理器接口

```rust
/// ICMP 协议处理器
pub struct IcmpProcessor {
    /// 统计信息
    stats: IcmpStats,
    /// 子网掩码（用于回复 Address Mask Request）
    subnet_mask: Option<Ipv4Addr>,
}

/// ICMP 统计信息
#[derive(Debug, Default)]
pub struct IcmpStats {
    /// 接收的消息数
    pub messages_received: u64,
    /// 发送的消息数
    pub messages_sent: u64,
    /// 接收的错误消息数
    pub errors_received: u64,
    /// 发送的 Echo Reply 数
    pub echo_replies_sent: u64,
}

impl IcmpProcessor {
    /// 创建新的 ICMP 处理器
    pub fn new() -> Self;

    /// 处理 ICMP 消息
    pub fn process_message(
        &mut self,
        packet: &mut Packet,
        source_ip: Ipv4Addr,
        dest_ip: Ipv4Addr,
    ) -> Result<Option<Vec<u8>>, ProcessError>;

    /// 发送 Echo Request（ping）
    pub fn send_echo_request(
        &mut self,
        dest_ip: Ipv4Addr,
        identifier: u16,
        sequence: u16,
        data: Vec<u8>,
    ) -> Result<Vec<u8>, ProcessError>;

    /// 构造错误消息
    pub fn build_error_message(
        &self,
        type_: IcmpType,
        code: u8,
        original_packet: &Packet,
    ) -> Result<Vec<u8>, ProcessError>;

    /// 获取统计信息
    pub fn stats(&self) -> &IcmpStats;
}
```

---

## 6. 与其他模块的交互

### 6.1 与 IP 层的交互

**从 IP 层接收：**
1. IP 数据报的 Protocol 字段为 1
2. 传递完整的 IP Payload（ICMP 消息）
3. 提供源地址和目的地址信息

**向 IP 层发送：**
1. 构造完整的 ICMP 消息
2. 调用 IP 层发送接口
3. IP 层负责封装 IP 头部并计算校验和

**接口示意：**
```rust
// IP 层调用 ICMP 处理器
if ip_header.protocol == 1 {
    let icmp_result = icmp_processor.process_message(
        &mut packet,
        ip_header.source_addr,
        ip_header.dest_addr
    );
    // 如果需要回复，通过 IP 层发送
}

// ICMP 发送消息通过 IP 层
let icmp_packet = icmp_processor.build_error_message(...);
ip_layer.send(icmp_packet, source_ip, dest_ip, 1);
```

### 6.2 与传输层（TCP/UDP）的交互

**通知传输层错误：**
当 ICMP 收到错误消息（如 Destination Unreachable、Time Exceeded）时：
1. 解析原始 IP 头部，提取协议类型
2. 提取原始数据的前 8 字节（包含传输层头部）
3. 根据协议类型通知 TCP 或 UDP
4. 传输层根据错误类型采取行动（如重传、关闭连接）

**接口示意：**
```rust
match icmp_header.type_ {
    IcmpType::DestinationUnreachable => {
        let proto = extract_protocol(original_ip_header);
        match proto {
            6 => tcp_layer.handle_icmp_error(icmp_error),
            17 => udp_layer.handle_icmp_error(icmp_error),
            _ => {},
        }
    }
    // ...
}
```

### 6.3 与配置模块的交互

**子网掩码查询：**
- 从配置模块获取本机子网掩码
- 用于回复 Address Mask Request

**路由缓存更新：**
- 收到 Redirect 消息时更新路由缓存
- 需要与路由模块交互

### 6.4 与统计模块的交互

- 记录各种 ICMP 消息的收发统计
- 支持 SNMP MIB-II ICMP 组的统计查询

---

## 7. 安全考虑

### 7.1 ICMP 攻击

**ICMP Flood 攻击：**
- **攻击方式**：攻击者向目标发送大量 Echo Request，耗尽目标资源
- **攻击影响**：目标主机或网络因处理大量 ICMP 消息而无法响应正常请求
- **防御措施**：
  - 限制 ICMP 消息处理速率
  - 使用防火墙过滤不必要的 ICMP 消息
  - 对 Echo Request 实现速率限制

**Ping of Death：**
- **攻击方式**：发送超过 IP 最大 MTU 的超大 ICMP Echo Request
- **攻击影响**：导致系统分片重组时缓冲区溢出
- **防御措施**：
  - 验证 ICMP 消息长度
  - 拒绝处理异常大小的消息
  - 实现严格的分片重组检查

**Smurf 攻击：**
- **攻击方式**：使用受害者 IP 作为源地址，向网络广播地址发送 Echo Request
- **攻击影响**：所有主机回复受害者，造成流量放大
- **防御措施**：
  - 不响应广播地址的 Echo Request
  - 路由器阻止定向广播
  - Ingress Filtering 验证源地址

### 7.2 实现建议

1. **速率限制**：对 ICMP 消息处理实现速率限制，防止资源耗尽
   - 每秒最多处理 N 个 ICMP 消息
   - 对每个源地址独立限速

2. **严格验证**：
   - 验证 ICMP 消息长度和格式
   - 验证 Checksum
   - 验证错误消息中引用的原始数据报

3. **限制响应**：
   - 不响应多播/广播地址的 Echo Request
   - 不生成关于其他 ICMP 错误消息的错误消息
   - 不生成关于多播/广播数据报的错误消息

4. **配置灵活性**：
   - 支持配置哪些 ICMP 消息类型可以接收/发送
   - 支持完全禁用 ICMP（虽然不推荐）

5. **日志记录**：
   - 记录异常的 ICMP 消息（如长度异常、校验和错误）
   - 记录速率限制的触发情况

---

## 8. 配置参数

```rust
/// ICMP 协议配置
pub struct IcmpConfig {
    /// 是否启用 ICMP 处理
    pub enabled: bool,  // 默认: true

    /// 是否响应 Echo Request
    pub respond_to_echo: bool,  // 默认: true

    /// 子网掩码（用于 Address Mask Reply）
    pub subnet_mask: Option<Ipv4Addr>,  // 默认: None

    /// ICMP 消息速率限制（消息/秒）
    pub rate_limit: Option<u64>,  // 默认: None（无限制）

    /// 是否发送 Redirect 消息
    pub send_redirects: bool,  // 默认: true

    /// 允许接收的 ICMP 类型
    pub allowed_types: Vec<IcmpType>,  // 默认: 所有标准类型

    /// ICMP TTL（发送时使用，从 IP TTL 计算）
    pub ttl: Option<u8>,  // 默认: None（使用 IP 层默认）
}

impl Default for IcmpConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            respond_to_echo: true,
            subnet_mask: None,
            rate_limit: None,
            send_redirects: true,
            allowed_types: vec![
                IcmpType::EchoRequest,
                IcmpType::EchoReply,
                IcmpType::DestinationUnreachable,
                IcmpType::TimeExceeded,
                IcmpType::ParameterProblem,
            ],
            ttl: None,
        }
    }
}
```

---

## 9. 测试场景

### 9.1 基本功能测试

1. **Echo Request/Reply（Ping）测试**
   - 发送 Echo Request，验证收到正确的 Echo Reply
   - 验证 Identifier 和 Sequence Number 匹配
   - 验证 Data 原样返回
   - 测试不同大小的 Data（0 字节、100 字节、最大 MTU）

2. **错误消息接收测试**
   - 模拟网络层返回 Destination Unreachable
   - 模拟网络层返回 Time Exceeded
   - 验证原始 IP 头部和数据正确提取

3. **Checksum 验证测试**
   - 发送错误的 Checksum，验证消息被拒绝
   - 验证发送的消息 Checksum 正确

### 9.2 边界情况测试

1. **最小报文测试**
   - 发送仅包含 Type、Code、Checksum 的 4 字节消息

2. **最大报文测试**
   - 发送接近 MTU 大小的 Echo Request
   - 验证分片场景（如果 IP 层支持）

3. **异常长度测试**
   - 发送声称长度但实际长度不足的消息
   - 发送声称长度但实际长度过长的消息

4. **未知类型测试**
   - 发送未定义的 Type 值
   - 验证被正确忽略或处理

### 9.3 异常情况测试

1. **错误消息不生成错误消息**
   - 接收到 ICMP 错误消息
   - 模拟生成新错误消息的情况
   - 验证不会因为 ICMP 错误消息再生成错误消息

2. **多播/广播不响应**
   - 向多播地址发送 Echo Request
   - 向广播地址发送 Echo Request
   - 验证不发送回复

3. **并发测试**
   - 同时发送多个 Echo Request
   - 验证回复正确匹配（Identifier/Sequence）

4. **速率限制测试**
   - 快速发送大量 Echo Request
   - 验证速率限制生效
   - 验证正常流量不受影响

---

## 10. 参考资料

1. **RFC 792** - Internet Control Message Protocol (ICMP)
2. **RFC 1122** - Requirements for Internet Hosts -- Communication Layers
3. **RFC 1812** - Requirements for IP Version 4 Routers
4. **RFC 950** - Internet Standard Subnetting Procedure
5. **RFC 1191** - Path MTU Discovery
6. **RFC 4443** - Internet Control Message Protocol (ICMPv6) for IPv6
7. **IANA ICMP Parameters** - https://www.iana.org/assignments/icmp-parameters
8. **Wikipedia - Internet Control Message Protocol** - https://en.wikipedia.org/wiki/Internet_Control_Message_Protocol
