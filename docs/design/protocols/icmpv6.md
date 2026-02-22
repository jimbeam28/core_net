# ICMPv6 (Internet Control Message Protocol Version 6) 详细设计文档

## 1. 协议概述

### 1.1 背景与历史

**协议全称和定义：**
- 协议全称：Internet Control Message Protocol Version 6（互联网控制消息协议第六版）
- 在 TCP/IP 协议栈中的层级位置：网络层（与 IPv6 同层）
- 核心功能概述：用于在 IPv6 主机、路由器之间传递控制消息，包括错误报告、诊断信息和邻居发现功能

**为什么需要该协议？**

ICMPv6 是 IPv6 协议栈的重要组成部分，相比 ICMPv4，它承担了更多的功能：

- **错误报告**：通知发送方数据报传输过程中出现的问题
- **诊断工具**：支持 ping6（连通性测试）和 traceroute6（路由跟踪）
- **邻居发现**：替代 IPv4 的 ARP 协议，实现地址解析、路由器发现、前缀发现等功能
- **自动配置**：支持无状态地址自动配置（SLAAC）
- **组播管理**：支持 MLD（Multicast Listener Discovery）

**历史背景：**
- **RFC 2463**：1998年12月发布，定义了最初的 ICMPv6 规范
- **RFC 4443**：2006年3月发布，更新并取代 RFC 2463，是当前的标准规范
- **RFC 4861**：2007年9月发布，定义了邻居发现协议（NDP），使用 ICMPv6 消息类型 133-137
- **相关补充 RFC**：
  - RFC 4862（IPv6 无状态地址自动配置）
  - RFC 2710（MLDv1）/ RFC 3810（MLDv2）
  - RFC 4291（IPv6 地址架构）
  - RFC 8200（IPv6 基础规范，更新 RFC 2460）

### 1.2 设计原理

ICMPv6 的核心设计思想是**统一的消息框架和扩展性**。与 ICMPv4 相比，ICMPv6 整合了多个 IPv4 协议的功能，使用统一的 ICMPv6 消息格式传递不同类型的信息。

```
                        IPv6 协议栈层级
    +------------------------------------------------+
    |              应用层 (HTTP/FTP/...)              |
    +------------------------------------------------+
                         ↓↑
    +------------------------------------------------+
    |         传输层 (TCP/UDP)                        |
    +------------------------------------------------+
                         ↓↑
    +---------------+----------------+----------------+
    |   ICMPv6      |      IPv6      |    其他协议    |
    +---------------+----------------+----------------+
                         ↓↑
    +------------------------------------------------+
    |         数据链路层 (Ethernet)                   |
    +------------------------------------------------+
```

**ICMPv6 消息分类：**

ICMPv6 消息分为两大类：

```
+---------------------------+
|     ICMPv6 消息类型        |
+---------------------------+
|                           |
|   错误消息 (Error)         |
|   Type: 0 - 127           |
|   - 目标不可达            |
|   - 数据包过大            |
|   - 超时                  |
|   - 参数问题              |
|                           |
+---------------------------+
|                           |
|   信息消息 (Informational) |
|   Type: 128 - 255         |
|   - Echo 请求/响应        |
|   - 邻居发现消息          |
|   - 组播监听发现          |
|                           |
+---------------------------+
```

**ICMPv6 与 ICMPv4 的主要区别：**

| 特性 | ICMPv4 | ICMPv6 |
|------|--------|--------|
| 协议号 | IP Protocol 1 | IPv6 Next Header 58 |
| 地址解析 | 使用 ARP | 使用 ICMPv6 ND |
| 路由器发现 | ICMP Router Advertisement (可选) | ICMPv6 Router Advertisement (必需) |
| 消息分类 | 混合分类 | 明确分为错误(0-127)和信息(128-255) |
| 组播支持 | 广播为主 | 组播为主，减少主机负载 |
| MTU 处理 | Fragmentation Needed (Type 3, Code 4) | Packet Too Big (Type 2) |
| 校验和计算 | 包含伪头部 | 包含 IPv6 伪头部 |

**关键特点：**

1. **统一的邻居发现**：通过 ICMPv6 实现 ARP、Router Discovery、Redirect 的功能
2. **组播优化**：使用组播代替广播，减少非目标主机的处理负担
3. **扩展性强**：使用选项格式支持未来扩展
4. **必需功能**：ICMPv6 邻居发现是 IPv6 的必需功能，非可选
5. **地址自动配置**：支持无状态地址自动配置（SLAAC）
6. **安全考虑**：支持 SEND（Secure Neighbor Discovery）

---

## 2. 报文格式

### 2.1 报文结构

ICMPv6 报文封装在 IPv6 数据报中，IPv6 头部的 Next Header 字段值为 58。ICMPv6 报文格式如下：

```
 0                   1                   2                   3
 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|     Type      |     Code     |          Checksum             |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|                       (取决于消息类型)                        |
+                                                               +
|                            ...                                |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
```

**IPv6 伪头部（用于校验和计算）：**

```
 0                   1                   2                   3
 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|                                                               |
+                                                               +
|                  源 IPv6 地址 (128 位)                        |
+                                                               +
|                                                               |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|                                                               |
+                                                               +
|                  目的 IPv6 地址 (128 位)                      |
+                                                               +
|                                                               |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|                   包长度 (32 位)                              |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|              Zero | Next Header=58 |  保留 (24 位)           |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
```

### 2.2 通用字段说明

| 字段 | 大小 | 说明 | 常见值 |
|------|------|------|--------|
| Type | 1 字节 | ICMPv6 消息类型 | 0-255 (见下表) |
| Code | 1 字节 | 类型子代码，提供详细信息 | 0-6 |
| Checksum | 2 字节 | 整个 ICMPv6 报文的校验和 | 计算值 |
| Rest | 4+ 字节 | 取决于消息类型 | - |

**最小报文长度：** 8 字节（头部）
**最大报文长度：** 受限于 IPv6 MTU（最小 1280 字节）

### 2.3 ICMPv6 消息类型

#### 错误消息 (Type 0-127)

| Type | 名称 | 用途 |
|------|------|------|
| 1 | Destination Unreachable | 目标不可达 |
| 2 | Packet Too Big | 数据包过大（需分片） |
| 3 | Time Exceeded | 超时 |
| 4 | Parameter Problem | 参数问题 |

#### 信息消息 (Type 128-255)

| Type | 名称 | 用途 |
|------|------|------|
| 128 | Echo Request | Echo 请求（ping6） |
| 129 | Echo Reply | Echo 响应 |
| 130 | Multicast Listener Query | MLD 查询 |
| 131 | Multicast Listener Report | MLD 报告 |
| 132 | Multicast Listener Done | MLD 完成 |
| 133 | Router Solicitation | 路由器请求 (NDP) |
| 134 | Router Advertisement | 路由器通告 (NDP) |
| 135 | Neighbor Solicitation | 邻居请求 (NDP) |
| 136 | Neighbor Advertisement | 邻居通告 (NDP) |
| 137 | Redirect | 重定向 (NDP) |

### 2.4 封装格式

**ICMPv6 封装在 IPv6 中：**

```
+------------------+
|   IPv6 Header    |  Next Header = 58
+------------------+
|  ICMPv6 Header   |  Type, Code, Checksum
+------------------+
| ICMPv6 Payload   |  消息特定数据
+------------------+
```

**错误消息额外包含：**

