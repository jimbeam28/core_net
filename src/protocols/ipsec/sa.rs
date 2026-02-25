// src/protocols/ipsec/sa.rs
//
// IPsec 安全关联 (SA) 和安全策略数据库 (SPD)
// RFC 4301: Security Architecture for the Internet Protocol

use std::collections::HashMap;
use std::time::{Duration, Instant};
use crate::common::addr::IpAddr;
use super::{IpsecError, IpsecResult};

// ========== 方向和模式 ==========

/// SA 方向
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SaDirection {
    /// 入站 SA
    Inbound,
    /// 出站 SA
    Outbound,
}

/// IPsec 模式
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IpsecMode {
    /// 传输模式
    Transport,
    /// 隧道模式
    Tunnel,
}

/// IPsec 协议
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IpsecProtocol {
    /// AH (协议号 51)
    Ah = 51,
    /// ESP (协议号 50)
    Esp = 50,
}

impl IpsecProtocol {
    /// 从协议号获取协议类型
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            51 => Some(IpsecProtocol::Ah),
            50 => Some(IpsecProtocol::Esp),
            _ => None,
        }
    }

    /// 获取协议号
    pub fn as_u8(&self) -> u8 {
        match self {
            IpsecProtocol::Ah => 51,
            IpsecProtocol::Esp => 50,
        }
    }
}

/// SA 状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SaState {
    /// 初始状态（密钥协商中）
    Larval,
    /// 成熟状态（可用）
    Mature,
    /// 即将过期（软超时）
    Dying,
    /// 已过期
    Dead,
}

// ========== 加密和认证算法 ==========

/// 加密变换（用于 ESP）
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CipherTransform {
    /// AES-CBC (RFC 3602)
    AesCbc { key_size: usize },
    /// AES-CTR (RFC 3686)
    AesCtr { key_size: usize },
    /// AES-GCM (RFC 4106) - 同时提供加密和认证
    AesGcm { key_size: usize, icv_size: usize },
    /// 3DES-CBC (RFC 2451)
    TripleDesCbc,
    /// 无加密（仅认证）
    Null,
}

impl CipherTransform {
    /// 获取密钥长度
    pub fn key_size(&self) -> usize {
        match self {
            Self::AesCbc { key_size } |
            Self::AesCtr { key_size } |
            Self::AesGcm { key_size, .. } => *key_size,
            Self::TripleDesCbc => 24,
            Self::Null => 0,
        }
    }

    /// 获取块大小
    pub fn block_size(&self) -> usize {
        match self {
            Self::AesCbc { .. } | Self::TripleDesCbc => 16,
            Self::AesCtr { .. } => 16,
            Self::AesGcm { .. } => 16,
            Self::Null => 1,
        }
    }
}

/// 认证变换（用于 AH 和 ESP）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthTransform {
    /// HMAC-MD5-96 (RFC 2403)
    HmacMd5,
    /// HMAC-SHA1-96 (RFC 2404)
    HmacSha1,
    /// HMAC-SHA2-256 (RFC 4868)
    HmacSha2_256,
    /// HMAC-SHA2-384
    HmacSha2_384,
    /// HMAC-SHA2-512
    HmacSha2_512,
    /// AES-XCBC-MAC-96 (RFC 3566)
    AesXcbc,
    /// 无认证
    Null,
}

impl AuthTransform {
    /// 获取密钥长度
    pub fn key_size(&self) -> usize {
        match self {
            Self::HmacMd5 => 16,
            Self::HmacSha1 => 20,
            Self::HmacSha2_256 => 32,
            Self::HmacSha2_384 => 48,
            Self::HmacSha2_512 => 64,
            Self::AesXcbc => 16,
            Self::Null => 0,
        }
    }

    /// 获取 ICV 长度
    pub fn icv_size(&self) -> usize {
        match self {
            Self::HmacMd5 | Self::HmacSha1 | Self::AesXcbc => 12,
            Self::HmacSha2_256 => 16,
            Self::HmacSha2_384 => 24,
            Self::HmacSha2_512 => 32,
            Self::Null => 0,
        }
    }
}

// ========== 安全关联 ==========

