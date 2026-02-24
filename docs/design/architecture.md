# CoreNet 网络协议栈架构设计

## 1. 项目概述

### 1.1 目标
设计并实现一个用于学习/研究的网络协议栈，支持完整的TCP/IP协议族。

### 1.2 设计目标
- **可学习性**: 代码结构清晰，便于理解网络协议原理
- **可扩展性**: 模块化设计，便于添加新协议支持
- **正确性**: 遵循RFC标准，实现正确的协议行为

### 1.3 项目状态
- ✅ 链路层完整实现（Ethernet、VLAN、ARP）
- ✅ 网络层完整实现（IPv4、IPv6、ICMP、ICMPv6、NDP）
- ✅ 传输层完整实现（TCP、UDP）
- ✅ 路由模块实现（IPv4/IPv6路由表、最长前缀匹配）
- ✅ IP分片与重组（IPv4/IPv6）
- ✅ IPv6扩展头（逐跳选项、路由、分片、目的选项）
- ✅ Socket API（bind、connect、send、recv、close等）

---

## 2. 整体架构

### 2.1 系统架构图

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                              CoreNet 网络协议栈                               │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │                      系统上下文 (SystemContext)                       │   │
│  │  ┌──────────────────────────────────────────────────────────────┐  │   │
│  │  │  Arc<Mutex<InterfaceManager>>  (网络接口)                    │  │   │
│  │  │  Arc<Mutex<ArpCache>>         (ARP 缓存)                    │  │   │
│  │  │  Arc<Mutex<EchoManager>>      (ICMP Echo)                   │  │   │
│  │  │  Arc<Mutex<TcpConnectionMgr>>(TCP 连接)                     │  │   │
│  │  │  Arc<Mutex<TcpSocketManager>> (TCP Socket)                  │  │   │
│  │  │  Arc<Mutex<UdpPortManager>>   (UDP 端口)                    │  │   │
│  │  │  Arc<Mutex<RouteTable>>       (路由表)                      │  │   │
│  │  │  Arc<Mutex<TimerHandle>>      (定时器)                      │  │   │
│  │  │  Arc<Mutex<Icmpv6Context>>    (ICMPv6上下文)                │  │   │
│  │  │  Arc<Mutex<ReassemblyTable>>  (IPv4分片重组)                │  │   │
│  │  │  Arc<Mutex<FragmentCache>>    (IPv6分片缓存)                │  │   │
│  │  │  Arc<Mutex<SocketManager>>    (Socket管理器)                │  │   │
│  │  │                   (依赖注入模式，支持 Clone)                  │  │   │
│  │  └──────────────────────────────────────────────────────────────┘  │   │
│  │       from_config() ──► 加载 interface.toml ──► 初始化所有组件      │   │
│  │       new() ──► 创建空上下文 (测试用)                               │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                        │                                     │
│                                        ▼                                     │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │                      网络接口管理 (Interface)                         │   │
│  │  ┌──────────────────────────────────────────────────────────────┐  │   │
│  │  │               全局接口管理器 (OnceLock<Mutex<>>)              │  │   │
│  │  │  ┌────────────┐  ┌────────────┐  ┌────────────┐             │  │   │
│  │  │  │   eth0     │  │    lo      │  │   eth1     │  ...        │  │   │
│  │  │  │  RxQ/TxQ   │  │  RxQ/TxQ   │  │  RxQ/TxQ   │             │  │   │
│  │  │  └────────────┘  └────────────┘  └────────────┘             │  │   │
│  │  └──────────────────────────────────────────────────────────────┘  │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                        │                                     │
│                                        ▼                                     │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │                        调度模块 (Scheduler)                          │   │
│  │  ┌──────────────────────────────────────────────────────────────┐  │   │
│  │  │                    Scheduler                                 │  │   │
│  │  │   run_all_interfaces() ──► 遍历所有接口 ──► 处理每个RxQ      │  │   │
│  │  └──────────────────────────────────────────────────────────────┘  │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                        │                                     │
│                                        ▼                                     │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │                       协议处理引擎 (Engine)                           │   │
│  │  ┌──────────────────────────────────────────────────────────────┐  │   │
│  │  │                   PacketProcessor (薄层设计)                  │  │   │
│  │  │        协议分发: 根据EtherType调用对应协议模块                │  │   │
│  │  └──────────────────────────────────────────────────────────────┘  │   │
│  │      │                          │                  │                │   │
│  │      ▼                          ▼                  ▼                │   │
│  │  ┌─────────┐           ┌─────────┐         ┌─────────┐             │   │
│  │  │ Ethernet│           │  VLAN   │         │   ARP   │             │   │
│  │  │  模块   │           │  模块   │         │  模块   │             │   │
│  │  └─────────┘           └─────────┘         └─────────┘             │   │
│  │                                                                        │   │
│  │      ┌─────────┐           ┌─────────┐         ┌─────────┐             │   │
│  │      │  IPv4   │           │  IPv6   │         │  ICMP   │             │   │
│  │      └─────────┘           └─────────┘         └─────────┘             │   │
│  │                                                                        │   │
│  │      ┌─────────┐           ┌─────────┐         ┌─────────┐             │   │
│  │      │  TCP    │           │   UDP   │         │  Route  │             │   │
│  │      └─────────┘           └─────────┘         │  模块   │             │   │
│  │                                              └─────────┘             │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                        │                                     │
│                                        ▼                                     │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │                         通用模块 (Common)                             │   │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐  │   │
│  │  │   Packet    │  │  RingQueue  │  │   Error     │  │ Mac/IPv4    │  │   │
│  │  │  报文描述符  │  │   环形队列   │  │   错误处理   │  │  地址类型    │  │   │
│  │  └─────────────┘  └─────────────┘  └─────────────┘  └─────────────┘  │   │
│  │  ┌─────────────┐                                                      │   │
│  │  │    Timer    │  定时器管理（驱动协议状态机）                          │   │
│  │  └─────────────┘                                                      │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

