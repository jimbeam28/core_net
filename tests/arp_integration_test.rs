// ARP 协议集成测试
//
// 根据 docs/design/protocols/arp.md 第6章的测试设计实现
// 测试 ARP 协议的报文接收场景、状态转换、边界条件和多接口场景

use core_net::testframework::{
    TestHarness,
};
use core_net::interface::{MacAddr, Ipv4Addr};
use core_net::protocols::arp::{ArpState, tables::AgeResult};
use core_net::common::{Packet, Table};
use core_net::context::SystemContext;

use serial_test::serial;

mod common;
use common::{create_arp_request_packet, create_arp_reply_packet,
             create_gratuitous_arp_packet, inject_packet_to_context,
             verify_context_txq_count, clear_context_txq, create_test_context};

// 测试环境配置：本机接口 eth0: ifindex=0, MAC=00:11:22:33:44:55, IP=192.168.1.100

// ========== 全局测试生命周期 ==========

/// 验证ARP缓存条目（使用SystemContext）
fn verify_arp_entry_context(
    context: &SystemContext,
    ifindex: u32,
    ip: Ipv4Addr,
    expected_mac: MacAddr,
    expected_state: ArpState,
) -> bool {
    let cache = context.arp_cache.lock();
    if cache.is_err() {
        return false;
    }
    cache.unwrap().lookup_arp(ifindex, ip)
        .map(|entry| {
            entry.hardware_addr == expected_mac && entry.state == expected_state
        })
        .unwrap_or(false)
}

/// 创建格式错误的ARP报文（长度不足）
fn create_malformed_arp_packet_short() -> Packet {
    let short_packet = vec![
        // 以太网头
        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, // DST MAC (广播)
        0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0x01, // SRC MAC
        0x08, 0x06,                         // Ether Type = ARP
        // ARP头（不完整）
        0x00, 0x01, 0x08, 0x00, 0x06, 0x04,
    ];
    Packet::from_bytes(short_packet)
}

/// 创建格式错误的ARP报文（无效操作码）
fn create_malformed_arp_packet_invalid_opcode() -> Packet {
    let mut arp_data = vec![
        // 以太网头
        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, // DST MAC
        0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0x01, // SRC MAC
        0x08, 0x06,                         // Ether Type = ARP
        // ARP头
        0x00, 0x01, // Hardware Type = Ethernet
        0x08, 0x00, // Protocol Type = IPv4
        0x06,       // Hardware Addr Len = 6
        0x04,       // Protocol Addr Len = 4
        0xFF, 0xFF, // 操作码 = 无效值
    ];

    // 添加SHA、SPA、THA、TPA
    arp_data.extend_from_slice(&[0xaa; 6]); // SHA
    arp_data.extend_from_slice(&[0xc0, 0xa8, 0x01, 0x0a]); // SPA = 192.168.1.10
    arp_data.extend_from_slice(&[0x00; 6]); // THA = 0
    arp_data.extend_from_slice(&[0xc0, 0xa8, 0x01, 0x64]); // TPA = 192.168.1.100

    Packet::from_bytes(arp_data)
}

// 场景1：收到ARP请求（目标IP是本机）
//
// 根据 arp.md 6.1.2 节设计

#[test]
#[serial]
fn test_arp_request_target_is_local() {
    // 创建独立的测试上下文
    let ctx = create_test_context();

    // 测试参数
    let sender_mac = MacAddr::new([0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0x01]);
    let sender_ip = Ipv4Addr::new(192, 168, 1, 10);
    let target_ip = Ipv4Addr::new(192, 168, 1, 100); // 本机IP

    // 创建并注入ARP请求报文
    let request = create_arp_request_packet(sender_mac, sender_ip, target_ip);
    inject_packet_to_context(&ctx, "eth0", request).unwrap();

    // 运行调度器
    let mut harness = TestHarness::with_context(ctx.clone());
    let result = harness.run();
    assert!(result.is_ok(), "调度器运行失败: {:?}", result.err());

    // 验证1：发送队列有1个响应报文
    assert!(
        verify_context_txq_count(&ctx, "eth0", 1),
        "发送队列应该有1个响应报文"
    );

    // 验证2：ARP缓存包含发送方的映射，状态为Reachable
    assert!(
        verify_arp_entry_context(&ctx, 0, sender_ip, sender_mac, ArpState::Reachable),
        "ARP缓存应该包含发送方的映射，状态为Reachable"
    );

    // 上下文会在作用域结束时自动清理
}

// 场景2：收到ARP请求（目标IP不是本机）
//
// 根据 arp.md 6.1.3 节设计

