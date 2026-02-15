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

## 6. 参考资料

- RFC 826 - An Ethernet Address Resolution Protocol
- RFC 1122 - Requirements for Internet Hosts
- RFC 5227 - IPv4 Address Conflict Detection
