# IPv4 协议详细设计文档

## 1. 协议概述

### 1.1 背景与历史

**协议全称和定义：**
- 协议全称：Internet Protocol Version 4 (IPv4) - 互联网协议第4版
- 在 TCP/IP 协议栈中的层级位置：网络层 (Network Layer, OSI 第3层)
- 核心功能概述：提供无连接、不可靠的数据报传输服务，负责寻址、路由和分段重组

**为什么需要 IPv4？**

IPv4 解决了异构网络之间互联互通的核心问题：
- **统一寻址**：提供全局唯一的 32 位地址空间，标识网络中的主机
- **路由转发**：通过路由机制将数据包从源主机传递到目的主机
- **分段重组**：处理不同网络 MTU 差异，支持数据包的分片和重组
- **协议复用**：通过协议字段支持多种上层协议（TCP、UDP、ICMP 等）

**历史背景：**
- **RFC 791**：1981年9月发布，替代早期的 RFC 760
- **RFC 792**：定义 ICMP 协议，用于错误报告和诊断
- **RFC 950**：1985年引入子网划分 (Subnetting)
- **RFC 1122**：1989年更新主机通信层要求
- **RFC 1519**：1993年引入无类域间路由 (CIDR)
- **RFC 815**：简化的重组算法

### 1.2 设计原理

IPv4 采用**数据报交换**模型，核心思想是"尽力而为" (Best Effort) 传输：

**CoreNet 实现范围：**
- **不支持分片和重组**：当前版本不实现 IP 分片和重组功能
- **上层协议仅支持 ICMP**：当前版本仅支持 ICMP 协议（Protocol=1），TCP/UDP 暂不实现

```
发送方                     路由器                      接收方
  |                          |                           |
  |--[IP数据报]-------------->|                           |
  |                          |---[IP数据报]-------------->|
  |   (可能分片)              |   (可能进一步分片)         |
  |                          |                           |
  v                          v                           v
 封装:                     转发:                       重组:
 数据链路层帧                查路由表                   重装分片
```

**关键特点：**

1. **无连接**：发送数据前不需要建立连接，每个数据报独立路由
2. **不可靠**：不保证数据报送达、不保证顺序、无重传机制
3. **尽最大努力**：尽力传输但不承诺服务质量
4. **分段透明**：上层协议感知不到分段重组过程（通过标识、标志、片偏移实现）

---

## 2. 报文格式

### 2.1 报文结构

```
 0                   1                   2                   3
 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|Version|  IHL  |Type of Service|          Total Length         |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|         Identification        |Flags|      Fragment Offset    |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|  Time to Live |    Protocol   |         Header Checksum       |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|                       Source Address                          |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|                    Destination Address                        |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|                    Options (optional)                         |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|                      Padding (optional)                      |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|                            Data                              |
|                              ...                              |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
```

**字段说明：**

| 字段 | 大小 | 说明 | 常见值 |
|------|------|------|--------|
| Version | 4 bits | IP 协议版本号 | 4 |
| IHL | 4 bits | 首部长度（以 4 字节为单位） | 5 (无选项) ~ 15 |
| Type of Service | 1 byte | 服务类型（QoS，现由 DSCP/ECN 替代） | 0 (普通服务) |
| Total Length | 2 bytes | IP 数据报总长度（首部+数据） | 最大 65535 |
| Identification | 2 bytes | 标识符，用于分段重组 | 随机生成 |
| Flags | 3 bits | 分段标志：DF/MF 保留 | 010 (DF=0, MF=0) |
| Fragment Offset | 13 bits | 片偏移（以 8 字节为单位） | 0 ~ 8189 |
| Time to Live | 1 byte | 生存时间，每跳减 1 | 64, 128, 255 |
| Protocol | 1 byte | 上层协议号 | 1(ICMP), 6(TCP), 17(UDP) |
| Header Checksum | 2 bytes | 首部校验和 | 计算值 |
| Source Address | 4 bytes | 源 IP 地址 | 发送方地址 |
| Destination Address | 4 bytes | 目的 IP 地址 | 接收方地址 |
| Options | 可变 | 可选字段（安全、时间戳等） | 通常为空 |
| Padding | 可变 | 填充使首部为 4 字节倍数 | 0 |

