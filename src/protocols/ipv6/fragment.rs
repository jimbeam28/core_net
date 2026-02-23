// src/protocols/ipv6/fragment.rs
//
// IPv6 分片与重组实现

use crate::protocols::Ipv6Addr;
use std::collections::HashMap;
use std::time::{Duration, Instant};

// --- 分片重组相关常量 ---

/// 默认分片缓存最大条目数
pub const DEFAULT_MAX_REASSEMBLY_ENTRIES: usize = 256;

/// 默认重组超时时间（秒，RFC 8200 要求）
pub const DEFAULT_REASSEMBLY_TIMEOUT: u64 = 60;

/// 每个数据包的最大分片数
pub const DEFAULT_MAX_FRAGMENTS_PER_PACKET: usize = 64;

// --- 重组键 ---

/// 重组键
///
/// 用于唯一标识一组分片，由源地址、目的地址和分片标识符组成。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ReassemblyKey {
    /// 源 IPv6 地址
    pub source_addr: Ipv6Addr,
    /// 目的 IPv6 地址
    pub dest_addr: Ipv6Addr,
    /// 分片标识符
    pub identification: u32,
}

impl ReassemblyKey {
    /// 创建新的重组键
    pub fn new(source_addr: Ipv6Addr, dest_addr: Ipv6Addr, identification: u32) -> Self {
        ReassemblyKey {
            source_addr,
            dest_addr,
            identification,
        }
    }
}

// --- 分片信息 ---

/// 分片信息
///
/// 存储单个分片的信息和数据。
#[derive(Debug, Clone)]
pub struct FragmentInfo {
    /// 分片偏移（以 8 字节为单位）
    pub offset: u16,
    /// 更多分片标志
    pub more_fragments: bool,
    /// 分片数据
    pub data: Vec<u8>,
}

impl FragmentInfo {
    /// 创建新的分片信息
    pub fn new(offset: u16, more_fragments: bool, data: Vec<u8>) -> Self {
        FragmentInfo {
            offset,
            more_fragments,
            data,
        }
    }

    /// 获取分片数据结束的字节偏移
    pub fn end_offset(&self) -> usize {
        (self.offset as usize * 8) + self.data.len()
    }

    /// 检查是否与另一个分片重叠
    pub fn overlaps_with(&self, other: &FragmentInfo) -> bool {
        let self_start = self.offset as usize * 8;
        let self_end = self.end_offset();
        let other_start = other.offset as usize * 8;
        let other_end = other.end_offset();

        // 检查是否有重叠
        !(self_end <= other_start || other_end <= self_start)
    }
}

// --- 重组条目 ---

/// 重组条目
///
/// 存储一组分片的重组状态。
#[derive(Debug, Clone)]
pub struct ReassemblyEntry {
    /// 重组键
    pub key: ReassemblyKey,
    /// 已收到的分片
    pub fragments: Vec<FragmentInfo>,
    /// 总数据长度（当收到 M=0 的分片后确定）
    pub total_length: Option<usize>,
    /// 首个分片到达时间
    pub first_arrival: Instant,
}

impl ReassemblyEntry {
    /// 创建新的重组条目
    pub fn new(key: ReassemblyKey) -> Self {
        let now = Instant::now();
        ReassemblyEntry {
            key,
            fragments: Vec::new(),
            total_length: None,
            first_arrival: now,
        }
    }

    /// 添加分片
    pub fn add_fragment(&mut self, fragment: FragmentInfo) -> Result<(), ReassemblyError> {
        // 检查分片数量限制
        if self.fragments.len() >= DEFAULT_MAX_FRAGMENTS_PER_PACKET {
            return Err(ReassemblyError::TooManyFragments {
                count: self.fragments.len(),
                max: DEFAULT_MAX_FRAGMENTS_PER_PACKET,
            });
        }

        // 检查重叠分片
        for existing in &self.fragments {
            if fragment.overlaps_with(existing) {
                // 根据 RFC 5722，重叠分片应该被丢弃
                return Err(ReassemblyError::FragmentOverlap {
                    offset: fragment.offset,
                    existing_offset: existing.offset,
                });
            }
        }

        // 如果是最后一片，设置总长度
        if !fragment.more_fragments {
            let offset_bytes = fragment.offset as usize * 8;
            let new_total = offset_bytes + fragment.data.len();

            // 如果已经设置了总长度，检查是否一致
            if let Some(existing_total) = self.total_length {
                if existing_total != new_total {
                    return Err(ReassemblyError::InconsistentTotalLength {
                        expected: existing_total,
                        found: new_total,
                    });
                }
            } else {
                self.total_length = Some(new_total);
            }
        }

        self.fragments.push(fragment);
        Ok(())
    }

