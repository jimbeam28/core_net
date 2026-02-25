// src/protocols/ospf/types.rs
//
// OSPF 共享类型定义
// 包含 OSPFv2 和 OSPFv3 共享的状态枚举和常量

use std::fmt;
use std::time::Instant;

/// OSPF 接口状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InterfaceState {
    /// 接口未启用 OSPF 或物理链路断开
    Down,
    /// 接口是环回接口，不发送 Hello
    Loopback,
    /// 等待 DR/BDR 选举
    Waiting,
    /// 点到点链路
    PointToPoint,
    /// 非 DR/BDR 路由器
    DROther,
    /// 指定路由器
    DR,
    /// 备份指定路由器
    Backup,
}

impl InterfaceState {
    pub fn name(&self) -> &'static str {
        match self {
            InterfaceState::Down => "Down",
            InterfaceState::Loopback => "Loopback",
            InterfaceState::Waiting => "Waiting",
            InterfaceState::PointToPoint => "Point-to-Point",
            InterfaceState::DROther => "DROther",
            InterfaceState::DR => "DR",
            InterfaceState::Backup => "Backup",
        }
    }

    /// 是否可以发送 Hello 报文
    pub fn can_send_hello(&self) -> bool {
        !matches!(self, InterfaceState::Down | InterfaceState::Loopback)
    }

    /// 是否需要选举 DR/BDR
    pub fn needs_dr_election(&self) -> bool {
        matches!(self, InterfaceState::Waiting | InterfaceState::DROther)
    }
}

impl fmt::Display for InterfaceState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name())
    }
}

/// OSPF 邻居状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NeighborState {
    /// 邻居状态未知，未通信
    Down,
    /// 在 NBMA 网络上尝试联系邻居
    Attempt,
    /// 收到 Hello，但双向通信未建立
    Init,
    /// 双向通信已建立
    TwoWay,
    /// 确定 Master/Slave 关系
    ExStart,
    /// 交换数据库描述报文
    Exchange,
    /// 请求并接收缺失的 LSA
    Loading,
    /// 邻接关系完全建立
    Full,
}

impl NeighborState {
    pub fn name(&self) -> &'static str {
        match self {
            NeighborState::Down => "Down",
            NeighborState::Attempt => "Attempt",
            NeighborState::Init => "Init",
            NeighborState::TwoWay => "2-Way",
            NeighborState::ExStart => "ExStart",
            NeighborState::Exchange => "Exchange",
            NeighborState::Loading => "Loading",
            NeighborState::Full => "Full",
        }
    }

    /// 双向通信是否已建立
    pub fn is_two_way_established(&self) -> bool {
        matches!(self,
            NeighborState::TwoWay |
            NeighborState::ExStart |
            NeighborState::Exchange |
            NeighborState::Loading |
            NeighborState::Full
        )
    }

    /// 邻接关系是否已建立
    pub fn is_adjacency_established(&self) -> bool {
        *self == NeighborState::Full
    }
}

impl fmt::Display for NeighborState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name())
    }
}

/// 接口类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InterfaceType {
    /// 广播网络（以太网）
    Broadcast,
    /// 点到点网络
    PointToPoint,
    /// 非广播多路访问（NBMA，如帧中继）
    NonBroadcast,
}

impl InterfaceType {
    /// 是否需要 DR/BDR 选举
    pub fn needs_dr_election(&self) -> bool {
        matches!(self, InterfaceType::Broadcast)
    }

    /// 是否是点到点网络
    pub fn is_point_to_point(&self) -> bool {
        matches!(self, InterfaceType::PointToPoint)
    }
}

impl fmt::Display for InterfaceType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            InterfaceType::Broadcast => write!(f, "Broadcast"),
            InterfaceType::PointToPoint => write!(f, "Point-to-Point"),
            InterfaceType::NonBroadcast => write!(f, "NBMA"),
        }
    }
}

/// LSA 序列号
///
/// LSA 序列号是 32 位有符号整数，用于检测 LSA 更新。
/// 初始值为 0x80000001，每次更新递增。
/// 达到 0x7FFFFFFF 后回卷到 0x80000001。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct LsaSequenceNumber(u32);

