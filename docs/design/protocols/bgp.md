# BGP（边界网关协议）详细设计文档

> **实现状态：简化版 - 仅类型定义**
>
> 本协议模块当前仅保留了类型定义和常量，包括：
> - 报文格式类型定义（Open、Update、Notification、Keepalive、Route-Refresh）
> - 状态机类型定义（Idle、Connect、Active、OpenSent、OpenConfirm、Established）
> - 对等体类型定义（IBGP/EBGP）
> - 路径属性类型定义（AS_PATH、NEXT_HOP、LOCAL_PREF、MED 等）
> - RIB 数据结构类型（Adj-RIB-In、Loc-RIB、Adj-RIB-Out）
> - 定时器类型定义（Connect Retry、Keepalive、Hold）
>
> 内部算法实现、状态机转换、详细处理流程均已删除。

## 1. 协议概述

### 1.1 背景与历史

**协议全称和定义：**
- 全称：Border Gateway Protocol 4（边界网关协议第4版）
- 在 TCP/IP 协议栈中的层级位置：应用层协议（运行在 TCP 之上，端口 179）
- 核心功能概述：自治系统（AS）之间的外部路由协议，实现互联网核心的路由交换

**为什么需要 BGP？**

互联网由数以万计的自治系统（AS）组成，每个 AS 由单个组织管理（如 ISP、大型企业、大学）。需要一种协议来实现：
- **AS 之间的路由信息交换**：在不同 AS 之间传递网络可达性信息（NLRI）
- **路径向量机制**：通过 AS_PATH 属性防止路由环路
- **策略路由**：支持基于复杂策略的路由选择（如商业偏好、性能优化）
- **可扩展性**：支持全球互联网规模的路由表（当前约 90 万+ 路由条目）

内部网关协议（IGP，如 OSPF、IS-IS）无法满足这些需求，因为它们设计用于单个 AS 内部，缺乏：
1. 路径信息（只记录跳数/开销）
2. 策略控制能力
3. 大规模路由表支持

**历史背景：**
- **RFC 1105**（1989）：BGP-1，最早版本
- **RFC 1163**（1990）：BGP-2，引入 AS_PATH 属性
- **RFC 1267**（1991）：BGP-3，增强路由聚合
- **RFC 1771**（1995）：BGP-4，当前版本的基础
- **RFC 4271**（2006）：BGP-4 标准修订版，当前标准
- **RFC 4760**（2007）：MP-BGP，多协议扩展（支持 IPv6、VPN、MPLS）

**补充 RFC：**
- RFC 4456：BGP Route Reflection（路由反射）
- RFC 1997：BGP Communities（BGP 团体）
- RFC 2439：BGP Route Flap Damping（路由抑制）
- RFC 6286：AS-Wide Unique BGP Identifier

### 1.2 设计原理

BGP 的核心设计思想是**路径向量路由协议**，结合了距离矢量和策略路由的特点。

```
AS Path 向量传输示例：

AS1 -------- AS2 -------- AS3 -------- AS4
 |            |            |            |
(10.0.1.0/24) (10.0.2.0/24) (10.0.3.0/24) (10.0.4.0/24)

AS1 向 AS2 通告路由时携带 AS_PATH: [AS1]
AS2 向 AS3 通告时携带 AS_PATH: [AS2, AS1]
AS3 向 AS4 通告时携带 AS_PATH: [AS3, AS2, AS1]
AS4 收到后，检查 AS_PATH 中是否包含自己的 AS 号 → 防环
```

**BGP 工作机制：**

```
+----------------+                   +----------------+
|   BGP Speaker  |                   |   BGP Speaker  |
|      (AS1)     |                   |      (AS2)     |
+-------+--------+                   +--------+-------+
        |                                     |
        | 1. TCP 连接建立 (端口 179)           |
        |<------------------------------------>|
        |                                     |
        | 2. OPEN 消息交换 (能力协商)           |
        |<====================================>|
        |                                     |
        | 3. UPDATE 消息 (路由交换)             |
        |<====================================>|
        |                                     |
        | 4. KEEPALIVE 心跳保持连接             |
        |<------------------------------------>|
        |                                     |
        | 5. UPDATE 持续路由更新               |
        |<====================================>|
```

**关键特点：**

1. **路径向量协议**：通过 AS_PATH 属性记录路由经过的自治系统序列，有效防止环路
2. **策略驱动**：基于本地策略选择最优路径，而非简单度量值（如跳数、带宽）
3. **增量更新**：仅在路由变化时发送 UPDATE，支持高效的路由收敛
4. **可靠传输**：使用 TCP 确保消息可靠传输，无需自己实现可靠机制
5. **扩展性**：通过路径属性（如 COMMUNITY、LOCAL_PREF）支持复杂路由策略

---

## 2. 报文格式

### 2.1 报文结构

所有 BGP 报文使用统一的 16 字节头部，后跟特定类型的数据部分。

```
 0                   1                   2                   3
 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|                                                               |
+                                                               +
|                                                               |
+                          Marker                              +
|                                                               |
+                                                               +
|                                                               |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|          Length               |      Type    |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|                                                               |
.                        类型特定的数据                          .
.                                                               .
|                                                               |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
```

**Marker（16 字节）：**
- 用于同步和认证
- 如果未启用认证，全为 1（0xFF）
- 如果启用了 MD5 认证，存储认证数据

**Length（2 字节）：**
- 整个 BGP 报文的总长度（包括头部）
- 范围：19 ~ 4096 字节

**Type（1 字节）：**
- 报文类型码

| 类型码 | 报文类型 | 用途 |
|--------|----------|------|
| 1 | OPEN | 建立对等体关系，协商参数 |
| 2 | UPDATE | 通告路由更新 |
| 3 | NOTIFICATION | 报告错误，关闭连接 |
| 4 | KEEPALIVE | 保持连接活跃 |
| 5 | ROUTE-REFRESH | 请求重新通告路由（RFC 2918） |

### 2.2 字段说明

