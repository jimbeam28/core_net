// BGP 协议集成测试
//
// 测试 BGP 协议的报文解析、封装、状态机、对等体管理

use core_net::protocols::bgp::{
    BgpHeader, BgpOpen, BgpUpdate, BgpNotification, BgpKeepalive,
    BgpMessage, BgpState, BgpPeerType, BgpPeer, BgpPeerManager, BgpRib,
    parse_bgp_message, encapsulate_bgp_message, IpPrefix, BgpRoute, PathAttribute,
    BGP_VERSION, BGP_MSG_OPEN, BGP_MSG_UPDATE, BGP_MSG_NOTIFICATION,
    BGP_MSG_KEEPALIVE, DEFAULT_HOLD_TIME,
};
use core_net::protocols::Ipv4Addr as CoreIpv4Addr;
use serial_test::serial;
use std::net::IpAddr;

// 测试环境配置

// ========== 基本功能测试组 ==========

#[test]
#[serial]
fn test_bgp_header_new() {
    let header = BgpHeader::new(29, BGP_MSG_OPEN);

    assert_eq!(header.msg_type, BGP_MSG_OPEN);
    assert_eq!(header.length, 29);
    assert!(header.validate_length());
}

#[test]
#[serial]
fn test_bgp_header_default_marker() {
    let marker = BgpHeader::default_marker();

    // Marker 应该全是 0xFF
    assert_eq!(marker, [0xFFu8; 16]);
}

#[test]
#[serial]
fn test_bgp_open_message() {
    let open = BgpOpen {
        version: BGP_VERSION,
        my_as: 65001,
        hold_time: 180,
        bgp_identifier: CoreIpv4Addr::new(10, 0, 0, 1),
        optional_parameters: vec![],
    };

    assert_eq!(open.version, 4);
    assert_eq!(open.my_as, 65001);
    assert_eq!(open.hold_time, 180);
    assert_eq!(open.bgp_identifier, CoreIpv4Addr::new(10, 0, 0, 1));
}

#[test]
#[serial]
fn test_bgp_open_encapsulate_and_parse() {
    let open = BgpOpen {
        version: BGP_VERSION,
        my_as: 65001,
        hold_time: 180,
        bgp_identifier: CoreIpv4Addr::new(10, 0, 0, 1),
        optional_parameters: vec![],
    };

    // 封装
    let data = encapsulate_bgp_message(&BgpMessage::Open(open.clone()));

    // 验证基本结构
    assert!(data.len() >= 29); // 最小 OPEN 长度
    assert_eq!(data[18], BGP_MSG_OPEN); // Type

    // 解析
    let parsed = parse_bgp_message(&data).unwrap();
    match parsed {
        BgpMessage::Open(parsed_open) => {
            assert_eq!(parsed_open.version, open.version);
            assert_eq!(parsed_open.my_as, open.my_as);
            assert_eq!(parsed_open.hold_time, open.hold_time);
            assert_eq!(parsed_open.bgp_identifier, open.bgp_identifier);
        }
        _ => panic!("Expected OPEN message"),
    }
}

#[test]
#[serial]
fn test_bgp_keepalive_encapsulate_and_parse() {
    // 封装
    let data = encapsulate_bgp_message(&BgpMessage::Keepalive(BgpKeepalive));

    // KEEPALIVE 只有头部，长度应该是 19
    assert_eq!(data.len(), 19);
    assert_eq!(data[18], BGP_MSG_KEEPALIVE);

    // 解析
    let parsed = parse_bgp_message(&data).unwrap();
    match parsed {
        BgpMessage::Keepalive(_) => {}
        _ => panic!("Expected KEEPALIVE message"),
    }
}

