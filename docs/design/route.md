# 路由模块设计

## 概述

路由模块（Route）负责管理系统的路由表，为IP层数据包提供路由查找功能，确定数据包的下一跳地址和出接口。路由模块是网络层转发功能的核心组件，通过最长前缀匹配（LPM）算法实现高效的路由查找。

**核心职责：**
- 维护路由表（IPv4/IPv6）
- 提供路由查找接口（最长前缀匹配）
- 支持默认路由
- 与接口管理器、ARP缓存协同工作

---

## 一、需求介绍

### 1.1 功能需求

- **需求1**：支持IPv4路由表条目（目标网络、子网掩码、网关、出接口）
- **需求2**：支持IPv6路由表条目（目标前缀、下一跳、出接口）
- **需求3**：支持默认路由（0.0.0.0/0 或 ::/0）
- **需求4**：提供最长前缀匹配（LPM）查找接口
- **需求5**：支持动态添加、删除路由条目
- **需求6**：支持路由表的查询和遍历

### 1.2 约束条件

- **零外部依赖**：仅使用 Rust 标准库
- **使用标准库数据结构**：使用 `Vec`、`HashMap` 等标准集合类型，不引入复杂的数据结构
- **纯内存模拟**：无真实路由表操作
- **线程安全**：通过 `Arc<Mutex<T>>` 封装，支持并发访问

---

## 二、架构设计

### 2.1 模块定位

路由模块位于网络层，为IP层提供路由决策服务：

```
┌─────────────────────────────────────────────────────────────────┐
│                        网络层 (Network Layer)                    │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  ┌────────────────────────────────────────────────────────────┐ │
│  │                     SystemContext                          │ │
│  │  ┌──────────────────────────────────────────────────────┐ │ │
│  │  │  Arc<Mutex<RouteTable>>  (路由表)                    │ │ │
│  │  │  Arc<Mutex<InterfaceManager>>  (网络接口)            │ │ │
│  │  │  Arc<Mutex<ArpCache>>         (ARP缓存)              │ │ │
│  │  └──────────────────────────────────────────────────────┘ │ │
│  └────────────────────────────────────────────────────────────┘ │
│                              │                                  │
│                              ▼                                  │
│  ┌────────────────────────────────────────────────────────────┐ │
│  │                     IP 层处理                              │ │
│  │  ┌─────────────┐  ┌─────────────┐  ┌────────────────────┐ │ │
│  │  │  IPv4 处理  │  │  IPv6 处理  │  │   路由查找         │ │ │
│  │  └─────────────┘  └─────────────┘  │  - LPM 算法        │ │ │
│  │                                      │  - 下一跳确定      │ │ │
│  │                                      │  - 出接口选择      │ │ │
│  │                                      └────────────────────┘ │ │
│  └────────────────────────────────────────────────────────────┘ │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

### 2.2 数据流向

```
IP数据包到达
    │
    ▼
检查目的地址
    │
    ├─────────────┐
    │             │
本机网络      远程网络
    │             │
    │             ▼
    │        路由查找
    │             │
    │             ▼
    │        最长前缀匹配
    │             │
    │             ▼
    │        ┌──────────────────┐
    │        │  获取路由信息:    │
    │        │  - 下一跳IP      │
    │        │  - 出接口        │
    │        └────────┬─────────┘
    │                 │
    ▼                 ▼
