// 路由模块集成测试
//
// 测试路由模块与系统其他组件的集成

use core_net::{
    Context,
    Ipv4Route, Ipv6Route, RouteError,
    Ipv4Addr, Ipv6Addr, IpAddr,
    InterfaceConfig, InterfaceState, MacAddr,
};

// ========== 测试辅助函数 ==========

/// 创建测试用的系统上下文
fn create_test_context() -> Context {
    let ctx = Context::new();

    // 添加测试接口
    let eth0_config = InterfaceConfig {
        name: "eth0".to_string(),
        mac_addr: MacAddr::new([0x00, 0x11, 0x22, 0x33, 0x44, 0x55]),
        ip_addr: Ipv4Addr::new(192, 168, 1, 100),
        ipv6_addr: Ipv6Addr::new(0xfe80, 0, 0, 0, 0, 0, 0, 1),
        netmask: Ipv4Addr::new(255, 255, 255, 0),
        gateway: Some(Ipv4Addr::new(192, 168, 1, 1)),
        mtu: Some(1500),
        state: Some(InterfaceState::Up),
    };

    ctx.interfaces.lock().unwrap()
        .add_from_config(eth0_config)
        .unwrap();

    ctx
}

/// 创建测试用的 IPv4 路由
fn create_test_ipv4_route(
    dest: Ipv4Addr,
    mask: Ipv4Addr,
    gateway: Option<Ipv4Addr>,
    interface: &str,
) -> Ipv4Route {
    Ipv4Route::new(dest, mask, gateway, interface.to_string())
}

// ========== 场景一：路由表与 SystemContext 集成 ==========

#[test]
fn test_route_table_in_context() {
    let ctx = create_test_context();

    // 添加 IPv4 路由
    let result = ctx.route_table.lock().unwrap()
        .add_ipv4_route(create_test_ipv4_route(
            Ipv4Addr::new(192, 168, 0, 0),
            Ipv4Addr::new(255, 255, 0, 0),
            Some(Ipv4Addr::new(192, 168, 1, 1)),
            "eth0",
        ));

    assert!(result.is_ok());

    // 验证路由可以查询
    let route = ctx.route_table.lock().unwrap()
        .lookup_ipv4(Ipv4Addr::new(192, 168, 1, 100));

    assert!(route.is_some());
    let lookup = route.unwrap();
    assert_eq!(lookup.interface, "eth0");
    assert_eq!(
        lookup.next_hop,
        Some(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1)))
    );
}

#[test]
fn test_route_table_clone_shares_state() {
    let ctx1 = create_test_context();
    let ctx2 = ctx1.clone();

    // 通过 ctx1 添加路由
    ctx1.route_table.lock().unwrap()
        .add_ipv4_route(create_test_ipv4_route(
            Ipv4Addr::new(10, 0, 0, 0),
            Ipv4Addr::new(255, 0, 0, 0),
            Some(Ipv4Addr::new(192, 168, 1, 1)),
            "eth0",
        ))
        .unwrap();

    // 通过 ctx2 可以查到路由
    let route = ctx2.route_table.lock().unwrap()
        .lookup_ipv4(Ipv4Addr::new(10, 0, 0, 1));

    assert!(route.is_some());
}

// ========== 场景二：路由查找与接口管理器集成 ==========

#[test]
fn test_route_lookup_with_interface_verification() {
    let ctx = create_test_context();

    // 添加路由指向不存在的接口
    let result = ctx.route_table.lock().unwrap()
        .add_ipv4_route(create_test_ipv4_route(
            Ipv4Addr::new(10, 0, 0, 0),
            Ipv4Addr::new(255, 0, 0, 0),
            None, // 直连网络
            "eth1", // 不存在的接口
        ));

    // 路由添加成功（不验证接口存在性）
    assert!(result.is_ok());

    // 查找路由
    let route = ctx.route_table.lock().unwrap()
        .lookup_ipv4(Ipv4Addr::new(10, 0, 0, 1));

    assert!(route.is_some());
    let lookup = route.unwrap();
    assert_eq!(lookup.interface, "eth1");

    // 验证接口是否实际存在
    let interfaces = ctx.interfaces.lock().unwrap();
    let iface_result = interfaces.get_by_name("eth1");

    assert!(iface_result.is_err(), "接口应该不存在");
}

#[test]
fn test_route_lookup_to_existing_interface() {
    let ctx = create_test_context();

    // 添加路由指向存在的接口
    ctx.route_table.lock().unwrap()
        .add_ipv4_route(create_test_ipv4_route(
            Ipv4Addr::new(192, 168, 0, 0),
            Ipv4Addr::new(255, 255, 0, 0),
            None, // 直连网络
            "eth0",
        ))
        .unwrap();

    // 查找路由
    let route = ctx.route_table.lock().unwrap()
        .lookup_ipv4(Ipv4Addr::new(192, 168, 1, 100));

    assert!(route.is_some());
    let lookup = route.unwrap();
    assert_eq!(lookup.interface, "eth0");

    // 验证接口存在
    let interfaces = ctx.interfaces.lock().unwrap();
    let iface = interfaces.get_by_name("eth0");

    assert!(iface.is_ok());
    let iface = iface.unwrap();
    assert_eq!(iface.name(), "eth0");
    assert_eq!(iface.ip_addr, Ipv4Addr::new(192, 168, 1, 100));
}

