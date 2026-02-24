# OSPF（Open Shortest Path First）协议详细设计文档

## 1. 协议概述

### 1.1 背景与历史

**协议全称和定义：**
- OSPF（Open Shortest Path First，开放最短路径优先）协议
- 基于 SPF（Shortest Path First）算法的链路状态路由协议
- 属于网络层（第3层）动态路由协议
- 核心功能：通过链路状态通告（LSA）在网络中传播拓扑信息，各路由器使用 Dijkstra 算法计算最短路径树

**为什么需要该协议？**

在早期的路由协议（如 RIP）使用距离矢量算法，存在收敛慢、计数到无穷等问题。OSPF 解决了以下问题：

1. **快速收敛**：当网络拓扑变化时，OSPF 能快速传播变化并重新计算路由
2. **无环路**：SPF 算法保证计算出的路由无环路
3. **支持大规模网络**：通过区域（Area）划分支持大型网络
4. **等价负载均衡**：支持多条等价路径的负载分担
5. **VLSM 和 CIDR 支持**：支持可变长子网掩码和无类域间路由

**历史背景：**
- **RFC 1131 (1989)**：OSPF 规范的第一个版本（OSPF 版本 1）
- **RFC 1247 (1991)**：OSPF 版本 2
- **RFC 1583 (1994)**：OSPF 版本 2 修订版
- **RFC 2178 (1997)**：OSPF 版本 2 进一步修订
- **RFC 2328 (1998)**：OSPF 版本 2 当前标准（STD 54），由 John Moy 编写
- **RFC 2740 (1999)**：OSPF 版本 3，支持 IPv6
- **RFC 5340 (2008)**：OSPFv3 更新标准

相关补充 RFC：
- RFC 5709：OSPFv2 加密算法更新
- RFC 6845：OSPF Stub Router 通告
- RFC 6860：纯传输网络隐藏模型
- RFC 7474：OSPF 反重放攻击安全模型
- RFC 5187：OSPFv3 优雅重启
- RFC 6506：OSPFv3 认证拖车支持

### 1.2 设计原理

OSPF 是基于**链路状态（Link-State）**算法的内部网关协议（IGP）。其核心设计思想是：

1. **链路状态数据库同步**：每个 OSPF 路由器维护一个相同的链路状态数据库（LSDB），描述整个自治系统的拓扑结构
2. **SPF 算法计算路由**：每个路由器基于 LSDB 运行 Dijkstra SPF 算法，以自己为根计算最短路径树
3. **分层区域设计**：自治系统可划分为多个区域，减少路由协议流量
4. **邻居发现与维护**：通过 Hello 报文发现邻居并维持邻接关系

**OSPF 网络拓扑示例：**

```
                    +-------------+
                    |   Area 0    |
                    |  (Backbone) |
                    |             |
      +-------------+----+----+---+-------------+
      |                  |    |                 |
  +---+---+          +---+--+ +-+---+         +---+---+
  | RT A   |          | RT B | | RT C|         | RT D  |
  | ABR    |          |      | |     |         |       |
  +---+---+          +------+------+         +---+---+
      |                  |    |                 |
      |            +-----+----+---+-----+        |
      |            |     Area 1       |        |
      |            |  (Non-backbone)  |        |
      |            +------------------+        |
      |                  |    |                 |
  +---+---+          +---+--+ +-+---+         +---+---+
  | RT E  |          | RT F | | RT G |         | RT H |
  +-------+          +------+------+         +-------+

  ABR: Area Border Router (区域边界路由器)
  Area 0: 骨干区域，必须连续且连接所有其他区域
```

**链路状态数据库与 SPF 计算：**

```
Router A 的 LSDB（链路状态数据库）:
┌─────────────────────────────────────────────────┐
│ LSA Type 1 (Router LSA) from Router A          │
│   - Link to Router B, Cost 10                  │
│   - Link to Network 192.168.1.0/24, Cost 1     │
├─────────────────────────────────────────────────┤
│ LSA Type 1 (Router LSA) from Router B          │
│   - Link to Router A, Cost 10                  │
│   - Link to Router C, Cost 5                   │
├─────────────────────────────────────────────────┤
│ LSA Type 2 (Network LSA) from DR (Router C)    │
│   - Network 10.0.0.0/24, Attached Routers:     │
│     Router B, Router C, Router D               │
└─────────────────────────────────────────────────┘
                    ↓
           运行 Dijkstra SPF 算法
                    ↓
     Router A 的最短路径树（SPT）:
            ┌── A (Root)
            │
         10 │
            ↓
            ┌── B
            │
         5  │
            ↓
            ┌── C ── 10.0.0.0/24
            │
         15 │
            ↓
            ┌── D
```

**关键特点：**

1. **链路状态通告（LSA）**：路由器生成 LSA 描述本地链路状态，洪泛到整个区域
2. **SPF 算法**：每台路由器独立运行 Dijkstra 算法计算最优路由
3. **快速收敛**：拓扑变化触发 LSA 更新，路由器快速重新计算
4. **区域分层**：支持多区域设计，骨干区域（Area 0）必须连续
5. **支持 VLSM**：每个 LSA 可携带子网掩码，支持可变长子网

### 1.3 OSPFv3 (IPv6) 概述

**OSPFv3 与 OSPFv2 的主要区别：**

OSPFv3 是为 IPv6 设计的 OSPF 版本，在保留 OSPFv2 核心机制的基础上进行了以下改进：

| 特性 | OSPFv2 (IPv4) | OSPFv3 (IPv6) |
|------|---------------|---------------|
| **地址携带** | LSA 中携带 IPv4 地址 | 地址从 LSA 中移除，使用新的 LSA 类型 |
| **运行粒度** | 基于 IP 子网 | 基于链路（Link） |
| **认证** | 内置认证（Type 0/1/2） | 移除认证，依赖 IPv6 AH/ESP |
| **协议多播地址** | 224.0.0.5, 224.0.0.6 | FF02::5, FF02::6 |
| **Router ID** | 32 位 IPv4 地址格式 | 仍使用 32 位（需手动配置） |
| **LSA 洪泛范围** | 固定范围 | 使用 U/S/A2 位灵活指定 |
| **多实例支持** | 不支持 | Instance ID 字段支持 |

**OSPFv3 的设计目标：**

1. **与 IPv6 地址无关的路由**：协议核心与地址语义分离
2. **支持多个地址前缀**：单个链路可配置多个 IPv6 前缀
3. **简化协议扩展**：选项字段更灵活，便于添加新特性
4. **向后兼容性**：保持与 OSPFv2 相同的邻居建立和数据库同步机制

**OSPFv3 组播地址：**

| 地址 | 用途 |
|------|------|
| FF02::5 | AllSPFRouters - 所有 OSPF 路由器 |
| FF02::6 | AllDRouters - 所有 DR/BDR 路由器 |

---

## 2. 报文格式

### 2.1 报文结构

OSPF 报文直接封装在 IP 报文中（协议号 89），不使用 UDP 或 TCP。

**OSPF 通用报文头部：**

```
 0                   1                   2                   3
 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|   Version #   |     Type      |         Packet Length         |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|                          Router ID                            |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|                           Area ID                             |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|           Checksum            |  AuType       |               |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+               +
|                                                               |
+                         Authentication                        +
|                                                               |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
```

### 2.2 字段说明

| 字段 | 大小 | 说明 | 常见值 |
|------|------|------|--------|
| Version # | 1 字节 | OSPF 版本号 | 2 (OSPFv2), 3 (OSPFv3) |
| Type | 1 字节 | OSPF 报文类型 | 1-5 (见下表) |
| Packet Length | 2 字节 | OSPF 报文总长度（含头部） | 含认证数据 |
| Router ID | 4 字节 | 发送路由器的标识符 | 32位 IP 格式 |
| Area ID | 4 字节 | 报文所属区域 | 0.0.0.0 (骨干区域) |
| Checksum | 2 字节 | 整个报文的校验和 | - |
| AuType | 2 字节 | 认证类型 | 0(无), 1(简单), 2(加密) |
| Authentication | 8 字节 | 认证数据 | 根据 AuType |

**OSPF 报文类型：**