/// 安全关联 (Security Association)
#[derive(Debug, Clone)]
pub struct SecurityAssociation {
    /// SA 方向
    pub direction: SaDirection,
    /// SPI
    pub spi: u32,
    /// 源地址
    pub src_addr: IpAddr,
    /// 目的地址
    pub dst_addr: IpAddr,
    /// IPsec 协议
    pub protocol: IpsecProtocol,
    /// IPsec 模式
    pub mode: IpsecMode,
    /// SA 状态
    pub state: SaState,
    /// 发送序列号
    pub tx_sequence: u64,
    /// 接收序列号（最高）
    pub rx_sequence: u64,
    /// 重放窗口
    pub replay_window: ReplayWindow,
    /// 加密算法
    pub cipher: Option<CipherTransform>,
    /// 加密密钥
    pub cipher_key: Option<Vec<u8>>,
    /// 认证算法
    pub auth: AuthTransform,
    /// 认证密钥
    pub auth_key: Vec<u8>,
    /// SA 创建时间
    pub created: Instant,
    /// SA 生命周期（秒）
    pub lifetime: Duration,
    /// 已处理的字节数
    pub bytes_processed: u64,
    /// 已处理的包数
    pub packets_processed: u64,
    /// 隧道模式下的源地址
    pub tunnel_src_addr: Option<IpAddr>,
    /// 隧道模式下的目的地址
    pub tunnel_dst_addr: Option<IpAddr>,
}

impl SecurityAssociation {
    /// 创建新的 SA
    pub fn new(
        direction: SaDirection,
        spi: u32,
        src_addr: IpAddr,
        dst_addr: IpAddr,
        protocol: IpsecProtocol,
        mode: IpsecMode,
        cipher: Option<CipherTransform>,
        auth: AuthTransform,
        lifetime: Duration,
    ) -> Self {
        Self {
            direction,
            spi,
            src_addr,
            dst_addr,
            protocol,
            mode,
            state: SaState::Mature,
            tx_sequence: 1,
            rx_sequence: 0,
            replay_window: ReplayWindow::new(64),
            cipher,
            cipher_key: None,
            auth,
            auth_key: Vec::new(),
            created: Instant::now(),
            lifetime,
            bytes_processed: 0,
            packets_processed: 0,
            tunnel_src_addr: None,
            tunnel_dst_addr: None,
        }
    }

    /// 设置密钥
    pub fn with_keys(mut self, cipher_key: Option<Vec<u8>>, auth_key: Vec<u8>) -> Self {
        self.cipher_key = cipher_key;
        self.auth_key = auth_key;
        self
    }

    /// 设置隧道地址
    pub fn with_tunnel(mut self, tunnel_src: Option<IpAddr>, tunnel_dst: Option<IpAddr>) -> Self {
        self.tunnel_src_addr = tunnel_src;
        self.tunnel_dst_addr = tunnel_dst;
        self
    }

    /// 检查 SA 是否过期
    pub fn is_expired(&self) -> bool {
        let age = self.created.elapsed();
        age >= self.lifetime
    }

    /// 检查是否即将过期（90% 生命周期）
    pub fn is_dying(&self) -> bool {
        let age = self.created.elapsed();
        let threshold = self.lifetime * 9 / 10;
        age >= threshold && !self.is_expired()
    }

    /// 检查重放
    pub fn check_replay(&mut self, seq: u64) -> bool {
        if seq > self.rx_sequence {
            // 新序列号
            self.rx_sequence = seq;
            self.replay_window.reset();
            true
        } else {
            // 检查重放窗口
            self.replay_window.check_and_mark(seq, self.rx_sequence)
        }
    }

    /// 获取下一个发送序列号
    pub fn next_sequence(&mut self) -> u64 {
        let seq = self.tx_sequence;
        self.tx_sequence += 1;
        seq
    }
}

/// 重放窗口
#[derive(Debug, Clone)]
pub struct ReplayWindow {
    /// 窗口大小
    window_size: u64,
    /// 位图（最多支持 1024 位）
    bitmap: Vec<u64>,
}

impl ReplayWindow {
    /// 创建新的重放窗口
    pub fn new(window_size: usize) -> Self {
        let words = (window_size + 63) / 64;
        Self {
            window_size: window_size as u64,
            bitmap: vec![0u64; words],
        }
    }

    /// 重置窗口
    pub fn reset(&mut self) {
        for word in &mut self.bitmap {
            *word = 0;
        }
    }

    /// 检查并标记序列号
    pub fn check_and_mark(&mut self, seq: u64, highest: u64) -> bool {
        if seq > highest {
            return true;
        }

        let offset = highest - seq;
        if offset >= self.window_size {
            return false; // 超出窗口
        }

        let word = (offset / 64) as usize;
        let bit = offset % 64;

        if word >= self.bitmap.len() {
            return false;
        }

        let mask = 1u64 << bit;
        if self.bitmap[word] & mask != 0 {
            return false; // 已接收
        }

        self.bitmap[word] |= mask;
        true
    }
}

// ========== 安全策略 ==========

