// src/protocols/bgp/peer.rs
//
// BGP 对等体状态机和管理实现（精简版）

use std::net::IpAddr;
use crate::protocols::bgp::config::BgpPeerConfig;
use crate::protocols::bgp::error::Result;
use crate::protocols::DEFAULT_HOLD_TIME;

/// BGP 对等体状态（精简版）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BgpState {
    /// 空闲状态
    Idle,
    /// 连接中
    Connecting,
    /// 已建立
    Established,
}

impl BgpState {
    /// 获取状态名称
    pub fn name(&self) -> &'static str {
        match self {
            BgpState::Idle => "Idle",
            BgpState::Connecting => "Connecting",
            BgpState::Established => "Established",
        }
    }
}

/// BGP 对等体（精简版）
#[derive(Debug, Clone)]
pub struct BgpPeer {
    /// 对等体地址
    pub peer_addr: IpAddr,
    /// 本地 AS 号
    pub local_as: u32,
    /// 远程 AS 号
    pub remote_as: u32,
    /// 当前状态
    pub state: BgpState,
    /// 保持时间
    pub hold_time: u16,
}

impl BgpPeer {
    /// 创建新的 BGP 对等体
    pub fn new(peer_addr: IpAddr, local_as: u32, remote_as: u32) -> Self {
        Self {
            peer_addr,
            local_as,
            remote_as,
            state: BgpState::Idle,
            hold_time: DEFAULT_HOLD_TIME,
        }
    }

    /// 从配置创建对等体（简化版）
    pub fn from_config(config: &BgpPeerConfig, local_as: u32) -> Self {
        Self::new(config.address, local_as, config.remote_as)
    }

    /// 获取对等体地址
    pub fn peer_addr(&self) -> IpAddr {
        self.peer_addr
    }

    /// 获取本地 AS
    pub fn local_as(&self) -> u32 {
        self.local_as
    }

    /// 获取远程 AS
    pub fn remote_as(&self) -> u32 {
        self.remote_as
    }

    /// 获取当前状态
    pub fn state(&self) -> BgpState {
        self.state
    }

    /// 状态转移
    pub fn transition_to(&mut self, new_state: BgpState) {
        println!("BGP: Peer {} state transition: {} -> {}",
            self.peer_addr, self.state.name(), new_state.name());
        self.state = new_state;
    }
}

/// BGP 对等体管理器（精简版）
#[derive(Debug, Clone)]
pub struct BgpPeerManager {
    /// 本地 AS 号
    pub local_as: u32,
    /// 本地路由器 ID
    pub router_id: crate::protocols::Ipv4Addr,
    /// 对等体列表
    peers: Vec<BgpPeer>,
}

impl BgpPeerManager {
    /// 创建新的 BGP 对等体管理器
    pub fn new(local_as: u32, router_id: crate::protocols::Ipv4Addr) -> Self {
        Self {
            local_as,
            router_id,
            peers: Vec::new(),
        }
    }

    /// 添加对等体
    pub fn add_peer(&mut self, peer: BgpPeer) -> Result<()> {
        self.peers.push(peer);
        Ok(())
    }

    /// 获取对等体数量
    pub fn peer_count(&self) -> usize {
        self.peers.len()
    }

    /// 获取所有对等体
    pub fn peers(&self) -> &[BgpPeer] {
        &self.peers
    }
}