| Type | 名称 | 用途 |
|------|------|------|
| 1 | Hello | 发现邻居、维持邻接关系、选举 DR/BDR |
| 2 | Database Description | 数据库描述，交换 LSA 头部信息 |
| 3 | Link State Request | 请求特定 LSA |
| 4 | Link State Update | 洪泛 LSA 更新 |
| 5 | Link State Acknowledgment | LSA 确认 |

**最小/最大报文长度：**
- 最小：Hello 报文约 24 字节（无认证）或 32 字节（带认证）
- 最大：受 MTU 限制，通常为 1500 字节（标准以太网）

### 2.3 报文类型详解

#### 2.3.1 Hello 报文（Type 1）

```
 0                   1                   2                   3
 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|                        Network Mask                          |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|         HelloInterval         |    Options    |    Rtr Pri   |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|         RouterDeadInterval    |                                   |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+                               +
|                                                               |
|                     Designated Router                         |
|                                                               |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|                                                               |
|                   Backup Designated Router                    |
|                                                               |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|                                                               |
|                                                               |
|                          Neighbor                             |
|                                                               |
|                                                               |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|                              ...                              |
```

#### 2.3.2 Database Description 报文（Type 2）

```
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|   Options     |                      0                        |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|                         Interface MTU                         |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|   Options     |                      0                        |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|        Database Description Sequence Number                   |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|                                                               |
+                      LSA Header                              +
|                                                               |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|                              ...                              |
```

#### 2.3.3 Link State Request 报文（Type 3）

```
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|                                                               |
+                      LSA Type 1                              +
|                                                               |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|                     Link State ID                            |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|                     Advertising Router                       |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|                              ...                              |
```

#### 2.3.4 Link State Update 报文（Type 4）

```
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|                           # LSAs                              |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|                                                               |
+                      LSA 1                                   +
|                                                               |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|                              ...                              |
```

#### 2.3.5 Link State Acknowledgment 报文（Type 5）

```
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|                                                               |
+                      LSA Header 1                            +
|                                                               |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|                              ...                              |
```

### 2.4 LSA 头部格式

所有 LSA 类型共享统一的头部：

```
 0                   1                   2                   3
 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|            LS Age             |      Options     |    LS Type   |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|                        Link State ID                          |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|                     Advertising Router                       |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|                     LS Sequence Number                        |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|         LS Checksum             |            Length           |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
```

### 2.5 封装格式

**OSPFv2 在 IP 报文中的封装：**

```
+-------------------+
|     IP Header     |
|  Protocol = 89    |
+-------------------+
|   OSPF Header     |
+-------------------+
|   OSPF Payload    |
|   (Hello/DBD/etc) |
+-------------------+
```

**IP 头部关键字段：**
- 协议号：89（OSPF）
- TTL：通常设置为 1（限制在本地网络）
- 组播地址：224.0.0.5（AllSPFRouters）或 224.0.0.6（AllDRouters）

**OSPFv3 在 IPv6 报文中的封装：**

```
+-------------------+
|    IPv6 Header    |
|  Next Header = 89 |
+-------------------+
|   OSPFv3 Header   |
+-------------------+
|  OSPFv3 Payload   |
|  (Hello/DBD/etc)  |
+-------------------+
```

**IPv6 头部关键字段：**
- 下一个报头：89（OSPF）
- 跳数限制：通常设置为 1（限制在本地链路）
- 组播地址：FF02::5（AllSPFRouters）或 FF02::6（AllDRouters）

---

### 2.6 OSPFv3 报文格式

**OSPFv3 通用报文头部：**

```
 0                   1                   2                   3
 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|   Version #   |     Type      |         Packet Length         |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|                          Router ID                            |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|                           Area ID                             |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|           Checksum            |  Instance ID   |              |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+               +
|                                                               |
+                         (reserved)                            +
|                                                               |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
```

**OSPFv3 与 OSPFv2 头部差异：**

| 字段 | OSPFv2 | OSPFv3 | 说明 |
|------|--------|--------|------|
| Version | 2 | 3 | 版本号不同 |
| AuType | 2 字节 | 移除 | OSPFv3 不内置认证 |
| Authentication | 8 字节 | 移除 | 依赖 IPv6 AH/ESP |
| Instance ID | 不存在 | 1 字节 | 支持多实例 |
| Reserved | 不存在 | 3 字节 | 保留字段 |

**Instance ID 字段：**
- 允许在同一条链路上运行多个 OSPFv3 实例
- 值为 0 表示主实例
- 非 0 值用于区分不同实例

### 2.7 OSPFv3 Hello 报文格式

```
 0                   1                   2                   3
 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|         Interface ID          |         HelloInterval         |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|    Options    |      Rtr Pri  |         RouterDeadInterval    |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|                                                               |
|                     Designated Router                         |
|                                                               |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|                                                               |
|                   Backup Designated Router                    |
|                                                               |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|                                                               |
|                          Neighbor                             |
|                                                               |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|                              ...                              |
```

**OSPFv3 Hello 与 OSPFv2 Hello 差异：**

| 字段 | OSPFv2 | OSPFv3 | 说明 |
|------|--------|--------|------|
| Network Mask | 4 字节 | 移除 | IPv6 不使用网络掩码 |
| Interface ID | 不存在 | 4 字节 | 接口索引标识 |
| Options | 1 字节 | 1 字节 | 选项位含义不同 |

**OSPFv3 Options 位：**

```
+--+--+--+--+--+--+--+--+
|DC|R|N|MC|E|V6|       |
+--+--+--+--+--+--+--+--+
| | | | |  |  |
| | | | |  |  +-- Unused (must be 0)
| | | | |  +----- V6 - Unknown forwarding capability
| | | | +-------- E - External routes advertised (ASBR)
| | | +---------- MC - Multicast capability
| | +------------ N - Whether this router is attached to an NSSA
| +-------------- R - Router flags (ASBR/ABR indicator)
+---------------- DC - Demand Circuits support
```

### 2.8 OSPFv3 LSA 类型

**OSPFv3 LSA 类型与功能映射：**

| LSA 类型 | OSPFv2 | OSPFv3 | 功能描述 | 洪泛范围 |
|----------|--------|--------|----------|----------|
| Router-LSA | Type-1 | Type-0x2001 | 路由器拓扑信息 | 区域内 |
| Network-LSA | Type-2 | Type-0x2002 | 网络拓扑信息 | 区域内 |
| Inter-Area-Prefix-LSA | Type-3 Summary | Type-0x2003 | 区域间 IPv6 前缀 | 区域内 |
| Inter-Area-Router-LSA | Type-4 Summary | Type-0x2004 | 区域间 ASBR 路由器 | 区域内 |
| AS-External-LSA | Type-5 | Type-0x4005 | AS 外部路由 | 全 AS |
| **Link-LSA** | 不存在 | **Type-0x0008** | 链路本地地址信息 | 本链路 |
| **Intra-Area-Prefix-LSA** | 不存在 | **Type-0x2009** | 区域内前缀信息 | 区域内 |
| NSSA-LSA | Type-7 | Type-0x2007 | NSSA 外部路由 | 区域内 |

**LSA 类型编码格式：**
- OSPFv3 LSA 类型使用 16 位编码
- 格式：`0x[U][S][A2][功能码]`
  - U 位：1 表示已知 LSA 类型，0 表示未知 LSA 类型应被洪泛
  - S 位：1 表示 LSA 具有特定洪泛范围
  - A2 位：与 S 位配合定义洪泛范围
  - 功能码：LSA 功能类型

**OSPFv3 新增 LSA 类型详解：**

#### 2.8.1 Link-LSA (Type-0x0008)

**用途：** 携带链路本地信息，洪泛范围仅限于本链路

**格式：**
```
 0                   1                   2                   3
 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|            LS Age             |     Type      |     LS ID    |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|                      Advertising Router                       |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|                     LS Sequence Number                        |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|         LS Checksum             |            Length           |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|           Priority              |            Options         |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|    Link-Local Interface Address                                 |
+                                                               +
|                                                               |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|      Number of Prefixes      |                                   |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+                               +
|                                                               |
|                         Prefix 1                              |
|                                                               |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|                              ...                              |
```

