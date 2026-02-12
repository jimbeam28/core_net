# 通用对象池设计

## 1. 概述

本文档描述 CoreNet 的通用对象池设计。对象池是一个性能优化组件，用于复用对象以减少频繁的内存分配和释放开销。

### 1.1 设计目标

1. **通用性**: 支持任意类型的对象池化
2. **线程安全**: 使用无锁或低锁策略实现高并发访问
3. **可配置**: 支持不同的池策略（容量、预热、收缩等）
4. **可观测**: 提供详细的统计信息用于性能分析
5. **零外部依赖**: 仅使用 Rust 标准库

### 1.2 核心组件

```
┌─────────────────────────────────────────────────────────────────┐
│                     应用层                                  │
├─────────────────────────────────────────────────────────────────┤
│                 Packet 适配层                                │
│         PacketPool / PooledPacket / PacketBuilder             │
├─────────────────────────────────────────────────────────────────┤
│                  通用对象池框架                              │
│              Pool<T> / PoolConfig / PoolStats                │
├─────────────────────────────────────────────────────────────────┤
│                  内存分配器                                  │
└─────────────────────────────────────────────────────────────────┘
```

### 1.3 文件结构

```
src/common/
├── pool/
│   ├── mod.rs           # 模块导出
│   ├── pool.rs         # 通用池实现 (Pool<T>)
│   ├── config.rs       # 配置和策略
│   ├── stats.rs       # 统计信息
│   └── error.rs       # 错误类型
└── packet_pool.rs     # Packet 适配层 (PacketPool, PooledPacket)
```

---

## 2. 通用对象池框架

### 2.1 核心结构

```rust
/// 通用对象池
pub struct Pool<T> {
    /// 空闲对象列表
    idle: Mutex<Vec<T>>,

    /// 池配置
    config: PoolConfig,

    /// 池统计（使用原子操作避免锁竞争）
    stats: Arc<PoolStats>,

    /// 对象工厂（用于创建新对象）
    factory: Box<dyn Fn() -> T + Send + Sync>,

    /// 对象重置器（用于清空对象状态）
    resetter: Option<Box<dyn Fn(&mut T) + Send + Sync>>,
}
```

### 2.2 PoolConfig - 池配置

```rust
/// 池配置
#[derive(Debug, Clone)]
pub struct PoolConfig {
    /// 池容量（最大对象数量）
    pub capacity: usize,

    /// 初始容量（池创建时预分配的对象数）
    pub initial_capacity: usize,

    /// 分配策略
    pub alloc_strategy: AllocStrategy,

    /// 等待策略（当池为空时）
    pub wait_strategy: WaitStrategy,

    /// 是否自动扩展（超出容量时是否允许临时分配）
    pub allow_overflow: bool,
}

impl Default for PoolConfig {
    fn default() -> Self {
        Self {
            capacity: 100,
            initial_capacity: 10,
            alloc_strategy: AllocStrategy::Fifo,
            wait_strategy: WaitStrategy::Timeout(Duration::from_millis(100)),
            allow_overflow: false,
        }
    }
}
```

### 2.3 AllocStrategy - 分配策略

```rust
/// 分配策略
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AllocStrategy {
    /// 先进先出（从头部获取，归还时追加到尾部）
    Fifo,

    /// 后进先出（从尾部获取，归还时追加到尾部）
    Lifo,

    /// 随机获取（用于减少锁竞争热点）
    Random,

    /// 优先分配最近使用的（利用 CPU 缓存局部性）
    Recent,
}
```

| 策略 | 适用场景 | 特点 |
|------|----------|------|
| Fifo | 公平性要求高 | 保证公平分配 |
| Lifo | 低延迟场景 | 利用缓存局部性 |
| Random | 高并发场景 | 减少锁竞争 |
| Recent | 短生命周期对象 | CPU 缓存友好 |

### 2.4 WaitStrategy - 等待策略

```rust
/// 等待策略
#[derive(Debug, Clone, Copy)]
pub enum WaitStrategy {
    /// 立即返回失败
    Immediate,

    /// 自旋等待指定次数
    Spin(usize),

    /// 让出 CPU 时间片
    Yield,

    /// 等待指定超时时间
    Timeout(Duration),

    /// 无限等待
    Blocking,
}
```