### 2.2 运行模式

**纯模拟模式**：
- 不使用 TUN/TAP 等真实网络接口
- 报文通过队列在模块间传递
- 支持报文注入和结果输出

### 2.3 数据流向

#### 上行（解析）流程

```
┌──────────────┐    ┌──────────────┐    ┌──────────────┐    ┌──────────────┐
│  测试注入    │ -> │  接收队列    │ -> │  调度器      │ -> │  处理引擎    │
│  (Injector) │    │    (RxQ)     │    │(Scheduler)   │    │  (Engine)    │
└──────────────┘    └──────────────┘    └──────────────┘    └──────┬───────┘
                                                                  │
                                                ┌─────────────────┴─────────┐
                                                │                           │
                                                ▼                           ▼
                                        ┌──────────────┐           ┌──────────────┐
                                        │ VLAN解析     │           │ ARP处理      │
                                        │ 去除标签     │           │ 缓存更新     │
                                        └──────────────┘           └──────────────┘
                                                │                           │
                                                └───────────┬───────────────┘
                                                            ▼
                                                ┌──────────────┐
                                                │  发送队列    │ <- 响应报文
                                                │    (TxQ)     │
                                                └──────────────┘
```

#### 下行（封装）流程

```
┌──────────────┐    ┌──────────────┐    ┌──────────────┐    ┌──────────────┐
│  应用数据    │ -> │  协议封装    │ -> │  添加标签    │ -> │  发送队列    │
│             │    │  (协议层)    │    │  (VLAN)      │    │    (TxQ)     │
└──────────────┘    └──────────────┘    └──────────────┘    └──────────────┘
```

---

## 3. 模块详解

### 3.1 上电启动模块 (PowerOn)

**职责**：系统资源初始化和释放

```
poweron/
├── mod.rs       # 模块入口
└── context.rs   # SystemContext 实现
```

**核心功能**：
- `boot_default()`: 系统启动，调用 interface 模块初始化
- `shutdown()`: 系统关闭，释放所有资源
- 持有 `SystemContext`，包含接口管理器的所有权

**设计原则**：
- 只负责启动和关闭，不管理配置文件路径
- 配置由 interface 模块自己管理

---

### 3.2 网络接口模块 (Interface)

**职责**：网络接口配置和状态管理

```
interface/
├── mod.rs           # 模块入口
├── types.rs         # MacAddr, Ipv4Addr, InterfaceState 等类型
├── iface.rs         # NetworkInterface 实现（包含队列）
├── manager.rs       # InterfaceManager 实现
├── config.rs        # 接口配置文件加载
└── interface.toml   # 接口配置文件（含队列配置）
```

