# Transmission Control Protocol (TCP) 详细设计文档

## 1. 协议概述

### 1.1 背景与历史

**协议全称和定义：**
- **全称**：Transmission Control Protocol（传输控制协议）
- **层级位置**：传输层（Transport Layer，OSI 第4层）
- **核心功能**：在不可靠的 IP 网络上提供可靠的、面向连接的字节流传输服务

**为什么需要 TCP 协议？**

IP 协议提供的是"尽力而为"（best-effort）的数据报服务，存在以下问题：
- **无连接**：数据包可能丢失、重复、乱序到达
- **无可靠性保证**：不确认数据是否送达
- **无流量控制**：发送方可能淹没接收方
- **无拥塞控制**：可能导致网络崩溃

TCP 协议通过以下机制解决上述问题：
- 建立和维护连接状态
- 序列号和确认应答机制
- 超时重传机制
- 滑动窗口流量控制
- 拥塞控制算法

**历史背景：**
- **RFC 761** (1980年1月) - TCP 最早描述
- **RFC 793** (1981年9月) - TCP 基础规范，由 Jon Postel 编写
- **RFC 1122** (1989年) - 修正 RFC 793 中的错误，明确实现要求
- **RFC 9293** (2022年8月) - 当前标准，取代 RFC 793

**重要补充 RFC：**
- **RFC 1323 / RFC 7323** - 高性能扩展（窗口缩放、时间戳、PAWS）
- **RFC 2018** - 选择性确认（SACK）
- **RFC 5681** - 拥塞控制（慢启动、拥塞避免、快重传、快恢复）
- **RFC 879** - 最大分段大小（MSS）
- **RFC 2988** - 重传超时计算
- **RFC 3168** - 显式拥塞通知（ECN）

### 1.2 设计原理

TCP 采用**面向连接的、可靠的字节流传输**模型，核心设计思想：

```
                    TCP 连接建立与数据传输

      Client                              Server
         |                                  |
         |  1. SYN                          |  LISTEN
         |--------------------------------->|
         |                                  | SYN-RCVD
         |  2. SYN-ACK                      |
         |<---------------------------------|
         |  ESTAB                           |
         |  3. ACK                          |
         |--------------------------------->|
         |                                  | ESTABLISHED
         |                                  |
         |  4. Data [Seq=1000, Len=100]     |
         |--------------------------------->|
         |                                  | 5. ACK [Ack=1100]
         |<---------------------------------|
         |                                  |
         |  6. FIN                          |
         |--------------------------------->|
         |                                  | CLOSE-WAIT
         |  7. FIN-ACK                      |
         |<---------------------------------|
         |  FIN-WAIT-2                      |
         |                                  | 8. FIN
         |<---------------------------------|
         |  9. ACK                          |
         |--------------------------------->|
         |                                  |
         |  CLOSED                          |  CLOSED
```

**关键特点：**

1. **面向连接**：通信前必须三次握手建立连接，通信后四次挥手断开连接

2. **可靠传输**：
   - 每个字节都有序列号
   - 接收方必须确认接收
   - 超时未确认则重传

3. **全双工通信**：同一连接可以双向传输数据，双方各自维护独立的序列号空间

4. **流量控制**：使用滑动窗口机制，接收方通过窗口大小告知发送方可接收的数据量

5. **拥塞控制**：通过慢启动、拥塞避免等算法动态调整发送速率，避免网络拥塞

6. **字节流服务**：不保留消息边界，应用层需要自行处理消息分界

---

## 2. 报文格式

### 2.1 报文结构

```
 0                   1                   2                   3
 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|          Source Port          |       Destination Port        |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|                        Sequence Number                        |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|                    Acknowledgment Number                      |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|  Data |           |U|A|P|R|S|F|                               |
| Offset| Reserved  |R|C|S|S|Y|I|            Window             |
|       |           |G|K|H|T|N|N|                               |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|           Checksum            |         Urgent Pointer        |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|                    Options                    |    Padding    |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|                             Data                              |
|                              ...                               |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
```

### 2.2 字段说明

| 字段 | 大小 | 说明 | 常见值 |
|------|------|------|--------|
| Source Port | 2 字节 | 源端口号 | 1024-65535（客户端），<1024（服务器） |
| Destination Port | 2 字节 | 目标端口号 | 80(HTTP), 443(HTTPS), 22(SSH) |
| Sequence Number | 4 字节 | 数据段的序列号（第一个字节的序号） | 初始随机值(ISN) |
| Acknowledgment Number | 4 字节 | 期望接收的下一个字节的序号 | 累计确认 |
| Data Offset | 4 位 | TCP 头部长度（以 4 字节为单位） | 5（20 字节基本头）到 15（60 字节最大头） |
| Reserved | 3 位 | 保留字段，必须为 0 | 0 |
| URG | 1 位 | 紧急指针有效 | 0/1 |
| ACK | 1 位 | 确认号有效 | 0/1 |
| PSH | 1 位 | 推送数据到应用层 | 0/1 |
| RST | 1 位 | 重置连接 | 0/1 |
| SYN | 1 位 | 同步序列号（建立连接） | 0/1 |
| FIN | 1 位 | 结束连接 | 0/1 |
| Window | 2 字节 | 接收窗口大小（字节数） | 动态调整 |
| Checksum | 2 字节 | 校验和（包含伪头部） | 计算值 |
| Urgent Pointer | 2 字节 | 紧急数据的偏移量（从序列号开始） | 紧急数据位置 |
| Options | 可变 | 选项字段 | MSS, WS, SACK permitted, Timestamps |
| Data | 可变 | 上层应用数据 | 应用层载荷 |

**最小/最大报文长度：**
- **最小**：20 字节（无选项、无数据）
- **最大**：65535 字节（受 IP 数据报限制），实际受 MSS 限制（通常 1460 字节）

### 2.3 TCP 伪头部（用于计算校验和）

```
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|          Source Address          |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|       Destination Address       |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|     Zero     |  Protocol  |    TCP Length    |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
```

### 2.4 常见 TCP 选项

| 选项 | 类型 | 长度 | 说明 |
|------|------|------|------|
| MSS | 2 | 4 | 最大分段大小 |
| Window Scale | 3 | 3 | 窗口缩放因子（RFC 7323） |
| SACK Permitted | 4 | 2 | 允许选择性确认 |
| SACK | 5 | 可变 | 选择性确认块 |
| Timestamps | 8 | 10 | 时间戳（RFC 7323） |
| NOP | 1 | 1 | 填充（对齐） |

