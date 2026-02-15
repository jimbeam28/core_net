# VLAN协议详细设计文档

## 概述

VLAN（Virtual Local Area Network，虚拟局域网）模块负责处理带有802.1Q标签的以太网帧的解析和封装。该模块位于链路层，在以太网协议和上层协议（如IPv4、IPv6、ARP）之间提供VLAN标签的插入、提取和处理功能。

**当前阶段目标**：实现基础的802.1Q标签解析和封装，支持单层VLAN标签。

---

## 一、需求介绍

### 1.1 功能需求

- **需求1**：解析带有802.1Q标签的以太网帧，提取VLAN ID、优先级等信息
- **需求2**：为普通以太网帧添加802.1Q标签进行封装
- **需求3**：支持TPID（Tag Protocol Identifier）识别，支持0x8100（标准VLAN）和0x9100（Q-in-Q）等
- **需求4**：处理带有VLAN标签的ARP报文（VLAN-aware ARP）
- **需求5**：支持删除VLAN标签（脱标签操作）
- **需求6**：验证VLAN ID的有效性（1-4094）

### 1.2 非功能需求

- **零依赖**：仅使用Rust标准库
- **纯内存模拟**：在Packet结构上进行内存操作，不涉及真实网络接口
- **性能要求**：解析和封装操作应高效，避免不必要的内存拷贝
- **可读性优先**：代码结构清晰，便于学习VLAN协议原理

---

## 二、架构设计

### 2.1 模块定位

VLAN模块在协议栈中的位置：

```
┌─────────────────────────────────────────────────────────┐
│              Network Layer (网络层)                      │
│  ┌─────────┐  ┌─────────┐  ┌─────────┐  ┌─────────┐  │
│  │  IPv4   │  │  IPv6   │  │  ARP    │  │  其他   │  │
│  └────┬────┘  └────┬────┘  └────┬────┘  └────┬────┘  │
└───────┼────────────┼────────────┼────────────┼─────────┘
        │            │            │            │
┌───────┴────────────┴────────────┴────────────┴─────────┐
│              Link Layer (链路层)                          │
│  ┌──────────────────────────────────────────────────┐  │
│  │               VLAN Module                        │  │
│  │  ┌─────────┐    ┌─────────┐    ┌─────────┐    │  │
│  │  │  Parse   │    │ Encap   │    │ Validate │    │  │
│  │  └─────────┘    └─────────┘    └─────────┘    │  │
│  └──────────────────────────────────────────────────┘  │
│  ┌──────────────────────────────────────────────────┐  │
│  │              Ethernet Module                     │  │
│  └──────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────┘
```

### 2.2 数据流向

**上行（解析）流程**：
```
以太网帧(带VLAN) -> VLAN解析 -> 去除标签 -> 上层协议处理
                   提取VLAN ID
                   提取PCP/DEI
```

**下行（封装）流程**：
```
上层协议数据 -> 添加VLAN标签 -> 计算新EtherType -> 以太网封装
               设置VLAN ID
               设置PCP/DEI
```

### 2.3 处理模型

```
┌──────────────────────────────────────────────────────────┐
│                    VLAN处理流程                          │
├──────────────────────────────────────────────────────────┤
│                                                         │
│  接收以太网帧                                            │
│       │                                                  │
│       v                                                  │
│  检查EtherType                                          │
│       │                                                  │
│       ├─> 0x8100 (标准802.1Q) ──> 解析VLAN标签            │
│       ├─> 0x9100 (Q-in-Q) ──────> 解析双层VLAN           │
│       ├─> 0x88A8 (802.1ad) ─────> 解析Provider Bridge     │
│       └─> 其他 ────────────────> 无VLAN标签，直接上交     │
│                                                         │
└──────────────────────────────────────────────────────────┘
```

---

## 三、核心数据结构

### 3.1 VlanTag

VLAN标签结构，表示802.1Q标签的内容。

```rust
/// 802.1Q VLAN标签
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VlanTag {
    /// 优先级代码点 (Priority Code Point), 3 bits, 范围 0-7
    pub pcp: u8,

    /// 丢弃指示 (Drop Eligible Indicator), 1 bit
    pub dei: bool,

    /// VLAN标识符 (VLAN Identifier), 12 bits, 范围 0-4095
    /// 有效范围: 1-4094 (0保留，4095预留)
    pub vid: u16,
}
```