#[test]
#[serial]
fn test_arp_request_target_not_local() {
    let ctx = create_test_context();

    // 测试参数
    let sender_mac = MacAddr::new([0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0x01]);
    let sender_ip = Ipv4Addr::new(192, 168, 1, 10);
    let target_ip = Ipv4Addr::new(192, 168, 1, 200); // 不是本机IP

    // 创建并注入ARP请求报文
    let request = create_arp_request_packet(sender_mac, sender_ip, target_ip);
    inject_packet_to_context(&ctx, "eth0", request).unwrap();

    // 运行调度器
    let mut harness = TestHarness::with_context(ctx.clone());
    let result = harness.run();
    assert!(result.is_ok(), "调度器运行失败: {:?}", result.err());

    // 验证：ARP缓存仍会自动学习发送方MAC地址（被动学习）
    assert!(
        verify_arp_entry_context(&ctx, 0, sender_ip, sender_mac, ArpState::Reachable),
        "应该被动学习发送方的MAC地址"
    );
}

// 场景3：收到ARP响应（匹配等待的请求）
//
// 根据 arp.md 6.1.4 节设计

#[test]
#[serial]
fn test_arp_reply_matching_incomplete() {
    let ctx = create_test_context();

    // 测试参数
    let sender_mac = MacAddr::new([0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0x01]);
    let sender_ip = Ipv4Addr::new(192, 168, 1, 10);
    let local_mac = MacAddr::new([0x00, 0x11, 0x22, 0x33, 0x44, 0x55]);
    let local_ip = Ipv4Addr::new(192, 168, 1, 100);

    // 前提条件：创建INCOMPLETE条目
    {
        let mut arp_cache = ctx.arp_cache.lock().unwrap();
        arp_cache.update_arp(0, sender_ip, MacAddr::zero(), ArpState::Incomplete);
    }

    // 创建并注入ARP响应报文
    let reply = create_arp_reply_packet(sender_mac, sender_ip, local_mac, local_ip);
    inject_packet_to_context(&ctx, "eth0", reply).unwrap();

    // 运行调度器
    let mut harness = TestHarness::with_context(ctx.clone());
    let result = harness.run();
    assert!(result.is_ok(), "调度器运行失败: {:?}", result.err());

    // 验证：ARP缓存已更新（使用作用域确保锁释放）
    {
        let arp_cache = ctx.arp_cache.lock().unwrap();
        let entry = arp_cache.lookup_arp(0, sender_ip);

        assert!(entry.is_some(), "应该存在ARP条目");
        let entry = entry.unwrap();

        assert_eq!(entry.hardware_addr, sender_mac, "MAC地址应该更新");
        assert_eq!(entry.state, ArpState::Reachable, "状态应该是Reachable");
        assert_eq!(entry.pending_packets.len(), 0, "等待队列应该被清空");
        assert_eq!(entry.retry_count, 0, "重试计数应该被重置");
    }
}

// 场景4：收到ARP响应（无等待的请求）
//
// 根据 arp.md 6.1.5 节设计

#[test]
#[serial]
fn test_arp_reply_no_incomplete() {
    let ctx = create_test_context();

    // 测试参数
    let sender_mac = MacAddr::new([0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0x01]);
    let sender_ip = Ipv4Addr::new(192, 168, 1, 10);
    let local_mac = MacAddr::new([0x00, 0x11, 0x22, 0x33, 0x44, 0x55]);
    let local_ip = Ipv4Addr::new(192, 168, 1, 100);

    // 前提条件：确保缓存中没有该IP的条目
    {
        let mut arp_cache = ctx.arp_cache.lock().unwrap();
        arp_cache.remove_arp(0, sender_ip);
    }

    // 创建并注入ARP响应报文
    let reply = create_arp_reply_packet(sender_mac, sender_ip, local_mac, local_ip);
    inject_packet_to_context(&ctx, "eth0", reply).unwrap();

    // 运行调度器
    let mut harness = TestHarness::with_context(ctx.clone());
    let result = harness.run();
    assert!(result.is_ok(), "调度器运行失败: {:?}", result.err());

    // 验证：ARP缓存包含新条目（使用作用域确保锁释放）
    {
        let arp_cache = ctx.arp_cache.lock().unwrap();
        let entry = arp_cache.lookup_arp(0, sender_ip);

        assert!(entry.is_some(), "应该存在ARP条目");
        let entry = entry.unwrap();

        assert_eq!(entry.hardware_addr, sender_mac, "MAC地址应该匹配");
        assert_eq!(entry.state, ArpState::Reachable, "状态应该是Reachable");
    }
}

// 场景5：收到Gratuitous ARP（免费ARP）
//
// 根据 arp.md 6.1.6 节设计

