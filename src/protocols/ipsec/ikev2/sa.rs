// src/protocols/ipsec/ikev2/sa.rs
//
// IKEv2 SA 管理和状态

use super::*;
use std::time::{Duration, Instant};
use std::collections::HashMap;
use crate::common::addr::IpAddr;

// ========== IKE SA 标识 ==========

/// IKE SA 标识
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct IkeSaId {
    /// 发起方 SPI
    pub initiator_spi: [u8; SPI_LEN],
    /// 响应方 SPI
    pub responder_spi: [u8; SPI_LEN],
}

impl IkeSaId {
    /// 创建新的 IKE SA 标识
    pub fn new(initiator_spi: [u8; SPI_LEN], responder_spi: [u8; SPI_LEN]) -> Self {
        Self {
            initiator_spi,
            responder_spi,
        }
    }

    /// 检查是否匹配（忽略 0 SPI）
    pub fn matches(&self, initiator_spi: &[u8], responder_spi: &[u8]) -> bool {
        let init_match = self.initiator_spi == *initiator_spi ||
            self.initiator_spi == [0u8; SPI_LEN] ||
            *initiator_spi == [0u8; SPI_LEN];
        let resp_match = self.responder_spi == *responder_spi ||
            self.responder_spi == [0u8; SPI_LEN] ||
            *responder_spi == [0u8; SPI_LEN];
        init_match && resp_match
    }
}

// ========== 角色 ==========

/// IKE SA 角色
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IkeRole {
    /// 发起方
    Initiator,
    /// 响应方
    Responder,
}

// ========== IKE SA 状态 ==========

/// IKE SA 状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IkeSaState {
    /// 空闲状态
    Idle,
    /// 已发送 IKE_SA_INIT 请求
    InitSent,
    /// 已发送 IKE_AUTH 请求
    AuthSent,
    /// IKE SA 已建立
    Established,
    /// 已删除
    Deleted,
}

impl IkeSaState {
    /// 检查是否可以处理数据包
    pub fn can_process(&self) -> bool {
        matches!(self, Self::Established)
    }

    /// 检查是否为终止状态
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Deleted)
    }
}

// ========== IKE 密钥材料 ==========

/// IKE 密钥材料
#[derive(Debug, Clone)]
pub struct IkeKeyMaterial {
    /// 用于派生密钥的材料
    pub sk_d: Vec<u8>,
    /// 发起方->响应方 完整性密钥
    pub sk_ai: Vec<u8>,
    /// 响应方->发起方 完整性密钥
    pub sk_ar: Vec<u8>,
    /// 发起方->响应方 加密密钥
    pub sk_ei: Vec<u8>,
    /// 响应方->发起方 加密密钥
    pub sk_er: Vec<u8>,
    /// 发起方->响应方 PRF 密钥
    pub sk_pi: Vec<u8>,
    /// 响应方->发起方 PRF 密钥
    pub sk_pr: Vec<u8>,
}

impl IkeKeyMaterial {
    /// 创建新的密钥材料
    pub fn new(
        sk_d: Vec<u8>,
        sk_ai: Vec<u8>,
        sk_ar: Vec<u8>,
        sk_ei: Vec<u8>,
        sk_er: Vec<u8>,
        sk_pi: Vec<u8>,
        sk_pr: Vec<u8>,
    ) -> Self {
        Self {
            sk_d,
            sk_ai,
            sk_ar,
            sk_ei,
            sk_er,
            sk_pi,
            sk_pr,
        }
    }

    /// 获取发起方的加密密钥
    pub fn initiator_enc_key(&self) -> &[u8] {
        &self.sk_ei
    }

    /// 获取发起方的认证密钥
    pub fn initiator_auth_key(&self) -> &[u8] {
        &self.sk_ai
    }

    /// 获取响应方的加密密钥
    pub fn responder_enc_key(&self) -> &[u8] {
        &self.sk_er
    }

