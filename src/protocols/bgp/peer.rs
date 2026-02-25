// src/protocols/bgp/peer.rs
//
// BGP 对等体状态机和管理实现

use std::net::IpAddr;
use std::time::Duration;
use crate::common::addr::Ipv4Addr as CoreIpv4Addr;
use crate::protocols::bgp::{
    config::BgpPeerConfig,
    rib::BgpRib,
    error::{BgpError, Result},
    BGP_VERSION, DEFAULT_HOLD_TIME,
};
use crate::protocols::bgp::message::{BgpMessage, BgpOpen, IpPrefix, BgpUpdate, BgpNotification, PathAttribute};
use crate::protocols::bgp::rib::BgpRoute;

/// BGP 对等体状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BgpState {
    /// 空闲状态
    Idle,
    /// 连接中
    Connect,
    /// 激活（监听入站连接）
    Active,
    /// OPEN 已发送
    OpenSent,
    /// OPEN 确认
    OpenConfirm,
    /// 已建立
    Established,
}

impl BgpState {
    /// 获取状态名称
    pub fn name(&self) -> &str {
        match self {
            BgpState::Idle => "Idle",
            BgpState::Connect => "Connect",
            BgpState::Active => "Active",
            BgpState::OpenSent => "OpenSent",
            BgpState::OpenConfirm => "OpenConfirm",
            BgpState::Established => "Established",
        }
    }

    /// 是否为活跃状态（已建立或正在建立连接）
    pub fn is_active(&self) -> bool {
        matches!(self, BgpState::OpenSent | BgpState::OpenConfirm | BgpState::Established)
    }

    /// 是否可以发送 UPDATE
    pub fn can_send_update(&self) -> bool {
        *self == BgpState::Established
    }
}

/// BGP 对等体
#[derive(Debug, Clone)]
pub struct BgpPeer {
    /// 对等体配置
    pub config: BgpPeerConfig,

    /// 当前状态
    pub state: BgpState,

    /// 对端 BGP 标识符
    pub remote_bgp_id: Option<CoreIpv4Addr>,

    /// 对端 AS 号
    pub remote_as: u32,

    /// 协商的 Hold Time
    pub hold_time: Duration,

    /// 协商的 Keepalive Time
    pub keepalive_time: Duration,

    /// 连接重试计数
    pub connect_retry_count: u32,

    /// 入站 RIB（存储从对等体接收的路由）
    pub adj_rib_in: BgpRib,

    /// 出站 RIB（存储准备发送给对等体的路由）
    pub adj_rib_out: BgpRib,

    /// 本地 BGP 标识符
    pub local_bgp_id: CoreIpv4Addr,

    /// 本地 AS 号
    pub local_as: u32,
}

impl BgpPeer {
    /// 创建新的对等体
    pub fn new(config: BgpPeerConfig, local_bgp_id: CoreIpv4Addr, local_as: u32) -> Self {
        let hold_time = Duration::from_secs(DEFAULT_HOLD_TIME as u64);
        let keepalive_time = Duration::from_secs((DEFAULT_HOLD_TIME / 3) as u64);

        Self {
            state: BgpState::Idle,
            remote_bgp_id: None,
            remote_as: config.remote_as,
            hold_time,
            keepalive_time,
            connect_retry_count: 0,
            adj_rib_in: BgpRib::new(),
            adj_rib_out: BgpRib::new(),
            local_bgp_id,
            local_as,
            config,
        }
    }

    /// 处理 BGP Start 事件（启动连接）
    pub fn bgp_start(&mut self) -> Result<()> {
        if self.state != BgpState::Idle {
            return Err(BgpError::InvalidPeerState(format!(
                "Expected Idle, got {}", self.state.name()
            )));
        }

        if self.config.passive {
            self.state = BgpState::Active;
        } else {
            self.state = BgpState::Connect;
        }

        Ok(())
    }

    /// 处理 TCP 连接成功
    pub fn tcp_connection_established(&mut self) -> Result<BgpOpen> {
        match self.state {
            BgpState::Connect | BgpState::Active => {
                self.state = BgpState::OpenSent;
                Ok(self.create_open_message())
            }
            _ => Err(BgpError::InvalidPeerState(format!(
                "Cannot connect in state {}", self.state.name()
            ))),
        }
    }