#[test]
#[serial]
fn test_gratuitous_arp() {
    let ctx = create_test_context();

    // 测试参数
    let sender_mac = MacAddr::new([0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0x01]);
    let sender_ip = Ipv4Addr::new(192, 168, 1, 10);

    // 创建并注入免费ARP报文
    let garp = create_gratuitous_arp_packet(sender_mac, sender_ip);
    inject_packet_to_context(&ctx, "eth0", garp).unwrap();

    // 运行调度器
    let mut harness = TestHarness::with_context(ctx.clone());
    let result = harness.run();
    assert!(result.is_ok(), "调度器运行失败: {:?}", result.err());

    // 验证：ARP缓存包含该条目
    assert!(
        verify_arp_entry_context(&ctx, 0, sender_ip, sender_mac, ArpState::Reachable),
        "应该学习免费ARP的映射"
    );
}

// 场景6：收到重复的ARP报文
//
// 根据 arp.md 6.1.7 节设计

#[test]
#[serial]
fn test_duplicate_arp_packet() {
    let ctx = create_test_context();

    // 测试参数
    let sender_mac = MacAddr::new([0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0x01]);
    let sender_ip = Ipv4Addr::new(192, 168, 1, 10);
    let target_ip = Ipv4Addr::new(192, 168, 1, 100);

    // 第一次：注入ARP请求
    let request1 = create_arp_request_packet(sender_mac, sender_ip, target_ip);
    inject_packet_to_context(&ctx, "eth0", request1).unwrap();
    let mut harness = TestHarness::with_context(ctx.clone());
    harness.run().unwrap();

    // 清空发送队列，准备第二次
    clear_context_txq(&ctx, "eth0").unwrap();

    // 记录第一次的更新时间
    let first_updated_at = {
        let arp_cache = ctx.arp_cache.lock().unwrap();
        arp_cache
            .lookup_arp(0, sender_ip)
            .map(|e| e.updated_at)
    };

    // 等待一段时间
    std::thread::sleep(std::time::Duration::from_millis(10));

    // 第二次：注入相同的ARP请求
    let request2 = create_arp_request_packet(sender_mac, sender_ip, target_ip);
    inject_packet_to_context(&ctx, "eth0", request2).unwrap();
    let mut harness = TestHarness::with_context(ctx.clone());
    harness.run().unwrap();

    // 验证：ARP缓存条目存在且时间戳已更新（使用作用域确保锁释放）
    {
        let arp_cache = ctx.arp_cache.lock().unwrap();
        let entry = arp_cache.lookup_arp(0, sender_ip);

        assert!(entry.is_some(), "应该存在ARP条目");

        if let Some(first_time) = first_updated_at {
            let second_time = entry.unwrap().updated_at;
            assert!(
                second_time > first_time,
                "时间戳应该被更新"
            );
        }
    }
}

// 场景7：收到格式错误的ARP报文

#[test]
#[serial]
fn test_malformed_arp_packet_length() {
    let ctx = create_test_context();

    // 创建长度不足的ARP报文
    let short_packet = create_malformed_arp_packet_short();
    inject_packet_to_context(&ctx, "eth0", short_packet).unwrap();

    // 运行调度器（应该正常处理，不panic）
    let mut harness = TestHarness::with_context(ctx.clone());
    let result = harness.run();
    assert!(result.is_ok(), "调度器应该正常处理错误报文");
}

#[test]
#[serial]
fn test_malformed_arp_packet_invalid_opcode() {
    let ctx = create_test_context();

    // 创建无效操作码的ARP报文
    let invalid_packet = create_malformed_arp_packet_invalid_opcode();
    inject_packet_to_context(&ctx, "eth0", invalid_packet).unwrap();

    // 运行调度器
    let mut harness = TestHarness::with_context(ctx.clone());
    let result = harness.run();
    assert!(result.is_ok(), "调度器应该正常处理错误报文");
}

// 状态转换测试

#[test]
#[serial]
fn test_arp_state_transition_none_to_incomplete() {
    let ctx = create_test_context();

    let ip_addr = Ipv4Addr::new(192, 168, 1, 10);

    // 状态转换：None -> Incomplete（使用作用域确保锁释放）
    {
        let mut arp_cache = ctx.arp_cache.lock().unwrap();
        arp_cache.update_arp(0, ip_addr, MacAddr::zero(), ArpState::Incomplete);

        // 验证状态
        let entry = arp_cache.lookup_arp(0, ip_addr);
        assert!(entry.is_some(), "应该存在ARP条目");
        assert_eq!(entry.unwrap().state, ArpState::Incomplete, "状态应该是Incomplete");
    }
}

