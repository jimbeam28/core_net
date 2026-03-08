# CoreNet 项目精简方案

## 1. Socket 模块（完全删除）

### 删除文件
```
src/socket/mod.rs
src/socket/types.rs
src/socket/entry.rs
src/socket/manager.rs
src/socket/error.rs
```

### 依赖清理
- `src/lib.rs`: 删除 Socket 模块导出
- `src/context.rs`: 删除 `socket_mgr` 字段
- `src/engine/processor.rs`: 删除 Socket 相关调用（UDP/TCP Delivered 处理改为直接打印日志）
- `src/protocols/tcp/socket_manager.rs`: 确认是否需要保留或合并到 TCP 模块

---

## 2. IPsec 模块（简化加密实现）

### 2.1 删除文件/目录
```
src/protocols/ipsec/ikev2/           # 整个目录删除（5个文件）
├── mod.rs
├── sa.rs
├── types.rs
├── exchange.rs
└── crypto.rs
```

### 2.2 修改文件

#### src/protocols/ipsec/mod.rs
**删除导出：**
- `pub mod ikev2;`
- `pub use ikev2::*;`

#### src/context.rs
**删除字段：**
```rust
pub ike_manager: Arc<Mutex<IkeSaManager>>,  // 删除
```

**删除方法：**
- `create_default_protocol_managers()` 中的 `ike_manager` 创建
- `ContextComponents` 中的 `ike_manager` 相关字段和方法

#### src/protocols/ipsec/crypto.rs（如存在）
**简化方案：**
- 保留函数签名，实际加密算法替换为 mock 实现
- `encrypt_aes_gcm()` -> 简单 XOR 或原样返回
- `decrypt_aes_gcm()` -> 简单 XOR 或原样返回
- `hmac_sha256()` -> 简单校验和或原样返回

#### src/protocols/ipsec/esp.rs
**简化 `decrypt_payload()`：**
```rust
// 原实现：实际解密
// 简化后：直接返回 payload（假设已解密）
pub fn decrypt_payload(&self, _cipher: &CipherTransform, _key: &[u8]) -> Vec<u8> {
    self.encrypted_data.clone()  // mock 实现
}
```

**简化 `verify_icv()`：**
```rust
// 原实现：HMAC 验证
// 简化后：始终返回 true
pub fn verify_icv(&self, _key: &[u8]) -> bool {
    true  // mock 实现
}
```

#### src/protocols/ipsec/ah.rs
**简化 `verify_icv()`：**
```rust
// 原实现：完整 ICV 计算和验证
// 简化后：始终返回 true
pub fn verify_icv(&self, _payload: &[u8], _key: &[u8]) -> bool {
    true  // mock 实现
}
```

#### src/protocols/ipsec/sa.rs
**简化 `SecurityAssociation`：**
- 移除 `cipher_key` 复杂管理
- 简化 `state` 生命周期
- `replay_window` 可保留基础功能或简化为计数器

#### src/protocols/ipsec/spd.rs
**简化 `SecurityPolicy`：**
- 保留基础匹配逻辑
- 简化动作处理（只保留 Allow/Discard，简化 Ipsec 动作）

---

## 3. IPv6 模块（简化扩展头）

### 3.1 删除文件
```
src/protocols/ipv6/extension.rs       # 扩展头链处理（如单独存在）
src/protocols/ipv6/routing.rs         # 路由扩展头详细处理
src/protocols/ipv6/fragment.rs        # 分片重组（如保留概念，简化实现）
```

### 3.2 修改文件

#### src/protocols/ipv6/mod.rs
**删除导出：**
- `pub mod extension;` （如存在）
- `pub mod routing;` （如存在）
- 复杂扩展头类型导出

**简化 `process_ipv6_packet()`：**
```rust
// 原实现：处理完整的扩展头链
// 简化后：只处理基础头部，遇到扩展头直接跳过或报错
pub fn process_ipv6_packet(packet: &mut Packet, ...) -> Result<...> {
    let header = Ipv6Header::parse(packet)?;

    // 简化：只处理直接上层协议，不遍历扩展头链
    match header.next_header {
        IpProtocol::IcmpV6 | IpProtocol::Tcp | IpProtocol::Udp => {
            // 直接处理
        }
        _ => {
            // 扩展头直接跳过或返回不支持
            return Ok(Ipv6ProcessResult::NoReply);
        }
    }
}
```