impl LsaSequenceNumber {
    /// 初始序列号
    pub const INITIAL: Self = Self(0x80000001);

    /// 最大序列号
    pub const MAX: Self = Self(0x7FFFFFFF);

    /// 创建新的序列号
    pub fn new() -> Self {
        Self::INITIAL
    }

    /// 递增序列号（处理回卷）
    pub fn increment(&self) -> Self {
        if self.0 == Self::MAX.0 {
            Self::INITIAL
        } else {
            Self(self.0 + 1)
        }
    }

    /// 获取原始值
    pub fn value(&self) -> u32 {
        self.0
    }

    /// 比较序列号是否更新（处理回卷情况）
    pub fn is_newer_than(&self, other: &Self) -> bool {
        // RFC 2328: 序列号比较逻辑
        // 如果差值在有效范围内且为正数，则 self 更新
        let diff = self.0.wrapping_sub(other.0) as i32;
        diff > 0
    }
}

impl Default for LsaSequenceNumber {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for LsaSequenceNumber {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "0x{:08X}", self.0)
    }
}

impl From<u32> for LsaSequenceNumber {
    fn from(value: u32) -> Self {
        Self(value)
    }
}

impl From<LsaSequenceNumber> for u32 {
    fn from(seq: LsaSequenceNumber) -> Self {
        seq.0
    }
}

/// OSPFv2 选项位
#[derive(Debug, Clone, Copy)]
pub struct OspfOptions {
    /// 支持 Demand Circuit
    pub dc: bool,
    /// 支持 Opaque LSA
    pub o: bool,
    /// 路由器标志
    pub r: bool,
    /// 不转发外部 LSA
    pub ea: bool,
    /// 是否连接到 NSSA
    pub n: bool,
    /// 支持 MOSPF（已废弃）
    pub mc: bool,
    /// 外部路由能力（ASBR）
    pub e: bool,
    /// 转发能力
    pub v: bool,
    /// IPv6 转发能力（OSPFv3）
    pub v6: bool,
}

impl OspfOptions {
    /// 空选项
    pub fn empty() -> Self {
        Self {
            dc: false,
            o: false,
            r: false,
            ea: false,
            n: false,
            mc: false,
            e: false,
            v: false,
            v6: false,
        }
    }

    /// 从字节解析（OSPFv2）
    pub fn from_byte(value: u8) -> Self {
        Self {
            dc: (value & 0x20) != 0,
            o: (value & 0x10) != 0,
            r: (value & 0x08) != 0,
            ea: (value & 0x04) != 0,
            n: (value & 0x02) != 0,
            mc: (value & 0x40) != 0,
            e: (value & 0x01) != 0,
            v: false,  // V 位在 OSPFv2 中未使用
            v6: false,  // V6 位在 OSPFv2 中未使用
        }
    }

    /// 转换为字节（OSPFv2）
    pub fn to_byte(&self) -> u8 {
        let mut value = 0u8;
        if self.dc { value |= 0x20; }
        if self.o { value |= 0x10; }
        if self.r { value |= 0x08; }
        if self.ea { value |= 0x04; }
        if self.n { value |= 0x02; }
        if self.mc { value |= 0x40; }
        if self.e { value |= 0x01; }
        // v 和 v6 在 OSPFv2 中不使用
        value
    }

    /// 从 16 位值解析（OSPFv3）
    pub fn from_u16(value: u16) -> Self {
        Self {
            dc: (value & 0x0020) != 0,
            r: (value & 0x0010) != 0,
            n: (value & 0x0008) != 0,
            mc: (value & 0x0004) != 0,
            e: (value & 0x0002) != 0,
            v6: (value & 0x0001) != 0,
            o: false,  // OSPFv3 中 O 位位置不同
            ea: false,
            v: false,  // V 位在 OSPFv3 中未使用
        }
    }

