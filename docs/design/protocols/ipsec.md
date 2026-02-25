# IPsec (IP Security) 协议详细设计文档

## 1. 协议概述

### 1.1 背景与历史

**协议全称和定义：**
- **IPsec (Internet Protocol Security)**：互联网协议安全套件
- 在 TCP/IP 协议栈中的层级位置：网络层 (IP 层)
- 核心功能概述：提供 IP 层的认证、加密和密钥管理服务

**为什么需要 IPsec？**

IPsec 解决了 IP 协议本身缺乏安全机制的问题：
1. **数据机密性**：防止数据在传输过程中被窃听
2. **数据完整性**：防止数据在传输过程中被篡改
3. **数据源认证**：验证数据包的发送者身份
4. **抗重放攻击**：防止攻击者截获并重放有效的数据包
5. **访问控制**：通过安全策略控制哪些流量可以被保护

**历史背景：**
- **1993-1995**：IPsec 最初开发，作为 IPv6 的强制要求
- **1998年**：第一代标准发布 (RFC 2401, 2402, 2406, 2407, 2408, 2409)
- **2005年**：IPsec 标准重新修订，发布现代化版本
  - RFC 4301: Security Architecture (替代 RFC 2401)
  - RFC 4302: Authentication Header - AH (替代 RFC 2402)
  - RFC 4303: Encapsulating Security Payload - ESP (替代 RFC 2406)
  - RFC 4304: Extended Sequence Number (ESN) for ESP
  - RFC 4305: Cryptographic Algorithm Implementation Requirements
  - RFC 4306: IKEv2 (后被 RFC 5996 和 RFC 7296 替代)
- **2014年**：IKEv2 最终标准 RFC 7296 发布
- **当前状态**：IPsec 广泛应用于 VPN、站点间互联、远程访问等场景

### 1.2 设计原理

IPsec 采用**组件化架构**，由三个核心组件构成：

```
                    IPsec 架构
                         |
    +--------------------+--------------------+
    |                    |                    |
    AH                  ESP               IKEv2
    (认证)             (加密+认证)         (密钥交换)
    协议号: 51          协议号: 50         UDP: 500/4500
    |                    |                    |
    v                    v                    v
+---------+         +---------+         +-------------+
| 完整性 |         | 加密 +   |         | 自动密钥    |
| + 认证 |         | 完整性   |         | 管理        |
+---------+         +---------+         +-------------+
```

**两种工作模式：**

```
传输模式 (Transport Mode)              隧道模式 (Tunnel Mode)
+------------------+                   +------------------+
| 原 IP 头         |                   | 新 IP 头         |
+------------------+                   +------------------+
| IPsec 头         |                   | IPsec 头         |
+------------------+                   +------------------+
| 原数据 (加密)    |                   | 原 IP 头 (加密)  |
+------------------+                   +------------------+
| IPsec 尾          |                   | 原数据 (加密)    |
+------------------+                   +------------------+
                                      | IPsec 尾          |
                                      +------------------+

适用: 端到端通信                        适用: VPN 网关
```

**关键特点：**
1. **网络层保护**：对所有上层协议 (TCP/UDP/ICMP 等) 透明
2. **灵活的协议组合**：AH、ESP 可单独使用或组合使用
3. **两种模式**：传输模式保护 IP 载荷，隧道模式保护整个 IP 包
4. **密钥管理**：支持手动配置和 IKE 自动密钥管理
5. **安全策略**：通过 SPD (Security Policy Database) 控制流量处理

---

## 2. 报文格式

### 2.1 Authentication Header (AH) 格式

AH 协议号: 51

```
 0                   1                   2                   3
 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
| Next Header   |  Payload Len  |          RESERVED             |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|                 Security Parameter Index (SPI)               |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|                    Sequence Number Field                     |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|                                                               |
+                 Integrity Check Value (ICV)                  +
|                                                               |
|                                                               |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
```

### 2.2 Encapsulating Security Payload (ESP) 格式

ESP 协议号: 50

```
 0                   1                   2                   3
 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|               Security Parameter Index (SPI)                 |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|                      Sequence Number                         |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|                    Payload Data* (variable)                  |
~                                                               ~
|                                                               |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|                      Padding (0-255 bytes)                   |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|  Pad Length   | Next Header   |                               |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|                 Integrity Check Value (ICV)*                 |
~                                                               ~
|                                                               |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
```

### 2.3 字段说明

#### AH 字段说明

