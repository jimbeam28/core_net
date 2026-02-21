# User Datagram Protocol (UDP) 详细设计文档

## 1. 协议概述

### 1.1 背景与历史

**协议全称和定义：**
- User Datagram Protocol（用户数据报协议）
- 在 TCP/IP 协议栈中的层级位置：传输层（Transport Layer，Layer 4）
- 核心功能概述：提供无连接、不可靠的数据报传输服务

**为什么需要该协议？**

UDP 的设计目的是为应用程序提供一种轻量级的数据传输机制，它不需要建立连接，不保证数据包的顺序到达或可靠传输，但具有以下优势：
- **低延迟**：无需连接建立和拆除的过程
- **简单高效**：头部开销仅 8 字节，远小于 TCP 的 20 字节
- **支持广播和多播**：适合一对多的通信场景
- **应用层控制**：应用程序可以灵活处理丢包和重传

UDP 适用于对实时性要求高、可以容忍少量丢包的应用场景，如 DNS 查询、视频流、在线游戏、VoIP 等。

**历史背景：**
- RFC 768：发布于 1980 年 8 月 28 日，作者 J. Postel（ISI）
- RFC 768 是 Internet 协议族早期的核心文档之一
- 与 TCP（RFC 793）共同构成了传输层的两大协议
- 相关补充 RFC：RFC 1122（主机需求）、RFC 2460（IPv6 中的 UDP）

### 1.2 设计原理

UDP 采用极其简单的设计思想：**最小化协议机制，将复杂性留给应用层**。

UDP 只提供两个核心功能：
1. **多路复用**：通过端口号区分不同的应用程序
2. **错误检测**：通过校验和检测传输过程中的数据损坏

**UDP 工作机制：**

```
应用层 A                    应用层 B
    |                           |
    v                           v
+-------+                   +-------+
| UDP   |                   | UDP   |
+-------+                   +-------+
    |                           |
    v                           v
+-------+                   +-------+
|  IP   |                   |  IP   |
+-------+                   +-------+

发送方：应用数据 → UDP 封装 → IP 封装 → 发送
接收方：接收 → IP 解封装 → UDP 解封装 → 应用数据
```

**关键特点：**
- **无连接**：无需握手，直接发送数据
- **不可靠**：不保证送达、不保证顺序、无拥塞控制
- **轻量级**：头部仅 8 字节，处理开销小
- **面向报文**：保持应用程序的报文边界

---

## 2. 报文格式

### 2.1 报文结构

UDP 报文头部固定为 8 字节，格式如下：

```
 0                   1                   2                   3
 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|          Source Port          |       Destination Port        |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|            Length             |           Checksum            |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|                             Data                              |
|                             ...                               |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
```

### 2.2 字段说明

| 字段 | 大小 | 说明 | 常见值 |
|------|------|------|--------|
| Source Port | 2 字节 | 发送方端口号，可选字段，不使用时为 0 | 1024-65535（动态端口） |
| Destination Port | 2 字节 | 接收方端口号，必填字段 | 0-65535（知名/动态端口） |
| Length | 2 字节 | UDP 数据报总长度（头部+数据），以字节为单位 | 最小值 8（仅头部） |
| Checksum | 2 字节 | 校验和，用于错误检测，IPv4 中可选 | 计算得出的值 |

**最小/最大报文长度：**
- **最小长度**：8 字节（仅头部，无数据）
- **最大长度**：65535 字节（受 IP 数据报长度限制）
- **实际限制**：受底层 MTU 限制，通常不超过 1500 字节

### 2.3 封装格式

**在 IPv4 中的封装：**

```
+-------------------+
|   以太网头部       |
+-------------------+
|     IP 头部       |  协议字段 = 17 (UDP)
+-------------------+
|     UDP 头部      |
+-------------------+
|     UDP 数据      |
+-------------------+
|   以太网尾部       |
+-------------------+
```

**在 IPv6 中的封装：**

```
+-------------------+
|   IPv6 基本头部   |  下一头部字段 = 17 (UDP)
+-------------------+
|   扩展头部（可选） |
+-------------------+
|     UDP 头部      |
+-------------------+
|     UDP 数据      |
+-------------------+
```

---

## 3. 状态机设计

### 3.0 状态变量

UDP 是无状态协议，不需要维护连接状态。以下是实现层需要的状态变量：