### 2.5 封装格式

```
+-------------------+
|   应用层数据       |
+-------------------+
|    TCP 头部       |
+-------------------+
|    IP 头部        |      IPv4: Protocol = 6
+-------------------+      IPv6: Next Header = 6
|  以太网头 / VLAN  |
+-------------------+
```

---

## 3. 状态机设计

### 3.0 状态变量

TCP 连接维护的关键状态变量：

| 变量名称 | 类型 | 用途 | 初始值 |
|---------|------|------|--------|
| snd_una | u32 | 发送但未确认的最小序列号 | 初始 ISN |
| snd_nxt | u32 | 下一个要发送的序列号 | 初始 ISN |
| snd_wnd | u16 | 发送窗口大小（接收方通告） | 0 |
| snd_up | u32 | 紧急指针 | 0 |
| rcv_nxt | u32 | 期望接收的下一个序列号 | 初始 ISN |
| rcv_wnd | u16 | 接收窗口大小 | 配置值 |
| iss | u32 | 初始发送序列号 | 随机值 |
| irs | u32 | 初始接收序列号 | 从 SYN 获取 |
| srtt | u32 | 平滑往返时间 | 0 |
| rttvar | u32 | 往返时间方差 | 0 |
| rto | u32 | 重传超时时间 | RFC2988 初始值 |
| cwnd | u32 | 拥塞窗口大小 | 初始值（通常 10*MSS 或 1460） |
| ssthresh | u32 | 慢启动阈值 | 初始值（通常无限大） |

### 3.1 TCP 状态定义

```
                              +---------+
                              | CLOSED  |
                              +---------+
                                 |   |
                    应用层主动    |   |    应用层被动打开
                    打开 SYN    |   |    LISTEN
                                 |   |
                                 v   v
+---------+               +-----------+           +---------+
|  SYN    |               |  SYN-RCVD |           |  SYN    |
| SENT    |               +-----------+           | SENT    |
+---------+                    |     ^             +---------+
      |                   收到 SYN |     |   收到 SYN
      |                             |     |
      | 收到 SYN/ACK                |     | 收到 SYN
      |，发送 ACK                   |     |
      v                             |     |
+---------+                         |     v
|  ESTAB  |<------------------------+   +-----------+
+---------+                              |  ESTAB    |
      |                                  +-----------+
      |    应用层关闭                     |    应用层关闭
      |    发送 FIN                      |    收到 FIN
      v                                  v
+---------+                         +-----------+
|  FIN-   |                         | CLOSE-WAIT|
| WAIT-1  |                         +-----------+
+---------+                              |
      |                                   |
      | 收到 ACK                          | 应用层关闭
      v                                   | 发送 FIN
+---------+                              v
|  FIN-   |                         +-----------+
| WAIT-2  |<------------------------|  LAST-ACK |
+---------+    收到 FIN              +-----------+
      |                                   |
      | 收到 FIN                          | 收到 ACK
      | 发送 ACK                          |
      v                                   v
+---------+                          +---------+
| TIME-   |                          |  CLOSED |
| WAIT    |                          +---------+
+---------+
      |
      | 2MSL 超时
      v
+---------+
|  CLOSED |
+---------+
```

### 3.2 状态转换详解

#### 3.2.1 CLOSED（关闭状态）

**描述：** 连接不存在或已完全关闭的初始状态

**进入条件：**
- 系统初始化
- 连接关闭完成后

**行为：** 无活动，等待应用层调用

**转换条件：**
- 应用层主动打开（active open）→ LISTEN 或 SYN_SENT
- 应用层被动打开（passive open）→ LISTEN

#### 3.2.2 LISTEN（监听状态）

**描述：** 等待远程 TCP 的连接请求

**进入条件：** 应用层调用被动打开（passive open）

**行为：**
- 监听指定端口
- 等待接收 SYN 报文

**转换条件：**
- 收到 SYN → 发送 SYN-ACK → SYN-RCVD
- 应用层发送数据（同时打开）→ SYN-SENT

#### 3.2.3 SYN-SENT（同步已发送）

**描述：** 已发送 SYN，等待连接确认

**进入条件：** 应用层调用主动打开（active open），发送 SYN

**行为：**
- 等待 SYN-ACK 响应
- 启动重传定时器

**转换条件：**
- 收到 SYN-ACK → 发送 ACK → ESTABLISHED
- 收到 SYN（同时打开）→ 发送 SYN-ACK → SYN-RCVD

#### 3.2.4 SYN-RCVD（同步已接收）

**描述：** 已收到并发送 SYN，等待确认

**进入条件：** 收到 SYN，发送 SYN-ACK

**行为：**
- 等待 ACK 确认
- 启动重传定时器

**转换条件：**
- 收到 ACK → ESTABLISHED
- 超时 → CLOSED 或重传 SYN-ACK

#### 3.2.5 ESTABLISHED（已建立连接）

**描述：** 连接已建立，可以进行双向数据传输

**进入条件：** 三次握手完成

**行为：**
- 发送和接收数据
- 维拥塞控制和流量控制
- 响应 ACK、FIN 等控制报文

**转换条件：**
- 应用层关闭（发送 FIN）→ FIN-WAIT-1
- 收到 FIN → 发送 ACK → CLOSE-WAIT

#### 3.2.6 FIN-WAIT-1（结束等待1）

**描述：** 应用层已关闭，发送 FIN，等待 ACK 或远程 FIN

**进入条件：** 应用层关闭，发送 FIN

**行为：**
- 等待对方确认 FIN
- 仍可接收数据

**转换条件：**
- 收到 ACK → FIN-WAIT-2
- 收到 FIN（同时关闭）→ 发送 ACK → CLOSING

#### 3.2.7 FIN-WAIT-2（结束等待2）

**描述：** 已发送 FIN 并收到 ACK，等待远程关闭

**进入条件：** 收到对 FIN-WAIT-1 中 FIN 的 ACK

**行为：**
- 半关闭状态，可接收数据
- 等待远程发送 FIN

**转换条件：**
- 收到 FIN → 发送 ACK → TIME-WAIT

#### 3.2.8 CLOSING（正在关闭）

**描述：** 双方同时关闭，等待对方确认

**进入条件：** FIN-WAIT-1 状态下收到 FIN（同时关闭）