**最小/最大报文长度：**
- 最小首部：20 字节（无选项）
- 最大首部：60 字节（40 字节选项）
- 最小数据报：20 字节（只有首部，无数据）
- 最大数据报：65535 字节
- **MTU 限制**：每个链路有 MTU（通常 1500），超出需要分片

**协议字段常用值：**

| 协议 | Protocol 字段值 |
|------|----------------|
| ICMP | 1 |
| TCP | 6 |
| UDP | 17 |
| IPv6 (tunnel) | 41 |
| OSPF | 89 |
| SCTP | 132 |

### 2.2 分段相关字段详解

**分段机制**确保数据包可以通过 MTU 较小的网络：

```
原始数据报 (ID: 12345, DF=0, MF=0, Offset=0)
        |
        v
MTU 限制
        |
        +----> 分片1 (ID: 12345, DF=0, MF=1, Offset=0)
        |
        +----> 分片2 (ID: 12345, DF=0, MF=1, Offset=185)
        |
        +----> 分片N (ID: 12345, DF=0, MF=0, Offset=370)
                  (最后一片 MF=0)
```

**Flags 字段：**
```
 0   1   2
+---+---+---+
| 0 | D | M |
|   | F | F |
|   |   |   |
+---+---+---+
```
- Bit 0: 保留，必须为 0
- Bit 1 (DF): Don't Fragment - 禁止分片，如果需要分片则丢弃并返回 ICMP
- Bit 2 (MF): More Fragments - 后续还有分片（最后一片 MF=0）

**Fragment Offset:**
- 单位为 8 字节，指向该分片数据在原始数据报中的位置
- 例如：Offset = 185 表示该分片数据从原始数据报的第 1480 字节开始

### 2.3 封装格式

**下层封装（以太网）：**
```
+------------------+
|  以太网首部      |
| (类型=0x0800)    |
+------------------+
|  IP 首部         |
+------------------+
|  IP 数据         |
+------------------+
|  以太网 FCS      |
+------------------+
```

**上层复用：**
```
IP 首部 (Protocol=6)  -->  TCP 首部  -->  TCP 数据
IP 首部 (Protocol=17) -->  UDP 首部  -->  UDP 数据
IP 首部 (Protocol=1)  -->  ICMP 报文
```

---

## 3. 状态机设计

IPv4 本质上是**无状态协议**，不维护连接状态。

### 3.0 状态变量

| 变量名称 | 类型 | 用途 | 初始值 |
|---------|------|------|--------|
| ttl | u8 | 数据报生存时间 | 配置值(64/128) |
| packet_size | u16 | 当前处理的数据报长度 | 解析后获得 |

**注：CoreNet 当前版本不支持分片和重组，因此不维护 identification 和分片相关状态。**

---

## 4. 报文处理逻辑

### 4.0 定时器

IPv4 使用的主要定时器：

| 定时器名称 | 启动条件 | 超时时间 | 超时动作 |
|-----------|---------|---------|---------|
| (无) | 当前版本不实现分片重组，无定时器 | - | - |

**注：CoreNet 当前版本不支持分片和重组，因此不使用重组定时器。**

### 4.1 接收处理总流程

```
      [接收 IP 数据报]
              |
              v
      [验证首部校验和]
              |
      +-------+-------+
      |               |
    校验失败        校验通过
      |               |
      v               v
    [丢弃]      [检查版本号 (IHL>=5)]
                      |
              +-------+-------+
              |               |
            版本=4           版本≠4
              |               |
              v               v
        [检查目的地址]    [丢弃/统计]
              |
      +-------+-------+
      |               |
   本机接收         转发
      |               |
      v               v
[检查协议字段]    [TTL -= 1]
      |               |
      v          +----+----+
  [多路分解]      |         |
      |        TTL=0      TTL>0
      |         |         |
      v         v         v
[提交上层]   [丢弃]    [路由转发]
                    [可能分片]
```