    /// 获取响应方的认证密钥
    pub fn responder_auth_key(&self) -> &[u8] {
        &self.sk_ar
    }

    /// 清除密钥材料（安全清理）
    pub fn clear(&mut self) {
        self.sk_d.clear();
        self.sk_ai.clear();
        self.sk_ar.clear();
        self.sk_ei.clear();
        self.sk_er.clear();
        self.sk_pi.clear();
        self.sk_pr.clear();
    }
}

impl Drop for IkeKeyMaterial {
    fn drop(&mut self) {
        self.clear();
    }
}

// ========== IKE SA 配置 ==========

/// IKE SA 配置
#[derive(Debug, Clone)]
pub struct IkeSaConfig {
    /// 角色
    pub role: IkeRole,
    /// 本地地址
    pub local_addr: IpAddr,
    /// 远端地址
    pub remote_addr: IpAddr,
    /// DH 组
    pub dh_group: IkeDhGroup,
    /// 认证方法
    pub auth_method: IkeAuthMethod,
    /// 预共享密钥（PSK 认证时）
    pub psk: Option<Vec<u8>>,
    /// SA 生命周期（软）
    pub lifetime_soft: Duration,
    /// SA 生命周期（硬）
    pub lifetime_hard: Duration,
    /// 是否启用 NAT 穿透
    pub nat_traversal: bool,
}

impl IkeSaConfig {
    /// 创建新的 IKE SA 配置
    pub fn new(
        role: IkeRole,
        local_addr: IpAddr,
        remote_addr: IpAddr,
        dh_group: IkeDhGroup,
        auth_method: IkeAuthMethod,
    ) -> Self {
        Self {
            role,
            local_addr,
            remote_addr,
            dh_group,
            auth_method,
            psk: None,
            lifetime_soft: Duration::from_secs(14400), // 4 小时
            lifetime_hard: Duration::from_secs(28800), // 8 小时
            nat_traversal: true,
        }
    }

    /// 设置预共享密钥
    pub fn with_psk(mut self, psk: Vec<u8>) -> Self {
        self.psk = Some(psk);
        self
    }

    /// 设置生命周期
    pub fn with_lifetime(mut self, soft: Duration, hard: Duration) -> Self {
        self.lifetime_soft = soft;
        self.lifetime_hard = hard;
        self
    }

    /// 设置 NAT 穿透
    pub fn with_nat_traversal(mut self, enabled: bool) -> Self {
        self.nat_traversal = enabled;
        self
    }
}

impl Default for IkeSaConfig {
    fn default() -> Self {
        Self {
            role: IkeRole::Initiator,
            local_addr: IpAddr::V4(crate::common::addr::Ipv4Addr::new(0, 0, 0, 0)),
            remote_addr: IpAddr::V4(crate::common::addr::Ipv4Addr::new(0, 0, 0, 0)),
            dh_group: IkeDhGroup::MODP2048,
            auth_method: IkeAuthMethod::SHARED_KEY,
            psk: None,
            lifetime_soft: Duration::from_secs(14400),
            lifetime_hard: Duration::from_secs(28800),
            nat_traversal: true,
        }
    }
}

// ========== IKE SA 条目 ==========

/// IKE SA 条目
#[derive(Debug, Clone)]
pub struct IkeSaEntry {
    /// IKE SA 标识
    pub id: IkeSaId,
    /// 角色
    pub role: IkeRole,
    /// 当前状态
    pub state: IkeSaState,
    /// 发起方 SPI
    pub initiator_spi: [u8; SPI_LEN],
    /// 响应方 SPI
    pub responder_spi: [u8; SPI_LEN],
    /// 消息 ID（下一条消息的 ID）
    pub message_id: u32,
    /// 本地 SPI
    pub local_spi: [u8; SPI_LEN],
    /// 远端 SPI
    pub remote_spi: [u8; SPI_LEN],
    /// DH 组
    pub dh_group: IkeDhGroup,
    /// 密钥材料
    pub keymat: Option<IkeKeyMaterial>,
    /// 对端地址
    pub remote_addr: IpAddr,
    /// 本地地址
    pub local_addr: IpAddr,
    /// 创建时间
    pub created_at: Instant,
    /// 生命周期（秒）
    pub lifetime: Duration,
    /// 关联的 CHILD SA SPIs
    pub child_sas: Vec<u32>,
    /// 本地 Nonce
    pub local_nonce: Vec<u8>,
    /// 远端 Nonce
    pub remote_nonce: Vec<u8>,
    /// DH 公钥
    pub local_public_key: Vec<u8>,
    /// DH 共享密钥
    pub dh_shared: Vec<u8>,
}