**行为：** 等待对 ACK 的确认

**转换条件：**
- 收到 ACK → TIME-WAIT

#### 3.2.9 TIME-WAIT（时间等待）

**描述：** 等待足够时间以确保远程 TCP 收到终止请求的确认

**进入条件：** 收到 FIN 并发送 ACK（主动关闭方）

**行为：**
- 等待 2MSL（Maximum Segment Lifetime，通常 60 秒）
- 确保最后的 ACK 能被重传（如果对方未收到）

**转换条件：**
- 2MSL 超时 → CLOSED

#### 3.2.10 CLOSE-WAIT（关闭等待）

**描述：** 收到远程关闭请求，等待应用层关闭

**进入条件：** ESTABLISHED 状态下收到 FIN

**行为：**
- 通知应用层远程关闭
- 可继续发送数据
- 等待应用层关闭

**转换条件：**
- 应用层关闭（发送 FIN）→ LAST-ACK

#### 3.2.11 LAST-ACK（最后确认）

**描述：** 等待对关闭请求的确认

**进入条件：** 应用层关闭，发送 FIN

**行为：** 等待远程对 FIN 的 ACK

**转换条件：**
- 收到 ACK → CLOSED

---

## 4. 报文处理逻辑

### 4.0 定时器

TCP 协议使用的定时器：

| 定时器名称 | 启动条件 | 超时时间 | 超时动作 |
|-----------|---------|---------|---------|
| 重传定时器 (RTO) | 发送数据时启动 | 动态计算（RFC2988） | 重传最早未确认的段 |
| 坚持定时器 (Persist) | 接收窗口为 0 时启动 | 估算 RTT | 探测窗口更新（零窗口探查） |
| 保活定时器 (Keepalive) | 连接空闲超时（可选） | 可配置（通常 2 小时） | 发送保活探测 |
| TIME_WAIT 定时器 | 进入 TIME-WAIT 状态 | 2MSL（通常 60 秒） | 关闭连接 → CLOSED |
| 延迟 ACK 定时器 | 收到数据需延迟确认时 | 通常 200ms | 发送 ACK |

### 4.1 连接建立（三次握手）

**处理流程：**

```
客户端                              服务器
  |                                   |
  |  1. SYN [Seq=Client_ISN]          |  LISTEN
  |--------------------------------->|
  |                                   | SYN-RCVD
  |  2. SYN-ACK [Seq=Server_ISN,      |
  |              Ack=Client_ISN+1]    |
  |<---------------------------------|
  |  ESTABLISHED                      |
  |  3. ACK [Seq=Client_ISN+1,        |
  |            Ack=Server_ISN+1]      |
  |--------------------------------->|
  |                                   | ESTABLISHED
```

#### 4.1.1 客户端发送 SYN

**处理流程：**

1. **生成参数：**
   - 生成初始序列号 (iss)：安全的随机 ISN
   - 设置选项：MSS, Window Scale, SACK Permitted, Timestamps

2. **处理步骤：**
   - 创建 SYN 报文
   - 计算校验和
   - 发送 SYN
   - 状态：CLOSED → SYN-SENT

3. **资源更新：**
   - 状态变量：iss → 生成值, snd_una → iss, snd_nxt → iss + 1
   - 定时器：启动重传定时器 (RTO)

4. **响应动作：**
   - 等待 SYN-ACK 响应

#### 4.1.2 服务器收到 SYN，发送 SYN-ACK

**处理流程：**

1. **提取信息：**
   - Source Port → 保存客户端端口
   - Sequence Number → irs（客户端 ISN）
   - Options → 保存 MSS, WS, SACK, Timestamps

2. **处理步骤：**
   - 验证 SYN 标志位
   - 创建 TCB（传输控制块）
   - 生成服务器 ISN (iss)
   - 构建 SYN-ACK 报文
   - 计算校验和
   - 发送 SYN-ACK

3. **资源更新：**
   - 表项：TCB [添加] 新建连接记录
   - 状态变量：irs → 接收值, iss → 生成值, snd_nxt → iss + 1, rcv_nxt → irs + 1
   - 定时器：启动重传定时器
   - 状态：LISTEN → SYN-RCVD

4. **响应动作：**
   - 发送 SYN-ACK [Seq=iss, Ack=irs+1]

#### 4.1.3 客户端收到 SYN-ACK，发送 ACK

**处理流程：**

1. **提取信息：**
   - Sequence Number → 服务器 ISN
   - Acknowledgment Number → 确认客户端 ISN
   - Options → 保存服务器选项

2. **处理步骤：**
   - 验证 ACK 位和确认号
   - 更新状态变量
   - 发送最终 ACK

3. **资源更新：**
   - 状态变量：snd_una → iss + 1, snd_nxt → iss + 1, rcv_nxt → 服务器 ISN + 1
   - 定时器：取消重传定时器
   - 状态：SYN-SENT → ESTABLISHED

4. **响应动作：**
   - 发送 ACK [Seq=iss+1, Ack=服务器 ISN+1]
   - 通知应用层连接已建立

#### 4.1.4 服务器收到 ACK

**处理流程：**

1. **提取信息：**
   - Acknowledgment Number → 确认服务器 ISN

2. **处理步骤：**
   - 验证确认号是否等于 iss + 1

3. **资源更新：**
   - 状态变量：snd_una → iss + 1
   - 定时器：取消重传定时器
   - 状态：SYN-RCVD → ESTABLISHED

4. **响应动作：**
   - 通知应用层连接已建立

### 4.2 连接终止（四次挥手）

**处理流程：**

```
客户端                              服务器
  |  ESTABLISHED                     |  ESTABLISHED
  |                                   |
  |  1. FIN                           |
  |--------------------------------->|  CLOSE-WAIT
  |  FIN-WAIT-1                       |
  |  2. ACK                           |
  |<---------------------------------|
  |  FIN-WAIT-2                       |
  |                                   |  3. FIN
  |<---------------------------------|
  |  TIME-WAIT                        |  LAST-ACK
  |  4. ACK                           |
  |--------------------------------->|  CLOSED
  |                                   |
  |  (等待 2MSL)                      |
  |  CLOSED                           |
```

#### 4.2.1 主动关闭方发送 FIN

**处理流程：**

1. **处理步骤：**
   - 应用层调用关闭
   - 发送 FIN 报文

2. **资源更新：**
   - 状态变量：snd_nxt → snd_nxt + 1
   - 定时器：启动重传定时器
   - 状态：ESTABLISHED → FIN-WAIT-1