| 字段 | 大小 | 说明 | 常见值 |
|------|------|------|--------|
| Next Header | 1 字节 | 紧跟 AH 头的协议类型 | 6(TCP), 17(UDP), 58(ICMPv6) |
| Payload Len | 1 字节 | AH 头长度（以 32 位字为单位，减 2） | 4 (24 字节头) |
| RESERVED | 2 字节 | 保留字段，必须为 0 | 0 |
| SPI | 4 字节 | 安全参数索引，标识 SA | 任意 32 位值 |
| Sequence Number | 4 字节 | 单调递增计数器，防重放 | 从 1 开始 |
| ICV | 可变 | 完整性校验值 | HMAC-MD5: 12 字节, HMAC-SHA1: 12 字节 |

#### ESP 字段说明

| 字段 | 大小 | 说明 | 常见值 |
|------|------|------|--------|
| SPI | 4 字节 | 安全参数索引，标识 SA | 任意 32 位值 |
| Sequence Number | 4 字节 | 单调递增计数器，防重放 | 从 1 开始 |
| Payload Data | 可变 | 加密的数据（包括协议头） | - |
| Padding | 0-255 字节 | 填充，使加密块对齐 | 0-255 字节 |
| Pad Length | 1 字节 | 填充长度 | 0-255 |
| Next Header | 1 字节 | 封装的协议类型 | 6(TCP), 17(UDP), 4(IPv4) |
| ICV | 可变 | 完整性校验值（可选） | HMAC-SHA1: 12 字节 |

**最小/最大报文长度：**
- AH 最小: 12 字节 (无 ICV)，实际: 24 字节 (ICV 至少 12 字节)
- ESP 最小: 10 字节 (无加密数据、无填充、无 ICV)
- ESP 典型: 取决于加密算法块大小

### 2.4 封装格式

#### IPv4 传输模式

```
原 IP 包:
+-------------+-------+
| IP 头        | 数据  |
+-------------+-------+

AH 处理后:
+-------------+-------+-------+
| IP 头        | AH    | 数据  | (ICV 覆盖整个包)
+-------------+-------+-------+
协议号 = 51

ESP 处理后:
+-------------+-------+-------+-------+
| IP 头        | ESP   | 数据  | ESP   |
|              | 头    | (加密)| 尾    |
+-------------+-------+-------+-------+
协议号 = 50
```

#### IPv4 隧道模式

```
原 IP 包:
+-------------+-------+
| 内 IP 头     | 数据  |
+-------------+-------+

AH 处理后:
+-------------+-------+-------------+-------+
| 外 IP 头     | AH    | 内 IP 头    | 数据  |
+-------------+-------+-------------+-------+
协议号 = 51

ESP 处理后:
+-------------+-------+-------------+-------+-------+
| 外 IP 头     | ESP   | 内 IP 头    | 数据  | ESP   |
|              | 头    | (加密)             | 尾    |
+-------------+-------+-------------+-------+-------+
协议号 = 50
```

#### IPv6 封装

IPv6 使用扩展头链，AH/ESP 作为扩展头插入：
- AH: 扩展头类型 = 51
- ESP: 扩展头类型 = 50
- 位置: 在路由/分段头之后，在上层协议头之前

---

## 3. 安全关联 (Security Association)

### 3.0 SA 状态变量

IPsec 核心是安全关联 (SA)，每个 SA 维护以下状态变量：

| 变量名称 | 类型 | 用途 | 初始值 |
|---------|------|------|--------|
| SPI | u32 | 安全参数索引，标识 SA | 任意非零值 |
| 源地址 | IpAddr | SA 的发起者地址 | - |
| 目的地址 | IpAddr | SA 的接收者地址 | - |
| 协议 | u8 | AH (51) 或 ESP (50) | 51 或 50 |
| 模式 | Mode | 传输或隧道 | Transport |
| 序列号 | u64 | 发送序列号 (ESN) | 1 |
| 重放窗口 | [u64; N] | 抗重放滑动窗口 | 全零 |
| 加密算法 | Cipher | ESP 加密算法 | AES-CBC |
| 加密密钥 | [u8] | 加密密钥 | IKE 协商 |
| 认证算法 | Auth | 认证算法 | HMAC-SHA1 |
| 认证密钥 | [u8] | 认证密钥 | IKE 协商 |
| 生命周期 | Duration | SA 有效期 | IKE 协商 |

### 3.1 SA 状态定义

SA 本身是状态记录，但不进行状态转换。SA 的生命周期：