### 4.2 接收处理

#### 4.2.1 本机接收处理

**处理流程：**

1. **提取信息：**
   - Version → 必须为 4
   - IHL → 计算首部长度
   - Protocol → 上层协议类型
   - Source Address → 源 IP（用于响应）
   - Destination Address → 必须匹配本机 IP 或广播/组播地址
   - Total Length → 验证数据完整性

2. **处理步骤：**
   - 验证首部校验和
   - 检查版本号
   - 验证首部长度（IHL >= 5）
   - 检查目的地址是否为本机地址
   - 检查是否需要重组（MF 或 Offset 非零）
   - 提取数据部分提交上层协议

3. **资源更新：**
   - 接口统计：接收字节数、数据报数 +1
   - 重组表：如有分片，添加/更新重组条目
   - 定时器：如有分片，启动/重置重组定时器

4. **响应动作：**
   - 校验和错误：静默丢弃（不发送 ICMP）
   - 协议不可达：如果 Protocol 字段不支持，发送 ICMP Type 3 Code 2
   - 正常：将数据部分传递给上层协议处理

#### 4.2.2 转发处理

**处理流程：**

1. **提取信息：**
   - TTL → 剩余生存时间
   - Destination Address → 用于路由查找
   - Flags → DF 标志影响分片行为

2. **处理步骤：**
   - TTL 减 1
   - 检查 TTL 是否为 0（超时）
   - 执行路由查找确定下一跳
   - 检查出接口 MTU，判断是否需要分片
   - 重新计算首部校验和

3. **资源更新：**
   - TTL → TTL - 1
   - Header Checksum → 重新计算

4. **响应动作：**
   - TTL=0：丢弃数据报，发送 ICMP Time Exceeded (Type 11 Code 0)
   - 正常：转发到下一跳

**注：CoreNet 当前版本不支持分片和重组，因此不实现分段处理逻辑。收到 MF=1 或 Fragment Offset 非 0 的数据报应直接丢弃。**

### 4.3 分段处理

**注：CoreNet 当前版本不支持分片和重组，本章节仅供参考，不实现。**

#### 4.3.1 发送分片

**触发条件：** 数据报长度 > 出接口 MTU

**处理步骤：**

```
原始数据报: 4000 字节, MTU = 1500
----------------------------------------------------
分片1: Total=1500, ID=12345, MF=1, Offset=0
  数据: 1480 字节 (1500 - 20)

分片2: Total=1500, ID=12345, MF=1, Offset=185
  数据: 1480 字节

分片3: Total=1040, ID=12345, MF=0, Offset=370
  数据: 1020 字节
```

**分片计算：**
- 每片数据长度 = (MTU - 首部长度) 且必须为 8 字节倍数
- 片偏移 = 累积数据长度 / 8
- 最后一片 MF=0

**资源更新：**
- Identification: 为所有分片生成相同 ID
- Fragment Offset: 每片递增
- MF: 最后一片为 0，其余为 1
- Header Checksum: 每片重新计算

#### 4.3.2 重组处理

**处理步骤：**

1. **提取信息：**
   - Identification, Source, Destination, Protocol → 重组键
   - Fragment Offset → 在原始数据报中的位置
   - MF → 是否为最后一片
   - Data → 分片数据

2. **重组逻辑：**
   ```
   for 每个收到的分片:
       基于 <源IP, 目的IP, 协议, ID> 查找/创建重组条目
       存储分片数据（按偏移量）
       检查是否收到所有分片（数据连续且 MF=0 已收到）
       if 所有分片到齐:
           组装完整数据报
           提交上层
           删除重组条目
   ```

3. **资源更新：**
   - 重组表：添加/更新分片信息
   - 定时器：启动/重置 60 秒超时

4. **响应动作：**
   - 重组成功：提交上层协议
   - 重组超时：丢弃所有分片，发送 ICMP Type 11 Code 1

---

## 5. 核心数据结构

### 5.0 表项/缓存

IPv4 维护的主要表项：

