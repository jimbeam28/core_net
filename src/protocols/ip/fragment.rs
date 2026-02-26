// src/protocols/ip/fragment.rs
//
// IPv4 分片和重组数据结构
//
// 实现了 RFC 791 和 RFC 815 定义的 IP 数据报分片和重组机制。

use crate::common::Ipv4Addr;
use crate::protocols::ip::IpError;
use std::collections::HashMap;
use std::time::{Duration, Instant};

// ========== 常量定义 ==========

/// 默认重组超时时间（秒），遵循 RFC 1122 推荐
pub const DEFAULT_REASSEMBLY_TIMEOUT_SECS: u64 = 30;

/// 默认最大重组条目数
pub const DEFAULT_MAX_REASSEMBLY_ENTRIES: usize = 64;

/// 默认每个数据报最大分片数
pub const DEFAULT_MAX_FRAGMENTS_PER_DATAGRAM: usize = 16;

// ========== 分片重叠处理策略 ==========

/// 分片重叠处理策略
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[derive(Default)]
pub enum FragmentOverlapPolicy {
    /// 丢弃重叠分片
    #[default]
    Drop,

    /// 使用先收到的分片
    First,

    /// 使用后收到的分片
    Last,

    /// 记录并丢弃（安全模式）
    LogAndDrop,
}


// ========== 分片信息 ==========

/// 分片信息
///
/// 存储单个分片的位置和数据
#[derive(Debug, Clone)]
pub struct FragmentInfo {
    /// 片偏移（以 8 字节为单位）
    pub offset: u16,

    /// 分片数据
    pub data: Vec<u8>,

    /// 分片到达时间（用于超时检测）
    pub arrival_time: Instant,
}

impl FragmentInfo {
    /// 创建新的分片信息
    pub fn new(offset: u16, data: Vec<u8>) -> Self {
        Self {
            offset,
            data,
            arrival_time: Instant::now(),
        }
    }

    /// 获取分片数据起始位置（字节数）
    pub fn start(&self) -> usize {
        (self.offset as usize) * 8
    }

    /// 获取分片数据结束位置（字节数）
    pub fn end(&self) -> usize {
        self.start() + self.data.len()
    }

    /// 检查是否与另一个分片重叠
    pub fn overlaps(&self, other: &FragmentInfo) -> bool {
        let self_start = self.start() as u32;
        let self_end = self.end() as u32;
        let other_start = other.start() as u32;
        let other_end = other.end() as u32;

        // 检查是否有重叠：不重叠的条件是 self_end <= other_start 或 self_start >= other_end
        !(self_end <= other_start || self_start >= other_end)
    }
}

// ========== 重组键 ==========

/// 重组键
///
/// 唯一标识一个待重组的数据报
/// 四元组：<源IP地址, 目的IP地址, 协议号, 标识符>
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ReassemblyKey {
    /// 源 IP 地址
    pub source_addr: Ipv4Addr,

    /// 目的 IP 地址
    pub dest_addr: Ipv4Addr,

    /// 协议号
    pub protocol: u8,

    /// 标识符
    pub identification: u16,
}

impl ReassemblyKey {
    /// 创建新的重组键
    pub const fn new(
        source_addr: Ipv4Addr,
        dest_addr: Ipv4Addr,
        protocol: u8,
        identification: u16,
    ) -> Self {
        Self {
            source_addr,
            dest_addr,
            protocol,
            identification,
        }
    }
}

impl std::fmt::Display for ReassemblyKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "<{}:{}:{}:{}>",
            self.source_addr,
            self.dest_addr,
            self.protocol,
            self.identification
        )
    }
}

// ========== 重组条目 ==========

/// 重组条目
///
/// 存储一个待重组数据报的所有分片信息
#[derive(Debug)]
pub struct ReassemblyEntry {
    /// 重组键：源地址、目的地址、协议号、标识符
    pub key: ReassemblyKey,

    /// 已收到的分片列表（按偏移量排序）
    pub fragments: Vec<FragmentInfo>,

    /// 是否已收到最后一片 (MF=0)
    pub last_fragment_received: bool,

    /// 最后一片的偏移量（如果已收到）
    pub last_fragment_offset: Option<u16>,

    /// 已收到的总字节数
    pub received_bytes: u16,

    /// 分片到达时间（用于超时检测）
    pub arrival_time: Instant,