    /// 检查是否所有分片已到齐
    pub fn is_complete(&self) -> bool {
        let total_length = match self.total_length {
            Some(len) => len,
            None => return false,
        };

        if self.fragments.is_empty() {
            return false;
        }

        // 计算已收到的总字节数
        let mut received_bytes = 0;
        for frag in &self.fragments {
            received_bytes += frag.data.len();
        }

        received_bytes >= total_length
    }

    /// 检查是否超时（60 秒）
    pub fn is_expired(&self) -> bool {
        self.first_arrival.elapsed() > Duration::from_secs(DEFAULT_REASSEMBLY_TIMEOUT)
    }

    /// 重组数据包
    pub fn reassemble(&self) -> Result<Vec<u8>, ReassemblyError> {
        if !self.is_complete() {
            return Err(ReassemblyError::Incomplete);
        }

        let total_length = self.total_length.unwrap();
        let mut buffer = vec![0u8; total_length];

        // 按偏移排序分片
        let mut sorted_fragments = self.fragments.clone();
        sorted_fragments.sort_by_key(|f| f.offset);

        // 复制分片数据
        for fragment in &sorted_fragments {
            let offset_bytes = fragment.offset as usize * 8;
            if offset_bytes + fragment.data.len() > buffer.len() {
                return Err(ReassemblyError::InvalidFragmentData);
            }
            buffer[offset_bytes..offset_bytes + fragment.data.len()]
                .copy_from_slice(&fragment.data);
        }

        Ok(buffer)
    }
}

// --- 分片重组错误 ---

/// 分片重组错误
#[derive(Debug)]
pub enum ReassemblyError {
    /// 分片数量超过限制
    TooManyFragments { count: usize, max: usize },

    /// 分片重叠
    FragmentOverlap { offset: u16, existing_offset: u16 },

    /// 总长度不一致
    InconsistentTotalLength { expected: usize, found: usize },

    /// 重组未完成
    Incomplete,

    /// 无效的分片数据
    InvalidFragmentData,

    /// 重组超时
    Timeout,
}

impl std::fmt::Display for ReassemblyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ReassemblyError::TooManyFragments { count, max } => {
                write!(f, "分片数量超过限制: {} >= {}", count, max)
            }
            ReassemblyError::FragmentOverlap { offset, existing_offset } => {
                write!(f, "分片重叠: 偏移 {} 与 {} 重叠", offset, existing_offset)
            }
            ReassemblyError::InconsistentTotalLength { expected, found } => {
                write!(f, "总长度不一致: 期望 {}, 发现 {}", expected, found)
            }
            ReassemblyError::Incomplete => {
                write!(f, "重组未完成")
            }
            ReassemblyError::InvalidFragmentData => {
                write!(f, "无效的分片数据")
            }
            ReassemblyError::Timeout => {
                write!(f, "重组超时")
            }
        }
    }
}

impl std::error::Error for ReassemblyError {}

// --- 分片缓存 ---

/// 分片缓存
///
/// 管理所有进行中的分片重组。
pub struct FragmentCache {
    /// 重组条目映射
    entries: HashMap<ReassemblyKey, ReassemblyEntry>,

    /// 最大条目数
    max_entries: usize,
}

impl FragmentCache {
    /// 创建新的分片缓存
    pub fn new(max_entries: usize) -> Self {
        FragmentCache {
            entries: HashMap::new(),
            max_entries,
        }
    }

    /// 添加分片
    ///
    /// 如果添加后重组完成，返回重组后的数据。
    pub fn add_fragment(
        &mut self,
        key: ReassemblyKey,
        fragment: FragmentInfo,
    ) -> Result<Option<Vec<u8>>, ReassemblyError> {
        // 清理超时条目
        self.cleanup_expired();

        // 检查是否超过最大条目数
        if !self.entries.contains_key(&key) && self.entries.len() >= self.max_entries {
            // 缓存已满，丢弃最老的条目
            if let Some((&k, _)) = self.entries
                .iter()
                .min_by_key(|(_, entry)| entry.first_arrival)
            {
                self.entries.remove(&k);
            }
        }

        // 获取或创建重组条目
        let entry = self.entries.entry(key).or_insert_with(|| {
            ReassemblyEntry::new(key)
        });

        entry.add_fragment(fragment)?;

        if entry.is_complete() {
            let entry_key = entry.key;
            let data = entry.reassemble()?;
            self.entries.remove(&entry_key);
            Ok(Some(data))
        } else {
            Ok(None)
        }
    }