#[test]
#[serial]
fn test_bgp_notification_encapsulate_and_parse() {
    let notification = BgpNotification {
        error_code: 1,  // Message Header Error
        error_subcode: 2, // Bad Message Length
        data: vec![],
    };

    // 封装
    let data = encapsulate_bgp_message(&BgpMessage::Notification(notification.clone()));

    // 验证基本结构
    assert!(data.len() >= 21); // 最小 NOTIFICATION 长度
    assert_eq!(data[18], BGP_MSG_NOTIFICATION);

    // 解析
    let parsed = parse_bgp_message(&data).unwrap();
    match parsed {
        BgpMessage::Notification(parsed_notif) => {
            assert_eq!(parsed_notif.error_code, notification.error_code);
            assert_eq!(parsed_notif.error_subcode, notification.error_subcode);
        }
        _ => panic!("Expected NOTIFICATION message"),
    }
}

#[test]
#[serial]
fn test_bgp_update_basic() {
    let update = BgpUpdate {
        withdrawn_routes: vec![],
        path_attributes: vec![
            PathAttribute::Origin { origin: 0 },
            PathAttribute::AsPath {
                as_sequence: vec![65001, 65002],
                as_set: vec![],
            },
            PathAttribute::NextHop {
                next_hop: CoreIpv4Addr::new(10, 0, 0, 1),
            },
        ],
        nlri: vec![
            IpPrefix::new(IpAddr::V4(std::net::Ipv4Addr::new(192, 168, 1, 0)), 24),
        ],
    };

    // 封装
    let data = encapsulate_bgp_message(&BgpMessage::Update(update.clone()));

    // 验证基本结构
    assert!(data.len() >= 23); // 最小 UPDATE 长度
    assert_eq!(data[18], BGP_MSG_UPDATE);

    // 解析
    let parsed = parse_bgp_message(&data).unwrap();
    match parsed {
        BgpMessage::Update(parsed_update) => {
            assert_eq!(parsed_update.withdrawn_routes.len(), 0);
            assert_eq!(parsed_update.nlri.len(), 1);
            assert_eq!(parsed_update.nlri[0].prefix_len, 24);
        }
        _ => panic!("Expected UPDATE message"),
    }
}

#[test]
#[serial]
fn test_bgp_update_with_withdrawn_routes() {
    let update = BgpUpdate {
        withdrawn_routes: vec![
            IpPrefix::new(IpAddr::V4(std::net::Ipv4Addr::new(10, 1, 0, 0)), 24),
        ],
        path_attributes: vec![],
        nlri: vec![],
    };

    // 封装
    let data = encapsulate_bgp_message(&BgpMessage::Update(update.clone()));

    // 解析
    let parsed = parse_bgp_message(&data).unwrap();
    match parsed {
        BgpMessage::Update(parsed_update) => {
            assert_eq!(parsed_update.withdrawn_routes.len(), 1);
            assert_eq!(parsed_update.nlri.len(), 0);
        }
        _ => panic!("Expected UPDATE message"),
    }
}

#[test]
#[serial]
fn test_bgp_iprefix() {
    let prefix = IpPrefix::new(
        IpAddr::V4(std::net::Ipv4Addr::new(192, 168, 1, 0)),
        24
    );

    assert_eq!(prefix.prefix_len, 24);
    match prefix.prefix {
        IpAddr::V4(addr) => {
            assert_eq!(addr, std::net::Ipv4Addr::new(192, 168, 1, 0));
        }
        _ => panic!("Expected IPv4 address"),
    }
}

#[test]
#[serial]
fn test_bgp_route() {
    let prefix = IpPrefix::new(
        IpAddr::V4(std::net::Ipv4Addr::new(10, 0, 0, 0)),
        24
    );
    let next_hop = IpAddr::V4(std::net::Ipv4Addr::new(192, 168, 1, 1));
    let peer = IpAddr::V4(std::net::Ipv4Addr::new(10, 0, 0, 2));

    let route = BgpRoute::new(prefix.clone(), next_hop, peer);

    assert_eq!(route.prefix, prefix);
    assert_eq!(route.next_hop, next_hop);
    assert!(route.valid);
    assert_eq!(route.age, 0);
}

