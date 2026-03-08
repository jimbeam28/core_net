# CoreNet 代码精简方案

## 合并小文件记录 (2026-03-08)

### 已完成的合并

| 原文件 | 目标文件 | 减少行数 |
|--------|----------|----------|
| `src/protocols/tcp/constant.rs` | `src/protocols/tcp/mod.rs` | ~74 |
| `src/protocols/tcp/error.rs` | `src/protocols/tcp/mod.rs` | ~142 |
| `src/common/tables.rs` | `src/common/mod.rs` | ~32 |
| `src/protocols/ip/protocol.rs` | `src/protocols/ip/mod.rs` | ~126 |
| `src/protocols/vlan/frame.rs` | `src/protocols/vlan/tag.rs` | ~37 |
| `src/route/error.rs` | `src/route/mod.rs` | ~65 |
| `src/protocols/udp/config.rs` | `src/protocols/udp/mod.rs` | ~100 |
| `src/protocols/ipv6/config.rs` | `src/protocols/ipv6/mod.rs` | ~101 |
| `src/protocols/icmpv6/error.rs` | `src/protocols/icmpv6/mod.rs` | ~66 |
| `src/protocols/icmpv6/config.rs` | `src/protocols/icmpv6/mod.rs` | ~174 |
| `src/poweron/mod.rs` (删除) | - | ~343 |

**总计减少约 460 行代码**

### 合并前
- 总计: 37,680 行
- 文件数: ~120 个

### 合并后
- 总计: 37,220 行
- 文件数: ~110 个

## 当前代码统计

| 类型 | 行数 | 占比 |
|------|------|------|
| 实际代码 | 32,127 | 85.2% |
| 注释 | 7,207 | 19.1% |
| 空行 | 5,553 | 14.7% |
| **总计** | **37,680** | 100% |

## 各模块代码分布

```
protocols/  - 28,149 行 (74.7%)
  ├── tcp       - 4,322 行
  ├── ospf      - 2,958 行
  ├── icmpv6    - 2,635 行
  ├── icmp      - 2,334 行
  ├── ip        - 2,471 行
  ├── ipv6      - 2,119 行
  ├── ipsec     - 2,067 行
  ├── udp       - 1,827 行
  ├── ospf2     - 1,728 行
  ├── bgp       - 1,560 行
  ├── ospf3     - 1,217 行
  ├── vlan      - 1,219 行
  ├── arp       - 1,326 行
  └── ethernet  - 168 行

engine/     - 2,086 行 (5.5%)
interface/  - 2,026 行 (5.4%)
common/     - 1,805 行 (4.8%)
route/      - 1,053 行 (2.8%)
scheduler/  - 1,042 行 (2.8%)
testframework/ - 501 行 (1.3%)
poweron/    - 343 行 (0.9%)
```

## 精简目标

从 37,680 行精简到 10,000 行左右（约精简 73%）

---

## 一、删除高级路由协议（预计减少 6,500 行）

### 1.1 删除 OSPF 相关代码 (-5,903 行)

```
src/protocols/ospf/     - 2,958 行 (核心模块)
src/protocols/ospf2/    - 1,728 行 (OSPFv2)
src/protocols/ospf3/    - 1,217 行 (OSPFv3)
```

**理由**：
- OSPF 是高级内部网关协议，教学用 TCP/IP 栈可以暂时不需要
- 保留基础的路由表功能即可

### 1.2 删除 BGP 代码 (-1,560 行)

```
src/protocols/bgp/      - 1,560 行
```

**理由**：
- BGP 是边界网关协议，复杂度极高
- 对基础网络学习非必需

---

## 二、删除 IPsec 模块（预计减少 2,067 行）

```
src/protocols/ipsec/    - 2,067 行
├── ah.rs
├── esp.rs
├── sa.rs
└── mod.rs
```

**理由**：
- IPsec 包含复杂的 SA/SPD 管理、加密、认证
- 可保留接口但删除完整实现，或完全移除

---