/// 策略动作
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PolicyAction {
    /// 丢弃数据包
    Discard,
    /// 绕过 IPsec 处理
    Bypass,
    /// 应用 IPsec（需要 SA）
    Apply,
}

/// 流量选择器
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrafficSelector {
    /// 源地址（可选）
    pub src_addr: Option<IpAddr>,
    /// 目的地址（可选）
    pub dst_addr: Option<IpAddr>,
    /// 上层协议（0 表示任意）
    pub upper_layer_protocol: u8,
    /// 源端口范围（0-0 表示任意）
    pub src_port_range: (u16, u16),
    /// 目的端口范围（0-0 表示任意）
    pub dst_port_range: (u16, u16),
}

impl TrafficSelector {
    /// 创建新的流量选择器
    pub fn new(
        src_addr: Option<IpAddr>,
        dst_addr: Option<IpAddr>,
        upper_layer_protocol: u8,
    ) -> Self {
        Self {
            src_addr,
            dst_addr,
            upper_layer_protocol,
            src_port_range: (0, 0),
            dst_port_range: (0, 0),
        }
    }

    /// 设置端口范围
    pub fn with_ports(mut self, src_port: (u16, u16), dst_port: (u16, u16)) -> Self {
        self.src_port_range = src_port;
        self.dst_port_range = dst_port;
        self
    }

    /// 检查是否匹配流量
    pub fn matches(&self, src: IpAddr, dst: IpAddr, proto: u8) -> bool {
        // 检查协议
        if self.upper_layer_protocol != 0 && self.upper_layer_protocol != proto {
            return false;
        }

        // 检查源地址
        if let Some(ref selector_src) = self.src_addr {
            if selector_src != &src {
                return false;
            }
        }

        // 检查目的地址
        if let Some(ref selector_dst) = self.dst_addr {
            if selector_dst != &dst {
                return false;
            }
        }

        true
    }
}

/// 安全策略
#[derive(Debug, Clone)]
pub struct SecurityPolicy {
    /// 流量选择器
    pub selector: TrafficSelector,
    /// 策略动作
    pub action: PolicyAction,
    /// 优先级（数值越小优先级越高）
    pub priority: u32,
    /// 引用的 SA（仅 Apply 时有效）
    pub sa_ref: Option<u32>, // SPI
}

impl SecurityPolicy {
    /// 创建新的安全策略
    pub fn new(selector: TrafficSelector, action: PolicyAction, priority: u32) -> Self {
        Self {
            selector,
            action,
            priority,
            sa_ref: None,
        }
    }

    /// 设置关联的 SA
    pub fn with_sa(mut self, spi: u32) -> Self {
        self.sa_ref = Some(spi);
        self
    }
}

// ========== SAD 和 SPD 管理器 ==========

/// SAD 条目
pub type SadEntry = SecurityAssociation;

/// SPD 条目
pub type SpdEntry = SecurityPolicy;

/// SAD 管理器
#[derive(Debug)]
pub struct SadManager {
    /// SA 表，键为 (SPI, 目的地址, 协议)
    sas: HashMap<(u32, IpAddr, IpsecProtocol), SadEntry>,
}

impl SadManager {
    /// 创建新的 SAD 管理器
    pub fn new() -> Self {
        Self {
            sas: HashMap::new(),
        }
    }

    /// 添加 SA
    pub fn add(&mut self, sa: SadEntry) -> IpsecResult<()> {
        let key = (sa.spi, sa.dst_addr, sa.protocol);
        self.sas.insert(key, sa);
        Ok(())
    }

    /// 查找 SA
    pub fn get(&self, spi: u32, dst_addr: IpAddr, protocol: IpsecProtocol) -> Option<&SadEntry> {
        self.sas.get(&(spi, dst_addr, protocol))
    }

    /// 获取可变的 SA
    pub fn get_mut(&mut self, spi: u32, dst_addr: IpAddr, protocol: IpsecProtocol) -> Option<&mut SadEntry> {
        self.sas.get_mut(&(spi, dst_addr, protocol))
    }

    /// 删除 SA
    pub fn remove(&mut self, spi: u32, dst_addr: IpAddr, protocol: IpsecProtocol) -> Option<SadEntry> {
        self.sas.remove(&(spi, dst_addr, protocol))
    }

    /// 获取所有 SA
    pub fn all(&self) -> impl Iterator<Item = &SadEntry> {
        self.sas.values()
    }

    /// 清理过期的 SA
    pub fn cleanup_expired(&mut self) {
        self.sas.retain(|_, sa| !sa.is_expired());
    }

    /// SA 数量
    pub fn len(&self) -> usize {
        self.sas.len()
    }
}