### 2.5 PoolStats - 池统计

```rust
/// 池统计（使用原子操作实现线程安全）
#[derive(Debug)]
pub struct PoolStats {
    /// 总分配次数
    pub total_allocations: AtomicU64,

    /// 池分配次数
    pub pooled_allocations: AtomicU64,

    /// 溢出分配次数（超出池容量的分配）
    pub overflow_allocations: AtomicU64,

    /// 当前池中空闲对象数
    pub idle_count: AtomicUsize,

    /// 使用中对象数
    pub active_count: AtomicUsize,

    /// 等待获取的请求数
    pub wait_count: AtomicUsize,

    /// 总等待时间（纳秒）
    pub total_wait_ns: AtomicU64,

    /// 归还次数
    pub release_count: AtomicU64,
}

impl PoolStats {
    /// 获取池使用率 (active / (active + idle))
    pub fn utilization_rate(&self) -> f64;

    /// 获取平均等待时间（纳秒）
    pub fn avg_wait_ns(&self) -> u64;

    /// 重置所有统计
    pub fn reset(&self);
}
```

### 2.6 PoolError - 池错误

```rust
/// 池错误
#[derive(Debug)]
pub enum PoolError {
    /// 池已空（无可用对象）
    Empty,

    /// 池已满（归还时超出容量）
    Full,

    /// 池已关闭
    Shutdown,

    /// 超时
    Timeout(Duration),

    /// 配置错误
    InvalidConfig(String),

    /// 其他错误
    Other(String),
}

impl fmt::Display for PoolError;
impl std::error::Error for PoolError;
```

### 2.7 PoolStatus - 池状态快照

```rust
/// 池状态快照
#[derive(Debug, Clone)]
pub struct PoolStatus {
    /// 空闲对象数
    pub idle: usize,

    /// 使用中对象数
    pub active: usize,

    /// 池使用率
    pub utilization: f64,
}
```

### 2.8 Pool<T> API

```rust
impl<T> Pool<T>
where
    T: Send + 'static,
{
    /// 创建新的对象池
    pub fn new<F>(factory: F, config: PoolConfig) -> Self
    where
        F: Fn() -> T + Send + Sync + 'static;

    /// 创建带有重置器的对象池
    pub fn with_resetter<F, R>(
        factory: F,
        resetter: R,
        config: PoolConfig,
    ) -> Self
    where
        F: Fn() -> T + Send + Sync + 'static,
        R: Fn(&mut T) + Send + Sync + 'static;

    /// 获取对象（可能等待）
    pub fn acquire(&self) -> Result<Pooled<T>, PoolError>;

    /// 尝试获取对象（非阻塞）
    pub fn try_acquire(&self) -> Result<Pooled<T>, PoolError>;

    /// 获取多个对象
    pub fn acquire_many(&self, count: usize) -> Result<Vec<Pooled<T>>, PoolError>;

    /// 归还对象
    pub fn release(&self, item: T);

    /// 归还并清空对象
    pub fn release_and_clear(&self, item: T)
    where
        T: Clear;

    /// 获取只读统计快照
    pub fn stats(&self) -> &PoolStats;

    /// 重置统计
    pub fn reset_stats(&self);

    /// 预热池（提前分配指定数量的对象）
    pub fn warm_up(&self, count: usize) -> Result<(), PoolError>;

    /// 收缩池（释放多余的空闲对象到指定数量）
    pub fn shrink(&self, target_count: usize);

    /// 获取当前池状态
    pub fn status(&self) -> PoolStatus;

    /// 关闭池（停止接受新的获取请求）
    pub fn shutdown(&self);
}
```

### 2.9 Pooled<T> - 池化对象包装器