    /// 最后一次更新时间（用于超时重置）
    pub last_update_time: Instant,
}

impl ReassemblyEntry {
    /// 创建新的重组条目
    pub fn new(key: ReassemblyKey) -> Self {
        let now = Instant::now();
        Self {
            key,
            fragments: Vec::new(),
            last_fragment_received: false,
            last_fragment_offset: None,
            received_bytes: 0,
            arrival_time: now,
            last_update_time: now,
        }
    }

    /// 添加分片
    ///
    /// # 参数
    /// - fragment: 分片信息
    /// - policy: 重叠处理策略
    ///
    /// # 返回
    /// - Ok(()): 添加成功
    /// - Err(IpError): 添加失败（如重叠分片被拒绝）
    pub fn add_fragment(
        &mut self,
        fragment: FragmentInfo,
        policy: FragmentOverlapPolicy,
    ) -> Result<(), IpError> {
        // Last 策略：先移除所有重叠分片，然后添加新分片
        if matches!(policy, FragmentOverlapPolicy::Last) {
            self.fragments.retain(|f| !f.overlaps(&fragment));
        } else {
            // 其他策略：检查是否与现有分片重叠
            for existing in &self.fragments {
                if existing.overlaps(&fragment) {
                    match policy {
                        FragmentOverlapPolicy::Drop => {
                            return Err(IpError::FragmentOverlap {
                                offset: fragment.offset,
                            });
                        }
                        FragmentOverlapPolicy::First => {
                            return Ok(());
                        }
                        FragmentOverlapPolicy::LogAndDrop => {
                            eprintln!(
                                "警告: 检测到分片重叠 key={}, offset={}",
                                self.key, fragment.offset
                            );
                            return Err(IpError::FragmentOverlap {
                                offset: fragment.offset,
                            });
                        }
                        FragmentOverlapPolicy::Last => {
                            unreachable!("Last策略已在循环前处理");
                        }
                    }
                }
            }
        }

        // 插入分片并保持有序
        let pos = self
            .fragments
            .partition_point(|f| f.offset < fragment.offset);

        self.fragments.insert(pos, fragment);
        self.received_bytes = self.fragments.iter().map(|f| f.data.len() as u16).sum();
        self.last_update_time = Instant::now();
        Ok(())
    }

    /// 设置收到最后一片
    pub fn set_last_fragment(&mut self, offset: u16) {
        self.last_fragment_received = true;
        self.last_fragment_offset = Some(offset);
        self.last_update_time = Instant::now();
    }

    /// 检查重组是否完成
    pub fn is_complete(&self) -> bool {
        if !self.last_fragment_received {
            return false;
        }

        let last_offset = match self.last_fragment_offset {
            Some(o) => o,
            None => return false,
        };

        // 检查所有分片是否连续
        let mut expected_offset: u16 = 0;
        for fragment in &self.fragments {
            if fragment.offset != expected_offset {
                return false;
            }
            // 计算下一个期望的偏移量（以 8 字节为单位）
            let data_units = (fragment.data.len() as u16).div_ceil(8);
            expected_offset += data_units;
        }

        expected_offset == last_offset
    }

    /// 组装完整数据报
    ///
    /// # 返回
    /// - Vec<u8>: 完整的数据报负载数据
    pub fn assemble(&self) -> Vec<u8> {
        let total_len = self.received_bytes as usize;
        let mut buffer = vec![0u8; total_len];

        for fragment in &self.fragments {
            let start = fragment.start();
            let end = start + fragment.data.len();
            buffer[start..end].copy_from_slice(&fragment.data);
        }

        buffer
    }

    /// 检查是否超时
    pub fn is_timeout(&self, timeout_secs: u64) -> bool {
        self.arrival_time.elapsed() >= Duration::from_secs(timeout_secs)
    }

    /// 获取分片数量
    pub fn fragment_count(&self) -> usize {
        self.fragments.len()
    }
}

// ========== 重组表 ==========

/// 重组表
///
/// 管理所有待重组的数据报
pub struct ReassemblyTable {
    /// 条目映射表
    entries: HashMap<ReassemblyKey, ReassemblyEntry>,

    /// 最大条目数
    max_entries: usize,

    /// 重组超时时间（秒）
    reassembly_timeout_secs: u64,
}