    /// 处理 TCP 连接失败
    pub fn tcp_connection_failed(&mut self) -> Result<()> {
        match self.state {
            BgpState::Connect => {
                self.connect_retry_count += 1;
                self.state = BgpState::Active;
                Ok(())
            }
            BgpState::Active => {
                self.connect_retry_count += 1;
                // 保持 Active 状态，等待重试
                Ok(())
            }
            _ => Err(BgpError::InvalidPeerState(format!(
                "Unexpected connection failure in state {}", self.state.name()
            ))),
        }
    }

    /// 处理接收到的 OPEN 消息
    pub fn handle_open(&mut self, open: &BgpOpen) -> Result<()> {
        if self.state != BgpState::OpenSent {
            return Err(BgpError::InvalidPeerState(format!(
                "Expected OpenSent, got {}", self.state.name()
            )));
        }

        // 验证版本
        if open.version != BGP_VERSION {
            return Err(BgpError::UnsupportedVersion(open.version));
        }

        // 验证 AS 号
        if open.my_as as u32 != self.remote_as {
            return Err(BgpError::InvalidPeerState(format!(
                "AS number mismatch: expected {}, got {}",
                self.remote_as, open.my_as
            )));
        }

        // 检查 BGP ID 冲突
        if open.bgp_identifier == self.local_bgp_id {
            return Err(BgpError::BgpIdentifierConflict);
        }

        // 协商 Hold Time
        let remote_hold = if open.hold_time == 0 { 0 } else { open.hold_time };
        let local_hold = DEFAULT_HOLD_TIME;
        let negotiated_hold = if remote_hold == 0 || local_hold == 0 {
            0
        } else {
            remote_hold.min(local_hold)
        };

        self.hold_time = Duration::from_secs(negotiated_hold as u64);
        self.keepalive_time = Duration::from_secs((negotiated_hold / 3) as u64);

        // 保存对端信息
        self.remote_bgp_id = Some(open.bgp_identifier);
        self.state = BgpState::OpenConfirm;

        Ok(())
    }

    /// 处理接收到的 KEEPALIVE 消息
    pub fn handle_keepalive(&mut self) -> Result<()> {
        match self.state {
            BgpState::OpenConfirm => {
                self.state = BgpState::Established;
                Ok(())
            }
            BgpState::Established => {
                // 保持 Established 状态
                Ok(())
            }
            _ => Err(BgpError::InvalidPeerState(format!(
                "Unexpected KEEPALIVE in state {}", self.state.name()
            ))),
        }
    }

    /// 处理接收到的 UPDATE 消息
    pub fn handle_update(&mut self, update: &BgpUpdate, local_as: u32) -> Result<()> {
        if self.state != BgpState::Established {
            return Err(BgpError::InvalidPeerState(format!(
                "Cannot handle UPDATE in state {}", self.state.name()
            )));
        }

        // 检查环路
        if let Some(as_attr) = update.path_attributes.iter()
            .find(|a| matches!(a, crate::protocols::bgp::message::PathAttribute::AsPath { .. })) {
            if let crate::protocols::bgp::message::PathAttribute::AsPath { as_sequence, .. } = as_attr {
                if as_sequence.contains(&local_as) {
                    return Err(BgpError::AsPathLoop);
                }
            }
        }

        // 处理撤销路由
        for prefix in &update.withdrawn_routes {
            self.adj_rib_in.remove(prefix);
        }

        // 添加新路由
        for prefix in &update.nlri {
            // 提取路径属性
            let next_hop = update.path_attributes.iter()
                .find_map(|a| {
                    if let PathAttribute::NextHop { next_hop } = a {
                        Some(IpAddr::V4(std::net::Ipv4Addr::new(
                            next_hop.bytes[0], next_hop.bytes[1],
                            next_hop.bytes[2], next_hop.bytes[3]
                        )))
                    } else {
                        None
                    }
                })
                .unwrap_or(self.config.address);

            let local_pref = update.path_attributes.iter()
                .find_map(|a| {
                    if let PathAttribute::LocalPref { local_pref } = a {
                        Some(local_pref)
                    } else {
                        None
                    }
                });

            let med = update.path_attributes.iter()
                .find_map(|a| {
                    if let PathAttribute::MultiExitDisc { med } = a {
                        Some(med)
                    } else {
                        None
                    }
                })
                .copied()
                .unwrap_or(0);

            let origin = update.path_attributes.iter()
                .find_map(|a| {
                    if let PathAttribute::Origin { origin } = a {
                        Some(origin)
                    } else {
                        None
                    }
                })
                .copied()
                .unwrap_or(0);

            let as_path = update.path_attributes.iter()
                .find_map(|a| {
                    if let PathAttribute::AsPath { as_sequence, .. } = a {
                        Some(as_sequence.clone())
                    } else {
                        None
                    }
                })
                .unwrap_or_default();

            let route = BgpRoute {
                prefix: prefix.clone(),
                next_hop,
                local_pref: local_pref.copied(),
                med,
                as_path,
                origin,
                peer: self.config.address,
                valid: true,
                age: 0,
            };

            self.adj_rib_in.add_or_update(route);
        }

        Ok(())
    }