3. **响应动作：**
   - 发送 FIN

#### 4.2.2 被动关闭方收到 FIN，发送 ACK

**处理流程：**

1. **提取信息：**
   - FIN 标志位
   - Sequence Number

2. **处理步骤：**
   - 验证 FIN
   - 更新接收序列号
   - 通知应用层对方关闭

3. **资源更新：**
   - 状态变量：rcv_nxt → rcv_nxt + 1
   - 状态：ESTABLISHED → CLOSE-WAIT

4. **响应动作：**
   - 发送 ACK [Ack=rcv_nxt]
   - 等待应用层关闭

#### 4.2.3 被动关闭方应用层关闭，发送 FIN

**处理流程：**

1. **处理步骤：**
   - 应用层关闭连接
   - 发送 FIN 报文

2. **资源更新：**
   - 状态变量：snd_nxt → snd_nxt + 1
   - 定时器：启动重传定时器
   - 状态：CLOSE-WAIT → LAST-ACK

3. **响应动作：**
   - 发送 FIN

#### 4.2.4 主动关闭方收到 ACK，等待 FIN

**处理流程：**

1. **提取信息：**
   - Acknowledgment Number

2. **处理步骤：**
   - 验证 ACK 确认了之前的 FIN

3. **资源更新：**
   - 状态变量：snd_una → snd_una + 1
   - 定时器：取消 FIN-WAIT-1 的重传定时器
   - 状态：FIN-WAIT-1 → FIN-WAIT-2

4. **响应动作：**
   - 等待对方的 FIN

#### 4.2.5 主动关闭方收到 FIN，发送 ACK

**处理流程：**

1. **提取信息：**
   - FIN 标志位
   - Sequence Number

2. **处理步骤：**
   - 验证 FIN
   - 发送 ACK

3. **资源更新：**
   - 状态变量：rcv_nxt → rcv_nxt + 1
   - 定时器：启动 TIME-WAIT 定时器 (2MSL)
   - 状态：FIN-WAIT-2 → TIME-WAIT

4. **响应动作：**
   - 发送 ACK

#### 4.2.6 TIME-WAIT 等待完成

**处理流程：**

1. **处理步骤：**
   - 等待 2MSL（确保最后的 ACK 能被重传）

2. **资源更新：**
   - 定时器：TIME-WAIT 定时器超时
   - 状态：TIME-WAIT → CLOSED

3. **响应动作：**
   - 释放 TCB 和资源

#### 4.2.7 被动关闭方收到 ACK

**处理流程：**

1. **提取信息：**
   - Acknowledgment Number

2. **处理步骤：**
   - 验证 ACK 确认了 FIN

3. **资源更新：**
   - 状态变量：snd_una → snd_una + 1
   - 定时器：取消重传定时器
   - 状态：LAST-ACK → CLOSED

4. **响应动作：**
   - 释放 TCB 和资源

### 4.3 数据传输

#### 4.3.1 发送数据

**处理流程：**

1. **准备数据：**
   - 从应用层获取数据
   - 分段（根据 MSS 和发送窗口）

2. **处理步骤：**
   - 分配序列号
   - 构建 TCP 报文头
   - 计算校验和
   - 将数据加入发送队列
   - 发送数据报文

3. **资源更新：**
   - 状态变量：snd_nxt → snd_nxt + 数据长度
   - 表项：重传队列 [添加] 未确认的数据段
   - 定时器：启动/重置重传定时器

4. **响应动作：**
   - 等待 ACK 确认

#### 4.3.2 接收数据并发送 ACK

**处理流程：**

1. **提取信息：**
   - Sequence Number → 数据起始序列号
   - Data Offset → TCP 头部长度
   - 数据载荷 → 应用数据

2. **处理步骤：**
   - 验证序列号是否在接收窗口内
   - 处理乱序到达（缓冲区）
   - 重组数据流（处理重复数据）
   - 将有序数据传递给应用层

3. **资源更新：**
   - 状态变量：rcv_nxt → rcv_nxt + 连续数据长度
   - 定时器：可能启动延迟 ACK 定时器

4. **响应动作：**
   - 发送 ACK [Ack=rcv_nxt, Win=rcv_wnd]
   - 可能使用延迟 ACK（等待 200ms 或有数据发送时）

#### 4.3.3 收到 ACK

**处理流程：**

1. **提取信息：**
   - Acknowledgment Number → 确认号
   - Window → 对方接收窗口大小
   - SACK 选项 → 选择性确认信息（如果支持）

2. **处理步骤：**
   - 更新 snd_una（确认已接收的数据）
   - 从重传队列移除已确认的数据
   - 更新发送窗口 (snd_wnd)
   - 如果使用 SACK，标记具体丢失的段
   - 可能触发快重传（3 个重复 ACK）

3. **资源更新：**
   - 状态变量：snd_una → Ack 号, snd_wnd → Window 值
   - 表项：重传队列 [删除] 已确认的段
   - 定时器：取消重传定时器（如果所有数据已确认）

4. **响应动作：**
   - 如果有新数据可发送，继续发送
   - 如果检测到丢包（3 个重复 ACK），触发快重传

### 4.4 重传机制

#### 4.4.1 超时重传

**处理流程：**

1. **触发条件：**
   - 重传定时器 (RTO) 超时
   - 未收到对发送数据的确认

2. **处理步骤：**
   - 重传 snd_una 指向的数据段
   - 指数退避：RTO = RTO * 2
   - 可能触发慢启动（重新开始拥塞控制）

3. **资源更新：**
   - 状态变量：ssthresh → cwnd / 2, cwnd → 1 MSS（慢启动）
   - 定时器：重置重传定时器（新的 RTO）

4. **响应动作：**
   - 重传最早未确认的段

#### 4.4.2 快重传（Fast Retransmit）

**处理流程：**

1. **触发条件：**
   - 收到 3 个对同一序列号的重复 ACK

2. **处理步骤：**
   - 立即重传丢失的段（不等 RTO 超时）
   - 不降低拥塞窗口

3. **资源更新：**
   - 状态变量：ssthresh → cwnd / 2（可能）
   - 表项：标记丢失的段

4. **响应动作：**
   - 重传丢失的段
   - 进入快恢复状态

### 4.5 流量控制

**处理流程：**