**核心结构**：
- `NetworkInterface`: 单个网络接口，包含独立的 RxQ 和 TxQ
- `InterfaceManager`: 多接口管理器
- 通过 `SystemContext` 的 `Arc<Mutex<InterfaceManager>>` 传递给各模块

**配置文件格式**：
```toml
[queue]
rxq_capacity = 256
txq_capacity = 256

[[interfaces]]
name = "eth0"
mac_addr = "00:11:22:33:44:55"
ip_addr = "192.168.1.100"
netmask = "255.255.255.0"
mtu = 1500
state = "Up"
```

**与 Scheduler 的关系**：
```
InterfaceManager (多接口)
    │
    ├─► eth0: RxQ0, TxQ0 ──┐
    │                      │
    ├─► lo:   RxQ1, TxQ1  ├──> Scheduler::run_all_interfaces()
    │                      │   遍历所有接口，处理每个 RxQ
    └─► eth1: RxQ2, TxQ2 ──┘
```

---

### 3.3 调度模块 (Scheduler)

**职责**：从接收队列取报文并调度给协议处理引擎

```
scheduler/
├── mod.rs       # 模块入口
└── scheduler.rs # Scheduler 实现
```

**核心功能**：
- `run()`: 单队列调度模式
- `run_all_interfaces()`: 多接口调度模式
- 从 RxQ 取报文 → 调用 Processor → 响应放入 TxQ

**与其他模块的关系**：
```
Scheduler ──┬──> interface (获取所有接口的 RxQ/TxQ)
            └──> engine (调用 PacketProcessor::process())
```

---

### 3.4 协议处理引擎 (Engine)

**职责**：协议分发和调用协调（薄层设计）

```
engine/
├── mod.rs       # 模块入口
└── processor.rs # PacketProcessor 实现
```

**核心功能**：
- `process()`: 协议分发入口
- 根据 EtherType 分发到对应协议模块
- 错误转换和传播

**薄层设计**：
```
PacketProcessor
    │
    ├──> dispatch_by_ether_type()
    │         │
    │         ├──> VLAN  ──> vlan::process_vlan_packet()
    │         ├──> ARP   ──> arp::process_arp()
    │         └──> IPv4
    │
    └──> 错误转换: From<VlanError>, From<CoreError> ...
```

**与协议模块的关系**：
- Processor 调用协议模块的接口
- 具体处理逻辑由协议模块实现
- Processor 只负责分发和错误转换

---

### 3.5 协议模块 (Protocols)

#### 3.5.1 以太网模块 (Ethernet)

```
protocols/ethernet/
├── mod.rs       # 模块入口
└── header.rs    # EthernetHeader 实现
```

**核心功能**：
- 以太网帧解析：`EthernetHeader::from_packet()`
- 以太网帧封装：`EthernetHeader::build()`
- EtherType 常量定义（IPv4, ARP, IPv6, VLAN）

#### 3.5.2 VLAN 模块

```
protocols/vlan/
├── mod.rs       # 模块入口
├── tag.rs       # VlanTag 结构
├── frame.rs     # VlanFrame 结构
└── parse.rs     # VLAN 解析和封装
```

**核心功能**：
- 解析 802.1Q 标签（支持 0x8100, 0x88A8）
- 支持 QinQ 双层标签
- VLAN ID 验证（1-4094）

**处理流程**：
```
接收带VLAN的帧 -> 解析外层标签 -> 解析内层标签 -> 提取内层协议类型
```

#### 3.5.3 ARP 模块

```
protocols/arp/
├── mod.rs       # 模块入口，导出 ArpPacket 和处理函数
├── tables.rs    # ArpCache, ArpEntry, ArpState, ArpConfig
└── global.rs    # 全局 ARP 缓存初始化
```

**核心功能**：
- ARP 报文解析和封装
- ARP 缓存管理（状态机：NONE → INCOMPLETE → REACHABLE → STALE → DELAY → PROBE）
- 自动学习（收到任何 ARP 报文都更新缓存）
- 响应生成（检查目标 IP 是否为本机接口）
- Gratuitous ARP 支持
- 待发送报文队列（用于 INCOMPLETE 状态）