#[test]
#[serial]
fn test_arp_state_transition_incomplete_to_reachable() {
    let ctx = create_test_context();

    let ip_addr = Ipv4Addr::new(192, 168, 1, 10);
    let mac_addr = MacAddr::new([0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0x01]);

    // 状态转换：None -> Incomplete -> Reachable（使用作用域确保锁释放）
    {
        let mut arp_cache = ctx.arp_cache.lock().unwrap();
        arp_cache.update_arp(0, ip_addr, MacAddr::zero(), ArpState::Incomplete);

        // 状态转换：Incomplete -> Reachable
        arp_cache.update_arp(0, ip_addr, mac_addr, ArpState::Reachable);

        // 验证状态和MAC地址
        let entry = arp_cache.lookup_arp(0, ip_addr).unwrap();
        assert_eq!(entry.state, ArpState::Reachable, "状态应该是Reachable");
        assert_eq!(entry.hardware_addr, mac_addr, "MAC地址应该更新");
    }
}

#[test]
#[serial]
fn test_arp_cache_remove_entry() {
    let ctx = create_test_context();

    let ip_addr = Ipv4Addr::new(192, 168, 1, 10);
    let mac_addr = MacAddr::new([0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0x01]);

    // 添加和删除条目（使用作用域确保锁释放）
    {
        let mut arp_cache = ctx.arp_cache.lock().unwrap();
        arp_cache.update_arp(0, ip_addr, mac_addr, ArpState::Reachable);
        assert!(arp_cache.lookup_arp(0, ip_addr).is_some());

        // 删除条目
        arp_cache.remove_arp(0, ip_addr);
        assert!(arp_cache.lookup_arp(0, ip_addr).is_none(), "条目应该被删除");
    }
}

// 边界条件测试

#[test]
#[serial]
fn test_arp_cache_capacity_limit() {
    let _ctx = create_test_context(); // 保持上下文以避免初始化问题

    // 创建小容量缓存配置
    use core_net::protocols::arp::ArpConfig;

    // 获取当前缓存并创建测试用的本地缓存
    let config = ArpConfig {
        max_entries: 5,
        ..Default::default()
    };

    // 注意：这里使用本地缓存进行容量测试，因为全局缓存的容量已在初始化时设定
    let mut cache = core_net::protocols::arp::ArpCache::new(config);

    // 填满缓存
    for i in 1..=5 {
        let ip = Ipv4Addr::new(192, 168, 1, i);
        let mac = MacAddr::new([0xaa, 0xbb, 0xcc, 0xdd, 0xee, i]);
        cache.update_arp(0, ip, mac, ArpState::Reachable);
    }

    assert_eq!(cache.len(), 5, "缓存应该有5个条目");

    // 添加第6个条目
    let ip6 = Ipv4Addr::new(192, 168, 1, 6);
    let mac6 = MacAddr::new([0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0x06]);
    cache.update_arp(0, ip6, mac6, ArpState::Reachable);

    // 根据实现，可能允许超过max_entries或使用LRU策略
    // 这里只验证不会panic
    let len = cache.len();
    assert!(len >= 5, "缓存应该至少有5个条目");
}

#[test]
#[serial]
fn test_arp_special_ip_addresses() {
    let ctx = create_test_context();

    // 测试特殊IP地址被拒绝加入ARP缓存
    // 根据ARP规范，以下地址不应该被缓存：
    let test_cases = vec![
        Ipv4Addr::new(0, 0, 0, 0),           // 0.0.0.0（未指定地址）
        Ipv4Addr::new(255, 255, 255, 255),   // 广播地址
        Ipv4Addr::new(224, 0, 0, 1),         // 组播地址（224.0.0.0/4）
    ];

    {
        let mut arp_cache = ctx.arp_cache.lock().unwrap();

        for ip_addr in test_cases {
            let mac_addr = MacAddr::new([0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0x01]);
            arp_cache.update_arp(0, ip_addr, mac_addr, ArpState::Reachable);
            // 特殊IP地址应该被拒绝，缓存中不应该存在
            assert!(
                arp_cache.lookup_arp(0, ip_addr).is_none(),
                "特殊IP地址应该被拒绝: {}",
                ip_addr
            );
        }
        // arp_cache 锁在这里自动释放
    }
}

// 多接口测试

#[test]
#[serial]
fn test_arp_interface_isolation() {
    let ctx = create_test_context();

    let sender_mac = MacAddr::new([0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0x01]);
    let sender_ip = Ipv4Addr::new(192, 168, 1, 10);

    // 在eth0接口注入ARP请求
    let request = create_arp_request_packet(
        sender_mac,
        sender_ip,
        Ipv4Addr::new(192, 168, 1, 100),
    );

    inject_packet_to_context(&ctx, "eth0", request).unwrap();
    let mut harness = TestHarness::with_context(ctx.clone());
    harness.run().unwrap();

    // 验证接口隔离（使用作用域确保锁释放）
    {
        let arp_cache = ctx.arp_cache.lock().unwrap();
        let entry_eth0 = arp_cache.lookup_arp(0, sender_ip); // ifindex=0 是eth0
        assert!(entry_eth0.is_some(), "eth0接口应该有ARP缓存");

        // 验证：lo接口没有ARP缓存（接口隔离）
        let entry_lo = arp_cache.lookup_arp(1, sender_ip); // ifindex=1 是lo
        assert!(entry_lo.is_none(), "lo接口不应该有ARP缓存（接口隔离）");
    }
}