#### src/protocols/ipv6/header.rs
**简化 `Ipv6Header`：**
- 保留基础字段解析
- 移除复杂的扩展头遍历逻辑

**简化 `extension_headers()` 方法（如存在）：**
```rust
// 原实现：返回完整扩展头迭代器
// 简化后：返回空或基础信息
pub fn extension_headers(&self) -> Option<()> {
    None  // 简化：不处理扩展头
}
```

#### src/protocols/icmpv6/mod.rs
**简化 NDP 处理：**
- `NeighborCache` 简化为静态表或基础 LRU
- 移除完整的 NDP 状态机

**简化 `process_ndp_packet()`：**
```rust
// 原实现：完整 NDP 状态机
// 简化后：基础邻居发现，无状态管理
fn process_ndp_packet(...) -> Result<...> {
    // 只解析报文，不维护复杂状态
    // 直接返回简化响应
}
```

#### src/protocols/icmpv6/ndp.rs（如单独文件）
**简化 `NeighborCache`：**
```rust
// 原实现：完整状态机（INCOMPLETE, REACHABLE, STALE...）
// 简化后：基础映射表
pub struct NeighborCache {
    entries: HashMap<Ipv6Addr, MacAddr>,  // 只保留地址映射
}

impl NeighborCache {
    pub fn lookup(&self, ip: &Ipv6Addr) -> Option<MacAddr> {
        self.entries.get(ip).copied()
    }

    pub fn update(&mut self, ip: Ipv6Addr, mac: MacAddr) {
        self.entries.insert(ip, mac);  // 简单插入，无状态管理
    }
}
```

#### src/context.rs
**简化 `icmpv6_context`：**
```rust
// 原实现：完整的 Icmpv6Context
// 简化后：基础上下文
pub struct Icmpv6Context {
    pub neighbor_cache: NeighborCache,  // 简化的邻居缓存
    pub echo_manager: EchoManager,      // 保留 Echo
}
```

---

## 4. OSPF 模块（简化状态机和LSDB）

### 4.1 删除文件
```
src/protocols/ospf2/lsa/              # LSA 目录（如单独存在）
├── mod.rs
├── router.rs
├── network.rs
├── summary.rs
└── external.rs

src/protocols/ospf2/lsdb.rs           # 完整 LSDB（如单独存在）
src/protocols/ospf2/interface.rs      # 完整接口状态机（简化后合并）
src/protocols/ospf2/packet/           # 报文子目录（合并到主模块）
├── mod.rs
├── hello.rs
├── dbd.rs
├── lsr.rs
├── lsu.rs
└── lsack.rs
```

### 4.2 修改文件

#### src/protocols/ospf/mod.rs
**简化导出：**
```rust
// 保留基础类型
pub use types::{OspfOptions, InterfaceType};
// 删除复杂类型
// pub use spf::{SpfNode, SpfVertex...};  // 简化 SPF
```

**简化 `run_spf_calculation()`：**
```rust
// 原实现：完整 Dijkstra 算法
// 简化后：基础最短路径概念演示
pub fn run_spf_calculation(router_id: Ipv4Addr, lsas: &[Lsa]) -> SpfResult {
    // 简化：直接返回输入作为路由，不实际计算
    // 或只处理 2-3 个节点的简单拓扑
    SpfResult {
        routes: lsas.iter().map(|lsa| SimpleRoute::from(lsa)).collect(),
        success: true,
    }
}
```

#### src/protocols/ospf2/mod.rs
**合并文件：**
- 将 `packet/hello.rs`, `packet/dbd.rs` 等合并到 `mod.rs` 或 `packet.rs`

**简化 `OspfInterface`：**
```rust
// 原实现：完整状态机（Down, Loopback, Waiting, P2P, DR, BDR...）
// 简化后：基础邻居发现
pub struct OspfInterface {
    pub ifindex: u32,
    pub area_id: u32,
    pub neighbors: Vec<SimpleNeighbor>,  // 简化邻居列表
    pub state: SimpleInterfaceState,     // 简化：Down/Up
}

pub enum SimpleInterfaceState {
    Down,
    Up,  // 合并所有 Up 状态
}
```

