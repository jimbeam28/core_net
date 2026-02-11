# 模拟网卡设计

## 1. 概述

模拟网卡（Simulated NIC）模块用于在纯模拟环境中提供类似真实网卡的接口，但不进行实际的网络收发。它是连接测试注入器、收发队列和协议处理引擎的桥梁。

## 2. 模块定位

```
┌──────────────┐         ┌──────────────┐         ┌──────────────┐
│  测试注入    │  ───>  │  模拟网卡    │  ───>  │  接收队列    │
│  (用户代码）  │  Rx    │  (SimNIC)    │  RxQ   │  (队列)       │
└──────────────┘         └──────────────┘         └──────────────┘

┌──────────────┐         ┌──────────────┐         ┌──────────────┐
│  结果读取    │  <──  │  模拟网卡    │  <──  │  发送队列    │
│  (用户代码）  │  Tx    │  (SimNIC)    │  TxQ   │  (队列)       │
└──────────────┘         └──────────────┘         └──────────────┘
```

## 3. 数据结构

### 3.1 核心结构

```rust
/// 模拟网卡
pub struct SimNic {
    name: String,              // 网卡名称
    mac_addr: MacAddr,         // MAC地址
    mtu: usize,               // MTU（默认1500）
    rx_queue: SafeQueue<Packet>,   // 接收队列
    tx_queue: SafeQueue<Packet>,   // 发送队列
    stats: NicStats,           // 统计信息
}

/// 网卡统计
pub struct NicStats {
    pub rx_packets: u64,       // 接收包数
    pub tx_packets: u64,       // 发送包数
    pub rx_bytes: u64,         // 接收字节数
    pub tx_bytes: u64,         // 发送字节数
    pub rx_errors: u64,        // 接收错误数
    pub tx_errors: u64,        // 发送错误数
}
```

### 3.2 收包参数

```rust
/// 收包参数
pub struct RxParams<'a> {
    /// 报文数据
    pub data: &'a [u8],

    /// 报文长度
    pub len: usize,

    /// 时间戳（可选，默认为当前时间）
    pub timestamp: Option<Instant>,
}

impl<'a> RxParams<'a> {
    /// 从切片创建
    pub fn from_slice(data: &'a [u8]) -> Self {
        Self {
            data,
            len: data.len(),
            timestamp: None,
        }
    }

    /// 带时间戳创建
    pub fn with_timestamp(mut self, ts: Instant) -> Self {
        self.timestamp = Some(ts);
        self
    }
}
```

### 3.3 发包参数

```rust
/// 发包参数
pub struct TxParams {
    /// 超时时间（None=无限等待）
    pub timeout: Option<Duration>,

    /// 是否自动移除包头（用于调试）
    pub strip_header: bool,
}

impl TxParams {
    pub fn new() -> Self {
        Self {
            timeout: None,
            strip_header: false,
        }
    }

    pub fn with_timeout(mut self, dur: Duration) -> Self {
        self.timeout = Some(dur);
        self
    }
}
```

## 4. 接口定义

### 4.1 SimNic 核心接口

```rust
impl SimNic {
    /// 创建新的模拟网卡
    pub fn new(
        name: String,
        mac_addr: MacAddr,
        rx_queue: SafeQueue<Packet>,
        tx_queue: SafeQueue<Packet>,
    ) -> Self;

    /// 创建带MTU的网卡
    pub fn with_mtu(
        name: String,
        mac_addr: MacAddr,
        mtu: usize,
        rx_queue: SafeQueue<Packet>,
        tx_queue: SafeQueue<Packet>,
    ) -> Self;

    /// 获取网卡名称
    pub fn name(&self) -> &str;

    /// 获取MAC地址
    pub fn mac_addr(&self) -> MacAddr;

    /// 获取MTU
    pub fn mtu(&self) -> usize;

    /// 设置MTU
    pub fn set_mtu(&mut self, mtu: usize);

    /// 获取统计信息
    pub fn stats(&self) -> &NicStats;

    /// 重置统计
    pub fn reset_stats(&mut self);

    /// 网卡是否启用
    pub fn is_up(&self) -> bool;

    /// 启用网卡
    pub fn bring_up(&mut self);

    /// 禁用网卡
    pub fn bring_down(&mut self);
}
```

### 4.2 收包接口

```rust
impl SimNic {
    /// 接收报文（用户调用此方法模拟网卡收到数据）
    ///
    /// # 参数
    /// - params: 收包参数（数据、长度、时间戳）
    ///
    /// # 返回
    /// - Ok(()): 成功投递到接收队列
    /// - Err(NicError): 投递失败（网卡未启用、队列满等）
    pub fn receive(&mut self, params: RxParams) -> Result<(), NicError>;

    /// 接收切片（便捷方法）
    pub fn receive_slice(&mut self, data: &[u8]) -> Result<(), NicError> {
        self.receive(RxParams::from_slice(data))
    }

    /// 批量接收
    pub fn receive_batch(&mut self, packets: &[RxParams]) -> Result<usize, NicError> {
        let mut count = 0;
        for params in packets {
            self.receive(*params)?;
            count += 1;
        }
        Ok(count)
    }
}
```

