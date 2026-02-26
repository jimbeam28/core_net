# CoreNet

> 一个使用 Rust 实现的纯模拟网络协议栈，用于学习/研究目的。
> 除了本句话，其余内容均由AI实现。

## 项目简介

CoreNet 是一个**纯模拟**的网络协议栈实现，支持完整的 TCP/IP 协议族。项目采用分层架构设计，代码结构清晰，便于理解网络协议原理。

### 特点

- **纯模拟环境**：不使用真实网络接口（TUN/TAP），仅通过队列传递报文
- **分层架构**：链路层 → 网络层 → 传输层 → 应用层
- **零外部依赖**：仅使用 Rust 标准库
- **学习导向**：代码设计注重可读性和可理解性
- **模块化**：便于添加新协议支持
- **多接口支持**：支持多个网络接口独立配置和管理
- **测试框架**：内置协议测试框架，便于验证协议实现

## 项目架构

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                              CoreNet 网络协议栈                               │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │                      网络接口管理 (Interface)                         │   │
│  │  ┌────────────┐  ┌────────────┐  ┌────────────┐                     │   │
│  │  │   eth0     │  │    lo      │  │   eth1     │  ...                │   │
│  │  │  RxQ/TxQ   │  │  RxQ/TxQ   │  │  RxQ/TxQ   │                     │   │
│  │  └────────────┘  └────────────┘  └────────────┘                     │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                        │                                     │
│                                        ▼                                     │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │                       调度模块 (Scheduler)                          │   │
│  │              遍历所有接口，从 RxQ 取报文调度处理                     │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                        │                                     │
│                                        ▼                                     │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │                       协议处理引擎 (Engine)                          │   │
│  │                    薄层设计：协议分发和协调                          │   │
│  │      ┌─────────┐    ┌─────────┐    ┌─────────┐                      │   │
│  │      │ Ethernet│    │  VLAN   │    │   ARP   │                      │   │
│  │      └─────────┘    └─────────┘    └─────────┘                      │   │
│  │      ┌─────────┐    ┌─────────┐    ┌─────────┐                      │   │
│  │      │  IPv4   │    │  IPv6   │    │  ICMP   │                      │   │
│  │      └─────────┘    └─────────┘    └─────────┘                      │   │
│  │      ┌─────────┐    ┌─────────┐    ┌─────────┐                      │   │
│  │      │  TCP    │    │   UDP   │    │  Route  │                      │   │
│  │      └─────────┘    └─────────┘    └─────────┘                      │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

### 数据流向

**上行（解析）流程**：
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
                                                │ IPv4/IPv6    │
                                                │ 协议分发     │
                                                └──────────────┘
                                                            │
                            ┌───────────────────────────┼───────────────────────────┐
                            │                           │                           │
                            ▼                           ▼                           ▼
                    ┌──────────────┐           ┌──────────────┐           ┌──────────────┐
                    │   ICMP/      │           │   TCP/       │           │   UDP/       │
                    │   ICMPv6     │           │   Socket     │           │   Socket     │
                    └──────────────┘           └──────────────┘           └──────────────┘
                            │                           │                           │
                            └───────────────────────────┼───────────────────────────┘
                                                        ▼
                                                ┌──────────────┐
                                                │  发送队列    │ <- 响应报文
                                                │    (TxQ)     │
                                                └──────────────┘