    /// 清理超时条目
    pub fn cleanup_expired(&mut self) {
        self.entries.retain(|_, entry| !entry.is_expired());
    }

    /// 清空所有条目
    pub fn clear(&mut self) {
        self.entries.clear();
    }

    /// 获取当前条目数
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// 检查是否为空
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

impl Default for FragmentCache {
    fn default() -> Self {
        FragmentCache::new(DEFAULT_MAX_REASSEMBLY_ENTRIES)
    }
}

// --- 分片创建 ---

/// 创建分片数据
///
/// 将大数据包分片为多个小数据包。
///
/// # 参数
/// - data: 原始数据
/// - mtu: 路径 MTU（不包括 IPv6 头部）
/// - identification: 分片标识符
/// - next_header: 上层协议号
///
/// # 返回
/// 分片列表，每个元素为 (fragment_offset, more_fragments, fragment_data)
pub fn create_fragments(
    data: &[u8],
    mtu: usize,
    _identification: u32,
    _next_header: u8,
) -> Vec<(u16, bool, Vec<u8>)> {
    let mut fragments = Vec::new();

    // 计算每个分片的最大数据长度
    // MTU - 分片头长度 (8 字节)
    let max_fragment_data = if mtu > 8 { mtu - 8 } else { 0 };

    if max_fragment_data == 0 || data.len() <= max_fragment_data {
        // 不需要分片
        fragments.push((0, false, data.to_vec()));
        return fragments;
    }

    // 分片数据长度必须是 8 字节的倍数（除了最后一片）
    let fragment_data_len = (max_fragment_data / 8) * 8;

    if fragment_data_len == 0 {
        // MTU 太小，无法分片
        fragments.push((0, false, data.to_vec()));
        return fragments;
    }

    let mut offset = 0u16;
    let mut pos = 0;

    while pos < data.len() {
        let remaining = data.len() - pos;
        let chunk_size = if remaining > fragment_data_len {
            fragment_data_len
        } else {
            remaining
        };

        let more_fragments = pos + chunk_size < data.len();

        fragments.push((
            offset,
            more_fragments,
            data[pos..pos + chunk_size].to_vec(),
        ));

        offset += (chunk_size / 8) as u16;
        pos += chunk_size;
    }

    fragments
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reassembly_key() {
        let src = Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 1);
        let dst = Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 2);
        let key = ReassemblyKey::new(src, dst, 12345);

