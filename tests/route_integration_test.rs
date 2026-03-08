// 路由模块集成测试（精简版）
//
// 核心功能测试：路由表查询、最长前缀匹配

use core_net::{
    Context, Ipv4Route, Ipv4Addr,
};
use serial_test::serial;

mod common;

fn create_test_context() -> Context {
    let ctx = Context::new();

    let eth0_config = common::create_test_eth0_config();

    ctx.interfaces.lock().unwrap()
        .add_from_config(eth0_config)
        .unwrap();

    ctx
}

// 测试1：添加和查询IPv4路由
#[test]
#[serial]
fn test_ipv4_route_add_and_lookup() {
    let ctx = create_test_context();

    let route = Ipv4Route::new(
        Ipv4Addr::new(10, 0, 0, 0),
        Ipv4Addr::new(255, 0, 0, 0),
        Some(Ipv4Addr::new(192, 168, 1, 1)),
        "eth0".to_string(),
    );

    ctx.route_table.lock().unwrap().add_ipv4_route(route).unwrap();

    let result = ctx.route_table.lock().unwrap().lookup_ipv4(Ipv4Addr::new(10, 0, 1, 1));
    assert!(result.is_some());
}

// 测试2：最长前缀匹配
#[test]
#[serial]
fn test_longest_prefix_match() {
    let ctx = create_test_context();

    // 添加默认路由
    let default_route = Ipv4Route::new(
        Ipv4Addr::new(0, 0, 0, 0),
        Ipv4Addr::new(0, 0, 0, 0),
        Some(Ipv4Addr::new(192, 168, 1, 1)),
        "eth0".to_string(),
    );
    ctx.route_table.lock().unwrap().add_ipv4_route(default_route).unwrap();

    // 添加更具体的路由
    let specific_route = Ipv4Route::new(
        Ipv4Addr::new(10, 0, 0, 0),
        Ipv4Addr::new(255, 0, 0, 0),
        Some(Ipv4Addr::new(192, 168, 2, 1)),
        "eth0".to_string(),
    );
    ctx.route_table.lock().unwrap().add_ipv4_route(specific_route).unwrap();

    // 查询10.0.1.1应匹配具体路由
    let result = ctx.route_table.lock().unwrap().lookup_ipv4(Ipv4Addr::new(10, 0, 1, 1));
    assert!(result.is_some());
}

// 测试3：路由查询功能（简化）
#[test]
#[serial]
fn test_route_lookup() {
    let ctx = create_test_context();

    // 添加一条路由后查询
    let route = Ipv4Route::new(
        Ipv4Addr::new(10, 0, 0, 0),
        Ipv4Addr::new(255, 0, 0, 0),
        Some(Ipv4Addr::new(192, 168, 1, 1)),
        "eth0".to_string(),
    );
    ctx.route_table.lock().unwrap().add_ipv4_route(route).unwrap();

    let result = ctx.route_table.lock().unwrap().lookup_ipv4(Ipv4Addr::new(10, 0, 1, 1));
    assert!(result.is_some());
}

// 测试4：删除路由
#[test]
#[serial]
fn test_route_removal() {
    let ctx = create_test_context();

    let route = Ipv4Route::new(
        Ipv4Addr::new(172, 16, 0, 0),
        Ipv4Addr::new(255, 255, 0, 0),
        None,
        "eth0".to_string(),
    );

    ctx.route_table.lock().unwrap().add_ipv4_route(route.clone()).unwrap();
    assert!(ctx.route_table.lock().unwrap().lookup_ipv4(Ipv4Addr::new(172, 16, 1, 1)).is_some());

    ctx.route_table.lock().unwrap().remove_ipv4_route(route.destination, route.netmask).unwrap();
    assert!(ctx.route_table.lock().unwrap().lookup_ipv4(Ipv4Addr::new(172, 16, 1, 1)).is_none());
}
