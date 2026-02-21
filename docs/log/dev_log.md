# 协议实现计划

## 实现现状

### ✅ 已完整实现的协议

| 协议 | 状态 | 测试覆盖 |
|------|------|----------|
| **Ethernet** | 完整 | 单元测试 |
| **VLAN (802.1Q/802.1ad)** | 完整 | 集成测试 |
| **ARP** | 完整 + 缓存 | 集成测试 |
| **ICMPv4** | Echo + Unreachable + Time Exceeded | 集成测试 |
| **IPv4** | 基础实现（无分片重组） | 集成测试 |
| **IPv6** | 基础实现（无扩展头/分片） | 集成测试 |

### ❌ 未实现的协议

| 协议 | 优先级 | 依赖 |
|------|--------|------|
| **UDP** | P0 | IPv4/IPv6 |
| **TCP** | P0 | IPv4/IPv6 |
| **ICMPv6** | P1 | IPv6 |
| **IPv6 ND** | P2 | ICMPv6 |
| **IP 分片/重组** | P2 | IPv4/IPv6 |

---

## 主线任务

### 阶段一：传输层基础（P0 - 核心功能）

#### 1. UDP 协议实现
- [ ] 生成 UDP 协议设计文档 (`/skill proto-design udp`)
- [ ] 实现 UDP 协议代码 (`/skill proto-apply udp`)
  - [ ] UDP 头部解析/封装
  - [ ] 校验和计算（带伪头部）
  - [ ] 端口多路分发
- [ ] 编写集成测试

#### 2. TCP 协议实现（简化版）
- [ ] 生成 TCP 协议设计文档 (`/skill proto-design tcp`)
- [ ] 实现 TCP 协议代码 (`/skill proto-apply tcp`)
  - [ ] TCP 头部解析/封装
  - [ ] 连接状态机（SYN/ACK/FIN/RST）
  - [ ] 序列号/ACK 号处理
  - [ ] 窗口机制（基本）
- [ ] 编写集成测试

### 阶段二：IPv6 完善（P1 - 协议完整性）

#### 3. ICMPv6 协议实现
- [ ] 生成 ICMPv6 协议设计文档 (`/skill proto-design icmpv6`)
- [ ] 实现 ICMPv6 协议代码 (`/skill proto-apply icmpv6`)
  - [ ] Echo Request/Reply
  - [ ] Neighbor Solicitation/Advertisement
  - [ ] 错误消息处理
- [ ] 编写集成测试

#### 4. IPv6 扩展头支持
- [ ] 逐跳选项头
- [ ] 分片头
- [ ] 路由头
- [ ] 目的选项头

### 阶段三：IP 层增强（P2 - 网络兼容性）

#### 5. IP 分片与重组
- [ ] IPv4 分片/重组
- [ ] IPv6 分片/重组
- [ ] 重组超时处理
- [ ] 集成测试

#### 6. Neighbor Discovery (ND)
- [ ] 地址解析（类似 ARP）
- [ ] 重复地址检测
- [ ] 路由器发现
- [ ] 前缀发现

### 阶段四：应用层接口（P3 - 可用性）

#### 7. Socket API 设计与实现
- [ ] Socket 类型定义
- [ ] bind/listen/accept/connect
- [ ] send/recv 接口
- [ ] Socket 缓冲区管理

---

## 支线任务

- [ ] 代码重构优化
- [ ] 性能测试与优化
- [ ] 文档完善
- [ ] 示例程序开发

---

## 实现优先级

```
P0 (最高优先级): UDP, TCP
P1: ICMPv6, IPv6 扩展头
P2: IP 分片重组, ND
P3: Socket API
```