```
      创建 (IKE 协商/手动配置)
           |
           v
       +-------+
       |  活跃  |  <-- 正常处理数据包
       +-------+
           |
      生命周期到期 / 密钥耗尽
           |
           v
       +-------+
       |  过期  |  <-- 不再处理新数据包
       +-------+
           |
           v
         删除
```

### 3.2 SA 数据库 (SAD)

#### 3.2.1 安全关联数据库 (SAD)

**用途：** 存储所有活动的 SA，用于查找和处理数据包

**SA 查找三元组 (SPI, 目的地址, 协议)：**

**关键操作：**
- **查询(键)**: 通过 (SPI, 目的地址, 协议) 三元组查找 SA
- **添加**: IKE 协商成功后添加新 SA
- **更新**: 更新序列号、重放窗口
- **删除**: SA 过期或删除时移除

#### 3.2.2 安全策略数据库 (SPD)

**用途：** 决定流量如何处理（丢弃、绕过、应用 IPsec）

**策略条目结构：**

| 字段 | 说明 |
|------|------|
| 选择器 | 流量匹配条件（源/目的地址、端口、协议） |
| 处理动作 | DISCARD / BYPASS / APPLY |
| SA 引用 | 指向使用的 SA |

**关键操作：**
- **查询(数据包)**: 根据包头信息查找匹配策略
- **添加**: 管理员配置或 IKE 添加策略
- **删除**: 管理员删除或 IKE 删除策略

---

## 4. 报文处理逻辑

### 4.0 定时器

IPsec SA 管理使用的定时器：

| 定时器名称 | 启动条件 | 超时时间 | 超时动作 |
|-----------|---------|---------|---------|
| SA 生命周期定时器 | SA 创建 | IKE 协商值 | 标记 SA 过期 |
| 重协商提前触发定时器 | SA 创建 | 生命周期 90% | 触发 IKE 重协商 |
| DPD 死对等检测定时器 | IKE SA 创建 | 10-30 秒 | 发送 DPD 探测 |
| 重放窗口清理定时器 | SA 创建 | 1 分钟 | 清理旧的重放窗口位 |

### 4.1 AH 接收处理总流程

```
收到 AH 数据包
      |
      v
+-------------+
| 1. 查找 SA  | -> 失败: 丢弃
+-------------+  (通过 SPI, 目的地址)
      |
      v
+-------------+
| 2. 验证 ICV | -> 失败: 丢弃
+-------------+
      |
      v
+-------------+
| 3. 检查重放 | -> 失败: 丢弃
+-------------+
      |
      v
+-------------+
| 4. 更新状态 | (序列号、重放窗口)
+-------------+
      |
      v
+-------------+
| 5. 去除 AH 头 |
+-------------+
      |
      v
+-------------+
| 6. 提交上层处理 |
+-------------+
```

### 4.2 ESP 接收处理流程

```
收到 ESP 数据包
      |
      v
+-------------+
| 1. 查找 SA  | -> 失败: 丢弃
+-------------+  (通过 SPI, 目的地址)
      |
      v
+-------------+
| 2. 验证 ICV | (如果配置了认证)
+-------------+  -> 失败: 丢弃
      |
      v
+-------------+
| 3. 检查重放 | -> 失败: 丢弃
+-------------+
      |
      v
+-------------+
| 4. 解密数据 | -> 失败: 丢弃
+-------------+
      |
      v
+-------------+
| 5. 去除填充 | 检查 Pad Length
+-------------+
      |
      v
+-------------+
| 6. 更新状态 | (序列号、重放窗口)
+-------------+
      |
      v
+-------------+
| 7. 去除 ESP 头尾 |
+-------------+
      |
      v
+-------------+
| 8. 提交上层处理 |
+-------------+
```

### 4.3 AH 发送处理流程

```
需要发送数据包
      |
      v
+-------------+
| 1. 查找 SPD | 确定处理策略
+-------------+
      |
      v
+-------------+
| 2. 查找 SA  | -> 无 SA: 触发 IKE 或丢弃
+-------------+
      |
      v
+-------------+
| 3. 构造 AH 头 | (SPI, 序列号)
+-------------+
      |
      v
+-------------+
| 4. 计算 ICV | (对整个包计算 HMAC)
+-------------+
      |
      v
+-------------+
| 5. 更新状态 | (序列号 +1)
+-------------+
      |
      v
+-------------+
| 6. 发送数据包 |
+-------------+
```

### 4.4 ESP 发送处理流程