本地交付      邻居解析/转发
```

### 2.3 与其他模块的交互

```
┌───────────────────────────────────────────────────────────────────┐
│                        模块交互关系                                │
├───────────────────────────────────────────────────────────────────┤
│                                                                   │
│  ┌──────────────┐        lookup()         ┌─────────────────────┐ │
│  │ IP Module    │ ───────────────────────>│  RouteTable         │ │
│  │ (IPv4/IPv6)  │         返回路由信息      │  - ipv4_routes      │ │
│  └──────────────┘                          │  - ipv6_routes      │ │
│        ▲                                   └──────────┬──────────┘ │
│        │                                              │             │
│        │  get_interface()                             │             │
│        └──────────────────────────────────────────────┘             │
│                                                         │             │
│                                                         ▼             │
│                                              ┌─────────────────────┐  │
│                                              │  InterfaceManager   │  │
│                                              │  - 获取接口信息      │  │
│                                              └─────────────────────┘  │
│                                                         │             │
│                                                         │             │
│                                                         ▼             │
│                                              ┌─────────────────────┐  │
│                                              │  ArpCache           │  │
│                                              │  - 解析下一跳MAC    │  │
│                                              └─────────────────────┘  │
│                                                                   │
└───────────────────────────────────────────────────────────────────┘
```

---

## 三、核心数据结构

### 3.1 IPv4 路由条目

```rust
/// IPv4 路由条目
///
/// 包含目标网络、子网掩码、网关和出接口信息
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ipv4Route {
    /// 目标网络地址
    pub destination: Ipv4Addr,

    /// 子网掩码
    pub netmask: Ipv4Addr,

    /// 网关地址（None 表示直连网络）
    pub gateway: Option<Ipv4Addr>,

    /// 出接口名称
    pub interface: String,

    /// 路由优先级（管理距离，可选）
    pub metric: Option<u32>,
}

impl Ipv4Route {
    /// 计算前缀长度
    pub fn prefix_len(&self) -> u8;

    /// 判断是否为默认路由
    pub fn is_default_route(&self) -> bool;

    /// 判断目标地址是否匹配此路由
    pub fn matches(&self, addr: Ipv4Addr) -> bool;
}
```

### 3.2 IPv6 路由条目

```rust
/// IPv6 路由条目
///
/// 包含目标前缀、下一跳和出接口信息
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ipv6Route {
    /// 目标前缀
    pub destination: Ipv6Addr,

    /// 前缀长度 (0-128)
    pub prefix_len: u8,

    /// 下一跳地址（None 表示直连网络）
    pub next_hop: Option<Ipv6Addr>,

    /// 出接口名称
    pub interface: String,

    /// 路由优先级（可选）
    pub metric: Option<u32>,
}

impl Ipv6Route {
    /// 判断是否为默认路由
    pub fn is_default_route(&self) -> bool;

    /// 判断目标地址是否匹配此路由
    pub fn matches(&self, addr: Ipv6Addr) -> bool;
}
```

### 3.3 路由查找结果

```rust
/// 路由查找结果
///
/// 包含查找到的路由信息：下一跳和出接口
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RouteLookup {
    /// 下一跳地址（None 表示直连）
    pub next_hop: Option<IpAddr>,

    /// 出接口名称
    pub interface: String,

    /// 路由优先级
    pub metric: u32,
}
```

### 3.4 路由表

```rust
/// 路由表
///
/// 管理 IPv4 和 IPv6 路由条目，提供路由查找功能
#[derive(Debug)]
pub struct RouteTable {
    /// IPv4 路由列表
    ipv4_routes: Vec<Ipv4Route>,

    /// IPv6 路由列表
    ipv6_routes: Vec<Ipv6Route>,
}

impl RouteTable {
    /// 创建新的空路由表
    pub fn new() -> Self;

    /// 添加 IPv4 路由
    pub fn add_ipv4_route(&mut self, route: Ipv4Route) -> Result<(), RouteError>;

    /// 删除 IPv4 路由
    pub fn remove_ipv4_route(&mut self, destination: Ipv4Addr, netmask: Ipv4Addr) -> Result<(), RouteError>;

    /// 查找 IPv4 路由（最长前缀匹配）
    pub fn lookup_ipv4(&self, dest: Ipv4Addr) -> Option<RouteLookup>;

    /// 添加 IPv6 路由
    pub fn add_ipv6_route(&mut self, route: Ipv6Route) -> Result<(), RouteError>;

    /// 删除 IPv6 路由
    pub fn remove_ipv6_route(&mut self, destination: Ipv6Addr, prefix_len: u8) -> Result<(), RouteError>;

    /// 查找 IPv6 路由（最长前缀匹配）
    pub fn lookup_ipv6(&self, dest: Ipv6Addr) -> Option<RouteLookup>;