1. **滑动窗口机制：**
   - 接收方通过 TCP 头中的 Window 字段通告接收窗口大小
   - 发送方确保 snd_nxt - snd_una <= snd_wnd
   - 接收方根据可用缓冲区动态调整窗口大小

2. **零窗口处理：**
   - 接收方窗口为 0 时，发送 Window = 0 的 ACK
   - 发送方启动坚持定时器（Persist Timer）
   - 定时发送 1 字节的零窗口探查
   - 接收方响应探查，通告新的窗口大小

### 4.6 拥塞控制

**处理流程：**

```
                    拥塞窗口变化示意

     cwnd
       ^
       |     / \           (慢启动)
       |    /   \          指数增长
       |   /     \
       |  /       \_______ (拥塞避免)
       | /                 线性增长
       |/_________________________> 时间
              |
              v
         检测到丢包
         cwnd → ssthresh
```

#### 4.6.1 慢启动（Slow Start）

**处理流程：**

1. **启动条件：**
   - 连接建立时
   - 超时重传后

2. **处理步骤：**
   - 每收到一个 ACK，cwnd 增加 1 MSS
   - 指数增长：cwnd = 2^k * MSS（k 为 RTT 轮数）
   - 直到 cwnd >= ssthresh

3. **资源更新：**
   - 状态变量：cwnd → cwnd + MSS（每个 ACK）

4. **响应动作：**
   - 增加发送速率

#### 4.6.2 拥塞避免（Congestion Avoidance）

**处理流程：**

1. **进入条件：**
   - cwnd >= ssthresh

2. **处理步骤：**
   - 每个 RTT，cwnd 增加 1 MSS
   - 线性增长，避免快速导致拥塞

3. **资源更新：**
   - 状态变量：cwnd → cwnd + (MSS * MSS / cwnd)（每个 ACK）

4. **响应动作：**
   - 稳定增加发送速率

#### 4.6.3 快恢复（Fast Recovery）

**处理流程：**

1. **触发条件：**
   - 收到 3 个重复 ACK（快重传触发）

2. **处理步骤：**
   - ssthresh = cwnd / 2
   - cwnd = ssthresh + 3 * MSS（计入 3 个重复 ACK）
   - 每收到一个重复 ACK，cwnd += MSS
   - 收到新的 ACK（确认新数据），cwnd = ssthresh（进入拥塞避免）

3. **资源更新：**
   - 状态变量：ssthresh → cwnd / 2, cwnd → ssthresh + 3*MSS

4. **响应动作：**
   - 快重传丢失的段
   - 不进入慢启动

### 4.7 连接重置（RST）

#### 4.7.1 发送 RST

**处理流程：**

1. **触发条件：**
   - 连接不存在（收到不存在的连接的报文）
   - 连接异常（应用层请求重置）
   - 收到非法报文（序列号错误等）

2. **处理步骤：**
   - 构建 RST 报文
   - 序列号设置为期望值（如果可能）
   - 发送 RST

3. **资源更新：**
   - 状态：当前状态 → CLOSED
   - 表项：TCB [删除]

4. **响应动作：**
   - 释放连接资源
   - 通知应用层连接重置

#### 4.7.2 收到 RST

**处理流程：**

1. **提取信息：**
   - RST 标志位
   - Sequence Number

2. **处理步骤：**
   - 验证 RST 的合法性
   - 立即终止连接

3. **资源更新：**
   - 状态：当前状态 → CLOSED
   - 表项：TCB [删除]

4. **响应动作：**
   - 释放所有资源
   - 通知应用层连接被重置

---

## 5. 核心数据结构

### 5.0 表项/缓存

TCP 协议维护的表项和缓存：

| 资源名称 | 用途 | 最大容量 | 淘汰策略 |
|---------|------|---------|---------|
| TCB (Transmission Control Block) | 存储 TCP 连接状态 | 系统限制 | 连接关闭后删除 |
| 重传队列 | 存储未确认的数据段 | 受拥塞窗口限制 | 确认后删除 |
| 接收缓冲区 | 缓存接收但未交付应用的数据 | 接收窗口大小 | 应用层读取后释放 |
| 乱序队列 | 缓存乱序到达的数据段 | 接收窗口大小 | 重组后释放 |

#### 5.0.1 TCB (Transmission Control Block)

**用途：** 存储每个 TCP 连接的完整状态信息

**关键操作：**
- **查询**：(本地 IP, 本地端口, 远程 IP, 远程端口) → TCB
- **添加**：三次握手完成时创建
- **更新**：收到数据/ACK 时更新序列号、窗口等
- **删除**：连接关闭后释放

### 5.1 报文结构