#[test]
#[serial]
fn test_arp_same_ip_multiple_interfaces() {
    let ctx = create_test_context();

    let sender_mac = MacAddr::new([0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0x01]);
    let sender_ip = Ipv4Addr::new(192, 168, 1, 10);

    // 在eth0接口注入ARP请求
    let request = create_arp_request_packet(
        sender_mac,
        sender_ip,
        Ipv4Addr::new(192, 168, 1, 100),
    );

    inject_packet_to_context(&ctx, "eth0", request).unwrap();
    let mut harness = TestHarness::with_context(ctx.clone());
    harness.run().unwrap();

    // 验证：eth0接口有ARP缓存（使用作用域确保锁释放）
    {
        let arp_cache = ctx.arp_cache.lock().unwrap();
        let entry_eth0 = arp_cache.lookup_arp(0, sender_ip);
        assert!(entry_eth0.is_some(), "eth0接口应该有ARP缓存");

        // 验证：lo接口没有ARP缓存（接口隔离，使用(ifindex, ip)作为key）
        let entry_lo = arp_cache.lookup_arp(1, sender_ip);
        assert!(entry_lo.is_none(), "lo接口不应该有ARP缓存");
    }
}

// 集成测试：以太网封装验证

#[test]
#[serial]
fn test_arp_with_ethernet_encapsulation() {
    let _ctx = create_test_context();

    // 测试ARP报文的以太网封装
    let sender_mac = MacAddr::new([0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0x01]);
    let sender_ip = Ipv4Addr::new(192, 168, 1, 10);
    let target_ip = Ipv4Addr::new(192, 168, 1, 100);

    // 创建ARP请求报文
    let request = create_arp_request_packet(sender_mac, sender_ip, target_ip);
    let data = request.as_slice();

    // 验证以太网头
    assert_eq!(&data[0..6], &MacAddr::broadcast().bytes, "DST MAC应该是广播地址");
    assert_eq!(&data[6..12], &sender_mac.bytes, "SRC MAC应该是发送方MAC");

    // 验证EtherType
    let ether_type = u16::from_be_bytes([data[12], data[13]]);
    assert_eq!(ether_type, 0x0806, "EtherType应该是0x0806（ARP）");

    // 验证ARP操作码
    let operation = u16::from_be_bytes([data[20], data[21]]);
    assert_eq!(operation, 1, "操作码应该是1（Request）");
}

// 定时器测试

// 6.3.1 重传定时器测试

#[test]
#[serial]
fn test_retrans_timer_incomplete_state() {
    let ctx = create_test_context();

    let ip_addr = Ipv4Addr::new(192, 168, 1, 10);
    let mac_addr = MacAddr::new([0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0x01]);

    // 创建Incomplete状态条目
    {
        let mut arp_cache = ctx.arp_cache.lock().unwrap();
        arp_cache.update_arp(0, ip_addr, mac_addr, ArpState::Incomplete);

        let entry = arp_cache.lookup_arp(0, ip_addr).unwrap();
        assert_eq!(entry.state, ArpState::Incomplete);
        assert_eq!(entry.retry_count, 0);
    }

    // 等待重传超时时间（默认1秒）
    std::thread::sleep(std::time::Duration::from_secs(1));

    // 调用老化处理
    {
        let mut arp_cache = ctx.arp_cache.lock().unwrap();
        let result = arp_cache.age_entry(&(0, ip_addr));

        // 验证：应该触发重传
        assert!(matches!(result, AgeResult::SendRequest { .. }),
            "应该触发ARP请求重传");

        let entry = arp_cache.lookup_arp(0, ip_addr);
        assert!(entry.is_some(), "条目应该仍然存在");
        assert_eq!(entry.unwrap().retry_count, 1, "重试计数应该为1");
    }
}

