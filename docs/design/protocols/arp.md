# ARP协议实现设计文档

## 1. 背景简介

### 1.1 协议作用

ARP（Address Resolution Protocol，地址解析协议）用于将IP地址解析为MAC地址。

**核心问题：** 在以太网中，设备通信使用MAC地址（硬件地址），而上层协议只知道IP地址（逻辑地址）。ARP负责完成IP地址到MAC地址的映射。

### 1.2 工作原理

**广播请求，单播响应：**
```
发送方                      接收方
   |                           |
   |--- ARP Request (广播) --->|  "谁有192.168.1.1？"
   |                           |
   |<-- ARP Reply (单播) -------|  "我是192.168.1.1，MAC是aa:bb:cc:dd:ee:ff"
```

**RFC参考：** RFC 826

---

## 2. 报文格式

### 2.1 报文结构

```
 0                   1                   2                   3
 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|     Hardware Type (HTYPE)     |     Protocol Type (PTYPE)    |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|  Hardware Address Length (HALEN) |  Protocol Address Length (PLEN) |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|              Operation (OPER)                 |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|                                                               |
+                  Sender Hardware Address (SHA)               +
|                                                               |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|                  Sender Protocol Address (SPA)               |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|                                                               |
+                  Target Hardware Address (THA)               +
|                                                               |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|                  Target Protocol Address (TPA)               |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
```

### 2.2 字段定义

| 字段 | 大小 | 说明 | 常见值 |
|------|------|------|---------|
| Hardware Type | 2字节 | 硬件类型 | 1 = Ethernet |
| Protocol Type | 2字节 | 协议类型 | 0x0800 = IPv4 |
| Hardware Addr Len | 1字节 | 硬件地址长度 | 6 (MAC地址) |
| Protocol Addr Len | 1字节 | 协议地址长度 | 4 (IPv4地址) |
| Operation | 2字节 | 操作码 | 1=请求, 2=响应 |
| Sender Hardware Addr | 可变 | 发送方MAC地址 | 6字节 |
| Sender Protocol Addr | 可变 | 发送方IP地址 | 4字节 |
| Target Hardware Addr | 可变 | 目标MAC地址 | 请求时为0 |
| Target Protocol Addr | 可变 | 目标IP地址 | 4字节 |

**最小报文长度：** 28字节（不含以太网头）

### 2.3 以太网封装

```
+------------------+----------------------+------------------+
|   Ethernet Header |       ARP Packet      |   Ethernet FCS  |
+------------------+----------------------+------------------+
| 14 bytes         | 28 bytes             | 4 bytes         |
+------------------+----------------------+------------------+

Ethernet Header:
- DST MAC: ff:ff:ff:ff:ff:ff (请求时广播，响应时单播)
- SRC MAC: 发送方MAC
- Ether Type: 0x0806 (ARP)
```

---

## 3. 状态变化

### 3.1 状态定义

每个ARP缓存条目有6种状态：

| 状态 | 说明 |
|------|------|
| NONE | 无映射，初始状态 |
| INCOMPLETE | 已发送请求，等待响应 |
| REACHABLE | 地址解析成功，可用 |
| STALE | 条目陈旧，可能失效 |
| DELAY | 延迟探测，避免频繁请求 |
| PROBE | 探测中，验证映射有效性 |

### 3.2 状态转换图

```
                    发送ARP请求
                         v
    +--------+       +------------+       +----------+
    |  NONE  |------>| INCOMPLETE  |------>| REACHABLE |
    +--------+       +------------+       +----------+
                         ^                   |
                         |                   | 老化超时
                         |                   v
                    +------------+       +----------+
                    |    NONE    |<------|  STALE   |
                    +------------+       +----------+
                                              |
                                    需要使用    | 收到Gratuitous ARP
                                              v
                    +------------+       +----------+
                    |    NONE    |<------|  DELAY   |
                    +------------+       +----------+
                                              |
                                    延迟到期    |
                                              v
                    +------------+       +----------+
                    |    NONE    |<------|  PROBE   |
                    +------------+       +----------+
```