**状态转换**：
```
NONE -> INCOMPLETE -> REACHABLE -> STALE -> DELAY -> PROBE -> NONE
```

#### 3.5.4 IPv4 模块

```
protocols/ip/
├── mod.rs       # 模块入口
├── header.rs    # Ipv4Header 结构
├── checksum.rs  # IP 校验和计算
├── protocol.rs  # Ipv4Protocol 枚举
├── error.rs     # IpError 枚举
├── config.rs    # Ipv4Config
├── packet.rs    # IP 报文处理逻辑
└── fragment.rs  # 分片与重组
```

**核心功能**：
- IPv4 头部解析（20-60 字节）
- 校验和验证
- 协议字段分发（ICMP, TCP, UDP）
- 分片与重组（RFC 791, RFC 815）
- 重叠检测和处理策略

**已实现**：
- ✅ 头部解析
- ✅ 校验和计算和验证
- ✅ 协议分发
- ✅ 分片和重组（30秒超时，64最大条目，16最大分片数）

#### 3.5.5 ICMP 模块

```
protocols/icmp/
├── mod.rs       # 模块入口
├── types.rs     # ICMP 类型枚举和常量
├── packet.rs    # IcmpPacket, IcmpEcho, IcmpDestUnreachable, IcmpTimeExceeded
├── echo.rs      # Echo 请求/响应处理
├── process.rs   # ICMP 报文处理入口
└── global.rs    # ICMP Echo 管理器（追踪待处理请求）
```

**核心功能**：
- Echo Request/Reply（ping）
- Destination Unreachable（目标不可达）
- Time Exceeded（超时）
- Echo 请求/响应匹配
- 校验和验证

**已实现**：
- ✅ Echo Request/Reply
- ✅ Destination Unreachable (Type 3)
- ✅ Time Exceeded (Type 11)
- ✅ 校验和验证

---

### 3.5.6 ICMPv6 模块

```
protocols/icmpv6/
├── mod.rs       # 模块入口
├── packet.rs    # Icmpv6Packet 结构
├── types.rs     # ICMPv6 类型枚举
├── process.rs   # ICMPv6 报文处理
├── neighbor.rs  # 邻居发现协议 (NDP)
├── checksum.rs  # ICMPv6 校验和（伪头部）
└── config.rs    # ICMPv6 配置
```

**核心功能**：
- Echo Request/Reply（ping6）
- 邻居发现协议 (NDP)
- 邻居通告 (Neighbor Advertisement)
- 邻居请求 (Neighbor Solicitation)
- 路由器通告/请求
- 错误报告（目标不可达、超时、参数问题）
- ICMPv6 校验和（包含伪头部）

**已实现**：
- ✅ Echo Request/Reply
- ✅ 邻居发现 (NDP)
- ✅ 错误报告
- ✅ 校验和验证

---

### 3.5.7 IPv6 模块

```
protocols/ipv6/
├── mod.rs       # 模块入口
├── header.rs    # Ipv6Header 结构
├── protocol.rs  # IpProtocol 枚举
├── error.rs     # Ipv6Error 枚举
├── config.rs    # Ipv6Config
├── packet.rs    # IPv6 报文处理逻辑
├── extension.rs # 扩展头处理
├── fragment.rs  # 分片与重组
└── options.rs   # 选项处理
```

**核心功能**：
- IPv6 头部解析（40 字节固定头部）
- 协议字段分发（ICMPv6、TCP、UDP）
- 地址验证
- 128位地址支持
- 扩展头链解析
- 分片与重组

**扩展头支持**：
- ✅ Hop-by-Hop Options (Next Header = 0)
- ✅ Routing Header Type 2 (Next Header = 43)
- ✅ Fragment Header (Next Header = 44)
- ✅ Destination Options (Next Header = 60)
- ❌ ESP/AH（返回错误）

**已实现**：
- ✅ 头部解析
- ✅ 协议分发
- ✅ 分片和重组（60秒超时，256最大条目，64最大分片数）
- ✅ 扩展头（逐跳选项、路由、分片、目的选项）
- ✅ 原子分片拒绝（RFC 6981）
- ✅ 重叠检测（RFC 5722）

#### 3.5.8 TCP 模块