```
需要发送数据包
      |
      v
+-------------+
| 1. 查找 SPD | 确定处理策略
+-------------+
      |
      v
+-------------+
| 2. 查找 SA  | -> 无 SA: 触发 IKE 或丢弃
+-------------+
      |
      v
+-------------+
| 3. 构造 ESP 头 | (SPI, 序列号)
+-------------+
      |
      v
+-------------+
| 4. 填充数据 | (加密块对齐)
+-------------+
      |
      v
+-------------+
| 5. 加密数据 |
+-------------+
      |
      v
+-------------+
| 6. 计算 ICV | (如果配置了认证)
+-------------+
      |
      v
+-------------+
| 7. 更新状态 | (序列号 +1)
+-------------+
      |
      v
+-------------+
| 8. 发送数据包 |
+-------------+
```

### 4.5 IKEv2 报文处理

IKEv2 使用 UDP 端口 500 (NAT 穿越时使用 4500)

#### IKE SA 初始化交换

```
发起者 (I)                          响应者 (R)
    |                                   |
    |  IKE_SA_INIT 请求 (HDR, SAi1, KEi, Ni)  -->
    |                                   |
    |                                   |  1. 生成 DH 密钥
    |                                   |  2. 选择算法套件
    |                                   |  3. 生成 Nr
    |  <--  IKE_SA_INIT 响应 (HDR, SAr1, KEr, Nr)
    |                                   |
    |  1. 生成 DH 密钥                    |
    |  2. 计算共享密钥                    |
    |                                   |
```

#### IKE AUTH 交换

```
发起者 (I)                          响应者 (R)
    |                                   |
    |  IKE_AUTH 请求 (HDR, SK {IDi, AUTH, SAi2, TSi, TSr})  -->
    |                                   |
    |                                   |  1. 验证 AUTH
    |                                   |  2. 检查 TSi/TSr
    |  <--  IKE_AUTH 响应 (HDR, SK {IDr, AUTH, SAr2, TSi, TSr})
    |                                   |
    |  1. 验证 AUTH                      |
    |  2. IKE SA 建立                    |  2. IKE SA 建立
    |                                   |
```

#### CREATE_CHILD_SA 交换

```
发起者 (I)                          响应者 (R)
    |                                   |
    |  CREATE_CHILD_SA 请求 (HDR, SK {SA, Ni, KEi, TSi, TSr})  -->
    |                                   |
    |  <--  CREATE_CHILD_SA 响应 (HDR, SK {SA, Nr, KEr, TSi, TSr})
    |                                   |
    |  Child SA (IPsec SA) 建立         |
```

---

## 5. 核心数据结构

### 5.0 IPsec 表项和缓存

| 资源名称 | 用途 | 最大容量 | 淘汰策略 |
|---------|------|---------|---------|
| SAD | 存储活动 SA | 系统配置 | 生命周期到期 / 手动删除 |
| SPD | 存储安全策略 | 系统配置 | 手动删除 |
| 重放窗口 | 抗重放保护 | 64 或 1024 位 | 滑动窗口 |

### 5.1 AH 报文结构

```rust
/// AH 报文头
#[derive(Debug, Clone, Copy)]
pub struct AhHeader {
    /// 紧跟 AH 头的协议类型
    pub next_header: u8,
    /// AH 头长度 (32 位字为单位，减 2)
    pub payload_len: u8,
    /// 安全参数索引
    pub spi: u32,
    /// 序列号
    pub sequence_number: u32,
}

/// AH 完整性校验值 (ICV)
pub struct Icv {
    /// ICV 数据 (HMAC-MD5: 12 字节, HMAC-SHA1: 12 字节)
    pub data: Vec<u8>,
}
```

### 5.2 ESP 报文结构

```rust
/// ESP 报文头
#[derive(Debug, Clone, Copy)]
pub struct EspHeader {
    /// 安全参数索引
    pub spi: u32,
    /// 序列号
    pub sequence_number: u32,
}

/// ESP 报文尾
pub struct EspTrailer {
    /// 填充长度
    pub pad_length: u8,
    /// 下一个头部
    pub next_header: u8,
    /// 填充数据
    pub padding: Vec<u8>,
}

/// ESP 完整性校验值
pub struct EspIcv {
    /// ICV 数据
    pub data: Vec<u8>,
}
```

### 5.3 SA 数据结构