### 3.2 VlanFrame

VLAN帧封装信息，用于构建带VLAN标签的以太网帧。

```rust
/// VLAN帧封装信息
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VlanFrame {
    /// VLAN标签
    pub tag: VlanTag,

    /// 标签协议标识符 (Tag Protocol Identifier)
    /// 0x8100: 标准802.1Q
    /// 0x9100: Q-in-Q
    /// 0x88A8: 802.1ad Provider Bridge
    pub tpid: u16,
}
```

### 3.3 VlanError

VLAN处理错误类型。

```rust
/// VLAN处理错误
#[derive(Debug)]
pub enum VlanError {
    /// 无效的VLAN ID
    InvalidVlanId { vid: u16 },

    /// 无效的PCP值 (超过7)
    InvalidPcp { pcp: u8 },

    /// 不支持的TPID
    UnsupportedTpid { tpid: u16 },

    /// 报文长度不足，无法解析VLAN标签
    InsufficientPacketLength { expected: usize, actual: usize },

    /// 双层VLAN标签暂不支持
    DoubleTagNotSupported,

    /// VLAN标签解析错误
    ParseError(String),
}
```

---

## 四、接口定义

### 4.1 VlanTag 基础操作

VLAN标签的创建和验证。

```rust
impl VlanTag {
    /// 创建新的VLAN标签
    ///
    /// # 参数
    /// - pcp: 优先级 (0-7)
    /// - dei: 丢弃指示
    /// - vid: VLAN ID (1-4094)
    ///
    /// # 返回
    /// - Ok(VlanTag): 创建成功
    /// - Err(VlanError): 参数无效
    pub fn new(pcp: u8, dei: bool, vid: u16) -> Result<Self, VlanError>;

    /// 创建默认VLAN标签 (PCP=0, DEI=false, VID=1)
    pub fn default() -> Self;

    /// 验证VLAN ID是否有效
    ///
    /// 有效范围: 1-4094
    pub fn is_valid_vid(vid: u16) -> bool;

    /// 验证PCP是否有效
    ///
    /// 有效范围: 0-7
    pub fn is_valid_pcp(pcp: u8) -> bool;

    /// 将VLAN标签编码为2字节 (网络字节序)
    pub fn to_bytes(&self) -> [u8; 2];

    /// 从2字节解析VLAN标签 (网络字节序)
    pub fn from_bytes(data: [u8; 2]) -> Result<Self, VlanError>;
}
```

### 4.2 VlanTag 解析接口

从Packet中解析VLAN标签。

```rust
impl VlanTag {
    /// 从Packet中解析VLAN标签
    ///
    /// # 参数
    /// - packet: 可变引用的Packet (读取后会移动offset)
    ///
    /// # 返回
    /// - Ok(VlanTag): 解析成功
    /// - Err(VlanError): 解析失败
    ///
    /// # 行为
    /// - 从当前offset读取2字节
    /// - 自动移动offset 2字节
    pub fn parse_from_packet(packet: &mut Packet) -> Result<Self, VlanError>;

    /// 查看Packet中的VLAN标签 (不移动offset)
    pub fn peek_from_packet(packet: &Packet) -> Result<Self, VlanError>;
}
```

### 4.3 VlanTag 封装接口

将VLAN标签写入Packet。

```rust
impl VlanTag {
    /// 将VLAN标签写入Packet (在当前位置插入)
    ///
    /// # 参数
    /// - packet: 可变引用的Packet
    /// - tpid: 标签协议标识符 (默认0x8100)
    ///
    /// # 返回
    /// - Ok(()): 写入成功
    /// - Err(VlanError): 写入失败
    ///
    /// # 行为
    /// - 在当前offset位置写入TPID (2字节)
    /// - 在TPID后写入VLAN标签 (2字节)
    /// - 移动offset 4字节
    pub fn write_to_packet(&self, packet: &mut Packet, tpid: u16) -> Result<(), VlanError>;

    /// 追加VLAN标签到Packet末尾
    pub fn append_to_packet(&self, packet: &mut Packet, tpid: u16) -> Result<(), VlanError>;
}
```

### 4.4 VLAN检测工具函数

用于检测Packet是否包含VLAN标签。

