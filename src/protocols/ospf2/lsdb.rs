// src/protocols/ospf2/lsdb.rs
//
// OSPFv2 链路状态数据库

use crate::common::Ipv4Addr;
use std::collections::HashMap;
use std::time::Instant;

/// LSA 条目
#[derive(Debug, Clone)]
pub struct LsaEntry {
    /// LSA 头部
    pub header: super::lsa::LsaHeader,

    /// 完整 LSA 数据
    pub data: Vec<u8>,

    /// 安装时间
    pub installed_at: Instant,

    /// 是否需要洪泛
    pub need_flooding: bool,
}

impl LsaEntry {
    /// 创建新的 LSA 条目
    pub fn new(header: super::lsa::LsaHeader, data: Vec<u8>) -> Self {
        Self {
            header,
            data,
            installed_at: Instant::now(),
            need_flooding: false,
        }
    }

    /// 检查 LSA 是否已过期
    pub fn is_expired(&self) -> bool {
        self.header.age >= super::lsa::LsaHeader::MAX_AGE
    }

    /// 检查是否需要刷新
    pub fn needs_refresh(&self) -> bool {
        // LSA 刷新间隔是 1800 秒（30 分钟）
        let refresh_interval = std::time::Duration::from_secs(1800);
        self.installed_at.elapsed() > refresh_interval
    }
}

/// 链路状态数据库
#[derive(Debug, Clone)]
pub struct LinkStateDatabase {
    /// LSA 存储：键为 (LSA Type, Link State ID, Advertising Router)
    pub lsas: HashMap<(u8, Ipv4Addr, Ipv4Addr), LsaEntry>,
}

impl LinkStateDatabase {
    /// 创建新的 LSDB
    pub fn new() -> Self {
        Self {
            lsas: HashMap::new(),
        }
    }

    /// 添加或更新 LSA
    pub fn install(&mut self, lsa: super::lsa::Lsa) -> Result<(), String> {
        let header = lsa.header().clone();
        let key = (header.lsa_type, header.link_state_id, header.advertising_router);
        let data = lsa.to_bytes();

        let entry = LsaEntry::new(header, data);
        self.lsas.insert(key, entry);

        Ok(())
    }

    /// 查找 LSA
    pub fn lookup(
        &self,
        lsa_type: u8,
        link_state_id: Ipv4Addr,
        advertising_router: Ipv4Addr,
    ) -> Option<&LsaEntry> {
        self.lsas.get(&(lsa_type, link_state_id, advertising_router))
    }

    /// 移除 LSA
    pub fn remove(
        &mut self,
        lsa_type: u8,
        link_state_id: Ipv4Addr,
        advertising_router: Ipv4Addr,
    ) -> bool {
        self.lsas.remove(&(lsa_type, link_state_id, advertising_router)).is_some()
    }

    /// 获取区域内所有 Router LSA
    pub fn get_router_lsas(&self) -> Vec<&LsaEntry> {
        self.lsas.values()
            .filter(|entry| entry.header.lsa_type == 1)
            .collect()
    }

    /// 获取区域内所有 Network LSA
    pub fn get_network_lsas(&self) -> Vec<&LsaEntry> {
        self.lsas.values()
            .filter(|entry| entry.header.lsa_type == 2)
            .collect()
    }

    /// 清空数据库
    pub fn clear(&mut self) {
        self.lsas.clear();
    }

    /// 获取 LSA 数量
    pub fn len(&self) -> usize {
        self.lsas.len()
    }

    /// 检查数据库是否为空
    pub fn is_empty(&self) -> bool {
        self.lsas.is_empty()
    }

    /// 清理过期的 LSA
    pub fn purge_expired(&mut self) -> usize {
        let mut expired_keys = Vec::new();

        for (key, entry) in &self.lsas {
            if entry.is_expired() {
                expired_keys.push(key.clone());
            }
        }

        let count = expired_keys.len();
        for key in expired_keys {
            self.lsas.remove(&key);
        }

        count
    }