```rust
/// 安全关联方向
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SaDirection {
    /// 入站 SA
    Inbound,
    /// 出站 SA
    Outbound,
}

/// IPsec 模式
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IpsecMode {
    /// 传输模式
    Transport,
    /// 隧道模式
    Tunnel,
}

/// IPsec 协议
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IpsecProtocol {
    /// AH (协议号 51)
    Ah = 51,
    /// ESP (协议号 50)
    Esp = 50,
}

/// 安全关联
pub struct SecurityAssociation {
    /// SA 方向
    pub direction: SaDirection,
    /// SPI
    pub spi: u32,
    /// 源地址
    pub src_addr: IpAddr,
    /// 目的地址
    pub dst_addr: IpAddr,
    /// IPsec 协议
    pub protocol: IpsecProtocol,
    /// IPsec 模式
    pub mode: IpsecMode,
    /// 发送序列号
    pub tx_sequence: u64,
    /// 重放窗口 (ESN 使用 64 位窗口)
    pub replay_window: BitArray<64>,
    /// 加密算法
    pub cipher: Option<CipherTransform>,
    /// 加密密钥
    pub cipher_key: Option<Vec<u8>>,
    /// 认证算法
    pub auth: AuthTransform,
    /// 认证密钥
    pub auth_key: Vec<u8>,
    /// SA 创建时间
    pub created: Instant,
    /// SA 生命周期 (秒)
    pub lifetime: Duration,
    /// 隧道模式下的源地址
    pub tunnel_src_addr: Option<IpAddr>,
    /// 隧道模式下的目的地址
    pub tunnel_dst_addr: Option<IpAddr>,
}
```

### 5.4 策略数据结构

```rust
/// 策略动作
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PolicyAction {
    /// 丢弃数据包
    Discard,
    /// 绕过 IPsec 处理
    Bypass,
    /// 应用 IPsec (需要 SA)
    Apply,
}

/// 流量选择器
pub struct TrafficSelector {
    /// 源地址范围
    pub src_addr_range: AddrRange,
    /// 目的地址范围
    pub dst_addr_range: AddrRange,
    /// 上层协议 (0 表示任意)
    pub upper_layer_protocol: u8,
    /// 源端口范围
    pub src_port_range: Range<u16>,
    /// 目的端口范围
    pub dst_port_range: Range<u16>,
}

/// 安全策略
pub struct SecurityPolicy {
    /// 流量选择器
    pub selector: TrafficSelector,
    /// 策略动作
    pub action: PolicyAction,
    /// 引用的 SA (仅 Apply 时有效)
    pub sa_ref: Option<u32>,
}
```

### 5.5 加密算法

```rust
/// 加密变换 (用于 ESP)
pub enum CipherTransform {
    /// AES-CBC (RFC 3602)
    AesCbc { key_size: usize },
    /// AES-CTR (RFC 3686)
    AesCtr { key_size: usize },
    /// AES-GCM (RFC 4106) - 同时提供加密和认证
    AesGcm { key_size: usize, icv_size: usize },
    /// 3DES-CBC (RFC 2451)
    TripleDesCbc,
}

/// 认证变换 (用于 AH 和 ESP)
pub enum AuthTransform {
    /// HMAC-MD5-96 (RFC 2403)
    HmacMd5,
    /// HMAC-SHA1-96 (RFC 2404)
    HmacSha1,
    /// HMAC-SHA2-256 (RFC 4868)
    HmacSha2_256,
    /// AES-XCBC-MAC-96 (RFC 3566)
    AesXcbc,
}
```

### 5.6 IKEv2 数据结构