| 资源名称 | 用途 | 最大容量 | 淘汰策略 |
|---------|------|---------|---------|
| (无) | 当前版本不支持分片重组，无表项 | - | - |

**注：CoreNet 当前版本不支持分片和重组，因此不维护分段重组表。**

### 5.1 报文结构

```rust
/// IPv4 首部结构
///
/// RFC 791 定义的 IPv4 数据报首部（固定 20 字节）
#[repr(C, packed)]
pub struct Ipv4Header {
    /// 版本 (4 bits) + 首部长度 (4 bits)
    /// 版本必须为 4，首部长度以 4 字节为单位
    pub version_ihl: u8,

    /// 服务类型 / 差分服务代码点
    pub type_of_service: u8,

    /// 总长度（首部 + 数据）
    pub total_length: u16,

    /// 标识符（用于分段重组）
    pub identification: u16,

    /// 标志 (3 bits) + 片偏移 (13 bits)
    pub flags_fragment: u16,

    /// 生存时间
    pub ttl: u8,

    /// 上层协议号
    pub protocol: u8,

    /// 首部校验和
    pub header_checksum: u16,

    /// 源 IP 地址
    pub source_addr: u32,

    /// 目的 IP 地址
    pub dest_addr: u32,
}

impl Ipv4Header {
    /// 获取版本号（应该是 4）
    pub const fn version(&self) -> u8 {
        self.version_ihl >> 4
    }

    /// 获取首部长度（以 4 字节为单位）
    pub const fn ihl(&self) -> u8 {
        self.version_ihl & 0x0F
    }

    /// 获取首部长度（字节数）
    pub const fn header_len(&self) -> u8 {
        (self.version_ihl & 0x0F) * 4
    }

    /// 获取 DF 标志（Don't Fragment）
    pub const fn df_flag(&self) -> bool {
        (self.flags_fragment & 0x4000) != 0
    }

    /// 获取 MF 标志（More Fragments）
    pub const fn mf_flag(&self) -> bool {
        (self.flags_fragment & 0x2000) != 0
    }

    /// 获取片偏移（以 8 字节为单位）
    pub const fn fragment_offset(&self) -> u16 {
        self.flags_fragment & 0x1FFF
    }

    /// 计算首部校验和
    pub fn calculate_checksum(&self) -> u16 {
        // 实现校验和计算
    }

    /// 验证首部校验和
    pub fn verify_checksum(&self) -> bool {
        self.calculate_checksum() == self.header_checksum
    }
}
```

### 5.2 枚举类型

```rust
/// IPv4 协议号（上层协议类型）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Ipv4Protocol {
    /// ICMP (Internet Control Message Protocol)
    Icmp = 1,

    /// TCP (Transmission Control Protocol) - 未实现
    Tcp = 6,

    /// UDP (User Datagram Protocol) - 未实现
    Udp = 17,

    /// IPv6 隧道 - 未实现
    Ipv6 = 41,

    /// OSPF (Open Shortest Path First) - 未实现
    Ospf = 89,

    /// SCTP (Stream Control Transmission Protocol) - 未实现
    Sctp = 132,

    /// 未知协议
    Unknown(u8),
}

impl From<u8> for Ipv4Protocol {
    fn from(value: u8) -> Self {
        match value {
            1 => Ipv4Protocol::Icmp,
            6 => Ipv4Protocol::Tcp,
            17 => Ipv4Protocol::Udp,
            41 => Ipv4Protocol::Ipv6,
            89 => Ipv4Protocol::Ospf,
            132 => Ipv4Protocol::Sctp,
            v => Ipv4Protocol::Unknown(v),
        }
    }
}

impl From<Ipv4Protocol> for u8 {
    fn from(protocol: Ipv4Protocol) -> Self {
        match protocol {
            Ipv4Protocol::Icmp => 1,
            Ipv4Protocol::Tcp => 6,
            Ipv4Protocol::Udp => 17,
            Ipv4Protocol::Ipv6 => 41,
            Ipv4Protocol::Ospf => 89,
            Ipv4Protocol::Sctp => 132,
            Ipv4Protocol::Unknown(v) => v,
        }
    }
}

// 注：当前版本仅支持 ICMP (Protocol=1)，其他协议返回 UnsupportedProtocol 错误
```