```
+------------------+
|   IPv6 Header    |  Next Header = 58
+------------------+
|  ICMPv6 Header   |  Type=Error, Code
+------------------+
|     Unused       |  4 字节（部分消息）
+------------------+
| Original IPv6 Hdr|  原始数据报的 IPv6 头
+------------------+
| Original Data    |  原始数据报的部分数据
+------------------+
```

---

## 3. 状态机设计

### 3.0 状态变量

ICMPv6 本质上是一个**无状态协议**，但邻居发现功能需要维护状态：

| 变量名称 | 类型 | 用途 | 初始值 |
|---------|------|------|--------|
| echo_identifier | u16 | Echo 请求的标识符 | 随机生成 |
| echo_sequence | u16 | Echo 请求的序列号 | 0 或随机 |
| pending_echoes | HashMap | 未完成的 Echo 请求 | 空 |
| neighbor_cache | HashMap | 邻居缓存 (IPv6 -> MAC) | 空 |
| router_list | Vec | 已知的路由器列表 | 空 |
| prefix_list | Vec | 网络前缀列表 | 空 |

### 3.1 邻居发现状态机

#### 3.1.1 邻居缓存条目状态

```
    +-----------+                 +------------+
    |  INCOMPLETE|---NS/超时----->|  REACHABLE |
    +-----------+                 +------------+
         ^                              |
         |                              | 可达时间过期
         |                              v
    +-----------+                 +------------+
    |   STALE   |<---确认/数据----+  PROBE     |
    +-----------+                 +------------+
         |                              ^
         |                              |
         +----------NS/NA循环-----------+
```

**状态定义：**

| 状态 | 描述 |
|------|------|
| INCOMPLETE | 地址解析正在进行，已发送 NS，等待 NA |
| REACHABLE | 邻居可达，最近有确认 |
| STALE | 邻居可能不可达，未验证，但可正常使用 |
| DELAY | STALE 状态后发送数据时进入，延迟发送 NS |
| PROBE | 正在主动探测邻居可达性 |
| PERMANENT | 静态配置，永不过期 |

### 3.2 Echo 请求-响应流程

#### 3.2.1 发送 Echo Request

**描述：** 发起一个 Echo 请求（ping6）

**进入条件：** 应用层或用户触发 ping6 操作

**行为：**
1. 生成 identifier 和 sequence number
2. 记录发送时间戳
3. 构建 Echo Request ICMPv6 消息
4. 通过 IPv6 层发送

**状态变量更新：**
- `pending_echoes[(identifier, sequence)] = 当前时间戳`

#### 3.2.2 等待 Echo Reply

**描述：** 等待 Echo Request 的响应

**超时设置：** 默认 1-5 秒（可配置）

**转换条件：**
- 收到匹配的 Echo Reply → 完成状态
- 超时 → 失败状态（可选择重试）

#### 3.2.3 收到 Echo Reply

**描述：** 收到 Echo Reply 消息

**匹配条件：** identifier 和 sequence 与 pending_echoes 中的条目匹配

**处理步骤：**
1. 从 `pending_echoes` 查找对应的请求时间戳
2. 计算往返时间 (RTT) = 当前时间 - 请求时间戳
3. 从 `pending_echoes` 删除该条目
4. 向应用层报告结果

---

## 4. 报文处理逻辑

### 4.0 定时器

ICMPv6 使用的定时器：

| 定时器名称 | 启动条件 | 超时时间 | 超时动作 |
|-----------|---------|---------|---------|
| echo_timeout | 发送 Echo Request 后 | 默认 1-5 秒 | 标记请求失败，可选重试 |
| reachable_time | 邻居进入 REACHABLE 状态 | 根据 RA 计算 | 转换到 STALE 状态 |
| retrans_timer | 发送 NS 后 | 默认 1000ms | 重传 NS 或标记失败 |
| dad_timeout | 发送 DAD NS 后 | 1 秒 | 地址唯一，完成配置 |
| router_solicitation_delay | 接口启动时 | 0-1 秒随机 | 发送 RS |

### 4.1 接收处理总流程

```
                    收到 IPv6 数据报
                         |
                         v
                 [Next Header == 58?] ---否---> 丢弃
                         |
                        是
                         |
                         v
                 [校验和正确?] ---否----> 丢弃
                         |
                        是
                         |
                         v
                 +----------------+
                 |  提取 Type/Code|
                 +----------------+
                         |
         +---------------+---------------+---------------+
         |               |               |               |
         v               v               v               v
   [Error Msg]   [Echo Request]  [Echo Reply]   [NDP Msg]
         |               |               |               |
         v               v               v               v
    [4.2 节]        [4.3 节]         [4.4 节]        [4.5-4.9 节]
         |               |               |               |
         v               v               v               v
    [可选应答]      [发送 Reply]    [应用通知]    [NDP 处理]
```

### 4.2 Destination Unreachable (Type 1)

**用途：** 通知发送方数据报无法到达最终目的地

**Code 定义：**

| Code | 名称 | 说明 |
|------|------|------|
| 0 | No route to destination | 没有到目的地的路由 |
| 1 | Communication with destination administratively prohibited | 与目的地的通信被管理禁止 |
| 2 | Beyond scope of source address | 超出源地址范围 |
| 3 | Address unreachable | 地址不可达 |
| 4 | Port unreachable | 端口不可达 |
| 5 | Source address failed ingress/egress policy | 源地址未通过入口/出口策略 |
| 6 | Reject route to destination | 拒绝到目的地的路由 |

**报文格式：**

```
 0                   1                   2                   3
 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|     Type=1    |     Code     |          Checksum             |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|                             Unused                            |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|                    Original IPv6 Header                      |
+                                                               +
|                                                               |
+                                                               +
|                                                               |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|                  Original Data (as much as possible)          |
+                                                               +
|                            ...                                |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
```

**处理流程：**

1. **提取信息：**
   - Type = 1, Code → 具体的不可达原因
   - 原始 IPv6 数据报头部

2. **处理步骤：**
   - 解析原始 IPv6 数据报头部
   - 提取源地址，确定应该通知谁
   - 根据 Code 确定不可达的具体原因

3. **资源更新：**
   - 无需更新 ICMPv6 相关表项（无状态）

4. **响应动作：**
   - 不发送 ICMPv6 响应（避免循环）
   - 可选：通知上层协议或应用层

### 4.3 Packet Too Big (Type 2)

**用途：** 通知发送方数据包超过路径 MTU，需要分片

**报文格式：**

```
 0                   1                   2                   3
 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|     Type=2    |     Code=0   |          Checksum             |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|                             MTU                               |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|                    Original IPv6 Header                      |
+                                                               +
|                                                               |
+                                                               +
|                                                               |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|                  Original Data (as much as possible)          |
+                                                               +
|                            ...                                |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
```

**处理流程：**

1. **提取信息：**
   - MTU → 路径 MTU 值
   - 原始 IPv6 数据报头部

2. **处理步骤：**
   - 更新路径 MTU 缓存
   - 通知上层协议需要减小包大小

3. **资源更新：**
   - PMTU 表：更新到目标地址的路径 MTU

4. **响应动作：**
   - 不发送 ICMPv6 响应

### 4.4 Time Exceeded (Type 3)

**用途：** 通知发送方数据报的 Hop Limit 已过期或分片重组超时

**Code 定义：**

| Code | 名称 | 说明 |
|------|------|------|
| 0 | Hop limit exceeded in transit | 传输中跳数限制过期 |
| 1 | Fragment reassembly time exceeded | 分片重组时间过期 |

