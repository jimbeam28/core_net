# 报文描述符设计

## 1. 概述

Packet是CoreNet中核心的数据结构，用于在协议栈各层之间传递报文数据。它封装了原始buffer和协议解析过程中的元数据。

## 2. 核心结构

```rust
use std::time::Instant;

/// 报文描述符
pub struct Packet {
    // === Buffer管理 ===
    pub buffer: Vec<u8>,           // 报文数据缓冲区
    pub length: usize,              // 报文实际长度
    pub capacity: usize,            // buffer总容量

    // === 元数据 ===
    pub timestamp: Instant,         // 接收时间戳
    pub interface: InterfaceId,     // 接收接口ID

    // === 协议信息（逐步填充） ===
    pub eth_src: Option<MacAddr>,          // 以太网源地址
    pub eth_dst: Option<MacAddr>,          // 以太网目的地址
    pub eth_type: Option<EtherType>,        // 以太网类型
    pub ip_version: Option<IpVersion>,      // IP版本 (4/6)
    pub ip_src: Option<IpAddr>,            // IP源地址
    pub ip_dst: Option<IpAddr>,            // IP目的地址
    pub ip_ttl: Option<u8>,                // TTL/Hop Limit
    pub protocol: Option<IpProtocol>,       // 传输层协议
    pub transport_src: Option<u16>,         // 传输层源端口
    pub transport_dst: Option<u16>,         // 传输层目的端口

    // === 解析状态 ===
    pub offset: usize,                      // 当前解析位置
    pub layers: Vec<Layer>,                // 已解析的协议层
}

/// 协议层标识
#[derive(Debug, Clone, Copy)]
pub enum Layer {
    Ethernet,
    Arp,
    IPv4,
    IPv6,
    ICMP,
    ICMPv6,
    TCP,
    UDP,
}

/// 接口ID类型
pub type InterfaceId = u32;
```

## 3. 主要方法

```rust
impl Packet {
    /// 创建新的空Packet
    pub fn new(capacity: usize) -> Self;

    /// 从已有数据创建Packet
    pub fn from_bytes(data: Vec<u8>) -> Self;

    /// 获取剩余可读取长度
    pub fn remaining(&self) -> usize;

    /// 检查是否有足够的数据可读
    pub fn has_remaining(&self, len: usize) -> bool;

    /// 读取指定字节数，不移动offset
    pub fn peek(&self, len: usize) -> Option<&[u8]>;

    /// 读取指定字节数，移动offset
    pub fn read(&mut self, len: usize) -> Option<&[u8]>;

    /// 跳过指定字节数
    pub fn skip(&mut self, len: usize) -> bool;

    /// 重置offset到指定位置
    pub fn seek(&mut self, offset: usize) -> bool;

    /// 添加协议层
    pub fn push_layer(&mut self, layer: Layer);

    /// 获取当前协议层
    pub fn current_layer(&self) -> Option<&Layer>;

    /// 预留头部空间（用于封装时添加协议头）
    pub fn reserve_header(&mut self, len: usize) -> bool;

    /// 清空数据，保留buffer
    pub fn clear(&mut self);

    /// 复制Packet（浅拷贝共享buffer）
    pub fn clone(&self) -> Self;
}
```

## 4. 内存管理策略

### 4.1 预留空间设计

为了支持逐层封装，Packet在头部预留足够空间：

```
+------------------+----------------------+------------------+
| Header Reserve |    Packet Data      |    Tail         |
| (用于添加协议头)  |    (实际报文数据)    |    (剩余空间)     |
+------------------+----------------------+------------------+
^                  ^                     ^
buffer_start       offset/data           length/capacity
```

### 4.2 Packet池设计

#### 4.2.1 池化Packet结构

```rust
/// 池化的Packet（带有释放回调和状态）
pub struct PooledPacket {
    /// 内部Packet
    pub inner: Packet,

    /// 所属的池ID
    pub pool_id: usize,

    /// 是否在池中（分配出去后为false）
    pub in_pool: AtomicBool,
}

impl PooledPacket {
    /// 创建新的池化Packet
    pub fn new(packet: Packet, pool_id: usize) -> Self {
        Self {
            inner: packet,
            pool_id,
            in_pool: AtomicBool::new(false),
        }
    }

    /// 获取Packet的引用
    pub fn as_packet(&self) -> &Packet {
        &self.inner
    }

    /// 获取Packet的可变引用
    pub fn as_packet_mut(&mut self) -> &mut Packet {
        &mut self.inner
    }
}

impl Deref for PooledPacket {
    type Target = Packet;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl DerefMut for PooledPacket {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}
```

#### 4.2.2 PacketPool数据结构

```rust
/// Packet池
pub struct PacketPool {
    /// 空闲Packet列表
    idle: Mutex<Vec<Packet>>,

    /// 池配置
    config: PoolConfig,

    /// 池统计
    stats: PoolStats,
}

/// 池配置
pub struct PoolConfig {
    /// 池容量（最大Packet数量）
    pub capacity: usize,

    /// 每个Packet的buffer大小
    pub packet_size: usize,

    /// 头部预留大小
    pub header_reserve: usize,

    /// 分配策略
    pub alloc_strategy: AllocStrategy,
}

/// 分配策略
pub enum AllocStrategy {
    /// 直接分配（不使用池）
    Direct,

    /// 池分配
    Pooled,

    /// 混合模式（小包用池，大包直接分配）
    Hybrid { threshold: usize },
}

/// 池统计
pub struct PoolStats {
    /// 总分配次数
    pub total_allocations: u64,

    /// 池分配次数
    pub pooled_allocations: u64,

    /// 直接分配次数
    pub direct_allocations: u64,

    /// 当前池中Packet数
    pub idle_count: usize,

    /// 使用中Packet数
    pub active_count: usize,

    /// 等待获取的请求数
    pub wait_count: usize,
}
```