```rust
/// IPv4 处理错误类型
#[derive(Debug)]
pub enum Ipv4Error {
    /// 版本号不匹配
    InvalidVersion { expected: u8, found: u8 },

    /// 首部长度无效
    InvalidHeaderLength { ihl: u8 },

    /// 校验和错误
    ChecksumError { expected: u16, calculated: u16 },

    /// 数据报长度不足
    PacketTooShort { expected: usize, found: usize },

    /// 数据报长度超过 MTU 且 DF 标志置位
    FragmentationNeeded { mtu: u16, length: u16 },

    /// TTL 超时
    TtlExceeded { ttl: u8 },

    /// 协议不支持
    UnsupportedProtocol { protocol: u8 },

    /// 目的地址不可达
    DestinationUnreachable { addr: Ipv4Addr },

    /// 重组超时
    ReassemblyTimeout { id: u16 },
}
```

```rust
// 注：当前版本不支持分片和重组，因此不定义 FragmentInfo 和 ReassemblyEntry 结构体
// 收到 MF=1 或 Fragment Offset 非 0 的数据报直接丢弃
```

---

## 6. 与其他模块的交互

### 6.1 与 Common 模块的交互

**[src/common/packet.rs](../src/common/packet.rs)**
- 使用 `Packet` 结构承载 IP 数据报
- `Packet::data()` 获取 IP 首部 + 数据
- `Packet::len()` 获取数据报总长度

**[src/common/error.rs](../src/common/error.rs)**
- `CoreError`：基础错误类型
- IP 模块定义 `Ipv4Error`，实现 `From<Ipv4Error> for CoreError`

**[src/common/addr.rs](../src/common/addr.rs)**
- `Ipv4Addr`：IP 地址类型（32 位网络序）
- 用于源地址和目的地址字段

### 6.2 与 Interface 模块的交互

**[src/interface/iface.rs](../src/interface/iface.rs)**
- **接口配置**：获取接口的 IP 地址、子网掩码、MTU
- **地址匹配**：检查数据报目的地址是否为本机接口地址
- **MTU 查询**：判断是否需要分段

**关键交互：**
```rust
// 检查目的地址是否为本机（依赖注入模式）
fn is_local_address(
    dest_addr: Ipv4Addr,
    interfaces: &Arc<Mutex<InterfaceManager>>
) -> bool {
    interfaces
        .lock()
        .unwrap()
        .get_interface_by_ip(dest_addr)
        .is_some()
}

// 获取接口 MTU
let mtu = context
    .interfaces
    .lock()
    .unwrap()
    .get_interface_by_ip(source_addr)?
    .mtu;
```

**说明**：IP 模块通过 `SystemContext` 获取 `Arc<Mutex<InterfaceManager>>` 的引用来访问接口信息，而非使用全局状态。

### 6.3 与协议模块的交互

**上层协议（通过 Protocol 字段多路分解）：**

| 协议 | 模块 | 提交接口 | 状态 |
|------|------|---------|------|
| ICMP (1) | [src/protocols/icmp/](../src/protocols/icmp/) | `icmp::handle_packet(data, src_addr, dest_addr)` | 已实现 |
| TCP (6) | [src/protocols/tcp/](../src/protocols/tcp/) | - | 未实现 |
| UDP (17) | [src/protocols/udp/](../src/protocols/udp/) | - | 未实现 |

**注：当前版本仅支持 ICMP 协议，收到其他协议类型的数据报应返回 ICMP 协议不可达消息。**

**下层协议（封装）：**
- [src/protocols/ethernet/](../src/protocols/ethernet/)：将以太网帧数据部分解析为 IP 数据报
- [src/protocols/vlan/](../src/protocols/vlan/)：VLAN 标签后承载 IP

### 6.4 与 Engine/Processor 的交互

**[src/engine/processor.rs](../src/engine/processor.rs)**

IP 层在协议处理流程中的位置：