```rust
/// IKEv2 头
pub struct IkeHeader {
    /// 发起者 SPI
    pub initiator_spi: [u8; 8],
    /// 响应者 SPI
    pub responder_spi: [u8; 8],
    /// 下一个载荷
    pub next_payload: u8,
    /// 版本
    pub version: u8,
    /// 交换类型
    pub exchange_type: IkeExchangeType,
    /// 标志
    pub flags: IkeFlags,
    /// 消息 ID
    pub message_id: u32,
    /// 长度
    pub length: u32,
}

/// IKEv2 交换类型
pub enum IkeExchangeType {
    /// IKE_SA_INIT
    IkeSaInit = 34,
    /// IKE_AUTH
    IkeAuth = 35,
    /// CREATE_CHILD_SA
    CreateChildSa = 36,
    /// INFORMATIONAL
    Informational = 37,
}

/// IKEv2 载荷类型
pub enum IkePayloadType {
    /// SA (安全关联)
    Sa = 33,
    /// KE (密钥交换)
    Ke = 34,
    /// IDi (发起者标识)
    Idi = 35,
    /// IDr (响应者标识)
    Idr = 36,
    /// AUTH (认证)
    Auth = 39,
    /// TSi (流量选择器-发起者)
    TSi = 44,
    /// TSr (流量选择器-响应者)
    TSr = 45,
}

/// IKE SA 状态
pub struct IkeSa {
    /// 发起者 SPI
    pub initiator_spi: [u8; 8],
    /// 响应者 SPI
    pub responder_spi: [u8; 8],
    /// 角色 (发起者/响应者)
    pub role: IkeRole,
    /// IKE SA 状态
    pub state: IkeSaState,
    /// 共享密钥材料
    pub sk_ai: Vec<u8>,  // 发起者->响应者的加密密钥
    pub sk_ar: Vec<u8>,  // 响应者->发起者的加密密钥
    pub sk_ei: Vec<u8>,  // 发起者->响应者的认证密钥
    pub sk_er: Vec<u8>,  // 响应者->发起者的认证密钥
    /// 本地 SPI 范围
    pub spi_range: Range<u32>,
    /// 消息 ID
    pub message_id: u32,
}

/// IKE SA 状态
pub enum IkeSaState {
    /// 初始状态
    Init,
    /// 等待 IKE_SA_INIT 响应
    WaitInitResponse,
    /// IKE SA 建立完成
    Established,
    /// 等待 IKE_AUTH 响应
    WaitAuthResponse,
}
```

---

## 6. 与其他模块的交互

### 6.1 与 Common 模块的交互

| 模块 | 使用的组件 | 用途 |
|------|-----------|------|
| packet.rs | Packet | IPsec 头解析/封装、载荷处理 |
| error.rs | CoreError | IPsec 错误类型 (SA 不存在、ICV 验证失败) |
| addr.rs | IpAddr, Ipv4Addr, Ipv6Addr | SA 地址、隧道地址 |
| queue.rs | RxQ/TxQ | 加密/解密后的数据包传递 |

### 6.2 与 Interface 模块的交互

| 接口信息 | 用途 |
|---------|------|
| 接口 IP 地址 | SA 的源/目的地址 |
| 接口 MTU | ESP 封装后分片判断 |
| 接口状态 | 接口 down 时清理 SA |

### 6.3 与协议模块的交互

**与 IPv4/IPv6 模块的交互：**
- IPsec 作为 IP 协议号处理 (50, 51)
- 隧道模式下，ESP 封装整个 IP 包，外层 IP 头由 IPsec 添加
- 传输模式下，IPsec 头插入 IP 头和数据之间

**与上层协议 (TCP/UDP/ICMP) 的交互：**
- IPsec 对上层协议透明
- 解密/验证后的数据包提交给对应协议处理

### 6.4 与 Engine/Processor 的交互

**协议分发：**
- 在 processor.rs 中注册 IPsec 协议处理器
- 协议号 50 → ESP 处理
- 协议号 51 → AH 处理

**出站处理：**
- 上层协议封装后，查询 SPD
- 如果策略要求应用 IPsec，先进行 IPsec 封装，再提交 IP 层

### 6.5 模块初始化顺序

```
1. SystemContext 创建
   |
2. 创建 SadManager (SAD)
   |
3. 创建 SpdManager (SPD)
   |
4. 创建 Ikev2Manager (可选)
   |
5. 创建 IpsecProcessor
   |
6. 注册到 PacketProcessor (协议号 50, 51)
```

### 6.6 数据流示例

**入站 (ESP 解密)：**
```
RxQ → IP 层 (解析协议号 50) → ESP 处理器 → 查找 SAD
  → 验证 ICV → 解密 → 去除 ESP 头尾 → 提交给协议号 6 (TCP)
```

**出站 (ESP 加密)：**
```
TCP 封装 → 查询 SPD → 查找 SAD → 添加 ESP 头 → 填充
  → 加密 → 计算 ICV → 添加 ESP 尾 → 提交 IP 层 → TxQ
```

---

## 7. 安全考虑

### 7.1 重放攻击

**攻击方式：**
- 攻击者截获有效的 IPsec 数据包
- 稍后重放该数据包
- 可能导致重复操作或资源耗尽

**防御措施：**
1. **序列号**: 每个 SA 维护单调递增的序列号
2. **重放窗口**: 使用滑动窗口检测重复或过旧的序列号
3. **窗口大小**: 默认 64 个序列号，ESN 扩展到 2^64

```rust
// 重放检测伪代码
fn check_replay(sa: &SA, seq: u64) -> bool {
    if seq > sa.highest_seq {
        return true;  // 新序列号
    }
    let window_offset = sa.highest_seq - seq;
    if window_offset >= WINDOW_SIZE {
        return false;  // 超出窗口，太旧
    }
    return !sa.replay_window[window_offset];  // 检查是否已见过
}
```