#### 4.2.3 PacketPool API

```rust
impl PacketPool {
    /// 创建新的Packet池
    pub fn new(config: PoolConfig) -> Result<Self, PoolError>;

    /// 获取Packet（可能等待）
    pub fn acquire(&self) -> Result<PooledPacket, PoolError>;

    /// 尝试获取Packet（非阻塞）
    pub fn try_acquire(&self) -> Result<PooledPacket, PoolError>;

    /// 归还Packet到池
    pub fn release(&self, packet: PooledPacket) -> Result<(), PoolError>;

    /// 归还Packet并清空数据
    pub fn release_and_clear(&self, packet: PooledPacket) -> Result<(), PoolError> {
        packet.inner.clear();
        self.release(packet)
    }

    /// 获取池统计
    pub fn stats(&self) -> PoolStats;

    /// 重置统计
    pub fn reset_stats(&mut self);

    /// 预热池（提前分配指定数量的Packet）
    pub fn warm_up(&self, count: usize) -> Result<(), PoolError>;

    /// 收缩池（释放多余的空闲Packet）
    pub fn shrink(&self, target_count: usize) -> Result<(), PoolError>;
}
```

#### 4.2.4 错误类型

```rust
/// 池错误
#[derive(Debug)]
pub enum PoolError {
    /// 池未初始化
    NotInitialized,

    /// 池已空（无可用Packet）
    Empty,

    /// 池已满（归还时目标池不存在）
    InvalidPacket,

    /// 池已满（无法分配更多）
    Full,

    /// 获取超时
    Timeout(Duration),

    /// 配置错误
    InvalidConfig(String),
}

impl fmt::Display for PoolError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            PoolError::NotInitialized => write!(f, "池未初始化"),
            PoolError::Empty => write!(f, "池已空"),
            PoolError::InvalidPacket => write!(f, "无效的Packet"),
            PoolError::Full => write!(f, "池已满"),
            PoolError::Timeout(d) => write!(f, "获取超时: {:?}", d),
            PoolError::InvalidConfig(msg) => {
                write!(f, "无效配置: {}", msg)
            }
        }
    }
}
```

### 4.3 内存布局

```
单个Packet内存布局：
┌──────────────────────────────────────────────────────────┐
│ Header │                    │ Data    │
│ Reserve│                    │ Section  │
├────────┼────────────────────────┼─────────┼────────┤
│  54B   │     协议头          │ Payload │    │
│(最大)  │ Eth(14) + IP(20)   │         │    │
│        │ + TCP/UDP(8~20)    │         │    │
└────────┴───────────────────────┴─────────┴────────┘
0        ^                        ^           ^
      header_start              data      length/capacity
```

### 4.4 池管理流程

```
分配流程：
┌────────┐    ┌────────┐    ┌────────┐    ┌────────┐
│ 请求者 │ -> │ 池统计 │ -> │ 空闲队列 │ -> │ 返回Packet│
└────────┘    └────────┘    └────────┘    └────────┘

归还流程：
┌────────┐    ┌────────┐    ┌────────┐
│ 使用者 │ -> │ 清空数据 │ -> │ 空闲队列 │
└────────┘    └────────┘    └────────┘
```

## 5. 使用示例

### 5.1 解析报文（上行）

```rust
// 接收原始数据
let mut packet = Packet::from_bytes(raw_data);

// 以太网层解析
if let Some(eth_frame) = EthernetFrame::parse(&mut packet)? {
    packet.eth_src = Some(eth_frame.src);
    packet.eth_dst = Some(eth_frame.dst);
    packet.eth_type = Some(eth_frame.eth_type);
    packet.push_layer(Layer::Ethernet);
}

// IP层解析
match packet.ip_version {
    Some(IpVersion::V4) => {
        if let Some(ipv4) = Ipv4Packet::parse(&mut packet)? {
            packet.ip_src = Some(IpAddr::V4(ipv4.src));
            packet.ip_dst = Some(IpAddr::V4(ipv4.dst));
            packet.protocol = Some(ipv4.protocol);
            packet.push_layer(Layer::IPv4);
        }
    }
    // ...
}
```

### 5.2 封装报文（下行）

```rust
// 创建新Packet
let mut packet = Packet::new(1500);
packet.reserve_header(54);  // 预留最大协议头空间

// 填充数据
packet.extend_from_slice(&payload);

// 逐层封装
TCP::encapsulate(&mut packet, src_port, dst_port)?;
IPv4::encapsulate(&mut packet, src_ip, dst_ip)?;
Ethernet::encapsulate(&mut packet, src_mac, dst_mac)?;
```

### 5.3 使用Packet池

```rust
// 创建池
let pool = PacketPool::new(PoolConfig {
    capacity: 100,
    packet_size: 2048,
    header_reserve: 128,
    alloc_strategy: AllocStrategy::Pooled,
})?;

// 分配Packet
let mut packet = pool.acquire()?;

// 使用packet（自动解引用为Packet）
packet.reserve_header(14);
packet.extend_from_slice(&data);

// 归还Packet（自动清空并回收到池）
pool.release_and_clear(packet)?;
```

## 6. 注意事项

1. **所有权转移**: Packet在协议层之间传递时转移所有权，避免不必要的拷贝
2. **可变性**: 解析过程需要修改offset和metadata，所以需要mut
3. **buffer复用**: 通过PacketPool复用Packet结构，减少内存分配
4. **零拷贝**: 尽量使用切片引用，避免数据拷贝