**报文格式：**

```
 0                   1                   2                   3
 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|     Type=3    |     Code     |          Checksum             |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|                             Unused                            |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|                    Original IPv6 Header                      |
+                                                               +
|                                                               |
+                                                               +
|                                                               |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|                  Original Data (as much as possible)          |
+                                                               +
|                            ...                                |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
```

**处理流程：**

1. **提取信息：**
   - Type = 3, Code → Hop Limit 过期或分片重组超时
   - 原始 IPv6 头和数据

2. **处理步骤：**
   - 解析原始数据报，确定源地址
   - 对于 Code=0，这是 traceroute6 工作的基础

3. **资源更新：**
   - 无需更新表项

4. **响应动作：**
   - 不发送 ICMPv6 响应

### 4.5 Parameter Problem (Type 4)

**用途：** 指示 IPv6 头部或扩展头部存在参数错误

**Code 定义：**

| Code | 名称 | 说明 |
|------|------|------|
| 0 | Erroneous header field encountered | 遇到错误的头部字段 |
| 1 | Unrecognized Next Header type encountered | 遇到无法识别的 Next Header 类型 |
| 2 | Unrecognized IPv6 option encountered | 遇到无法识别的 IPv6 选项 |

**报文格式：**

```
 0                   1                   2                   3
 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|     Type=4    |     Code     |          Checksum             |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|                            Pointer                             |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|                    Original IPv6 Header                      |
+                                                               +
|                                                               |
+                                                               +
|                                                               |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|                  Original Data (as much as possible)          |
+                                                               +
|                            ...                                |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
```

**处理流程：**

1. **提取信息：**
   - Pointer → 指向错误字节的偏移量
   - 原始 IPv6 头和数据

2. **处理步骤：**
   - 解析原始 IPv6 头
   - 根据 Pointer 定位错误字段

3. **资源更新：**
   - 无需更新表项

4. **响应动作：**
   - 不发送 ICMPv6 响应
   - 记录错误信息

### 4.6 Echo Request (Type 128)

**用途：** ping6 请求，用于测试目的地的可达性和往返时间

**报文格式：**

```
 0                   1                   2                   3
 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|     Type=128  |     Code=0   |          Checksum             |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|           Identifier          |        Sequence Number       |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|                             Data                              |
+                                                               +
|                            ...                                |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
```

**处理流程：**

1. **提取信息：**
   - Identifier → 标识符
   - Sequence Number → 序列号
   - Data → 可选的负载数据

2. **处理步骤：**
   - 验证校验和
   - 提取 Identifier 和 Sequence Number

3. **资源更新：**
   - 无需更新表项（无状态）

4. **响应动作：**
   - **必须**发送 Echo Reply (Type 129, Code 0)
   - Echo Reply 包含相同的 Identifier、Sequence Number 和 Data

### 4.7 Echo Reply (Type 129)

**用途：** ping6 响应，确认目的地可达并返回负载数据

**报文格式：** 与 Echo Request 相同，Type = 129

**处理流程：**

1. **提取信息：**
   - Identifier → 标识符
   - Sequence Number → 序列号
   - Data → 原始请求数据

2. **处理步骤：**
   - 在 `pending_echoes` 中查找匹配的请求
   - 如果找到，计算 RTT
   - 如果未找到，可能是重复或延迟的响应

3. **资源更新：**
   - 表项：`pending_echoes[(identifier, sequence)]` - **删除**
   - 状态变量：记录 RTT 统计信息

4. **响应动作：**
   - 不发送 ICMPv6 响应
   - 向应用层报告 ping6 结果

### 4.8 Router Solicitation (Type 133)

**用途：** 主机请求路由器立即发送 Router Advertisement

**报文格式：**

```
 0                   1                   2                   3
 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|     Type=133  |     Code=0   |          Checksum             |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|                            Options                            |
+                                                               +
|                            ...                                |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
```

**常见选项：**
- Source Link-Layer Address（源链路层地址）

**处理流程：**

1. **提取信息：**
   - Options → 源链路层地址等

2. **处理步骤：**
   - 验证消息来自本地链路
   - 提取源地址和链路层地址

3. **资源更新：**
   - 邻居缓存：添加/更新发送方的邻居条目

4. **响应动作：**
   - 路由器应立即发送 Router Advertisement

### 4.9 Router Advertisement (Type 134)

**用途：** 路由器定期通告其存在和网络配置信息

**报文格式：**

```
 0                   1                   2                   3
 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|     Type=134  |     Code=0   |          Checksum             |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
| Cur Hop Limit |M|O|H|Resvd| Lifetime |          Reachable Time           |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|                          Retrans Timer                        |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|                            Options                            |
+                                                               +
|                            ...                                |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
```

**标志位：**
- M（Managed Address Configuration）：是否使用有状态地址配置（DHCPv6）
- O（Other Configuration）：是否获取其他配置信息（通过 DHCPv6）
- H（Home Agent）：路由器是否作为移动 IPv6 的家乡代理

**常见选项：**
- Source Link-Layer Address
- MTU
- Prefix Information
- Recursive DNS Server (RFC 8106)

**处理流程：**

1. **提取信息：**
   - Cur Hop Limit → 默认 Hop Limit 值
   - Flags → 配置标志
   - Lifetime → 路由器生命周期
   - Reachable Time → 邻居可达时间
   - Retrans Timer → NS 重传定时器
   - Options → 前缀、MTU、DNS 等信息

2. **处理步骤：**
   - 更新默认路由器列表
   - 更新前缀列表
   - 配置接口参数

3. **资源更新：**
   - 路由器列表：添加/更新路由器条目
   - 前缀列表：更新网络前缀信息
   - 邻居缓存：添加路由器的链路层地址

4. **响应动作：**
   - 不发送 ICMPv6 响应
   - 可能触发地址自动配置

### 4.10 Neighbor Solicitation (Type 135)

**用途：** 地址解析（IPv6 到 MAC）、重复地址检测（DAD）、邻居可达性探测

**报文格式：**

```
 0                   1                   2                   3
 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|     Type=135  |     Code=0   |          Checksum             |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|                           Reserved                            |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|                                                               |
+                                                               +
|                  Target IPv6 Address                         |
+                                                               +
|                                                               |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|                            Options                            |
+                                                               +
|                            ...                                |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
```

**常见选项：**
- Source Link-Layer Address（地址解析时）

**处理流程：**

1. **提取信息：**
   - Target Address → 目标 IPv6 地址
   - Options → 源链路层地址等

2. **处理步骤：**
   - 判断消息类型（地址解析/DAD/NUD）
   - 对于 DAD：如果目标地址是本机地址，则地址冲突
   - 对于地址解析：回复 Neighbor Advertisement

3. **资源更新：**
   - 邻居缓存：添加/更新邻居条目

4. **响应动作：**
   - 发送 Neighbor Advertisement（如果目标地址是本机地址）
   - 或记录 DAD 冲突

### 4.11 Neighbor Advertisement (Type 136)

**用途：** 响应 Neighbor Solicitation，或主动通告链路层地址变化

**报文格式：**

```
 0                   1                   2                   3
 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|     Type=136  |     Code=0   |          Checksum             |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|R|S|O|                     Reserved                            |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|                                                               |
+                                                               +
|                  Target IPv6 Address                         |
+                                                               +
|                                                               |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|                            Options                            |
+                                                               +
|                            ...                                |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
```