### 7.2 密钥管理安全

**威胁：**
- 密钥泄露导致所有流量可解密
- 弱密钥易受暴力破解

**防御措施：**
1. **强密钥**: 使用足够长度的密钥 (AES-128/256)
2. **定期更新**: SA 生命周期限制，定期重协商
3. **完美前向保密 (PFS)**: 使用 DH/ECDH 交换，旧密钥泄露不影响新密钥
4. **密钥分离**: 加密密钥和认证密钥独立

### 7.3 DoS 攻击

**攻击方式：**
- 发送大量伪造的 IPsec 包
- 导致 ICV 验证消耗大量 CPU

**防御措施：**
1. **Cookie 机制**: IKEv2 使用状态Cookie抵御 DoS
2. **限流**: 限制未建立 SA 的流量
3. **先验证后解密**: ESP 先验证 ICV，再解密

### 7.4 实现建议

1. **加密算法优先级**:
   - 推荐: AES-GCM (同时提供加密和认证)
   - 避免: 3DES (安全性不足)、DES (不安全)

2. **密钥长度**:
   - AES: 最少 128 位，推荐 256 位
   - DH/ECDH: 最少 2048 位 (DH) 或 256 位 (ECDH)

3. **NAT 穿越**:
   - 使用 UDP 封装 ESP (端口 4500)
   - 实现 NAT-Discovery

4. **抗重放窗口**:
   - 实现滑动窗口算法
   - 支持扩展序列号 (ESN)

5. **错误处理**:
   - 验证失败静默丢弃，不发送错误消息
   - 记录安全事件日志

---

## 8. 配置参数

```rust
/// IPsec 全局配置
pub struct IpsecConfig {
    // ========== SA 配置 ==========
    /// SA 默认生命周期 (秒)
    pub sa_lifetime_secs: u64,  // 默认: 3600 (1 小时)

    /// SA 最大生命周期 (秒)
    pub sa_lifetime_max_secs: u64,  // 默认: 86400 (24 小时)

    /// 重协商提前触发时间 (生命周期百分比)
    pub rekey_margin_percent: u8,  // 默认: 90 (90% 时触发)

    // ========== 重放保护配置 ==========
    /// 重放窗口大小 (位)
    pub replay_window_size: usize,  // 默认: 64

    /// 是否启用扩展序列号 (ESN)
    pub enable_esn: bool,  // 默认: false

    // ========== 加密算法配置 ==========
    /// 支持的加密算法 (按优先级排序)
    pub cipher_suites: Vec<CipherTransform>,  // 默认: [AesGcm, AesCbc]

    /// 支持的认证算法
    pub auth_suites: Vec<AuthTransform>,  // 默认: [HmacSha2_256, HmacSha1]

    // ========== IKEv2 配置 ==========
    /// 是否启用 IKEv2
    pub enable_ikev2: bool,  // 默认: true

    /// IKEv2 监听端口
    pub ikev2_port: u16,  // 默认: 500

    /// NAT 穿越端口
    pub ikev2_nat_port: u16,  // 默认: 4500

    /// DPD (Dead Peer Detection) 间隔 (秒)
    pub dpd_interval_secs: u64,  // 默认: 30

    /// IKE SA 重协商间隔 (秒)
    pub ike_rekey_interval_secs: u64,  // 默认: 14400 (4 小时)

    // ========== 性能配置 ==========
    /// SAD 最大容量
    pub sad_max_entries: usize,  // 默认: 10000

    /// SPD 最大容量
    pub spd_max_entries: usize,  // 默认: 10000

    /// 单个 SA 最大流量 (字节，0 表示无限制)
    pub sa_max_bytes: u64,  // 默认: 0
}

impl Default for IpsecConfig {
    fn default() -> Self {
        Self {
            sa_lifetime_secs: 3600,
            sa_lifetime_max_secs: 86400,
            rekey_margin_percent: 90,
            replay_window_size: 64,
            enable_esn: false,
            cipher_suites: vec![
                CipherTransform::AesGcm { key_size: 256, icv_size: 16 },
                CipherTransform::AesCbc { key_size: 256 },
            ],
            auth_suites: vec![
                AuthTransform::HmacSha2_256,
                AuthTransform::HmacSha1,
            ],
            enable_ikev2: true,
            ikev2_port: 500,
            ikev2_nat_port: 4500,
            dpd_interval_secs: 30,
            ike_rekey_interval_secs: 14400,
            sad_max_entries: 10000,
            spd_max_entries: 10000,
            sa_max_bytes: 0,
        }
    }
}
```

