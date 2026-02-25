// src/protocols/bgp/process.rs
//
// BGP 报文处理逻辑
// BGP 运行在 TCP 之上（端口 179），此模块处理接收到的 BGP 报文

use crate::context::SystemContext;
use std::net::IpAddr;

use super::error::{BgpError, Result};
use super::message::{BgpMessage, BgpNotification, BgpOpen, BgpUpdate};
use super::packet::{parse_bgp_message, encapsulate_bgp_message};
use super::{BgpState, BgpRoute, IpPrefix};
use super::config::BgpPeerConfig;
use super::peer::BgpPeer;

/// BGP 处理结果
#[derive(Debug, Clone, PartialEq)]
pub enum BgpProcessResult {
    /// 无需响应
    NoReply,

    /// 需要发送 BGP 响应
    Reply(Vec<u8>),

    /// 需要发送 TCP 数据（包含 BGP 报文）
    SendData(Vec<u8>),

    /// 连接已建立
    ConnectionEstablished,

    /// 连接已关闭
    ConnectionClosed,

    /// 需要关闭连接（发送错误通知）
    CloseConnection(Vec<u8>),
}

/// 处理接收到的 BGP 报文
///
/// # 参数
/// - `data`: BGP 报文数据（来自 TCP）
/// - `source_addr`: 对等体 IP 地址
/// - `_local_addr`: 本地 IP 地址（未使用，保留用于未来扩展）
/// - `context`: 系统上下文
///
/// # 返回
/// - `Ok(BgpProcessResult)`: 处理结果
/// - `Err(BgpError)`: 处理失败
pub fn process_bgp_packet(
    data: &[u8],
    source_addr: IpAddr,
    _local_addr: IpAddr,
    context: &SystemContext,
) -> Result<BgpProcessResult> {
    // 1. 解析 BGP 报文
    let msg = parse_bgp_message(data)?;

    // 2. 获取本地 AS 号（避免借用冲突）
    let local_as = {
        let bgp_mgr = context.bgp_manager.lock()
            .map_err(|e| BgpError::Other(format!("锁定 BGP 管理器失败: {}", e)))?;
        bgp_mgr.local_as
    };

    // 3. 获取 BGP 管理器并查找对等体
    let mut bgp_mgr = context.bgp_manager.lock()
        .map_err(|e| BgpError::Other(format!("锁定 BGP 管理器失败: {}", e)))?;

    let peer_addr = source_addr;
    let peer = bgp_mgr.find_peer_mut(&peer_addr)
        .ok_or_else(|| BgpError::Other(format!("未找到对等体配置: {:?}", peer_addr)))?;

    // 4. 根据消息类型处理
    match msg {
        BgpMessage::Open(open) => {
            handle_open_message(peer, &open, local_as)
        }
        BgpMessage::Keepalive(_) => {
            handle_keepalive_message(peer)
        }
        BgpMessage::Update(update) => {
            handle_update_message(peer, &update, local_as)
        }
        BgpMessage::Notification(notification) => {
            handle_notification_message(peer, &notification)
        }
        BgpMessage::RouteRefresh(_) => {
            // 简化实现：暂不支持路由刷新
            Ok(BgpProcessResult::NoReply)
        }
    }
}

/// 处理 OPEN 消息
fn handle_open_message(
    peer: &mut BgpPeer,
    open: &BgpOpen,
    _local_as: u32,
) -> Result<BgpProcessResult> {
    // 处理 OPEN 消息
    peer.handle_open(open)?;

    // 如果状态转换到 OpenConfirm，需要回复 KEEPALIVE
    if peer.state == BgpState::OpenConfirm {
        let keepalive = peer.create_keepalive_message();
        let data = encapsulate_bgp_message(&keepalive);
        Ok(BgpProcessResult::SendData(data))
    } else {
        Ok(BgpProcessResult::NoReply)
    }
}