```rust
/// TCP 报文头部
#[repr(C, packed)]
pub struct TcpHeader {
    /// 源端口号
    pub source_port: u16,
    /// 目标端口号
    pub destination_port: u16,
    /// 序列号
    pub sequence_number: u32,
    /// 确认号
    pub acknowledgment_number: u32,
    /// 数据偏移（高 4 位）+ 保留（高 4 位）+ 标志位（低 8 位）
    pub data_offset_and_flags: u16,
    /// 窗口大小
    pub window_size: u16,
    /// 校验和
    pub checksum: u16,
    /// 紧急指针
    pub urgent_pointer: u16,
}

impl TcpHeader {
    /// 获取数据偏移（以 4 字节为单位）
    pub const fn data_offset(&self) -> u8 {
        ((self.data_offset_and_flags >> 12) & 0x0F) as u8
    }

    /// 设置数据偏移
    pub fn set_data_offset(&mut self, offset: u8) {
        let mask = 0x0FFF;
        self.data_offset_and_flags = (self.data_offset_and_flags & mask) | ((offset as u16) << 12);
    }

    /// 获取标志位
    pub const fn flags(&self) -> u8 {
        (self.data_offset_and_flags & 0xFF) as u8
    }

    /// 设置标志位
    pub fn set_flags(&mut self, flags: u8) {
        let mask = 0xFF00;
        self.data_offset_and_flags = (self.data_offset_and_flags & mask) | (flags as u16);
    }

    /// 检查 FIN 标志
    pub const fn is_fin(&self) -> bool {
        self.flags() & 0x01 != 0
    }

    /// 检查 SYN 标志
    pub const fn is_syn(&self) -> bool {
        self.flags() & 0x02 != 0
    }

    /// 检查 RST 标志
    pub const fn is_rst(&self) -> bool {
        self.flags() & 0x04 != 0
    }

    /// 检查 PSH 标志
    pub const fn is_psh(&self) -> bool {
        self.flags() & 0x08 != 0
    }

    /// 检查 ACK 标志
    pub const fn is_ack(&self) -> bool {
        self.flags() & 0x10 != 0
    }

    /// 检查 URG 标志
    pub const fn is_urg(&self) -> bool {
        self.flags() & 0x20 != 0
    }

    /// 检查 ECE 标志
    pub const fn is_ece(&self) -> bool {
        self.flags() & 0x40 != 0
    }

    /// 检查 CWR 标志
    pub const fn is_cwr(&self) -> bool {
        self.flags() & 0x80 != 0
    }
}

/// TCP 连接四元组（标识符）
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TcpConnectionId {
    /// 本地 IP 地址
    pub local_ip: Ipv4Addr,
    /// 本地端口
    pub local_port: u16,
    /// 远程 IP 地址
    pub remote_ip: Ipv4Addr,
    /// 远程端口
    pub remote_port: u16,
}

/// TCP 连接状态
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TcpState {
    /// 关闭状态
    Closed = 0,
    /// 监听状态
    Listen = 1,
    /// 同步已发送
    SynSent = 2,
    /// 同步已接收
    SynReceived = 3,
    /// 已建立连接
    Established = 4,
    /// 结束等待1
    FinWait1 = 5,
    /// 结束等待2
    FinWait2 = 6,
    /// 正在关闭
    Closing = 7,
    /// 时间等待
    TimeWait = 8,
    /// 关闭等待
    CloseWait = 9,
    /// 最后确认
    LastAck = 10,
}

/// TCP 传输控制块（TCB）
pub struct Tcb {
    /// 连接标识符
    pub id: TcpConnectionId,
    /// 连接状态
    pub state: TcpState,

    // 发送状态变量
    /// 发送但未确认的最小序列号
    pub snd_una: u32,
    /// 下一个要发送的序列号
    pub snd_nxt: u32,
    /// 发送窗口大小
    pub snd_wnd: u16,
    /// 紧急指针
    pub snd_up: u32,
    /// 初始发送序列号
    pub iss: u32,

    // 接收状态变量
    /// 期望接收的下一个序列号
    pub rcv_nxt: u32,
    /// 接收窗口大小
    pub rcv_wnd: u16,
    /// 初始接收序列号
    pub irs: u32,

    // 定时器和 RTT 估计
    /// 平滑往返时间
    pub srtt: u32,
    /// 往返时间方差
    pub rttvar: u32,
    /// 重传超时时间
    pub rto: u32,

    // 拥塞控制
    /// 拥塞窗口大小
    pub cwnd: u32,
    /// 慢启动阈值
    pub ssthresh: u32,

    // 选项
    /// 最大分段大小（MSS）
    pub mss: u16,
    /// 窗口缩放因子
    pub window_scale: u8,
    /// 是否支持 SACK
    pub sack_permitted: bool,
    /// 时间戳
    pub timestamps: bool,

    // 定时器句柄
    /// 重传定时器
    pub retransmit_timer: Option<TimerHandle>,
    /// 坚持定时器
    pub persist_timer: Option<TimerHandle>,
    /// TIME_WAIT 定时器
    pub time_wait_timer: Option<TimerHandle>,
    /// 延迟 ACK 定时器
    pub delayed_ack_timer: Option<TimerHandle>,
}

/// TCP 伪头部（用于计算校验和）
#[repr(C, packed)]
pub struct TcpPseudoHeader {
    /// 源 IP 地址
    pub source_addr: Ipv4Addr,
    /// 目标 IP 地址
    pub dest_addr: Ipv4Addr,
    /// 零
    pub zero: u8,
    /// 协议号（TCP = 6）
    pub protocol: u8,
    /// TCP 长度
    pub tcp_length: u16,
}

/// TCP 选项类型
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TcpOptionKind {
    /// 行尾（选项结束）
    EndOfOptionList = 0,
    /// 无操作（填充）
    NoOperation = 1,
    /// 最大分段大小（MSS）
    MaxSegmentSize = 2,
    /// 窗口缩放
    WindowScale = 3,
    /// SACK 允许
    SackPermitted = 4,
    /// SACK 选项
    Sack = 5,
    /// 时间戳
    Timestamps = 8,
}

/// TCP 选项
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TcpOption {
    /// 最大分段大小
    MaxSegmentSize { mss: u16 },
    /// 窗口缩放
    WindowScale { shift: u8 },
    /// SACK 允许
    SackPermitted,
    /// SACK 块
    Sack { blocks: Vec<(u32, u32)> },
    /// 时间戳
    Timestamps { ts_val: u32, ts_ecr: u32 },
}

/// TCP 定时器句柄
pub type TimerHandle = usize;
```

### 5.2 枚举类型

```rust
/// TCP 错误类型
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TcpError {
    /// 校验和错误
    ChecksumError,
    /// 序列号不在窗口内
    SequenceOutOfWindow,
    /// 连接不存在
    ConnectionNotExist,
    /// 连接已关闭
    ConnectionClosed,
    /// 无效状态
    InvalidState,
    /// 缓冲区已满
    BufferFull,
    /// 重传次数超限
    RetransmitExceeded,
    /// 连接超时
    ConnectionTimeout,
    /// 连接被重置
    ConnectionReset,
    /// 无效选项
    InvalidOption,
}

/// TCP 事件
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TcpEvent {
    /// 应用层主动打开
    ActiveOpen { remote_addr: SocketAddr },
    /// 应用层被动打开（监听）
    PassiveOpen { local_port: u16 },
    /// 应用层发送数据
    SendData { data: Vec<u8> },
    /// 应用层关闭连接
    Close,
    /// 应用层中止连接
    Abort,
    /// 收到 TCP 报文
    ReceivePacket { packet: Vec<u8> },
    /// 重传定时器超时
    RetransmitTimeout,
    /// 坚持定时器超时
    PersistTimeout,
    /// TIME_WAIT 定时器超时
    TimeWaitTimeout,
    /// 延迟 ACK 定时器超时
    DelayedAckTimeout,
}

/// Socket 地址
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SocketAddr {
    /// IP 地址
    pub ip: Ipv4Addr,
    /// 端口号
    pub port: u16,
}
```

---

## 6. 与其他模块的交互

**TCP 协议在 CoreNet 项目中的模块依赖关系：**

### 6.1 与 Common 模块的交互