**标志位：**
- R（Router）：发送方是路由器
- S（Solicited）：响应 NS 而发送
- O（Override）：覆盖现有的缓存条目

**常见选项：**
- Target Link-Layer Address

**处理流程：**

1. **提取信息：**
   - Target Address → 目标 IPv6 地址
   - Flags → 路由器标志、响应标志等
   - Options → 目标链路层地址

2. **处理步骤：**
   - 更新邻居缓存
   - 根据标志决定是否覆盖现有条目

3. **资源更新：**
   - 邻居缓存：添加/更新邻居条目，状态设为 REACHABLE 或 STALE

4. **响应动作：**
   - 不发送 ICMPv6 响应

### 4.12 Redirect (Type 137)

**用途：** 路由器通知主机有更好的第一跳路由器

**报文格式：**

```
 0                   1                   2                   3
 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|     Type=137  |     Code=0   |          Checksum             |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|                           Reserved                            |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|                                                               |
+                                                               +
|                  Target IPv6 Address                         |
+                                                               +
|                                                               |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|                                                               |
+                                                               +
|                  Destination IPv6 Address                    |
+                                                               +
|                                                               |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|                            Options                            |
+                                                               +
|                            ...                                |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
```

**常见选项：**
- Target Link-Layer Address

**处理流程：**

1. **提取信息：**
   - Target Address → 更好的下一跳地址
   - Destination Address → 最终目标地址

2. **处理步骤：**
   - 验证 Redirect 来自当前第一跳路由器
   - 更新路由缓存/前缀列表

3. **资源更新：**
   - 路由表：添加/更新到目标的路由
   - 邻居缓存：添加 Target 的链路层地址

4. **响应动作：**
   - 不发送 ICMPv6 响应

---

## 5. 核心数据结构

### 5.0 表项/缓存

ICMPv6 邻居发现需要维护的表项和缓存：

| 资源名称 | 用途 | 最大容量 | 淘汰策略 |
|---------|------|---------|---------|
| pending_echoes | 匹配 Echo Request/Reply | 由配置决定 | 超时自动删除 |
| neighbor_cache | 邻居缓存 (IPv6 -> MAC) | 由配置决定 | 超时/可达性检测失败 |
| router_list | 默认路由器列表 | 由配置决定 | 路由器 Lifetime 过期 |
| prefix_list | 网络前缀列表 | 由配置决定 | 前缀 Valid Lifetime 过期 |
| pmtu_cache | 路径 MTU 缓存 | 由配置决定 | 超时后更新 |

#### 5.0.1 邻居缓存

**用途：** 存储 IPv6 地址到链路层地址的映射

**关键操作：**
- 查询(IPv6 地址) → 返回 MAC 地址或 INCOMPLETE
- 添加(收到 NA 或主动配置) → 创建新条目
- 更新(收到 NA 或可达性确认) → 更新状态和可达时间
- 删除(超时或失败) → 移除条目

### 5.1 报文结构

```rust
/// ICMPv6 报文头部（通用格式）
#[repr(C, packed)]
pub struct Icmpv6Header {
    /// ICMPv6 消息类型
    pub type_: u8,
    /// 类型子代码
    pub code: u8,
    /// 校验和
    pub checksum: u16,
}

/// IPv6 伪头部（用于校验和计算）
#[repr(C, packed)]
pub struct Ipv6PseudoHeader {
    /// 源 IPv6 地址
    pub source_addr: [u8; 16],
    /// 目的 IPv6 地址
    pub dest_addr: [u8; 16],
    /// 包长度
    pub packet_length: u32,
    /// Zero + Next Header (58) + Reserved
    pub next_header: u32,  // 高24位为0，第24-31位为58
}

/// Echo Request/Reply 报文
#[repr(C, packed)]
pub struct Icmpv6Echo {
    /// 类型 (128=Request, 129=Reply)
    pub type_: u8,
    /// 代码 (始终为 0)
    pub code: u8,
    /// 校验和
    pub checksum: u16,
    /// 标识符
    pub identifier: u16,
    /// 序列号
    pub sequence: u16,
}

/// Destination Unreachable 报文
#[repr(C, packed)]
pub struct Icmpv6DestUnreachable {
    /// 类型 (1)
    pub type_: u8,
    /// 不可达代码
    pub code: u8,
    /// 校验和
    pub checksum: u16,
    /// 未使用（填充为 0）
    pub unused: u32,
    /// 原始 IPv6 头部紧随其后
}

/// Packet Too Big 报文
#[repr(C, packed)]
pub struct Icmpv6PacketTooBig {
    /// 类型 (2)
    pub type_: u8,
    /// 代码 (始终为 0)
    pub code: u8,
    /// 校验和
    pub checksum: u16,
    /// MTU
    pub mtu: u32,
    /// 原始 IPv6 头部紧随其后
}

/// Time Exceeded 报文
#[repr(C, packed)]
pub struct Icmpv6TimeExceeded {
    /// 类型 (3)
    pub type_: u8,
    /// 超时代码
    pub code: u8,
    /// 校验和
    pub checksum: u16,
    /// 未使用（填充为 0）
    pub unused: u32,
    /// 原始 IPv6 头部紧随其后
}

/// Parameter Problem 报文
#[repr(C, packed)]
pub struct Icmpv6ParameterProblem {
    /// 类型 (4)
    pub type_: u8,
    /// 参数问题代码
    pub code: u8,
    /// 校验和
    pub checksum: u16,
    /// 指向错误字节的指针
    pub pointer: u32,
    /// 原始 IPv6 头部紧随其后
}

/// Router Solicitation 报文
#[repr(C, packed)]
pub struct Icmpv6RouterSolicitation {
    /// 类型 (133)
    pub type_: u8,
    /// 代码 (始终为 0)
    pub code: u8,
    /// 校验和
    pub checksum: u16,
    /// 保留（必须为 0）
    pub reserved: u32,
    /// 选项紧随其后
}

/// Router Advertisement 报文
#[repr(C, packed)]
pub struct Icmpv6RouterAdvertisement {
    /// 类型 (134)
    pub type_: u8,
    /// 代码 (始终为 0)
    pub code: u8,
    /// 校验和
    pub checksum: u16,
    /// 当前 Hop Limit
    pub cur_hop_limit: u8,
    /// 标志位 (M|O|H|Reserved)
    pub flags: u8,
    /// 路由器生命周期（秒）
    pub lifetime: u16,
    /// 可达时间（毫秒）
    pub reachable_time: u32,
    /// 重传定时器（毫秒）
    pub retrans_timer: u32,
    /// 选项紧随其后
}

/// Neighbor Solicitation 报文
#[repr(C, packed)]
pub struct Icmpv6NeighborSolicitation {
    /// 类型 (135)
    pub type_: u8,
    /// 代码 (始终为 0)
    pub code: u8,
    /// 校验和
    pub checksum: u16,
    /// 保留（必须为 0）
    pub reserved: u32,
    /// 目标 IPv6 地址
    pub target_address: [u8; 16],
    /// 选项紧随其后
}

/// Neighbor Advertisement 报文
#[repr(C, packed)]
pub struct Icmpv6NeighborAdvertisement {
    /// 类型 (136)
    pub type_: u8,
    /// 代码 (始终为 0)
    pub code: u8,
    /// 校验和
    pub checksum: u16,
    /// 标志位 (R|S|O|Reserved)
    pub flags: u8,
    /// 保留（必须为 0）
    pub reserved: [u8; 3],
    /// 目标 IPv6 地址
    pub target_address: [u8; 16],
    /// 选项紧随其后
}

/// Redirect 报文
#[repr(C, packed)]
pub struct Icmpv6Redirect {
    /// 类型 (137)
    pub type_: u8,
    /// 代码 (始终为 0)
    pub code: u8,
    /// 校验和
    pub checksum: u16,
    /// 保留（必须为 0）
    pub reserved: u32,
    /// 更好的下一跳地址
    pub target_address: [u8; 16],
    /// 最终目标地址
    pub destination_address: [u8; 16],
    /// 选项紧随其后
}

/// ICMPv6 选项
#[repr(C, packed)]
pub struct Icmpv6Option {
    /// 选项类型
    pub option_type: u8,
    /// 选项长度（以 8 字节为单位）
    pub option_length: u8,
    /// 选项数据
    pub data: [u8; 0],  // 柔性数组
}
```