**关键字段：**
- Priority：路由器优先级（用于 DR/BDR 选举）
- Options：路由器能力选项
- Link-Local Interface Address：链路本地地址
- Prefixes：该链路关联的 IPv6 前缀列表

#### 2.8.2 Intra-Area-Prefix-LSA (Type-0x2009)

**用途：** 将 IPv6 前缀关联到 Router-LSA 或 Network-LSA

**格式：**
```
 0                   1                   2                   3
 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|            LS Age             |     Type      |     LS ID    |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|                      Advertising Router                       |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|                     LS Sequence Number                        |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|         LS Checksum             |            Length           |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|      Number of Prefixes      |      Referenced LS Type       |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|                     Referenced Link State ID                  |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|                                                               |
|                         Prefix 1                              |
|                                                               |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|                              ...                              |
```

**关键字段：**
- Referenced LS Type：关联的 LSA 类型（Router-LSA 或 Network-LSA）
- Referenced Link State ID：关联的 LSA ID
- Prefixes：IPv6 前缀列表及其度量值

**前缀格式：**
```
 0                   1                   2                   3
 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|   PrefixLen   |      Options |           Reserved            |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|                           Prefix                              |
+                      (variable length)                        +
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
```

### 2.9 OSPFv3 Router-LSA 格式

**OSPFv3 Router-LSA (Type-0x2001)：**

```
 0                   1                   2                   3
 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|            LS Age             |     Type      |     LS ID    |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|                      Advertising Router                       |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|                     LS Sequence Number                        |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|         LS Checksum             |            Length           |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|     Options    |          # of links         |                |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+                +
|                                                               |
~                        Link 1                                 ~
|                                                               |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|                              ...                              |
```

**OSPFv3 Router-LSA 与 OSPFv2 Router-LSA 差异：**

| 字段 | OSPFv2 | OSPFv3 |
|------|--------|--------|
| 选项位 | 每个 LSA 单独 | 放在 LSA 开头 |
| 链路 ID | IPv4 地址 | 邻居接口 ID |
| 链路数据 | 接口 IP 地址 | 接口索引 |
| 地址前缀 | 包含在 LSA 中 | 移到 Intra-Area-Prefix-LSA |

**OSPFv3 链路格式：**

```
 0                   1                   2                   3
 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|                          Neighbor Interface ID                |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|                          Neighbor Router ID                   |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|      Type     |     Metric    |                                 |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+                                 +
|                                                               |
~                        Optional Sub-TLVs                      ~
|                                                               |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
```

**链路类型（Type）：**
- 1：点对点链路
- 2：跨网段链路
- 3：虚链路
- 4： transit 网络

---

## 3. 状态机设计

### 3.0 状态变量

OSPF 协议维护的状态变量：

**接口状态变量：**

| 变量名称 | 类型 | 用途 | 初始值 |
|---------|------|------|--------|
| IfState | InterfaceState | 接口的 OSPF 状态 | Down |
| HelloTimer | Timer | 发送 Hello 报文的定时器 | 随机启动 |
| WaitTimer | Timer | 等待 DR/BDR 选举完成 | 默认 RouterDeadInterval |
| RxInterval | u32 | 接收 Hello 报文的间隔 | 动态更新 |

**邻居状态变量：**

| 变量名称 | 类型 | 用途 | 初始值 |
|---------|------|------|--------|
| NeighborState | NeighborState | 邻居的 OSPF 状态 | Down |
| InactivityTimer | Timer | 邻居不活动检测定时器 | RouterDeadInterval |
| DDSeqNumber | u32 | Database Description 序列号 | 初始随机值 |

**路由器状态变量：**

| 变量名称 | 类型 | 用途 | 初始值 |
|---------|------|------|--------|
| RouterId | Ipv4Addr | 路由器唯一标识符 | 配置或自动选择 |
| AreaId | Ipv4Addr | 所属区域 ID | 0.0.0.0（骨干） |
| LsaSequenceNumber | u32 | LSA 序列号 | 0x80000001 |

### 3.1 接口状态机

**接口状态定义：**

```
                    +-------------------------+
                    |         Down            |
                    +-------------------------+
                               |
                        Interface Up
                               |
                               v
                    +-------------------------+
                    |      Loopback           |
                    +-------------------------+
                               |
                        (Loopback interface)
                               |
                     +---------+---------+
                     |                   |
                     v                   v
            +----------------+   +----------------+
            |    Waiting     |   |     Point-to-  |
            |                |   |     Point      |
            +----------------+   +----------------+
                     |                   |
                Wait Timer           (P2P network)
                     |                   |
                     v                   v
            +----------------+   +----------------+
            |     DR Other   |   |  Point-to-Point|
            +----------------+   +----------------+
                     |                   |
                     |                   |
                     +-----> [DR] <------+
                            |
                            v
                     +----------------+
                     |       Backup    |
                     +----------------+
```

**接口状态详解：**

| 状态 | 说明 |
|------|------|
| Down | 接口未启用 OSPF 或物理链路断开 |
| Loopback | 接口是环回接口，不发送 Hello |
| Waiting | 等待 DR/BDR 选举 |
| Point-to-Point | 点到点链路，不需要选举 DR |
| DR Other | 非 DR/BDR 路由器 |
| DR | 指定路由器（Designated Router） |
| Backup | 备份指定路由器 |

### 3.2 邻居状态机

**邻居状态定义：**

```
                    +-------------------------+
                    |         Down            |
                    +-------------------------+
                               |
                        Receive Hello
                               |
                               v
                    +-------------------------+
                    |      Attempt            |
                    +-------------------------+
                               |
                        Hello Received
                               |
                               v
                    +-------------------------+
                    |        Init             |
                    +-------------------------+
                               |
                    Receive Hello with self
                    in Neighbor List (2-Way)
                               |
                     +---------+---------+
                     |                   |
                     v                   v
            +----------------+   +----------------+
            |      2-Way     |   |    ExStart     |
            +----------------+   +----------------+
                     |                   |
                     |         Negotiate
                     |         Master/Slave
                     |                   |
                     |                   v
                     |            +----------------+
                     |            |     Exchange    |
                     |            +----------------+
                     |                   |
                     |            Exchange Complete
                     |                   |
                     |                   v
                     |            +----------------+
                     |            |     Loading     |
                     |            +----------------+
                     |                   |
                     |            Loading Complete
                     |                   |
                     +-----> [Full] <----+
```

**邻居状态详解：**

| 状态 | 说明 |
|------|------|
| Down | 邻居状态未知，未通信 |
| Attempt | 在 NBMA 网络上尝试联系邻居 |
| Init | 收到 Hello，但双向通信未建立 |
| 2-Way | 双向通信已建立，可决定是否建立邻接关系 |
| ExStart | 确定 Master/Slave 关系和初始 DD 序列号 |
| Exchange | 交换数据库描述报文 |
| Loading | 请求并接收缺失的 LSA |
| Full | 邻接关系完全建立，数据库同步完成 |

### 3.3 状态转换详解

#### 3.3.1 接口状态转换

##### **Down → Waiting**

**进入条件：**
- 接口启用 OSPF
- 接口类型是广播或多路访问网络

**行为：**
- 启动 HelloTimer，按 HelloInterval 发送 Hello 报文
- 启动 WaitTimer，等待 RouterDeadInterval

**转换条件：**
- WaitTimer 超时 → 进入 DR Other 状态（选举自己为 DR/BDR）
- 收到优先级更高的 Hello → 转移到 DR Other 或 Backup 状态

#### 3.3.2 邻居状态转换

##### **Down → Init**

**进入条件：**
- 收到邻居的 Hello 报文

**行为：**
- 创建邻居状态机
- 记录邻居的 Router ID

**转换条件：**
- 收到包含自己 Router ID 的 Hello → 进入 2-Way 状态

##### **2-Way → ExStart**

**进入条件：**
- 决定与邻居建立邻接关系（DR 与 BDR 之间，或点对点链路）

**行为：**
- 生成空的 Database Description 报文
- 设置 I（Initialize）位，M（More）位，MS（Master）位
- 初始化 DD 序列号

**转换条件：**
- 完成 Master/Slave 协商 → 进入 Exchange 状态

##### **Exchange → Loading**