```rust
/// 池化对象包装器
///
/// 当 Pooled 被 dropped 时，自动将对象归还到池中
pub struct Pooled<T> {
    /// 内部对象
    inner: Option<T>,

    /// 所属的池
    pool: Option<Arc<Pool<T>>>,

    /// 是否需要归还（防止重复归还）
    released: AtomicBool,
}

impl<T> Pooled<T> {
    /// 获取内部对象的不可变引用
    pub fn inner(&self) -> &T;

    /// 获取内部对象的可变引用
    pub fn inner_mut(&mut self) -> &mut T;

    /// 手动归还对象到池（提前释放）
    pub fn release(self);

    /// 归还并清空对象
    pub fn release_and_clear(self)
    where
        T: Clear;

    /// 转换内部对象（用于类型转换）
    pub fn map<U, F>(self, f: F) -> U
    where
        F: FnOnce(T) -> U;
}

// 不实现 Clone，避免意外复制导致重复释放
```

### 2.10 Clear Trait

```rust
/// 可清空对象 trait
///
/// 实现此 trait 的类型可以被对象池自动清空
pub trait Clear {
    fn clear(&mut self);
}
```

---

## 3. Packet 适配层

### 3.1 设计目标

Packet 适配层提供以下功能：
1. **便捷的 Packet 池创建**：根据网络 MTU 和预期负载配置池
2. **Packet 特定优化**：预分配协议头空间、零拷贝读取等
3. **统计增强**：Packet 相关的额外统计（如字节数、层数等）
4. **构建器模式**：方便创建和初始化 Packet

### 3.2 PacketPoolConfig - Packet 池配置

```rust
/// Packet 池配置
#[derive(Debug, Clone)]
pub struct PacketPoolConfig {
    /// 池基础配置
    pub pool: PoolConfig,

    /// 每个 Packet 的 buffer 大小（字节）
    pub packet_size: usize,

    /// 头部预留空间（用于添加协议头）
    pub header_reserve: usize,

    /// 尾部预留空间（用于添加填充等）
    pub trailer_reserve: usize,

    /// 预设的源接口 ID（可选）
    pub default_interface: Option<InterfaceId>,
}

impl Default for PacketPoolConfig {
    fn default() -> Self {
        Self {
            pool: PoolConfig::default(),
            packet_size: 1514,        // 标准以太网 MTU
            header_reserve: 128,       // 足够容纳所有协议头
            trailer_reserve: 0,
            default_interface: None,
        }
    }
}

impl PacketPoolConfig {
    /// 创建高吞吐量配置
    pub fn high_throughput() -> Self;

    /// 创建低延迟配置
    pub fn low_latency() -> Self;

    /// 创建内存受限配置
    pub fn memory_constrained() -> Self;
}
```

#### 预设配置对比

| 配置 | 容量 | 分配策略 | Packet 大小 | 适用场景 |
|------|-------|----------|------------|----------|
| Default | 100 | Fifo | 1514B | 通用场景 |
| high_throughput | 1000 | Recent | 9000B | 大流量传输 |
| low_latency | 500 | Lifo | 1514B | 实时通信 |
| memory_constrained | 50 | Fifo | 1514B | 嵌入式设备 |

### 3.3 PacketPoolStats - Packet 池统计

```rust
/// Packet 池统计（扩展自基础池统计）
#[derive(Debug)]
pub struct PacketPoolStats {
    /// 基础池统计
    pub base: PoolStats,

    /// 总处理字节数
    pub total_bytes: AtomicU64,

    /// 当前在池中的 Packet 总容量（字节）
    pub pooled_capacity_bytes: AtomicU64,

    /// 平均 Packet 数据长度（字节）
    pub avg_packet_length: AtomicU64,
}

impl PacketPoolStats {
    /// 获取内存利用率（总字节 / 池总容量）
    pub fn memory_efficiency(&self) -> f64;
}
```

### 3.4 PacketPool 结构

```rust
/// Packet 对象池
///
/// 提供便捷的 Packet 申请和释放接口
pub struct PacketPool {
    /// 内部通用池
    inner: Pool<Packet>,

    /// Packet 池配置
    config: PacketPoolConfig,

    /// 扩展统计
    stats: Arc<PacketPoolStats>,
}

impl Clone for PacketPool;
```

### 3.5 PooledPacket - 池化的 Packet