```
[以太网接收] → [VLAN 处理] → [IP 处理] → [上层协议多路分解]
                     ↑              ↓
                 [IP 解析]      [TCP/UDP/ICMP]
```

**处理流程：**
```rust
// 在 Processor::process_packet() 中
if ethertype == EtherType::Ipv4 {
    let packet = ipv4::parse(packet.data())?;
    ipv4::handle(packet)?;
}
```

### 6.5 数据流示例

**上游（接收）流程：**
```
1. Injector → RxQ
2. Scheduler 从 RxQ 取出 Packet
3. Processor 解析以太网首部
4. Processor 解析 VLAN（如有）
5. Processor 解析 IPv4 首部
6. IPv4 模块验证校验和、检查目的地址
7. IPv4 模块检查分片标志（MF 或 Offset 非 0 则丢弃）
8. IPv4 模块根据 Protocol 字段分发到上层（仅支持 ICMP）
9. ICMP 模块处理数据
```

**下游（发送）流程：**
```
1. ICMP 协议构造报文
2. IPv4 模块封装：添加 IPv4 首部
3. IPv4 模块检查 MTU（不支持分片，超过 MTU 则丢弃）
4. IP 数据报添加以太网首部
5. 发送到 TxQ
```

### 6.6 模块初始化顺序

```
1. Common 模块初始化
2. Interface 模块初始化（配置接口 IP 地址）
3. SystemContext 创建（包含 Arc<Mutex<InterfaceManager>>, Arc<Mutex<ArpCache>>）
4. IPv4 模块初始化（配置参数，通过 SystemContext 访问接口）
5. Engine/Processor 初始化（注册 IPv4 处理器，接收 SystemContext 引用）
6. Scheduler 启动（接收 SystemContext 引用）
```

---

## 7. 安全考虑

### 7.1 主要安全威胁

#### 7.1.1 IP 欺骗 (IP Spoofing)

**攻击方式：**
- 攻击者伪造源 IP 地址发送数据报
- 用于隐藏攻击源、绕过访问控制
- 可能导致反射攻击、DDoS 放大

**攻击影响：**
- 接收方无法验证数据报真实来源
- 可能被用于拒绝服务攻击
- 可能绕过基于 IP 的认证

**防御措施：**
- **入口过滤 (Ingress Filtering)**：在边缘路由器过滤源地址不符合预期的数据报（RFC 2827）
- **出口过滤 (Egress Filtering)**：防止内部网络发送伪造源地址的数据报
- **uRPF (Unicast Reverse Path Forwarding)**：验证源地址是否可从接收接口路由到达

#### 7.1.2 分片攻击

**注：当前版本不支持分片和重组，因此不受分片攻击影响。收到任何分片数据报（MF=1 或 Fragment Offset 非 0）直接丢弃。**

#### 7.1.3 TTL 攻击

**攻击方式：**
- **Traceroute 探测**：利用 TTL 超时返回 ICMP 消息探测网络拓扑
- **TTL 消耗**：通过大量数据报消耗中间路由器资源

**防御措施：**
- **限速**：对 ICMP Time Exceeded 消息限速
- **TTL 随机化**：某些应用使用随机初始 TTL 值

### 7.2 实现建议

1. **校验和验证**：必须验证接收数据报的校验和，丢弃错误数据报
2. **严格的长度检查**：验证 Total Length 与实际接收的数据长度一致
3. **分片拒绝**：收到 MF=1 或 Fragment Offset 非 0 的数据报直接丢弃
4. **协议限制**：仅支持 ICMP（Protocol=1），其他协议返回 ICMP 协议不可达
5. **选项字段处理**：谨慎处理 IP 选项，验证长度和格式
6. **TTL 初始值**：使用合理的默认值（64），避免硬编码
7. **速率限制**：对 ICMP 错误消息限速，防止被利用进行反射攻击
8. **日志记录**：记录异常情况（版本错误、校验和错误、协议不支持）

---

## 8. 配置参数