#[test]
#[serial]
fn test_bgp_route_preference() {
    let prefix = IpPrefix::new(
        IpAddr::V4(std::net::Ipv4Addr::new(10, 0, 0, 0)),
        24
    );
    let next_hop = IpAddr::V4(std::net::Ipv4Addr::new(192, 168, 1, 1));
    let peer = IpAddr::V4(std::net::Ipv4Addr::new(10, 0, 0, 2));

    let mut route1 = BgpRoute::new(prefix.clone(), next_hop, peer);
    route1.local_pref = Some(200);
    route1.as_path = vec![100, 200];
    route1.med = 100;

    let mut route2 = BgpRoute::new(prefix, next_hop, peer);
    route2.local_pref = Some(100);
    route2.as_path = vec![100];
    route2.med = 0;

    // route1 有更高的 Local Pref，应该有更高的优先级
    assert!(route1.preference() > route2.preference());
}

// ========== 状态机测试组 ==========

#[test]
#[serial]
fn test_bgp_peer_state_transitions() {
    let config = core_net::protocols::bgp::BgpPeerConfig {
        name: "test_peer".to_string(),
        address: IpAddr::V4(std::net::Ipv4Addr::new(10, 0, 0, 1)),
        remote_as: 65001,
        peer_type: BgpPeerType::Internal, // IBGP
        enabled: true,
        passive: false,
    };

    let local_bgp_id = CoreIpv4Addr::new(1, 1, 1, 1);
    let remote_bgp_id = CoreIpv4Addr::new(2, 2, 2, 2); // 不同的 BGP ID

    let mut peer = BgpPeer::new(
        config,
        local_bgp_id,
        65001, // IBGP：本地 AS 与远程 AS 相同
    );

    // 初始状态应为 Idle
    assert_eq!(peer.state, BgpState::Idle);

    // BGP Start 应转换到 Connect
    peer.bgp_start().unwrap();
    assert_eq!(peer.state, BgpState::Connect);

    // TCP 连接成功应转换到 OpenSent
    let _open = peer.tcp_connection_established().unwrap();

    // 创建模拟的远程 OPEN（使用不同的 BGP ID）
    let remote_open = BgpOpen {
        version: BGP_VERSION,
        my_as: 65001,
        hold_time: 180,
        bgp_identifier: remote_bgp_id,
        optional_parameters: vec![],
    };

    assert_eq!(peer.state, BgpState::OpenSent);

    // 处理远程 OPEN 消息应转换到 OpenConfirm
    peer.handle_open(&remote_open).unwrap();
    assert_eq!(peer.state, BgpState::OpenConfirm);

    // 处理 KEEPALIVE 应转换到 Established
    peer.handle_keepalive().unwrap();
    assert_eq!(peer.state, BgpState::Established);
}

#[test]
#[serial]
fn test_bgp_peer_create_open_message() {
    let config = core_net::protocols::bgp::BgpPeerConfig {
        name: "test_peer".to_string(),
        address: IpAddr::V4(std::net::Ipv4Addr::new(10, 0, 0, 1)),
        remote_as: 65001,
        peer_type: BgpPeerType::External,
        enabled: true,
        passive: false,
    };

    let peer = BgpPeer::new(
        config,
        CoreIpv4Addr::new(1, 1, 1, 1),
        65000,
    );

    let open = peer.create_open_message();

    assert_eq!(open.version, BGP_VERSION);
    assert_eq!(open.my_as, 65000);
    assert_eq!(open.hold_time, DEFAULT_HOLD_TIME);
    assert_eq!(open.bgp_identifier, CoreIpv4Addr::new(1, 1, 1, 1));
}

#[test]
#[serial]
fn test_bgp_state_is_active() {
    assert!(!BgpState::Idle.is_active());
    assert!(!BgpState::Connect.is_active());
    assert!(!BgpState::Active.is_active());
    assert!(BgpState::OpenSent.is_active());
    assert!(BgpState::OpenConfirm.is_active());
    assert!(BgpState::Established.is_active());
}

#[test]
#[serial]
fn test_bgp_state_can_send_update() {
    assert!(!BgpState::Idle.can_send_update());
    assert!(!BgpState::Connect.can_send_update());
    assert!(!BgpState::Active.can_send_update());
    assert!(!BgpState::OpenSent.can_send_update());
    assert!(!BgpState::OpenConfirm.can_send_update());
    assert!(BgpState::Established.can_send_update());
}

