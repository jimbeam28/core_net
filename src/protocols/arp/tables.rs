// src/protocols/arp/tables.rs
//
// ARP 表实现
// 管理 ARP 缓存条目

use std::collections::HashMap;
use std::collections::VecDeque;
use std::fmt;
use std::time::Instant;
use crate::protocols::{MacAddr, Ipv4Addr, Packet};
use crate::common::tables::Table;

// ========== ARP 状态枚举 ==========

/// ARP 缓存条目状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArpState {
    /// 无映射
    None,
    /// 等待响应
    Incomplete,
    /// 可用
    Reachable,
    /// 陈旧
    Stale,
    /// 延迟探测
    Delay,
    /// 探测中
    Probe,
}

/// ARP 条目老化结果
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgeResult {
    /// 无变化
    NoChange,
    /// 需要发送ARP请求（Incomplete或Probe超时）
    SendRequest { was_incomplete: bool },
    /// 条目被删除（超过最大重试次数）
    EntryDeleted,
    /// 状态变化到Stale（仅通知，无需发送请求）
    ToStale,
    /// 状态变化到Probe（Delay超时）
    ToProbe,
}

// ========== ARP 配置 ==========

/// ARP 配置参数
#[derive(Debug, Clone)]
pub struct ArpConfig {
    /// 重传定时器（秒）
    pub retrans_timeout: u64,
    /// 老化定时器（秒）
    pub aging_timeout: u64,
    /// 延迟定时器（秒）
    pub delay_timeout: u64,
    /// 探测定时器（秒）
    pub probe_timeout: u64,
    /// 最大重试次数
    pub max_retries: u32,
    /// 缓存最大条目数
    pub max_entries: usize,
    /// 是否启用 gratuitous ARP
    pub enable_gratuitous: bool,
    /// 每个条目等待队列的最大数据包数量
    pub max_pending_packets: usize,
}

impl Default for ArpConfig {
    fn default() -> Self {
        ArpConfig {
            retrans_timeout: 1,
            aging_timeout: 30,
            delay_timeout: 5,
            probe_timeout: 1,
            max_retries: 3,
            max_entries: 512,
            enable_gratuitous: true,
            max_pending_packets: 100,
        }
    }
}

// ========== ARP 缓存条目 ==========

/// ARP 缓存条目
#[derive(Clone)]
pub struct ArpEntry {
    /// 网络接口索引
    pub ifindex: u32,
    /// 协议地址 (IP)
    pub proto_addr: Ipv4Addr,
    /// 硬件地址 (MAC)
    pub hardware_addr: MacAddr,
    /// 条目状态
    pub state: ArpState,
    /// 创建时间戳
    pub created_at: Instant,
    /// 最后更新时间戳
    pub updated_at: Instant,
    /// 最后确认时间戳
    pub confirmed_at: Instant,
    /// 等待队列（INCOMPLETE 状态时使用）
    pub pending_packets: VecDeque<Packet>,
    /// 重试计数
    pub retry_count: u32,
}

impl fmt::Debug for ArpEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ArpEntry")
            .field("ifindex", &self.ifindex)
            .field("proto_addr", &self.proto_addr)
            .field("hardware_addr", &self.hardware_addr)
            .field("state", &self.state)
            .field("created_at", &self.created_at)
            .field("updated_at", &self.updated_at)
            .field("confirmed_at", &self.confirmed_at)
            .field("pending_packets_count", &self.pending_packets.len())
            .field("retry_count", &self.retry_count)
            .finish()
    }
}

impl ArpEntry {
    /// 创建新的 ARP 条目
    pub fn new(ifindex: u32, proto_addr: Ipv4Addr, hardware_addr: MacAddr) -> Self {
        let now = Instant::now();
        ArpEntry {
            ifindex,
            proto_addr,
            hardware_addr,
            state: ArpState::None,
            created_at: now,
            updated_at: now,
            confirmed_at: now,
            pending_packets: VecDeque::new(),
            retry_count: 0,
        }
    }