```
protocols/tcp/
├── mod.rs           # 模块入口
├── constant.rs      # TCP 常量定义
├── config.rs        # TcpConfig 配置
├── error.rs         # TcpError 错误类型
├── header.rs        # TcpHeader 头部结构
├── segment.rs       # TcpSegment 报文段
├── tcb.rs           # TCB (传输控制块)
├── connection.rs    # TcpConnection 连接状态管理
├── process.rs       # TCP 报文处理
├── socket.rs        # TcpSocket Socket实现
└── socket_manager.rs # TcpSocketManager Socket管理
```

**核心功能**：
- 三次握手（SYN、SYN-ACK、ACK）
- 四次挥手（FIN、ACK）
- 滑动窗口和流量控制
- 重传机制
- 拥塞控制（慢启动、拥塞避免）
- 连接状态管理（LISTEN、SYN_SENT、SYN_RECEIVED、ESTABLISHED、FIN_WAIT1/2、CLOSE_WAIT、LAST_ACK、TIME_WAIT）
- Socket API（bind、connect、send、recv、close）
- 端口复用和TIME_WAIT状态
- MSS选项支持

**已实现**：
- ✅ 三次握手（RFC 793, RFC 9293）
- ✅ 四次挥手
- ✅ 滑动窗口
- ✅ 重传机制
- ✅ Socket API
- ✅ 连接管理
- ✅ 拥塞控制

#### 3.5.9 UDP 模块

```
protocols/udp/
├── mod.rs       # 模块入口
├── header.rs    # UdpHeader 头部结构
├── packet.rs    # UdpDatagram 数据报
├── process.rs   # UDP 报文处理
├── config.rs    # UdpConfig 配置
├── port.rs      # 端口管理器和端口表
└── socket.rs    # UdpSocket Socket实现
```

**核心功能**：
- UDP 数据报解析和封装
- 端口绑定机制（知名端口、注册端口、临时端口）
- 端口表管理（端口到回调的映射）
- Socket API（bind、sendto、recvfrom、close）
- 端口不可达ICMP响应

**已实现**：
- ✅ 数据报解析/封装
- ✅ 端口绑定
- ✅ Socket API
- ✅ 回调机制
- ✅ 端口不可达响应

---

### 3.6 路由模块 (Route)

```
route/
├── mod.rs       # 模块入口
├── ipv4.rs      # Ipv4Route IPv4路由
├── ipv6.rs      # Ipv6Route IPv6路由
├── table.rs     # RouteTable 路由表
└── error.rs     # RouteError 错误类型
```

**核心功能**：
- IPv4/IPv6 路由表管理
- 最长前缀匹配（LPM）算法
- 默认路由支持
- 路由优先级管理

**已实现**：
- ✅ IPv4 路由表
- ✅ IPv6 路由表
- ✅ 最长前缀匹配
- ✅ 默认路由

---

### 3.7 Socket 模块

```
socket/
├── mod.rs       # 模块入口
├── types.rs     # Socket类型定义（AddressFamily, SocketType, SocketProtocol, SocketAddr等）
├── entry.rs     # SocketEntry（状态管理、缓冲区、监听队列、Socket选项）
├── manager.rs   # SocketManager（socket, bind, listen, accept, connect, send, sendto, recv, recvfrom, close）
└── error.rs     # SocketError 错误类型
```

**核心功能**：
- POSIX风格Socket API
- Socket生命周期管理（fd分配、创建、销毁）
- TCP/UDP Socket支持（IPv4/IPv6）
- 绑定、监听、接受、连接操作
- 发送/接收缓冲区管理
- Socket文件描述符管理
- Socket选项支持（SO_REUSEADDR等）
- 地址冲突检测和端口复用
- TCP连接映射和事件通知
- 数据分发（TCP/UDP）

**已实现**：
- ✅ socket() - 创建Socket（支持TCP/UDP、IPv4/IPv6）
- ✅ bind() - 绑定地址（含地址冲突检测）
- ✅ listen() - 监听连接（支持backlog配置）
- ✅ accept() - 接受连接（从监听队列取连接）
- ✅ connect() - 发起连接（TCP三次握手）
- ✅ send() - 发送数据（面向连接）
- ✅ sendto() - 发送数据（无连接）
- ✅ recv() - 接收数据（面向连接）
- ✅ recvfrom() - 接收数据（无连接）
- ✅ close() - 关闭Socket（含资源清理）