#[test]
#[serial]
fn test_bgp_peer_handle_notification() {
    let config = core_net::protocols::bgp::BgpPeerConfig {
        name: "test_peer".to_string(),
        address: IpAddr::V4(std::net::Ipv4Addr::new(10, 0, 0, 1)),
        remote_as: 65001,
        peer_type: BgpPeerType::External,
        enabled: true,
        passive: false,
    };

    let mut peer = BgpPeer::new(
        config,
        CoreIpv4Addr::new(1, 1, 1, 1),
        65000,
    );

    // 设置为 Established 状态
    peer.state = BgpState::Established;

    let notification = BgpNotification {
        error_code: 6,  // Cease
        error_subcode: 0,
        data: vec![],
    };

    peer.handle_notification(&notification);

    // 应该回到 Idle 状态
    assert_eq!(peer.state, BgpState::Idle);
}

#[test]
#[serial]
fn test_bgp_peer_hold_timer_expired() {
    let config = core_net::protocols::bgp::BgpPeerConfig {
        name: "test_peer".to_string(),
        address: IpAddr::V4(std::net::Ipv4Addr::new(10, 0, 0, 1)),
        remote_as: 65001,
        peer_type: BgpPeerType::External,
        enabled: true,
        passive: false,
    };

    let mut peer = BgpPeer::new(
        config,
        CoreIpv4Addr::new(1, 1, 1, 1),
        65000,
    );

    // 设置为 Established 状态
    peer.state = BgpState::Established;

    // 添加一些路由
    let prefix = IpPrefix::new(
        IpAddr::V4(std::net::Ipv4Addr::new(10, 0, 0, 0)),
        24
    );
    let route = BgpRoute::new(
        prefix.clone(),
        IpAddr::V4(std::net::Ipv4Addr::new(192, 168, 1, 1)),
        IpAddr::V4(std::net::Ipv4Addr::new(10, 0, 0, 2)),
    );
    peer.adj_rib_in.add_or_update(route);

    // Hold Timer 超时
    peer.hold_timer_expired();

    // 应该回到 Idle 状态，RIB 被清空
    assert_eq!(peer.state, BgpState::Idle);
    assert!(peer.adj_rib_in.is_empty());
}

// ========== 对等体管理器测试组 ==========

#[test]
#[serial]
fn test_bgp_peer_manager_new() {
    let mgr = BgpPeerManager::new(65000, CoreIpv4Addr::new(1, 1, 1, 1));

    assert_eq!(mgr.local_as, 65000);
    assert_eq!(mgr.local_bgp_id, CoreIpv4Addr::new(1, 1, 1, 1));
    assert_eq!(mgr.peers.len(), 0);
}

#[test]
#[serial]
fn test_bgp_peer_manager_add_peer() {
    let mut mgr = BgpPeerManager::new(65000, CoreIpv4Addr::new(1, 1, 1, 1));

    let config = core_net::protocols::bgp::BgpPeerConfig {
        name: "test_peer".to_string(),
        address: IpAddr::V4(std::net::Ipv4Addr::new(10, 0, 0, 1)),
        remote_as: 65001,
        peer_type: BgpPeerType::External,
        enabled: true,
        passive: false,
    };

    mgr.add_peer(config).unwrap();

    assert_eq!(mgr.peers.len(), 1);
}

#[test]
#[serial]
fn test_bgp_peer_manager_find_peer() {
    let mut mgr = BgpPeerManager::new(65000, CoreIpv4Addr::new(1, 1, 1, 1));

    let addr = IpAddr::V4(std::net::Ipv4Addr::new(10, 0, 0, 1));
    let config = core_net::protocols::bgp::BgpPeerConfig {
        name: "test_peer".to_string(),
        address: addr,
        remote_as: 65001,
        peer_type: BgpPeerType::External,
        enabled: true,
        passive: false,
    };

    mgr.add_peer(config).unwrap();

    let peer = mgr.find_peer(&addr);
    assert!(peer.is_some());
    assert_eq!(peer.unwrap().remote_as, 65001);
}