```rust
/// 池化的 Packet
///
/// 实现 Deref/DerefMut 以便像普通 Packet 一样使用
/// Drop 时自动归还到池
pub struct PooledPacket {
    /// 池化的 Packet
    inner: Pooled<Packet>,

    /// 所属的池（用于归还和统计）
    pool: Arc<PacketPool>,

    /// 是否记录字节数（用于统计）
    track_bytes: bool,
}

impl PooledPacket {
    /// 归还 Packet 到池（提前释放）
    pub fn release(self);

    /// 归还并清空 Packet
    pub fn release_and_clear(self);

    /// 提取内部 Packet（消耗 PooledPacket，不归还到池）
    ///
    /// 警告：这会导致内存泄漏，除非后续手动归还！
    pub fn detach(mut self) -> Packet;

    /// 获取所属池的引用
    pub fn pool(&self) -> &PacketPool;
}

impl Deref for PooledPacket {
    type Target = Packet;
}

impl DerefMut for PooledPacket;
```

### 3.6 PacketPool API

```rust
impl PacketPool {
    /// 创建新的 Packet 池
    pub fn new(config: PacketPoolConfig) -> Result<Self, PoolError>;

    /// 创建默认配置的 Packet 池
    pub fn with_capacity(capacity: usize) -> Result<Self, PoolError>;

    /// 获取 Packet（可能等待）
    pub fn acquire(&self) -> Result<PooledPacket, PoolError>;

    /// 尝试获取 Packet（非阻塞）
    pub fn try_acquire(&self) -> Result<PooledPacket, PoolError>;

    /// 归还 Packet 到池
    pub fn release(&self, packet: Packet);

    /// 归还并清空 Packet
    pub fn release_and_clear(&self, mut packet: Packet);

    /// 获取统计
    pub fn stats(&self) -> &PacketPoolStats;

    /// 预热池（提前分配指定数量的 Packet）
    pub fn warm_up(&self, count: usize) -> Result<(), PoolError>;

    /// 收缩池
    pub fn shrink(&self, target_count: usize);

    /// 关闭池
    pub fn shutdown(&self);
}
```

### 3.7 PacketBuilder - Packet 构建器

```rust
/// Packet 构建器
///
/// 提供流式 API 用于创建和初始化 Packet
pub struct PacketBuilder {
    pool: Arc<PacketPool>,
    packet: Option<PooledPacket>,
}

impl PacketBuilder {
    /// 创建新的构建器
    pub fn new(pool: &PacketPool) -> Result<Self, PoolError>;

    /// 获取 Packet
    pub fn acquire(&mut self) -> Result<&mut PooledPacket, PoolError>;

    /// 设置时间戳
    pub fn with_timestamp(mut self, timestamp: Instant) -> Self;

    /// 设置接口 ID
    pub fn with_interface(mut self, interface: InterfaceId) -> Self;

    /// 填充数据
    pub fn with_data(mut self, data: &[u8]) -> Result<Self, CoreError>;

    /// 预留头部空间
    pub fn with_header_reserve(mut self, len: usize) -> Result<Self, CoreError>;

    /// 构建（消耗 Builder，返回 PooledPacket）
    pub fn build(mut self) -> Result<PooledPacket, PoolError>;
}
```

---

## 4. 使用示例

### 4.1 基础使用

```rust
use core_net::pool::{PacketPool, PacketPoolConfig};

// 创建 Packet 池
let pool = PacketPool::with_capacity(100)?;

// 获取 Packet
let mut packet = pool.acquire()?;

// 使用 Packet（自动解引用）
packet.extend_from_slice(&[0x01, 0x02, 0x03])?;
packet.ip_src = Some(IpAddr::v4(192, 168, 1, 1));

// Packet 离开作用域时自动归还
```

### 4.2 自定义配置

```rust
// 高吞吐量配置
let config = PacketPoolConfig::high_throughput();
let pool = PacketPool::new(config)?;

// 低延迟配置
let config = PacketPoolConfig::low_latency();
let pool = PacketPool::new(config)?;

// 自定义配置
let config = PacketPoolConfig {
    pool: PoolConfig {
        capacity: 500,
        initial_capacity: 50,
        alloc_strategy: AllocStrategy::Lifo,
        wait_strategy: WaitStrategy::Spin(10),
        allow_overflow: false,
    },
    packet_size: 9000,  // Jumbo 帧
    header_reserve: 256,
    trailer_reserve: 64,
    default_interface: Some(1),
};
let pool = PacketPool::new(config)?;
```