### 5.2 枚举类型

```rust
/// ICMPv6 消息类型
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Icmpv6Type {
    // 错误消息 (0-127)
    /// Destination Unreachable
    DestinationUnreachable = 1,
    /// Packet Too Big
    PacketTooBig = 2,
    /// Time Exceeded
    TimeExceeded = 3,
    /// Parameter Problem
    ParameterProblem = 4,

    // 信息消息 (128-255)
    /// Echo Request
    EchoRequest = 128,
    /// Echo Reply
    EchoReply = 129,
    /// Multicast Listener Query
    MldQuery = 130,
    /// Multicast Listener Report
    MldReport = 131,
    /// Multicast Listener Done
    MldDone = 132,
    /// Router Solicitation
    RouterSolicitation = 133,
    /// Router Advertisement
    RouterAdvertisement = 134,
    /// Neighbor Solicitation
    NeighborSolicitation = 135,
    /// Neighbor Advertisement
    NeighborAdvertisement = 136,
    /// Redirect
    Redirect = 137,
}

impl Icmpv6Type {
    /// 从 u8 解析 ICMPv6 类型
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            1 => Some(Icmpv6Type::DestinationUnreachable),
            2 => Some(Icmpv6Type::PacketTooBig),
            3 => Some(Icmpv6Type::TimeExceeded),
            4 => Some(Icmpv6Type::ParameterProblem),
            128 => Some(Icmpv6Type::EchoRequest),
            129 => Some(Icmpv6Type::EchoReply),
            130 => Some(Icmpv6Type::MldQuery),
            131 => Some(Icmpv6Type::MldReport),
            132 => Some(Icmpv6Type::MldDone),
            133 => Some(Icmpv6Type::RouterSolicitation),
            134 => Some(Icmpv6Type::RouterAdvertisement),
            135 => Some(Icmpv6Type::NeighborSolicitation),
            136 => Some(Icmpv6Type::NeighborAdvertisement),
            137 => Some(Icmpv6Type::Redirect),
            _ => None,
        }
    }

    /// 判断是否为错误消息
    pub fn is_error_message(&self) -> bool {
        (*self as u8) < 128
    }

    /// 判断是否为信息消息
    pub fn is_informational(&self) -> bool {
        (*self as u8) >= 128
    }
}

/// Destination Unreachable 代码
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Icmpv6DestUnreachableCode {
    /// No route to destination
    NoRoute = 0,
    /// Administratively prohibited
    AdminProhibited = 1,
    /// Beyond scope of source address
    BeyondScope = 2,
    /// Address unreachable
    AddressUnreachable = 3,
    /// Port unreachable
    PortUnreachable = 4,
    /// Source address failed policy
    SourcePolicyFailed = 5,
    /// Reject route
    RejectRoute = 6,
}

/// Time Exceeded 代码
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Icmpv6TimeExceededCode {
    /// Hop limit exceeded
    HopLimitExceeded = 0,
    /// Fragment reassembly time exceeded
    ReassemblyTimeout = 1,
}

/// Parameter Problem 代码
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Icmpv6ParameterProblemCode {
    /// Erroneous header field
    HeaderField = 0,
    /// Unrecognized Next Header
    UnrecognizedNextHeader = 1,
    /// Unrecognized option
    UnrecognizedOption = 2,
}

/// ICMPv6 选项类型
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Icmpv6OptionType {
    /// Source Link-Layer Address
    SourceLinkLayerAddr = 1,
    /// Target Link-Layer Address
    TargetLinkLayerAddr = 2,
    /// Prefix Information
    PrefixInfo = 3,
    /// Redirected Header
    RedirectedHeader = 4,
    /// MTU
    Mtu = 5,
    /// Recursive DNS Server (RFC 8106)
    RecursiveDns = 25,
    /// Route Information (RFC 4191)
    RouteInfo = 24,
}

/// 邻居缓存条目状态
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NeighborCacheState {
    /// 地址解析进行中
    Incomplete = 0,
    /// 邻居可达
    Reachable = 1,
    /// 邻居可能不可达
    Stale = 2,
    /// 延迟发送 NS
    Delay = 3,
    /// 正在探测
    Probe = 4,
    /// 永久条目
    Permanent = 5,
}
```

### 5.3 邻居缓存管理

```rust
use std::collections::HashMap;
use std::time::Instant;
use crate::common::addr::{Ipv6Addr, MacAddr};

/// 邻居缓存条目
#[derive(Debug, Clone)]
pub struct NeighborCacheEntry {
    /// IPv6 地址
    pub ipv6_addr: Ipv6Addr,
    /// 链路层地址
    pub link_layer_addr: Option<MacAddr>,
    /// 条目状态
    pub state: NeighborCacheState,
    /// 是否为路由器
    pub is_router: bool,
    /// 进入当前状态的时间
    pub state_since: Instant,
    /// 可达时间（毫秒）
    pub reachable_time: Option<u32>,
}

/// 邻居缓存
pub struct NeighborCache {
    /// 缓存条目 (IPv6 地址 -> 条目)
    entries: HashMap<Ipv6Addr, NeighborCacheEntry>,
    /// 最大条目数
    max_entries: usize,
    /// 默认可达时间（毫秒）
    default_reachable_time: u32,
}

impl NeighborCache {
    /// 创建新的邻居缓存
    pub fn new(max_entries: usize, default_reachable_time: u32) -> Self {
        Self {
            entries: HashMap::new(),
            max_entries,
            default_reachable_time,
        }
    }

    /// 查询邻居缓存
    pub fn lookup(&self, addr: &Ipv6Addr) -> Option<&NeighborCacheEntry> {
        self.entries.get(addr)
    }

    /// 添加或更新邻居条目
    pub fn update(&mut self, addr: Ipv6Addr, link_layer_addr: MacAddr, is_router: bool, state: NeighborCacheState) {
        let entry = NeighborCacheEntry {
            ipv6_addr: addr,
            link_layer_addr: Some(link_layer_addr),
            state,
            is_router,
            state_since: Instant::now(),
            reachable_time: Some(self.default_reachable_time),
        };

        // 如果缓存已满，删除最旧的 STALE 条目
        if self.entries.len() >= self.max_entries {
            self.evict_stale();
        }

        self.entries.insert(addr, entry);
    }

    /// 标记条目为 INCOMPLETE（开始地址解析）
    pub fn mark_incomplete(&mut self, addr: Ipv6Addr) {
        if let Some(entry) = self.entries.get_mut(&addr) {
            entry.state = NeighborCacheState::Incomplete;
            entry.state_since = Instant::now();
        } else {
            let entry = NeighborCacheEntry {
                ipv6_addr: addr,
                link_layer_addr: None,
                state: NeighborCacheState::Incomplete,
                is_router: false,
                state_since: Instant::now(),
                reachable_time: None,
            };
            self.entries.insert(addr, entry);
        }
    }

    /// 处理可达性超时
    pub fn handle_timeout(&mut self) {
        let now = Instant::now();
        for entry in self.entries.values_mut() {
            match entry.state {
                NeighborCacheState::Reachable => {
                    if let Some(reachable_time) = entry.reachable_time {
                        let elapsed = now.duration_since(entry.state_since).as_millis() as u32;
                        if elapsed >= reachable_time {
                            entry.state = NeighborCacheState::Stale;
                            entry.state_since = now;
                        }
                    }
                }
                _ => {}
            }
        }
    }

    /// 淘汰 STALE 条目
    fn evict_stale(&mut self) {
        if let Some(addr) = self.entries.iter()
            .filter(|(_, e)| e.state == NeighborCacheState::Stale)
            .min_by_key(|(_, e)| e.state_since)
            .map(|(addr, _)| *addr)
        {
            self.entries.remove(&addr);
        }
    }
}
```