**简化 `OspfNeighbor`：**
```rust
// 原实现：完整状态机（Down, Init, 2Way, ExStart, Exchange, Loading, Full）
// 简化后：基础状态
pub struct OspfNeighbor {
    pub router_id: Ipv4Addr,
    pub ip_addr: Ipv4Addr,
    pub state: SimpleNeighborState,
}

pub enum SimpleNeighborState {
    Down,
    Init,
    Full,  // 合并 2Way/ExStart/Exchange/Loading/Full
}
```

**简化 `process_ospfv2_packet()`：**
```rust
// 只处理 Hello 报文基础字段
pub fn process_ospfv2_packet(...) -> Result<...> {
    match ospf_type {
        OspfType::Hello => {
            // 简化：只解析，不维护复杂状态
            // 更新简化邻居表
        }
        _ => {
            // 其他报文类型只解析不处理，或返回不支持
            Ok(OspfProcessResult::NoReply)
        }
    }
}
```

#### src/protocols/ospf2/lsa.rs（新建合并文件）
**简化 LSA 类型：**
```rust
// 原实现：完整 LSA 类型（Router, Network, Summary, External...）
// 简化后：只保留基础 Router-LSA 概念
pub enum SimpleLsa {
    Router(RouterLsa),  // 只保留 Router LSA
}

pub struct RouterLsa {
    pub header: LsaHeader,
    pub links: Vec<RouterLink>,
}
```

#### src/protocols/ospf3/mod.rs
**简化策略与 OSPFv2 相同：**
- 简化接口/邻居状态机
- 只处理 Hello 报文
- 简化 LSA 类型

#### src/context.rs
**简化 `ospf_manager`：**
```rust
pub struct OspfManager {
    pub router_id: u32,
    pub v2_interfaces: Vec<SimpleOspfInterface>,  // 简化接口列表
    pub v3_interfaces: Vec<SimpleOspfV3Interface>,
    // 删除：完整 LSDB
    // 删除：复杂区域管理
}
```

---

## 5. BGP 模块（简化 FSM 和路径属性）

### 5.1 删除文件
```
src/protocols/bgp/fsm.rs              # 完整 FSM（简化后合并）
src/protocols/bgp/path_attr.rs        # 完整路径属性（简化）
src/protocols/bgp/rib.rs              # 完整 RIB（简化）
src/protocols/bgp/policy.rs           # 策略（如存在）
```

### 5.2 修改文件

#### src/protocols/bgp/mod.rs
**简化导出：**
```rust
// 保留基础报文类型
pub use header::{BgpHeader, BgpMessageType};
pub use open::BgpOpen;
pub use update::BgpUpdate;

// 删除复杂类型
// pub use fsm::BgpFsm;
// pub use rib::BgpRib;
```

**简化 `BgpPeer`：**
```rust
// 原实现：完整 FSM（Idle, Connect, Active, OpenSent, OpenConfirm, Established）
// 简化后：基础状态
pub struct BgpPeer {
    pub peer_addr: Ipv4Addr,
    pub local_as: u32,
    pub peer_as: u32,
    pub state: SimpleBgpState,
    pub routes: Vec<SimpleBgpRoute>,  // 简化路由表
}

pub enum SimpleBgpState {
    Idle,
    Connecting,  // 合并 Connect + Active
    Established, // 合并 OpenSent + OpenConfirm + Established
}
```

**简化 `process_bgp_packet()`：**
```rust
pub fn process_bgp_packet(data: &[u8], ...) -> Result<...> {
    let msg = parse_bgp_message(data)?;

    match msg {
        BgpMessage::Open(open) => {
            // 简化：只记录，不复杂状态转换
            Ok(BgpProcessResult::Reply(create_keepalive()))
        }
        BgpMessage::Update(update) => {
            // 简化：只提取基础路由信息
            let routes = extract_simple_routes(&update);
            Ok(BgpProcessResult::NoReply)
        }
        BgpMessage::Keepalive => {
            Ok(BgpProcessResult::NoReply)
        }
        _ => Ok(BgpProcessResult::NoReply),
    }
}
```