**进入条件：**
- Database Description 交换完成
- 收到带有 M=0 的 DD 报文

**行为：**
- 发送 Link State Request 报文，请求缺失的 LSA

**转换条件：**
- 收到所有请求的 LSA → 进入 Full 状态

**相关资源：**
- 涉及的表项：Link State Retransmission List
- 涉及的定时器：RxmtInterval（重传定时器）

---

## 4. 报文处理逻辑

### 4.0 定时器

OSPF 协议使用的定时器：

| 定时器名称 | 启动条件 | 超时时间 | 超时动作 |
|-----------|---------|---------|---------|
| HelloTimer | 接口启动 OSPF | HelloInterval (默认 10s) | 发送 Hello 报文 |
| WaitTimer | 接口进入 Waiting 状态 | RouterDeadInterval (默认 40s) | 结束 DR/BDR 选举等待 |
| InactivityTimer | 收到邻居 Hello | RouterDeadInterval | 邻居状态转为 Down |
| RxmtTimer | 发送需可靠传输的 LSA | RxmtInterval (默认 5s) | 重传 LSA |
| PollTimer | NBMA 网络手动轮询 | PollInterval (默认 120s) | 向邻居发送 Hello |

### 4.1 接收处理总流程

```
                收到 OSPF 报文
                       |
                       v
            +----------------------+
            |    版本/区域验证     |
            +----------------------+
                       |
                       v
            +----------------------+
            |    认证检查 (可选)   |
            +----------------------+
                       |
                       v
            +----------------------+
            |    报文类型分发      |
            +----------------------+
                       |
        +--------------+--------------+--------------+--------------+
        |              |              |              |              |
        v              v              v              v              v
    +-------+    +-------+     +-------+       +-------+       +-------+
    | Hello |    |  DD   |     |  LSR  |       |  LSU  |       |  LSAck|
    +-------+    +-------+     +-------+       +-------+       +-------+
        |              |              |              |              |
        v              v              v              v              v
   邻居状态      数据库同步      请求LSA      处理LSA更新      LSA确认
   维护/选举
```

### 4.2 Hello 报文处理

**处理流程：**

1. **提取信息：**
   - Router ID → 邻居标识符
   - HelloInterval / DeadInterval → 参数验证
   - Options → 能力协商
   - Priority → DR/BDR 选举
   - DR / BDR → 网络指定路由器
   - Neighbor List → 检测双向通信

2. **处理步骤：**
   - 检查 HelloInterval 和 DeadInterval 是否匹配本地配置
   - 检查 Area ID 是否匹配
   - 检查认证参数（如果启用）
   - 检查邻居列表中是否包含自己的 Router ID

3. **资源更新：**
   - 邻居表：查找或创建邻居条目
   - InactivityTimer：重置为 RouterDeadInterval
   - 邻居状态：Down → Init 或 Init → 2-Way

4. **响应动作：**
   - 如果满足 DR/BDR 选举条件，执行选举算法
   - 如果邻居是新的，在下一个 HelloInterval 加入邻居列表

**DR/BDR 选举算法：**

```
FOR each eligible router (Priority > 0) on network:
    IF router's Priority > current DR.Priority:
        new DR = this router
    ELSE IF router's Priority == current DR.Priority:
        IF router's RouterID > current DR.RouterID:
            new DR = this router

FOR each eligible router (Priority > 0) excluding DR:
    IF router's Priority > current BDR.Priority:
        new BDR = this router
    ELSE IF router's Priority == current BDR.Priority:
        IF router's RouterID > current BDR.RouterID:
            new BDR = this router
```

### 4.3 Database Description 报文处理

**处理流程：**

1. **提取信息：**
   - Interface MTU → MTU 检查
   - Options → 能力协商
   - I/M/MS 位 → 确定报文角色
   - DD Sequence Number → 可靠传输
   - LSA Headers → 数据库摘要

2. **处理步骤：**
   - 检查 Interface MTU 是否匹配（如果不匹配，丢弃报文）
   - 检查序列号，确保可靠传输
   - 根据 MS 位确定 Master 或 Slave 角色
   - 比较 LSA 头部，确定需要请求的 LSA

3. **资源更新：**
   - 邻居状态：ExStart → Exchange 或 Exchange → Loading
   - Link State Request List：添加需要请求的 LSA
   - Link State Retransmission List：跟踪发送的 DD 报文

4. **响应动作：**
   - 发送包含 LSA 头部的 DD 报文
   - 如果完成交换，发送 M=0 的 DD 报文

### 4.4 Link State Request 报文处理

**处理流程：**

1. **提取信息：**
   - LSA Type → 请求的 LSA 类型
   - Link State ID → LSA 标识符
   - Advertising Router → 生成 LSA 的路由器

2. **处理步骤：**
   - 检查每个请求的 LSA 是否在本地数据库中
   - 验证 LSA 的年龄是否有效（不是 MaxAge）

3. **资源更新：**
   - 无状态更新

4. **响应动作：**
   - 发送 Link State Update 报文，包含请求的 LSA

### 4.5 Link State Update 报文处理

**处理流程：**

1. **提取信息：**
   - # LSAs → LSA 数量
   - 每个 LSA 的完整内容

2. **处理步骤：**
   - 对每个 LSA 执行以下检查：
     - 验证 LSA 校验和
     - 检查 LSA 年龄
     - 比较序列号，确定是否更新
   - 如果 LSA 更新，洪泛到其他接口
   - 更新链路状态数据库

3. **资源更新：**
   - 链路状态数据库：添加/更新 LSA
   - LSA 序列号：递增（如果本地生成）
   - SPF 计算标志：设置，触发路由重计算

4. **响应动作：**
   - 发送 Link State Acknowledgment 报文（直接确认或延迟确认）
   - 如果 LSA 变化，触发 SPF 计算
   - 洪泛 LSA 到其他邻居（除了发送者）

**LSA 处理逻辑：**

```
FOR each LSA in received LSU:
    IF LSA not in database OR
       LSA.seq > database_LSA.seq OR
       (LSA.seq == database_LSA.seq AND LSA.checksum != database_LSA.checksum):
        // 需要更新
        replace LSA in database
        flood LSA to other interfaces
        send acknowledgment
        schedule SPF calculation
    ELSE:
        // 不需要更新
        send acknowledgment
```

### 4.6 Link State Acknowledgment 报文处理

**处理流程：**

1. **提取信息：**
   - 确认的 LSA 头部列表

2. **处理步骤：**
   - 查找 Link State Retransmission List 中对应的 LSA
   - 从重传列表中移除已确认的 LSA

3. **资源更新：**
   - Link State Retransmission List：移除已确认的 LSA
   - RxmtTimer：停止对应 LSA 的重传定时器

4. **响应动作：**
   - 无

---

## 5. 核心数据结构

### 5.0 表项/缓存

OSPF 协议维护的表项和缓存：

| 资源名称 | 用途 | 最大容量 | 淘汰策略 |
|---------|------|---------|---------|
| 邻居表 | 维护邻居状态和参数 | 无限（受内存限制） | InactivityTimer 超时 |
| 链路状态数据库 | 存储所有 LSA | 无限（受内存限制） | LS Age 达到 MaxAge |
| 邻接关系数据库 | 维护完全邻接的邻居 | 无限 | 邻居状态转为 Down 时删除 |
| 路由表 | SPF 计算结果 | 无限 | LSA 更新后重新计算 |

#### 5.0.1 邻居表（Neighbor Table）

**用途：** 维护所有检测到的 OSPF 邻居的状态和参数

**关键操作：**
- 查询：Router ID
- 添加：收到新的 Hello 报文
- 更新：收到 Hello 报文时刷新 InactivityTimer
- 删除：InactivityTimer 超时或接口 Down

#### 5.0.2 链路状态数据库（LSDB）

**用途：** 存储区域内所有 LSA，用于 SPF 计算

**关键操作：**
- 查询：LS Type + Link State ID + Advertising Router
- 添加：收到新的 LSA
- 更新：收到序列号更高的 LSA
- 删除：LS Age 达到 MaxAge（3600 秒）后刷新

#### 5.0.3 路由表（Routing Table）

**用途：** 存储 SPF 计算生成的路由