        assert_eq!(key.source_addr, src);
        assert_eq!(key.dest_addr, dst);
        assert_eq!(key.identification, 12345);
    }

    #[test]
    fn test_fragment_info_overlap() {
        let frag1 = FragmentInfo::new(0, true, vec![1u8; 16]);
        let frag2 = FragmentInfo::new(2, true, vec![2u8; 16]); // offset=16 bytes

        // frag1: 0-16 bytes, frag2: 16-32 bytes，不重叠
        assert!(!frag1.overlaps_with(&frag2));

        let frag3 = FragmentInfo::new(1, true, vec![3u8; 16]); // offset=8 bytes
        // frag1: 0-16 bytes, frag3: 8-24 bytes，重叠
        assert!(frag1.overlaps_with(&frag3));
    }

    #[test]
    fn test_reassembly_entry() {
        let key = ReassemblyKey::new(
            Ipv6Addr::UNSPECIFIED,
            Ipv6Addr::UNSPECIFIED,
            12345,
        );

        let mut entry = ReassemblyEntry::new(key);

        // 添加两个分片
        let frag1 = FragmentInfo::new(0, true, vec![1u8; 16]);
        let frag2 = FragmentInfo::new(2, false, vec![2u8; 8]);

        assert!(entry.add_fragment(frag1).is_ok());
        assert!(entry.add_fragment(frag2).is_ok());

        // 检查总长度
        assert_eq!(entry.total_length, Some(24)); // offset=2 means 16 bytes + 8 bytes
    }

    #[test]
    fn test_reassembly_complete() {
        let key = ReassemblyKey::new(
            Ipv6Addr::UNSPECIFIED,
            Ipv6Addr::UNSPECIFIED,
            12345,
        );

        let mut entry = ReassemblyEntry::new(key);

        // 添加完整的分片序列
        let frag1 = FragmentInfo::new(0, true, vec![1u8; 16]); // 0-15
        let frag2 = FragmentInfo::new(2, false, vec![2u8; 8]);  // 16-23

        entry.add_fragment(frag1).unwrap();
        entry.add_fragment(frag2).unwrap();

        assert!(entry.is_complete());

        // 重组数据
        let reassembled = entry.reassemble().unwrap();
        assert_eq!(reassembled.len(), 24);
        assert_eq!(&reassembled[0..16], &[1u8; 16]);
        assert_eq!(&reassembled[16..24], &[2u8; 8]);
    }

    #[test]
    fn test_reassembly_overlap_detection() {
        let key = ReassemblyKey::new(
            Ipv6Addr::UNSPECIFIED,
            Ipv6Addr::UNSPECIFIED,
            12345,
        );

        let mut entry = ReassemblyEntry::new(key);

        // 添加第一个分片
        let frag1 = FragmentInfo::new(0, true, vec![1u8; 16]);
        entry.add_fragment(frag1).unwrap();

        // 尝试添加重叠的分片
        let frag2 = FragmentInfo::new(1, true, vec![2u8; 16]); // 重叠
        assert!(entry.add_fragment(frag2).is_err());
    }

    #[test]
    fn test_fragment_cache() {
        let mut cache = FragmentCache::new(2);

        let key = ReassemblyKey::new(
            Ipv6Addr::UNSPECIFIED,
            Ipv6Addr::UNSPECIFIED,
            12345,
        );

        // 添加不完整的分片
        let frag1 = FragmentInfo::new(0, true, vec![1u8; 16]);
        let result = cache.add_fragment(key, frag1).unwrap();
        assert!(result.is_none());
        assert_eq!(cache.len(), 1);

        // 添加最后一片
        let frag2 = FragmentInfo::new(2, false, vec![2u8; 8]);
        let result = cache.add_fragment(key, frag2).unwrap();
        assert!(result.is_some());
        assert_eq!(cache.len(), 0); // 应该被移除
    }

    #[test]
    fn test_create_fragments() {
        let data = vec![1u8; 100]; // 100 字节数据
        let mtu = 60; // MTU 60 字节

        let fragments = create_fragments(&data, mtu, 12345, 58);

        // 每个分片最多 (60 - 8) / 8 * 8 = 48 字节（8字节对齐）
        // 100 字节需要 3 个分片: 48 + 48 + 4
        assert_eq!(fragments.len(), 3);

        // 检查第一个分片
        assert_eq!(fragments[0].0, 0); // offset
        assert!(fragments[0].1); // more_fragments
        assert_eq!(fragments[0].2.len(), 48);

        // 检查第二个分片
        assert_eq!(fragments[1].0, 6); // offset = 48 / 8
        assert!(fragments[1].1); // more_fragments
        assert_eq!(fragments[1].2.len(), 48);

        // 检查最后一个分片
        assert_eq!(fragments[2].0, 12); // offset = 96 / 8
        assert!(!fragments[2].1); // more_fragments = false
        assert_eq!(fragments[2].2.len(), 4);
    }

    #[test]
    fn test_create_fragments_no_fragmentation_needed() {
        let data = vec![1u8; 20]; // 20 字节数据
        let mtu = 60; // MTU 60 字节

        let fragments = create_fragments(&data, mtu, 12345, 58);

        // 不需要分片
        assert_eq!(fragments.len(), 1);
        assert_eq!(fragments[0].0, 0);
        assert!(!fragments[0].1);
        assert_eq!(fragments[0].2.len(), 20);
    }

    #[test]
    fn test_reassembly_entry_timeout() {
        let key = ReassemblyKey::new(
            Ipv6Addr::UNSPECIFIED,
            Ipv6Addr::UNSPECIFIED,
            12345,
        );

        let entry = ReassemblyEntry::new(key);

        // 新创建的条目不应该超时
        assert!(!entry.is_expired());
    }
}