| 字段 | 大小 | 说明 | 常见值 |
|------|------|------|--------|
| Marker | 16 字节 | 同步标记或认证数据 | 全 1（未认证时） |
| Length | 2 字节 | 报文总长度 | 19 ~ 4096 |
| Type | 1 字节 | 报文类型 | 1-5 |

**最小报文长度：**
- BGP Header: 19 字节（Header 18 字节 + Type 1 字节，无数据）
- KEEPALIVE: 19 字节（仅头部）
- OPEN: 最小 29 字节
- UPDATE: 最小 23 字节（无路由 Withdrawn 或 NLRI）

### 2.3 封装格式

BGP 使用 TCP 作为传输协议：

```
+-------------------+
|   IP Header       |
|   Protocol: 6     |
+-------------------+
|   TCP Header      |
|   Dst Port: 179   |
+-------------------+
|   BGP Message     |
|   - Marker        |
|   - Length        |
|   - Type          |
|   - Data          |
+-------------------+
```

**TCP 端口：** 179（BGP 服务器端口）

---

## 2.3 OPEN 报文格式

```
 0                   1                   2                   3
 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|                                                               |
+                                                               +
|                                                               |
+                          Marker (16 bytes)                   +
|                                                               |
+                                                               +
|                                                               |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|          Length               |      Type    = 1             |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|    Version    |   My AS       |       Hold Time              |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|     BGP Identifier            |      Opt Parm Len            |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|                                                               |
.                    Optional Parameters                       .
.                                                               .
|                                                               |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
```

| 字段 | 大小 | 说明 | 常见值 |
|------|------|------|--------|
| Version | 1 字节 | BGP 版本号 | 4 |
| My AS | 2 字节 | 本地 AS 号 | 实际 AS 号 |
| Hold Time | 2 字节 | 保活超时时间（秒） | 180（推荐） |
| BGP Identifier | 4 字节 | BGP 标识符（通常是路由器 IP） | 路由器 IP |
| Opt Parm Len | 1 字节 | 可选参数长度 | 0 或 >0 |
| Optional Parameters | 变长 | 可选参数列表 | 能力、认证等 |

### 2.4 UPDATE 报文格式

```
 0                   1                   2                   3
 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|                                                               |
+                                                               +
|                                                               +
+                          Marker (16 bytes)                   +
|                                                               |
+                                                               +
|                                                               |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|          Length               |      Type    = 2             |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|   Withdrawn Routes Length     |                                 |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+                                 +
|                                                               |
.                    Withdrawn Routes (变长)                    .
.                                                               |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|        Total Path Attribute Length                           |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|                                                               |
.                 Path Attributes (变长)                         .
.                                                               |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|                                                               |
.                    Network Layer Reachability Info           .
.                    (NLRI) (变长)                               .
|                                                               |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
```

| 字段 | 大小 | 说明 |
|------|------|------|
| Withdrawn Routes Length | 2 字节 | 撤销路由字段的长度 |
| Withdrawn Routes | 变长 | 要撤销的路由（IP 前缀列表） |
| Total Path Attribute Length | 2 字节 | 路径属性总长度 |
| Path Attributes | 变长 | 路径属性列表 |
| NLRI | 变长 | 网络层可达性信息（IP 前缀列表） |

### 2.5 NOTIFICATION 报文格式

```
 0                   1                   2                   3
 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|                                                               |
+                                                               +
|                                                               +
+                          Marker (16 bytes)                   +
|                                                               |
+                                                               +
|                                                               |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|          Length               |      Type    = 3             |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|    ErrorCode   |    ErrorSubCode       |                       |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|                                                               |
.                       Data (变长)                             .
|                                                               |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
```

| 字段 | 大小 | 说明 |
|------|------|------|
| ErrorCode | 1 字节 | 错误码 |
| ErrorSubCode | 1 字节 | 子错误码 |
| Data | 变长 | 错误相关数据 |

---

## 3. 状态机设计

BGP 使用有限状态机（FSM）管理对等体连接的生命周期。每个 BGP 对等体对应一个 FSM 实例。

### 3.0 状态变量

BGP FSM 维护的状态变量：

| 变量名称 | 类型 | 用途 | 初始值 |
|---------|------|------|--------|
| ConnectRetryCounter | u32 | 连接重试计数器 | 0 |
| ConnectRetryTimer | Duration | 连接重试定时器 | 60 秒 |
| HoldTimer | Duration | 保活定时器 | 180 秒 |
| KeepaliveTimer | Duration | 心跳发送定时器 | HoldTime / 3 |
| SendMsgLength | usize | 已发送消息长度 | 0 |
| RecvMsgLength | usize | 已接收消息长度 | 0 |

### 3.1 状态定义

```
                            +-----------+
                            |   Idle    |
                            +-----------+
                                 |
                                 | BGP Start
                                 v
                            +-----------+
                            |   Connect |
                            +-----------+
                                 |
                +----------------+----------------+
                | TCP Success     | TCP Fail /    |
                v                 | ConnectRetry  |
         +-----------+           | Timeout        |
         |   OpenSent|           v
         +-----------+      +-----------+
                |            |    Active  |
                | OpenRecv   +-----------+
                v                |
         +-----------+          | ConnectRetry Success
         | OpenConfirm|         v
         +-----------+   +-----------+
                |       |   Idle     |
                |       +-----------+
                v
         +-----------+
         | Established|
         +-----------+
                |
                | Notification / TCP Fail
                v
            (释放资源)
                |
                v
         +-----------+
         |   Idle    |
         +-----------+
```

| 状态 | 说明 |
|------|------|
| Idle | 初始状态，拒绝所有入站连接，等待 BGP Start 事件 |
| Connect | 尝试建立 TCP 连接，等待连接完成 |
| Active | TCP 连接失败，等待重试 |
| OpenSent | TCP 连接成功，已发送 OPEN，等待对方 OPEN |
| OpenConfirm | 收到对方 OPEN，发送 KEEPALIVE 确认 |
| Established | 双方交换 KEEPALIVE 完成，可交换 UPDATE |

### 3.2 状态转换详解

#### 3.2.1 Idle（空闲）