    /// 更新硬件地址和状态
    pub fn update(&mut self, hardware_addr: MacAddr, state: ArpState) {
        let old_state = self.state;
        self.hardware_addr = hardware_addr;
        self.state = state;
        self.updated_at = Instant::now();
        if state == ArpState::Reachable {
            self.confirmed_at = Instant::now();
        }
        // 状态转换时重置 retry_count
        if old_state != state {
            self.retry_count = 0;
        }
    }

    /// 添加等待的数据包
    ///
    /// 如果等待队列已满，返回错误
    pub fn add_pending(&mut self, packet: Packet, max_pending: usize) -> crate::common::Result<()> {
        if self.pending_packets.len() >= max_pending {
            return Err(crate::common::CoreError::invalid_packet(
                format!("等待队列已满: {} >= {}", self.pending_packets.len(), max_pending)
            ));
        }
        self.pending_packets.push_back(packet);
        Ok(())
    }

    /// 获取并清空等待队列
    pub fn take_pending(&mut self) -> VecDeque<Packet> {
        std::mem::take(&mut self.pending_packets)
    }
}

// ========== ARP 表 ==========

/// ARP 表键类型：(接口索引, IP 地址)
pub type ArpKey = (u32, Ipv4Addr);

/// ARP 缓存表
#[derive(Debug)]
pub struct ArpCache {
    /// 缓存条目：key = (ifindex, ip_addr)
    entries: HashMap<ArpKey, ArpEntry>,
    /// 配置参数
    config: ArpConfig,
}

impl ArpCache {
    /// 创建新的 ARP 缓存
    pub fn new(config: ArpConfig) -> Self {
        ArpCache {
            entries: HashMap::new(),
            config,
        }
    }

    /// 获取配置引用
    pub fn config(&self) -> &ArpConfig {
        &self.config
    }

    // ARP 特有的接口

    /// 查找 ARP 条目
    pub fn lookup_arp(&self, ifindex: u32, ip_addr: Ipv4Addr) -> Option<&ArpEntry> {
        self.entries.get(&(ifindex, ip_addr))
    }

    /// 查找并返回可变引用
    pub fn lookup_mut_arp(&mut self, ifindex: u32, ip_addr: Ipv4Addr) -> Option<&mut ArpEntry> {
        self.entries.get_mut(&(ifindex, ip_addr))
    }

    /// 更新 ARP 条目
    ///
    /// 如果缓存已满，将淘汰最旧的条目（LRU策略）
    /// 拒绝特殊IP地址（0.0.0.0、255.255.255.255、组播地址等）
    pub fn update_arp(&mut self, ifindex: u32, ip_addr: Ipv4Addr, mac_addr: MacAddr, state: ArpState) {
        // 拒绝特殊IP地址
        if Self::is_special_ip(&ip_addr) {
            return;
        }

        // 检查是否需要淘汰旧条目
        if self.entries.len() >= self.config.max_entries {
            // 查找最旧的条目（根据updated_at）
            if let Some(oldest_key) = self.find_oldest_entry() {
                // 如果新条目的key与最旧的不同，则淘汰最旧的
                let new_key = (ifindex, ip_addr);
                if oldest_key != new_key && !self.entries.contains_key(&new_key) {
                    self.entries.remove(&oldest_key);
                }
            }
        }

        let entry = self.entries.entry((ifindex, ip_addr)).or_insert_with(|| {
            ArpEntry::new(ifindex, ip_addr, mac_addr)
        });
        entry.update(mac_addr, state);
    }

    /// 查找最旧的ARP条目（用于LRU淘汰）
    ///
    /// 跳过正在等待响应的重要状态条目（Incomplete、Delay、Probe）
    /// 返回最旧条目的key，如果没有可淘汰的条目则返回None
    fn find_oldest_entry(&self) -> Option<ArpKey> {
        self.entries
            .iter()
            .filter(|(_, entry)| {
                // 跳过重要状态：这些条目正在等待响应或有待发送的数据包
                !matches!(entry.state,
                    ArpState::Incomplete | ArpState::Delay | ArpState::Probe
                )
            })
            .min_by(|a, b| a.1.updated_at.cmp(&b.1.updated_at))
            .map(|(key, _)| *key)
    }