    /// 处理接收到的 NOTIFICATION 消息
    pub fn handle_notification(&mut self, _notification: &BgpNotification) {
        // 无论在什么状态，收到 NOTIFICATION 都回到 Idle
        self.state = BgpState::Idle;
        self.adj_rib_in.clear();
        self.adj_rib_out.clear();
    }

    /// 处理 Hold Timer 超时
    pub fn hold_timer_expired(&mut self) {
        self.state = BgpState::Idle;
        self.adj_rib_in.clear();
        self.adj_rib_out.clear();
    }

    /// 处理连接断开
    pub fn connection_closed(&mut self) {
        self.state = BgpState::Idle;
        self.adj_rib_in.clear();
        self.adj_rib_out.clear();
    }

    /// 创建 OPEN 消息
    pub fn create_open_message(&self) -> BgpOpen {
        // 简化实现：不包含可选参数
        BgpOpen {
            version: BGP_VERSION,
            my_as: self.local_as as u16,
            hold_time: DEFAULT_HOLD_TIME,
            bgp_identifier: self.local_bgp_id,
            optional_parameters: vec![],
        }
    }

    /// 创建 KEEPALIVE 消息
    pub fn create_keepalive_message(&self) -> BgpMessage {
        BgpMessage::Keepalive(crate::protocols::bgp::message::BgpKeepalive)
    }

    /// 添加到出站 RIB
    pub fn advertise_route(&mut self, route: crate::protocols::bgp::rib::BgpRoute) {
        self.adj_rib_out.add_or_update(route);
    }

    /// 从出站 RIB 撤销路由
    pub fn withdraw_route(&mut self, prefix: &IpPrefix) {
        self.adj_rib_out.remove(prefix);
    }

    /// 获取待发送的 UPDATE 消息
    pub fn get_pending_updates(&self) -> Vec<BgpUpdate> {
        // 简化实现：返回所有出站 RIB 中的路由
        // 实际实现应该跟踪哪些路由需要发送
        if self.adj_rib_out.is_empty() {
            return vec![];
        }

        let mut update = BgpUpdate {
            withdrawn_routes: vec![],
            path_attributes: vec![],
            nlri: vec![],
        };

        // 添加必须的路径属性
        update.path_attributes.push(crate::protocols::bgp::message::PathAttribute::Origin { origin: 0 });
        update.path_attributes.push(crate::protocols::bgp::message::PathAttribute::AsPath {
            as_sequence: vec![self.local_as],
            as_set: vec![],
        });

        // 添加 NEXT_HOP
        if let IpAddr::V4(addr) = self.config.address {
            update.path_attributes.push(PathAttribute::NextHop {
                next_hop: CoreIpv4Addr::new(addr.octets()[0], addr.octets()[1], addr.octets()[2], addr.octets()[3]),
            });
        }

        // 添加路由
        for route in self.adj_rib_out.routes() {
            update.nlri.push(route.prefix);
        }

        vec![update]
    }
}

/// BGP 对等体管理器
#[derive(Debug)]
pub struct BgpPeerManager {
    /// 本地 AS 号
    pub local_as: u32,

    /// 本地 BGP 标识符
    pub local_bgp_id: CoreIpv4Addr,

    /// 对等体列表
    pub peers: Vec<BgpPeer>,
}

impl BgpPeerManager {
    /// 创建新的对等体管理器
    pub fn new(local_as: u32, local_bgp_id: CoreIpv4Addr) -> Self {
        Self {
            local_as,
            local_bgp_id,
            peers: Vec::new(),
        }
    }

    /// 添加对等体
    pub fn add_peer(&mut self, config: BgpPeerConfig) -> Result<()> {
        if !config.enabled {
            return Ok(());
        }

        let peer = BgpPeer::new(config, self.local_bgp_id, self.local_as);
        self.peers.push(peer);
        Ok(())
    }

    /// 根据地址查找对等体
    pub fn find_peer(&self, address: &IpAddr) -> Option<&BgpPeer> {
        self.peers.iter().find(|p| &p.config.address == address)
    }