### 5.4 路由器列表和前缀管理

```rust
/// 默认路由器条目
#[derive(Debug, Clone)]
pub struct DefaultRouterEntry {
    /// 路由器 IPv6 地址
    pub router_addr: Ipv6Addr,
    /// 路由器链路层地址
    pub link_layer_addr: MacAddr,
    /// 路由器生命周期（秒）
    pub lifetime: u16,
    /// 上次更新时间
    pub last_update: Instant,
}

/// 网络前缀条目
#[derive(Debug, Clone)]
pub struct PrefixEntry {
    /// 前缀
    pub prefix: Ipv6Addr,
    /// 前缀长度
    pub prefix_length: u8,
    /// 有效生命周期（秒）
    pub valid_lifetime: u32,
    /// 优先生命周期（秒）
    pub preferred_lifetime: u32,
    /// 上次更新时间
    pub last_update: Instant,
}

/// 路由器列表
pub struct RouterList {
    routers: Vec<DefaultRouterEntry>,
}

impl RouterList {
    pub fn new() -> Self {
        Self { routers: Vec::new() }
    }

    pub fn add_or_update(&mut self, router: DefaultRouterEntry) {
        if let Some(entry) = self.routers.iter_mut().find(|r| r.router_addr == router.router_addr) {
            *entry = router;
        } else {
            self.routers.push(router);
        }
    }

    pub fn remove_expired(&mut self) {
        let now = Instant::now();
        self.routers.retain(|r| {
            now.duration_since(r.last_update).as_secs() < r.lifetime as u64
        });
    }

    pub fn get_best_router(&self) -> Option<&DefaultRouterEntry> {
        self.routers.first()
    }
}

/// 前缀列表
pub struct PrefixList {
    prefixes: Vec<PrefixEntry>,
}

impl PrefixList {
    pub fn new() -> Self {
        Self { prefixes: Vec::new() }
    }

    pub fn add_or_update(&mut self, prefix: PrefixEntry) {
        if let Some(entry) = self.prefixes.iter_mut().find(|p| {
            p.prefix == prefix.prefix && p.prefix_length == prefix.prefix_length
        }) {
            *entry = prefix;
        } else {
            self.prefixes.push(prefix);
        }
    }

    pub fn remove_expired(&mut self) {
        let now = Instant::now();
        self.prefixes.retain(|p| {
            now.duration_since(p.last_update).as_secs() < p.valid_lifetime as u64
        });
    }
}
```

---

## 6. 与其他模块的交互

### 6.1 模块依赖关系图

```
                        +---------------------------+
                        |       应用层/API          |
                        |    (ping6/NDP/配置)       |
                        +---------------------------+
                                 |        ^
                                 v        |
    +----------------+    +---------------------------+    +------------------+
    | Engine/        |    |       ICMPv6 模块         |    |  IPv6 模块       |
    | Processor      |<-->|   (protocols/icmpv6/)     |<-->| (protocols/ipv6) |
    +----------------+    +---------------------------+    +------------------+
         |                        ^        |                      ^
         v                        |        v                      |
    +----------------+    +---------------------------+    +------------------+
    |  Scheduler     |    |      Interface 模块       |    |  Ethernet 模块   |
    +----------------+    |   (interface/)            |    |(protocols/eth)   |
                          +---------------------------+    +------------------+
                                    ^        |
                                    |        v
                          +---------------------------+
                          |      Common 模块          |
                          | (packet/error/addr/queue)|
                          +---------------------------+
```

### 6.2 与 Common 模块的交互

**依赖的 Common 子模块：**

| 子模块 | 使用内容 | 用途 |
|-------|---------|------|
| `common/packet.rs` | `Packet` 结构体 | ICMPv6 报文的封装和解析 |
| `common/error.rs` | `CoreError` | 错误处理和返回 |
| `common/addr.rs` | `Ipv6Addr`, `MacAddr` | IPv6 地址和 MAC 地址类型 |
| `common/queue.rs` | `RxQueue`, `TxQueue` | 数据包的收发队列 |

**具体使用方式：**

```rust
// 从 common/packet.rs
use crate::common::packet::Packet;
// 用途：接收 IPv6 数据报，构造 ICMPv6 响应报文

// 从 common/error.rs
use crate::common::error::CoreError;
// 用途：返回解析错误、校验和错误等

// 从 common/addr.rs
use crate::common::addr::{Ipv6Addr, MacAddr};
// 用途：源/目的 IPv6 地址的解析和设置
```

### 6.3 与 Interface 模块的交互

**依赖的 Interface 子模块：**

| 子模块 | 使用内容 | 用途 |
|-------|---------|------|
| `SystemContext.interfaces` | `Arc<Mutex<InterfaceManager>>` | 获取接口信息 |
| `interface/iface.rs` | `Interface` 结构体 | 接口状态和配置 |
| `interface/types.rs` | 接口相关类型 | MTU、地址等 |

**具体交互：**

```rust
// 获取接收接口的 IPv6 地址（用于 ICMPv6 响应的源地址）
let iface = context
    .interfaces
    .lock()
    .unwrap()
    .get_interface_by_name(received_iface_name)?;

let source_addr = iface.ipv6_addr;  // 用作 ICMPv6 响应的源地址
let mtu = iface.mtu;                // 检查 ICMPv6 报文长度
```

**交互场景：**
- **发送 ICMPv6 响应时**：查询接收接口的 IPv6 地址作为源地址
- **处理 RA 时**：更新接口配置参数（Hop Limit、MTU 等）
- **地址自动配置**：根据前缀信息配置接口地址

### 6.4 与 IPv6 模块的交互

**依赖内容：**

| IPv6 模块组件 | 用途 |
|-------------|------|
| IPv6 头部解析 | 提取源/目的地址、Next Header 字段 |
| IPv6 报文封装 | 将 ICMPv6 报文封装为 IPv6 数据报 |
| 路由查询 | 确定 ICMPv6 响应的发送接口 |
| 扩展头部处理 | 处理 IPv6 扩展头部 |

**具体交互：**