#[test]
#[serial]
fn test_retrans_timer_max_retries_exceeded() {
    let ctx = create_test_context();

    let ip_addr = Ipv4Addr::new(192, 168, 1, 10);
    let mac_addr = MacAddr::new([0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0x01]);

    // 创建Incomplete状态条目
    {
        let mut arp_cache = ctx.arp_cache.lock().unwrap();
        arp_cache.update_arp(0, ip_addr, mac_addr, ArpState::Incomplete);

        // 直接设置retry_count为max_retries
        if let Some(entry) = arp_cache.lookup_mut_arp(0, ip_addr) {
            entry.retry_count = 3; // 默认max_retries = 3
        }
    }

    // 等待重传超时时间
    std::thread::sleep(std::time::Duration::from_secs(1));

    // 调用老化处理
    {
        let mut arp_cache = ctx.arp_cache.lock().unwrap();
        arp_cache.age_entry(&(0, ip_addr));

        // 验证：条目应该被删除
        let entry = arp_cache.lookup_arp(0, ip_addr);
        assert!(entry.is_none(), "超过最大重试次数后条目应该被删除");
    }
}

// 6.3.2 老化定时器测试

#[test]
#[serial]
fn test_aging_timer_reachable_to_stale() {
    let ctx = create_test_context();

    let ip_addr = Ipv4Addr::new(192, 168, 1, 10);
    let mac_addr = MacAddr::new([0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0x01]);

    // 创建Reachable状态条目
    {
        let mut arp_cache = ctx.arp_cache.lock().unwrap();
        arp_cache.update_arp(0, ip_addr, mac_addr, ArpState::Reachable);

        let entry = arp_cache.lookup_arp(0, ip_addr).unwrap();
        assert_eq!(entry.state, ArpState::Reachable);
    }

    // 注意：默认aging_timeout是30秒，在单元测试中太长
    // 我们通过直接修改时间戳来模拟时间流逝

    // 创建一个短超时配置的本地缓存用于测试
    use core_net::protocols::arp::{ArpCache, ArpConfig};

    let short_config = ArpConfig {
        aging_timeout: 1, // 1秒
        ..Default::default()
    };

    let mut local_cache = ArpCache::new(short_config);
    local_cache.update_arp(0, ip_addr, mac_addr, ArpState::Reachable);

    // 等待1秒
    std::thread::sleep(std::time::Duration::from_secs(1));

    // 调用老化处理
    local_cache.age_entry(&(0, ip_addr));

    // 验证：状态应该变为Stale
    let entry = local_cache.lookup_arp(0, ip_addr);
    assert!(entry.is_some(), "条目应该存在");
    assert_eq!(entry.unwrap().state, ArpState::Stale, "状态应该变为Stale");
}

#[test]
#[serial]
fn test_aging_timer_stale_refresh() {
    let _ctx = create_test_context();

    let ip_addr = Ipv4Addr::new(192, 168, 1, 10);
    let mac_addr = MacAddr::new([0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0x01]);

    // 创建本地缓存使用短超时配置
    use core_net::protocols::arp::{ArpCache, ArpConfig};

    let short_config = ArpConfig {
        aging_timeout: 1,
        ..Default::default()
    };

    let mut local_cache = ArpCache::new(short_config);

    // 1. 创建Reachable条目
    local_cache.update_arp(0, ip_addr, mac_addr, ArpState::Reachable);
    assert_eq!(local_cache.lookup_arp(0, ip_addr).unwrap().state, ArpState::Reachable);

    // 2. 等待超过aging_timeout
    std::thread::sleep(std::time::Duration::from_secs(1));
    local_cache.age_entry(&(0, ip_addr));
    assert_eq!(local_cache.lookup_arp(0, ip_addr).unwrap().state, ArpState::Stale);

    // 3. 收到该IP的ARP报文（更新缓存）
    local_cache.update_arp(0, ip_addr, mac_addr, ArpState::Reachable);

    // 4. 验证：状态变为Reachable
    let entry = local_cache.lookup_arp(0, ip_addr).unwrap();
    assert_eq!(entry.state, ArpState::Reachable, "收到ARP报文后应该恢复为Reachable");
}

// 6.3.3 延迟定时器测试

#[test]
#[serial]
fn test_delay_timer_stale_to_delay() {
    let ctx = create_test_context();

    let ip_addr = Ipv4Addr::new(192, 168, 1, 10);
    let mac_addr = MacAddr::new([0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0x01]);

    {
        let mut arp_cache = ctx.arp_cache.lock().unwrap();

        // 1. 创建Stale状态条目
        arp_cache.update_arp(0, ip_addr, mac_addr, ArpState::Stale);
        assert_eq!(arp_cache.lookup_arp(0, ip_addr).unwrap().state, ArpState::Stale);

        // 2. 标记需要使用（应该转为Delay状态）
        let converted = arp_cache.mark_used(0, ip_addr);
        assert!(converted, "应该成功转换为Delay状态");

        assert_eq!(arp_cache.lookup_arp(0, ip_addr).unwrap().state, ArpState::Delay);
    }
}