/// 处理 KEEPALIVE 消息
fn handle_keepalive_message(
    peer: &mut BgpPeer,
) -> Result<BgpProcessResult> {
    // 处理 KEEPALIVE 消息
    peer.handle_keepalive()?;

    // 如果状态转换到 Established，需要发送初始 UPDATE
    if peer.state == BgpState::Established {
        // 获取待发送的 UPDATE 消息
        let updates = peer.get_pending_updates();
        if !updates.is_empty() {
            // 发送第一个 UPDATE
            let data = encapsulate_bgp_message(&BgpMessage::Update(updates[0].clone()));
            Ok(BgpProcessResult::SendData(data))
        } else {
            Ok(BgpProcessResult::ConnectionEstablished)
        }
    } else {
        Ok(BgpProcessResult::NoReply)
    }
}

/// 处理 UPDATE 消息
fn handle_update_message(
    peer: &mut BgpPeer,
    update: &BgpUpdate,
    local_as: u32,
) -> Result<BgpProcessResult> {
    // 验证必须属性
    validate_mandatory_attributes(update)?;

    // 处理 UPDATE 消息
    peer.handle_update(update, local_as)?;

    // 定期发送 KEEPALIVE
    // TODO: 需要定时器机制

    Ok(BgpProcessResult::NoReply)
}

/// 处理 NOTIFICATION 消息
fn handle_notification_message(
    peer: &mut BgpPeer,
    notification: &BgpNotification,
) -> Result<BgpProcessResult> {
    peer.handle_notification(notification);
    Ok(BgpProcessResult::ConnectionClosed)
}

/// 验证必须的路径属性
fn validate_mandatory_attributes(update: &BgpUpdate) -> Result<()> {
    // 检查 ORIGIN
    let has_origin = update.path_attributes.iter()
        .any(|a| matches!(a, crate::protocols::bgp::message::PathAttribute::Origin { .. }));

    if !has_origin {
        return Err(BgpError::MissingRequiredAttribute("ORIGIN".to_string()));
    }

    // 检查 AS_PATH
    let has_as_path = update.path_attributes.iter()
        .any(|a| matches!(a, crate::protocols::bgp::message::PathAttribute::AsPath { .. }));

    if !has_as_path {
        return Err(BgpError::MissingRequiredAttribute("AS_PATH".to_string()));
    }

    // 检查 NEXT_HOP（如果 NLRI 不为空）
    if !update.nlri.is_empty() {
        let has_next_hop = update.path_attributes.iter()
            .any(|a| matches!(a, crate::protocols::bgp::message::PathAttribute::NextHop { .. }));

        if !has_next_hop {
            return Err(BgpError::MissingRequiredAttribute("NEXT_HOP".to_string()));
        }
    }

    Ok(())
}

/// BGP 定时器事件
#[derive(Debug, Clone, PartialEq)]
pub enum BgpTimerEvent {
    /// Keepalive 定时器触发
    KeepaliveTimer,
    /// Hold 定时器触发
    HoldTimer,
    /// Connect Retry 定时器触发
    ConnectRetryTimer,
}