**描述：** 初始状态，拒绝所有入站 BGP 连接

**进入条件：**
- BGP FSM 初始化
- 收到 NOTIFICATION 或发生致命错误
- 人工重置

**行为：**
- 拒绝所有入站 BGP 连接
- 释放所有资源（路由、定时器）
- 初始化 ConnectRetryCounter 为 0

**转换条件：**
- **BGP Start 事件** → Connect（发起出站连接）
- **BGP Start 事件** → Active（允许入站连接后）
- （注：某些实现支持直接从 Idle 进入 Active）

**相关资源：**
- 无需分配资源

#### 3.2.2 Connect（连接中）

**描述：** 尝试建立 TCP 连接

**进入条件：**
- 从 Idle 收到 BGP Start 事件

**行为：**
- 启动 ConnectRetryTimer
- 尝试建立 TCP 连接到对等体
- 等待 TCP 连接完成

**转换条件：**
- **TCP 连接成功** → OpenSent（发送 OPEN）
- **ConnectRetryTimer 超时** → Active（重试连接，ConnectRetryCounter++）
- **其他事件导致回到 Idle** → Idle
- **BGP Start 事件（支持入站）** → Active

**相关资源：**
- ConnectRetryTimer

#### 3.2.3 Active（激活）

**描述：** TCP 连接失败，等待重试；同时监听入站连接

**进入条件：**
- 从 Connect 连接失败
- 从 Connect 等待入站连接

**行为：**
- 继续尝试建立 TCP 连接
- 监听入站 BGP 连接
- ConnectRetryTimer 运行中

**转换条件：**
- **ConnectRetryTimer 超时** → Connect（重新尝试）
- **TCP 连接成功** → OpenSent
- **BGP Start 事件** → Active（无变化）
- **其他事件导致回到 Idle** → Idle

**相关资源：**
- ConnectRetryTimer
- TCP 连接监听

#### 3.2.4 OpenSent（OPEN 已发送）

**描述：** TCP 连接已建立，已发送 OPEN，等待对方 OPEN

**进入条件：**
- 从 Connect/Active 收到 TCP 连接成功

**行为：**
- 发送 OPEN 消息
- 停止 ConnectRetryTimer
- 启动 HoldTimer

**转换条件：**
- **收到 OPEN（BGP ID 不一致）** → Idle（发送 NOTIFICATION）
- **收到 OPEN（HoldTime 不一致）** → OpenConfirm（协商取较小值）
- **收到其他消息** → Idle（发送 NOTIFICATION）
- **HoldTimer 超时** → Idle
- **TCP 连接断开** → Idle

**相关资源：**
- HoldTimer
- 已发送的 OPEN 消息

#### 3.2.5 OpenConfirm（OPEN 确认）

**描述：** 收到对方 OPEN，发送 KEEPALIVE 等待确认

**进入条件：**
- 从 OpenSent 收到合法的 OPEN

**行为：**
- 发送 KEEPALIVE
- 保持 HoldTimer 运行

**转换条件：**
- **收到 KEEPALIVE** → Established（连接建立完成）
- **收到其他消息** → Idle（发送 NOTIFICATION）
- **HoldTimer 超时** → Idle
- **TCP 连接断开** → Idle

**相关资源：**
- HoldTimer

#### 3.2.6 Established（已建立）

**描述：** 连接完全建立，可交换路由信息

**进入条件：**
- 从 OpenConfirm 收到 KEEPALIVE

**行为：**
- 启动 KeepaliveTimer（HoldTime / 3）
- 定期发送 KEEPALIVE
- 接收/发送 UPDATE 消息
- 接收 KEEPALIVE 时重置 HoldTimer

**转换条件：**
- **收到 NOTIFICATION** → Idle
- **收到 UPDATE 处理失败** → Idle（发送 NOTIFICATION）
- **HoldTimer 超时** → Idle
- **TCP 连接断开** → Idle
- **人工停止** → Idle

**相关资源：**
- HoldTimer
- KeepaliveTimer
- 路由表（Adj-RIB-In/Adj-RIB-Out/Loc-RIB）
- 定时器：ConnectRetryTimer, HoldTimer, KeepaliveTimer

---

## 4. 报文处理逻辑

### 4.0 定时器

BGP 使用的定时器：

| 定时器名称 | 启动条件 | 超时时间 | 超时动作 |
|-----------|---------|---------|---------|
| ConnectRetryTimer | 进入 Connect 状态 | 60 秒（可配置） | 重试 TCP 连接 |
| HoldTimer | 收到 OPEN/KEEPALIVE/UPDATE | 协商值（默认 180 秒） | 发送 NOTIFICATION，回 Idle |
| KeepaliveTimer | 进入 Established 状态 | HoldTime / 3 | 发送 KEEPALIVE |

### 4.1 接收处理总流程

```
+----------------+
|  接收 BGP 报文  |
+-------+--------+
        |
        v
+-------+--------+
|  验证 Marker   | 失败 → NOTIFICATION，关闭连接
+-------+--------+
        |
        v
+-------+--------+
|  验证 Length   | 失败 → NOTIFICATION，关闭连接
+-------+--------+
        |
        v
+-------+--------+
|  解析 Type     |
+-------+--------+
        |
        +------+-------+-------+-------+
        |      |       |       |       |
        v      v       v       v       v
     OPEN   UPDATE  NOTIF   KEEPALIVE  ROUTE-REFRESH
        |      |       |       |              |
        v      v       v       v              v
   处理OPEN 处理UPDATE 处理NOTIF 处理KEEPALIVE 处理ROUTE-REFRESH
```

### 4.2 OPEN 报文处理

**处理流程：**

1. **提取信息：**
   - Version → BGP 版本（必须为 4）
   - My AS → 对端 AS 号
   - Hold Time → 保活时间（协商取最小值）
   - BGP Identifier → 对端 BGP ID（路由器标识）
   - Optional Parameters → 能力协商（MP-BGP、Route Refresh 等）