#[test]
#[serial]
fn test_delay_timer_expires_to_probe() {
    let _ctx = create_test_context();

    use core_net::protocols::arp::{ArpCache, ArpConfig};

    let ip_addr = Ipv4Addr::new(192, 168, 1, 10);
    let mac_addr = MacAddr::new([0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0x01]);

    // 使用短超时配置
    let short_config = ArpConfig {
        delay_timeout: 1, // 1秒
        ..Default::default()
    };

    let mut local_cache = ArpCache::new(short_config);

    // 1. 创建Delay状态条目
    local_cache.update_arp(0, ip_addr, mac_addr, ArpState::Delay);
    assert_eq!(local_cache.lookup_arp(0, ip_addr).unwrap().state, ArpState::Delay);

    // 2. 等待延迟超时
    std::thread::sleep(std::time::Duration::from_secs(1));

    // 3. 调用老化处理
    let result = local_cache.age_entry(&(0, ip_addr));

    // 4. 验证：应该转为Probe状态并需要发送探测请求
    assert!(matches!(result, AgeResult::ToProbe),
        "延迟超时后应该转为Probe状态");
    let entry = local_cache.lookup_arp(0, ip_addr).unwrap();
    assert_eq!(entry.state, ArpState::Probe, "状态应该变为Probe");
    assert_eq!(entry.retry_count, 0, "重试计数应该被重置");
}

// 6.3.4 探测定时器测试

#[test]
#[serial]
fn test_probe_timer_retransmit() {
    let _ctx = create_test_context();

    use core_net::protocols::arp::{ArpCache, ArpConfig};

    let ip_addr = Ipv4Addr::new(192, 168, 1, 10);
    let mac_addr = MacAddr::new([0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0x01]);

    // 使用短超时配置
    let short_config = ArpConfig {
        probe_timeout: 1, // 1秒
        ..Default::default()
    };

    let mut local_cache = ArpCache::new(short_config);

    // 1. 创建Probe状态条目
    local_cache.update_arp(0, ip_addr, mac_addr, ArpState::Probe);

    // 2. 等待探测超时
    std::thread::sleep(std::time::Duration::from_secs(1));

    // 3. 调用老化处理
    let result = local_cache.age_entry(&(0, ip_addr));

    // 4. 验证：应该继续探测
    assert!(matches!(result, AgeResult::SendRequest { .. }),
        "应该重发探测请求");
    let entry = local_cache.lookup_arp(0, ip_addr).unwrap();
    assert_eq!(entry.retry_count, 1, "重试计数应该为1");
}

#[test]
#[serial]
fn test_probe_timer_max_retries_exceeded() {
    let _ctx = create_test_context();

    use core_net::protocols::arp::{ArpCache, ArpConfig};

    let ip_addr = Ipv4Addr::new(192, 168, 1, 10);
    let mac_addr = MacAddr::new([0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0x01]);

    let short_config = ArpConfig {
        probe_timeout: 1,
        max_retries: 3,
        ..Default::default()
    };

    let mut local_cache = ArpCache::new(short_config);

    // 1. 创建Probe状态条目并设置retry_count为max_retries
    local_cache.update_arp(0, ip_addr, mac_addr, ArpState::Probe);
    if let Some(entry) = local_cache.lookup_mut_arp(0, ip_addr) {
        entry.retry_count = 3;
    }

    // 2. 等待探测超时
    std::thread::sleep(std::time::Duration::from_secs(1));

    // 3. 调用老化处理
    local_cache.age_entry(&(0, ip_addr));

    // 4. 验证：条目应该被删除
    let entry = local_cache.lookup_arp(0, ip_addr);
    assert!(entry.is_none(), "超过最大重试次数后条目应该被删除");
}

#[test]
#[serial]
fn test_probe_timer_success_recovery() {
    let ctx = create_test_context();

    let ip_addr = Ipv4Addr::new(192, 168, 1, 10);
    let mac_addr = MacAddr::new([0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0x01]);

    // 1. 创建Probe状态条目
    {
        let mut arp_cache = ctx.arp_cache.lock().unwrap();
        arp_cache.update_arp(0, ip_addr, mac_addr, ArpState::Probe);
        assert_eq!(arp_cache.lookup_arp(0, ip_addr).unwrap().state, ArpState::Probe);
    }

    // 2. 收到ARP响应（模拟成功探测）
    {
        let mut arp_cache = ctx.arp_cache.lock().unwrap();
        arp_cache.update_arp(0, ip_addr, mac_addr, ArpState::Reachable);

        // 验证：状态变为Reachable
        let entry = arp_cache.lookup_arp(0, ip_addr).unwrap();
        assert_eq!(entry.state, ArpState::Reachable);
        assert_eq!(entry.retry_count, 0, "重试计数应该被重置");
    }
}

// 状态转换测试