/// 处理 BGP 定时器事件
///
/// # 参数
/// - `peer_addr`: 对等体地址
/// - `event`: 定时器事件
/// - `context`: 系统上下文
///
/// # 返回
/// - `Ok(BgpProcessResult)`: 处理结果
/// - `Err(BgpError)`: 处理失败
pub fn process_bgp_timer(
    peer_addr: &IpAddr,
    event: BgpTimerEvent,
    context: &SystemContext,
) -> Result<BgpProcessResult> {
    let mut bgp_mgr = context.bgp_manager.lock()
        .map_err(|e| BgpError::Other(format!("锁定 BGP 管理器失败: {}", e)))?;

    let peer = bgp_mgr.find_peer_mut(peer_addr)
        .ok_or_else(|| BgpError::Other(format!("对等体不存在: {:?}", peer_addr)))?;

    match event {
        BgpTimerEvent::KeepaliveTimer => {
            // 发送 KEEPALIVE
            if peer.state == BgpState::Established {
                let keepalive = peer.create_keepalive_message();
                let data = encapsulate_bgp_message(&keepalive);
                Ok(BgpProcessResult::SendData(data))
            } else {
                Ok(BgpProcessResult::NoReply)
            }
        }
        BgpTimerEvent::HoldTimer => {
            // Hold 定时器超时，发送 NOTIFICATION 并关闭连接
            peer.hold_timer_expired();
            let notification = BgpNotification {
                error_code: 4, // Hold Timer Expired
                error_subcode: 0,
                data: vec![],
            };
            let data = encapsulate_bgp_message(&BgpMessage::Notification(notification));
            Ok(BgpProcessResult::CloseConnection(data))
        }
        BgpTimerEvent::ConnectRetryTimer => {
            // Connect Retry 定时器超时，尝试重新连接
            // 简化实现：回到 Idle 状态
            if peer.state == BgpState::Connect || peer.state == BgpState::Active {
                peer.state = BgpState::Idle;
                // 重新触发连接
                peer.bgp_start()?;
            }
            Ok(BgpProcessResult::NoReply)
        }
    }
}

/// 创建 BGP 连接（主动发起连接）
///
/// # 参数
/// - `peer_config`: 对等体配置
/// - `context`: 系统上下文
///
/// # 返回
/// - `Ok(BgpProcessResult)`: 处理结果，包含需要发送的 OPEN 消息
/// - `Err(BgpError)`: 处理失败
pub fn create_bgp_connection(
    peer_config: BgpPeerConfig,
    context: &SystemContext,
) -> Result<BgpProcessResult> {
    let mut bgp_mgr = context.bgp_manager.lock()
        .map_err(|e| BgpError::Other(format!("锁定 BGP 管理器失败: {}", e)))?;

    // 添加对等体（如果不存在）
    if bgp_mgr.find_peer(&peer_config.address).is_none() {
        bgp_mgr.add_peer(peer_config.clone())?;
    }

    let peer = bgp_mgr.find_peer_mut(&peer_config.address)
        .ok_or_else(|| BgpError::Other("对等体添加失败".to_string()))?;

    // 启动对等体
    peer.bgp_start()?;

    // 模拟 TCP 连接建立
    let open = peer.tcp_connection_established()?;

    // 封装 OPEN 消息
    let data = encapsulate_bgp_message(&BgpMessage::Open(open));

    Ok(BgpProcessResult::SendData(data))
}

/// 路由发布：将路由添加到 BGP 并发送 UPDATE
///
/// # 参数
/// - `peer_addr`: 对等体地址
/// - `route`: 要发布的路由
/// - `context`: 系统上下文
///
/// # 返回
/// - `Ok(Option<Vec<u8>>)`: 需要发送的 UPDATE 消息（如果有）
/// - `Err(BgpError)`: 处理失败
pub fn advertise_route(
    peer_addr: &IpAddr,
    route: BgpRoute,
    context: &SystemContext,
) -> Result<Option<Vec<u8>>> {
    let mut bgp_mgr = context.bgp_manager.lock()
        .map_err(|e| BgpError::Other(format!("锁定 BGP 管理器失败: {}", e)))?;

    let peer = bgp_mgr.find_peer_mut(peer_addr)
        .ok_or_else(|| BgpError::Other(format!("对等体不存在: {:?}", peer_addr)))?;

    // 只有在 Established 状态才能发送 UPDATE
    if peer.state != BgpState::Established {
        return Ok(None);
    }

    // 添加到出站 RIB
    peer.advertise_route(route.clone());

    // 构造 UPDATE 消息
    let update = super::message::BgpUpdate {
        withdrawn_routes: vec![],
        path_attributes: vec![
            super::message::PathAttribute::Origin { origin: route.origin },
            super::message::PathAttribute::AsPath {
                as_sequence: route.as_path.clone(),
                as_set: vec![],
            },
        ],
        nlri: vec![route.prefix],
    };

    let data = encapsulate_bgp_message(&BgpMessage::Update(update));
    Ok(Some(data))
}