    /// 获取所有 IPv4 路由
    pub fn ipv4_routes(&self) -> &[Ipv4Route];

    /// 获取所有 IPv6 路由
    pub fn ipv6_routes(&self) -> &[Ipv6Route];

    /// 清空路由表
    pub fn clear(&mut self);
}
```

---

## 四、接口定义

### 4.1 IPv4 路由查找

```rust
impl RouteTable {
    /// 查找 IPv4 路由（最长前缀匹配）
    ///
    /// 根据目标地址查找最佳匹配路由，按照最长前缀匹配原则。
    ///
    /// # 参数
    ///
    /// - `dest`: 目标 IP 地址
    ///
    /// # 返回
    ///
    /// - `Some(RouteLookup)`: 找到匹配路由，包含下一跳和出接口
    /// - `None`: 没有找到匹配路由
    ///
    /// # 查找算法
    ///
    /// 1. 遍历所有 IPv4 路由条目
    /// 2. 筛选出与目标地址匹配的条目（目标地址 & 子网掩码 == 目标网络）
    /// 3. 在匹配条目中选择前缀长度最长的
    /// 4. 返回路由信息
    pub fn lookup_ipv4(&self, dest: Ipv4Addr) -> Option<RouteLookup> {
        self.ipv4_routes
            .iter()
            .filter(|route| route.matches(dest))
            .max_by_key(|route| route.prefix_len())
            .map(|route| RouteLookup {
                next_hop: route.gateway.map(IpAddr::V4),
                interface: route.interface.clone(),
                metric: route.metric.unwrap_or(0),
            })
    }
}
```

### 4.2 IPv6 路由查找

```rust
impl RouteTable {
    /// 查找 IPv6 路由（最长前缀匹配）
    ///
    /// 根据目标地址查找最佳匹配路由，按照最长前缀匹配原则。
    ///
    /// # 参数
    ///
    /// - `dest`: 目标 IPv6 地址
    ///
    /// # 返回
    ///
    /// - `Some(RouteLookup)`: 找到匹配路由，包含下一跳和出接口
    /// - `None`: 没有找到匹配路由
    ///
    /// # 查找算法
    ///
    /// 1. 遍历所有 IPv6 路由条目
    /// 2. 筛选出与目标地址匹配的条目（前缀匹配）
    /// 3. 在匹配条目中选择前缀长度最长的
    /// 4. 返回路由信息
    pub fn lookup_ipv6(&self, dest: Ipv6Addr) -> Option<RouteLookup> {
        self.ipv6_routes
            .iter()
            .filter(|route| route.matches(dest))
            .max_by_key(|route| route.prefix_len)
            .map(|route| RouteLookup {
                next_hop: route.next_hop.map(IpAddr::V6),
                interface: route.interface.clone(),
                metric: route.metric.unwrap_or(0),
            })
    }
}
```

### 4.3 路由管理

```rust
impl RouteTable {
    /// 添加 IPv4 路由
    ///
    /// # 参数
    ///
    /// - `route`: 要添加的路由条目
    ///
    /// # 返回
    ///
    /// - `Ok(())`: 路由添加成功
    /// - `Err(RouteError)`: 添加失败（如重复路由、接口不存在）
    pub fn add_ipv4_route(&mut self, route: Ipv4Route) -> Result<(), RouteError>;

    /// 删除 IPv4 路由
    ///
    /// # 参数
    ///
    /// - `destination`: 目标网络地址
    /// - `netmask`: 子网掩码
    ///
    /// # 返回
    ///
    /// - `Ok(())`: 路由删除成功
    /// - `Err(RouteError)`: 删除失败（如路由不存在）
    pub fn remove_ipv4_route(&mut self, destination: Ipv4Addr, netmask: Ipv4Addr) -> Result<(), RouteError>;
}
```

---

## 五、模块结构

```
src/route/
├── mod.rs           # 模块入口，导出公共类型
├── table.rs         # RouteTable 实现
├── ipv4.rs          # Ipv4Route 实现
├── ipv6.rs          # Ipv6Route 实现
└── error.rs         # RouteError 定义
```

### 模块导出

```rust
// src/route/mod.rs
mod ipv4;
mod ipv6;
mod table;
mod error;