    /// 转换为 16 位值（OSPFv3）
    pub fn to_u16(&self) -> u16 {
        let mut value = 0u16;
        if self.dc { value |= 0x0020; }
        if self.r { value |= 0x0010; }
        if self.n { value |= 0x0008; }
        if self.mc { value |= 0x0004; }
        if self.e { value |= 0x0002; }
        if self.v6 { value |= 0x0001; }
        value
    }
}

impl Default for OspfOptions {
    fn default() -> Self {
        Self {
            dc: false,
            o: false,
            r: false,
            ea: false,
            n: false,
            mc: false,
            e: true,  // 默认支持外部路由
            v: false,
            v6: false,
        }
    }
}

impl fmt::Display for OspfOptions {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut flags = Vec::new();
        if self.dc { flags.push("DC"); }
        if self.o { flags.push("O"); }
        if self.r { flags.push("R"); }
        if self.ea { flags.push("EA"); }
        if self.n { flags.push("N"); }
        if self.mc { flags.push("MC"); }
        if self.e { flags.push("E"); }
        if self.v { flags.push("V"); }
        if self.v6 { flags.push("V6"); }
        write!(f, "{}", flags.join("|"))
    }
}

/// OSPF 认证类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthType {
    /// 无认证
    None = 0,
    /// 简单密码认证
    Simple = 1,
    /// 加密认证（MD5 或更新算法）
    Crypto = 2,
}

impl AuthType {
    pub fn from_u16(value: u16) -> Self {
        match value {
            0 => AuthType::None,
            1 => AuthType::Simple,
            2 => AuthType::Crypto,
            _ => AuthType::None,
        }
    }

    pub fn to_u16(&self) -> u16 {
        *self as u16
    }
}

/// OSPF 定时器
#[derive(Debug, Clone)]
pub struct OspfTimers {
    /// Hello 间隔（秒）
    pub hello_interval: u16,
    /// 路由器死亡间隔（秒）
    pub dead_interval: u32,
    /// 重传间隔（秒）
    pub retransmit_interval: u32,
    /// 传输延迟（秒）
    pub transmit_delay: u32,
}

impl Default for OspfTimers {
    fn default() -> Self {
        Self {
            hello_interval: 10,
            dead_interval: 40,
            retransmit_interval: 5,
            transmit_delay: 1,
        }
    }
}

/// OSPF 统计信息
#[derive(Debug, Clone, Default)]
pub struct OspfStats {
    /// 发送的 Hello 报文数
    pub hello_sent: u64,
    /// 接收的 Hello 报文数
    pub hello_received: u64,
    /// 发送的 DD 报文数
    pub dd_sent: u64,
    /// 接收的 DD 报文数
    pub dd_received: u64,
    /// 发送的 LSR 报文数
    pub lsr_sent: u64,
    /// 接收的 LSR 报文数
    pub lsr_received: u64,
    /// 发送的 LSU 报文数
    pub lsu_sent: u64,
    /// 接收的 LSU 报文数
    pub lsu_received: u64,
    /// 发送的 LSAck 报文数
    pub lsack_sent: u64,
    /// 接收的 LSAck 报文数
    pub lsack_received: u64,
    /// SPF 计算次数
    pub spf_runs: u64,
    /// LSA 更新次数
    pub lsa_updates: u64,
}

impl OspfStats {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn increment_hello_sent(&mut self) {
        self.hello_sent += 1;
    }

    pub fn increment_hello_received(&mut self) {
        self.hello_received += 1;
    }

    pub fn increment_spf_runs(&mut self) {
        self.spf_runs += 1;
    }

    pub fn increment_lsa_updates(&mut self) {
        self.lsa_updates += 1;
    }
}

/// OSPF 邻居信息
#[derive(Debug, Clone)]
pub struct OspfNeighbor {
    /// 邻居路由器 ID
    pub router_id: crate::common::Ipv4Addr,
    /// 邻居 IP 地址
    pub ip_addr: crate::common::Ipv4Addr,
    /// 邻居状态
    pub state: NeighborState,
    /// 邻居优先级
    pub priority: u8,
    /// 邻居的 DR
    pub dr: crate::common::Ipv4Addr,
    /// 邻居的 BDR
    pub bdr: crate::common::Ipv4Addr,
    /// Database Description 序列号
    pub dd_seq_number: u32,
    /// 最后收到 Hello 的时间
    pub last_hello_time: Instant,
    /// Inactivity Timer
    pub inactivity_timer: Instant,
    /// 是否是 Master
    pub is_master: bool,
    /// Database Description 交换是否完成
    pub dd_exchange_complete: bool,
}

