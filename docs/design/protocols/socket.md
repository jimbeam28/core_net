# Socket API 详细设计文档

## 1. 协议概述

### 1.1 背景与历史

**协议全称和定义：**
- Socket API（套接字应用程序编程接口）
- 位于应用程序与网络协议栈之间的接口层
- 提供统一的网络通信抽象，支持多种协议族和 socket 类型

**为什么需要 Socket API？**

Socket API 解决了应用程序如何与网络协议栈交互的问题：
1. **统一接口**：为不同协议（TCP、UDP、原始 IP）提供一致的编程接口
2. **协议抽象**：应用程序无需关心底层协议细节
3. **资源管理**：管理网络连接的生命周期和状态
4. **数据缓冲**：提供发送/接收缓冲区管理

**历史背景：**
- **1982年**：BSD 4.2 首次引入 Berkeley Sockets API
- **1996年**：IEEE Std 1003.1g 标准化协议独立接口（PII）
- **2001年**：被纳入 IEEE 1003.1-2001 POSIX 基础规范
- **相关 RFC**：
  - [RFC 3493](https://www.rfc-editor.org/rfc/rfc3493) - Basic Socket Interface Extensions for IPv6
  - [RFC 3542](https://www.rfc-editor.org/rfc/rfc3542) - Advanced Sockets API for IPv6

### 1.2 设计原理

Socket API 基于"一切皆文件"的 Unix 哲学，将网络通信端点抽象为可读写的文件描述符。

**核心架构：**
```
┌─────────────────────────────────────────────────────────────┐
│                      应用程序                                │
│  ┌─────────┐  ┌─────────┐  ┌─────────┐  ┌─────────┐        │
│  │Socket A │  │Socket B │  │Socket C │  │Socket D │        │
│  └────┬────┘  └────┬────┘  └────┬────┘  └────┬────┘        │
└───────┼────────────┼────────────┼────────────┼──────────────┘
        │            │            │            │
        └────────────┴────────────┴────────────┘
                             │
┌────────────────────────────▼─────────────────────────────────┐
│                    Socket Layer                              │
│  - Socket 表管理与查找                                        │
│  - 地址绑定与路由                                             │
│  - 连接状态管理                                               │
└────────────────────────────┬─────────────────────────────────┘
                             │
        ┌────────────────────┼────────────────────┐
        │                    │                    │
┌───────▼───────┐  ┌────────▼────────┐  ┌───────▼───────┐
│     TCP       │  │      UDP        │  │   Raw IP      │
│   (SOCK_STREAM)│  │  (SOCK_DGRAM)   │  │ (SOCK_RAW)    │
└───────┬───────┘  └────────┬────────┘  └───────┬───────┘
        │                    │                    │
        └────────────────────┴────────────────────┘
                             │
┌────────────────────────────▼─────────────────────────────────┐
│                   IP Layer (IPv4/IPv6)                       │
└────────────────────────────┬─────────────────────────────────┘
                             │
┌────────────────────────────▼─────────────────────────────────┐
│              Interface Layer (Queue-based)                   │
└─────────────────────────────────────────────────────────────┘
```

**关键特点：**
1. **协议族独立**：支持 IPv4、IPv6 等多种协议族
2. **类型抽象**：流式（TCP）、数据报（UDP）、原始套接字
3. **异步 I/O**：支持非阻塞和多路复用
4. **缓冲管理**：独立的发送和接收队列

---

## 2. Socket 结构定义

### 2.1 Socket 描述符

Socket 使用 `SocketFd` 类型作为唯一标识符：

```rust
/// Socket 文件描述符
///
/// 内部维护一个递增的整数，类似于 Linux 的 fd 分配机制
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SocketFd(pub u32);

impl SocketFd {
    /// 无效的 Socket 描述符
    pub const INVALID: Self = Self(u32::MAX);

    /// 标准输入（保留，本实现不使用）
    pub const STDIN: Self = Self(0);

    /// 标准输出（保留，本实现不使用）
    pub const STDOUT: Self = Self(1);

    /// 标准错误（保留，本实现不使用）
    pub const STDERR: Self = Self(2);

    /// 第一个可用的 Socket 描述符
    pub const FIRST_AVAILABLE: Self = Self(3);
}
```

### 2.2 协议族与类型

```rust
/// 协议族 (Address Family)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AddressFamily {
    /// IPv4 协议族
    AF_INET,
    /// IPv6 协议族
    AF_INET6,
}

/// Socket 类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SocketType {
    /// 流式套接字 (TCP)
    SOCK_STREAM,
    /// 数据报套接字 (UDP)
    SOCK_DGRAM,
}

/// Socket 协议
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SocketProtocol {
    /// 默认协议 (0)
    Default,
    /// ICMP 协议 (1)
    ICMP,
    /// TCP 协议 (6)
    TCP,
    /// UDP 协议 (17)
    UDP,
}
```

### 2.3 Socket 地址

```rust
/// Socket 地址枚举
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SocketAddr {
    V4(SocketAddrV4),
    V6(SocketAddrV6),
}

/// IPv4 Socket 地址
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SocketAddrV4 {
    /// IP 地址
    pub ip: Ipv4Addr,
    /// 端口号
    pub port: u16,
}

/// IPv6 Socket 地址
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SocketAddrV6 {
    /// IP 地址
    pub ip: Ipv6Addr,
    /// 端口号
    pub port: u16,
    /// 流标签
    pub flowinfo: u32,
    /// 范围 ID
    pub scope_id: u32,
}
```

### 2.4 Socket 状态（TCP）

```rust
/// TCP 连接状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TcpState {
    /// 不存在
    Closed,
    /// 正在建立连接
    Listen,
    /// SYN 已发送
    SynSent,
    /// SYN 已接收
    SynReceived,
    /// 连接已建立
    Established,
    /// 正在关闭
    FinWait1,
    /// 半关闭状态
    FinWait2,
    /// 对方已关闭
    CloseWait,
    /// FIN 已发送
    Closing,
    /// 等待 FIN
    LastAck,
    /// 等待远程关闭
    TimeWait,
}
```

---

## 3. Socket 表项管理

### 3.0 状态变量

Socket 层维护的状态变量：

| 变量名称 | 类型 | 用途 | 初始值 |
|---------|------|------|--------|
| next_fd | u32 | 下一个可分配的 fd | 3 |
| sockets | HashMap<SocketFd, SocketEntry> | Socket 表项集合 | 空 |

### 3.1 Socket 表项

```rust
/// Socket 表项
///
/// 每个 Socket 对应一个表项，包含其完整的状态信息
pub struct SocketEntry {
    /// Socket 文件描述符
    pub fd: SocketFd,

    /// 协议族
    pub family: AddressFamily,

    /// Socket 类型
    pub socket_type: SocketType,

    /// 协议
    pub protocol: SocketProtocol,

    /// Socket 状态（仅 TCP 有效）
    pub state: SocketState,

    /// 绑定的本地地址
    pub local_addr: Option<SocketAddr>,

    /// 连接的对端地址（仅面向连接的 Socket）
    pub peer_addr: Option<SocketAddr>,

    /// 接收缓冲区
    pub rx_buffer: VecDeque<Vec<u8>>,

    /// 发送缓冲区
    pub tx_buffer: VecDeque<Vec<u8>>,

    /// 接收缓冲区大小限制（字节）
    pub rx_buffer_size: usize,

    /// 发送缓冲区大小限制（字节）
    pub tx_buffer_size: usize,

    /// Socket 选项
    pub options: SocketOptions,

    /// 是否阻塞模式
    pub blocking: bool,

    /// 监听队列（仅 SOCK_STREAM 且状态为 Listen 时有效）
    pub listen_queue: Option<ListenQueue>,
}

/// Socket 状态（内部表示）
pub enum SocketState {
    /// TCP 连接状态
    Tcp(TcpState),
    /// UDP 无状态
    Udp,
}

/// Socket 选项
#[derive(Debug, Clone)]
pub struct SocketOptions {
    /// SO_REUSEADDR
    pub reuse_addr: bool,
    /// SO_REUSEPORT
    pub reuse_port: bool,
    /// SO_BROADCAST
    pub broadcast: bool,
    /// SO_KEEPALIVE
    pub keepalive: bool,
    /// SO_RCVBUF
    pub rcvbuf: usize,
    /// SO_SNDBUF
    pub sndbuf: usize,
}

impl Default for SocketOptions {
    fn default() -> Self {
        Self {
            reuse_addr: false,
            reuse_port: false,
            broadcast: false,
            keepalive: false,
            rcvbuf: DEFAULT_SOCKET_BUFFER_SIZE,
            sndbuf: DEFAULT_SOCKET_BUFFER_SIZE,
        }
    }
}
```

### 3.2 监听队列

```rust
/// 监听队列（用于 TCP 服务端）
pub struct ListenQueue {
    /// 最大挂起连接数
    pub backlog: usize,

    /// 已完成三次握手、等待 accept 的连接队列
    ///
    /// 每个 SocketFd 代表一个已建立的连接
    pub pending_connections: VecDeque<SocketFd>,

    /// 正在完成三次握手的连接队列（SYN RCVD 状态）
    pub half_open: HashSet<SocketFd>,
}
```

---

## 4. Socket API 函数

### 4.1 socket() - 创建 Socket

```rust
/// 创建一个新的 Socket
///
/// # 参数
/// - `domain`: 协议族（AF_INET/AF_INET6）
/// - `type`: Socket 类型（SOCK_STREAM/SOCK_DGRAM）
/// - `protocol`: 协议编号（通常为 0 表示自动选择）
///
/// # 返回
/// - 成功：返回 SocketFd
/// - 失败：返回 SocketError
pub fn socket(
    domain: AddressFamily,
    type: SocketType,
    protocol: SocketProtocol,
) -> Result<SocketFd, SocketError>;
```

**处理流程：**
1. 验证协议族与 Socket 类型的组合是否合法
2. 分配新的 SocketFd（递增计数器）
3. 创建 `SocketEntry` 并初始化
4. 将 SocketEntry 插入 sockets 表
5. 返回 SocketFd

**错误处理：**
- `SocketError::InvalidProtocol` - 不支持的协议族/类型组合
- `SocketError::TableFull` - Socket 表已满

---

### 4.2 bind() - 绑定地址

```rust
/// 绑定 Socket 到本地地址
///
/// # 参数
/// - `fd`: Socket 文件描述符
/// - `addr`: 本地地址
///
/// # 返回
/// - 成功：Ok(())
/// - 失败：返回 SocketError
pub fn bind(
    fd: SocketFd,
    addr: &SocketAddr,
) -> Result<(), SocketError>;
```

**处理流程：**
1. 查找 SocketEntry
2. 验证协议族与地址类型匹配
3. 验证端口未被占用（除非设置了 SO_REUSEADDR）
4. 更新 `local_addr` 字段

**错误处理：**
- `SocketError::InvalidFd` - 无效的 Socket 描述符
- `SocketError::AlreadyBound` - Socket 已绑定
- `SocketError::AddrInUse` - 地址已被占用
- `SocketError::InvalidProtocol` - 地址类型与协议族不匹配

---

### 4.3 listen() - 开始监听

```rust
/// 将 Socket 置为监听模式（仅面向连接的 Socket）
///
/// # 参数
/// - `fd`: Socket 文件描述符
/// - `backlog`: 挂起连接队列的最大长度
///
/// # 返回
/// - 成功：Ok(())
/// - 失败：返回 SocketError
pub fn listen(
    fd: SocketFd,
    backlog: usize,
) -> Result<(), SocketError>;
```

**处理流程：**
1. 验证 Socket 类型为 SOCK_STREAM
2. 验证 Socket 已绑定（local_addr 不为空）
3. 创建 `ListenQueue`
4. 更新 `state` 为 `Listen`

**错误处理：**
- `SocketError::InvalidFd` - 无效的 Socket 描述符
- `SocketError::NotBound` - Socket 未绑定
- `SocketError::NotStream` - Socket 不是流式套接字

---

### 4.4 accept() - 接受连接

```rust
/// 接受一个挂起的连接（仅面向连接的 Socket）
///
/// # 参数
/// - `fd`: 监听 Socket 文件描述符
///
/// # 返回
/// - 成功：返回新的 SocketFd（代表已建立的连接）
/// - 失败：返回 SocketError
pub fn accept(fd: SocketFd) -> Result<SocketFd, SocketError>;
```

**处理流程：**
1. 验证 Socket 状态为 Listen
2. 从 `listen_queue.pending_connections` 中取出一个 SocketFd
3. 返回该 SocketFd

**错误处理：**
- `SocketError::InvalidFd` - 无效的 Socket 描述符
- `SocketError::NotListening` - Socket 未处于监听状态
- `SocketError::WouldBlock` - 无挂起连接且为非阻塞模式

---

### 4.5 connect() - 发起连接

```rust
/// 发起到对端的连接（仅面向连接的 Socket）
///
/// # 参数
/// - `fd`: Socket 文件描述符
/// - `addr`: 对端地址
///
/// # 返回
/// - 成功：Ok(())
/// - 失败：返回 SocketError
pub fn connect(
    fd: SocketFd,
    addr: &SocketAddr,
) -> Result<(), SocketError>;
```

**处理流程：**
1. 验证 Socket 类型为 SOCK_STREAM
2. 验证协议族与地址类型匹配
3. 更新 `peer_addr` 字段
4. 触发 TCP 三次握手（发送 SYN）
5. 更新 `state` 为 `SynSent`
6. **阻塞模式**：等待连接建立或失败
7. **非阻塞模式**：立即返回 `SocketError::InProgress`

**资源更新：**
- `peer_addr`: 设置为对端地址
- `state`: `Closed` → `SynSent` → `Established`

**错误处理：**
- `SocketError::InvalidFd` - 无效的 Socket 描述符
- `SocketError::AlreadyConnected` - Socket 已连接
- `SocketError::InvalidProtocol` - 地址类型不匹配
- `SocketError::ConnRefused` - 连接被拒绝
- `SocketError::ConnTimedOut` - 连接超时
- `SocketError::InProgress` - 非阻塞模式下连接正在进行

---

### 4.6 send() / sendto() - 发送数据

```rust
/// 发送数据（面向连接的 Socket）
pub fn send(
    fd: SocketFd,
    buf: &[u8],
    flags: SendFlags,
) -> Result<usize, SocketError>;

/// 发送数据（无连接的 Socket）
pub fn sendto(
    fd: SocketFd,
    buf: &[u8],
    flags: SendFlags,
    dest_addr: &SocketAddr,
) -> Result<usize, SocketError>;
```

**处理流程（TCP）：**
1. 验证 Socket 状态为 Established
2. 检查发送缓冲区空间
3. 将数据加入 `tx_buffer`
4. 触发 TCP 层发送

**处理流程（UDP）：**
1. 验证 Socket 已绑定
2. 检查发送缓冲区空间
3. 将数据加入 `tx_buffer`
4. 触发 UDP 层封装和发送

---

### 4.7 recv() / recvfrom() - 接收数据

```rust
/// 接收数据（面向连接的 Socket）
pub fn recv(
    fd: SocketFd,
    buf: &mut [u8],
    flags: RecvFlags,
) -> Result<usize, SocketError>;

/// 接收数据（无连接的 Socket）
pub fn recvfrom(
    fd: SocketFd,
    buf: &mut [u8],
    flags: RecvFlags,
    src_addr: &mut Option<SocketAddr>,
) -> Result<usize, SocketError>;
```

**处理流程：**
1. 从 `rx_buffer` 中取出数据包
2. 复制数据到用户缓冲区
3. 返回实际复制的字节数

**错误处理：**
- `SocketError::InvalidFd` - 无效的 Socket 描述符
- `SocketError::WouldBlock` - 接收缓冲区空且为非阻塞模式
- `SocketError::NotConnected` - Socket 未连接（recv）

---

### 4.8 close() - 关闭 Socket

```rust
/// 关闭 Socket
pub fn close(fd: SocketFd) -> Result<(), SocketError>;
```

**处理流程：**
1. 查找 SocketEntry
2. **TCP 且状态为 Established/Listen**：发送 FIN，启动关闭流程
3. 从 sockets 表中移除 SocketEntry
4. 释放所有资源（缓冲区、定时器等）

---

## 5. 核心数据结构

### 5.1 Socket 管理器

```rust
/// Socket 管理器
///
/// 管理所有 Socket 的创建、查找、销毁
pub struct SocketManager {
    /// 下一个可分配的 fd
    next_fd: u32,

    /// Socket 表项映射
    sockets: HashMap<SocketFd, SocketEntry>,

    /// 已绑定的地址集合（用于检查地址冲突）
    bound_addresses: HashMap<(AddressFamily, u16), SocketFd>,
}
```

### 5.2 错误类型

```rust
/// Socket 错误类型
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SocketError {
    /// 无效的 Socket 描述符
    InvalidFd,
    /// 不支持的协议
    InvalidProtocol,
    /// Socket 已绑定
    AlreadyBound,
    /// Socket 未绑定
    NotBound,
    /// 地址已被占用
    AddrInUse,
    /// 地址不可用
    AddrNotAvailable,
    /// Socket 不是流式套接字
    NotStream,
    /// Socket 状态无效
    InvalidState,
    /// Socket 未监听
    NotListening,
    /// Socket 已连接
    AlreadyConnected,
    /// Socket 未连接
    NotConnected,
    /// 连接被拒绝
    ConnRefused,
    /// 连接超时
    ConnTimedOut,
    /// 连接被重置
    ConnReset,
    /// 非阻塞模式下操作会阻塞
    WouldBlock,
    /// 操作正在进行中
    InProgress,
    /// 操作被中断
    Interrupted,
    /// 缓冲区空间不足
    NoBufferSpace,
    /// Socket 表已满
    TableFull,
    /// 其他错误
    Other(String),
}
```

---

## 6. 与其他模块的交互

### 6.1 模块依赖关系

```
┌─────────────────────────────────────────────────────────────┐
│                   Socket Layer (src/socket/)                │
│  - socket.rs: Socket API 函数                               │
│  - manager.rs: SocketManager                                │
│  - types.rs: 类型定义                                        │
└─────────────────────────────┬───────────────────────────────┘
                              │
         ┌────────────────────┼────────────────────┐
         │                    │                    │
         ▼                    ▼                    ▼
┌─────────────────┐  ┌─────────────────┐  ┌─────────────────┐
│   Common 模块    │  │  Interface 模块  │  │  Protocol 模块  │
│ - error.rs      │  │ - iface.rs      │  │ - tcp/          │
│ - addr.rs       │  │ - manager.rs    │  │   udp/          │
│ - queue.rs      │  │                 │  │   ip/           │
└─────────────────┘  └─────────────────┘  └─────────────────┘
         │                    │                    │
         └────────────────────┼────────────────────┘
                              │
                              ▼
                    ┌─────────────────┐
                    │  SystemContext  │
                    │  - socket_mgr   │
                    │  - tcp_connections│
                    │  - tcp_sockets   │
                    │  - udp_ports     │
                    └─────────────────┘
```

### 6.2 协议层集成架构

#### 6.2.1 数据流向总览

```
┌─────────────────────────────────────────────────────────────────────┐
│                        Socket API 层                                │
│  ┌─────────┐  ┌─────────┐  ┌─────────┐  ┌─────────┐              │
│  │socket() │  │ bind()  │  │connect()│  │ send()  │ ...         │
│  └────┬────┘  └────┬────┘  └────┬────┘  └────┬────┘              │
└───────┼────────────┼────────────┼────────────┼──────────────────┘
        │            │            │            │
        ▼            ▼            ▼            ▼
┌─────────────────────────────────────────────────────────────────┐
│                       SocketManager                               │
│  - Socket 表管理 (HashMap<SocketFd, SocketEntry>)               │
│  - 地址绑定表 (HashMap<(Family, Port), HashSet<SocketFd>>)      │
│  - TCP 连接映射 (HashMap<TcpConnectionId, SocketFd>)            │
│  - UDP 端口绑定 (UdpPortManager)                                  │
└───────┬───────────────────────────────┬───────────────────────────┘
        │                               │
        │ 发送路径                       │ 接收路径
        ▼                               ▼
┌───────────────────────┐     ┌─────────────────────────────────────┐
│   TCP/UDP 协议层       │     │   TCP/UDP 协议层                     │
│  - tcp::process.rs    │     │  - tcp::process_tcp_packet()         │
│  - udp::process.rs    │     │  - udp::process_udp_packet()        │
│  - 封装数据报         │     │  - 解析数据报                        │
└───────┬───────────────┘     │  - 查找目标 Socket                   │
        │                       │  - 推送数据到 rx_buffer              │
        ▼                       └─────────────────────────────────────┘
┌───────────────────────┐
│   IP 层               │
│  - ipv4/ipv6 封装     │
└───────┬───────────────┘
        ▼
┌───────────────────────┐
│   接口层              │
│  - Scheduler 调度     │
└───────────────────────┘
```

#### 6.2.2 发送路径详解

**TCP 发送流程：**
```
应用调用
    │
    ▼
SocketManager::send(fd, data)
    │ 1. 查找 SocketEntry
    │ 2. 检查状态 (Established)
    │ 3. 写入 tx_buffer
    │ 4. 调用 TCP 层发送
    ▼
TcpConnectionManager::send_data(conn_id, data)
    │ 1. 查找 TCB
    │ 2. 分段 (MSS)
    │ 3. 分配序列号
    │ 4. 封装 TCP 报文
    ▼
IP 层封装 → 接口发送
```

**UDP 发送流程：**
```
应用调用
    │
    ▼
SocketManager::sendto(fd, data, dest_addr)
    │ 1. 查找 SocketEntry
    │ 2. 检查绑定状态
    │ 3. 写入 tx_buffer
    │ 4. 调用 UDP 层封装
    ▼
udp::encapsulate_udp_datagram(src_port, dst_port, data)
    │ 1. 构建 UDP 头部
    │ 2. 计算校验和
    ▼
IP 层封装 → 接口发送
```

#### 6.2.3 接收路径详解

**TCP 接收流程：**
```
网络接收 → IP 层解封装
    │
    ▼
tcp::process_tcp_packet(packet, src_addr, dst_addr, context)
    │ 1. 解析 TCP 报文
    │ 2. 查找 TcpConnectionId 对应的连接
    │ 3. 更新 TCB 状态
    │ 4. 提取数据载荷
    │ 5. 查找对应的 SocketFd
    ▼
SocketManager::deliver_tcp_data(conn_id, data, src_addr)
    │ 1. 查找 SocketEntry
    │ 2. 写入 rx_buffer
    ▼
应用调用 recv() 读取数据
```

**UDP 接收流程：**
```
网络接收 → IP 层解封装
    │
    ▼
udp::process_udp_packet(packet, src_addr, dst_addr, context)
    │ 1. 解析 UDP 数据报
    │ 2. 查找目标端口
    │ 3. 查找对应的 SocketFd
    ▼
SocketManager::deliver_udp_data(socket_fd, data, src_addr, src_port)
    │ 1. 查找 SocketEntry
    │ 2. 写入 rx_buffer
    │ 3. 记录源地址信息
    ▼
应用调用 recvfrom() 读取数据和源地址
```

### 6.3 与 SystemContext 的集成

```rust
// src/context.rs
pub struct SystemContext {
    // ... 现有字段 ...

    /// Socket 管理器
    pub socket_mgr: Arc<Mutex<SocketManager>>,

    /// TCP 连接管理器
    pub tcp_connections: Arc<Mutex<TcpConnectionManager>>,

    /// TCP Socket 管理器
    pub tcp_sockets: Arc<Mutex<TcpSocketManager>>,

    /// UDP 端口管理器
    pub udp_ports: Arc<Mutex<UdpPortManager>>,
}
```

### 6.4 Socket-连接映射机制

为了将接收到的协议数据分发到正确的 Socket，需要维护以下映射关系：

```rust
// SocketManager 中的映射表
pub struct SocketManager {
    // ... 现有字段 ...

    /// TCP 连接到 Socket 的映射
    /// Key: TcpConnectionId, Value: SocketFd
    tcp_connection_map: HashMap<TcpConnectionId, SocketFd>,

    /// SocketFd 到连接信息的反向映射
    /// Key: SocketFd, Value: Option<TcpConnectionId>
    socket_connection_map: HashMap<SocketFd, Option<TcpConnectionId>>,
}
```

**映射操作：**
- **connect() 成功后**：建立 `TcpConnectionId → SocketFd` 映射
- **accept() 返回后**：建立新连接的映射
- **数据接收时**：通过 `TcpConnectionId` 查找 `SocketFd`
- **close() 时**：清理映射关系

### 6.5 事件通知机制

Socket 层需要监听协议层的事件以更新状态：

```rust
// 协议层定义的事件类型
pub enum SocketEvent {
    /// TCP 连接已建立
    TcpConnected { conn_id: TcpConnectionId },
    /// TCP 连接已关闭
    TcpClosed { conn_id: TcpConnectionId },
    /// 接收到数据
    DataReceived { data: Vec<u8>, src_addr: SocketAddr },
}

// SocketManager 处理事件
impl SocketManager {
    pub fn handle_tcp_event(&mut self, event: SocketEvent) {
        match event {
            SocketEvent::TcpConnected { conn_id } => {
                if let Some(fd) = self.tcp_connection_map.get(&conn_id) {
                    if let Some(entry) = self.sockets.get_mut(fd) {
                        entry.state = SocketState::Tcp(TcpState::Established);
                    }
                }
            }
            // ... 其他事件处理
        }
    }
}
```

---

## 7. 配置参数

```rust
/// Socket 配置
pub struct SocketConfig {
    /// Socket 表最大容量
    pub max_sockets: usize,  // 默认: 1024

    /// 默认接收缓冲区大小
    pub default_rx_buffer_size: usize,  // 默认: 8192

    /// 默认发送缓冲区大小
    pub default_tx_buffer_size: usize,  // 默认: 8192

    /// 最小接收缓冲区大小
    pub min_rx_buffer_size: usize,  // 默认: 256

    /// 最小发送缓冲区大小
    pub min_tx_buffer_size: usize,  // 默认: 256

    /// 最大接收缓冲区大小
    pub max_rx_buffer_size: usize,  // 默认: 65536

    /// 最大发送缓冲区大小
    pub max_tx_buffer_size: usize,  // 默认: 65536

    /// 默认监听队列长度
    pub default_listen_backlog: usize,  // 默认: 128
}

impl Default for SocketConfig {
    fn default() -> Self {
        Self {
            max_sockets: 1024,
            default_rx_buffer_size: 8192,
            default_tx_buffer_size: 8192,
            min_rx_buffer_size: 256,
            min_tx_buffer_size: 256,
            max_rx_buffer_size: 65536,
            max_tx_buffer_size: 65536,
            default_listen_backlog: 128,
        }
    }
}

// 常量定义
const DEFAULT_SOCKET_BUFFER_SIZE: usize = 8192;
const MAX_SOCKET_TABLE_SIZE: usize = 1024;
```

---

## 8. 测试场景

### 8.1 基本功能测试

1. **Socket 创建与销毁**
   - 创建 TCP Socket（AF_INET, SOCK_STREAM）
   - 创建 UDP Socket（AF_INET, SOCK_DGRAM）
   - 关闭 Socket 并验证资源释放

2. **地址绑定**
   - 绑定到可用端口
   - 绑定到特定 IP 地址
   - SO_REUSEADDR 选项测试

3. **TCP 连接**
   - 主动连接（connect）
   - 被动连接（listen + accept）
   - 三次握手验证

4. **数据传输**
   - TCP send/recv
   - UDP sendto/recvfrom
   - 大数据包传输

### 8.2 边界情况测试

1. **缓冲区边界**
   - 发送超过缓冲区容量的数据
   - 接收缓冲区满时的行为

2. **地址冲突**
   - 绑定已占用的地址
   - 同时绑定同一端口（SO_REUSEADDR）

3. **连接状态**
   - 未连接的 Socket 调用 send
   - 已连接的 Socket 调用 connect

### 8.3 异常情况测试

1. **无效操作**
   - 无效的 SocketFd
   - 错误的协议族/类型组合

2. **非阻塞模式**
   - 无数据可读时 recv
   - 缓冲区满时 send

3. **连接异常**
   - 连接超时
   - 连接被拒绝
   - 连接重置

---

## 9. 安全考虑

### 9.1 资源限制

**Socket 表耗尽攻击：**
- 攻击者创建大量 Socket 耗尽系统资源
- 防御：
  - 设置 `max_sockets` 上限
  - 实现每用户/进程的配额限制

### 9.2 缓冲区溢出

**大包攻击：**
- 攻击者发送超大数据包耗尽内存
- 防御：
  - 限制单个 Socket 的缓冲区大小
  - 限制全局内存使用量

### 9.3 实现建议

1. **输入验证：** 所有用户输入必须进行验证
2. **边界检查：** 缓冲区操作前检查边界
3. **资源清理：** 确保 close() 释放所有资源
4. **日志记录：** 记录关键操作和错误

---

## 10. 实现计划

### 10.1 第一阶段：基础框架

1. 实现 `types.rs` - 类型定义
2. 实现 `SocketEntry` 结构
3. 实现 `SocketManager` 框架
4. 实现 `socket()` 和 `close()`

### 10.2 第二阶段：UDP 支持

1. 实现 `bind()` 和 `bind()` 验证
2. 实现 `sendto()` 和 `recvfrom()`
3. 实现与 UDP 模块的集成
4. UDP Socket 测试

### 10.3 第三阶段：TCP 基础支持

1. 实现 `listen()` 和 `accept()`
2. 实现 `connect()`（与 TCP 模块集成）
3. 实现 `send()` 和 `recv()`
4. TCP 状态机集成

### 10.4 第四阶段：高级功能

1. 实现非阻塞模式
2. 实现 Socket 选项
3. 实现多路复用（select/poll）
4. 性能优化

---

## 11. 参考资料

1. **IEEE Std 1003.1-2017** - POSIX Standard (Unix API)
2. **[RFC 3493](https://www.rfc-editor.org/rfc/rfc3493)** - Basic Socket Interface Extensions for IPv6
3. **[RFC 3542](https://www.rfc-editor.org/rfc/rfc3542)** - Advanced Sockets API for IPv6
4. **W. Richard Stevens** - "Unix Network Programming, Volume 1"
5. **Linux Kernel Source** - `net/socket.c`, `net/ipv4/`

---

## 附录：Socket API 快速参考

| 函数 | TCP | UDP | 说明 |
|------|-----|-----|------|
| socket() | ✓ | ✓ | 创建 Socket |
| bind() | ✓ | ✓ | 绑定地址 |
| listen() | ✓ | - | 开始监听 |
| accept() | ✓ | - | 接受连接 |
| connect() | ✓ | - | 发起连接 |
| send() | ✓ | - | 发送数据 |
| sendto() | - | ✓ | 发送数据（带地址）|
| recv() | ✓ | - | 接收数据 |
| recvfrom() | - | ✓ | 接收数据（带地址）|
| close() | ✓ | ✓ | 关闭 Socket |