pub use ipv4::Ipv4Route;
pub use ipv6::Ipv6Route;
pub use table::RouteTable;
pub use error::{RouteError, RouteResult};
```

---

## 六、错误处理

### 6.1 错误类型定义

```rust
/// 路由模块错误类型
#[derive(Debug)]
pub enum RouteError {
    /// 路由已存在
    RouteAlreadyExists {
        destination: String,
    },

    /// 路由不存在
    RouteNotFound {
        destination: String,
    },

    /// 接口不存在
    InterfaceNotFound {
        interface: String,
    },

    /// 无效的前缀长度
    InvalidPrefixLength {
        prefix_len: u8,
    },

    /// 无效的子网掩码
    InvalidNetmask {
        netmask: Ipv4Addr,
    },

    /// 路由表已满
    RouteTableFull,
}

impl std::fmt::Display for RouteError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::RouteAlreadyExists { destination } => {
                write!(f, "Route already exists: {}", destination)
            }
            Self::RouteNotFound { destination } => {
                write!(f, "Route not found: {}", destination)
            }
            Self::InterfaceNotFound { interface } => {
                write!(f, "Interface not found: {}", interface)
            }
            Self::InvalidPrefixLength { prefix_len } => {
                write!(f, "Invalid prefix length: {}", prefix_len)
            }
            Self::InvalidNetmask { netmask } => {
                write!(f, "Invalid netmask: {}", netmask)
            }
            Self::RouteTableFull => {
                write!(f, "Route table is full")
            }
        }
    }
}

impl std::error::Error for RouteError {}
```

---

## 七、SystemContext 集成

路由表需要添加到 `SystemContext` 中，以便各模块共享访问。

### 7.1 修改 SystemContext

```rust
// src/context.rs
use crate::route::RouteTable;

#[derive(Clone)]
pub struct SystemContext {
    pub interfaces: Arc<Mutex<InterfaceManager>>,
    pub arp_cache: Arc<Mutex<ArpCache>>,
    pub icmp_echo: Arc<Mutex<EchoManager>>,
    pub route_table: Arc<Mutex<RouteTable>>,  // 新增
}

impl SystemContext {
    pub fn new() -> Self {
        Self {
            interfaces: Arc::new(Mutex::new(InterfaceManager::new(256, 256))),
            arp_cache: Arc::new(Mutex::new(ArpCache::new())),
            icmp_echo: Arc::new(Mutex::new(EchoManager::new())),
            route_table: Arc::new(Mutex::new(RouteTable::new())),  // 新增
        }
    }