| 变量名称 | 类型 | 用途 | 初始值 |
|---------|------|------|--------|
| 无 | - | UDP 本身不维护任何状态变量 | - |

**说明：** UDP 是真正的无状态协议，不维护连接信息、序列号、确认号等状态。所有状态由应用层管理。

### 3.1 状态定义

**UDP 无状态机。**

UDP 不维护任何连接状态，每个 UDP 数据报都是独立处理的。

### 3.2 状态转换详解

**无状态转换。**

UDP 的处理是事件驱动的：
- **发送**：应用层调用发送接口 → UDP 封装 → 传递给 IP 层
- **接收**：从 IP 层接收数据报 → UDP 解封装 → 根据端口号分发给应用层

---

## 4. 报文处理逻辑

### 4.0 定时器

UDP 协议本身不使用任何定时器。

| 定时器名称 | 启动条件 | 超时时间 | 超时动作 |
|-----------|---------|---------|---------|
| 无 | - | - | - |

**说明：** 超时重传、连接保活等功能由应用层或上层协议实现。

### 4.1 接收处理总流程

```
          接收 UDP 数据报
                 |
                 v
        +----------------+
        | 验证数据报长度  |--- 长度错误 ---> 丢弃 + 记录错误
        +----------------+
                 |
                 v
        +----------------+
        |  验证校验和     |--- 校验失败 ---> 丢弃 + 记录错误
        +----------------+
                 |
                 v
        +----------------+
        |  查找目标端口   |--- 端口未绑定 ---> 丢弃 + 发送 ICMP 端口不可达
        +----------------+
                 |
                 v
        +----------------+
        |  提取 UDP 数据   |
        +----------------+
                 |
                 v
        +----------------+
        |  分发给应用层    |
        +----------------+
```

### 4.2 发送处理流程

**处理流程：**

1. **提取信息：**
   - 应用层数据 → 数据载荷
   - 源端口 → 应用层提供的端口号
   - 目标端口 → 应用层提供的目标端口号

2. **处理步骤：**
   - 构建 UDP 头部
   - 计算校验和（包含伪头部）
   - 将 UDP 数据报传递给 IP 层

3. **资源更新：**
   - 无状态更新

4. **响应动作：**
   - 无响应（UDP 是单向发送）

### 4.3 校验和计算

UDP 校验和覆盖范围包括：
1. **UDP 伪头部**（Pseudo-header）：包含 IP 层信息
2. **UDP 头部**
3. **UDP 数据**

**IPv4 伪头部格式：**

```
 0                   1                   2                   3
 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|                       Source Address                          |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|                    Destination Address                        |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|      Zero     |   Protocol    |        UDP Length            |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
```

**计算步骤：**
1. 构建伪头部（源 IP、目标 IP、协议号 17、UDP 长度）
2. 将伪头部、UDP 头部、UDP 数据拼接
3. 按 16 位进行二进制反码求和
4. 取反得到校验和

---

## 5. 核心数据结构

### 5.0 表项/缓存

UDP 协议本身不需要维护表项或缓存。

端口绑定信息由应用层或操作系统管理（在 CoreNet 中可能需要简单的端口映射表）。

| 资源名称 | 用途 | 最大容量 | 淘汰策略 |
|---------|------|---------|---------|
| 端口绑定表 | 将端口号映射到应用层接收器 | 由系统配置决定 | 由应用层管理 |

#### 5.0.1 端口绑定表（可选，由上层管理）

**用途：** 将接收到的 UDP 数据报分发到正确的应用层处理器

**关键操作：**
- 查询：根据端口号查找对应的处理器
- 绑定：应用层注册监听端口
- 解绑：应用层取消监听端口

### 5.1 UDP 头部结构