**类型定义**：
- `SocketFd` - Socket文件描述符（保留0-2，从3开始分配）
- `AddressFamily` - 协议族（AF_INET/AF_INET6）
- `SocketType` - Socket类型（SOCK_STREAM/SOCK_DGRAM）
- `SocketProtocol` - 协议（Default/ICMP/TCP/UDP）
- `SocketAddr` - Socket地址（IPv4/IPv6）
- `TcpState` - TCP连接状态（11种状态）
- `SendFlags/RecvFlags` - 发送/接收标志

**Socket选项**：
- `SO_REUSEADDR` - 地址复用
- `SO_REUSEPORT` - 端口复用
- `SO_BROADCAST` - 广播
- `SO_KEEPALIVE` - 保活
- `SO_RCVBUF/SO_SNDBUF` - 缓冲区大小

**与其他模块的集成**：
- 与TCP模块通过 `TcpSocketManager` 交互
- 与UDP模块通过 `UdpPortManager` 交互
- 提供数据分发接口 `deliver_tcp_data()` 和 `deliver_udp_data()`
- 提供连接事件通知接口 `notify_tcp_event()`
- 支持TCP连接映射 `map_tcp_connection()`

---

### 3.8 通用模块 (Common)

```
common/
├── mod.rs       # 模块入口
├── packet.rs    # Packet 报文描述符
├── queue.rs     # RingQueue 环形队列
├── error.rs     # CoreError 错误类型
├── addr.rs      # MacAddr, Ipv4Addr 等地址类型
├── tables.rs    # 通用表结构
└── timer.rs     # Timer 定时器
```

#### 3.8.1 Packet（报文描述符）

**核心结构**：
```rust
pub struct Packet {
    pub data: Vec<u8>,  // 数据缓冲区
    pub offset: usize,  // 当前读取偏移量
}
```

**所有权转移**：
- Packet 在队列间**移动**，不克隆
- `enqueue()` 转移所有权
- `dequeue()` 获取所有权

#### 3.8.2 RingQueue（环形队列）

**核心结构**：
```rust
pub struct RingQueue<T> {
    buffer: Vec<Option<T>>,
    capacity: usize,
    head: usize,  // 读指针
    tail: usize,  // 写指针
    count: usize, // 元素数量
}
```

**队列模型**：SPSC（单生产者单消费者）

#### 3.8.3 Error（错误处理）

**三种错误类型**：
- `CoreError`: 通用错误（在 common 模块）
- `ProcessError`: 处理错误（在 engine 模块）
- `ScheduleError`: 调度错误（在 scheduler 模块）

**错误转换**：各模块实现 `From<T>` trait 进行转换

#### 3.8.4 Timer（定时器）

**核心功能**：
- 驱动协议状态机（如TCP重传、ARP超时）
- 支持定时回调注册
- 支持定时器取消

---

## 4. 模块关系图

### 4.1 依赖关系

```
┌─────────────────────────────────────────────────────────────────────┐
│                          依赖层次                                   │
├─────────────────────────────────────────────────────────────────────┤
│                                                                     │
│  ┌─────────────────────────────────────────────────────────────┐   │
│  │                        应用层                               │   │
│  │                   Socket API (部分实现)                      │   │
│  └─────────────────────────────────────────────────────────────┘   │
│                              ▲                                     │
│  ┌─────────────────────────────────────────────────────────────┐   │
│  │                        传输层                               │   │
│  │            TCP / UDP / Socket Manager                       │   │
│  └─────────────────────────────────────────────────────────────┘   │
│                              ▲                                     │
│  ┌─────────────────────────────────────────────────────────────┐   │
│  │                   路由层 (可选)                              │   │
│  │          RouteTable (IPv4/IPv6 LPM)                         │   │
│  └─────────────────────────────────────────────────────────────┘   │
│                              ▲                                     │
│  ┌─────────────────────────────────────────────────────────────┐   │
│  │                        网络层                               │   │
│  │          IPv4 / IPv6 / ICMP / ICMPv6                        │   │
│  └─────────────────────────────────────────────────────────────┘   │
│                              ▲                                     │
│  ┌─────────────────────────────────────────────────────────────┐   │
│  │                        链路层                               │   │
│  │              Ethernet / VLAN / ARP                          │   │
│  └─────────────────────────────────────────────────────────────┘   │
│                              ▲                                     │
│  ┌─────────────────────────────────────────────────────────────┐   │
│  │                  接口层 (Interface)                          │   │
│  │             NetworkInterface / RxQ / TxQ                    │   │
│  └─────────────────────────────────────────────────────────────┘   │
│                              ▲                                     │
│  ┌─────────────────────────────────────────────────────────────┐   │
│  │                   通用层 (Common)                            │   │
│  │       Packet / Queue / Error / Timer / Tables               │   │
│  └─────────────────────────────────────────────────────────────┘   │
│                                                                     │
└─────────────────────────────────────────────────────────────────────┘
```