### 4.3 发包接口

```rust
impl SimNic {
    /// 发送报文（用户调用此方法获取要发送的数据）
    ///
    /// # 参数
    /// - params: 发包参数（超时等）
    ///
    /// # 返回
    /// - Ok(Vec<u8>): 发送报文的buffer
    /// - Err(NicError): 发送失败（超时、网卡未启用等）
    pub fn send(&mut self, params: TxParams) -> Result<Vec<u8>, NicError>;

    /// 非阻塞发送
    pub fn try_send(&mut self) -> Result<Option<Vec<u8>>, NicError>;

    /// 批量发送（直到队列为空）
    pub fn drain(&mut self) -> Result<Vec<Vec<u8>>, NicError> {
        let mut packets = Vec::new();
        while let Ok(Some(packet)) = self.try_send() {
            packets.push(packet);
        }
        Ok(packets)
    }
}
```

### 4.4 错误类型

```rust
/// 网卡错误
#[derive(Debug)]
pub enum NicError {
    /// 网卡未启用
    NotUp,

    /// 网卡已关闭
    Closed,

    /// 接收队列已满
    RxQueueFull,

    /// 发送队列为空
    TxQueueEmpty,

    /// 发送超时
    TxTimeout(Duration),

    /// 报文过大（超过MTU）
    PacketTooLarge { len: usize, mtu: usize },

    /// 报文过小
    PacketTooSmall,

    /// 无效参数
    InvalidParams(String),
}

impl fmt::Display for NicError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            NicError::NotUp => write!(f, "网卡未启用"),
            NicError::Closed => write!(f, "网卡已关闭"),
            NicError::RxQueueFull => write!(f, "接收队列已满"),
            NicError::TxQueueEmpty => write!(f, "发送队列为空"),
            NicError::TxTimeout(d) => write!(f, "发送超时: {:?}", d),
            NicError::PacketTooLarge { len, mtu } => {
                write!(f, "报文过大: {} > MTU({})", len, mtu)
            }
            NicError::PacketTooSmall => write!(f, "报文过小"),
            NicError::InvalidParams(msg) => {
                write!(f, "无效参数: {}", msg)
            }
        }
    }
}
```

## 5. 辅助接口

### 5.1 环回环接口

```rust
impl SimNic {
    /// 创建回环网卡（测试用）
    /// 回环模式：发送的报文自动回到接收队列
    pub fn loopback(
        name: String,
        mac_addr: MacAddr,
        queue: SafeQueue<Packet>,
    ) -> Self;

    /// 启用/禁用回环模式
    pub fn set_loopback(&mut self, enabled: bool);

    /// 是否为回环模式
    pub fn is_loopback(&self) -> bool;
}
```

### 5.2 镜像接口

```rust
impl SimNic {
    /// 设置镜像模式（复制所有收发的报文）
    pub fn set_mirror(
        &mut self,
        mirror_tx: bool,   // 镜像发送
        mirror_rx: bool,   // 镜像接收
        callback: impl Fn(&[u8]),  // 回调函数
    );

    /// 取消镜像
    pub fn clear_mirror(&mut self);
}
```

### 5.3 过滤接口

```rust
impl SimNic {
    /// 设置接收过滤器
    pub fn set_rx_filter(
        &mut self,
        filter: NicFilter,
    );

    /// 清除过滤器
    pub fn clear_filter(&mut self);
}

/// 网卡过滤器
pub enum NicFilter {
    /// 接受所有报文
    All,

    /// 仅接收指定MAC地址的报文
    MacAddr(MacAddr),

    /// 仅接收指定以太网类型的报文
    EtherType(Vec<EtherType>),

    /// 自定义过滤器
    Custom(Box<dyn Fn(&Packet) -> bool>),
}
```

## 6. 常量定义

```rust
/// 默认MTU
pub const DEFAULT_MTU: usize = 1500;

/// 最小MTU
pub const MIN_MTU: usize = 576;

/// 最大MTU（巨型帧）
pub const MAX_MTU: usize = 9000;

/// 以太网最小帧长
pub const MIN_ETH_FRAME: usize = 64;

/// 以太网最大帧长
pub const MAX_ETH_FRAME: usize = 1518;

/// 默认网卡名称前缀
pub const DEFAULT_NIC_PREFIX: &str = "sim";

/// 默认发送超时
pub const DEFAULT_TX_TIMEOUT_MS: u64 = 1000;
```

## 7. 使用场景示例

```rust
// 创建队列
let rx_queue = Arc::new(SpscQueue::new(256));
let tx_queue = Arc::new(SpscQueue::new(256));

// 创建模拟网卡
let mut nic = SimNic::new(
    "sim0".to_string(),
    MacAddr::new(0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF),
    rx_queue.clone(),
    tx_queue.clone(),
);
nic.bring_up();

// 收包：注入测试报文
let packet_data = hex::decode("fffffffffffaabbccddeeff0800...")?;
nic.receive_slice(&packet_data)?;

// 发包：获取处理结果
let sent_data = nic.send(TxParams::new())?;
// 验证sent_data是否符合预期
```