```rust
/// IPv4 协议配置参数
pub struct Ipv4Config {
    /// 默认 TTL 值
    pub default_ttl: u8,  // 默认: 64

    /// 最小 MTU（RFC 规定至少 576 字节）
    pub min_mtu: u16,  // 默认: 576

    /// 默认 MTU（标准以太网）
    pub default_mtu: u16,  // 默认: 1500

    /// 是否验证校验和
    pub verify_checksum: bool,  // 默认: true

    /// 是否处理 IP 选项
    pub process_options: bool,  // 默认: true

    /// ICMP 错误消息速率限制（每秒）
    pub icmp_error_rate_limit: u32,  // 默认: 100
}

impl Default for Ipv4Config {
    fn default() -> Self {
        Self {
            default_ttl: 64,
            min_mtu: 576,
            default_mtu: 1500,
            verify_checksum: true,
            process_options: true,
            icmp_error_rate_limit: 100,
        }
    }
}

// 注：当前版本不支持分片和重组，因此不包含 allow_fragmentation、df_flag、
// reassembly_timeout、max_reassembly_entries、max_fragments_per_datagram 等参数
```

---

## 9. 测试场景

### 9.1 基本功能测试

1. **正常数据报接收**
   - 发送完整的 IPv4 数据报到本机地址
   - 验证正确解析首部字段
   - 验证正确分发到上层协议

2. **TTL 递减测试**
   - 转发数据报时 TTL 正确递减
   - TTL=1 时再转发应变为 0 并触发超时

3. **协议多路分解**
   - 发送 Protocol=1 → 应分发到 ICMP
   - 发送 Protocol=6 → 应返回 ICMP 协议不可达
   - 发送 Protocol=17 → 应返回 ICMP 协议不可达

### 9.2 边界情况测试

1. **最小/最大长度**
   - 发送 20 字节数据报（仅首部）
   - 发送 65535 字节数据报（最大长度）

2. **首部边界**
   - IHL=5（最小首部 20 字节）
   - IHL=15（最大首部 60 字节）

3. **TTL 边界**
   - TTL=1 接收后转发应超时
   - TTL=255 应允许转发

### 9.3 分段相关测试

1. **分片拒绝测试**
   - 发送 MF=1 的数据报 → 应丢弃并发送 ICMP 参数错误
   - 发送 Fragment Offset 非 0 的数据报 → 应丢弃并发送 ICMP 参数错误

**注：当前版本不支持分片和重组，收到任何分片数据报应直接丢弃。**

### 9.4 异常情况测试

1. **校验和错误**
   - 发送错误校验和的数据报
   - 应静默丢弃

2. **版本错误**
   - 发送 Version≠4 的数据报
   - 应丢弃

3. **首部长度异常**
   - 发送 IHL<5 的数据报
   - 应丢弃

4. **长度不匹配**
   - Total Length 字段与实际数据不符
   - 应丢弃

5. **重叠分片**
   - 发送偏移量重叠的分片
   - 应检测并处理

### 9.5 性能测试

1. **高速率接收**
   - 发送大量 ICMP Echo Request 数据报
   - 验证正确响应 Echo Reply

2. **ICMP 处理**
   - 发送各种 ICMP 类型消息
   - 验证正确响应和处理

---

## 10. 参考资料

1. **RFC 791** - Internet Protocol (DARPA Internet Program Protocol Specification)
2. **RFC 792** - Internet Control Message Protocol (ICMP)
3. **RFC 815** - IP Datagram Reassembly Algorithms
4. **RFC 950** - Internet Standard Subnetting Procedure
5. **RFC 1122** - Requirements for Internet Hosts -- Communication Layers
6. **RFC 1519** - Classless Inter-Domain Routing (CIDR)
7. **RFC 1812** - Requirements for IP Version 4 Routers
8. **RFC 2460** - Internet Protocol, Version 6 (IPv6) Specification
9. **RFC 3022** - Traditional IP Network Address Translator (Traditional NAT)
10. **RFC 6810** - The IPv4 Variable Length Mask Option

---

*文档版本：v1.0*
*生成日期：2026-02-19*
*CoreNet 项目 - IPv4 协议设计*