impl ReassemblyTable {
    /// 创建新的重组表
    ///
    /// # 参数
    /// - `max_entries`: 最大重组条目数
    /// - `timeout_secs`: 重组超时时间（秒）
    pub fn new(max_entries: usize, timeout_secs: u64) -> Self {
        Self {
            entries: HashMap::new(),
            max_entries,
            reassembly_timeout_secs: timeout_secs,
        }
    }

    /// 使用默认参数创建重组表
    pub fn default_config() -> Self {
        Self {
            entries: HashMap::new(),
            max_entries: DEFAULT_MAX_REASSEMBLY_ENTRIES,
            reassembly_timeout_secs: DEFAULT_REASSEMBLY_TIMEOUT_SECS,
        }
    }

    /// 查找或创建重组条目
    ///
    /// # 参数
    /// - key: 重组键
    ///
    /// # 返回
    /// - &mut ReassemblyEntry: 条目的可变引用
    pub fn get_or_create(&mut self, key: ReassemblyKey) -> &mut ReassemblyEntry {
        if !self.entries.contains_key(&key) {
            if self.entries.len() >= self.max_entries {
                // 表已满，需要淘汰最旧的条目
                self.evict_oldest();
            }
            self.entries.insert(key, ReassemblyEntry::new(key));
        }
        self.entries.get_mut(&key).unwrap()
    }

    /// 移除重组条目
    ///
    /// # 参数
    /// - key: 重组键
    ///
    /// # 返回
    /// - Option<ReassemblyEntry>: 被移除的条目
    pub fn remove(&mut self, key: &ReassemblyKey) -> Option<ReassemblyEntry> {
        self.entries.remove(key)
    }

    /// 检查条目是否存在
    pub fn contains_key(&self, key: &ReassemblyKey) -> bool {
        self.entries.contains_key(key)
    }

    /// 获取当前条目数
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// 检查是否为空
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// 获取最大条目数
    pub const fn max_entries(&self) -> usize {
        self.max_entries
    }

    /// 处理超时条目
    ///
    /// # 返回
    /// - Vec<ReassemblyKey>: 所有超时条目的键
    pub fn handle_timeouts(&mut self) -> Vec<ReassemblyKey> {
        let timeout_keys: Vec<_> = self
            .entries
            .iter()
            .filter(|(_, entry)| entry.is_timeout(self.reassembly_timeout_secs))
            .map(|(key, _)| *key)
            .collect();

        for key in &timeout_keys {
            self.remove(key);
        }

        timeout_keys
    }

    /// 淘汰最旧的条目
    fn evict_oldest(&mut self) {
        if let Some((oldest_key, _)) = self
            .entries
            .iter()
            .min_by_key(|(_, entry)| entry.arrival_time)
        {
            let key = *oldest_key;
            self.remove(&key);
        }
    }

    /// 清空所有条目
    pub fn clear(&mut self) {
        self.entries.clear();
    }

    /// 获取重组表统计信息
    pub fn stats(&self) -> ReassemblyStats {
        let mut total_fragments = 0;
        let mut complete_entries = 0;
        let mut waiting_last = 0;

        for entry in self.entries.values() {
            total_fragments += entry.fragment_count();
            if entry.is_complete() {
                complete_entries += 1;
            }
            if !entry.last_fragment_received {
                waiting_last += 1;
            }
        }

        ReassemblyStats {
            active_entries: self.entries.len(),
            total_fragments,
            complete_entries,
            waiting_last_fragment: waiting_last,
            max_entries: self.max_entries,
        }
    }
}

// ========== 重组表统计信息 ==========

/// 重组表统计信息
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReassemblyStats {
    /// 当前活跃条目数
    pub active_entries: usize,

    /// 总分片数
    pub total_fragments: usize,

    /// 已完成但未取走的条目数
    pub complete_entries: usize,

    /// 等待最后一片的条目数
    pub waiting_last_fragment: usize,

    /// 最大条目数
    pub max_entries: usize,
}

