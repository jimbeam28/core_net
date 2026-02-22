# CoreNet

> 一个使用 Rust 实现的纯模拟网络协议栈，用于学习/研究目的。

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

### 已实现 ✅
- **链路层**
  - ✅ Ethernet - 帧解析/封装
  - ✅ VLAN (802.1Q/802.1AD) - 单/双标签支持
  - ✅ ARP - 缓存管理、响应生成、状态机

- **网络层**
  - ✅ IPv4 - 头部解析、校验和、协议分发（暂不支持分片）
  - ✅ IPv6 - 头部解析、协议分发（暂不支持分片/扩展头）
  - ✅ ICMP - Echo Request/Reply、Destination Unreachable、Time Exceeded
  - ✅ ICMPv6 - Echo Request/Reply
  - ✅ 路由 - IPv4/IPv6路由表、最长前缀匹配

- **传输层**
  - ✅ UDP - 数据报收发、端口绑定、Socket API、回调机制
  - ✅ TCP - 三次握手、四次挥手、滑动窗口、重传、Socket API

### 计划中 ⏳
- **网络层**
  - ⏳ IP 分片与重组
  - ⏳ IPv6 扩展头（逐跳选项、分片、路由、目的选项）
  - ⏳ ND (Neighbor Discovery) - IPv6 邻居发现
- **应用层**
  - ⏳ 完整 Socket API（bind/connect/send/recv等）

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
│   │   └── test_framework.md  # 测试框架设计
│   │   └── protocols/         # 协议设计文档
│   │       ├── vlan.md        # VLAN 协议设计
│   │       └── arp.md         # ARP 协议设计
│   └── log/                   # 开发日志
│       └── dev_log.md         # 开发日志
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
│   │   ├── ethernet/          # 以太网协议 ✅
│   │   ├── vlan/              # VLAN 协议 ✅
│   │   ├── arp/               # ARP 协议 ✅
│   │   ├── ip/                # IPv4 协议 ✅
│   │   ├── ipv6/              # IPv6 协议 ✅
│   │   ├── icmp/              # ICMP/ICMPv6 协议 ✅
│   │   ├── udp/               # UDP 协议 ✅
│   │   └── tcp/               # TCP 协议 ✅
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
- `ip` - IPv4 协议
- `ipv6` - IPv6 协议
- `icmp` - ICMP/ICMPv6 协议
- `udp` - UDP 协议和 Socket
- `tcp` - TCP 协议、连接管理和 Socket

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

## 实现计划

### 阶段一：基础框架 ✅
- [x] 目录结构创建
- [x] common 模块实现（Packet、RingQueue、Error、Addr、Tables、Timer）
- [x] engine 模块实现（薄层设计）
- [x] testframework 模块实现
- [x] poweron 模块实现
- [x] interface 模块实现（多接口支持）
- [x] scheduler 模块实现

### 阶段二：基础协议 ✅
- [x] 以太网层
- [x] VLAN (802.1Q/802.1AD)
- [x] ARP（含缓存、状态机）
- [x] IPv4 基础（头部解析、校验和、协议分发）
- [x] ICMP 协议（Echo、Dest Unreachable、Time Exceeded）

**目标**：能够 ping 通网关 ✅ 已实现

### 阶段三：传输层 ✅
- [x] UDP 协议
  - [x] 数据报解析/封装
  - [x] 端口绑定机制
  - [x] Socket API
  - [x] 回调机制
- [x] TCP 协议
  - [x] 三次握手
  - [x] 四次挥手
  - [x] 滑动窗口
  - [x] 重传机制
  - [x] Socket API

**目标**：能够建立 TCP 连接并传输数据 ✅ 已实现

### 阶段四：IPv6 ✅
- [x] IPv6 基础（头部解析、协议分发）
- [x] ICMPv6（Echo Request/Reply）
- [x] 路由模块（IPv4/IPv6路由表、最长前缀匹配）

**目标**：支持 IPv6 基础通信 ✅ 已实现

### 阶段五：高级功能（进行中）
- [ ] IP 分片与重组
- [ ] IPv6 扩展头
- [ ] ND (Neighbor Discovery)
- [ ] 完整 Socket API

---

## 完成度

**各模块完成度**：
- Common 模块: 100%
- Interface 模块: 100%
- Scheduler 模块: 100%
- Engine 模块: 100%
- Route 模块: 80%（支持基础路由，不支持动态路由）
- Ethernet 协议: 100%
- VLAN 协议: 100%
- ARP 协议: 100%
- IPv4 协议: 85%（支持头部解析和校验和，不支持分片/重组）
- IPv6 协议: 75%（支持基础头部，不支持分片/扩展头）
- ICMP 协议: 100%
- ICMPv6 协议: 60%（仅支持 Echo）
- UDP 协议: 90%（支持基础功能）
- TCP 协议: 85%（支持基础连接和数据传输）

**整体项目完成度: ~90%**

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
- [ICMP 协议设计](docs/design/protocols/icmp.md) - ICMP/ICMPv6 协议实现
- [TCP 协议设计](docs/design/protocols/tcp.md) - TCP 协议实现
- [UDP 协议设计](docs/design/protocols/udp.md) - UDP 协议实现

## 参考资料

本项目遵循以下 RFC 标准：

| 协议 | RFC | 描述 | 状态 |
|------|-----|------|------|
| Ethernet | IEEE 802.3 | 以太网标准 | ✅ 已实现 |
| VLAN | IEEE 802.1Q | 虚拟局域网 | ✅ 已实现 |
| ARP | RFC 826 | 地址解析协议 | ✅ 已实现 |
| IPv4 | RFC 791 | 互联网协议 | ✅ 已实现（无分片） |
| IPv6 | RFC 8200 | 互联网协议第 6 版 | ✅ 已实现（无分片/扩展头） |
| ICMP | RFC 792 | 互联网控制报文协议 | ✅ 已实现 |
| ICMPv6 | RFC 4443 | ICMPv6 | ✅ 部分实现（Echo） |
| TCP | RFC 793, RFC 9293 | 传输控制协议 | ✅ 已实现（基础功能） |
| UDP | RFC 768 | 用户数据报协议 | ✅ 已实现 |

## 开发日志

详细的开发进度和决策记录请查看 [开发日志](docs/log/dev_log.md)。

## 许可证

MIT License

## 致谢

本项目使用 AI 辅助设计和实现。