/// 路由撤销：从 BGP 撤销路由
///
/// # 参数
/// - `peer_addr`: 对等体地址
/// - `prefix`: 要撤销的前缀
/// - `context`: 系统上下文
///
/// # 返回
/// - `Ok(Option<Vec<u8>>)`: 需要发送的 UPDATE 消息（如果有）
/// - `Err(BgpError)`: 处理失败
pub fn withdraw_route(
    peer_addr: &IpAddr,
    prefix: &IpPrefix,
    context: &SystemContext,
) -> Result<Option<Vec<u8>>> {
    let mut bgp_mgr = context.bgp_manager.lock()
        .map_err(|e| BgpError::Other(format!("锁定 BGP 管理器失败: {}", e)))?;

    let peer = bgp_mgr.find_peer_mut(peer_addr)
        .ok_or_else(|| BgpError::Other(format!("对等体不存在: {:?}", peer_addr)))?;

    // 只有在 Established 状态才能发送 UPDATE
    if peer.state != BgpState::Established {
        return Ok(None);
    }

    // 从出站 RIB 移除
    peer.withdraw_route(prefix);

    // 构造 UPDATE 消息
    let update = super::message::BgpUpdate {
        withdrawn_routes: vec![prefix.clone()],
        path_attributes: vec![],
        nlri: vec![],
    };

    let data = encapsulate_bgp_message(&BgpMessage::Update(update));
    Ok(Some(data))
}

// ========== 单元测试 ==========

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocols::bgp::{BgpPeerType, BgpState};
    use crate::common::addr::Ipv4Addr as CoreIpv4Addr;
    use std::net::Ipv4Addr;

    #[test]
    fn test_validate_mandatory_attributes_valid() {
        let update = BgpUpdate {
            withdrawn_routes: vec![],
            path_attributes: vec![
                super::super::message::PathAttribute::Origin { origin: 0 },
                super::super::message::PathAttribute::AsPath {
                    as_sequence: vec![65001],
                    as_set: vec![],
                },
                super::super::message::PathAttribute::NextHop {
                    next_hop: CoreIpv4Addr::new(10, 0, 0, 1),
                },
            ],
            nlri: vec![],
        };

        assert!(validate_mandatory_attributes(&update).is_ok());
    }

    #[test]
    fn test_validate_mandatory_attributes_missing_origin() {
        let update = BgpUpdate {
            withdrawn_routes: vec![],
            path_attributes: vec![
                super::super::message::PathAttribute::AsPath {
                    as_sequence: vec![65001],
                    as_set: vec![],
                },
                super::super::message::PathAttribute::NextHop {
                    next_hop: CoreIpv4Addr::new(10, 0, 0, 1),
                },
            ],
            nlri: vec![],
        };

        assert!(validate_mandatory_attributes(&update).is_err());
    }

    #[test]
    fn test_validate_mandatory_attributes_missing_next_hop_with_nlri() {
        let update = BgpUpdate {
            withdrawn_routes: vec![],
            path_attributes: vec![
                super::super::message::PathAttribute::Origin { origin: 0 },
                super::super::message::PathAttribute::AsPath {
                    as_sequence: vec![65001],
                    as_set: vec![],
                },
            ],
            nlri: vec![IpPrefix::new(
                IpAddr::V4(Ipv4Addr::new(192, 168, 1, 0)),
                24
            )],
        };

        assert!(validate_mandatory_attributes(&update).is_err());
    }

    #[test]
    fn test_validate_mandatory_attributes_no_nlri_no_next_hop_required() {
        let update = BgpUpdate {
            withdrawn_routes: vec![],
            path_attributes: vec![
                super::super::message::PathAttribute::Origin { origin: 0 },
                super::super::message::PathAttribute::AsPath {
                    as_sequence: vec![65001],
                    as_set: vec![],
                },
            ],
            nlri: vec![],
        };

        // 没有 NLRI 时，NEXT_HOP 不是必须的
        assert!(validate_mandatory_attributes(&update).is_ok());
    }
}