#### src/protocols/bgp/update.rs
**简化 `BgpUpdate`：**
```rust
pub struct BgpUpdate {
    pub withdrawn_routes: Vec<Ipv4Prefix>,
    pub path_attributes: Vec<SimplePathAttribute>,  // 简化属性列表
    pub nlri: Vec<Ipv4Prefix>,
}

pub enum SimplePathAttribute {
    Origin(u8),
    AsPath(Vec<u32>),  // 简化：只保留 AS 号列表
    NextHop(Ipv4Addr),
    // 删除：LocalPref, MED, Community 等复杂属性
}
```

#### src/context.rs
**简化 `bgp_manager`：**
```rust
pub struct BgpPeerManager {
    pub local_as: u32,
    pub router_id: Ipv4Addr,
    pub peers: Vec<BgpPeer>,  // 简化对等体列表
    // 删除：完整 RIB
    // 删除：策略引擎
}
```

---

## 6. TCP 模块（简化拥塞控制和定时器）

### 6.1 删除文件
```
src/protocols/tcp/timer.rs            # 完整定时器（简化后合并）
src/protocols/tcp/congestion.rs       # 拥塞控制算法（如单独存在）
```

### 6.2 修改文件

#### src/protocols/tcp/mod.rs
**简化导出：**
```rust
// 保留基础类型
pub use header::TcpHeader;
pub use segment::TcpSegment;
pub use connection::TcpConnection;

// 删除复杂类型
// pub use congestion::{CongestionControl, Cubic, Reno};
```

#### src/protocols/tcp/tcb.rs
**简化 `Tcb`：**
```rust
pub struct Tcb {
    // 保留基础序列号管理
    pub snd_una: u32,
    pub snd_nxt: u32,
    pub rcv_nxt: u32,
    pub rcv_wnd: u16,

    // 简化：删除复杂拥塞控制状态
    // pub cwnd: u32;      // 保留概念，简化实现
    // pub ssthresh: u32;  // 保留概念，简化实现

    // 简化定时器
    pub timers: SimpleTimers,  // 合并为简单定时器
}

pub struct SimpleTimers {
    pub retransmission: Option<Instant>,  // 只保留重传定时器
    // 删除：DelayedACK, Keepalive, Persist, TimeWait 定时器
}
```

**简化状态机：**
```rust
// 原实现：11 种状态
// 简化后：6 种核心状态
pub enum TcpState {
    Closed,
    SynSent,
    SynReceived,
    Established,
    FinWait,      // 合并 FinWait1 + FinWait2
    TimeWait,     // 简化实现
}
```

#### src/protocols/tcp/connection.rs
**简化 `TcpConnection`：**
```rust
pub struct TcpConnection {
    pub id: TcpConnectionId,
    pub state: TcpState,
    pub tcb: Tcb,
    pub rx_buffer: Vec<u8>,  // 简化缓冲区
    pub tx_buffer: Vec<u8>,
    // 删除：复杂窗口管理
    // 删除：完整拥塞控制
}

impl TcpConnection {
    pub fn on_packet(&mut self, segment: &TcpSegment) -> Result<...> {
        // 简化：基础状态转换
        // 删除：复杂拥塞控制处理
        // 删除：SACK 处理
        // 删除：窗口缩放复杂逻辑
    }
}
```

#### src/protocols/tcp/process.rs
**简化 `process_tcp_packet()`：**
```rust
pub fn process_tcp_packet(...) -> Result<...> {
    // 简化：基础三次握手/四次挥手
    // 简化：基础数据传输
    // 删除：复杂拥塞控制更新
    // 删除：SACK 处理
}
```

#### src/context.rs
**简化 TCP 相关字段：**
```rust
pub tcp_connections: Arc<Mutex<TcpConnectionManager>>,  // 简化版
pub tcp_sockets: Arc<Mutex<TcpSocketManager>>,          // 简化版
pub tcp_timers: Arc<Mutex<TcpTimerManager>>,            // 简化版
```

---

## 7. Context 模块（删除 Socket 和简化 IKE）