impl IkeSaEntry {
    /// 创建新的 IKE SA 条目
    pub fn new(config: IkeSaConfig, initiator_spi: [u8; SPI_LEN]) -> Self {
        let local_spi = if matches!(config.role, IkeRole::Initiator) {
            initiator_spi
        } else {
            [0u8; SPI_LEN] // 响应方稍后设置
        };

        Self {
            id: IkeSaId::new(initiator_spi, [0u8; SPI_LEN]),
            role: config.role,
            state: IkeSaState::Idle,
            initiator_spi,
            responder_spi: [0u8; SPI_LEN],
            message_id: 0,
            local_spi,
            remote_spi: [0u8; SPI_LEN],
            dh_group: config.dh_group,
            keymat: None,
            remote_addr: config.remote_addr,
            local_addr: config.local_addr,
            created_at: Instant::now(),
            lifetime: config.lifetime_hard,
            child_sas: Vec::new(),
            local_nonce: Vec::new(),
            remote_nonce: Vec::new(),
            local_public_key: Vec::new(),
            dh_shared: Vec::new(),
        }
    }

    /// 设置响应方 SPI
    pub fn set_responder_spi(&mut self, responder_spi: [u8; SPI_LEN]) {
        self.responder_spi = responder_spi;
        self.id.responder_spi = responder_spi;
        if matches!(self.role, IkeRole::Responder) {
            self.local_spi = responder_spi;
        }
        self.remote_spi = if matches!(self.role, IkeRole::Initiator) {
            responder_spi
        } else {
            self.initiator_spi
        };
    }

    /// 设置状态
    pub fn set_state(&mut self, state: IkeSaState) {
        self.state = state;
    }

    /// 增加消息 ID
    pub fn next_message_id(&mut self) -> u32 {
        let id = self.message_id;
        self.message_id += 1;
        id
    }

    /// 设置密钥材料
    pub fn set_keymat(&mut self, keymat: IkeKeyMaterial) {
        self.keymat = Some(keymat);
    }

    /// 检查是否过期
    pub fn is_expired(&self) -> bool {
        self.created_at.elapsed() >= self.lifetime
    }

    /// 检查是否即将过期（90%）
    pub fn is_dying(&self) -> bool {
        let age = self.created_at.elapsed();
        let threshold = self.lifetime * 9 / 10;
        age >= threshold && !self.is_expired()
    }

    /// 获取加密密钥
    pub fn get_enc_key(&self, is_initiator: bool) -> Option<&[u8]> {
        self.keymat.as_ref().map(|k| {
            if is_initiator {
                k.initiator_enc_key()
            } else {
                k.responder_enc_key()
            }
        })
    }

    /// 获取认证密钥
    pub fn get_auth_key(&self, is_initiator: bool) -> Option<&[u8]> {
        self.keymat.as_ref().map(|k| {
            if is_initiator {
                k.initiator_auth_key()
            } else {
                k.responder_auth_key()
            }
        })
    }

    /// 添加 CHILD SA
    pub fn add_child_sa(&mut self, spi: u32) {
        if !self.child_sas.contains(&spi) {
            self.child_sas.push(spi);
        }
    }

    /// 移除 CHILD SA
    pub fn remove_child_sa(&mut self, spi: u32) {
        self.child_sas.retain(|&s| s != spi);
    }