---

## 9. 测试场景

### 9.1 基本功能测试

#### 1. AH 传输模式测试
- **测试内容**:
  - 发送方添加 AH 头，计算 ICV
  - 接收方验证 ICV，检查重放
  - 数据包正确传递给上层协议

#### 2. ESP 传输模式测试
- **测试内容**:
  - 发送方加密数据，添加 ESP 头尾
  - 接收方解密数据，验证 ICV
  - 数据包正确传递给上层协议

#### 3. ESP 隧道模式测试
- **测试内容**:
  - 发送方封装整个 IP 包
  - 接收方解密，提取内层 IP 包
  - 内层包正确路由

#### 4. IKEv2 SA 建立测试
- **测试内容**:
  - IKE_SA_INIT 交换
  - IKE_AUTH 交换
  - CREATE_CHILD_SA 交换
  - SA 正确创建并可使用

### 9.2 边界情况测试

#### 1. 序列号回绕测试
- **测试内容**:
  - 序列号达到 2^32 (非 ESN) 或 2^64 (ESN)
  - 验证正确处理

#### 2. MTU 分片测试
- **测试内容**:
  - ESP 封装后超过 MTU
  - 验证正确触发 IP 层分片

#### 3. 重放窗口边界测试
- **测试内容**:
  - 序列号刚好在窗口边界
  - 验证正确的接受/拒绝

### 9.3 异常情况测试

#### 1. ICV 验证失败测试
- **测试内容**:
  - 修改载荷数据
  - 验证接收方丢弃数据包

#### 2. SA 不存在测试
- **测试内容**:
  - 收到未知 SPI 的数据包
  - 验证正确丢弃

#### 3. 重放攻击测试
- **测试内容**:
  - 发送重复序列号的数据包
  - 验证被重放窗口检测到

#### 4. 密钥过期测试
- **测试内容**:
  - SA 生命周期到期
  - 验证触发重协商或丢弃数据包

#### 5. DoS 攻击测试
- **测试内容**:
  - 发送大量伪造的 IPsec 包
  - 验证系统资源不被耗尽

### 9.4 性能测试

#### 1. 加密性能测试
- **测试内容**:
  - 测量不同加密算法的吞吐量
  - AES-GCM vs AES-CBC + HMAC

#### 2. SA 查找性能测试
- **测试内容**:
  - 大量 SA 条目下的查找性能
  - 验证哈希表/查找表效率

#### 3. 并发处理测试
- **测试内容**:
  - 多个 SA 并发处理数据包
  - 验证无数据竞争

---

## 10. 参考资料

### 主要 RFC 标准

1. **RFC 4301** - Security Architecture for the Internet Protocol
2. **RFC 4302** - IP Authentication Header (AH)
3. **RFC 4303** - IP Encapsulating Security Payload (ESP)
4. **RFC 4304** - Extended Sequence Number (ESN) for ESP
5. **RFC 4305** - Cryptographic Algorithm Implementation Requirements for ESP and AH
6. **RFC 4306** - Internet Key Exchange (IKEv2) Protocol (被 RFC 5996 替代)
7. **RFC 4307** - Cryptographic Algorithms for Use with IKEv2
8. **RFC 5996** - Internet Key Exchange Protocol Version 2 (IKEv2) (被 RFC 7296 替代)
9. **RFC 7296** - Internet Key Exchange Protocol Version 2 (IKEv2) - **当前标准**
10. **RFC 3947** - Negotiation of NAT-Traversal in the IKE
11. **RFC 3948** - UDP Encapsulation of IPsec ESP Packets

### 算法相关 RFC

12. **RFC 2403** - The Use of HMAC-MD5-96 within ESP and AH
13. **RFC 2404** - The Use of HMAC-SHA-1-96 within ESP and AH
14. **RFC 3602** - The AES-CBC Cipher Algorithm and Its Use with IPsec
15. **RFC 4106** - The Use of AES-GCM in IPsec ESP
16. **RFC 4868** - Using HMAC-SHA-256, HMAC-SHA-384, and HMAC-SHA-512 with IPsec

### 其他参考资料

17. **NIST SP 800-77** - Guide to IPsec VPNs
18. **strongSwan 文档** - 开源 IPsec 实现
19. **Libreswan 文档** - 开源 IPsec 实现