**关键操作：**
- 添加：SPF 计算发现新路由
- 更新：LSA 变化触发重新计算
- 删除：路由失效

### 5.1 报文结构

**OSPFv2 报文结构：**

```rust
/// OSPF 通用报文头部
#[derive(Debug, Clone)]
pub struct OspfHeader {
    /// OSPF 版本号 (2 for OSPFv2, 3 for OSPFv3)
    pub version: u8,
    /// 报文类型 (1-5)
    pub packet_type: OspfType,
    /// 报文总长度（含头部）
    pub length: u16,
    /// 路由器 ID
    pub router_id: Ipv4Addr,
    /// 区域 ID
    pub area_id: Ipv4Addr,
    /// 校验和
    pub checksum: u16,
    /// 认证类型
    pub auth_type: u16,
    /// 认证数据
    pub authentication: u64,
}

/// OSPF 报文类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OspfType {
    Hello = 1,
    DatabaseDescription = 2,
    LinkStateRequest = 3,
    LinkStateUpdate = 4,
    LinkStateAck = 5,
}

/// Hello 报文 (OSPFv2)
#[derive(Debug, Clone)]
pub struct OspfHello {
    /// 网络掩码
    pub network_mask: Ipv4Addr,
    /// Hello 发送间隔（秒）
    pub hello_interval: u16,
    /// 选项位
    pub options: u8,
    /// 路由器优先级（用于 DR/BDR 选举）
    pub router_priority: u8,
    /// 路由器死亡间隔（秒）
    pub router_dead_interval: u32,
    /// 指定路由器
    pub designated_router: Ipv4Addr,
    /// 备份指定路由器
    pub backup_designated_router: Ipv4Addr,
    /// 邻居列表
    pub neighbors: Vec<Ipv4Addr>,
}
```

**OSPFv3 报文结构：**

```rust
/// OSPFv3 通用报文头部
#[derive(Debug, Clone)]
pub struct OspfV3Header {
    /// OSPF 版本号 (3)
    pub version: u8,
    /// 报文类型 (1-5)
    pub packet_type: OspfType,
    /// 报文总长度（含头部）
    pub length: u16,
    /// 路由器 ID (仍为 32 位)
    pub router_id: Ipv4Addr,
    /// 区域 ID
    pub area_id: Ipv4Addr,
    /// 校验和
    pub checksum: u16,
    /// 实例 ID (支持多实例)
    pub instance_id: u8,
    /// 保留字段
    pub reserved: u32,
}

/// Hello 报文 (OSPFv3)
#[derive(Debug, Clone)]
pub struct OspfV3Hello {
    /// 接口 ID
    pub interface_id: u32,
    /// Hello 发送间隔（秒）
    pub hello_interval: u16,
    /// 选项位
    pub options: u16,
    /// 路由器优先级（用于 DR/BDR 选举）
    pub router_priority: u8,
    /// 路由器死亡间隔（秒）
    pub router_dead_interval: u32,
    /// 指定路由器
    pub designated_router: Ipv4Addr,
    /// 备份指定路由器
    pub backup_designated_router: Ipv4Addr,
    /// 邻居 Router ID 列表
    pub neighbors: Vec<Ipv4Addr>,
}

/// OSPFv3 选项位
#[derive(Debug, Clone, Copy)]
pub struct OspfV3Options {
    /// 支持 Demand Circuit
    pub dc: bool,
    /// 路由器标志（ASBR/ABR 指示）
    pub r: bool,
    /// 是否连接到 NSSA
    pub n: bool,
    /// 组播能力
    pub mc: bool,
    /// 外部路由能力（ASBR）
    pub e: bool,
    /// 未知转发能力
    pub v6: bool,
}

/// IPv6 前缀
#[derive(Debug, Clone)]
pub struct Ipv6Prefix {
    /// 前缀长度 (0-128)
    pub prefix_length: u8,
    /// 前缀选项
    pub options: u8,
    /// 前缀地址
    pub prefix: Ipv6Addr,
}
```

### 5.2 LSA 结构

**OSPFv2 LSA 结构：**

```rust
/// LSA 头部
#[derive(Debug, Clone)]
pub struct LsaHeader {
    /// LSA 年龄（秒）
    pub age: u16,
    /// 选项位
    pub options: u8,
    /// LSA 类型 (1-11)
    pub lsa_type: u8,
    /// 链路状态 ID
    pub link_state_id: Ipv4Addr,
    /// 通告路由器
    pub advertising_router: Ipv4Addr,
    /// LSA 序列号
    pub sequence_number: u32,
    /// LSA 校验和
    pub checksum: u16,
    /// LSA 长度（含头部）
    pub length: u16,
}

/// LSA 类型 (OSPFv2)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LsaType {
    RouterLsa = 1,              // 路由器 LSA
    NetworkLsa = 2,             // 网络 LSA
    SummaryNetworkLsa = 3,      // 网络汇总 LSA
    SummaryAsbrLsa = 4,         // ASBR 汇总 LSA
    ASExternalLsa = 5,          // AS 外部 LSA
    // GroupMembershipLsa = 6,  // 组成员 LSA (MOSPF, 已废弃)
    Type7Lsa = 7,               // NSSA LSA
    // ... 其他类型
}

/// 路由器 LSA (Type-1)
#[derive(Debug, Clone)]
pub struct RouterLsa {
    pub header: LsaHeader,
    /// 选项位
    pub options: u8,
    /// 链路数量
    pub link_count: u16,
    /// 链路列表
    pub links: Vec<RouterLink>,
}

/// 路由器 LSA 中的链路
#[derive(Debug, Clone)]
pub struct RouterLink {
    /// 链路 ID
    pub link_id: Ipv4Addr,
    /// 链路数据
    pub link_data: Ipv4Addr,
    /// 链路类型 (1-4)
    pub link_type: u8,
    /// 度量值 (cost)
    pub metric: u16,
}
```

**OSPFv3 LSA 结构：**

```rust
/// OSPFv3 LSA 头部
#[derive(Debug, Clone)]
pub struct LsaV3Header {
    /// LSA 年龄（秒）
    pub age: u16,
    /// LSA 类型 (16 位)
    pub lsa_type: u16,
    /// 链路状态 ID
    pub link_state_id: Ipv4Addr,
    /// 通告路由器
    pub advertising_router: Ipv4Addr,
    /// LSA 序列号
    pub sequence_number: u32,
    /// LSA 校验和
    pub checksum: u16,
    /// LSA 长度（含头部）
    pub length: u16,
}

/// OSPFv3 LSA 类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LsaV3Type {
    RouterLsa = 0x2001,                 // 路由器 LSA
    NetworkLsa = 0x2002,                // 网络 LSA
    InterAreaPrefixLsa = 0x2003,        // 区域间前缀 LSA
    InterAreaRouterLsa = 0x2004,        // 区域间路由器 LSA
    ASExternalLsa = 0x4005,             // AS 外部 LSA
    Type7Lsa = 0x2007,                  // NSSA LSA
    LinkLsa = 0x0008,                   // 链路 LSA (OSPFv3 新增)
    IntraAreaPrefixLsa = 0x2009,        // 区域内前缀 LSA (OSPFv3 新增)
}

/// OSPFv3 路由器 LSA (Type-0x2001)
#[derive(Debug, Clone)]
pub struct RouterLsaV3 {
    pub header: LsaV3Header,
    /// 选项位
    pub options: u16,
    /// 链路列表
    pub links: Vec<RouterLinkV3>,
}

/// OSPFv3 链路
#[derive(Debug, Clone)]
pub struct RouterLinkV3 {
    /// 邻居接口 ID
    pub neighbor_interface_id: u32,
    /// 邻居路由器 ID
    pub neighbor_router_id: Ipv4Addr,
    /// 链路类型 (1-4)
    pub link_type: u8,
    /// 度量值 (cost)
    pub metric: u16,
}

/// OSPFv3 Link-LSA (Type-0x0008) - 新增 LSA 类型
#[derive(Debug, Clone)]
pub struct LinkLsa {
    pub header: LsaV3Header,
    /// 路由器优先级
    pub priority: u8,
    /// 选项位
    pub options: u16,
    /// 链路本地接口地址
    pub link_local_addr: Ipv6Addr,
    /// 关联的 IPv6 前缀列表
    pub prefixes: Vec<Ipv6Prefix>,
}

/// OSPFv3 Intra-Area-Prefix-LSA (Type-0x2009) - 新增 LSA 类型
#[derive(Debug, Clone)]
pub struct IntraAreaPrefixLsa {
    pub header: LsaV3Header,
    /// 前缀数量
    pub num_prefixes: u16,
    /// 关联的 LSA 类型
    pub referenced_ls_type: u16,
    /// 关联的 LSA ID
    pub referenced_link_state_id: Ipv4Addr,
    /// IPv6 前缀列表
    pub prefixes: Vec<PrefixWithMetric>,
}

/// 带度量值的前缀
#[derive(Debug, Clone)]
pub struct PrefixWithMetric {
    /// IPv6 前缀
    pub prefix: Ipv6Prefix,
    /// 度量值 (cost)
    pub metric: u16,
}
```