```

## 协议支持

### 链路层
- Ethernet - 帧解析/封装
- VLAN (802.1Q/802.1AD) - 单/双标签支持
- ARP - 缓存管理、响应生成、状态机

### 网络层
- IPv4 - 头部解析、校验和、协议分发、分片与重组
- IPv6 - 头部解析、协议分发、分片与重组、扩展头支持
- ICMP - Echo Request/Reply、Destination Unreachable、Time Exceeded
- ICMPv6 - Echo Request/Reply、邻居发现(NDP)
- IPsec (AH/ESP/IKEv2) - SA/SPD管理、重放窗口保护、传输/隧道模式、入站处理、密钥交换
- 路由 - IPv4/IPv6路由表、最长前缀匹配

### 传输层
- UDP - 数据报收发、端口绑定、Socket API、回调机制
- TCP - 三次握手、四次挥手、滑动窗口、重传、Socket API、拥塞控制、定时器管理（重传、TimeWait、Keepalive、Delayed ACK）

### 应用层
- Socket API - POSIX风格API（socket、bind、listen、accept、connect、send、sendto、recv、recvfrom、close）

### 动态路由协议
- OSPFv2 (RFC 2328) - Hello/DD/LSR/LSU/LSAck报文、LSA类型、接口/邻居状态机、LSDB、SPF算法、DR/BDR选举
- OSPFv3 (RFC 5340) - OSPF for IPv6，支持链路本地地址、OSPFv3 LSA类型
- BGP-4 (RFC 4271) - Open/Update/Notification/Keepalive报文、状态机、对等体管理、RIB、路径属性
- IKEv2 (RFC 7296) - IKE_SA_INIT、IKE_AUTH、CREATE_CHILD_SA、INFORMATIONAL 交换

## 目录结构

```
core_net/
├── docs/                      # 设计文档
│   ├── design/                # 设计文档
│   │   ├── architecture.md    # 整体架构设计
│   │   ├── engine.md          # 报文处理模块
│   │   ├── interface.md       # 网络接口模块
│   │   ├── scheduler.md       # 调度模块
│   │   ├── poweron.md         # 上电启动模块
│   │   ├── packet.md          # 报文描述符设计
│   │   ├── queue.md           # 环形队列设计
│   │   ├── error.md           # 错误处理设计
│   │   ├── test_framework.md  # 测试框架设计
│   │   ├── context.md         # 系统上下文设计
│   │   └── protocols/         # 协议设计文档
│   │       ├── vlan.md        # VLAN 协议设计
│   │       ├── arp.md         # ARP 协议设计
│   │       ├── ip.md          # IPv4 协议设计
│   │       ├── ipv6.md        # IPv6 协议设计
│   │       ├── icmp.md        # ICMP 协议设计
│   │       ├── icmpv6.md      # ICMPv6 协议设计
│   │       ├── tcp.md         # TCP 协议设计
│   │       ├── udp.md         # UDP 协议设计
│   │       ├── ospf.md        # OSPFv2 协议设计
│   │       ├── ospfv3.md      # OSPFv3 协议设计
│   │       ├── bgp.md         # BGP 协议设计
│   │       ├── ipsec.md       # IPsec 协议设计
│   │       ├── socket.md      # Socket API 设计
│   │       └── route.md       # 路由模块设计
├── src/                       # 源代码
│   ├── main.rs                # 主入口
│   ├── lib.rs                 # 库入口
│   ├── context.rs             # SystemContext（依赖注入）
│   ├── common/                # 核心基础模块
│   │   ├── addr.rs            # 地址类型 (MacAddr, Ipv4Addr, Ipv6Addr)
│   │   ├── error.rs           # 错误处理 (CoreError)
│   │   ├── packet.rs          # 报文描述符 (Packet)
│   │   ├── queue.rs           # 环形队列 (RingQueue)
│   │   ├── tables.rs          # 通用表结构
│   │   ├── timer.rs           # 定时器 (Timer)
│   │   └── mod.rs
│   ├── poweron/               # 系统生命周期
│   │   ├── context.rs         # SystemContext（旧版，已废弃）
│   │   └── mod.rs
│   ├── interface/             # 网络接口管理
│   │   ├── iface.rs           # NetworkInterface
│   │   ├── manager.rs         # InterfaceManager
│   │   ├── config.rs          # 配置文件加载
│   │   ├── types.rs           # 接口类型
│   │   ├── global.rs          # 全局接口管理器（已废弃）
│   │   └── mod.rs
│   ├── scheduler/             # 报文调度器
│   │   ├── scheduler.rs       # Scheduler
│   │   └── mod.rs
│   ├── engine/                # 协议处理引擎
│   │   ├── processor.rs       # PacketProcessor
│   │   └── mod.rs
│   ├── route/                 # 路由模块
│   │   ├── ipv4.rs            # IPv4路由
│   │   ├── ipv6.rs            # IPv6路由
│   │   ├── table.rs           # 路由表
│   │   ├── error.rs           # 路由错误
│   │   └── mod.rs
│   ├── protocols/             # 协议实现
│   │   ├── ethernet/          # 以太网协议
│   │   ├── vlan/              # VLAN 协议
│   │   ├── arp/               # ARP 协议
│   │   ├── ip/                # IPv4 协议
│   │   ├── ipv6/              # IPv6 协议
│   │   ├── icmp/              # ICMP 协议
│   │   ├── icmpv6/            # ICMPv6 协议
│   │   ├── udp/               # UDP 协议
│   │   ├── tcp/               # TCP 协议
│   │   ├── ospf/              # OSPF 共享核心模块
│   │   ├── ospf2/             # OSPFv2 协议
│   │   ├── ospf3/             # OSPFv3 协议
│   │   ├── bgp/               # BGP-4 协议
│   │   └── ipsec/             # IPsec 协议（AH/ESP、SA/SPD）
│   │       └── ikev2/         # IKEv2 协议（密钥交换）
│   ├── socket/                # Socket API（POSIX 风格实现）
│   │   ├── types.rs           # Socket 类型定义
│   │   ├── entry.rs           # Socket 表项
│   │   ├── manager.rs         # Socket 管理器
│   │   ├── error.rs           # Socket 错误
│   │   └── mod.rs             # 模块入口
│   └── testframework/         # 测试框架
│       ├── harness.rs         # TestHarness
│       ├── injector.rs        # PacketInjector
│       └── global_state.rs    # GlobalStateManager
├── Cargo.toml                 # 项目配置
├── CLAUDE.md                  # Claude Code 指导文件
└── README.md                  # 本文件
```

## 核心模块

### common
核心数据结构和工具类型：
- `Packet` - 报文描述符，拥有数据缓冲区
- `RingQueue<T>` - SPSC 环形队列
- `CoreError` - 通用错误类型
- `MacAddr`, `Ipv4Addr` - 地址类型

### poweron
系统生命周期管理：
- `boot_default()` - 系统启动，初始化接口
- `shutdown()` - 系统关闭，释放资源

### interface
网络接口配置和管理：
- `NetworkInterface` - 单个接口（含 RxQ/TxQ）
- `InterfaceManager` - 多接口管理器
- 配置文件：`interface.toml`

### scheduler
报文调度和多接口处理：
- `run_all_interfaces()` - 遍历所有接口处理报文
- 从 RxQ 取报文 → 调用 Processor → 响应放入 TxQ

### engine
协议分发和薄层协调：
- `process()` - 协议分发入口
- 根据 EtherType 分发到对应协议模块
- 支持 IPv4/IPv6、ICMP/ICMPv6、TCP/UDP 分发

### route
路由管理模块：
- `RouteTable` - IPv4/IPv6 路由表
- 最长前缀匹配（LPM）算法
- 默认路由支持

### protocols
协议实现（参考对应 RFC 标准）：
- `ethernet` - 以太网帧解析/封装
- `vlan` - 802.1Q/802.1AD 标签处理
- `arp` - ARP 协议和缓存管理
- `ip` - IPv4 协议、分片与重组（RFC 791, RFC 815）
- `ipv6` - IPv6 协议、分片与重组、扩展头（RFC 8200）
- `icmp` - ICMP 协议（Echo、Dest Unreachable、Time Exceeded）
- `icmpv6` - ICMPv6 协议（Echo、NDP、错误报告）
- `udp` - UDP 协议、UdpSocket、UdpPortManager（RFC 768）
- `tcp` - TCP 协议、TcpSocket、连接管理、拥塞控制（RFC 793, RFC 9293）
- `ospf` - OSPF 共享核心模块（SPF算法、DR/BDR选举、LSA洪泛）
- `ospf2` - OSPFv2 协议（RFC 2328）
- `ospf3` - OSPFv3 协议（RFC 5340）
- `bgp` - BGP-4 协议（RFC 4271）
- `ipsec` - IPsec 协议（RFC 4301, 4302, 4303）- AH/ESP、SA/SPD管理、重放窗口保护
- `ipsec/ikev2` - IKEv2 协议（RFC 7296）- 密钥交换、状态机、Payload类型

### socket
POSIX风格 Socket API 实现（应用层网络接口）：
- `types.rs` - Socket 类型定义（SocketFd、AddressFamily、SocketType、SocketAddr、TcpState等）
- `entry.rs` - Socket 表项（状态管理、缓冲区、监听队列、Socket选项）
- `manager.rs` - Socket 管理器（完整 POSIX API：socket/bind/listen/accept/connect/send/sendto/recv/recvfrom/close）
- `error.rs` - Socket 错误类型
- 与 TCP/UDP 模块集成，支持数据分发和连接事件通知

**注意**：模块级便利函数（`socket()`, `bind()` 等）保留用于未来全局状态支持。推荐直接使用 `SocketManager` 的方法。

### testframework
协议测试框架：
- `TestHarness` - 测试工具
- `PacketInjector` - 报文注入器

## 快速开始

### 环境要求

- Rust 2024 Edition 或更高版本
- Cargo 包管理器

### 编译运行

```bash
# 克隆项目
git clone <repository_url>
cd core_net