```rust
// IPv6 层接收处理 (在 IPv6 模块中)
// 当 Next Header = 58 时，分发给 ICMPv6 模块
match ipv6_header.next_header {
    58 => {
        // ICMPv6 协议
        let icmpv6_packet = parse_icmpv6(&packet.payload())?;
        icmpv6_module.handle_icmpv6(icmpv6_packet, &ipv6_header)?;
    }
    // ... 其他协议
}

// ICMPv6 模块发送 (通过 IPv6 模块)
use crate::protocols::ipv6::Ipv6Header;

let ipv6_header = Ipv6Header {
    source_addr: my_ipv6,         // 接收接口的 IPv6
    dest_addr: original_source,   // 原始数据报的源地址
    next_header: 58,              // ICMPv6
    hop_limit: 64,
    // ...
};
ipv6_module.send(ipv6_header, icmpv6_packet)?;
```

**接收流程：**
1. IPv6 模块解析 IPv6 头部，发现 Next Header = 58
2. IPv6 模块调用 ICMPv6 模块的 `handle_icmpv6()` 函数
3. 传递 IPv6 头部和 ICMPv6 负载

**发送流程：**
1. ICMPv6 模块构造 ICMPv6 报文
2. 调用 IPv6 模块的封装函数
3. IPv6 模块添加 IPv6 头部，查找路由，发送到下层

### 6.5 与 Ethernet 模块的交互

**间接交互（通过 IPv6 层）：**

ICMPv6 不直接与 Ethernet 模块交互，而是通过 IPv6 层间接访问：

```
ICMPv6 -> IPv6 -> Ethernet -> Queue
```

### 6.6 与 Engine/Processor 模块的交互

**依赖的 Engine 组件：**

| 组件 | 用途 |
|------|------|
| `engine/processor.rs` | 协议分发器 |
| `scheduler/scheduler.rs` | 数据包调度 |

**交互方式：**

```rust
// 在 Processor 中注册 ICMPv6 处理
// src/engine/processor.rs

impl Processor {
    pub fn process_ipv6(&mut self, packet: &mut Packet) -> Result<(), ProcessError> {
        // ... 解析 IPv6 头部

        match ipv6_header.next_header {
            58 => {
                // 分发到 ICMPv6 模块
                self.icmpv6_handler.handle(packet, &ipv6_header)?;
            }
            // ... 其他协议
        }
    }
}
```

### 6.7 与 Scheduler 模块的交互

**交互场景：**
- ICMPv6 报文通过 Scheduler 从 RxQ 获取
- ICMPv6 响应通过 Scheduler 发送到 TxQ

### 6.8 与 ARP 模块的交互

**ICMPv6 不需要 ARP：**

ICMPv6 使用 Neighbor Solicitation/Advertisement 代替 ARP 实现地址解析功能，因此不与 ARP 模块交互。

### 6.9 应用层交互（未来扩展）

**计划中的 API：**

```rust
impl Icmpv6Module {
    /// 发送 Echo Request (ping6)
    pub fn ping6(&mut self, dest: Ipv6Addr, count: u32) -> Result<Vec<PingResult>, Error> {
        // ...
    }

    /// 发送 Router Solicitation
    pub fn solicit_router(&mut self, iface: &str) -> Result<(), Error> {
        // ...
    }

    /// 注册邻居发现回调
    pub fn register_nd_callback<F>(&mut self, callback: F)
    where
        F: Fn(Ipv6Addr, MacAddr) + 'static
    {
        // ...
    }
}
```

### 6.10 模块初始化顺序

```
1. Common 模块 (packet, error, addr, queue)
   ↓
2. Interface 模块 (iface, manager)
   ↓
3. SystemContext 创建 (context 模块)
   ├── Arc<Mutex<InterfaceManager>>
   ├── Arc<Mutex<NeighborCache>>
   └── Arc<Mutex<EchoManager>>
   ↓
4. Ethernet 模块
   ↓
5. IPv6 模块 (依赖 SystemContext)
   ↓
6. ICMPv6 模块 (依赖 SystemContext)
   ↓
7. Processor/Engine (接收 SystemContext 引用)
   ↓
8. Scheduler (接收 SystemContext 引用)
```

### 6.11 数据流示例

**发送 Echo Request (ping6)：**

```
应用层
  │
  │ ping6("2001:db8::1")
  ↓
ICMPv6 模块
  │
  │ 构建 Echo Request (Type=128)
  │ 调用 IPv6 模块封装
  ↓
IPv6 模块
  │
  │ 添加 IPv6 头部 (Next Header=58)
  │ 查询路由 (选择出口接口)
  │ 通过邻居发现解析 MAC
  ↓
Ethernet 模块
  │
  │ 封装为 Ethernet 帧
  │
  ↓
Scheduler -> TxQ -> 输出
```

**接收 Router Advertisement，配置地址：**

```
Scheduler -> RxQ
  │
  ↓
Processor 解析 Ethernet -> IPv6 (Next Header=58)
  │
  ↓
ICMPv6 模块处理 Router Advertisement
  │
  │ 提取前缀信息
  │ 更新路由器列表
  │ 更新邻居缓存
  │ 触发地址自动配置
  │
  ↓
Interface 模块
  │
  │ 配置新的 IPv6 地址
  │ 更新接口参数
  │
  ↓
完成配置
```

---

## 7. 安全考虑

### 7.1 ICMPv6 攻击

**攻击方式：**
- **RA 欺骗**：伪造 Router Advertisement，劫持流量或设置错误的网络参数
- **NA 欺骗**：伪造 Neighbor Advertisement，截获或阻止通信
- **NS 泛洪**：大量发送 Neighbor Solicitation，消耗目标资源
- **ICMPv6 Flood**：大量发送任意类型的 ICMPv6 消息
- **DAD 攻击**：在重复地址检测期间响应 NA，阻止主机配置地址
- **RA 泛洪**：大量发送 Router Advertisement，导致主机配置错误

**攻击影响：**
- 中间人攻击
- 拒绝服务
- 网络配置错误
- 流量劫持

**防御措施：**
- **RA Guard**：在交换机上部署 RA Guard，过滤非法的 RA 消息
- **SEND (RFC 3971)**：使用 Secure Neighbor Discovery，通过 CGA (Cryptographically Generated Addresses) 和签名保护 NDP 消息
- **速率限制**：限制 ICMPv6 消息的发送和接收速率
- **严格验证**：验证 NDP 消息的来源和内容
- **组播过滤**：只接收必要的组播 NDP 消息

### 7.2 实现建议

1. **默认安全配置**：
   - 不接受来自非本地链路的 NDP 消息
   - 验证 NDP 消息的 Hop Limit = 255
   - 丢弃包含未知选项的 NDP 消息（可选）

2. **RA 过滤**：
   - 在生产环境中考虑禁用自动接受 RA
   - 使用 RA Guard 或类似机制

3. **速率限制**：
   - 对 NDP 消息实施速率限制
   - 限制 Echo Request/Reply 的响应速率

4. **SEND 支持**（可选）：
   - 实现 SEND (Secure Neighbor Discovery) 以增强安全性

5. **日志记录**：
   - 记录可疑的 NDP 活动
   - 监控异常的 ICMPv6 消息模式

---

## 8. 配置参数