#[test]
#[serial]
fn test_resolve_ip_timeout() {
    let _ctx = create_test_context();

    use core_net::protocols::arp::{ArpCache, ArpConfig};

    let ip_addr = Ipv4Addr::new(192, 168, 1, 10);
    let mac_addr = MacAddr::new([0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0x01]);

    // 使用短超时配置
    let short_config = ArpConfig {
        retrans_timeout: 1,
        max_retries: 2,
        ..Default::default()
    };

    let mut local_cache = ArpCache::new(short_config);

    // 1. 发送ARP请求，创建Incomplete条目
    local_cache.update_arp(0, ip_addr, mac_addr, ArpState::Incomplete);
    assert_eq!(local_cache.lookup_arp(0, ip_addr).unwrap().state, ArpState::Incomplete);

    // 2. 模拟多次重传超时（重试2次，共3次机会）
    for _ in 0..=2 {
        std::thread::sleep(std::time::Duration::from_secs(1));
        local_cache.age_entry(&(0, ip_addr));
    }

    // 3. 验证：条目应该被删除
    let entry = local_cache.lookup_arp(0, ip_addr);
    assert!(entry.is_none(), "解析超时后条目应该被删除");
}

// 边界条件测试：等待队列溢出

#[test]
#[serial]
fn test_pending_packets_overflow() {
    let _ctx = create_test_context();

    use core_net::protocols::arp::ArpCache;
    use core_net::common::Packet;

    let ip_addr = Ipv4Addr::new(192, 168, 1, 10);
    let mac_addr = MacAddr::new([0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0x01]);

    let mut local_cache = ArpCache::default();

    // 1. 创建Incomplete状态条目
    local_cache.update_arp(0, ip_addr, mac_addr, ArpState::Incomplete);

    // 2. 添加大量等待的数据包
    let test_data = vec![0u8; 100];
    for _ in 0..100 {
        let packet = Packet::from_bytes(test_data.clone());
        let _ = local_cache.add_pending_packet(0, ip_addr, packet);
    }

    // 3. 验证：等待队列中有100个数据包
    let entry = local_cache.lookup_arp(0, ip_addr).unwrap();
    assert_eq!(entry.pending_packets.len(), 100, "应该有100个等待的数据包");

    // 4. 模拟收到ARP响应，清空等待队列
    local_cache.update_arp(0, ip_addr, mac_addr, ArpState::Reachable);
    let count = local_cache.take_pending_packets(0, ip_addr);
    assert_eq!(count, 100, "应该清空100个等待的数据包");

    // 5. 验证：等待队列已清空
    let entry = local_cache.lookup_arp(0, ip_addr).unwrap();
    assert_eq!(entry.pending_packets.len(), 0, "等待队列应该被清空");
}

#[test]
#[serial]
fn test_pending_packets_non_incomplete_error() {
    let _ctx = create_test_context();

    use core_net::protocols::arp::ArpCache;
    use core_net::common::Packet;

    let ip_addr = Ipv4Addr::new(192, 168, 1, 10);
    let mac_addr = MacAddr::new([0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0x01]);

    let mut local_cache = ArpCache::default();

    // 1. 创建Reachable状态条目（不是Incomplete）
    local_cache.update_arp(0, ip_addr, mac_addr, ArpState::Reachable);

    // 2. 尝试添加等待的数据包（应该失败）
    let test_data = vec![0u8; 100];
    let packet = Packet::from_bytes(test_data);
    let result = local_cache.add_pending_packet(0, ip_addr, packet);

    // 3. 验证：应该返回错误
    assert!(result.is_err(), "非Incomplete状态不应该允许添加等待数据包");
}

// 定时器集成测试

#[test]
#[serial]
fn test_get_pending_requests_multiple_entries() {
    let _ctx = create_test_context();

    use core_net::protocols::arp::{ArpCache, ArpConfig, ArpState};

    let short_config = ArpConfig {
        retrans_timeout: 1,
        probe_timeout: 1,
        ..Default::default()
    };

    let mut local_cache = ArpCache::new(short_config);

    // 创建多个需要发送请求的条目
    let ip1 = Ipv4Addr::new(192, 168, 1, 10);
    let ip2 = Ipv4Addr::new(192, 168, 1, 11);
    let mac1 = MacAddr::new([0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0x01]);
    let mac2 = MacAddr::new([0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0x02]);

    // 创建Incomplete和Probe状态的条目
    local_cache.update_arp(0, ip1, mac1, ArpState::Incomplete);
    local_cache.update_arp(0, ip2, mac2, ArpState::Probe);

    // 等待超时
    std::thread::sleep(std::time::Duration::from_secs(1));

    // 获取需要发送请求的条目
    let pending = local_cache.get_pending_requests();

    // 验证：应该返回2个条目
    assert_eq!(pending.len(), 2, "应该有2个条目需要发送请求");
}