2. **处理步骤：**
   - 验证 Version = 4
   - 检查 BGP Identifier 是否与本地冲突
   - 协商 HoldTime（取本地和远程的较小值）
   - 处理 Optional Parameters（能力协商）
   - 记录对端信息

3. **资源更新：**
   - 定时器：HoldTimer [启动] 协商值
   - 定时器：ConnectRetryTimer [停止]
   - 状态变量：HoldTimer → 协商值

4. **响应动作：**
   - 状态转换：OpenSent → OpenConfirm
   - 发送 KEEPALIVE 确认

### 4.3 UPDATE 报文处理

**处理流程：**

1. **提取信息：**
   - Withdrawn Routes → 需要撤销的路由前缀
   - Path Attributes → 路径属性（AS_PATH, NEXT_HOP, LOCAL_PREF, MED 等）
   - NLRI → 通告的路由前缀

2. **处理步骤：**

   **步骤 1：处理撤销路由**
   - 遍历 Withdrawn Routes 列表
   - 从 Adj-RIB-In 中删除对应路由
   - 触发路由选择过程

   **步骤 2：验证路径属性**
   - 检查必需属性（ORIGIN, AS_PATH, NEXT_HOP）是否存在
   - 验证 AS_PATH 中不包含本地 AS（环路检测）
   - 验证 NEXT_HOP 的可达性
   - 处理可选属性

   **步骤 3：处理路由通告**
   - 解析 NLRI 前缀列表
   - 将路由添加到 Adj-RIB-In
   - 应用入站策略过滤
   - 触发路由选择过程

3. **资源更新：**
   - 表项：Adj-RIB-In [添加/删除] 路由条目
   - 定时器：HoldTimer [重置]
   - 状态变量：路由表变化标志 → true

4. **响应动作：**
   - 运行路由选择决策（Decision Process）
   - 更新 Loc-RIB
   - 生成 UPDATE 发送给其他对等体（Adj-RIB-Out）

### 4.4 NOTIFICATION 报文处理

**处理流程：**

1. **提取信息：**
   - ErrorCode → 错误类型
   - ErrorSubCode → 子错误类型
   - Data → 错误相关数据

2. **处理步骤：**
   - 记录错误日志
   - 关闭 TCP 连接
   - 释放所有资源

3. **资源更新：**
   - 定时器：所有定时器 [停止]
   - 表项：Adj-RIB-In, Adj-RIB-Out, Loc-RIB [清空]

4. **响应动作：**
   - 状态转换：任何状态 → Idle
   - 释放 BGP 资源

### 4.5 KEEPALIVE 报文处理

**处理流程：**

1. **提取信息：**
   - 无数据部分

2. **处理步骤：**
   - 重置 HoldTimer

3. **资源更新：**
   - 定时器：HoldTimer [重置]

4. **响应动作：**
   - 无（仅保持连接）

### 4.6 ROUTE-REFRESH 报文处理

**处理流程：**

1. **提取信息：**
   - AFI → 地址族标识（IPv4/IPv6）
   - SAFI → 子地址族标识（Unicast/Multicast/VPN）

2. **处理步骤：**
   - 检查本地是否支持 Route Refresh 能力
   - 重新运行出站策略
   - 重新发送指定地址族的路由

3. **资源更新：**
   - 定时器：HoldTimer [重置]

4. **响应动作：**
   - 发送 UPDATE（包含指定地址族的路由）

---

## 5. 核心数据结构

### 5.0 表项/缓存

BGP 维护的表项和缓存：

| 资源名称 | 用途 | 最大容量 | 淘汰策略 |
|---------|------|---------|---------|
| Adj-RIB-In | 存储从对等体接收的所有路由（未过滤） | 无限制 | 对等体断开时清空 |
| Adj-RIB-Out | 存储准备发送给对等体的路由（策略后） | 无限制 | 对等体断开时清空 |
| Loc-RIB | 本地路由表（BGP 选路后的路由） | 无限制 | 撤销/替换 |

#### 5.0.1 Adj-RIB-In（入站路由信息库）

**用途：** 存储从每个对等体接收到的所有路由（应用入站策略前）

**关键操作：**
- **查询：** 根据前缀、对等体查询
- **添加：** 收到 UPDATE 时添加
- **更新：** 收到相同前缀的更新时替换
- **删除：** 收到 Withdrawn 时删除

#### 5.0.2 Adj-RIB-Out（出站路由信息库）

**用途：** 存储经过出站策略过滤后，准备发送给对等体的路由

**关键操作：**
- **查询：** 根据前缀、对等体查询
- **添加/更新：** 路由选择后更新
- **删除：** 收到 Route-Refresh 或策略变化时重建

#### 5.0.3 Loc-RIB（本地路由信息库）

**用途：** 存储经过 BGP 路由选择决策后的最优路由

**关键操作：**
- **查询：** 前缀查询（最长匹配）
- **添加/更新：** Decision Process 更新
- **删除：** 路由撤销时删除

### 5.1 报文结构

#### 5.1.1 BGP 头部

```rust
/// BGP 报文头部（所有 BGP 报文通用）
#[derive(Debug, Clone)]
pub struct BgpHeader {
    /// 同步标记（16 字节），用于认证或同步
    pub marker: [u8; 16],
    /// 报文总长度（包含头部）
    pub length: u16,
    /// 报文类型：1=OPEN, 2=UPDATE, 3=NOTIFICATION, 4=KEEPALIVE, 5=ROUTE-REFRESH
    pub msg_type: u8,
}
```

#### 5.1.2 OPEN 报文

```rust
/// BGP OPEN 报文
#[derive(Debug, Clone)]
pub struct BgpOpen {
    /// BGP 版本号（必须为 4）
    pub version: u8,
    /// 本地 AS 号
    pub my_as: u16,
    /// 保活时间（秒）
    pub hold_time: u16,
    /// BGP 标识符（通常是路由器 IP）
    pub bgp_identifier: Ipv4Addr,
    /// 可选参数
    pub optional_parameters: Vec<OptionalParameter>,
}

/// 可选参数
#[derive(Debug, Clone)]
pub enum OptionalParameter {
    /// 认证信息（RFC 4271）
    Authentication {
        auth_code: u8,
        data: Vec<u8>,
    },
    /// 能力通告（RFC 5492）
    Capabilities {
        capabilities: Vec<Capability>,
    },
}
```