### 5.3 接口与邻居数据结构

```rust
/// OSPF 接口状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InterfaceState {
    Down,
    Loopback,
    Waiting,
    PointToPoint,
    DR Other,
    DR,
    Backup,
}

/// OSPF 邻居状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NeighborState {
    Down,
    Attempt,
    Init,
    TwoWay,
    ExStart,
    Exchange,
    Loading,
    Full,
}

/// OSPF 接口
#[derive(Debug, Clone)]
pub struct OspfInterface {
    /// 接口名称
    pub name: String,
    /// 接口状态
    pub state: InterfaceState,
    /// 接口 IP 地址
    pub ip_addr: Ipv4Addr,
    /// 接口掩码
    pub mask: Ipv4Addr,
    /// 区域 ID
    pub area_id: Ipv4Addr,
    /// 接口类型 (Broadcast, PointToPoint, NBMA)
    pub if_type: InterfaceType,
    /// Hello 间隔
    pub hello_interval: u16,
    /// 路由器死亡间隔
    pub dead_interval: u32,
    /// 路由器优先级
    pub priority: u8,
    /// 指定路由器
    pub dr: Ipv4Addr,
    /// 备份指定路由器
    pub bdr: Ipv4Addr,
    /// 接口 Cost
    pub cost: u32,
}

/// OSPF 邻居
#[derive(Debug, Clone)]
pub struct OspfNeighbor {
    /// 邻居路由器 ID
    pub router_id: Ipv4Addr,
    /// 邻居状态
    pub state: NeighborState,
    /// 邻居 IP 地址
    pub ip_addr: Ipv4Addr,
    /// 邻居优先级
    pub priority: u8,
    /// 邻居的 DR
    pub dr: Ipv4Addr,
    /// 邻居的 BDR
    pub bdr: Ipv4Addr,
    /// Database Description 序列号
    pub dd_seq_number: u32,
    /// 最后收到 Hello 的时间
    pub last_hello_time: Instant,
}

/// 接口类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InterfaceType {
    Broadcast,      // 广播网络（以太网）
    PointToPoint,   // 点到点网络
    NonBroadcast,   // 非广播多路访问（NBMA，如帧中继）
}
```

### 5.4 链路状态数据库

```rust
/// 链路状态数据库
#[derive(Debug, Clone)]
pub struct LinkStateDatabase {
    /// LSA 条目：键为 (LSA Type, Link State ID, Advertising Router)
    pub lsas: HashMap<(u8, Ipv4Addr, Ipv4Addr), LsaEntry>,
}

/// LSA 条目
#[derive(Debug, Clone)]
pub struct LsaEntry {
    /// LSA 头部
    pub header: LsaHeader,
    /// LSA 完整内容
    pub data: Vec<u8>,
    /// 安装时间
    pub installed_at: Instant,
    /// 是否需要洪泛
    pub need_flooding: bool,
}
```

### 5.5 SPF 计算相关

```rust
/// SPF 节点（路由器或网络）
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SpfNode {
    Router(Ipv4Addr),  // 路由器 ID
    Network(Ipv4Addr), // 网络 ID (通常是 DR 的接口 IP)
}

/// SPF 顶点
#[derive(Debug, Clone)]
pub struct SpfVertex {
    pub node: SpfNode,
    pub distance: u32,        // 从根到该节点的距离
    pub parent: Option<Box<SpfVertex>>,
    pub lsa: LsaEntry,
}

/// 路由表条目
#[derive(Debug, Clone)]
pub struct RouteEntry {
    /// 目标网络
    pub destination: Ipv4Addr,
    /// 子网掩码
    pub mask: Ipv4Addr,
    /// 下一跳
    pub next_hop: Option<Ipv4Addr>,
    /// 出接口
    pub outgoing_interface: String,
    /// 路由类型
    pub route_type: RouteType,
}

/// 路由类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RouteType {
    IntraArea,     // 区域内路由
    InterArea,     // 区域间路由
    External1,     // 类型 1 外部路由（E1）
    External2,     // 类型 2 外部路由（E2）
}
```

---

## 6. 与其他模块的交互

OSPF 协议模块与 CoreNet 项目中其他模块的交互关系：

### 6.1 与 Common 模块的交互

**packet.rs (Packet)：**
- 使用 `Packet` 结构接收和发送 OSPF 报文
- OSPF 报文从 `Packet.data()` 读取，修改后通过 `Packet.data_mut()` 写入
- OSPF 报文长度受 MTU 限制

**error.rs (CoreError)：**
- 处理解析错误：`CoreError::ParseError`
- 处理无效报文：`CoreError::InvalidPacket`
- 可能需要新增 OSPF 特定错误类型

**addr.rs (MacAddr, Ipv4Addr, Ipv6Addr)：**
- 使用 `Ipv4Addr` 作为 Router ID 和 Area ID
- 使用 `Ipv4Addr` 表示目标网络和下一跳

**queue.rs (RxQ/TxQ)：**
- 从接口的 RxQ 读取接收到的 OSPF 报文
- 将 OSPF 响应报文写入 TxQ

**tables.rs (Table)：**
- 使用通用表结构实现邻居表、LSDB

**timer.rs (Timer)：**
- 实现 HelloTimer、InactivityTimer、RxmtTimer

### 6.2 与 Interface 模块的交互

**iface.rs (Interface)：**
- 读取接口 IP 地址、子网掩码、MTU
- 读取接口状态（UP/DOWN）
- OSPF 在接口启用时开始发送 Hello 报文

**manager.rs (InterfaceManager)：**
- 查询所有 OSPF 启用的接口
- 监听接口状态变化事件

### 6.3 与其他协议模块的交互

**IPv4 模块：**
- OSPFv2 报文封装在 IPv4 报文中（协议号 89）
- 设置 IP TTL = 1
- 使用组播地址：224.0.0.5 (AllSPFRouters)、224.0.0.6 (AllDRouters)

**IPv6 模块：**
- OSPFv3 报文封装在 IPv6 报文中（下一个报头 = 89）
- 设置跳数限制 = 1
- 使用组播地址：FF02::5 (AllSPFRouters)、FF02::6 (AllDRouters)

**路由模块 (route)：**
- 将 SPF 计算结果注入路由表
- 路由表用于 IP 模块的转发决策

### 6.4 与 Engine/Processor 的交互

**processor.rs (PacketProcessor)：**
- OSPF 作为 IP 层的协议处理器注册
- 当 IP 协议号为 89 时，分发到 OSPF 模块

### 6.5 与 Scheduler 的交互

**scheduler.rs (Scheduler)：**
- Scheduler 从接口 RxQ 获取报文后，分发到 OSPF 处理器
- OSPF 将响应报文放入 TxQ，由 Scheduler 发送

### 6.6 模块初始化顺序

```
1. SystemContext 初始化
   ↓
2. InterfaceManager 创建接口
   ↓
3. OspfManager 注册到 IP 协议处理器（协议号 89）
   ↓
4. OspfManager 在启用的接口上启动 HelloTimer
   ↓
5. Scheduler 开始处理报文流
```

### 6.7 数据流示例

**接收 OSPF Hello 报文：**

```
RxQ (Interface eth0)
  → Packet with IP header (Protocol=89)
  → PacketProcessor::process()
  → IPv4 module dispatches to OSPF
  → OspfManager::handle_hello()
  → 更新邻居表
  → 刷新 InactivityTimer
```