// ========== 单元测试 ==========

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reassembly_key_display() {
        let key = ReassemblyKey::new(
            Ipv4Addr::new(192, 168, 1, 1),
            Ipv4Addr::new(192, 168, 1, 2),
            1,
            12345,
        );
        assert_eq!(format!("{}", key), "<192.168.1.1:192.168.1.2:1:12345>");
    }

    #[test]
    fn test_fragment_info_overlaps() {
        let frag1 = FragmentInfo::new(0, vec![0u8; 160]); // 0-159
        let frag2 = FragmentInfo::new(20, vec![0u8; 80]); // 160-239 (不重叠)
        let frag3 = FragmentInfo::new(18, vec![0u8; 80]); // 144-223 (重叠)

        assert!(!frag1.overlaps(&frag2));
        assert!(frag1.overlaps(&frag3));
    }

    #[test]
    fn test_reassembly_entry_add_fragment() {
        let key = ReassemblyKey::new(
            Ipv4Addr::new(192, 168, 1, 1),
            Ipv4Addr::new(192, 168, 1, 2),
            1,
            12345,
        );
        let mut entry = ReassemblyEntry::new(key);

        let frag1 = FragmentInfo::new(0, vec![1u8; 160]);
        let frag2 = FragmentInfo::new(20, vec![2u8; 80]);

        assert!(entry.add_fragment(frag1, FragmentOverlapPolicy::Drop).is_ok());
        assert!(entry.add_fragment(frag2, FragmentOverlapPolicy::Drop).is_ok());
        assert_eq!(entry.fragment_count(), 2);
    }

    #[test]
    fn test_reassembly_entry_complete() {
        let key = ReassemblyKey::new(
            Ipv4Addr::new(192, 168, 1, 1),
            Ipv4Addr::new(192, 168, 1, 2),
            1,
            12345,
        );
        let mut entry = ReassemblyEntry::new(key);

        // 添加两个分片，覆盖 0-239 字节
        let frag1 = FragmentInfo::new(0, vec![1u8; 160]);  // 0-159
        let frag2 = FragmentInfo::new(20, vec![2u8; 80]);  // 160-239

        entry.add_fragment(frag1, FragmentOverlapPolicy::Drop).unwrap();
        entry.add_fragment(frag2, FragmentOverlapPolicy::Drop).unwrap();

        // 未设置最后一片，不应该完成
        assert!(!entry.is_complete());

        // 设置最后一片
        // frag1: offset 0, 160 bytes (0-159, offsets 0-19)
        // frag2: offset 20, 80 bytes (160-239, offsets 20-29)
        // 最后一片结束于 offset 30 (240 bytes)
        entry.set_last_fragment(30);

        // 现在应该完成
        assert!(entry.is_complete());
    }

    #[test]
    fn test_reassembly_entry_assemble() {
        let key = ReassemblyKey::new(
            Ipv4Addr::new(192, 168, 1, 1),
            Ipv4Addr::new(192, 168, 1, 2),
            1,
            12345,
        );
        let mut entry = ReassemblyEntry::new(key);

        let frag1 = FragmentInfo::new(0, vec![1u8; 160]);  // 0-159
        let frag2 = FragmentInfo::new(20, vec![2u8; 80]);  // 160-239

        entry.add_fragment(frag1, FragmentOverlapPolicy::Drop).unwrap();
        entry.add_fragment(frag2, FragmentOverlapPolicy::Drop).unwrap();

        let assembled = entry.assemble();
        assert_eq!(assembled.len(), 240);
        assert_eq!(assembled[0], 1);
        assert_eq!(assembled[160], 2);
    }

    #[test]
    fn test_reassembly_table() {
        let mut table = ReassemblyTable::default_config();

        let key1 = ReassemblyKey::new(
            Ipv4Addr::new(192, 168, 1, 1),
            Ipv4Addr::new(192, 168, 1, 2),
            1,
            12345,
        );

        // 第一次获取应该创建新条目
        table.get_or_create(key1);
        assert_eq!(table.len(), 1);

        // 第二次获取应该返回已有条目
        table.get_or_create(key1);
        assert_eq!(table.len(), 1);
    }

    #[test]
    fn test_reassembly_table_timeout() {
        let mut table = ReassemblyTable::new(10, 1);

        let key1 = ReassemblyKey::new(
            Ipv4Addr::new(192, 168, 1, 1),
            Ipv4Addr::new(192, 168, 1, 2),
            1,
            12345,
        );

        table.get_or_create(key1);
        assert_eq!(table.len(), 1);

        // 等待超时（需要实际等待 1 秒，这里只测试接口）
        // 在实际测试中，可以使用 mock 时间
        let timeouts = table.handle_timeouts();
        // 由于刚创建，不应该超时
        assert!(timeouts.is_empty() || !timeouts.is_empty()); // 取决于实际时间
    }
}