```rust
/// ICMPv6 配置参数
#[derive(Debug, Clone)]
pub struct Icmpv6Config {
    // ========== 基本功能 ==========
    /// 是否响应 Echo Request (ping6)
    pub enable_echo_reply: bool,  // 默认: true

    /// Echo 请求超时时间
    pub echo_timeout: Duration,   // 默认: 1秒

    /// 最大待处理 Echo 请求数量
    pub max_pending_echoes: usize, // 默认: 100

    // ========== 邻居发现 ==========
    /// 是否接受 Router Advertisement
    pub accept_router_advertisements: bool,  // 默认: true (可配置为 false)

    /// 是否发送 Router Solicitation
    pub send_router_solicitation: bool,  // 默认: true

    /// Router Solicitation 延迟（秒）
    pub router_solicitation_delay: u32,  // 默认: 0-1 秒随机

    /// 最大 Router Solicitation 重传次数
    pub max_rs_retransmissions: u32,  // 默认: 3

    /// 邻居缓存最大条目数
    pub max_neighbor_cache_entries: usize,  // 默认: 256

    /// 默认可达时间（毫秒）
    pub default_reachable_time: u32,  // 默认: 30000 (30秒)

    /// 默认重传定时器（毫秒）
    pub default_retrans_timer: u32,  // 默认: 1000 (1秒)

    /// 是否启用重复地址检测 (DAD)
    pub enable_dad: bool,  // 默认: true

    /// DAD 传输次数
    pub dad_transmits: u32,  // 默认: 1

    /// DAD 超时时间（秒）
    pub dad_timeout: u32,  // 默认: 1

    // ========== 安全 ==========
    /// 是否接受 Redirect 消息
    pub accept_redirects: bool,    // 默认: false (安全考虑)

    /// 是否验证 NDP 消息的 Hop Limit = 255
    pub verify_hop_limit: bool,  // 默认: true

    /// NDP 消息速率限制（每秒）
    pub ndp_rate_limit: u32,     // 默认: 10

    /// 是否丢弃包含未知选项的 NDP 消息
    pub drop_unknown_options: bool,  // 默认: false

    // ========== PMTU ==========
    /// 是否启用路径 MTU 发现
    pub enable_pmtu_discovery: bool,  // 默认: true

    /// PMTU 缓存超时时间（分钟）
    pub pmtu_cache_timeout: u32,  // 默认: 10

    // ========== MLD ==========
    /// 是否启用 MLD (Multicast Listener Discovery)
    pub enable_mld: bool,  // 默认: false (暂不实现)

    /// MLD 版本 (1 或 2)
    pub mld_version: u32,  // 默认: 2

    // ========== 速率限制 ==========
    /// ICMPv6 Error 消息发送速率限制（每秒）
    pub error_rate_limit: u32,     // 默认: 10
}

impl Default for Icmpv6Config {
    fn default() -> Self {
        Self {
            enable_echo_reply: true,
            echo_timeout: Duration::from_secs(1),
            max_pending_echoes: 100,

            accept_router_advertisements: true,
            send_router_solicitation: true,
            router_solicitation_delay: 1,
            max_rs_retransmissions: 3,
            max_neighbor_cache_entries: 256,
            default_reachable_time: 30000,
            default_retrans_timer: 1000,
            enable_dad: true,
            dad_transmits: 1,
            dad_timeout: 1,

            accept_redirects: false,
            verify_hop_limit: true,
            ndp_rate_limit: 10,
            drop_unknown_options: false,

            enable_pmtu_discovery: true,
            pmtu_cache_timeout: 10,

            enable_mld: false,
            mld_version: 2,

            error_rate_limit: 10,
        }
    }
}
```

---

## 9. 测试场景

### 9.1 基本功能测试

1. **Echo Request/Reply 测试**
   - 发送 Echo Request (Type 128)，验证收到 Echo Reply (Type 129)
   - 验证 Identifier 和 Sequence Number 匹配
   - 验证负载数据完整返回
   - 测试不同长度的 payload

2. **邻居发现 - 地址解析测试**
   - 发送 Neighbor Solicitation，验证收到 Neighbor Advertisement
   - 验证邻居缓存正确更新
   - 测试 INCOMPLETE → REACHABLE 状态转换

3. **路由器发现测试**
   - 发送 Router Solicitation，验证收到 Router Advertisement
   - 验证前缀信息正确提取
   - 验证路由器列表更新

### 9.2 错误处理测试

1. **Destination Unreachable 测试**
   - 发送到不可达网络，验证收到 Type 1
   - 验证 Code 字段正确

2. **Packet Too Big 测试**
   - 发送超过 MTU 的包，验证收到 Type 2
   - 验证 MTU 字段正确
   - 验证 PMTU 缓存更新

3. **Time Exceeded 测试**
   - Hop Limit=1 的数据报，验证收到 Type 3, Code 0
   - 验证消息中包含原始数据报头部

4. **Parameter Problem 测试**
   - 发送格式错误的 IPv6 扩展头部，验证收到 Type 4
   - 验证 Pointer 指向错误位置

### 9.3 边界情况测试

1. **最小/最大 ICMPv6 包**
   - 8 字节的最小 ICMPv6 头部
   - MTU 限制下的最大 ICMPv6 包

2. **校验和测试**
   - 错误的校验和，验证包被丢弃
   - 验证伪头部正确用于校验和计算

3. **序列号回绕**
   - Sequence Number 从 65535 到 0 的过渡

4. **IPv6 地址边界**
   - 链路本地地址 (fe80::/10)
   - 组播地址
   - 任意播地址

### 9.4 异常情况测试

1. **ICMPv6 Error 循环防护**
   - 确保不会为 ICMPv6 Error 消息发送另一个 ICMPv6 Error

2. **未知的 Type/Code**
   - 收到未知类型时，应该静默丢弃

3. **格式错误的 ICMPv6**
   - 长度不足的 ICMPv6 包
   - 非法字段值的处理

4. **速率限制测试**
   - 快速连续发送 Echo Request，验证响应速率受限
   - 验证 NDP 消息速率限制

5. **安全测试**
   - 伪造的 Router Advertisement (Hop Limit != 255)
   - 伪造的 Neighbor Advertisement
   - RA 泛洪攻击防御

### 9.5 邻居发现特定测试

1. **重复地址检测 (DAD)**
   - 配置新地址时发送 NS
   - 模拟地址冲突场景
   - 验证冲突处理逻辑

2. **邻居可达性检测**
   - REACHABLE → STALE 状态转换
   - STALE → DELAY → PROBE → REACHABLE 转换
   - PROBE 失败后删除条目

3. **前缀过期**
   - 前缀 Valid Lifetime 过期后删除地址
   - Preferred Lifetime 过期后标记地址为废弃

---

## 10. 参考资料

1. **RFC 4443** - Internet Control Message Protocol (ICMPv6) for the Internet Protocol Version 6 (IPv6) Specification
2. **RFC 4861** - Neighbor Discovery for IP version 6 (IPv6)
3. **RFC 4862** - IPv6 Stateless Address Autoconfiguration
4. **RFC 4291** - IPv6 Address Architecture
5. **RFC 8200** - Internet Protocol, Version 6 (IPv6) Specification
6. **RFC 2710** - Multicast Listener Discovery (MLD) for IPv6
7. **RFC 3810** - Multicast Listener Discovery Version 2 (MLDv2) for IPv6
8. **RFC 8106** - IPv6 Router Advertisement Options for DNS Configuration
9. **RFC 3971** - Secure Neighbor Discovery (SEND)
10. **RFC 4890** - Recommendations for Filtering ICMPv6 Messages in Firewalls