    pub fn with_components(
        interfaces: Arc<Mutex<InterfaceManager>>,
        arp_cache: Arc<Mutex<ArpCache>>,
        icmp_echo: Arc<Mutex<EchoManager>>,
        route_table: Arc<Mutex<RouteTable>>,  // 新增
    ) -> Self { ... }
}
```

### 7.2 使用示例

```rust
// IP 层使用路由表查找
fn forward_ipv4_packet(
    ctx: &SystemContext,
    dest: Ipv4Addr,
) -> Result<Option<InterfaceLookup>, IpError> {
    // 查找路由
    let route = ctx.route_table
        .lock()
        .unwrap()
        .lookup_ipv4(dest)
        .ok_or(IpError::NoRouteToHost { addr: dest })?;

    // 获取出接口
    let iface = ctx.interfaces
        .lock()
        .unwrap()
        .get_by_name(&route.interface)
        .map_err(|_| IpError::InterfaceNotFound {
            name: route.interface.clone(),
        })?;

    // 解析下一跳 MAC（如果有网关）
    let next_hop_mac = if let Some(next_hop) = route.next_hop {
        Some(ctx.arp_cache
            .lock()
            .unwrap()
            .lookup(next_hop)?)
    } else {
        None
    };

    Ok(Some(InterfaceLookup {
        interface: iface.clone(),
        next_hop_mac,
    }))
}
```

---

## 八、测试策略

### 8.1 单元测试范围

| 测试维度 | 覆盖要点 |
|---------|---------|
| **IPv4 路由** | 添加路由、删除路由、查找路由 |
| **IPv6 路由** | 添加路由、删除路由、查找路由 |
| **最长前缀匹配** | 验证选择最长前缀的路由 |
| **默认路由** | 验证 0.0.0.0/0 和 ::/0 路由 |
| **直连路由** | gateway=None 的路由 |
| **边界条件** | 空路由表、单条路由、多条路由 |
| **错误处理** | 重复路由、不存在的路由 |

### 8.2 集成测试场景

**场景一：IP 层路由查找**
- 创建包含多条路由的 RouteTable
- 模拟 IP 数据包到达
- 验证返回正确的下一跳和出接口

**场景二：与接口管理器集成**
- 添加路由时验证接口存在性
- 删除接口时清理相关路由
- 获取出接口的详细信息

**场景三：与 ARP 缓存集成**
- 查找路由后解析下一跳 MAC 地址
- 处理 ARP 解析失败的情况

### 8.3 测试用例示例

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_longest_prefix_match() {
        let mut table = RouteTable::new();

        // 添加多条路由
        table.add_ipv4_route(Ipv4Route {
            destination: Ipv4Addr::new(192, 168, 0, 0),
            netmask: Ipv4Addr::new(255, 255, 0, 0),
            gateway: Some(Ipv4Addr::new(192, 168, 1, 1)),
            interface: "eth0".to_string(),
            metric: Some(100),
        }).unwrap();

        table.add_ipv4_route(Ipv4Route {
            destination: Ipv4Addr::new(192, 168, 1, 0),
            netmask: Ipv4Addr::new(255, 255, 255, 0),
            gateway: Some(Ipv4Addr::new(192, 168, 1, 254)),
            interface: "eth0".to_string(),
            metric: Some(100),
        }).unwrap();

        // 测试最长前缀匹配
        let dest = Ipv4Addr::new(192, 168, 1, 100);
        let route = table.lookup_ipv4(dest).unwrap();

        // 应该选择 /24 而不是 /16
        assert_eq!(route.next_hop, Some(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 254))));
    }

    #[test]
    fn test_default_route() {
        let mut table = RouteTable::new();

        // 添加默认路由
        table.add_ipv4_route(Ipv4Route {
            destination: Ipv4Addr::new(0, 0, 0, 0),
            netmask: Ipv4Addr::new(0, 0, 0, 0),
            gateway: Some(Ipv4Addr::new(192, 168, 1, 1)),
            interface: "eth0".to_string(),
            metric: Some(100),
        }).unwrap();

        // 任意地址都应该匹配默认路由
        let dest = Ipv4Addr::new(8, 8, 8, 8);
        let route = table.lookup_ipv4(dest).unwrap();

        assert_eq!(route.next_hop, Some(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1))));
    }
}
```

---

## 九、实现路线图

| 阶段 | 内容 | 状态 |
|------|------|------|
| Phase 1 | 基础数据结构（Ipv4Route、Ipv6Route、RouteLookup） | 待实现 |
| Phase 2 | RouteTable 基础实现（添加、删除路由） | 待实现 |
| Phase 3 | IPv4 最长前缀匹配查找 | 待实现 |
| Phase 4 | IPv6 最长前缀匹配查找 | 待实现 |
| Phase 5 | SystemContext 集成 | 待实现 |
| Phase 6 | 单元测试 | 待规划 |
| Phase 7 | 与 IP 层集成 | 待规划 |

---

## 十、参考资料

- RFC 1812: Requirements for IP Version 4 Routers
- RFC 8200: Internet Protocol, Version 6 (IPv6) Specification
- [架构设计](architecture.md) - 整体架构说明
- [网络接口模块](interface.md) - InterfaceManager 设计
- [IPv4 协议设计](protocols/ip.md) - IPv4 协议实现
- [IPv6 协议设计](protocols/ipv6.md) - IPv6 协议实现

---

*文档版本：v1.0*
*生成日期：2026-02-22*
*CoreNet 项目 - 路由模块设计*
