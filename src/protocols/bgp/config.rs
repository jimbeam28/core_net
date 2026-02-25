// src/protocols/bgp/config.rs
//
// BGP 配置结构定义

use std::net::IpAddr;
use crate::common::addr::Ipv4Addr as CoreIpv4Addr;
use crate::protocols::bgp::{DEFAULT_HOLD_TIME, DEFAULT_CONNECT_RETRY_TIME};

/// BGP 配置
#[derive(Debug, Clone)]
pub struct BgpConfig {
    /// 本地 AS 号
    pub local_as: u32,

    /// BGP 标识符（通常是路由器 IP）
    pub bgp_id: CoreIpv4Addr,

    /// Hold Time（秒）
    pub hold_time: u16,

    /// Connect Retry Time（秒）
    pub connect_retry_time: u16,

    /// 是否支持 4 字节 AS 号
    pub support_4byte_as: bool,

    /// 是否支持多协议扩展（MP-BGP）
    pub support_multiprotocol: bool,

    /// 是否支持路由刷新
    pub support_route_refresh: bool,

    /// 对等体列表
    pub peers: Vec<BgpPeerConfig>,
}

impl Default for BgpConfig {
    fn default() -> Self {
        Self {
            local_as: 0,
            bgp_id: CoreIpv4Addr::unspecified(),
            hold_time: DEFAULT_HOLD_TIME,
            connect_retry_time: DEFAULT_CONNECT_RETRY_TIME,
            support_4byte_as: true,
            support_multiprotocol: true,
            support_route_refresh: true,
            peers: Vec::new(),
        }
    }
}

/// BGP 对等体配置
#[derive(Debug, Clone)]
pub struct BgpPeerConfig {
    /// 对等体名称（用于标识）
    pub name: String,

    /// 对等体 IP 地址
    pub address: IpAddr,

    /// 对等体 AS 号
    pub remote_as: u32,

    /// 对等体类型
    pub peer_type: BgpPeerType,

    /// 是否启用该对等体
    pub enabled: bool,

    /// 是否为被动模式（仅接受入站连接）
    pub passive: bool,
}

impl Default for BgpPeerConfig {
    fn default() -> Self {
        Self {
            name: String::new(),
            address: IpAddr::V4(std::net::Ipv4Addr::new(0, 0, 0, 0)),
            remote_as: 0,
            peer_type: BgpPeerType::External,
            enabled: true,
            passive: false,
        }
    }
}

/// BGP 对等体类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BgpPeerType {
    /// 外部 BGP（EBGP）：不同 AS 之间的对等体
    External,
    /// 内部 BGP（IBGP）：同一 AS 内的对等体
    Internal,
}

/// BGP 路由策略（简化实现）
#[derive(Debug, Clone, Default)]
pub struct BgpPolicy {
    /// 策略语句列表
    pub statements: Vec<BgpPolicyStatement>,
}

/// BGP 策略语句
#[derive(Debug, Clone)]
pub struct BgpPolicyStatement {
    /// 匹配条件
    pub match_condition: BgpMatchCondition,
    /// 动作
    pub action: BgpPolicyAction,
}

/// BGP 匹配条件
#[derive(Debug, Clone)]
pub enum BgpMatchCondition {
    /// 匹配所有
    All,
    /// 匹配特定前缀
    Prefix { prefix: super::message::IpPrefix },
    /// 匹配 AS_PATH 长度
    AsPathLength { min: usize, max: usize },
}

/// BGP 策略动作
#[derive(Debug, Clone)]
pub enum BgpPolicyAction {
    /// 允许
    Accept,
    /// 拒绝
    Reject,
    /// 设置本地优先级
    SetLocalPref { local_pref: u32 },
}