**发送 OSPF Hello 报文：**

```
HelloTimer 超时
  → OspfManager::send_hello()
  → 构建 OspfHello 报文
  → 封装到 IPv4 (Dst=224.0.0.5, TTL=1)
  → 封装到 Ethernet
  → 写入 TxQ
  → Scheduler 发送
```

**SPF 计算触发流程：**

```
收到 LSU 更新 LSA
  → OspfManager::handle_lsu()
  → 更新 LSDB
  → 设置 spf_calculation_pending = true
  → 启动 SPF 计算延迟定时器（默认 5 秒）
  → 延迟到期后执行 SPF 计算
  → 生成路由更新
  → 注入路由表
```

---

## 7. 安全考虑

### 7.1 路由伪造攻击

**攻击方式：**
- 攻击者发送伪造的 OSPF 报文，声明虚假链路
- 导致路由表被污染，流量被劫持或黑洞

**防御措施：**
- **认证机制**：OSPF 支持三种认证类型
  - Type 0：无认证（不安全）
  - Type 1：简单密码认证（明文，易被破解）
  - Type 2：加密认证（使用 MD5 或更新的 SHA 算法，RFC 5709）
- **源地址验证**：检查接收报文的源 IP 地址是否符合预期

### 7.2 Hello 报文洪泛

**攻击方式：**
- 攻击者发送大量伪造 Hello 报文
- 导致路由器创建大量邻居状态，消耗内存

**防御措施：**
- 限制每个接口的最大邻居数
- 对邻居状态设置内存上限
- 实现 Hello 报文速率限制

### 7.3 LSA 洪泛攻击

**攻击方式：**
- 攻击者频繁发送 LSA 更新
- 导致网络中大量 LSA 洪泛，消耗带宽

**防御措施：**
- **LSA 速率限制**：限制单个路由器生成 LSA 的速率
- **LSA 阈值**：当 LSA 更新超过阈值时触发告警
- **MaxAge 机制**：LSA 在 MaxAge（3600 秒）后自动失效

### 7.4 重放攻击

**攻击方式：**
- 攻击者捕获并重放有效的 OSPF 报文
- 扰乱路由计算或定时器

**防御措施：**
- **序列号**：LSA 和 DD 报文使用序列号检测重放
- **时间戳**：检查 LSA 年龄的合理性
- **认证拖车**：包含序列号，防止重放（RFC 5709, RFC 7474）

### 7.5 拒绝服务攻击

**攻击方式：**
- 攻击者发送精心构造的畸形报文
- 导致路由器崩溃或资源耗尽

**防御措施：**
- **输入验证**：严格验证所有报文字段
- **长度检查**：丢弃超过 MTU 的报文
- **速率限制**：对各种 OSPF 报文类型设置速率限制

### 7.6 实现建议

1. **强制启用认证**：默认启用 Type 2 认证，使用 HMAC-SHA 系列算法（RFC 5709）
2. **LSA 速率限制**：实现 LSA 生成速率限制，默认为每 5 秒 1 个 LSA
3. **邻居数限制**：每个接口最多支持 100 个邻居
4. **日志记录**：记录所有认证失败和异常行为
5. **配置验证**：启动时验证 OSPF 配置的合法性

---

## 8. 配置参数

### 8.1 OSPFv2 配置

```rust
/// OSPFv2 配置参数
#[derive(Debug, Clone)]
pub struct OspfConfig {
    // ========== 全局配置 ==========

    /// 路由器 ID（如果未配置，自动选择最大 IP 地址）
    pub router_id: Option<Ipv4Addr>,

    /// 是否启用 SPF 计算（用于调试）
    pub spf_enabled: bool,

    /// SPF 计算延迟时间（秒），默认 5 秒
    pub spf_delay: u32,

    /// SPF 计算最小间隔时间（秒），默认 10 秒
    pub spf_hold_time: u32,

    /// LSA 生成间隔时间（秒），默认 5 秒
    pub lsa_generation_interval: u32,

    /// 单个接口最大邻居数
    pub max_neighbors: usize,

    // ========== 接口配置 ==========

    /// OSPF 启用的接口列表
    pub interfaces: Vec<OspfInterfaceConfig>,

    // ========== 认证配置 ==========

    /// 认证类型 (0=None, 1=Simple, 2=Crypto)
    pub auth_type: u16,

    /// 简单认证密码（Type 1）
    pub auth_key: Option<String>,

    /// 加密认证配置（Type 2）
    pub crypto_auth: Option<CryptoAuthConfig>,
}

/// OSPFv2 接口配置
#[derive(Debug, Clone)]
pub struct OspfInterfaceConfig {
    /// 接口名称
    pub name: String,

    /// 区域 ID
    pub area_id: Ipv4Addr,

    /// 接口类型
    pub if_type: InterfaceType,

    /// Hello 间隔（秒），默认 10
    pub hello_interval: u16,  // 默认: 10

    /// 路由器死亡间隔（秒），默认 40
    pub dead_interval: u32,  // 默认: 40

    /// 路由器优先级（0-255），0 表示不参与 DR 选举
    pub priority: u8,  // 默认: 1

    /// 接口 Cost，默认根据带宽自动计算
    pub cost: Option<u32>,

    /// 重传间隔（秒），默认 5
    pub retransmit_interval: u32,  // 默认: 5

    /// 传输延迟（秒），默认 1
    pub transmit_delay: u32,  // 默认: 1

    /// 是否被动接口（只接收不发送）
    pub passive: bool,  // 默认: false
}

/// 加密认证配置
#[derive(Debug, Clone)]
pub struct CryptoAuthConfig {
    /// 认证算法 (HMAC-MD5, HMAC-SHA1, HMAC-SHA256 等)
    pub algorithm: AuthAlgorithm,

    /// 认证密钥 ID
    pub key_id: u8,

    /// 认证密钥
    pub key: Vec<u8>,
}

/// 认证算法
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthAlgorithm {
    HmacMd5,
    HmacSha1,
    HmacSha256,
    HmacSha384,
    HmacSha512,
}
```

### 8.2 OSPFv3 配置

```rust
/// OSPFv3 配置参数
#[derive(Debug, Clone)]
pub struct OspfV3Config {
    // ========== 全局配置 ==========

    /// 路由器 ID（必须手动配置，无 IPv4 地址时需指定）
    pub router_id: Ipv4Addr,  // OSPFv3 仍使用 32 位 Router ID

    /// 是否启用 SPF 计算
    pub spf_enabled: bool,

    /// SPF 计算延迟时间（秒），默认 5 秒
    pub spf_delay: u32,

    /// SPF 计算最小间隔时间（秒），默认 10 秒
    pub spf_hold_time: u32,

    /// LSA 生成间隔时间（秒），默认 5 秒
    pub lsa_generation_interval: u32,

    /// 单个接口最大邻居数
    pub max_neighbors: usize,

    // ========== 接口配置 ==========

    /// OSPFv3 启用的接口列表
    pub interfaces: Vec<OspfV3InterfaceConfig>,
}

/// OSPFv3 接口配置
#[derive(Debug, Clone)]
pub struct OspfV3InterfaceConfig {
    /// 接口名称
    pub name: String,

    /// 接口索引（用于 Interface ID）
    pub interface_id: u32,

    /// 区域 ID
    pub area_id: Ipv4Addr,

    /// 接口类型
    pub if_type: InterfaceType,

    /// 接口的链路本地地址
    pub link_local_addr: Ipv6Addr,

    /// 接口的全球单播地址列表
    pub global_addrs: Vec<Ipv6Addr>,

    /// Hello 间隔（秒），默认 10
    pub hello_interval: u16,  // 默认: 10

    /// 路由器死亡间隔（秒），默认 40
    pub dead_interval: u32,  // 默认: 40

    /// 路由器优先级（0-255），0 表示不参与 DR 选举
    pub priority: u8,  // 默认: 1

    /// 接口 Cost，默认根据带宽自动计算
    pub cost: Option<u32>,

    /// 重传间隔（秒），默认 5
    pub retransmit_interval: u32,  // 默认: 5

    /// 传输延迟（秒），默认 1
    pub transmit_delay: u32,  // 默认: 1

    /// 是否被动接口（只接收不发送）
    pub passive: bool,  // 默认: false

    /// Instance ID（支持多实例）
    pub instance_id: u8,  // 默认: 0
}
```