#[test]
#[serial]
fn test_bgp_peer_manager_start_all() {
    let mut mgr = BgpPeerManager::new(65000, CoreIpv4Addr::new(1, 1, 1, 1));

    let config = core_net::protocols::bgp::BgpPeerConfig {
        name: "test_peer".to_string(),
        address: IpAddr::V4(std::net::Ipv4Addr::new(10, 0, 0, 1)),
        remote_as: 65001,
        peer_type: BgpPeerType::External,
        enabled: true,
        passive: false,
    };

    mgr.add_peer(config).unwrap();

    mgr.start_all().unwrap();

    let addr = IpAddr::V4(std::net::Ipv4Addr::new(10, 0, 0, 1));
    let peer = mgr.find_peer(&addr).unwrap();

    // 应该在 Connect 状态
    assert_eq!(peer.state, BgpState::Connect);
}

// ========== RIB 测试组 ==========

#[test]
#[serial]
fn test_bgp_rib_new() {
    let rib = BgpRib::new();

    assert!(rib.is_empty());
    assert_eq!(rib.len(), 0);
}

#[test]
#[serial]
fn test_bgp_rib_add_and_find() {
    let mut rib = BgpRib::new();

    let prefix = IpPrefix::new(
        IpAddr::V4(std::net::Ipv4Addr::new(192, 168, 1, 0)),
        24
    );
    let route = BgpRoute::new(
        prefix.clone(),
        IpAddr::V4(std::net::Ipv4Addr::new(10, 0, 0, 1)),
        IpAddr::V4(std::net::Ipv4Addr::new(10, 0, 0, 2)),
    );

    rib.add_or_update(route);

    assert_eq!(rib.len(), 1);

    let found = rib.find(&prefix);
    assert!(found.is_some());
    assert_eq!(
        found.unwrap().next_hop,
        IpAddr::V4(std::net::Ipv4Addr::new(10, 0, 0, 1))
    );
}

#[test]
#[serial]
fn test_bgp_rib_remove() {
    let mut rib = BgpRib::new();

    let prefix = IpPrefix::new(
        IpAddr::V4(std::net::Ipv4Addr::new(192, 168, 1, 0)),
        24
    );
    let route = BgpRoute::new(
        prefix.clone(),
        IpAddr::V4(std::net::Ipv4Addr::new(10, 0, 0, 1)),
        IpAddr::V4(std::net::Ipv4Addr::new(10, 0, 0, 2)),
    );

    rib.add_or_update(route);
    assert_eq!(rib.len(), 1);

    rib.remove(&prefix);
    assert_eq!(rib.len(), 0);
    assert!(rib.find(&prefix).is_none());
}

#[test]
#[serial]
fn test_bgp_rib_longest_prefix_match() {
    let mut rib = BgpRib::new();

    // 添加 10.0.0.0/8
    rib.add_or_update(BgpRoute::new(
        IpPrefix::new(IpAddr::V4(std::net::Ipv4Addr::new(10, 0, 0, 0)), 8),
        IpAddr::V4(std::net::Ipv4Addr::new(10, 0, 0, 1)),
        IpAddr::V4(std::net::Ipv4Addr::new(10, 0, 0, 2)),
    ));

    // 添加 10.0.1.0/24
    rib.add_or_update(BgpRoute::new(
        IpPrefix::new(IpAddr::V4(std::net::Ipv4Addr::new(10, 0, 1, 0)), 24),
        IpAddr::V4(std::net::Ipv4Addr::new(10, 0, 1, 1)),
        IpAddr::V4(std::net::Ipv4Addr::new(10, 0, 1, 2)),
    ));

    // 查询 10.0.1.100 应该匹配 /24
    let result = rib.lookup(&IpAddr::V4(std::net::Ipv4Addr::new(10, 0, 1, 100)));
    assert!(result.is_some());
    assert_eq!(
        result.unwrap().next_hop,
        IpAddr::V4(std::net::Ipv4Addr::new(10, 0, 1, 1))
    );
}