## 三、精简协议实现（预计减少 4,000 行）

### 3.1 精简 TCP 模块 (-1,500 行)

当前：4,322 行 → 目标：2,800 行

**精简方向**：
- 合并 `socket.rs` 和 `socket_manager.rs`
- 简化 TCB 状态机的注释和文档
- 删除部分边缘情况的复杂处理
- 精简 `segment.rs` 中的辅助函数

### 3.2 精简 ICMP/ICMPv6 (-800 行)

当前：4,969 行 → 目标：4,200 行

**精简方向**：
- 简化 ICMPv6 NDP 实现
- 合并重复的报文处理逻辑
- 删除过于详细的注释

### 3.3 精简 IPv6 模块 (-600 行)

当前：2,119 行 → 目标：1,500 行

**精简方向**：
- 简化扩展头处理
- 精简分片重组逻辑

### 3.4 精简 VLAN 模块 (-500 行)

当前：1,219 行 → 目标：700 行

**精简方向**：
- 删除复杂的 VLAN 过滤逻辑
- 简化双层标签处理

### 3.5 精简其他协议 (-600 行)

- ARP: 1,326 行 → 900 行
- UDP: 1,827 行 → 1,300 行
- IP: 2,471 行 → 1,800 行

---

## 四、精简冗余代码结构（预计减少 2,000 行）

### 4.1 合并 mod.rs 文件

当前每个模块都有独立的 mod.rs，可以合并：

```
// 之前
src/common/mod.rs       - 30 行
src/engine/mod.rs       - 15 行
src/interface/mod.rs    - 11 行
src/route/mod.rs        - 14 行
...

// 之后：使用 lib.rs 统一导出
```

### 4.2 删除废弃代码

```
src/poweron/            - 343 行 (legacy context)
src/interface/global.rs - 删除（已在 lib.rs 中 deprecated）
```

### 4.3 合并小文件

将一些小文件合并到主文件中：

```
# 可合并的文件
src/protocols/tcp/constant.rs  → 合并到 mod.rs
src/protocols/tcp/error.rs     → 合并到 mod.rs
src/protocols/udp/port.rs      → 合并到 socket.rs
src/protocols/ip/protocol.rs   → 合并到 mod.rs
src/protocols/ip/error.rs      → 合并到 mod.rs
src/route/error.rs             → 合并到 mod.rs
```

---

## 五、精简注释和文档（预计减少 4,000 行）

当前注释 7,207 行，可精简至 3,000 行

### 5.1 删除过度注释

- 删除显而易见的注释（如 getter/setter 的说明）
- 删除重复描述 RFC 内容的注释
- 简化复杂算法前的长篇说明

### 5.2 精简文档注释

- 当前文档注释 5,028 行
- 目标文档注释 2,000 行
- 保留函数签名和关键参数说明

---

## 六、精简测试框架（预计减少 300 行）

```
src/testframework/      - 501 行 → 300 行
```

- 简化错误处理代码
- 合并部分测试辅助函数

---

## 七、精简空行（预计减少 3,000 行）

当前空行 5,553 行，可精简至 2,500 行

**精简方向**：
- 删除连续多个空行
- 删除函数内不必要的空行
- 删除文件末尾的多个空行

---

## 精简优先级和阶段

### 第一阶段：删除完整模块（立即执行，减少 8,567 行）

```bash
# 删除高级路由协议
rm -rf src/protocols/ospf/
rm -rf src/protocols/ospf2/
rm -rf src/protocols/ospf3/
rm -rf src/protocols/bgp/

# 删除 IPsec
rm -rf src/protocols/ipsec/
```

### 第二阶段：合并文件和删除废弃代码（减少 1,500 行）

- 合并小文件
- 删除 poweron 模块
- 删除 interface/global.rs

### 第三阶段：精简注释和空行（减少 7,000 行）

- 脚本自动删除多余空行
- 手动审查并精简过度注释

### 第四阶段：简化协议实现（减少 4,000 行）