    /// 清理资源
    pub fn cleanup(&mut self) {
        if let Some(mut keymat) = self.keymat.take() {
            keymat.clear();
        }
        self.local_nonce.clear();
        self.remote_nonce.clear();
        self.local_public_key.clear();
        self.dh_shared.clear();
    }
}

impl Drop for IkeSaEntry {
    fn drop(&mut self) {
        self.cleanup();
    }
}

// ========== IKE SA 管理器 ==========

/// IKE SA 管理器
#[derive(Debug)]
pub struct IkeSaManager {
    /// IKE SA 表，键为 (发起方 SPI, 响应方 SPI)
    sas: HashMap<IkeSaId, IkeSaEntry>,
    /// SPI 到 SA ID 的映射（用于快速查找）
    spi_to_id: HashMap<[u8; SPI_LEN], IkeSaId>,
    /// 远端地址到 SA ID 的映射
    addr_to_id: HashMap<IpAddr, Vec<IkeSaId>>,
}

impl IkeSaManager {
    /// 创建新的 IKE SA 管理器
    pub fn new() -> Self {
        Self {
            sas: HashMap::new(),
            spi_to_id: HashMap::new(),
            addr_to_id: HashMap::new(),
        }
    }

    /// 添加 IKE SA
    pub fn add(&mut self, sa: IkeSaEntry) -> IkeResult<()> {
        let id = sa.id;

        // 添加到 SPI 映射
        self.spi_to_id.insert(sa.initiator_spi, id);
        if sa.responder_spi != [0u8; SPI_LEN] {
            self.spi_to_id.insert(sa.responder_spi, id);
        }

        // 添加到地址映射
        self.addr_to_id
            .entry(sa.remote_addr)
            .or_insert_with(Vec::new)
            .push(id);

        self.sas.insert(id, sa);
        Ok(())
    }

    /// 通过 SPI 查找 IKE SA
    pub fn get_by_spi(&self, spi: &[u8; SPI_LEN]) -> Option<&IkeSaEntry> {
        let id = self.spi_to_id.get(spi)?;
        self.sas.get(id)
    }

    /// 获取可变的 IKE SA（通过 SPI）
    pub fn get_by_spi_mut(&mut self, spi: &[u8; SPI_LEN]) -> Option<&mut IkeSaEntry> {
        let id = self.spi_to_id.get(spi)?.clone();
        self.sas.get_mut(&id)
    }

    /// 通过 IKE SA ID 查找
    pub fn get(&self, id: &IkeSaId) -> Option<&IkeSaEntry> {
        self.sas.get(id)
    }

    /// 获取可变的 IKE SA
    pub fn get_mut(&mut self, id: &IkeSaId) -> Option<&mut IkeSaEntry> {
        self.sas.get_mut(id)
    }