# 构建
cargo build

# 运行
cargo run

# 运行测试
cargo test

# 检查代码
cargo check

# 格式化
cargo fmt

# 代码检查
cargo clippy
```

## 设计文档

详细设计文档请查看 [docs/design/](docs/design/) 目录：

### 系统设计
- [整体架构设计](docs/design/architecture.md) - 完整的系统架构和模块关系
- [报文处理引擎](docs/design/engine.md) - 协议分发和处理协调
- [网络接口管理](docs/design/interface.md) - 多接口配置和管理
- [调度模块](docs/design/scheduler.md) - 报文调度和队列管理
- [系统生命周期](docs/design/poweron.md) - 系统启动和关闭
- [系统上下文](docs/design/context.md) - 依赖注入模式实现
- [路由模块](docs/design/route.md) - 路由表和最长前缀匹配

### 基础组件
- [报文描述符](docs/design/packet.md) - Packet 结构设计
- [环形队列](docs/design/queue.md) - 环形队列实现细节
- [错误处理](docs/design/error.md) - 错误类型定义和处理
- [测试框架](docs/design/test_framework.md) - 协议测试框架设计

### 协议设计
- [VLAN 协议设计](docs/design/protocols/vlan.md) - 802.1Q VLAN 标签处理
- [ARP 协议设计](docs/design/protocols/arp.md) - ARP 协议和缓存管理
- [IPv4 协议设计](docs/design/protocols/ip.md) - IPv4 协议实现
- [IPv6 协议设计](docs/design/protocols/ipv6.md) - IPv6 协议实现
- [ICMP 协议设计](docs/design/protocols/icmp.md) - ICMP 协议实现
- [ICMPv6 协议设计](docs/design/protocols/icmpv6.md) - ICMPv6 协议实现
- [TCP 协议设计](docs/design/protocols/tcp.md) - TCP 协议实现
- [UDP 协议设计](docs/design/protocols/udp.md) - UDP 协议实现
- [OSPFv2 协议设计](docs/design/protocols/ospf.md) - OSPFv2 协议实现
- [OSPFv3 协议设计](docs/design/protocols/ospfv3.md) - OSPFv3 协议实现
- [BGP 协议设计](docs/design/protocols/bgp.md) - BGP-4 协议实现
- [IPsec 协议设计](docs/design/protocols/ipsec.md) - IPsec 协议实现（AH/ESP、SA/SPD）
- [Socket API 设计](docs/design/socket.md) - Socket API 实现

## 参考资料

本项目遵循以下 RFC 标准：

| 协议 | RFC | 描述 |
|------|-----|------|
| Ethernet | IEEE 802.3 | 以太网标准 |
| VLAN | IEEE 802.1Q | 虚拟局域网 |
| ARP | RFC 826 | 地址解析协议 |
| IPv4 | RFC 791 | 互联网协议 |
| IPv6 | RFC 8200 | 互联网协议第 6 版 |
| ICMP | RFC 792 | 互联网控制报文协议 |
| ICMPv6 | RFC 4443 | ICMPv6 |
| TCP | RFC 793, RFC 9293 | 传输控制协议 |
| UDP | RFC 768 | 用户数据报协议 |
| IPsec (AH) | RFC 4302 | IP 认证头 |
| IPsec (ESP) | RFC 4303 | IP 封装安全载荷 |
| IPsec (架构) | RFC 4301 | IPsec 安全架构 |
| IKEv2 | RFC 7296 | Internet Key Exchange v2 |
| OSPFv2 | RFC 2328 | 开放最短路径优先 v2 |
| OSPFv3 | RFC 5340 | OSPF for IPv6 |
| BGP-4 | RFC 4271 | 边界网关协议 4 |
| Socket API | POSIX | Socket API |

## 许可证

MIT License

## 致谢

本项目使用 AI 辅助设计和实现。