### 3.3 收到报文后的状态转换

#### 3.3.1 收到ARP请求（Operation = 1）

**第一步：更新缓存（自动学习）**
```
对于收到的任何ARP报文，首先更新缓存：
- IP地址 = SPA (Sender Protocol Address)
- MAC地址 = SHA (Sender Hardware Address)
- 状态设为 REACHABLE
- 重置老化定时器
```

**第二步：判断是否需要响应**
```
if TPA (Target Protocol Address) == 本机任一接口IP地址 {
    需要响应
} else {
    不响应，仅更新缓存
}
```

**第三步：构造响应报文**
```
ARP Reply:
- Hardware Type = 1
- Protocol Type = 0x0800
- Hardware Addr Len = 6
- Protocol Addr Len = 4
- Operation = 2 (Reply)
- SHA = 本机MAC地址
- SPA = 本机IP地址 (即收到的TPA)
- THA = 收到的SHA
- TPA = 收到的SPA

以太网封装:
- DST MAC = 收到的SHA
- SRC MAC = 本机MAC
- Ether Type = 0x0806
```

#### 3.3.2 收到ARP响应（Operation = 2）

**第一步：更新缓存**
```
- IP地址 = SPA
- MAC地址 = SHA
- 状态设为 REACHABLE
- 重置老化定时器
```

**第二步：检查是否有等待的请求**
```
查找是否有该IP的 INCOMPLETE 状态条目：
if 存在 {
    状态改为 REACHABLE
    处理所有等待发送的数据包队列
    清空等待队列
}
```

**不响应：** 收到ARP响应后不需要发送任何响应报文

#### 3.3.3 收到Gratuitous ARP（免费ARP）

**识别特征：** SPA == TPA（源IP和目标IP相同）

**处理：**
```
1. 更新缓存（SPA -> SHA映射）
2. 状态设为 REACHABLE
3. 如果检测到IP冲突（已有相同IP映射但MAC不同）:
   报告地址冲突错误
```

---

## 4. 状态存储与管理

### 4.1 缓存条目结构

```rust
pub struct ArpEntry {
    // 网络接口索引
    pub ifindex: u32,

    // 协议地址 (IP)
    pub proto_addr: Ipv4Addr,

    // 硬件地址 (MAC)
    pub hardware_addr: MacAddr,

    // 条目状态
    pub state: ArpState,

    // 时间戳
    pub created_at: Instant,
    pub updated_at: Instant,
    pub confirmed_at: Instant,

    // 等待队列（INCOMPLETE状态时使用）
    pub pending_packets: VecDeque<Packet>,
}

pub enum ArpState {
    None,           // 无映射
    Incomplete,     // 等待响应
    Reachable,      // 可用
    Stale,         // 陈旧
    Delay,         // 延迟探测
    Probe,         // 探测中
}
```

### 4.2 从接收报文更新本地状态

```rust
// 伪代码：处理接收到的ARP报文
fn process_arp_packet(packet: &Packet, ifindex: u32, local_ips: &[Ipv4Addr]) {
    let operation = packet.get_operation();
    let spa = packet.get_spa();
    let sha = packet.get_sha();
    let tpa = packet.get_tpa();

    // 第一步：更新缓存（无论什么类型的ARP报文）
    update_arp_cache(ifindex, spa, sha, ArpState::Reachable);

    // 第二步：根据操作类型处理
    match operation {
        1 => { // ARP Request
            if local_ips.contains(&tpa) {
                // 目标是本机，需要响应
                send_arp_reply(ifindex, sha, spa, tpa);
            }
        }
        2 => { // ARP Reply
            // 检查是否有等待的请求
            if let Some(entry) = find_incomplete_entry(ifindex, spa) {
                entry.state = ArpState::Reachable;
                entry.hardware_addr = sha;
                process_pending_packets(entry);
            }
        }
        _ => {}
    }
}
```

### 4.3 根据本地表生成响应报文