```rust
/// UDP 头部结构
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UdpHeader {
    /// 源端口号，0 表示未使用
    pub source_port: u16,
    /// 目标端口号
    pub destination_port: u16,
    /// UDP 数据报长度（包含头部）
    pub length: u16,
    /// 校验和
    pub checksum: u16,
}

impl UdpHeader {
    /// UDP 头部固定大小
    pub const HEADER_SIZE: usize = 8;

    /// 从字节流解析 UDP 头部
    pub fn parse(data: &[u8]) -> Result<Self, CoreError> {
        if data.len() < Self::HEADER_SIZE {
            return Err(CoreError::ParseError("UDP header too short".into()));
        }

        let source_port = u16::from_be_bytes([data[0], data[1]]);
        let destination_port = u16::from_be_bytes([data[2], data[3]]);
        let length = u16::from_be_bytes([data[4], data[5]]);
        let checksum = u16::from_be_bytes([data[6], data[7]]);

        Ok(Self {
            source_port,
            destination_port,
            length,
            checksum,
        })
    }

    /// 将头部序列化为字节
    pub fn serialize(&self) -> [u8; Self::HEADER_SIZE] {
        let mut buf = [0u8; Self::HEADER_SIZE];
        buf[0..2].copy_from_slice(&self.source_port.to_be_bytes());
        buf[2..4].copy_from_slice(&self.destination_port.to_be_bytes());
        buf[4..6].copy_from_slice(&self.length.to_be_bytes());
        buf[6..8].copy_from_slice(&self.checksum.to_be_bytes());
        buf
    }
}
```

### 5.2 UDP 数据报结构

```rust
/// UDP 数据报
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UdpDatagram<'a> {
    /// UDP 头部
    pub header: UdpHeader,
    /// UDP 数据载荷
    pub payload: &'a [u8],
}

impl<'a> UdpDatagram<'a> {
    /// 从字节流解析 UDP 数据报
    pub fn parse(data: &'a [u8]) -> Result<Self, CoreError> {
        let header = UdpHeader::parse(data)?;

        // 验证长度
        if header.length < 8 {
            return Err(CoreError::ParseError("Invalid UDP length".into()));
        }

        let payload_len = (header.length as usize) - UdpHeader::HEADER_SIZE;
        if data.len() < payload_len + UdpHeader::HEADER_SIZE {
            return Err(CoreError::ParseError("UDP data too short".into()));
        }

        let payload = &data[UdpHeader::HEADER_SIZE..UdpHeader::HEADER_SIZE + payload_len];

        Ok(Self { header, payload })
    }

    /// 计算 UDP 校验和（包含伪头部）
    pub fn calculate_checksum(
        &self,
        source_ip: Ipv4Addr,
        dest_ip: Ipv4Addr,
    ) -> u16 {
        let mut sum = 0u32;

        // 伪头部
        sum += u32::from(source_ip.to_be());
        sum += u32::from(dest_ip.to_be());
        sum += 17u32; // 协议号
        sum += u32::from(self.header.length);

        // UDP 头部
        sum += u32::from(self.header.source_port);
        sum += u32::from(self.header.destination_port);
        sum += u32::from(self.header.length);

        // 数据
        let mut i = 0;
        while i + 1 < self.payload.len() {
            let word = u16::from_be_bytes([self.payload[i], self.payload[i + 1]]);
            sum += u32::from(word);
            i += 2;
        }

        // 处理奇数字节
        if i < self.payload.len() {
            sum += u32::from(self.payload[i]) << 8;
        }

        // 处理进位
        while sum >> 16 != 0 {
            sum = (sum & 0xFFFF) + (sum >> 16);
        }

        !sum as u16
    }

    /// 验证校验和
    pub fn verify_checksum(
        &self,
        source_ip: Ipv4Addr,
        dest_ip: Ipv4Addr,
    ) -> bool {
        if self.header.checksum == 0 {
            // IPv4 中校验和可选
            return true;
        }

        self.calculate_checksum(source_ip, dest_ip) == self.header.checksum
    }
}
```

### 5.3 UDP 校验和伪头部

```rust
/// UDP 伪头部（用于计算校验和）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UdpPseudoHeader {
    /// 源 IP 地址
    pub source_ip: Ipv4Addr,
    /// 目标 IP 地址
    pub dest_ip: Ipv4Addr,
    /// 协议号（UDP = 17）
    pub protocol: u8,
    /// UDP 长度
    pub udp_length: u16,
}

impl UdpPseudoHeader {
    /// 创建 UDP 伪头部
    pub fn new(
        source_ip: Ipv4Addr,
        dest_ip: Ipv4Addr,
        udp_length: u16,
    ) -> Self {
        Self {
            source_ip,
            dest_ip,
            protocol: 17,
            udp_length,
        }
    }

    /// 伪头部大小
    pub const SIZE: usize = 12;
}
```

---

## 6. 与其他模块的交互

UDP 协议在 CoreNet 项目中的模块交互如下：

### 6.1 与 Common 模块的交互