    /// 根据地址查找可变对等体
    pub fn find_peer_mut(&mut self, address: &IpAddr) -> Option<&mut BgpPeer> {
        self.peers.iter_mut().find(|p| &p.config.address == address)
    }

    /// 获取所有已建立连接的对等体
    pub fn established_peers(&self) -> Vec<&BgpPeer> {
        self.peers.iter()
            .filter(|p| p.state == BgpState::Established)
            .collect()
    }

    /// 启动所有对等体
    pub fn start_all(&mut self) -> Result<()> {
        for peer in &mut self.peers {
            peer.bgp_start()?;
        }
        Ok(())
    }

    /// 处理接收到的 BGP 消息
    pub fn handle_message(&mut self, addr: &IpAddr, msg: &BgpMessage) -> Result<()> {
        // 提前保存 local_as 以避免借用冲突
        let local_as = self.local_as;

        let peer = self.find_peer_mut(addr)
            .ok_or_else(|| BgpError::Other(format!("Peer not found: {:?}", addr)))?;

        match msg {
            BgpMessage::Open(open) => {
                peer.handle_open(open)?;
                // 返回 KEEPALIVE 以完成连接
                Ok(())
            }
            BgpMessage::Keepalive(_) => {
                peer.handle_keepalive()
            }
            BgpMessage::Update(update) => {
                peer.handle_update(update, local_as)
            }
            BgpMessage::Notification(notif) => {
                peer.handle_notification(notif);
                Ok(())
            }
            BgpMessage::RouteRefresh(_) => {
                // 简化实现：暂不支持
                Ok(())
            }
        }
    }

    /// 获取所有对等体的入站 RIB
    pub fn get_all_rib_in(&self) -> Vec<(IpAddr, BgpRib)> {
        self.peers.iter()
            .map(|p| (p.config.address, p.adj_rib_in.clone()))
            .collect()
    }

    /// 清空所有对等体状态
    pub fn clear(&mut self) {
        self.peers.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocols::bgp::config::BgpPeerType;

    #[test]
    fn test_peer_state_transitions() {
        let config = BgpPeerConfig {
            name: "test".to_string(),
            address: IpAddr::V4(std::net::Ipv4Addr::new(10, 0, 0, 1)),
            remote_as: 65001,
            peer_type: BgpPeerType::Internal,  // IBGP
            enabled: true,
            passive: false,
        };

        let local_bgp_id = CoreIpv4Addr::new(1, 1, 1, 1);
        let remote_bgp_id = CoreIpv4Addr::new(2, 2, 2, 2); // 不同的 BGP ID

        let mut peer = BgpPeer::new(
            config,
            local_bgp_id,
            65001,  // IBGP：本地 AS 与远程 AS 相同
        );

        // 初始状态应为 Idle
        assert_eq!(peer.state, BgpState::Idle);

        // BGP Start 应转换到 Connect
        peer.bgp_start().unwrap();
        assert_eq!(peer.state, BgpState::Connect);

        // TCP 连接成功应转换到 OpenSent
        let _open = peer.tcp_connection_established().unwrap();
        assert_eq!(peer.state, BgpState::OpenSent);

        // 创建模拟的远程 OPEN（使用不同的 BGP ID）
        let remote_open = BgpOpen {
            version: BGP_VERSION,
            my_as: 65001,
            hold_time: 180,
            bgp_identifier: remote_bgp_id,
            optional_parameters: vec![],
        };

        // 处理 OPEN 消息应转换到 OpenConfirm
        peer.handle_open(&remote_open).unwrap();
        assert_eq!(peer.state, BgpState::OpenConfirm);

        // 处理 KEEPALIVE 应转换到 Established
        peer.handle_keepalive().unwrap();
        assert_eq!(peer.state, BgpState::Established);
    }

    #[test]
    fn test_peer_manager() {
        let mut mgr = BgpPeerManager::new(65000, CoreIpv4Addr::new(1, 1, 1, 1));

        let config = BgpPeerConfig {
            name: "test".to_string(),
            address: IpAddr::V4(std::net::Ipv4Addr::new(10, 0, 0, 1)),
            remote_as: 65001,
            peer_type: BgpPeerType::External,
            enabled: true,
            passive: false,
        };

        mgr.add_peer(config).unwrap();
        assert_eq!(mgr.peers.len(), 1);

        let addr = IpAddr::V4(std::net::Ipv4Addr::new(10, 0, 0, 1));
        let peer = mgr.find_peer(&addr);
        assert!(peer.is_some());
    }
}