#[test]
#[serial]
fn test_bgp_rib_clear() {
    let mut rib = BgpRib::new();

    rib.add_or_update(BgpRoute::new(
        IpPrefix::new(IpAddr::V4(std::net::Ipv4Addr::new(10, 0, 0, 0)), 24),
        IpAddr::V4(std::net::Ipv4Addr::new(10, 0, 0, 1)),
        IpAddr::V4(std::net::Ipv4Addr::new(10, 0, 0, 2)),
    ));

    assert!(!rib.is_empty());

    rib.clear();

    assert!(rib.is_empty());
}

// ========== 错误处理测试组 ==========

#[test]
#[serial]
fn test_bgp_parse_invalid_length() {
    // 数据太短
    let data = vec![0xFF; 10];

    let result = parse_bgp_message(&data);
    assert!(result.is_err());
}

#[test]
#[serial]
fn test_bgp_parse_invalid_marker() {
    // 创建无效的 Marker（不全为 0xFF）
    let mut data = vec![0x00; 19];
    data[16] = 18;  // Length
    data[17] = 0;   // Length
    data[18] = 4;   // Type = OPEN

    let result = parse_bgp_message(&data);
    assert!(result.is_err());
}

#[test]
#[serial]
fn test_bgp_parse_unsupported_version() {
    // Marker (16 bytes)
    let mut data = vec![0xFFu8; 16];
    // Length
    data.extend_from_slice(&29u16.to_be_bytes());
    // Type = OPEN
    data.push(1);
    // Version = 3 (不支持)
    data.push(3);
    // My AS
    data.extend_from_slice(&65001u16.to_be_bytes());
    // Hold Time
    data.extend_from_slice(&180u16.to_be_bytes());
    // BGP Identifier
    data.extend_from_slice(&[10, 0, 0, 1]);
    // Opt Param Len
    data.push(0);

    let result = parse_bgp_message(&data);
    assert!(result.is_err());
}

// ========== 边界情况测试组 ==========

#[test]
#[serial]
fn test_bgp_parse_empty_update() {
    // 创建一个空的 UPDATE（没有 Withdrawn Routes 和 NLRI）
    let mut data = vec![0xFFu8; 16];  // Marker
    data.extend_from_slice(&23u16.to_be_bytes());  // Length = 23 (最小)
    data.push(2);  // Type = UPDATE

    // Withdrawn Routes Length = 0
    data.extend_from_slice(&0u16.to_be_bytes());
    // Total Path Attribute Length = 0
    data.extend_from_slice(&0u16.to_be_bytes());

    let result = parse_bgp_message(&data);
    assert!(result.is_ok());

    match result.unwrap() {
        BgpMessage::Update(update) => {
            assert_eq!(update.withdrawn_routes.len(), 0);
            assert_eq!(update.path_attributes.len(), 0);
            assert_eq!(update.nlri.len(), 0);
        }
        _ => panic!("Expected UPDATE message"),
    }
}

#[test]
#[serial]
fn test_bgp_max_message_length() {
    // 创建一个 4096 字节的消息（最大长度）
    let mut data = vec![0xFFu8; 16];  // Marker
    data.extend_from_slice(&4096u16.to_be_bytes());  // Length
    data.push(4);  // Type = KEEPALIVE

    // 填充剩余部分
    while data.len() < 4096 {
        data.push(0);
    }

    let result = parse_bgp_message(&data);
    assert!(result.is_ok());
}

#[test]
#[serial]
fn test_bgp_exceeds_max_message_length() {
    // 创建一个超过 4096 字节的消息
    let mut data = vec![0xFFu8; 16];  // Marker
    data.extend_from_slice(&4097u16.to_be_bytes());  // Length = 4097 (超过最大值)
    data.push(4);  // Type = KEEPALIVE

    let result = parse_bgp_message(&data);
    assert!(result.is_err());
}