// ========== 场景三：IPv6 路由与接口集成 ==========

#[test]
fn test_ipv6_route_in_context() {
    let ctx = create_test_context();

    // 添加 IPv6 路由
    let result = ctx.route_table.lock().unwrap()
        .add_ipv6_route(Ipv6Route::new(
            Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 0),
            32,
            None,
            "eth0".to_string(),
        ));

    assert!(result.is_ok());

    // 验证路由可以查询
    let route = ctx.route_table.lock().unwrap()
        .lookup_ipv6(Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 1));

    assert!(route.is_some());
    let lookup = route.unwrap();
    assert_eq!(lookup.interface, "eth0");
    assert!(lookup.next_hop.is_none()); // 直连网络
}

// ========== 场景四：默认路由处理 ==========

#[test]
fn test_default_route_fallback() {
    let ctx = create_test_context();

    // 添加具体路由
    ctx.route_table.lock().unwrap()
        .add_ipv4_route(create_test_ipv4_route(
            Ipv4Addr::new(192, 168, 0, 0),
            Ipv4Addr::new(255, 255, 0, 0),
            None,
            "eth0",
        ))
        .unwrap();

    // 添加默认路由
    ctx.route_table.lock().unwrap()
        .add_ipv4_route(create_test_ipv4_route(
            Ipv4Addr::new(0, 0, 0, 0),
            Ipv4Addr::new(0, 0, 0, 0),
            Some(Ipv4Addr::new(192, 168, 1, 1)),
            "eth0",
        ))
        .unwrap();

    // 匹配具体路由的地址
    let route = ctx.route_table.lock().unwrap()
        .lookup_ipv4(Ipv4Addr::new(192, 168, 1, 100));
    assert!(route.is_some());
    assert_eq!(route.unwrap().next_hop, None); // 直连

    // 不匹配具体路由，应该使用默认路由
    let route = ctx.route_table.lock().unwrap()
        .lookup_ipv4(Ipv4Addr::new(8, 8, 8, 8));
    assert!(route.is_some());
    let lookup = route.unwrap();
    assert_eq!(
        lookup.next_hop,
        Some(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1)))
    );
}

// ========== 场景五：路由表持久化与清空 ==========

#[test]
fn test_route_table_persistence() {
    let ctx = create_test_context();

    // 添加多条路由
    for i in 0..5 {
        let dest = Ipv4Addr::new(192, 168, i, 0);
        let mask = Ipv4Addr::new(255, 255, 255, 0);
        ctx.route_table.lock().unwrap()
            .add_ipv4_route(create_test_ipv4_route(
                dest, mask, None, "eth0",
            ))
            .unwrap();
    }

    // 验证路由数量
    let count = ctx.route_table.lock().unwrap().ipv4_routes().len();
    assert_eq!(count, 5);

    // 清空路由表
    ctx.route_table.lock().unwrap().clear();

    // 验证已清空
    assert!(ctx.route_table.lock().unwrap().is_empty());
}

// ========== 场景六：路由删除操作 ==========

#[test]
fn test_route_deletion() {
    let ctx = create_test_context();

    // 添加路由
    ctx.route_table.lock().unwrap()
        .add_ipv4_route(create_test_ipv4_route(
            Ipv4Addr::new(192, 168, 0, 0),
            Ipv4Addr::new(255, 255, 0, 0),
            None,
            "eth0",
        ))
        .unwrap();

    // 验证路由存在
    let route = ctx.route_table.lock().unwrap()
        .lookup_ipv4(Ipv4Addr::new(192, 168, 1, 100));
    assert!(route.is_some());

    // 删除路由
    let result = ctx.route_table.lock().unwrap()
        .remove_ipv4_route(
            Ipv4Addr::new(192, 168, 0, 0),
            Ipv4Addr::new(255, 255, 0, 0),
        );

    assert!(result.is_ok());

    // 验证路由已删除
    let route = ctx.route_table.lock().unwrap()
        .lookup_ipv4(Ipv4Addr::new(192, 168, 1, 100));
    assert!(route.is_none());
}

#[test]
fn test_delete_nonexistent_route() {
    let ctx = create_test_context();

    // 删除不存在的路由
    let result = ctx.route_table.lock().unwrap()
        .remove_ipv4_route(
            Ipv4Addr::new(10, 0, 0, 0),
            Ipv4Addr::new(255, 0, 0, 0),
        );

    assert!(result.is_err());
    if let Err(RouteError::RouteNotFound { destination }) = result {
        assert!(destination.contains("10.0.0.0"));
    } else {
        panic!("应该返回 RouteNotFound 错误");
    }
}