- 逐模块简化实现
- 合并重复逻辑

---

## 预期效果

| 阶段 | 预计减少 | 累计剩余 |
|------|----------|----------|
| 初始 | - | 37,680 |
| 第一阶段 | 8,567 | 29,113 |
| 第二阶段 | 1,500 | 27,613 |
| 第三阶段 | 7,000 | 20,613 |
| 第四阶段 | 10,000 | 10,613 |

**最终目标：约 10,000 行代码**

---

## 保留的核心功能

精简后保留的基础功能：

```
common/       - 基础数据结构（Packet, Queue, Address）
engine/       - 协议处理器
interface/    - 网络接口管理
route/        - 基础路由表
scheduler/    - 包调度器
context/      - 系统上下文
socket/       - Socket API

protocols/
├── ethernet  - 以太网帧处理
├── vlan      - VLAN 标签（简化版）
├── arp       - ARP 协议（简化版）
├── ip        - IPv4（简化版）
├── ipv6      - IPv6（简化版）
├── icmp      - ICMP Echo（简化版）
├── icmpv6    - ICMPv6 Echo + NDP（简化版）
├── udp       - UDP + Socket（简化版）
└── tcp       - TCP + Socket（简化版）

testframework/ - 测试框架（简化版）
```

---

## 实施建议

1. **先做第一阶段**：删除整个模块风险最低，可立即减少大量代码
2. **保留 Git 历史**：每次精简后提交，便于回滚
3. **逐步验证**：每次精简后运行 `cargo build && cargo test`
4. **保留设计文档**：docs/design/ 下的协议文档有价值，保留供学习

---

## 附：原精简方案

以下内容为原精简方案，保留作为参考：

### 原方案 - Socket 模块（完全删除）

删除文件：
```
src/socket/mod.rs
src/socket/types.rs
src/socket/entry.rs
src/socket/manager.rs
src/socket/error.rs
```

依赖清理：
- `src/lib.rs`: 删除 Socket 模块导出
- `src/context.rs`: 删除 `socket_mgr` 字段
- `src/engine/processor.rs`: 删除 Socket 相关调用

### 原方案 - IPsec 模块（简化加密实现）

删除 IKEv2 目录：
```
src/protocols/ipsec/ikev2/
```

简化加密为 mock 实现：
```rust
pub fn mock_encrypt(data: &[u8], _key: &[u8]) -> Vec<u8> {
    data.to_vec()
}

pub fn mock_decrypt(data: &[u8], _key: &[u8]) -> Vec<u8> {
    data.to_vec()
}
```

### 原方案 - IPv6 模块（简化扩展头）

简化 `process_ipv6_packet()`：
```rust
pub fn process_ipv6_packet(packet: &mut Packet, ...) -> Result<...> {
    let header = Ipv6Header::parse(packet)?;

    // 简化：只处理直接上层协议，不遍历扩展头链
    match header.next_header {
        IpProtocol::IcmpV6 | IpProtocol::Tcp | IpProtocol::Udp => {
            // 直接处理
        }
        _ => {
            // 扩展头直接跳过
            return Ok(Ipv6ProcessResult::NoReply);
        }
    }
}
```

### 原方案 - OSPF 模块（简化状态机）

简化邻居状态机：
```rust
pub enum SimpleNeighborState {
    Down,
    Init,
    Full,  // 合并所有后续状态
}
```

### 原方案 - BGP 模块（简化 FSM）

简化对等体状态：
```rust
pub enum SimpleBgpState {
    Idle,
    Connecting,  // 合并 Connect + Active
    Established,
}
```

### 原方案 - TCP 模块（简化拥塞控制）

简化状态机：
```rust
pub enum TcpState {
    Closed,
    SynSent,
    SynReceived,
    Established,
    FinWait,      // 合并 FinWait1 + FinWait2
    TimeWait,
}
```

简化定时器：
```rust
pub struct SimpleTimers {
    pub retransmission: Option<Instant>,
}
```
