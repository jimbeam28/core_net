# CoreNet

> 一个使用 Rust 实现的简易网络协议栈，用于学习/研究目的。

## 项目简介

CoreNet 是一个**纯模拟**的网络协议栈实现，支持完整的 TCP/IP 协议族。项目采用分层架构设计，代码结构清晰，便于理解网络协议原理。

### 特点

- **纯模拟环境**：不使用真实网络接口（TUN/TAP），仅通过队列传递报文
- **分层架构**：链路层 → 网络层 → 传输层 → 应用层
- **零外部依赖**：仅使用 Rust 标准库
- **学习导向**：代码设计注重可读性和可理解性
- **模块化**：便于添加新协议支持

### 支持的协议

- **链路层**：Ethernet、ARP
- **网络层**：IPv4、IPv6
- **传输层**：TCP、UDP、ICMP、ICMPv6
- **辅助协议**：邻居发现(ND)

## 项目架构

```
┌──────────────┐         ┌──────────────┐         ┌──────────────┐
│  测试/模拟    │  ───>  │  处理线程    │  ───>  │  结果输出     │
│  注入器       │  RxQ   │  (Engine)     │  TxQ   │  (可选)       │
└──────────────┘         └──────────────┘         └──────────────┘

测试数据流向：
  上行（解析）：注入器 → RxQ → 协议解析 → TxQ → 输出
  下行（封装）：测试输入 → 协议封装 → TxQ → 验证
```

## 目录结构

```
core_net/
├── docs/                 # 设计文档
│   ├── arch.md            # 架构设计（主文档）
│   └── detail/           # 详细设计文档
│       ├── queue.md       # 队列设计
│       ├── packet.md      # 报文描述符
│       ├── types.md       # 通用类型
│       ├── nic.md         # 模拟网卡
│       ├── error.md       # 错误处理
│       └── test.md        # 测试工具
├── src/                  # 源代码
│   ├── main.rs           # 主入口
│   ├── lib.rs            # 库入口
│   ├── common/           # 通用模块
│   │   ├── mod.rs
│   │   ├── error.rs       # 错误类型定义
│   │   ├── packet.rs      # 报文描述符
│   │   ├── queue.rs       # 环形队列
│   │   └── types.rs       # 通用类型(MacAddr等）
│   ├── protocols/         # 协议实现
│   │   ├── mod.rs
│   │   ├── ethernet/     # 以太网层
│   │   ├── ipv4/         # IPv4层
│   │   ├── ipv6/         # IPv6层
│   │   ├── tcp/          # TCP协议
│   │   ├── udp/          # UDP协议
│   │   ├── icmp/         # ICMP协议
│   │   ├── icmpv6/       # ICMPv6协议
│   │   ├── arp/          # ARP协议
│   │   └── nd/          # 邻居发现
│   ├── engine/           # 协议处理引擎
│   │   └── mod.rs
│   └── test/            # 测试工具
│       ├── mod.rs
│       ├── injector.rs   # 报文注入器
│       └── output.rs     # 结果输出
├── Cargo.toml          # 项目配置
└── README.md          # 本文件
```

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

### 阶段一：基础框架
- [x] 目录结构创建
- [ ] common 模块实现
- [ ] engine 模块实现
- [ ] test 模块实现

### 阶段二：基础协议
- [ ] 以太网层
- [ ] ARP 协议
- [ ] IPv4 基础
- [ ] ICMP 协议

**目标**：能够 ping 通网关

### 阶段三：传输层
- [ ] UDP 协议
- [ ] TCP 基础实现

**目标**：能够建立 TCP 连接并传输数据

### 阶段四：IPv6
- [ ] IPv6 基础
- [ ] ICMPv6
- [ ] 邻居发现

### 阶段五：应用接口
- [ ] Socket API
- [ ] 测试工具完善

## 设计文档

详细设计文档请查看 [docs/arch.md](docs/arch.md)：

- [队列设计](detail/queue.md) - 环形队列实现细节
- [报文描述符](detail/packet.md) - Packet 结构设计
- [通用类型](detail/types.md) - MacAddr、IpAddr 等类型定义
- [模拟网卡](detail/nic.md) - 模拟网卡接口设计
- [错误处理](detail/error.md) - 错误类型定义和处理
- [测试工具](detail/test.md) - 报文注入和结果输出设计

## 参考资料

本项目遵循以下 RFC 标准：

- [RFC 791](https://datatracker.github.io/rfc/rfc791) - Internet Protocol (IPv4)
- [RFC 793](https://datatracker.github.io/rfc/rfc793) - Transmission Control Protocol (TCP)
- [RFC 768](https://datatracker.github.io/rfc/rfc768) - User Datagram Protocol (UDP)
- [RFC 792](https://datatracker.github.io/rfc/rfc792) - Internet Control Message Protocol (ICMP)
- [RFC 826](https://datatracker.github.io/rfc/rfc826) - An Ethernet Address Resolution Protocol (ARP)
- [RFC 2460](https://datatracker.github.io/rfc/rfc2460) - Internet Protocol, Version 6 (IPv6)

## 许可证

MIT License

## 致谢

本项目使用 AI 辅助设计和实现。