    /// 检查IP地址是否为特殊地址（不应加入ARP缓存）
    ///
    /// 包括：
    /// - 0.0.0.0（未指定地址）
    /// - 255.255.255.255（广播地址）
    /// - 224.0.0.0/4（组播地址）
    fn is_special_ip(ip: &Ipv4Addr) -> bool {
        ip.is_unspecified() || ip.is_broadcast() || ip.is_multicast()
    }

    /// 删除 ARP 条目
    pub fn remove_arp(&mut self, ifindex: u32, ip_addr: Ipv4Addr) -> Option<ArpEntry> {
        self.entries.remove(&(ifindex, ip_addr))
    }

    // ========== 定时器处理方法 ==========

    /// 处理单个ARP条目的老化
    ///
    /// 根据条目状态和时间戳判断是否需要状态转换
    ///
    /// # 参数
    /// - key: ARP条目键
    ///
    /// # 返回
    /// - AgeResult: 老化处理结果
    pub fn age_entry(&mut self, key: &ArpKey) -> AgeResult {
        let now = Instant::now();
        let config = &self.config;

        if let Some(entry) = self.entries.get_mut(key) {
            match entry.state {
                ArpState::Reachable => {
                    // 检查是否超时转为Stale
                    if now.duration_since(entry.confirmed_at).as_secs() >= config.aging_timeout {
                        entry.state = ArpState::Stale;
                        AgeResult::ToStale
                    } else {
                        AgeResult::NoChange
                    }
                }
                ArpState::Stale => {
                    // Stale状态在需要使用时转为Delay，这里不主动转换
                    AgeResult::NoChange
                }
                ArpState::Delay => {
                    // 检查延迟定时器是否到期
                    if now.duration_since(entry.updated_at).as_secs() >= config.delay_timeout {
                        entry.state = ArpState::Probe;
                        entry.retry_count = 0;
                        entry.updated_at = now;
                        AgeResult::ToProbe
                    } else {
                        AgeResult::NoChange
                    }
                }
                ArpState::Probe => {
                    // 检查探测定时器是否到期
                    if now.duration_since(entry.updated_at).as_secs() >= config.probe_timeout {
                        if entry.retry_count >= config.max_retries {
                            // 超过最大重试次数，删除条目
                            self.entries.remove(key);
                            AgeResult::EntryDeleted
                        } else {
                            // 增加重试计数，由调用方在发送请求后更新
                            entry.retry_count += 1;
                            entry.updated_at = now;
                            AgeResult::SendRequest { was_incomplete: false }
                        }
                    } else {
                        AgeResult::NoChange
                    }
                }
                ArpState::Incomplete => {
                    // 检查重传定时器是否到期
                    if now.duration_since(entry.updated_at).as_secs() >= config.retrans_timeout {
                        if entry.retry_count >= config.max_retries {
                            // 超过最大重试次数，删除条目
                            self.entries.remove(key);
                            AgeResult::EntryDeleted
                        } else {
                            // 增加重试计数，由调用方在发送请求后更新
                            entry.retry_count += 1;
                            entry.updated_at = now;
                            AgeResult::SendRequest { was_incomplete: true }
                        }
                    } else {
                        AgeResult::NoChange
                    }
                }
                ArpState::None => {
                    // None状态不处理
                    AgeResult::NoChange
                }
            }
        } else {
            AgeResult::NoChange
        }
    }

    /// 标记条目需要使用（用于Stale -> Delay转换）
    ///
    /// 当需要使用一个Stale状态的条目时，调用此方法将其转为Delay状态
    ///
    /// # 参数
    /// - ifindex: 接口索引
    /// - ip_addr: IP地址
    ///
    /// # 返回
    /// - true: 条目存在且转为Delay状态
    /// - false: 条目不存在或不是Stale状态
    pub fn mark_used(&mut self, ifindex: u32, ip_addr: Ipv4Addr) -> bool {
        let key = (ifindex, ip_addr);
        if let Some(entry) = self.entries.get_mut(&key)
            && entry.state == ArpState::Stale {
                entry.state = ArpState::Delay;
                entry.updated_at = Instant::now();
                return true;
            }
        false
    }