#### 5.1.3 UPDATE 报文

```rust
/// BGP UPDATE 报文
#[derive(Debug, Clone)]
pub struct BgpUpdate {
    /// 撤销的路由前缀列表
    pub withdrawn_routes: Vec<IpPrefix>,
    /// 路径属性
    pub path_attributes: Vec<PathAttribute>,
    /// 网络层可达性信息（通告的路由前缀）
    pub nlri: Vec<IpPrefix>,
}

/// 路径属性
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PathAttribute {
    /// ORIGIN（必须）：路由起源
    Origin {
        /// 0=IGP, 1=EGP, 2=INCOMPLETE
        origin: u8,
    },
    /// AS_PATH（必须）：AS 路径
    AsPath {
        /// AS 序列段（严格）
        as_sequence: Vec<u32>,
        /// AS 集合段（松散）
        as_set: Vec<u32>,
    },
    /// NEXT_HOP（必须）：下一跳 IP
    NextHop {
        next_hop: Ipv4Addr,
    },
    /// MULTI_EXIT_DISC（可选）：MED，用于 AS 内路由选择
    MultiExitDisc {
        med: u32,
    },
    /// LOCAL_PREF（可选）：本地优先级，用于出站选路
    LocalPref {
        local_pref: u32,
    },
    /// ATOMIC_AGGREGATE（可选）：聚合路由标志
    AtomicAggregate,
    /// AGGREGATOR（可选）：聚合者信息
    Aggregator {
        as_number: u32,
        router_id: Ipv4Addr,
    },
    /// COMMUNITY（可选）：BGP 团体（RFC 1997）
    Community {
        communities: Vec<u32>,
    },
    /// MP_REACH_NLRI（可选）：多协议可达 NLRI（RFC 4760）
    MpReachNlri {
        afi: u16,           // 地址族标识
        safi: u8,           // 子地址族标识
        next_hop: Vec<u8>,  // 下一跳（可能是 IPv6）
        nlri: Vec<MpNlri>,  // 多协议 NLRI
    },
    /// MP_UNREACH_NLRI（可选）：多协议不可达 NLRI（RFC 4760）
    MpUnreachNlri {
        afi: u16,
        safi: u8,
        nlri: Vec<MpNlri>,
    },
}
```

#### 5.1.4 NOTIFICATION 报文

```rust
/// BGP NOTIFICATION 报文
#[derive(Debug, Clone)]
pub struct BgpNotification {
    /// 错误码
    pub error_code: u8,
    /// 子错误码
    pub error_subcode: u8,
    /// 错误数据
    pub data: Vec<u8>,
}

/// 错误码定义
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum BgpErrorCode {
    /// 消息头错误
    MessageHeaderError = 1,
    /// OPEN 消息错误
    OpenMessageError = 2,
    /// UPDATE 消息错误
    UpdateMessageError = 3,
    /// 保活定时器超时
    HoldTimerExpired = 4,
    /// 有限状态机错误
    FiniteStateMachineError = 5,
    /// 停止
    Cease = 6,
}

/// 消息头错误子码
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum MessageHeaderErrorSubcode {
    ConnectionNotSynchronized = 1,
    BadMessageLength = 2,
    BadMessageType = 3,
}
```

#### 5.1.5 KEEPALIVE 报文

```rust
/// BGP KEEPALIVE 报文（仅包含头部，无数据）
#[derive(Debug, Clone)]
pub struct BgpKeepalive;
```

#### 5.1.6 ROUTE-REFRESH 报文

```rust
/// BGP ROUTE-REFRESH 报文（RFC 2918）
#[derive(Debug, Clone)]
pub struct BgpRouteRefresh {
    /// 地址族标识（1=IPv4, 2=IPv6）
    pub afi: u16,
    /// 保留（必须为 0）
    pub reserved: u8,
    /// 子地址族标识（1=Unicast, 2=Multicast）
    pub safi: u8,
}
```

### 5.2 枚举类型

#### 5.2.1 BGP 状态

```rust
/// BGP 有限状态机状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BgpState {
    /// 空闲状态
    Idle,
    /// 连接中
    Connect,
    /// 激活（监听入站连接）
    Active,
    /// OPEN 已发送
    OpenSent,
    /// OPEN 确认
    OpenConfirm,
    /// 已建立
    Established,
}
```

#### 5.2.2 BGP 能力

```rust
/// BGP 能力类型（RFC 5492）
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BgpCapability {
    /// 多协议扩展（MP-BGP, RFC 4760）
    MultiProtocol {
        afi: u16,
        safi: u8,
    },
    /// 路由刷新（RFC 2918）
    RouteRefresh,
    /// 支持 4 字节 AS 号（RFC 6793）
    FourOctetAsNumber,
    /// 支持 Capability 参数（RFC 5492）
    CapabilityNegotiation,
    /// 支持路由反射（RFC 4456）
    RouteReflection {
        cluster_id: u32,
    },
    /// 其他未知能力
    Unknown {
        code: u8,
        data: Vec<u8>,
    },
}
```

#### 5.2.3 IP 前缀

```rust
/// IP 前缀（用于 NLRI 和 Withdrawn Routes）
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct IpPrefix {
    /// IP 地址
    pub prefix: IpAddr,
    /// 前缀长度
    pub prefix_len: u8,
}

/// 多协议 NLRI（用于 MP-BGP）
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MpNlri {
    /// NLRI 数据（格式取决于 AFI/SAFI）
    pub data: Vec<u8>,
}
```

### 5.3 BGP 对等体