```rust
/// 检查以太网帧是否包含VLAN标签
///
/// # 参数
/// - packet: Packet引用
///
/// # 返回
/// - Some(tpid): 包含VLAN标签，返回TPID
/// - None: 不包含VLAN标签
///
/// # 行为
/// - 检查当前offset位置的EtherType是否为VLAN TPID
/// - 不移动offset
pub fn has_vlan_tag(packet: &Packet) -> Option<u16>;

/// 检查指定的EtherType是否为VLAN TPID
///
/// # 参数
/// - ether_type: 以太网类型字段
///
/// # 返回
/// - true: 是VLAN TPID
/// - false: 不是VLAN TPID
pub fn is_vlan_tpid(ether_type: u16) -> bool;
```

---

## 五、模块结构

```
src/protocols/vlan/
├── mod.rs               # 模块入口，导出公共接口
├── tag.rs              # VlanTag结构体及其实现
├── frame.rs            # VlanFrame结构体及其实现
├── error.rs            # VlanError错误类型定义
└── parse.rs            # VLAN解析和封装的辅助函数
```

### 模块导出

```rust
mod tag;
mod frame;
mod error;
mod parse;

pub use tag::VlanTag;
pub use frame::VlanFrame;
pub use error::VlanError;

pub use parse::{
    has_vlan_tag,
    is_vlan_tpid,
};
```

---

## 六、错误处理

### 6.1 错误类型定义

```rust
/// VLAN处理错误
#[derive(Debug)]
pub enum VlanError {
    /// 无效的VLAN ID
    InvalidVlanId { vid: u16 },

    /// 无效的PCP值
    InvalidPcp { pcp: u8 },

    /// 不支持的TPID
    UnsupportedTpid { tpid: u16 },

    /// 报文长度不足
    InsufficientPacketLength { expected: usize, actual: usize },

    /// 双层VLAN标签暂不支持
    DoubleTagNotSupported,

    /// VLAN标签解析错误
    ParseError(String),
}
```

### 6.2 错误处理策略

1. **验证优先**：在创建VlanTag时验证VLAN ID和PCP的有效性
2. **错误传播**：所有解析/封装函数返回Result，错误向上传播
3. **错误转换**：实现From trait，支持与其他错误类型的自动转换
4. **清晰信息**：错误消息应包含足够的调试信息（如无效值、期望长度等）

### 6.3 Display实现

```rust
impl std::fmt::Display for VlanError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidVlanId { vid } =>
                write!(f, "无效的VLAN ID: {} (有效范围: 1-4094)", vid),
            Self::InvalidPcp { pcp } =>
                write!(f, "无效的PCP值: {} (有效范围: 0-7)", pcp),
            Self::UnsupportedTpid { tpid } =>
                write!(f, "不支持的TPID: 0x{:04x}", tpid),
            Self::InsufficientPacketLength { expected, actual } =>
                write!(f, "报文长度不足: 期望{}字节, 实际{}字节", expected, actual),
            Self::DoubleTagNotSupported =>
                write!(f, "双层VLAN标签暂不支持"),
            Self::ParseError(msg) =>
                write!(f, "VLAN解析错误: {}", msg),
        }
    }
}
```

---

## 七、协议规范

### 7.1 802.1Q标签格式

```
 0                   1
 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
| |PCP| |  VLAN ID (高12位)  |
| |   |C| (VID)              |
| |   |I|                    |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|   VLAN ID (低12位补全)      |
|                            |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
```

| 字段 | 位数 | 说明 |
|------|------|------|
| PCP | 3 bits | 优先级代码点，0-7 |
| DEI | 1 bit | 丢弃指示 |
| VID | 12 bits | VLAN标识符，1-4094 |

### 7.2 VLAN帧格式

**标准以太网帧（无VLAN）**：
```
+--------+--------+-------+----------+-----+
| DST MAC| SRC MAC| Type  |  Payload| FCS |
| 6 bytes| 6 bytes|2 bytes| 46-1500 |4 bytes|
+--------+--------+-------+----------+-----+
```

**带VLAN标签的以太网帧**：
```
+--------+--------+-------+-------+-------+----------+-----+
| DST MAC| SRC MAC| TPID  | TCI   | Type  |  Payload| FCS |
| 6 bytes| 6 bytes|2 bytes|2 bytes|2 bytes| 42-1500 |4 bytes|
+--------+--------+-------+-------+-------+----------+-----+
```