```rust
// 伪代码：发送ARP响应
fn send_arp_reply(ifindex: u32, target_mac: MacAddr, target_ip: Ipv4Addr, local_ip: Ipv4Addr) {
    // 获取本机接口信息
    let local_mac = get_interface_mac(ifindex);

    // 构造ARP响应报文
    let mut reply = ArpPacket::new();
    reply.set_hardware_type(1);
    reply.set_protocol_type(0x0800);
    reply.set_hw_addr_len(6);
    reply.set_proto_addr_len(4);
    reply.set_operation(2); // Reply
    reply.set_sha(local_mac);
    reply.set_spa(local_ip);
    reply.set_tha(target_mac);
    reply.set_tpa(target_ip);

    // 封装成以太网帧
    let frame = EthernetFrame::new()
        .dst_mac(target_mac)
        .src_mac(local_mac)
        .ether_type(0x0806)
        .payload(reply.to_bytes());

    // 发送
    send_frame(ifindex, frame);
}
```

### 4.4 定时器管理

| 定时器类型 | 触发状态 | 超时时间 | 到期动作 |
|-----------|---------|---------|---------|
| 重传定时器 | INCOMPLETE | 1秒 | 重发ARP请求 |
| 老化定时器 | REACHABLE | 30秒 | 转为STALE状态 |
| 延迟定时器 | DELAY | 5秒 | 转为PROBE状态 |
| 探测定时器 | PROBE | 1秒 | 重发探测请求 |

### 4.5 配置参数

```rust
pub struct ArpConfig {
    pub retrans_timeout: u64,      // 重传超时（秒），默认1
    pub aging_timeout: u64,         // 老化超时（秒），默认30
    pub delay_timeout: u64,          // 延迟超时（秒），默认5
    pub probe_timeout: u64,          // 探测超时（秒），默认1
    pub max_retries: u32,           // 最大重试次数，默认3
    pub max_entries: usize,         // 最大缓存条目数，默认512
}
```

---

## 5. 与其他模块的交互

### 5.1 IPv4模块调用ARP

```
发送数据包时：
1. 查找目标IP的ARP缓存
2. 如果状态为 REACHABLE：
   - 直接获取MAC地址发送
3. 如果状态为 NONE 或 STALE：
   - 创建/更新条目为 INCOMPLETE
   - 发送ARP请求
   - 将数据包加入等待队列
4. 如果状态为 INCOMPLETE：
   - 将数据包加入等待队列
```

### 5.2 以太网模块交互

```
接收：
- 以太网Type = 0x0806 → 交给ARP模块

发送：
- ARP请求: DST MAC = ff:ff:ff:ff:ff:ff (广播)
- ARP响应: DST MAC = 目标MAC (单播)
```

---

## 6. 测试设计

### 6.1 报文接收场景测试

本节详细描述收到各种ARP报文后的预期行为，包括本地资源更新和响应报文内容。

#### 6.1.1 测试环境配置

```
本机接口配置：
- ifindex: 1
- MAC地址: 00:11:22:33:44:55
- IP地址: 192.168.1.10

远程主机：
- 主机A: MAC=aa:bb:cc:dd:ee:01, IP=192.168.1.100
- 主机B: MAC=aa:bb:cc:dd:ee:02, IP=192.168.1.200
```

#### 6.1.2 场景1：收到ARP请求（目标IP是本机）

**输入报文：**
```
ARP Request (Operation = 1):
  Hardware Type = 1
  Protocol Type = 0x0800
  Hardware Addr Len = 6
  Protocol Addr Len = 4
  SHA = aa:bb:cc:dd:ee:01 (主机A的MAC)
  SPA = 192.168.1.100 (主机A的IP)
  THA = 00:00:00:00:00:00 (广播)
  TPA = 192.168.1.10 (本机IP)

以太网封装:
  DST MAC = ff:ff:ff:ff:ff:ff
  SRC MAC = aa:bb:cc:dd:ee:01
  Ether Type = 0x0806
```