### 7.1 修改 src/context.rs

**删除字段：**
```rust
pub socket_mgr: Arc<Mutex<SocketManager>>,        // 删除
pub ike_manager: Arc<Mutex<IkeSaManager>>,        // 删除
```

**保留字段（简化版）：**
```rust
pub struct SystemContext {
    pub interfaces: Arc<Mutex<InterfaceManager>>,
    pub arp_cache: Arc<Mutex<ArpCache>>,
    pub icmp_echo: Arc<Mutex<EchoManager>>,
    pub tcp_connections: Arc<Mutex<TcpConnectionManager>>,
    pub tcp_sockets: Arc<Mutex<TcpSocketManager>>,
    pub udp_ports: Arc<Mutex<UdpPortManager>>,
    pub tcp_timers: Arc<Mutex<TcpTimerManager>>,
    pub timers: Arc<Mutex<TimerHandle>>,
    pub route_table: Arc<Mutex<RouteTable>>,
    pub icmpv6_context: Arc<Mutex<Icmpv6Context>>,      // 简化版
    pub ip_reassembly: Arc<Mutex<ReassemblyTable>>,
    pub ipv6_fragment_cache: Arc<Mutex<FragmentCache>>,
    pub ospf_manager: Arc<Mutex<OspfManager>>,          // 简化版
    pub bgp_manager: Arc<Mutex<BgpPeerManager>>,        // 简化版
    pub sad_mgr: Arc<Mutex<SadManager>>,                // 简化版
    pub spd_mgr: Arc<Mutex<SpdManager>>,                // 简化版
}
```

**简化 `ContextComponents`：**
- 删除 `socket_mgr`, `ike_manager` 相关字段和方法

---

## 8. Engine/Processor 模块（删除 Socket 调用）

### 8.1 修改 src/engine/processor.rs

**简化 `handle_udp()`：**
```rust
fn handle_udp(&self, eth_hdr: EthernetHeader, ip_hdr: ip::Ipv4Header, packet: Packet) -> ProcessResult {
    // ... 处理逻辑 ...

    match result {
        udp::UdpProcessResult::NoReply => Ok(None),
        udp::UdpProcessResult::PortUnreachable(original_ip) => {
            // ... 构造 ICMP 端口不可达 ...
        }
        udp::UdpProcessResult::Delivered(local_port, src_addr, src_port, data) => {
            // 删除：Socket 分发逻辑
            // if let Ok(mut socket_mgr) = self.context.socket_mgr.lock() {
            //     let _ = socket_mgr.deliver_udp_data(local_port, data, src_addr, src_port);
            // }

            // 改为：直接打印或记录
            if self.verbose {
                println!("UDP: Data delivered to port {} ({} bytes)", local_port, data.len());
            }
            Ok(None)
        }
    }
}
```

**简化 `handle_tcp()`：**
```rust
fn handle_tcp(&self, eth_hdr: EthernetHeader, ip_hdr: ip::Ipv4Header, packet: Packet) -> ProcessResult {
    // ... 处理逻辑 ...

    match result {
        tcp::TcpProcessResult::NoReply => Ok(None),
        tcp::TcpProcessResult::Delivered(conn_id, data) => {
            // 删除：Socket 分发
            // if let Ok(mut socket_mgr) = self.context.socket_mgr.lock() {
            //     let _ = socket_mgr.deliver_tcp_data(&conn_id, data);
            // }

            // 改为：直接打印
            if self.verbose {
                println!("TCP: Data delivered on {:?} ({} bytes)", conn_id, data.len());
            }
            Ok(None)
        }
        // ... 其他结果处理 ...
    }
}
```

---

## 9. lib.rs 导出清理

### 9.1 修改 src/lib.rs

**删除 Socket 相关导出：**
```rust
// 删除以下代码块：
// pub mod socket;
// pub use socket::{
//     SocketManager, SocketError, SocketConfig,
//     SocketFd, SocketAddr, SocketAddrV4, SocketAddrV6,
//     AddressFamily, SocketType, SocketProtocol,
//     SendFlags, RecvFlags, TcpState,
// };
```

**简化 IPsec 导出（如需要）：**
```rust
// 删除 IKEv2 导出
// pub use protocols::ipsec::ikev2::*;
```