impl OspfNeighbor {
    pub fn new(router_id: crate::common::Ipv4Addr, ip_addr: crate::common::Ipv4Addr) -> Self {
        let now = Instant::now();
        Self {
            router_id,
            ip_addr,
            state: NeighborState::Down,
            priority: 1,
            dr: crate::common::Ipv4Addr::unspecified(),
            bdr: crate::common::Ipv4Addr::unspecified(),
            dd_seq_number: 0,
            last_hello_time: now,
            inactivity_timer: now,
            is_master: false,
            dd_exchange_complete: false,
        }
    }

    /// 重置 Inactivity Timer
    pub fn reset_inactivity_timer(&mut self, dead_interval: u32) {
        self.inactivity_timer = Instant::now()
            .checked_add(std::time::Duration::from_secs(dead_interval as u64))
            .unwrap_or(self.inactivity_timer);
    }

    /// 检查 Inactivity Timer 是否超时
    pub fn is_inactivity_timer_expired(&self) -> bool {
        Instant::now() > self.inactivity_timer
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_interface_state_can_send_hello() {
        assert!(!InterfaceState::Down.can_send_hello());
        assert!(!InterfaceState::Loopback.can_send_hello());
        assert!(InterfaceState::Waiting.can_send_hello());
        assert!(InterfaceState::PointToPoint.can_send_hello());
        assert!(InterfaceState::DROther.can_send_hello());
        assert!(InterfaceState::DR.can_send_hello());
        assert!(InterfaceState::Backup.can_send_hello());
    }

    #[test]
    fn test_neighbor_state_is_two_way_established() {
        assert!(!NeighborState::Down.is_two_way_established());
        assert!(!NeighborState::Attempt.is_two_way_established());
        assert!(!NeighborState::Init.is_two_way_established());
        assert!(NeighborState::TwoWay.is_two_way_established());
        assert!(NeighborState::ExStart.is_two_way_established());
        assert!(NeighborState::Exchange.is_two_way_established());
        assert!(NeighborState::Loading.is_two_way_established());
        assert!(NeighborState::Full.is_two_way_established());
    }

    #[test]
    fn test_lsa_sequence_number_increment() {
        let seq = LsaSequenceNumber::new();
        assert_eq!(seq.value(), 0x80000001);

        let seq2 = seq.increment();
        assert_eq!(seq2.value(), 0x80000002);

        // 测试回卷
        let max_seq = LsaSequenceNumber::MAX;
        let wrapped = max_seq.increment();
        assert_eq!(wrapped.value(), 0x80000001);
    }

    #[test]
    fn test_lsa_sequence_number_is_newer_than() {
        let seq1 = LsaSequenceNumber::new();
        let seq2 = seq1.increment();

        assert!(seq2.is_newer_than(&seq1));
        assert!(!seq1.is_newer_than(&seq2));

        // 测试回卷情况
        let max_seq = LsaSequenceNumber::MAX;
        let wrapped = max_seq.increment();
        assert!(wrapped.is_newer_than(&max_seq));
    }

    #[test]
    fn test_ospf_options_from_byte() {
        let opts = OspfOptions::from_byte(0x01);  // E 位
        assert!(opts.e);
        assert!(!opts.dc);
        assert!(!opts.n);

        let opts = OspfOptions::from_byte(0x60);  // DC + MC (0x20 | 0x40)
        assert!(opts.dc);
        assert!(opts.mc);
    }

    #[test]
    fn test_ospf_options_to_byte() {
        let mut opts = OspfOptions::empty();
        opts.e = true;
        opts.dc = true;
        assert_eq!(opts.to_byte(), 0x21);
    }
}