// ========== 场景七：路由优先级（metric） ==========

#[test]
fn test_route_metric() {
    let ctx = create_test_context();

    // 添加带优先级的路由
    let route1 = Ipv4Route::with_metric(
        Ipv4Addr::new(10, 0, 0, 0),
        Ipv4Addr::new(255, 0, 0, 0),
        Some(Ipv4Addr::new(192, 168, 1, 1)),
        "eth0".to_string(),
        100,
    );

    ctx.route_table.lock().unwrap()
        .add_ipv4_route(route1)
        .unwrap();

    // 查找路由并验证优先级
    let route = ctx.route_table.lock().unwrap()
        .lookup_ipv4(Ipv4Addr::new(10, 0, 0, 1));

    assert!(route.is_some());
    assert_eq!(route.unwrap().metric, 100);
}

// ========== 场景八：IPv6 默认路由 ==========

#[test]
fn test_ipv6_default_route() {
    let ctx = create_test_context();

    // 添加 IPv6 默认路由
    ctx.route_table.lock().unwrap()
        .add_ipv6_route(Ipv6Route::new(
            Ipv6Addr::UNSPECIFIED,
            0,
            Some(Ipv6Addr::new(0xfe80, 0, 0, 0, 0, 0, 0, 1)),
            "eth0".to_string(),
        ))
        .unwrap();

    // 任意 IPv6 地址都应该匹配默认路由
    let test_addrs = vec![
        Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 1),
        Ipv6Addr::new(0x2001, 0xb8, 0, 0, 0, 0, 0, 1),
        Ipv6Addr::new(0x2400, 0, 0, 0, 0, 0, 0, 1),
    ];

    for addr in test_addrs {
        let route = ctx.route_table.lock().unwrap()
            .lookup_ipv6(addr);

        assert!(route.is_some(), "应该匹配默认路由: {}", addr);
        let lookup = route.unwrap();
        assert_eq!(lookup.interface, "eth0");
        assert!(lookup.next_hop.is_some());
    }
}

// ========== 场景九：直连路由与网关路由 ==========

#[test]
fn test_direct_route_vs_gateway_route() {
    let ctx = create_test_context();

    // 添加直连路由（无网关）
    ctx.route_table.lock().unwrap()
        .add_ipv4_route(create_test_ipv4_route(
            Ipv4Addr::new(192, 168, 1, 0),
            Ipv4Addr::new(255, 255, 255, 0),
            None, // 直连
            "eth0",
        ))
        .unwrap();

    // 添加网关路由
    ctx.route_table.lock().unwrap()
        .add_ipv4_route(create_test_ipv4_route(
            Ipv4Addr::new(10, 0, 0, 0),
            Ipv4Addr::new(255, 0, 0, 0),
            Some(Ipv4Addr::new(192, 168, 1, 254)),
            "eth0",
        ))
        .unwrap();

    // 直连路由 - 无下一跳
    let route1 = ctx.route_table.lock().unwrap()
        .lookup_ipv4(Ipv4Addr::new(192, 168, 1, 100));
    assert!(route1.is_some());
    assert!(route1.unwrap().next_hop.is_none());

    // 网关路由 - 有下一跳
    let route2 = ctx.route_table.lock().unwrap()
        .lookup_ipv4(Ipv4Addr::new(10, 0, 0, 1));
    assert!(route2.is_some());
    assert_eq!(
        route2.unwrap().next_hop,
        Some(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 254)))
    );
}

// ========== 场景十：路由表遍历 ==========

#[test]
fn test_route_table_iteration() {
    let ctx = create_test_context();

    // 添加多条路由
    let routes = vec![
        (Ipv4Addr::new(192, 168, 0, 0), Ipv4Addr::new(255, 255, 0, 0)),
        (Ipv4Addr::new(10, 0, 0, 0), Ipv4Addr::new(255, 0, 0, 0)),
        (Ipv4Addr::new(172, 16, 0, 0), Ipv4Addr::new(255, 240, 0, 0)),
    ];

    for (dest, mask) in &routes {
        ctx.route_table.lock().unwrap()
            .add_ipv4_route(create_test_ipv4_route(
                *dest, *mask, None, "eth0",
            ))
            .unwrap();
    }

    // 遍历路由
    let route_table = ctx.route_table.lock().unwrap();
    let ipv4_routes = route_table.ipv4_routes();
    assert_eq!(ipv4_routes.len(), 3);

    // 验证路由存在
    for (dest, mask) in &routes {
        let found = ipv4_routes.iter().any(|r| {
            r.destination == *dest && r.netmask == *mask
        });
        assert!(found, "应该找到路由: {}/{}", dest, mask);
    }
}