```rust
/// BGP 对等体信息
#[derive(Debug, Clone)]
pub struct BgpPeer {
    /// 对等体 IP 地址
    pub address: IpAddr,
    /// 对等体 AS 号
    pub as_number: u32,
    /// 连接状态
    pub state: BgpState,
    /// BGP 标识符
    pub bgp_id: Ipv4Addr,
    /// 协商的 Hold Time
    pub hold_time: Duration,
    /// 对等体类型
    pub peer_type: BgpPeerType,
    /// 入站 RIB
    pub adj_rib_in: BgpRib,
    /// 出站 RIB
    pub adj_rib_out: BgpRib,
}

/// BGP 对等体类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BgpPeerType {
    /// 外部 BGP（EBGP）：不同 AS 之间的对等体
    External,
    /// 内部 BGP（IBGP）：同一 AS 内的对等体
    Internal,
}
```

### 5.4 BGP 路由表

```rust
/// BGP 路由信息库（RIB）
#[derive(Debug, Clone)]
pub struct BgpRib {
    /// 路由条目列表
    pub routes: Vec<BgpRoute>,
}

/// BGP 路由条目
#[derive(Debug, Clone)]
pub struct BgpRoute {
    /// 网络前缀
    pub prefix: IpPrefix,
    /// 下一跳
    pub next_hop: IpAddr,
    /// 本地优先级（仅 IBGP）
    pub local_pref: Option<u32>,
    /// MED（多出口鉴别器）
    pub med: u32,
    /// AS 路径长度
    pub as_path_length: usize,
    /// 起源类型
    pub origin: u8,
    /// BGP 团体
    pub communities: Vec<u32>,
    /// 来自哪个对等体
    pub peer: IpAddr,
}
```

### 5.5 BGP 配置

```rust
/// BGP 配置
#[derive(Debug, Clone)]
pub struct BgpConfig {
    /// 本地 AS 号
    pub local_as: u32,
    /// BGP 标识符（通常是路由器 IP）
    pub bgp_id: Ipv4Addr,
    /// Hold Time（秒，默认 180）
    pub hold_time: u16,
    /// Connect Retry Time（秒，默认 60）
    pub connect_retry_time: u16,
    /// Keepalive Time（秒，默认 HoldTime / 3）
    pub keepalive_time: Option<u16>,
    /// 对等体列表
    pub peers: Vec<BgpPeerConfig>,
    /// 是否启用 MD5 认证
    pub enable_md5_auth: bool,
    /// 是否支持 4 字节 AS 号
    pub support_4byte_as: bool,
}

/// BGP 对等体配置
#[derive(Debug, Clone)]
pub struct BgpPeerConfig {
    /// 对等体 IP 地址
    pub address: IpAddr,
    /// 对等体 AS 号
    pub remote_as: u32,
    /// 对等体类型
    pub peer_type: BgpPeerType,
    /// 是否启用该对等体
    pub enabled: bool,
}
```

---

## 6. 与其他模块的交互

### 6.1 与 Common 模块的交互

**使用的 Common 组件：**

| 组件 | 用途 | 使用方式 |
|------|------|----------|
| `packet::Packet` | BGP 报文的封装和解析 | 将 BGP 消息封装到 TCP 数据段中 |
| `error::CoreError` | 错误处理 | BGP 解析错误转换为 CoreError |
| `addr::Ipv4Addr` | IPv4 地址处理 | BGP Identifier、Next Hop |
| `queue::RingQueue` | 队列操作 | 用于报文缓冲（通过 Scheduler） |

### 6.2 与 Interface 模块的交互

**使用的 Interface 组件：**

| 组件 | 用途 | 使用方式 |
|------|------|----------|
| `Interface` | 网络接口信息 | 获取本地 IP 地址、MTU |
| `InterfaceManager` | 接口管理 | 查询可用接口，绑定 BGP 连接 |

**交互示例：**
```rust
// 获取接口 IP 地址作为 BGP ID
let local_ip = context.interfaces.lock().unwrap()
    .get_interface_by_name("eth0")
    .map(|iface| iface.ipv4_addr);
```

### 6.3 与 TCP 模块的交互

BGP 作为应用层协议，运行在 TCP 之上：

| 组件 | 用途 | 使用方式 |
|------|------|----------|
| `TcpSocket` | BGP 连接的建立和维护 | 创建 TCP Socket 连接到端口 179 |
| `SocketManager` | Socket 管理 | 注册 BGP Socket，接收数据 |

**交互流程：**
```
BGP 模块 → SocketManager → 创建 TCP Socket → 连接到对等体:179
BGP 模块 ← SocketManager ← 接收 TCP 数据 → 解析 BGP 消息
```

### 6.4 与 Route 模块的交互

BGP 将选中的最优路由注入到路由表：

| 组件 | 用途 | 使用方式 |
|------|------|----------|
| `RouteTable` | 路由表管理 | 将 BGP 路由注入 IPv4/IPv6 路由表 |
| `RouteEntry` | 路由条目 | 创建 BGP 类型的路由条目 |

**交互示例：**
```rust
// 将 BGP 路由注入路由表
for route in &loc_rib {
    let entry = RouteEntry {
        dest: route.prefix,
        gateway: route.next_hop,
        metric: route.local_pref.unwrap_or(100),
        protocol: RouteProtocol::Bgp,
        interface: iface.clone(),
    };
    context.route_table.lock().unwrap().add_route(entry);
}
```

### 6.5 与 Engine/Processor、Scheduler 的交互

**与 Processor 的交互：**
- Processor 解析 TCP 数据段后，将应用层数据（BGP 消息）传递给 BGP 模块处理
- BGP 模块不是通过 Processor 直接调用，而是通过 Socket API

**与 Scheduler 的交互：**
- Scheduler 负责调度 BGP 定时器（HoldTimer、KeepaliveTimer、ConnectRetryTimer）
- BGP 模块通过定时器回调触发报文发送

**数据流示例：**
```
网络 → RxQ → Ethernet → IP → TCP → Socket API → BGP 模块
BGP 模块 → Socket API → TCP → IP → Ethernet → TxQ → 网络
```

### 6.6 模块初始化顺序