    /// 构建用于 SPF 计算的 LSA 描述符哈希表
    ///
    /// 将 LSDB 中的 LSA 条目转换为 SPF 算法所需的 LsaDescriptor 格式
    pub fn build_lsa_descriptors(&self) -> std::collections::HashMap<(u8, Ipv4Addr, Ipv4Addr), crate::protocols::ospf::spf::LsaDescriptor> {
        use crate::protocols::ospf::spf::{LsaDescriptor, LsaLink};
        use crate::protocols::ospf2::lsa::{RouterLink, LsaHeader};

        let mut descriptors = std::collections::HashMap::new();

        for (key, entry) in &self.lsas {
            let (lsa_type, link_state_id, advertising_router) = *key;

            // 只处理 Router LSA (Type 1) 和 Network LSA (Type 2)
            if lsa_type == 1 {
                // Router LSA: 解析链路信息
                let mut links = Vec::new();
                let lsa_data = &entry.data[LsaHeader::LENGTH..];

                // Router LSA 数据格式：每个链路 12 字节
                let num_links = lsa_data.len() / 12;
                for i in 0..num_links {
                    let offset = i * 12;
                    if offset + 12 <= lsa_data.len() {
                        if let Ok(router_link) = RouterLink::from_bytes(&lsa_data[offset..offset+12]) {
                            links.push(LsaLink {
                                link_id: router_link.link_id,
                                link_data: router_link.link_data,
                                link_type: router_link.link_type,
                                metric: router_link.metric as u32,
                            });
                        }
                    }
                }

                let descriptor = LsaDescriptor::router_lsa(
                    link_state_id,
                    advertising_router,
                    entry.header.sequence_number,
                    links,
                );
                descriptors.insert(*key, descriptor);

            } else if lsa_type == 2 {
                // Network LSA: 提取连接的路由器列表
                let mut attached_routers = Vec::new();
                let lsa_data = &entry.data[LsaHeader::LENGTH..];

                // Network LSA 数据格式：每个路由器 4 字节
                let num_routers = lsa_data.len() / 4;
                for i in 0..num_routers {
                    let offset = i * 4;
                    if offset + 4 <= lsa_data.len() {
                        let router_bytes: [u8; 4] = lsa_data[offset..offset+4].try_into().unwrap();
                        attached_routers.push(Ipv4Addr::from_bytes(router_bytes));
                    }
                }

                let descriptor = LsaDescriptor::network_lsa(
                    link_state_id,
                    advertising_router,
                    entry.header.sequence_number,
                    attached_routers,
                );
                descriptors.insert(*key, descriptor);
            }
        }

        descriptors
    }
}

impl Default for LinkStateDatabase {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocols::ospf2::lsa::{RouterLsa, RouterLink, Lsa, LsaHeader};

    #[test]
    fn test_lsdb_new() {
        let lsdb = LinkStateDatabase::new();
        assert!(lsdb.is_empty());
        assert_eq!(lsdb.len(), 0);
    }

    #[test]
    fn test_lsdb_install_lookup() {
        let mut lsdb = LinkStateDatabase::new();

        // 创建一个简单的 Router LSA
        let mut lsa = RouterLsa::new(
            Ipv4Addr::new(1, 1, 1, 1),
            Ipv4Addr::new(1, 1, 1, 1),
        );

        let link = RouterLink::new(
            RouterLink::TYPE_POINT_TO_POINT,
            Ipv4Addr::new(1, 1, 1, 2),
            Ipv4Addr::new(10, 0, 0, 1),
            10,
        );
        lsa.add_link(link);

        lsdb.install(Lsa::Router(lsa)).unwrap();

        assert!(!lsdb.is_empty());
        assert_eq!(lsdb.len(), 1);

        let entry = lsdb.lookup(1, Ipv4Addr::new(1, 1, 1, 1), Ipv4Addr::new(1, 1, 1, 1));
        assert!(entry.is_some());
    }

    #[test]
    fn test_lsdb_remove() {
        let mut lsdb = LinkStateDatabase::new();

        let lsa = RouterLsa::new(
            Ipv4Addr::new(1, 1, 1, 1),
            Ipv4Addr::new(1, 1, 1, 1),
        );

        lsdb.install(Lsa::Router(lsa)).unwrap();

        assert!(lsdb.remove(1, Ipv4Addr::new(1, 1, 1, 1), Ipv4Addr::new(1, 1, 1, 1)));
        assert!(lsdb.is_empty());
    }

    #[test]
    fn test_lsdb_clear() {
        let mut lsdb = LinkStateDatabase::new();

        let lsa1 = RouterLsa::new(
            Ipv4Addr::new(1, 1, 1, 1),
            Ipv4Addr::new(1, 1, 1, 1),
        );

        let lsa2 = RouterLsa::new(
            Ipv4Addr::new(1, 1, 1, 2),
            Ipv4Addr::new(1, 1, 1, 2),
        );

        lsdb.install(Lsa::Router(lsa1)).unwrap();
        lsdb.install(Lsa::Router(lsa2)).unwrap();

        assert_eq!(lsdb.len(), 2);

        lsdb.clear();
        assert!(lsdb.is_empty());
    }

    #[test]
    fn test_lsa_entry_is_expired() {
        let header = LsaHeader::new(
            1,
            Ipv4Addr::new(1, 1, 1, 1),
            Ipv4Addr::new(1, 1, 1, 1),
        );
        let entry = LsaEntry::new(header, vec![0u8; 20]);

        assert!(!entry.is_expired());

        let mut expired_header = LsaHeader::new(
            1,
            Ipv4Addr::new(1, 1, 1, 1),
            Ipv4Addr::new(1, 1, 1, 1),
        );
        expired_header.age = 3600;
        let expired_entry = LsaEntry::new(expired_header, vec![0u8; 20]);

        assert!(expired_entry.is_expired());
    }
}