- **`src/common/packet.rs`**：
  - 使用 `Packet` 结构封装 TCP 报文（头部 + 数据）
  - TCP 报文作为 IP 数据报的载荷
  - 需要访问和修改 `Packet` 的缓冲区

- **`src/common/error.rs`**：
  - 定义 `TcpError`，实现与 `CoreError` 的转换
  - 错误类型：解析错误、校验和错误、状态错误等

- **`src/common/addr.rs`**：
  - 使用 `Ipv4Addr` 表示连接四元组中的 IP 地址
  - 将来可能支持 `Ipv6Addr`（IPv6 TCP）

### 6.2 与 Interface 模块的交互

- **`src/interface/iface.rs`**：
  - 通过接口获取本地 IP 地址
  - 获取接口 MTU（影响 MSS 计算）

- **`src/context.rs`**（SystemContext）：
  - TCP 需要共享状态（连接表 TCB）
  - 在 SystemContext 中添加 `tcp_connections: Arc<Mutex<TcpConnectionManager>>`

### 6.3 与 IP 协议模块的交互

- **`src/protocols/ip/`**（IPv4）：
  - TCP 报文封装在 IPv4 数据报中（Protocol = 6）
  - IP 头部中的源/目的 IP 地址用于 TCP 伪头部计算
  - IP 分片：TCP 尽量避免超过 MTU（MSS 选项）

- **`src/protocols/ipv6/`**（IPv6）：
  - TCP 报文封装在 IPv6 中（Next Header = 6）
  - IPv6 扩展头部处理

### 6.4 与 Engine/Processor、Scheduler 的交互

- **`src/engine/processor.rs`**：
  - 在 IP 层解析后，根据 Protocol = 6 分发到 TCP 处理器
  - TCP 处理器更新连接状态、发送响应

- **`src/scheduler/scheduler.rs`**：
  - TCP 报文通过调度器在接口队列间流动
  - 需要支持定时器调度（重传、TIME_WAIT 等）

### 6.5 模块初始化顺序

```
1. SystemContext::new()
   ├── InterfaceManager::new()
   └── TcpConnectionManager::new()

2. 绑定端口
   └── TcpConnectionManager::bind(port) → 创建 LISTEN 状态的 TCB

3. 接收数据包
   └── Scheduler::receive_packet()
       └── Processor::process()
           └── IP 层 (Protocol=6)
               └── TcpHandler::handle_packet()

4. 发送数据包
   └── TcpConnectionManager::send_data()
       └── 封装 TCP 报文
           └── IP 层封装
               └── Scheduler::transmit()
```

### 6.6 数据流示例

**连接建立流程：**

```
应用层                          TCP 层                        IP 层
  |                              |                             |
  | connect(remote)              |                             |
  |----------------------------->|                             |
  |                              | 生成 SYN，创建 TCB           |
  |                              |-------------------------->  | 构建 IP 数据报
  |                              |                             |（封装 TCP）
  |                              |                             |
  |                              |  <------------------------- | 收到 SYN-ACK
  |                              |  更新 TCB 状态               |
  |                              |  发送 ACK                    |
  |                              |-------------------------->  |
  |                              |                             |
  | 连接已建立                    |  ESTABLISHED 状态            |
```

**数据传输流程：**

```
应用层                          TCP 层                        IP 层
  |                              |                             |
  | send(data)                   |                             |
  |----------------------------->|                             |
  |                              | 分段（MSS）                  |
  |                              | 分配序列号                   |
  |                              |-------------------------->  |
  |                              |  <------------------------- | 收到 ACK
  |                              |  滑动窗口更新                |
  |                              |  拥塞窗口更新                |
```

---

## 7. 安全考虑

### 7.1 SYN Flood 攻击

**攻击方式：**
- 攻击者发送大量 SYN 报文，不完成三次握手
- 服务器为每个 SYN 分配 TCB 和资源
- 导致资源耗尽，无法处理合法连接

**防御措施：**
- **SYN Cookies**：不立即分配完整 TCB，使用编码的 ISN 验证
- **SYN Cache**：限制半连接队列大小
- **RST 攻击防护**：验证 RST 报文的序列号

### 7.2 序列号猜测攻击

**攻击方式：**
- 攻击者猜测 TCP 序列号，注入伪造数据或劫持连接

**防御措施：**
- **随机 ISN**：使用安全的随机数生成初始序列号
- **PAWS (Protect Against Wrapped Sequence Numbers)**：使用时间戳防止旧报文干扰

### 7.3 重置攻击（RST Attack）

**攻击方式：**
- 发送伪造的 RST 报文，中断合法连接

**防御措施：**
- 严格验证 RST 报文的序列号（必须在窗口内）
- 使用 MD5 签名选项（RFC 2385）

### 7.4 实现建议

1. **边界检查**：所有数组访问前验证长度，防止越界
2. **状态转换验证**：严格检查状态转换合法性
3. **资源限制**：限制连接数量、缓冲区大小，防止资源耗尽
4. **超时处理**：合理设置超时时间，防止僵尸连接占用资源
5. **随机化**：ISN、时间戳等使用安全随机源

---

## 8. 配置参数