### 8.3 OSPFv2 与 OSPFv3 配置差异

| 配置项 | OSPFv2 | OSPFv3 | 说明 |
|--------|--------|--------|------|
| Router ID | 可选（自动选择） | 必须配置 | IPv6 环境无 IPv4 地址时需手动配置 |
| 地址配置 | IP + 掩码 | 链路本地 + 全球地址 | OSPFv3 支持多个 IPv6 地址 |
| 认证配置 | 内置认证 | 移除 | 依赖 IPv6 AH/ESP |
| 实例 ID | 不支持 | 支持 | OSPFv3 Instance ID |
| 接口标识 | 接口名称 | Interface ID（索引） | OSPFv3 使用数值标识 |
| 组播地址 | 224.0.0.5/6 | FF02::5/6 | IPv6 组播地址 |

---

---

## 9. 测试场景

### 9.1 基本功能测试

1. **Hello 报文交换与邻居发现**
   - 测试内容：两台 OSPF 路由器通过点对点链路连接，验证 Hello 报文交换
   - 预期结果：邻居状态从 Down → Init → 2-Way → Full

2. **DR/BDR 选举**
   - 测试内容：在广播网络上连接多台路由器，验证 DR/BDR 选举
   - 预期结果：优先级最高的路由器成为 DR，次高的成为 BDR

3. **数据库同步**
   - 测试内容：路由器启动时交换 Database Description 报文
   - 预期结果：两台路由器的 LSDB 同步

4. **LSA 洪泛**
   - 测试内容：路由器生成新的 Router LSA，验证洪泛过程
   - 预期结果：所有路由器收到 LSA 并发送确认

5. **SPF 计算与路由生成**
   - 测试内容：构建三路由器拓扑，触发 SPF 计算
   - 预期结果：生成正确的路由表条目

### 9.2 边界情况测试

1. **MTU 边界测试**
   - 测试内容：发送接近 MTU 限制的 Database Description 报文
   - 预期结果：报文正确处理，不分片

2. **最大邻居数测试**
   - 测试内容：创建超过最大邻居数的邻居连接
   - 预期结果：新邻居被拒绝或最老邻居被移除

3. **LSA 序列号回卷测试**
   - 测试内容：LSA 序列号达到 0x7FFFFFFF 后继续递增
   - 预期结果：正确处理序列号回卷（0x80000001）

4. **Router ID 冲突测试**
   - 测试内容：两台路由器使用相同的 Router ID
   - 预期结果：检测到冲突，记录错误日志

### 9.3 异常情况测试

1. **Hello 报文丢失**
   - 测试内容：模拟 Hello 报文丢失（超过 RouterDeadInterval）
   - 预期结果：邻居状态转为 Down

2. **认证失败**
   - 测试内容：发送错误认证密钥的 OSPF 报文
   - 预期结果：报文被丢弃，记录认证失败日志

3. **畸形报文处理**
   - 测试内容：发送长度错误、校验和错误的 OSPF 报文
   - 预期结果：报文被丢弃，不引起系统崩溃

4. **LSA 更新风暴**
   - 测试内容：短时间内发送大量 LSA 更新
   - 预期结果：LSA 速率限制生效，系统不崩溃

5. **区域边界路由器故障**
   - 测试内容：ABR 故障，测试区域间路由
   - 预期结果：检测到 ABR 故障，更新路由表

### 9.4 集成测试

1. **多区域 OSPF 网络**
   - 测试内容：配置 Area 0 和 Area 1，测试区域间路由
   - 预期结果：ABR 正确生成 Summary LSA，区域间路由正常

2. **OSPF over IPv6 (OSPFv3)**
   - 测试内容：在 IPv6 网络上运行 OSPFv3
   - 预期结果：邻居建立和路由计算正常

3. **路由重分发**
   - 测试内容：将静态路由和直连路由重分发到 OSPF
   - 预期结果：生成正确的外部 LSA (Type-5 或 Type-7)

---

## 10. 参考资料

### 10.1 OSPFv2 (IPv4) 标准

1. **RFC 2328** - OSPF Version 2 (OSPFv2) - [https://www.rfc-editor.org/rfc/rfc2328](https://www.rfc-editor.org/rfc/rfc2328)
2. **RFC 5709** - OSPFv2 Cryptographic Authentication - [https://www.rfc-editor.org/rfc/rfc5709](https://www.rfc-editor.org/rfc/rfc5709)
3. **RFC 6845** - OSPF Stub Router Advertisement - [https://www.rfc-editor.org/rfc/rfc6845](https://www.rfc-editor.org/rfc/rfc6845)
4. **RFC 6860** - Hiding Transit-Only Networks in OSPF - [https://www.rfc-editor.org/rfc/rfc6860](https://www.rfc-editor.org/rfc/rfc6860)
5. **RFC 7474** - OSPF Neighbor Priority-Based Hybrid BFD - [https://www.rfc-editor.org/rfc/rfc7474](https://www.rfc-editor.org/rfc/rfc7474)

### 10.2 OSPFv3 (IPv6) 标准

6. **RFC 5340** - OSPF for IPv6 (OSPFv3) - [https://www.rfc-editor.org/rfc/rfc5340](https://www.rfc-editor.org/rfc/rfc5340)
7. **RFC 2740** - OSPF for IPv6 (Original OSPFv3, Obsoleted by 5340) - [https://www.rfc-editor.org/rfc/rfc2740](https://www.rfc-editor.org/rfc/rfc2740)
8. **RFC 5187** - OSPFv3 Graceful Restart - [https://www.rfc-editor.org/rfc/rfc5187](https://www.rfc-editor.org/rfc/rfc5187)
9. **RFC 6506** - Supporting Authentication Trailer for OSPFv3 - [https://www.rfc-editor.org/rfc/rfc6506](https://www.rfc-editor.org/rfc/rfc6506)
10. **RFC 5329** - Traffic Engineering Extensions to OSPF Version 3 - [https://www.rfc-editor.org/rfc/rfc5329](https://www.rfc-editor.org/rfc/rfc5329)
11. **RFC 6969** - OSPFv3 Instance ID Registry Update - [https://www.rfc-editor.org/rfc/rfc6969](https://www.rfc-editor.org/rfc/rfc6969)
12. **RFC 7166** - Supporting Authentication Trailer for OSPFv3 (Updated) - [https://www.rfc-editor.org/rfc/rfc7166](https://www.rfc-editor.org/rfc/rfc7166)

### 10.3 历史与概述

13. **RFC 2329** - OSPF Standardization Report - [https://www.rfc-editor.org/rfc/rfc2329](https://www.rfc-editor.org/rfc/rfc2329)

### 10.4 OSPFv2 与 OSPFv3 快速对比

| 特性 | OSPFv2 | OSPFv3 |
|------|--------|--------|
| **标准 RFC** | RFC 2328 | RFC 5340 |
| **支持协议** | IPv4 | IPv6 |
| **Router ID** | 32 位 IPv4 格式 | 32 位（需手动配置） |
| **组播地址** | 224.0.0.5, 224.0.0.6 | FF02::5, FF02::6 |
| **IP 协议号** | 89 | 89 (IPv6 Next Header) |
| **认证** | 内置 (Type 0/1/2) | 移除，依赖 IPv6 AH/ESP |
| **地址携带** | LSA 中包含地址 | LSA 不含地址，用新 LSA 类型 |
| **运行粒度** | 基于 IP 子网 | 基于链路（Link） |
| **LSA 类型** | Type 1-7, 11 | Type 0x2001-0x2009 |
| **新增 LSA** | - | Link-LSA, Intra-Area-Prefix-LSA |
| **多实例** | 不支持 | Instance ID 支持 |
| **报文头部** | 含认证字段 | 含 Instance ID，移除认证 |

---

**文档版本：** v1.1
**生成日期：** 2025-02-24
**作者：** Claude Code (proto-design skill)
**适用项目：** CoreNet - 纯模拟网络协议栈
