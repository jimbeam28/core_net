// src/protocols/ospf3/lsdb.rs
//
// OSPFv3 链路状态数据库

use std::collections::HashMap;
use std::time::Instant;

use super::packet::LsaHeader;

/// LSA 数据库条目
#[derive(Debug, Clone)]
pub struct LsaEntry {
    /// LSA 头部
    pub header: LsaHeader,
    /// LSA 完整数据
    pub data: Vec<u8>,
    /// 安装时间
    pub installed_at: Instant,
}

impl LsaEntry {
    pub fn new(header: LsaHeader, data: Vec<u8>) -> Self {
        Self {
            header,
            data,
            installed_at: Instant::now(),
        }
    }

    /// 检查 LSA 是否已过期
    pub fn is_expired(&self) -> bool {
        self.header.age >= 3600
    }
}

/// OSPFv3 链路状态数据库
#[derive(Debug, Clone)]
pub struct LinkStateDatabasev3 {
    /// LSA 存储：Key = (lsa_type, link_state_id, advertising_router)
    lsas: HashMap<(u16, u32, u32), LsaEntry>,
}

impl LinkStateDatabasev3 {
    pub fn new() -> Self {
        Self {
            lsas: HashMap::new(),
        }
    }

    /// 安装 LSA
    pub fn install(&mut self, header: LsaHeader, data: Vec<u8>) -> Result<(), String> {
        let key = (header.lsa_type, header.link_state_id, header.advertising_router);

        // 检查是否需要更新
        if let Some(existing) = self.lsas.get(&key) {
            // 如果新 LSA 更新，则替换
            if header.sequence_number > existing.header.sequence_number {
                self.lsas.insert(key, LsaEntry::new(header, data));
            }
        } else {
            self.lsas.insert(key, LsaEntry::new(header, data));
        }

        Ok(())
    }

    /// 查找 LSA
    pub fn lookup(&self, lsa_type: u16, link_state_id: u32, advertising_router: u32) -> Option<&LsaEntry> {
        self.lsas.get(&(lsa_type, link_state_id, advertising_router))
    }

    /// 移除 LSA
    pub fn remove(&mut self, lsa_type: u16, link_state_id: u32, advertising_router: u32) -> bool {
        self.lsas.remove(&(lsa_type, link_state_id, advertising_router)).is_some()
    }

    /// 清空数据库
    pub fn clear(&mut self) {
        self.lsas.clear();
    }

    /// 数据库是否为空
    pub fn is_empty(&self) -> bool {
        self.lsas.is_empty()
    }

    /// 数据库中的 LSA 数量
    pub fn len(&self) -> usize {
        self.lsas.len()
    }

    /// 清理过期的 LSA
    pub fn purge_expired(&mut self) -> usize {
        let mut expired_keys = Vec::new();

        for (key, entry) in &self.lsas {
            if entry.is_expired() {
                expired_keys.push(*key);
            }
        }

        let count = expired_keys.len();
        for key in expired_keys {
            self.lsas.remove(&key);
        }

        count
    }
}

impl Default for LinkStateDatabasev3 {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lsdb_new() {
        let lsdb = LinkStateDatabasev3::new();
        assert!(lsdb.is_empty());
        assert_eq!(lsdb.len(), 0);
    }

    #[test]
    fn test_lsdb_install_lookup() {
        let mut lsdb = LinkStateDatabasev3::new();

        let header = LsaHeader::new(
            0x2001,
            0x00000001,
            0x00000001,
        );

        lsdb.install(header.clone(), vec![0u8; 20]).unwrap();

        assert!(!lsdb.is_empty());
        assert_eq!(lsdb.len(), 1);

        let entry = lsdb.lookup(0x2001, header.link_state_id, header.advertising_router);
        assert!(entry.is_some());
    }
}