```
1. Common 模块初始化
2. Interface 模块初始化
3. Route 模块初始化
4. TCP 模块初始化
5. Socket 模块初始化
6. BGP 模块初始化
   - 创建 BgpContext（包含所有对等体状态）
   - 加载 BgpConfig
   - 为每个对等体创建 FSM 实例
   - 启动定时器任务
   - 连接到对等体
7. Scheduler 启动
```

---

## 7. 安全考虑

### 7.1 路由劫持攻击

**攻击方式：**
- 攻击者通告不属于其前缀的路由
- 导致流量被劫持到攻击者网络

**攻击影响：**
- 流量劫持、中间人攻击、服务拒绝

**防御措施：**
1. **过滤：** 根据路由策略过滤非法路由（如 RFC 6811）
2. **RPKI：** 使用资源公钥基础设施验证路由起源（RFC 6480）
3. **BGPsec：** 使用数字签名验证 AS_PATH（RFC 8205）

### 7.2 路由泄露攻击

**攻击方式：**
- 攻击者将路由从一个 ISP 泄露到另一个 ISP
- 导致次优路由或流量劫持

**攻击影响：**
- 流量绕路、性能下降、拥塞

**防御措施：**
1. **过滤：** 根据业务关系过滤路由
2. ** communities：** 使用 BGP 团体标记路由传播范围
3. **RPKI：** 验证路由授权

### 7.3 BGP 会话劫持

**攻击方式：**
- 攻击者猜测 TCP 序列号，劫持 BGP 会话
- 发送恶意 UPDATE 或 NOTIFICATION

**攻击影响：**
- BGP 会话中断、路由表混乱

**防御措施：**
1. **MD5 认证：** 启用 TCP MD5 签名选项（RFC 2385）
2. **IPsec：** 使用 IPsec 保护 BGP 会话
3. **GTSM：** 通用 TTL 安全机制（RFC 5082）

### 7.4 BGP 消息篡改

**攻击方式：**
- 攻击者篡改 BGP 消息内容
- 发送恶意 UPDATE 或 NOTIFICATION

**攻击影响：**
- 路由表被污染、服务中断

**防御措施：**
1. **MD5 认证：** 验证消息完整性
2. **BGPsec：** 使用数字签名保护路径属性
3. **过滤：** 严格验证 UPDATE 内容

### 7.5 实现建议

1. **默认拒绝：** 默认拒绝所有未授权的 BGP 连接
2. **认证：** 强制启用 MD5 认证或更高级认证机制
3. **过滤：** 实现入站和出站路由过滤
4. **限速：** 限制 BGP UPDATE 发送速率，防止路由震荡
5. **日志：** 记录所有 BGP 事件和错误，便于审计
6. **验证：** 验证所有 UPDATE 消息的合法性
7. **超时：** 使用合理的 HoldTime 和 Keepalive 值

---

## 8. 配置参数

```rust
/// BGP 协议配置
#[derive(Debug, Clone)]
pub struct BgpConfig {
    // === 基本配置 ===

    /// 本地 AS 号（默认：0，必须配置）
    pub local_as: u32,

    /// BGP 标识符（默认：0.0.0.0，通常使用路由器 IP）
    pub bgp_id: Ipv4Addr,

    // === 定时器配置 ===

    /// Hold Time（秒，默认：180）
    /// 接收 BGP 消息的最大超时时间
    pub hold_time: u16,

    /// Connect Retry Time（秒，默认：60）
    /// TCP 连接重试的超时时间
    pub connect_retry_time: u16,

    /// Keepalive Time（秒，默认：None，自动计算为 HoldTime / 3）
    pub keepalive_time: Option<u16>,

    // === 能力配置 ===

    /// 是否支持 4 字节 AS 号（默认：true）
    pub support_4byte_as: bool,

    /// 是否支持多协议扩展（MP-BGP）（默认：true）
    pub support_multiprotocol: bool,

    /// 是否支持路由刷新（默认：true）
    pub support_route_refresh: bool,

    // === 安全配置 ===

    /// 是否启用 MD5 认证（默认：false）
    pub enable_md5_auth: bool,

    /// MD5 认证密钥（当 enable_md5_auth=true 时必填）
    pub md5_key: Option<String>,

    /// 是否启用 GTSM（默认：false）
    pub enable_gtsm: bool,

    // === 对等体配置 ===

    /// BGP 对等体列表
    pub peers: Vec<BgpPeerConfig>,

    // === 路由策略配置 ===

    /// 入站路由策略（默认：允许所有）
    pub inbound_policy: BgpPolicy,

    /// 出站路由策略（默认：允许所有）
    pub outbound_policy: BgpPolicy,

    // === 性能配置 ===

    /// 最大 UPDATE 消息发送速率（条/秒，默认：1000）
    pub max_update_rate: u32,

    /// 路由抑制参数（RFC 2439）
    pub flap_damping: Option<BgpFlapDampingConfig>,
}

/// BGP 对等体配置
#[derive(Debug, Clone)]
pub struct BgpPeerConfig {
    /// 对等体名称（用于标识）
    pub name: String,

    /// 对等体 IP 地址
    pub address: IpAddr,

    /// 对等体 AS 号
    pub remote_as: u32,

    /// 对等体类型（EBGP/IBGP）
    pub peer_type: BgpPeerType,

    /// 是否启用该对等体（默认：true）
    pub enabled: bool,

    /// 是否为被动模式（仅接受入站连接）
    pub passive: bool,

    /// 对等体特定的 Hold Time（可选）
    pub hold_time: Option<u16>,

    /// 对等体特定的 MD5 密钥（可选）
    pub md5_key: Option<String>,

    /// 对等体特定的入站策略（可选）
    pub inbound_policy: Option<BgpPolicy>,

    /// 对等体特定的出站策略（可选）
    pub outbound_policy: Option<BgpPolicy>,
}

/// BGP 路由策略
#[derive(Debug, Clone)]
pub struct BgpPolicy {
    /// 策略语句列表
    pub statements: Vec<BgpPolicyStatement>,
}

/// BGP 策略语句
#[derive(Debug, Clone)]
pub struct BgpPolicyStatement {
    /// 匹配条件
    pub match_condition: BgpMatchCondition,
    /// 动作
    pub action: BgpPolicyAction,
}

/// BGP 匹配条件
#[derive(Debug, Clone)]
pub enum BgpMatchCondition {
    /// 匹配所有
    All,
    /// 匹配特定前缀
    Prefix { prefix: IpPrefix },
    /// 匹配前缀列表
    PrefixList { prefixes: Vec<IpPrefix> },
    /// 匹配 AS_PATH 正则表达式
    AsPath { regex: String },
    /// 匹配团体
    Community { communities: Vec<u32> },
}

/// BGP 策略动作
#[derive(Debug, Clone)]
pub enum BgpPolicyAction {
    /// 允许
    Accept,
    /// 拒绝
    Reject,
    /// 设置属性
    SetAttributes {
        local_pref: Option<u32>,
        med: Option<u32>,
        communities: Option<Vec<u32>>,
    },
}

/// BGP 路由抑制配置
#[derive(Debug, Clone)]
pub struct BgpFlapDampingConfig {
    /// 抑制半衰期（秒，默认：900）
    pub half_life: u32,

    /// 抑制阈值（默认：16）
    pub suppress_value: u32,

    /// 复用阈值（默认：750）
    pub reuse_value: u32,

    /// 最大抑制时间（秒，默认：3600）
    pub max_suppress: u32,
}

// 默认配置实现
impl Default for BgpConfig {
    fn default() -> Self {
        Self {
            local_as: 0,
            bgp_id: Ipv4Addr::UNSPECIFIED,
            hold_time: 180,
            connect_retry_time: 60,
            keepalive_time: None,
            support_4byte_as: true,
            support_multiprotocol: true,
            support_route_refresh: true,
            enable_md5_auth: false,
            md5_key: None,
            enable_gtsm: false,
            peers: Vec::new(),
            inbound_policy: BgpPolicy { statements: vec![] },
            outbound_policy: BgpPolicy { statements: vec![] },
            max_update_rate: 1000,
            flap_damping: None,
        }
    }
}

impl Default for BgpPeerConfig {
    fn default() -> Self {
        Self {
            name: String::new(),
            address: IpAddr::V4(Ipv4Addr::UNSPECIFIED),
            remote_as: 0,
            peer_type: BgpPeerType::External,
            enabled: true,
            passive: false,
            hold_time: None,
            md5_key: None,
            inbound_policy: None,
            outbound_policy: None,
        }
    }
}
```