### 4.2 调用关系

```
┌─────────────────────────────────────────────────────────────────────┐
│                          调用流程                                   │
├─────────────────────────────────────────────────────────────────────┤
│                                                                     │
│  main()                                                             │
│    │                                                                │
│    ├──> poweron::boot_default()                                     │
│    │         │                                                      │
│    │         └──> interface::init_default()                         │
│    │                   │                                            │
│    │                   └──> 加载 interface.toml                     │
│    │                   └──> 创建 InterfaceManager                   │
│    │                                                                │
│    ├──> scheduler::Scheduler::new()                                 │
│    │         │                                                      │
│    │         └──> engine::PacketProcessor::new()                    │
│    │                                                                │
│    └──> scheduler.run_all_interfaces()                              │
│              │                                                      │
│              └──> for each interface:                               │
│                      │                                              │
│                      ├──> rxq.dequeue() ──> Packet                  │
│                      │                                              │
│                      └──> processor.process(packet)                 │
│                                │                                    │
│                                ├──> EthernetHeader::from_packet()   │
│                                │                                    │
│                                └──> dispatch_by_ether_type()        │
│                                      │                              │
│                                      ├──> VLAN:                     │
│                                      │      └──> vlan::process_...  │
│                                      │                              │
│                                      ├──> ARP:                      │
│                                      │      └──> arp::process_arp() │
│                                      │                              │
│                                      ├──> IPv4:                    │
│                                      │      ├──> IP协议分发         │
│                                      │      ├──> ICMP: handle_icmp()│
│                                      │      ├──> TCP:  handle_tcp() │
│                                      │      └──> UDP:  handle_udp() │
│                                      │                              │
│                                      └──> IPv6:                    │
│                                             ├──> IP协议分发         │
│                                             └──> ICMPv6:handle_icmpv6()│
│                                      └──> 返回 ProcessResult        │
│                                                                │    │
│                                                                ▼    │
│                                            Ok(Some(response)) ──> txq
│                                                                     │
└─────────────────────────────────────────────────────────────────────┘
```

### 4.3 数据流转

```
┌─────────────────────────────────────────────────────────────────────┐
│                        Packet 所有权流转                              │
├─────────────────────────────────────────────────────────────────────┤
│                                                                     │
│  测试注入                                                          │
│    │                                                                │
│    ├──> Packet::from_bytes() ──> 创建 Packet                       │
│    │                                                                │
│    └──> rxq.enqueue(packet) ──> Packet 移动到 RxQ                  │
│                    │                                                │
│                    ▼                                                │
│  调度处理                                                          │
│    │                                                                │
│    ├──> rxq.dequeue() ──> Packet 移动到 Scheduler                   │
│    │                                                                │
│    └──> processor.process(packet) ──> Packet 移动到 Engine          │
│                    │                                                │
│                    ▼                                                │
│  协议解析                                                          │
│    │                                                                │
│    ├──> vlan::process_vlan_packet(&mut packet) ──> 借用引用         │
│    │                                                                │
│    └──> arp::process_arp(&mut packet) ──> 借用引用                  │
│                    │                                                │
│                    ▼                                                │
│  响应生成（如 ARP Reply）                                           │
│    │                                                                │
│    └──> txq.enqueue(response_packet) ──> 响应 Packet 移动到 TxQ     │
│                                                                     │
└─────────────────────────────────────────────────────────────────────┘
```

---

## 5. 协议分层