**本地资源更新：**
```rust
// ARP缓存新增/更新条目
ArpEntry {
    ifindex: 1,
    proto_addr: 192.168.1.100,
    hardware_addr: aa:bb:cc:dd:ee:01,
    state: Reachable,      // 从None/其他状态 -> Reachable
    created_at: <now>,
    updated_at: <now>,     // 刷新时间戳
    confirmed_at: <now>,   // 刷新确认时间
    pending_packets: [],   // 空队列
    retry_count: 0,
}
```

**响应报文：**
```
ARP Reply (Operation = 2):
  Hardware Type = 1
  Protocol Type = 0x0800
  Hardware Addr Len = 6
  Protocol Addr Len = 4
  Operation = 2
  SHA = 00:11:22:33:44:55 (本机MAC)
  SPA = 192.168.1.10 (本机IP，即收到的TPA)
  THA = aa:bb:cc:dd:ee:01 (请求者的MAC，即收到的SHA)
  TPA = 192.168.1.100 (请求者的IP，即收到的SPA)

以太网封装:
  DST MAC = aa:bb:cc:dd:ee:01 (单播给请求者)
  SRC MAC = 00:11:22:33:44:55
  Ether Type = 0x0806
```

#### 6.1.3 场景2：收到ARP请求（目标IP不是本机）

**输入报文：**
```
ARP Request (Operation = 1):
  SHA = aa:bb:cc:dd:ee:01
  SPA = 192.168.1.100
  THA = 00:00:00:00:00:00
  TPA = 192.168.1.200 (主机B的IP，不是本机)
```

**本地资源更新：**
```rust
// 仍然更新缓存（自动学习）
ArpEntry {
    ifindex: 1,
    proto_addr: 192.168.1.100,
    hardware_addr: aa:bb:cc:dd:ee:01,
    state: Reachable,  // 自动学习发送方的映射
    updated_at: <now>, // 刷新时间戳
    // ... 其他字段
}
```

**响应报文：** 无（不发送任何响应）

#### 6.1.4 场景3：收到ARP响应（匹配等待的请求）

**前提条件：** 已发送ARP请求，缓存中存在INCOMPLETE状态的条目

**初始状态：**
```rust
ArpEntry {
    ifindex: 1,
    proto_addr: 192.168.1.100,
    hardware_addr: 00:00:00:00:00:00,  // 未解析
    state: Incomplete,  // 等待响应
    pending_packets: [packet1, packet2],  // 有2个等待的数据包
    retry_count: 1,
}
```

**输入报文：**
```
ARP Reply (Operation = 2):
  SHA = aa:bb:cc:dd:ee:01
  SPA = 192.168.1.100
  THA = 00:11:22:33:44:55 (本机MAC)
  TPA = 192.168.1.10 (本机IP)
```

**本地资源更新：**
```rust
ArpEntry {
    ifindex: 1,
    proto_addr: 192.168.1.100,
    hardware_addr: aa:bb:cc:dd:ee:01,  // 更新为正确的MAC
    state: Reachable,      // Incomplete -> Reachable
    updated_at: <now>,
    confirmed_at: <now>,
    pending_packets: [],   // 清空等待队列，数据包被处理
    retry_count: 0,        // 重置重试计数
}
```

**响应报文：** 无

**附加行为：** 等待队列中的数据包应该被处理（发送到目标MAC）

#### 6.1.5 场景4：收到ARP响应（无等待的请求）

**前提条件：** 缓存中没有该IP的INCOMPLETE条目

**输入报文：**
```
ARP Reply (Operation = 2):
  SHA = aa:bb:cc:dd:ee:01
  SPA = 192.168.1.100
  THA = 00:11:22:33:44:55
  TPA = 192.168.1.10
```

**本地资源更新：**
```rust
// 仍会更新缓存（自动学习）
ArpEntry {
    ifindex: 1,
    proto_addr: 192.168.1.100,
    hardware_addr: aa:bb:cc:dd:ee:01,
    state: Reachable,  // 如果之前不存在，创建新条目
    // 如果已存在，更新硬件地址和时间戳
}
```