| 组件 | 交互方式 | 说明 |
|------|---------|------|
| `packet::Packet` | 使用 | UDP 数据报封装在 Packet 中进行传递 |
| `error::CoreError` | 使用 | 报告解析错误和校验错误 |
| `addr::Ipv4Addr` | 使用 | 伪头部校验和计算需要 IP 地址 |
| `addr::Ipv6Addr` | 使用（未来） | IPv6 下的 UDP 伪头部 |

### 6.2 与 Interface 模块的交互

| 组件 | 交互方式 | 说明 |
|------|---------|------|
| Interface 配置 | 读取 | 获取接口的 IP 地址用于校验和计算 |
| MTU | 读取 | UDP 数据报长度不应超过接口 MTU |

### 6.3 与其他协议模块的交互

| 协议 | 交互方向 | 说明 |
|------|---------|------|
| IPv4 | 下层 | UDP 数据报封装在 IPv4 数据报中（协议号 = 17） |
| IPv6 | 下层 | UDP 数据报封装在 IPv6 数据报中（下一头部 = 17） |
| ICMP | 平层 | 端口不可达时生成 ICMP Destination Unreachable 消息 |

**数据流示例：**

```
发送方向：
应用数据 → UDP 封装 → IPv4 封装 → 以太网封装 → 发送

接收方向：
接收 → 以太网解封装 → IPv4 解封装 → UDP 解封装 → 应用数据
                              ↓
                         协议分发 (Protocol = 17)
```

### 6.4 与 Engine/Processor 的交互

在 `src/engine/processor.rs` 中，需要添加 UDP 协议的分发逻辑：

```rust
// 在 IP 层处理后，根据协议号分发
match ip_header.protocol {
    1 => icmp::handle_icmp(packet, context),
    17 => udp::handle_udp(packet, context),  // 新增
    _ => { /* 未知协议 */ }
}
```

### 6.5 与 Scheduler 的交互

Scheduler 负责将数据报从 RxQ 传递到 Processor，Processor 完成协议处理后，UDP 数据报最终分发给应用层（在 CoreNet 中可能是输出到 TxQ 用于验证）。

### 6.6 模块初始化顺序

由于 UDP 是无状态协议，不需要复杂的初始化：

1. 系统启动
2. Interface Manager 初始化（绑定 IP 地址）
3. Engine/Processor 初始化
4. UDP 处理器准备就绪（无需状态初始化）

---

## 7. 安全考虑

### 7.1 放大攻击（Amplification Attack）

**攻击方式：**
- 攻击者伪造源 IP 地址，向服务器发送小请求
- 服务器响应发送到受害者，响应数据远大于请求数据
- 常见于基于 UDP 的服务（如 DNS、NTP、Memcached）

**攻击影响：**
- 受害者接收大量流量，导致带宽耗尽
- 被用作反射攻击的中间人

**防御措施：**
- **响应小于请求**：设计协议时确保响应不超过请求
- **速率限制**：限制对单个源 IP 的响应速率
- **源验证**：启用 ingress/egress 过滤（网络层）
- **随机化端口**：使用不易猜测的源端口

### 7.2 校验和欺骗

**攻击方式：**
- 攻击者构造错误校验和的数据包
- 绕过安全检测或导致系统崩溃

**防御措施：**
- **严格校验**：始终验证校验和，拒绝校验失败的数据包
- **日志记录**：记录校验失败的事件用于监控

### 7.3 端口扫描

**攻击方式：**
- 扫描开放的 UDP 端口寻找服务漏洞
- UDP 端口扫描比 TCP 慢（需要超时判断）

**防御措施：**
- **端口隐藏**：仅暴露必要的端口
- **服务认证**：对 UDP 服务实施认证机制
- **异常检测**：监控异常的端口访问模式

### 7.4 实现建议

1. **校验和强制验证**：在实现中始终验证校验和，即使在 IPv4 中它是可选的
2. **长度验证**：严格检查 UDP 长度字段，防止缓冲区溢出
3. **端口验证**：拒绝发送到知名系统端口（< 1024）的数据报，除非明确需要
4. **速率限制**：对 ICMP 错误响应实施速率限制，避免被利用进行反射攻击
5. **日志记录**：记录异常事件（校验失败、端口不可达等）用于调试和监控

---

## 8. 配置参数