```rust
/// TCP 协议配置
#[derive(Debug, Clone)]
pub struct TcpConfig {
    // ========== 基本配置 ==========

    /// 最大分段大小（MSS），默认 1460 字节（以太网 MTU 1500 - IP 20 - TCP 20）
    pub max_segment_size: u16,  // 默认: 1460

    /// 默认接收窗口大小，默认 65535 字节
    pub default_window_size: u16,  // 默认: 65535

    /// 最小接收窗口大小，默认 1460 字节（1 MSS）
    pub min_window_size: u16,  // 默认: 1460

    // ========== 超时配置 ==========

    /// 初始重传超时时间（RTO），默认 1 秒
    pub initial_rto: u32,  // 默认: 1000 (ms)

    /// 最小 RTO，默认 200ms（RFC2988 建议）
    pub min_rto: u32,  // 默认: 200 (ms)

    /// 最大 RTO，默认 120 秒（RFC2988 建议）
    pub max_rto: u32,  // 默认: 120000 (ms)

    /// TIME_WAIT 状态持续时间（2MSL），默认 60 秒
    pub time_wait_duration: u32,  // 默认: 60000 (ms)

    /// 延迟 ACK 定时器，默认 200ms
    pub delayed_ack_timeout: u32,  // 默认: 200 (ms)

    // ========== 拥塞控制配置 ==========

    /// 初始拥塞窗口，默认 10 * MSS（RFC6928）
    pub initial_cwnd: u32,  // 默认: 14600 (10 * 1460)

    /// 初始慢启动阈值，默认无限大
    pub initial_ssthresh: u32,  // 默认: u32::MAX

    // ========== 连接限制 ==========

    /// 最大连接数（包括 TIME_WAIT），默认 1000
    pub max_connections: usize,  // 默认: 1000

    /// 最大半连接数（SYN_RCVD），默认 100
    pub max_half_connections: usize,  // 默认: 100

    /// 最大重传次数，默认 12 次（约 9 分钟）
    pub max_retransmit_attempts: u32,  // 默认: 12

    // ========== 功能开关 ==========

    /// 是否启用窗口缩放，默认 true
    pub enable_window_scale: bool,  // 默认: true

    /// 是否启用 SACK，默认 true
    pub enable_sack: bool,  // 默认: true

    /// 是否启用时间戳，默认 true
    pub enable_timestamps: bool,  // 默认: true

    /// 是否启用延迟 ACK，默认 true
    pub enable_delayed_ack: bool,  // 默认: true

    /// 是否启用 SYN Cookies（防御 SYN Flood），默认 false
    pub enable_syn_cookies: bool,  // 默认: false
}

impl Default for TcpConfig {
    fn default() -> Self {
        Self {
            max_segment_size: 1460,
            default_window_size: 65535,
            min_window_size: 1460,
            initial_rto: 1000,
            min_rto: 200,
            max_rto: 120000,
            time_wait_duration: 60000,
            delayed_ack_timeout: 200,
            initial_cwnd: 14600,
            initial_ssthresh: u32::MAX,
            max_connections: 1000,
            max_half_connections: 100,
            max_retransmit_attempts: 12,
            enable_window_scale: true,
            enable_sack: true,
            enable_timestamps: true,
            enable_delayed_ack: true,
            enable_syn_cookies: false,
        }
    }
}
```

---

## 9. 测试场景

### 9.1 基本功能测试

1. **三次握手测试**
   - 测试内容：客户端主动打开，三次握手完成，双方进入 ESTABLISHED 状态
   - 验证点：SYN、SYN-ACK、ACK 的序列号和确认号正确

2. **数据传输与确认测试**
   - 测试内容：发送数据，接收 ACK，窗口更新
   - 验证点：序列号递增、确认号正确、滑动窗口工作

3. **四次挥手测试**
   - 测试内容：主动关闭方发送 FIN，四次挥手完成，双方进入 CLOSED 状态
   - 验证点：FIN、ACK、TIME_WAIT 状态转换正确

4. **双向数据传输测试**
   - 测试内容：同时发送和接收数据
   - 验证点：双向序列号独立管理，不互相干扰

5. **同时关闭测试**
   - 测试内容：双方同时发送 FIN
   - 验证点：CLOSING 状态转换正确

### 9.2 边界情况测试

1. **零窗口测试**
   - 测试内容：接收窗口为 0 时，发送方停止发送，启动坚持定时器
   - 验证点：零窗口探查正常工作

2. **最大分段大小测试**
   - 测试内容：发送超过 MSS 的数据，自动分段
   - 验证点：分段不超过 MSS，序列号连续

3. **窗口缩放测试**
   - 测试内容：使用窗口缩放选项，窗口 > 65535
   - 验证点：大窗口正确处理

3. **序列号回绕测试**
   - 测试内容：序列号接近 2^32-1，回绕到 0
   - 验证点：使用 PAWS（时间戳）防止旧报文干扰

### 9.3 异常情况测试

1. **超时重传测试**
   - 测试内容：模拟丢包，触发超时重传
   - 验证点：RTO 计算、指数退避、慢启动

2. **快重传和快恢复测试**
   - 测试内容：模拟 3 个重复 ACK
   - 验证点：立即重传、快恢复状态、ssthresh 调整

3. **RST 处理测试**
   - 测试内容：收到 RST 报文，连接立即关闭
   - 验证点：连接状态正确转换，资源释放

4. **非法报文测试**
   - 测试内容：收到校验和错误、序列号错误、标志位冲突的报文
   - 验证点：丢弃报文，可能发送 RST

5. **SYN Flood 防御测试**
   - 测试内容：大量 SYN，不完成三次握手
   - 验证点：半连接队列限制，SYN Cookies 可选

6. **连接超时测试**
   - 测试内容：连接长时间无活动
   - 验证点：保活定时器（可选）或连接超时关闭

7. **乱序报文测试**
   - 测试内容：报文乱序到达
   - 验证点：乱序队列缓存，重组后交付应用层

8. **重复报文测试**
   - 测试内容：收到重复的数据报文
   - 验证点：丢弃重复数据，发送重复 ACK

---

## 10. 参考资料

1. **[RFC 9293](https://datatracker.ietf.org/doc/rfc9293/)** - Transmission Control Protocol (TCP) (2022) - 当前 TCP 标准
2. **[RFC 793](https://datatracker.ietf.org/doc/rfc793/)** - Transmission Control Protocol (1981) - 原始 TCP 规范
3. **[RFC 1122](https://datatracker.ietf.org/doc/rfc1122/)** - Requirements for Internet Hosts -- Communication Layers (1989)
4. **[RFC 7323](https://datatracker.ietf.org/doc/rfc7323/)** - TCP Extensions for High Performance (2014) - 窗口缩放、时间戳、PAWS
5. **[RFC 2018](https://datatracker.ietf.org/doc/rfc2018/)** - TCP Selective Acknowledgment Options (1996) - SACK
6. **[RFC 5681](https://datatracker.ietf.org/doc/rfc5681/)** - TCP Congestion Control (2009) - 拥塞控制标准
7. **[RFC 879](https://datatracker.ietf.org/doc/rfc879/)** - TCP Maximum Segment Size (1983) - MSS
8. **[RFC 2988](https://datatracker.ietf.org/doc/rfc2988/)** - Computing TCP's Retransmission Timer (2000) - RTO 计算
9. **[RFC 3168](https://datatracker.ietf.org/doc/rfc3168/)** - The Addition of Explicit Congestion Notification (ECN) to IP (2001)
10. **[RFC 6268](https://datatracker.ietf.org/doc/rfc6268/)** - TCP Security (2011) - 安全考虑