**响应报文：** 无

#### 6.1.6 场景5：收到Gratuitous ARP（免费ARP）

**识别特征：** SPA == TPA

**输入报文：**
```
ARP Request/Reply (Operation = 1或2):
  SHA = aa:bb:cc:dd:ee:01
  SPA = 192.168.1.100
  THA = 00:00:00:00:00:00
  TPA = 192.168.1.100  // SPA == TPA，免费ARP
```

**本地资源更新：**
```rust
ArpEntry {
    ifindex: 1,
    proto_addr: 192.168.1.100,
    hardware_addr: aa:bb:cc:dd:ee:01,
    state: Reachable,  // 更新为可达
    updated_at: <now>,
    confirmed_at: <now>,
}
```

**IP冲突检测：**
```rust
// 如果之前缓存中有相同的IP但不同的MAC
if existing_entry.hardware_addr != packet.sender_hardware_addr {
    // 报告IP冲突
    return Err(CoreError::ip_conflict(
        "192.168.1.100",
        "aa:bb:cc:dd:ee:01",
        existing_entry.hardware_addr
    ));
}
```

**响应报文：** 无（除非TPA是本机IP且是Request类型）

#### 6.1.7 场景6：收到重复的ARP报文

**输入报文：** 与场景1相同的请求

**本地资源更新：**
```rust
// 更新已有条目的时间戳
ArpEntry {
    proto_addr: 192.168.1.100,
    hardware_addr: aa:bb:cc:dd:ee:01,  // 保持不变
    state: Reachable,  // 保持不变
    updated_at: <now>,  // 刷新时间戳
    confirmed_at: <now>,  // 刷新确认时间
    // 不创建新条目
}
```

#### 6.1.8 场景7：收到格式错误的ARP报文

**错误情况1：长度不足**
```
输入：只有20字节的ARP报文
预期：返回 CoreError::invalid_packet("ARP报文长度不足")
本地资源：不更新任何缓存
响应：无
```

**错误情况2：无效的操作码**
```
输入：Operation = 999
预期：返回 CoreError::invalid_packet("无效的ARP操作码")
本地资源：不更新任何缓存
响应：无
```

**错误情况3：硬件地址长度不匹配**
```
输入：hardware_addr_len = 8（但实际是6）
预期：根据实现决定，建议拒绝
本地资源：不更新
响应：无
```

---

### 6.2 状态转换测试

#### 6.2.1 完整解析流程

| 初始状态 | 触发事件 | 期望结果 |
|---------|---------|---------|
| None | 发送ARP请求 | → Incomplete |
| Incomplete | 收到ARP响应 | → Reachable，pending_packets被处理 |
| Reachable | 老化超时(30秒) | → Stale |
| Stale | 需要发送数据 | → Delay |
| Delay | 延迟超时(5秒) | → Probe |
| Probe | 收到ARP响应 | → Reachable |
| Probe | 探测超时(1秒) | → Probe（重试）或 None（超过max_retries） |

#### 6.2.2 状态转换测试用例

```rust
// 测试用例1：正常解析流程
test_resolve_ip_success() {
    // 1. 初始状态：None
    // 2. 调用resolve_ip(192.168.1.100)
    // 3. 验证：状态变为Incomplete，发送了ARP请求
    // 4. 收到响应
    // 5. 验证：状态变为Reachable，pending_packets被清空
}

// 测试用例2：解析超时
test_resolve_ip_timeout() {
    // 1. 发送ARP请求，状态变为Incomplete
    // 2. 等待超过retrans_timeout * max_retries
    // 3. 验证：条目被删除或状态变为None
    // 4. pending_packets被丢弃
}

// 测试用例3：STALE状态刷新
test_stale_entry_refresh() {
    // 1. 创建Reachable条目
    // 2. 等待超过aging_timeout
    // 3. 验证：状态变为Stale
    // 4. 收到该IP的Gratuitous ARP
    // 5. 验证：状态变为Reachable
}
```