| 层级 | 协议 | RFC | 状态 |
|------|------|-----|------|
| 应用层 | Socket API | - | ✅ 已实现（POSIX风格） |
| 传输层 | TCP | RFC 793, RFC 9293 | ✅ 已实现（完整状态机、拥塞控制） |
| 传输层 | UDP | RFC 768 | ✅ 已实现 |
| 网络层 | IPv4 | RFC 791 | ✅ 已实现（含分片/重组） |
| 网络层 | IPv6 | RFC 8200 | ✅ 已实现（含分片/重组/扩展头） |
| 网络层 | ICMP | RFC 792 | ✅ 已实现 |
| 网络层 | ICMPv6 | RFC 4443 | ✅ 已实现（Echo、NDP） |
| 路由 | 路由表 | - | ✅ 已实现（最长前缀匹配） |
| 链路层 | Ethernet | IEEE 802.3 | ✅ 已实现 |
| 链路层 | VLAN | IEEE 802.1Q | ✅ 已实现 |
| 链路层 | ARP | RFC 826 | ✅ 已实现 |

**实现详情**：
- **TCP**: 三次握手、四次挥手、滑动窗口、重传机制、拥塞控制、Socket API、连接管理、MSS选项
- **UDP**: 端口绑定、数据报收发、Socket API、回调机制、端口不可达响应
- **IPv6**: 基础头部解析、协议分发、ICMPv6 Echo支持、分片与重组、扩展头支持
- **路由**: IPv4/IPv6路由表、最长前缀匹配（LPM）
- **Socket API**: POSIX风格API（socket, bind, listen, accept, connect, send, recv, close）
- **IP分片**: IPv4/IPv6分片与重组（超时处理、重叠检测）

**未实现功能**：
- IPSec（ESP/AH扩展头返回错误）
- 动态路由协议（OSPF、BGP等）

---

## 6. 技术选型

| 项目 | 选择 | 说明 |
|------|------|------|
| 运行模式 | 纯模拟 | 不使用真实网络接口，仅队列传递 |
| 队列实现 | 自定义环形缓冲区 | 学习数据结构原理 |
| 队列类型 | SPSC | 单生产者单消费者 |
| 错误处理 | 手动实现 Error enum | 学习 Rust 错误处理 |
| 外部依赖 | 零依赖 | 纯标准库实现 |
| 并发模型 | 单线程处理 | 处理线程单线程逐层解析 |
| 全局状态 | 依赖注入 (Arc<Mutex<T>>) | 便于测试和并发控制 |
| 配置管理 | 模块自治 | 各模块管理自己的配置 |

---

## 7. 参考资料

- RFC 791: Internet Protocol (IPv4)
- RFC 793: Transmission Control Protocol (TCP)
- RFC 768: User Datagram Protocol (UDP)
- RFC 792: Internet Control Message Protocol (ICMP)
- RFC 826: An Ethernet Address Resolution Protocol (ARP)
- RFC 2460: Internet Protocol, Version 6 (IPv6)
- IEEE 802.1Q: Virtual LANs
- IEEE 802.3: Ethernet

---

## 8. 相关文档

### 设计文档

- [SystemContext 设计](context.md) - 系统上下文和依赖注入架构
- [报文描述符](packet.md) - Packet 结构设计
- [环形队列设计](queue.md) - 环形队列实现细节
- [错误处理](error.md) - 错误类型定义和处理
- [上电启动模块](poweron.md) - 系统资源初始化和释放
- [网络接口模块](interface.md) - 接口管理和配置
- [报文处理模块](engine.md) - 协议分发和处理协调
- [调度模块](scheduler.md) - 报文调度和队列管理

### 协议设计

- [VLAN 协议设计](protocols/vlan.md) - 802.1Q VLAN 标签处理
- [ARP 协议设计](protocols/arp.md) - ARP 协议和缓存管理
- [IPv4 协议设计](protocols/ip.md) - IPv4 协议实现
- [IPv6 协议设计](protocols/ipv6.md) - IPv6 协议实现
- [ICMP 协议设计](protocols/icmp.md) - ICMP 协议实现
- [ICMPv6 协议设计](protocols/icmpv6.md) - ICMPv6 协议实现
- [TCP 协议设计](protocols/tcp.md) - TCP 协议实现
- [UDP 协议设计](protocols/udp.md) - UDP 协议实现
- [路由模块设计](route.md) - 路由表和最长前缀匹配
- [Socket API 设计](socket.md) - Socket API 实现