    /// 获取需要发送请求的条目列表
    ///
    /// 遍历所有条目，返回需要发送ARP请求的条目
    ///
    /// # 返回
    /// 需要发送请求的条目列表：(ifindex, ip_addr, state)
    pub fn get_pending_requests(&mut self) -> Vec<(u32, Ipv4Addr, bool)> {
        let mut pending = Vec::new();
        let keys: Vec<ArpKey> = self.entries.keys().copied().collect();

        for key in keys {
            match self.age_entry(&key) {
                AgeResult::SendRequest { was_incomplete } => {
                    pending.push((key.0, key.1, !was_incomplete)); // true = probe, false = initial resolve
                }
                AgeResult::EntryDeleted | AgeResult::ToStale | AgeResult::ToProbe | AgeResult::NoChange => {
                    // 其他结果不需要发送请求
                }
            }
        }

        pending
    }

    /// 添加等待的数据包到条目
    ///
    /// 支持以下状态的条目添加等待队列：Incomplete, Delay, Probe
    ///
    /// # 参数
    /// - ifindex: 接口索引
    /// - ip_addr: IP地址
    /// - packet: 等待的数据包
    ///
    /// # 返回
    /// - Ok(()): 添加成功
    /// - Err(CoreError): 条目不存在或状态不支持等待队列
    pub fn add_pending_packet(&mut self, ifindex: u32, ip_addr: Ipv4Addr, packet: Packet) -> crate::common::Result<()> {
        let key = (ifindex, ip_addr);
        if let Some(entry) = self.entries.get_mut(&key) {
            // 支持 Incomplete、Delay、Probe 状态的等待队列
            let can_queue = matches!(entry.state,
                ArpState::Incomplete | ArpState::Delay | ArpState::Probe
            );
            if can_queue {
                entry.add_pending(packet, self.config.max_pending_packets)
            } else {
                Err(crate::common::CoreError::invalid_packet(
                    format!("ARP条目状态不支持等待队列: {:?}", entry.state)
                ))
            }
        } else {
            Err(crate::common::CoreError::invalid_packet(
                "ARP条目不存在"
            ))
        }
    }

    /// 获取并清空等待队列
    ///
    /// # 参数
    /// - ifindex: 接口索引
    /// - ip_addr: IP地址
    ///
    /// # 返回
    /// 等待队列的数据包数量
    pub fn take_pending_packets(&mut self, ifindex: u32, ip_addr: Ipv4Addr) -> usize {
        let key = (ifindex, ip_addr);
        if let Some(entry) = self.entries.get_mut(&key) {
            let pending = entry.take_pending();
            pending.len()
        } else {
            0
        }
    }
}

// 实现 Default trait
impl Default for ArpCache {
    fn default() -> Self {
        Self::new(ArpConfig::default())
    }
}

// 实现 Table trait
impl Table<ArpKey, ArpEntry> for ArpCache {
    fn lookup(&self, key: &ArpKey) -> Option<&ArpEntry> {
        self.entries.get(key)
    }

    fn lookup_mut(&mut self, key: &ArpKey) -> Option<&mut ArpEntry> {
        self.entries.get_mut(key)
    }

    fn insert(&mut self, key: ArpKey, value: ArpEntry) -> Option<ArpEntry> {
        self.entries.insert(key, value)
    }

    fn remove(&mut self, key: &ArpKey) -> Option<ArpEntry> {
        self.entries.remove(key)
    }

    fn clear(&mut self) {
        self.entries.clear();
    }

    fn len(&self) -> usize {
        self.entries.len()
    }

    fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    fn cleanup(&mut self) {
        let now = Instant::now();
        let entries_to_remove: Vec<_> = self.entries
            .iter()
            .filter(|(_, entry)| {
                // 只清理处于 None 状态且创建时间超过 30 秒的条目
                // 避免删除刚创建的临时状态
                if entry.state == ArpState::None {
                    // 检查是否创建超过 30 秒（使用 aging_timeout 作为参考）
                    let age_threshold = self.config.aging_timeout;
                    if now.duration_since(entry.created_at).as_secs() >= age_threshold {
                        return true;
                    }
                }
                false
            })
            .map(|(key, _)| *key)
            .collect();

        for key in entries_to_remove {
            self.entries.remove(&key);
        }
    }
}