---

### 6.3 定时器测试

#### 6.3.1 重传定时器（Retransmission Timer）

| 测试场景 | 期望行为 |
|---------|---------|
| INCOMPLETE状态，1秒无响应 | 重发ARP请求，retry_count++ |
| 重试3次仍无响应 | 删除条目，清空pending_packets |
| 收到响应后立即取消 | 定时器停止，状态变为Reachable |

#### 6.3.2 老化定时器（Aging Timer）

| 测试场景 | 期望行为 |
|---------|---------|
| REACHABLE状态，30秒无更新 | 状态变为STALE |
| STALE状态收到任何该IP的ARP报文 | 状态变为REACHABLE，定时器重置 |
| STALE状态需要使用 | 触发延迟探测流程 |

#### 6.3.3 延迟定时器（Delay Timer）

| 测试场景 | 期望行为 |
|---------|---------|
| STALE状态需要使用数据 | 状态变为DELAY，启动5秒定时器 |
| DELAY期间收到该IP的ARP报文 | 状态变为REACHABLE，取消定时器 |
| DELAY定时器到期 | 状态变为PROBE，发送探测请求 |

#### 6.3.4 探测定时器（Probe Timer）

| 测试场景 | 期望行为 |
|---------|---------|
| PROBE状态，1秒无响应 | 重发探测请求（ARP请求） |
| 收到探测响应 | 状态变为REACHABLE |
| 超过max_retries无响应 | 状态变为None，删除条目 |

---

### 6.4 边界条件测试

#### 6.4.1 缓存容量限制

```rust
// 测试用例：缓存满时的行为
test_cache_full() {
    // 1. 创建max_entries个条目（默认512）
    // 2. 尝试添加第513个条目
    // 3. 验证：根据策略（LRU/拒绝/覆盖）
    //    建议：使用LRU策略淘汰最旧的条目
}
```

#### 6.4.2 等待队列溢出

```rust
// 测试用例：大量数据包等待ARP解析
test_pending_packets_overflow() {
    // 1. 创建INCOMPLETE条目
    // 2. 添加大量数据包到pending_packets
    // 3. 验证：是否有最大队列限制
    //    建议：设置max_pending_packets限制
}
```

#### 6.4.3 特殊IP地址

```rust
// 测试用例：处理特殊IP地址
test_special_addresses() {
    // 测试以下IP的处理：
    // - 0.0.0.0
    // - 255.255.255.255
    // - 224.0.0.0/24 (组播)
    // 期望：拒绝或特殊处理
}
```

---

### 6.5 多接口测试

#### 6.5.1 接口隔离

```rust
// 测试用例：不同接口的ARP缓存独立
test_interface_isolation() {
    // 接口1: 192.168.1.10/24
    // 接口2: 192.168.2.10/24
    //
    // 1. 在接口1解析192.168.1.100
    // 2. 在接口2查询192.168.1.100
    // 3. 验证：接口2的缓存为空（接口隔离）
}
```

#### 6.5.2 同IP多接口

```rust
// 测试用例：相同IP在不同接口
test_same_ip_multiple_interfaces() {
    // 接口1和接口2配置相同IP（不常见但可能）
    // 验证：ARP缓存使用 (ifindex, ip) 作为key
}
```

---

### 6.6 集成测试

#### 6.6.1 IPv4模块调用ARP

```rust
// 测试用例：IPv4发送数据包触发ARP解析
test_ipv4_arp_integration() {
    // 1. IPv4模块要发送数据到192.168.1.100
    // 2. 缓存中无该IP的映射
    // 3. 验证：ARP请求被发送
    // 4. 数据包被加入pending_packets
    // 5. 收到ARP响应
    // 6. 验证：数据包被正确发送
}
```

#### 6.6.2 以太网封装

```rust
// 测试用例：验证ARP报文的以太网封装
test_ethernet_encapsulation() {
    // 1. 构造ARP请求
    // 2. 封装为以太网帧
    // 3. 验证：
    //    - 请求时：DST MAC = ff:ff:ff:ff:ff:ff
    //    - 响应时：DST MAC = 请求者的MAC
    //    - Ether Type = 0x0806
}
```