---

## 10. 文件精简统计

### 10.1 完全删除的文件/目录

| 路径 | 文件数 | 说明 |
|------|--------|------|
| src/socket/ | 5 | 整个目录 |
| src/protocols/ipsec/ikev2/ | 5 | 整个目录 |
| src/protocols/ospf2/lsa/ | 5 | LSA 子目录 |
| src/protocols/ospf2/packet/ | 5 | 报文子目录 |
| src/protocols/ospf2/lsdb.rs | 1 | 完整 LSDB |
| src/protocols/ospf2/interface.rs | 1 | 完整接口状态机 |
| src/protocols/bgp/fsm.rs | 1 | 完整 FSM |
| src/protocols/bgp/path_attr.rs | 1 | 完整路径属性 |
| src/protocols/bgp/rib.rs | 1 | 完整 RIB |
| src/protocols/bgp/policy.rs | 1 | 策略（如存在） |
| src/protocols/tcp/timer.rs | 1 | 完整定时器 |
| src/protocols/tcp/congestion.rs | 1 | 拥塞控制（如存在） |
| src/protocols/ipv6/extension.rs | 1 | 扩展头处理 |
| src/protocols/ipv6/routing.rs | 1 | 路由扩展头 |
| src/protocols/icmpv6/ndp/ | 3 | NDP 子目录 |

**总计删除约 35 个文件**

### 10.2 需要大幅修改的文件

| 文件 | 修改内容 |
|------|----------|
| src/context.rs | 删除 socket_mgr, ike_manager；简化其他管理器 |
| src/lib.rs | 删除 Socket 导出；简化 IPsec 导出 |
| src/engine/processor.rs | 删除 Socket 调用；简化协议处理 |
| src/protocols/ipsec/mod.rs | 删除 ikev2 导出；简化加密调用 |
| src/protocols/ipsec/esp.rs | mock 解密/验证 |
| src/protocols/ipsec/ah.rs | mock ICV 验证 |
| src/protocols/ipv6/mod.rs | 简化扩展头处理 |
| src/protocols/icmpv6/mod.rs | 简化 NDP 处理 |
| src/protocols/ospf/mod.rs | 简化 SPF 算法 |
| src/protocols/ospf2/mod.rs | 合并文件；简化状态机 |
| src/protocols/ospf3/mod.rs | 合并文件；简化状态机 |
| src/protocols/bgp/mod.rs | 简化 FSM；简化路径属性 |
| src/protocols/tcp/tcb.rs | 简化状态机；简化定时器 |
| src/protocols/tcp/connection.rs | 简化拥塞控制处理 |

**总计修改约 20-25 个文件**

---

## 11. 精简后文件结构