```rust
/// UDP 协议配置
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UdpConfig {
    /// 是否强制验证校验和（IPv4 中可选，默认启用）
    pub enforce_checksum: bool,
    /// 是否在端口不可达时发送 ICMP 消息
    pub send_icmp_unreachable: bool,
    /// 最大 UDP 数据报大小（受 MTU 限制）
    pub max_datagram_size: u16,
    /// 端口绑定表的最大容量（0 表示无限制）
    pub max_port_bindings: usize,
}

impl Default for UdpConfig {
    fn default() -> Self {
        Self {
            enforce_checksum: true,
            send_icmp_unreachable: true,
            max_datagram_size: 1472,  // 1500 (以太网 MTU) - 20 (IP 头部) - 8 (UDP 头部)
            max_port_bindings: 0,     // 无限制
        }
    }
}
```

---

## 9. 测试场景

### 9.1 基本功能测试

1. **头部解析测试**
   - 测试内容：验证从字节流正确解析 UDP 头部各字段
   - 预期结果：源端口、目标端口、长度、校验和正确提取

2. **头部序列化测试**
   - 测试内容：验证 UDP 头部正确序列化为字节
   - 预期结果：序列化后字节与原始数据一致

3. **数据报封装测试**
   - 测试内容：将应用数据封装为 UDP 数据报
   - 预期结果：长度字段正确，数据完整

4. **数据报解封装测试**
   - 测试内容：从 UDP 数据报提取应用数据
   - 预期结果：数据完整无误

### 9.2 边界情况测试

1. **最小长度数据报**
   - 测试内容：处理仅包含 8 字节头部的 UDP 数据报
   - 预期结果：正常解析，载荷为空

2. **最大长度数据报**
   - 测试内容：处理长度为 65535 字节的 UDP 数据报
   - 预期结果：正常解析（在模拟环境中可能受限）

3. **零源端口**
   - 测试内容：处理源端口为 0 的 UDP 数据报
   - 预期结果：正常解析，源端口未使用

4. **奇数长度载荷**
   - 测试内容：处理载荷长度为奇数的 UDP 数据报
   - 预期结果：校验和计算正确处理填充字节

5. **零校验和（IPv4）**
   - 测试内容：处理校验和为 0 的 UDP 数据报（IPv4 允许）
   - 预期结果：根据配置决定是否接受

### 9.3 异常情况测试

1. **长度字段错误**
   - 测试内容：UDP 头部声明的长度与实际数据不符
   - 预期结果：返回解析错误

2. **校验和错误**
   - 测试内容：UDP 数据报校验和计算错误
   - 预期结果：丢弃数据报，记录错误

3. **数据截断**
   - 测试内容：接收到的数据短于 UDP 长度字段声明的长度
   - 预期结果：返回解析错误

4. **端口未绑定**
   - 测试内容：接收到目标端口未绑定的 UDP 数据报
   - 预期结果：丢弃数据报，可选择发送 ICMP 端口不可达

5. **超大载荷**
   - 测试内容：载荷长度超过接口 MTU
   - 预期结果：根据配置决定是否拒绝或分片（IP 层处理）

### 9.4 集成测试

1. **完整发送接收流程**
   - 测试内容：从应用层发送数据，经过 UDP 封装、IP 封装、链路层封装，最终接收并解封装
   - 预期结果：数据完整传输，各层头部正确

2. **多路复用测试**
   - 测试内容：同时向多个端口发送 UDP 数据报
   - 预期结果：各端口独立接收，数据不混淆

3. **ICMP 交互测试**
   - 测试内容：端口不可达时生成 ICMP 消息
   - 预期结果：正确生成 ICMP Destination Unreachable 消息

---

## 10. 参考资料

1. **[RFC 768](https://www.rfc-editor.org/rfc/rfc768)** - User Datagram Protocol
2. **[RFC 1122](https://www.rfc-editor.org/rfc/rfc1122)** - Requirements for Internet Hosts -- Communication Layers（第 4.1 节 UDP）
3. **[RFC 2460](https://www.rfc-editor.org/rfc/rfc2460)** - Internet Protocol, Version 6 (IPv6) Specification（第 8.1 节 UDP）
4. **[RFC 5405](https://www.rfc-editor.org/rfc/rfc5405)** - Unicast UDP Usage Guidelines for Application Designers
5. **[RFC 8085](https://www.rfc-editor.org/rfc/rfc8085)** - UDP Usage Guidelines（更新 RFC 5405）