---

### 6.7 测试实现建议

#### 6.7.1 测试框架结构

```rust
// tests/arp_test.rs
pub struct ArpTestContext {
    pub cache: ArpCache,
    pub ifindex: u32,
    pub local_mac: MacAddr,
    pub local_ips: Vec<Ipv4Addr>,
    pub packets_sent: Vec<ArpPacket>,
}

impl ArpTestContext {
    pub fn new() -> Self { ... }
    pub fn send_arp_packet(&mut self, packet: &ArpPacket) -> Result<()> { ... }
    pub fn assert_cache_entry(&self, ip: Ipv4Addr, expected: &ArpEntry) { ... }
    pub fn assert_reply_sent(&self, expected: &ArpPacket) { ... }
}
```

#### 6.7.2 测试辅助函数

```rust
/// 创建测试用的ARP请求报文
fn create_test_request(spa: Ipv4Addr, tpa: Ipv4Addr) -> ArpPacket { ... }

/// 创建测试用的ARP响应报文
fn create_test_reply(spa: Ipv4Addr, sha: MacAddr) -> ArpPacket { ... }

/// 模拟时间流逝（用于定时器测试）
fn advance_time(duration: Duration) { ... }

/// 验证缓存状态
fn assert_cache_state(cache: &ArpCache, ip: Ipv4Addr, expected_state: ArpState) { ... }
```

---

## 7. 实现状态

### 7.1 已实现功能

| 功能模块 | 状态 | 说明 |
|---------|------|------|
| ARP报文解析 | ✅ | `ArpPacket::from_packet()` |
| ARP报文编码 | ✅ | `ArpPacket::to_bytes()` |
| 以太网封装 | ✅ | `encapsulate_ethernet()` |
| 报文处理 | ✅ | `handle_arp_packet()` |
| 缓存管理 | ✅ | `ArpCache` 完整实现 |
| 状态转换 | ✅ | 所有6种状态转换 |
| 等待队列 | ✅ | `pending_packets` 处理 |
| IP冲突检测 | ✅ | Gratuitous ARP 冲突检测 |
| LRU淘汰 | ✅ | 缓存满时自动淘汰 |
| 特殊IP过滤 | ✅ | 拒绝0.0.0.0/广播/组播 |
| 主动解析 | ✅ | `resolve_ip()` 函数 |
| ARP请求发送 | ✅ | `send_arp_request()` 函数 |
| 定时器处理 | ✅ | `process_arp_timers()` 函数 |

### 7.2 依赖组件

| 组件 | 位置 | 状态 |
|------|------|------|
| 定时器系统 | `src/common/timer.rs` | ✅ 已实现 |
| 错误类型 | `src/common/error.rs` | ✅ 已添加`IpConflict` |
| SystemContext | `src/context.rs` | ✅ 已集成定时器 |

### 7.3 待集成功能

以下功能已实现但需要上层模块集成：

1. **定时器驱动调度**：`process_arp_timers()` 需要在主循环或调度器中定期调用
2. **IPv4模块集成**：`resolve_ip()` 需要在IPv4发送数据包时调用
3. **Stale状态使用**：`mark_used()` 需要在发送数据包前查询时调用

### 7.4 测试覆盖

| 测试类型 | 文件 | 状态 |
|---------|------|------|
| 报文接收场景 | `arp_integration_test.rs` | ✅ |
| 状态转换 | `arp_integration_test.rs` | ✅ |
| 定时器 | `arp_integration_test.rs` | ✅ |
| 边界条件 | `arp_integration_test.rs` | ✅ |
| 多接口 | `arp_integration_test.rs` | ✅ |

---

## 8. 参考资料

- RFC 826 - An Ethernet Address Resolution Protocol
- RFC 1122 - Requirements for Internet Hosts
- RFC 5227 - IPv4 Address Conflict Detection