```
src/
├── lib.rs
├── context.rs
├── main.rs
├── common/
│   ├── mod.rs
│   ├── packet.rs
│   ├── queue.rs
│   ├── error.rs
│   ├── addr.rs
│   └── timer.rs
├── interface/
│   ├── mod.rs
│   ├── iface.rs
│   ├── manager.rs
│   ├── types.rs
│   └── config.rs
├── engine/
│   ├── mod.rs
│   └── processor.rs
├── scheduler/
│   ├── mod.rs
│   └── scheduler.rs
├── route/
│   ├── mod.rs
│   ├── table.rs
│   ├── ipv4.rs
│   └── ipv6.rs
├── protocols/
│   ├── mod.rs
│   ├── ethernet/
│   │   ├── mod.rs
│   │   └── header.rs
│   ├── vlan/
│   │   ├── mod.rs
│   │   ├── tag.rs
│   │   └── parse.rs
│   ├── arp/
│   │   ├── mod.rs
│   │   └── tables.rs
│   ├── ip/
│   │   ├── mod.rs
│   │   ├── header.rs
│   │   ├── packet.rs
│   │   └── checksum.rs
│   ├── ipv6/
│   │   ├── mod.rs
│   │   ├── header.rs
│   │   └── packet.rs         # 简化扩展头
│   ├── icmp/
│   │   ├── mod.rs
│   │   ├── packet.rs
│   │   ├── echo.rs
│   │   └── process.rs
│   ├── icmpv6/
│   │   ├── mod.rs
│   │   ├── packet.rs
│   │   └── process.rs        # 简化 NDP
│   ├── udp/
│   │   ├── mod.rs
│   │   ├── header.rs
│   │   ├── packet.rs
│   │   └── port.rs
│   ├── tcp/
│   │   ├── mod.rs
│   │   ├── header.rs
│   │   ├── segment.rs
│   │   ├── connection.rs     # 简化
│   │   ├── tcb.rs            # 简化
│   │   ├── process.rs
│   │   └── socket_manager.rs # 简化
│   ├── ospf/
│   │   ├── mod.rs            # 简化
│   │   ├── types.rs
│   │   ├── config.rs
│   │   └── spf.rs            # 简化
│   ├── ospf2/
│   │   ├── mod.rs            # 合并 packet/*, 简化状态机
│   │   ├── header.rs
│   │   └── neighbor.rs       # 简化
│   ├── ospf3/
│   │   ├── mod.rs            # 简化
│   │   └── header.rs
│   ├── bgp/
│   │   ├── mod.rs            # 合并 fsm, rib, path_attr
│   │   ├── header.rs
│   │   ├── open.rs
│   │   ├── update.rs         # 简化路径属性
│   │   └── peer.rs           # 简化 FSM
│   └── ipsec/
│       ├── mod.rs            # 删除 ikev2
│       ├── ah.rs             # mock 验证
│       ├── esp.rs            # mock 解密
│       ├── sa.rs             # 简化
│       └── spd.rs            # 简化
└── testframework/
    ├── mod.rs
    ├── harness.rs
    └── injector.rs
```

---

## 12. 关键代码简化示例

### 12.1 IPsec mock 加密
```rust
// src/protocols/ipsec/crypto.rs
pub fn mock_encrypt(data: &[u8], _key: &[u8]) -> Vec<u8> {
    data.to_vec()  // 原样返回
}

pub fn mock_decrypt(data: &[u8], _key: &[u8]) -> Vec<u8> {
    data.to_vec()  // 原样返回
}

pub fn mock_hmac(_data: &[u8], _key: &[u8]) -> Vec<u8> {
    vec![0u8; 16]  // 返回固定长度假 ICV
}
```

### 12.2 OSPF 简化邻居状态
```rust
// src/protocols/ospf2/neighbor.rs
pub struct SimpleNeighbor {
    pub router_id: Ipv4Addr,
    pub ip_addr: Ipv4Addr,
    pub state: NeighborState,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum NeighborState {
    Down,
    Init,
    Full,  // 合并所有后续状态
}

impl SimpleNeighbor {
    pub fn on_hello(&mut self, _hello: &OspfHello) {
        // 简化：收到 Hello 直接转到 Full
        if self.state == NeighborState::Down {
            self.state = NeighborState::Init;
        } else if self.state == NeighborState::Init {
            self.state = NeighborState::Full;
        }
    }
}
```

### 12.3 BGP 简化路径属性
```rust
// src/protocols/bgp/update.rs
pub struct BgpUpdate {
    pub withdrawn_routes: Vec<Ipv4Prefix>,
    pub attributes: SimpleAttributes,
    pub nlri: Vec<Ipv4Prefix>,
}

pub struct SimpleAttributes {
    pub origin: u8,
    pub as_path: Vec<u32>,
    pub next_hop: Ipv4Addr,
}

impl SimpleAttributes {
    pub fn from_bytes(data: &[u8]) -> Result<Self> {
        // 简化：只解析 3 个核心属性
        // 跳过其他属性
    }
}
```

### 12.4 TCP 简化定时器
```rust
// src/protocols/tcp/tcb.rs
pub struct SimpleTimers {
    pub retransmission: Option<Instant>,
}

impl SimpleTimers {
    pub fn new() -> Self {
        Self {
            retransmission: None,
        }
    }

    pub fn set_retrans(&mut self, timeout: Duration) {
        self.retransmission = Some(Instant::now() + timeout);
    }

    pub fn check_expired(&self) -> bool {
        self.retransmission.map_or(false, |t| t <= Instant::now())
    }
}
```