### 4.3 使用构建器

```rust
let mut packet = PacketBuilder::new(&pool)?
    .with_interface(1)
    .with_data(&payload)?
    .with_header_reserve(14)?
    .build()?;
```

### 4.4 协议处理流程

```rust
// 接收路径
fn handle_rx_packet(pool: &PacketPool, raw_data: Vec<u8>) -> Result<(), CoreError> {
    let mut packet = pool.acquire()?;

    // 填充数据并解析
    packet.extend_from_slice(&raw_data)?;

    // 解析以太网头
    // 解析 IP 头
    // 解析传输层

    Ok(())
}

// 发送路径
fn prepare_tx_packet(pool: &PacketPool, payload: &[u8]) -> Result<PooledPacket, CoreError> {
    let mut packet = pool.acquire()?;

    // 预留协议头空间
    packet.reserve_header(54)?;

    // 填充负载数据
    packet.extend_from_slice(payload)?;

    // 逐层封装
    // TCP::encapsulate(&mut packet, ...)?;
    // IPv4::encapsulate(&mut packet, ...)?;
    // Ethernet::encapsulate(&mut packet, ...)?;

    Ok(packet)
}
```

### 4.5 池预热与监控

```rust
let pool = PacketPool::with_capacity(100)?;

// 预热：提前分配 50 个 Packet
pool.warm_up(50)?;

// 使用一段时间后查看统计
let stats = pool.stats();
println!("总分配: {}", stats.base.total_allocations.load(Ordering::Relaxed));
println!("池分配: {}", stats.base.pooled_allocations.load(Ordering::Relaxed));
println!("池使用率: {:.2}%", stats.base.utilization_rate() * 100.0);
println!("内存效率: {:.2}%", stats.memory_efficiency() * 100.0);
```

---

## 5. 内存管理

### 5.1 内存布局

```
单个 Packet 内存布局：
┌────────────┬───────────────────────────┬──────────────────┐
│ Header     │   Data Section           │   Free Space     │
│ Reserve    │   (payload data)         │                 │
├────────────┼───────────────────────────┼──────────────────┤
│ 128B      │   variable              │   variable       │
│(config)   │   (length)             │   (capacity-len) │
└────────────┴───────────────────────────┴──────────────────┘
0           ^                        ^                 ^ capacity
          header_reserve           length

总容量 = header_reserve + packet_size + trailer_reserve
```

### 5.2 内存估算

```rust
/// 估算池的内存占用
pub fn estimate_memory_usage(config: &PacketPoolConfig) -> usize {
    let packet_size = config.header_reserve + config.packet_size + config.trailer_reserve;

    // 每个 Packet 的内存
    let per_packet = std::mem::size_of::<Packet>() + packet_size;

    // 池的总内存（考虑最大容量）
    per_packet * config.pool.capacity
}

// 示例：100 个 1514 字节 Packet 的池
// 约：~800 字节 (Packet) + 1642 字节 (buffer) ≈ 2.4KB per Packet
// 总计：~240KB
```

---

## 6. 性能考虑

### 6.1 优化策略

| 策略 | 说明 | 适用场景 |
|------|------|----------|
| LIFO 分配 | 利用 CPU 缓存局部性 | 低延迟场景 |
| 批量操作 | 一次获取/归还多个对象 | 批量处理 |
| 线程本地池 | 每线程小池 + 主池后端 | 多线程环境 |
| 预分配 | 系统启动时预热池 | 避免运行时分配 |

### 6.2 性能指标

| 指标 | 说明 | 计算方式 |
|------|------|----------|
| 分配延迟 | acquire() 的平均耗时 | 统计平均时间 |
| 归还延迟 | release() 的平均耗时 | 统计平均时间 |
| 命中率 | 池分配次数 / 总分配次数 | pooled / total |
| 利用率 | 使用中 / 总数 | active / (active + idle) |

### 6.3 扩展方向

1. **无锁实现**：使用无锁队列替代 Mutex<Vec>
2. **分层池**：按对象大小分层，减少内存碎片
3. **压缩池**：定期压缩空闲列表，释放内存
4. **监控集成**：集成日志/指标系统