- TPID: Tag Protocol Identifier (0x8100)
- TCI: Tag Control Information (PCP + DEI + VID)

### 7.3 支持的TPID值

| TPID | 名称 | 说明 |
|------|------|------|
| 0x8100 | 802.1Q | 标准VLAN标签 |
| 0x9100 | Q-in-Q | 双层标签（Stacking VLAN）|
| 0x88A8 | 802.1ad | Provider Bridge VLAN |

---

## 八、测试策略

### 8.1 单元测试

**VlanTag创建和验证**：
```rust
#[test]
fn test_vlan_tag_creation() {
    // 测试正常VLAN ID (1-4094)
    // 测试边界值 (1, 4094)
    // 测试无效VLAN ID (0, 4095)
}

#[test]
fn test_pcp_validation() {
    // 测试PCP范围 (0-7)
    // 测试无效PCP (>7)
}

#[test]
fn test_vlan_tag_encode_decode() {
    // 测试编码为字节后解码的一致性
}
```

**Packet解析和封装**：
```rust
#[test]
fn test_parse_vlan_from_packet() {
    // 测试从Packet解析VLAN标签
    // 测试offset移动正确
}

#[test]
fn test_write_vlan_to_packet() {
    // 测试写入VLAN标签到Packet
    // 验证写入的字节正确
}

#[test]
fn test_has_vlan_tag() {
    // 测试VLAN标签检测
    // 测试带VLAN和不带VLAN的情况
}
```

### 8.2 集成测试

**完整VLAN帧处理**：
```rust
#[test]
fn test_vlan_frame_roundtrip() {
    // 1. 创建以太网帧
    // 2. 添加VLAN标签
    // 3. 解析VLAN标签
    // 4. 验证数据一致性
}

#[test]
fn test_vlan_arp_packet() {
    // 测试带有VLAN标签的ARP报文
}

#[test]
fn test_double_vlan_detection() {
    // 测试检测双层VLAN并返回错误
}
```

### 8.3 边界测试

- 测试最小VLAN帧长度
- 测试VLAN ID = 0 和 4095 的错误处理
- 测试Packet长度不足的情况
- 测试不支持的TPID处理

---

## 九、实现路线图

| 阶段 | 内容 | 状态 |
|------|------|------|
| Phase 1 | 核心数据结构 (VlanTag, VlanError) | 待实现 |
| Phase 2 | VLAN标签解析和编码接口 | 待规划 |
| Phase 3 | Packet集成 (parse_from_packet, write_to_packet) | 待规划 |
| Phase 4 | 以太网模块集成 | 待规划 |
| Phase 5 | 双层VLAN (Q-in-Q) 支持 | 待规划 |

---

## 十、与其他模块的交互

### 10.1 与以太网模块的交互

**接收方向**：
```
以太网模块接收帧 -> 检查EtherType
                  -> 0x8100 -> VLAN模块解析
                  -> 提取VLAN信息
                  -> 去除VLAN标签
                  -> 根据内层EtherType分发到上层协议
```

**发送方向**：
```
上层协议数据 -> 需要VLAN标签?
                  -> Yes -> VLAN模块封装
                  ->      -> 添加TPID + TCI
                  ->      -> 调整EtherType位置
                  -> 以太网模块发送
```

### 10.2 与IP/ARP模块的交互

- IP和ARP模块需要知道报文来自哪个VLAN
- 可以在处理上下文中传递VLAN ID信息
- ARP请求/响应需要考虑VLAN隔离

---

## 十一、设计原则

1. **标准遵循**：严格遵循IEEE 802.1Q标准
2. **类型安全**：使用Rust类型系统确保数据有效性
3. **零拷贝**：尽可能使用引用，避免不必要的数据拷贝
4. **清晰错误**：提供详细的错误信息便于调试
5. **渐进实现**：先实现单层VLAN，再扩展到双层VLAN

---

## 十二、参考资料

1. **IEEE 802.1Q** - Virtual LANs Standard
2. **RFC 5518** - Transmission of IPv4 over IEEE 802.1Q VLANs
3. **RFC 7348** - Virtual eXtensible Local Area Network (VXLAN)