impl Default for SadManager {
    fn default() -> Self {
        Self::new()
    }
}

/// SPD 管理器
#[derive(Debug)]
pub struct SpdManager {
    /// 策略列表（按优先级排序）
    policies: Vec<SpdEntry>,
}

impl SpdManager {
    /// 创建新的 SPD 管理器
    pub fn new() -> Self {
        Self {
            policies: Vec::new(),
        }
    }

    /// 添加策略
    pub fn add(&mut self, policy: SpdEntry) {
        // 插入并保持优先级排序
        let pos = self.policies
            .iter()
            .position(|p| p.priority > policy.priority)
            .unwrap_or(self.policies.len());
        self.policies.insert(pos, policy);
    }

    /// 查找匹配的策略
    pub fn lookup(&self, src: IpAddr, dst: IpAddr, proto: u8) -> Option<&SpdEntry> {
        self.policies
            .iter()
            .find(|p| p.selector.matches(src, dst, proto))
    }

    /// 删除策略
    pub fn remove(&mut self, priority: u32) -> Option<SpdEntry> {
        let pos = self.policies.iter().position(|p| p.priority == priority)?;
        Some(self.policies.remove(pos))
    }

    /// 获取所有策略
    pub fn all(&self) -> &[SpdEntry] {
        &self.policies
    }

    /// 策略数量
    pub fn len(&self) -> usize {
        self.policies.len()
    }
}

impl Default for SpdManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sa_creation() {
        let sa = SecurityAssociation::new(
            SaDirection::Outbound,
            0x12345678,
            IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1)),
            IpAddr::V4(Ipv4Addr::new(192, 168, 1, 2)),
            IpsecProtocol::Esp,
            IpsecMode::Transport,
            Some(CipherTransform::AesCbc { key_size: 128 }),
            AuthTransform::HmacSha1,
            Duration::from_secs(3600),
        );

        assert_eq!(sa.spi, 0x12345678);
        assert_eq!(sa.protocol, IpsecProtocol::Esp);
        assert_eq!(sa.mode, IpsecMode::Transport);
        assert_eq!(sa.state, SaState::Mature);
    }

    #[test]
    fn test_replay_window() {
        let mut window = ReplayWindow::new(64);

        // 第一个序列号
        assert!(window.check_and_mark(1, 1));

        // 重放检测
        assert!(!window.check_and_mark(1, 1));

        // 新序列号
        assert!(window.check_and_mark(2, 2));

        // 超出窗口
        assert!(!window.check_and_mark(1, 100));
    }

    #[test]
    fn test_traffic_selector() {
        let selector = TrafficSelector::new(
            Some(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1))),
            Some(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 2))),
            6, // TCP
        );

        assert!(selector.matches(
            IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1)),
            IpAddr::V4(Ipv4Addr::new(192, 168, 1, 2)),
            6,
        ));

        assert!(!selector.matches(
            IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1)),
            IpAddr::V4(Ipv4Addr::new(192, 168, 1, 3)),
            6,
        ));
    }

    #[test]
    fn test_sad_manager() {
        let mut sad = SadManager::new();

        let sa = SecurityAssociation::new(
            SaDirection::Outbound,
            0x12345678,
            IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1)),
            IpAddr::V4(Ipv4Addr::new(192, 168, 1, 2)),
            IpsecProtocol::Esp,
            IpsecMode::Transport,
            None,
            AuthTransform::Null,
            Duration::from_secs(3600),
        );

        sad.add(sa.clone()).unwrap();

        let found = sad.get(0x12345678, sa.dst_addr, IpsecProtocol::Esp);
        assert!(found.is_some());
        assert_eq!(found.unwrap().spi, 0x12345678);
    }

    #[test]
    fn test_spd_manager() {
        let mut spd = SpdManager::new();

        let policy = SecurityPolicy::new(
            TrafficSelector::new(None, None, 0),
            PolicyAction::Bypass,
            100,
        );

        spd.add(policy.clone());

        assert_eq!(spd.len(), 1);

        let found = spd.lookup(
            IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1)),
            IpAddr::V4(Ipv4Addr::new(192, 168, 1, 2)),
            6,
        );

        assert!(found.is_some());
        assert_eq!(found.unwrap().action, PolicyAction::Bypass);
    }

    #[test]
    fn test_protocol_from_u8() {
        assert_eq!(IpsecProtocol::from_u8(50), Some(IpsecProtocol::Esp));
        assert_eq!(IpsecProtocol::from_u8(51), Some(IpsecProtocol::Ah));
        assert_eq!(IpsecProtocol::from_u8(99), None);
    }
}