---

## 9. 测试场景

### 9.1 基本功能测试

1. **BGP 连接建立测试**
   - 测试内容：建立 EBGP 和 IBGP 连接
   - 验证点：OPEN 交换、KEEPALIVE、状态转换到 Established

2. **路由通告测试**
   - 测试内容：对等体之间交换 UPDATE 消息
   - 验证点：NLRI 解析、路径属性处理、路由表更新

3. **路由撤销测试**
   - 测试内容：发送带有 Withdrawn Routes 的 UPDATE
   - 验证点：路由从表中删除

4. **心跳保持测试**
   - 测试内容：周期性发送 KEEPALIVE
   - 验证点：HoldTimer 不超时、连接保持

### 9.2 边界情况测试

1. **最大报文长度测试**
   - 测试内容：发送 4096 字节的 UPDATE
   - 验证点：正确处理或拒绝

2. **空 UPDATE 测试**
   - 测试内容：发送不带 NLRI 和 Withdrawn Routes 的 UPDATE
   - 验证点：正确处理

3. **HoldTime 协商测试**
   - 测试内容：双方 HoldTime 不一致
   - 验证点：使用较小值

4. **大量路由测试**
   - 测试内容：发送 1000+ 路由条目
   - 验证点：性能、内存占用

### 9.3 异常情况测试

1. **NOTIFICATION 处理测试**
   - 测试内容：发送各种 NOTIFICATION
   - 验证点：正确关闭连接、释放资源

2. **HoldTimer 超时测试**
   - 测试内容：停止发送 KEEPALIVE
   - 验证点：超时后发送 NOTIFICATION、关闭连接

3. **TCP 连接断开测试**
   - 测试内容：模拟 TCP 连接中断
   - 验证点：释放资源、尝试重连

4. ** malformed UPDATE 测试**
   - 测试内容：发送缺少必须属性的 UPDATE
   - 验证点：发送 NOTIFICATION、拒绝处理

5. **环路检测测试**
   - 测试内容：AS_PATH 包含本地 AS
   - 验证点：拒绝路由、发送 NOTIFICATION

### 9.4 策略测试

1. **入站策略测试**
   - 测试内容：配置入站过滤规则
   - 验证点：非法路由被拒绝

2. **出站策略测试**
   - 测试内容：配置出站过滤规则
   - 验证点：只发送允许的路由

3. **Local Pref 测试**
   - 测试内容：IBGP 路由设置不同 Local Pref
   - 验证点：优先选择高 Local Pref 路由

### 9.5 性能测试

1. **路由收敛测试**
   - 测试内容：大规模路由变化的收敛时间
   - 验证点：收敛时间在可接受范围内

2. **UPDATE 速率测试**
   - 测试内容：高速发送 UPDATE
   - 验证点：不丢包、处理正常

---

## 10. 参考资料

1. **RFC 4271** - A Border Gateway Protocol 4 (BGP-4)
2. **RFC 4760** - Multiprotocol Extensions for BGP-4 (MP-BGP)
3. **RFC 2918** - Route Refresh Capability for BGP-4
4. **RFC 4456** - BGP Route Reflection
5. **RFC 1997** - BGP Communities Attribute
6. **RFC 2439** - BGP Route Flap Damping
7. **RFC 5082** - Generalized TTL Security Mechanism (GTSM)
8. **RFC 6793** - BGP Support for Four-Octet Autonomous System (AS) Number Space
9. **RFC 8205** - BGPsec Protocol Specification
10. **RFC 6811** - BGP Prefix Origin Validation
11. **RFC 6480** - An Infrastructure to Support Secure Internet BGP
12. **RFC 2385** - Protection of BGP Sessions via TCP MD5 Signature Option