    /// 通过远端地址查找所有 IKE SA
    pub fn get_by_remote_addr(&self, addr: &IpAddr) -> Vec<&IkeSaEntry> {
        self.addr_to_id
            .get(addr)
            .map(|ids| {
                ids.iter()
                    .filter_map(|id| self.sas.get(id))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// 删除 IKE SA
    pub fn remove(&mut self, id: &IkeSaId) -> Option<IkeSaEntry> {
        let sa = self.sas.remove(id)?;

        // 清理 SPI 映射
        self.spi_to_id.remove(&sa.initiator_spi);
        if sa.responder_spi != [0u8; SPI_LEN] {
            self.spi_to_id.remove(&sa.responder_spi);
        }

        // 清理地址映射
        if let Some(ids) = self.addr_to_id.get_mut(&sa.remote_addr) {
            ids.retain(|i| i != id);
            if ids.is_empty() {
                self.addr_to_id.remove(&sa.remote_addr);
            }
        }

        Some(sa)
    }

    /// 获取所有 IKE SA
    pub fn all(&self) -> impl Iterator<Item = &IkeSaEntry> {
        self.sas.values()
    }

    /// 清理过期的 IKE SA
    pub fn cleanup_expired(&mut self) {
        let expired_ids: Vec<IkeSaId> = self.sas
            .iter()
            .filter(|(_, sa)| sa.is_expired())
            .map(|(id, _)| *id)
            .collect();

        for id in expired_ids {
            self.remove(&id);
        }
    }

    /// IKE SA 数量
    pub fn len(&self) -> usize {
        self.sas.len()
    }

    /// 是否为空
    pub fn is_empty(&self) -> bool {
        self.sas.is_empty()
    }
}

impl Default for IkeSaManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::addr::Ipv4Addr;

    #[test]
    fn test_sa_id() {
        let id = IkeSaId::new(
            [0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08],
            [0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18],
        );

        assert!(id.matches(
            &[0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08],
            &[0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18],
        ));

        // 测试忽略 0 SPI 匹配
        assert!(id.matches(
            &[0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08],
            &[0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
        ));
    }

    #[test]
    fn test_sa_state() {
        assert!(!IkeSaState::Idle.can_process());
        assert!(IkeSaState::Established.can_process());
        assert!(IkeSaState::Deleted.is_terminal());
        assert!(!IkeSaState::Established.is_terminal());
    }

    #[test]
    fn test_key_material() {
        let keymat = IkeKeyMaterial::new(
            vec![1u8; 32],
            vec![2u8; 32],
            vec![3u8; 32],
            vec![4u8; 32],
            vec![5u8; 32],
            vec![6u8; 32],
            vec![7u8; 32],
        );

        assert_eq!(keymat.initiator_enc_key(), &[4u8; 32]);
        assert_eq!(keymat.responder_enc_key(), &[5u8; 32]);
        assert_eq!(keymat.initiator_auth_key(), &[2u8; 32]);
        assert_eq!(keymat.responder_auth_key(), &[3u8; 32]);
    }

    #[test]
    fn test_sa_entry() {
        let config = IkeSaConfig::new(
            IkeRole::Initiator,
            IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1)),
            IpAddr::V4(Ipv4Addr::new(192, 168, 1, 2)),
            IkeDhGroup::MODP2048,
            IkeAuthMethod::SHARED_KEY,
        );

        let spi = [0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08];
        let mut sa = IkeSaEntry::new(config, spi);

        assert_eq!(sa.initiator_spi, spi);
        assert_eq!(sa.state, IkeSaState::Idle);

        sa.set_responder_spi([0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18]);
        sa.set_state(IkeSaState::InitSent);

        assert_eq!(sa.responder_spi, [0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18]);
        assert_eq!(sa.state, IkeSaState::InitSent);
    }

    #[test]
    fn test_sa_manager() {
        let mut manager = IkeSaManager::new();

        let config = IkeSaConfig::new(
            IkeRole::Initiator,
            IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1)),
            IpAddr::V4(Ipv4Addr::new(192, 168, 1, 2)),
            IkeDhGroup::MODP2048,
            IkeAuthMethod::SHARED_KEY,
        );

        let spi = [0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08];
        let sa = IkeSaEntry::new(config, spi);

        manager.add(sa.clone()).unwrap();
        assert_eq!(manager.len(), 1);

        let found = manager.get_by_spi(&spi);
        assert!(found.is_some());
        assert_eq!(found.unwrap().initiator_spi, spi);

        manager.remove(&sa.id);
        assert_eq!(manager.len(), 0);
    }

    #[test]
    fn test_sa_config_default() {
        let config = IkeSaConfig::default();
        assert_eq!(config.role, IkeRole::Initiator);
        assert_eq!(config.dh_group, IkeDhGroup::MODP2048);
        assert_eq!(config.auth_method, IkeAuthMethod::SHARED_KEY);
        assert_eq!(config.lifetime_soft, Duration::from_secs(14400));
        assert_eq!(config.lifetime_hard, Duration::from_secs(28800));
    }
}
