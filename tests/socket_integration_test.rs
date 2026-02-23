// tests/socket_integration_test.rs
//
// Socket API 集成测试

use core_net::testframework::GlobalStateManager;
use core_net::socket::{
    AddressFamily, SendFlags, RecvFlags, SocketAddr, SocketAddrV4, SocketFd, SocketProtocol, SocketType,
};
use core_net::common::addr::Ipv4Addr;
use serial_test::serial;

// ========== 辅助函数 ==========

/// 创建测试用的 IPv4 Socket 地址
fn test_addr_v4(port: u16) -> SocketAddr {
    SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), port))
}

// ========== Socket 创建与销毁测试组 ==========

#[test]
#[serial]
fn test_socket_create_tcp() {
    let ctx = GlobalStateManager::create_context();
    let mut socket_mgr = ctx.socket_mgr.lock().unwrap();

    // 创建 TCP Socket
    let fd = socket_mgr
        .socket(AddressFamily::AF_INET, SocketType::SOCK_STREAM, SocketProtocol::Default)
        .unwrap();

    assert_eq!(fd.0, 3); // 第一个可用的 fd
    assert_eq!(socket_mgr.socket_count(), 1);

    // 验证 Socket 属性
    let entry = socket_mgr.get_entry(fd).unwrap();
    assert_eq!(entry.family, AddressFamily::AF_INET);
    assert_eq!(entry.socket_type, SocketType::SOCK_STREAM);
    assert_eq!(entry.protocol, SocketProtocol::TCP);
}

#[test]
#[serial]
fn test_socket_create_udp() {
    let ctx = GlobalStateManager::create_context();
    let mut socket_mgr = ctx.socket_mgr.lock().unwrap();

    // 创建 UDP Socket
    let fd = socket_mgr
        .socket(AddressFamily::AF_INET, SocketType::SOCK_DGRAM, SocketProtocol::Default)
        .unwrap();

    assert_eq!(socket_mgr.socket_count(), 1);

    let entry = socket_mgr.get_entry(fd).unwrap();
    assert_eq!(entry.socket_type, SocketType::SOCK_DGRAM);
    assert_eq!(entry.protocol, SocketProtocol::UDP);
}

#[test]
#[serial]
fn test_socket_create_multiple() {
    let ctx = GlobalStateManager::create_context();
    let mut socket_mgr = ctx.socket_mgr.lock().unwrap();

    let fd1 = socket_mgr
        .socket(AddressFamily::AF_INET, SocketType::SOCK_STREAM, SocketProtocol::Default)
        .unwrap();
    let fd2 = socket_mgr
        .socket(AddressFamily::AF_INET, SocketType::SOCK_DGRAM, SocketProtocol::Default)
        .unwrap();

    assert_eq!(socket_mgr.socket_count(), 2);
    assert_ne!(fd1.0, fd2.0);
}

#[test]
#[serial]
fn test_socket_close() {
    let ctx = GlobalStateManager::create_context();
    let mut socket_mgr = ctx.socket_mgr.lock().unwrap();

    let fd = socket_mgr
        .socket(AddressFamily::AF_INET, SocketType::SOCK_STREAM, SocketProtocol::Default)
        .unwrap();

    assert_eq!(socket_mgr.socket_count(), 1);

    socket_mgr.close(fd).unwrap();
    assert_eq!(socket_mgr.socket_count(), 0);
}

#[test]
#[serial]
fn test_socket_close_invalid_fd() {
    let ctx = GlobalStateManager::create_context();
    let mut socket_mgr = ctx.socket_mgr.lock().unwrap();

    let result = socket_mgr.close(SocketFd::INVALID);
    assert!(result.is_err());
}

// ========== 地址绑定测试组 ==========

#[test]
#[serial]
fn test_socket_bind() {
    let ctx = GlobalStateManager::create_context();
    let mut socket_mgr = ctx.socket_mgr.lock().unwrap();

    let fd = socket_mgr
        .socket(AddressFamily::AF_INET, SocketType::SOCK_DGRAM, SocketProtocol::Default)
        .unwrap();

    let addr = test_addr_v4(8080);
    socket_mgr.bind(fd, &addr).unwrap();

    let entry = socket_mgr.get_entry(fd).unwrap();
    assert!(entry.is_bound());
    assert_eq!(entry.local_addr, Some(addr));
}

#[test]
#[serial]
fn test_socket_bind_already_bound() {
    let ctx = GlobalStateManager::create_context();
    let mut socket_mgr = ctx.socket_mgr.lock().unwrap();

    let fd = socket_mgr
        .socket(AddressFamily::AF_INET, SocketType::SOCK_DGRAM, SocketProtocol::Default)
        .unwrap();

    let addr = test_addr_v4(8080);
    socket_mgr.bind(fd, &addr).unwrap();

    let result = socket_mgr.bind(fd, &addr);
    assert!(result.is_err());
}

#[test]
#[serial]
fn test_socket_bind_addr_in_use() {
    let ctx = GlobalStateManager::create_context();
    let mut socket_mgr = ctx.socket_mgr.lock().unwrap();

    let fd1 = socket_mgr
        .socket(AddressFamily::AF_INET, SocketType::SOCK_DGRAM, SocketProtocol::Default)
        .unwrap();
    let fd2 = socket_mgr
        .socket(AddressFamily::AF_INET, SocketType::SOCK_DGRAM, SocketProtocol::Default)
        .unwrap();

    let addr = test_addr_v4(8080);
    socket_mgr.bind(fd1, &addr).unwrap();

    let result = socket_mgr.bind(fd2, &addr);
    assert!(result.is_err());
}

#[test]
#[serial]
fn test_socket_bind_invalid_fd() {
    let ctx = GlobalStateManager::create_context();
    let mut socket_mgr = ctx.socket_mgr.lock().unwrap();

    let addr = test_addr_v4(8080);
    let result = socket_mgr.bind(SocketFd::INVALID, &addr);
    assert!(result.is_err());
}

// ========== TCP 监听测试组 ==========

#[test]
#[serial]
fn test_socket_listen() {
    let ctx = GlobalStateManager::create_context();
    let mut socket_mgr = ctx.socket_mgr.lock().unwrap();

    let fd = socket_mgr
        .socket(AddressFamily::AF_INET, SocketType::SOCK_STREAM, SocketProtocol::Default)
        .unwrap();

    let addr = test_addr_v4(8080);
    socket_mgr.bind(fd, &addr).unwrap();
    socket_mgr.listen(fd, 128).unwrap();

    let entry = socket_mgr.get_entry(fd).unwrap();
    assert!(entry.is_listening());
    assert!(entry.listen_queue.is_some());
}

#[test]
#[serial]
fn test_socket_listen_not_bound() {
    let ctx = GlobalStateManager::create_context();
    let mut socket_mgr = ctx.socket_mgr.lock().unwrap();

    let fd = socket_mgr
        .socket(AddressFamily::AF_INET, SocketType::SOCK_STREAM, SocketProtocol::Default)
        .unwrap();

    let result = socket_mgr.listen(fd, 128);
    assert!(result.is_err());
}

#[test]
#[serial]
fn test_socket_listen_not_stream() {
    let ctx = GlobalStateManager::create_context();
    let mut socket_mgr = ctx.socket_mgr.lock().unwrap();

    let fd = socket_mgr
        .socket(AddressFamily::AF_INET, SocketType::SOCK_DGRAM, SocketProtocol::Default)
        .unwrap();

    let addr = test_addr_v4(8080);
    socket_mgr.bind(fd, &addr).unwrap();

    let result = socket_mgr.listen(fd, 128);
    assert!(result.is_err());
}

// ========== 数据传输测试组 ==========

#[test]
#[serial]
fn test_socket_send_recv_tcp() {
    let ctx = GlobalStateManager::create_context();
    let mut socket_mgr = ctx.socket_mgr.lock().unwrap();

    let fd = socket_mgr
        .socket(AddressFamily::AF_INET, SocketType::SOCK_STREAM, SocketProtocol::Default)
        .unwrap();

    // 手动设置状态为 Established（模拟已连接）
    {
        let entry = socket_mgr.get_entry_mut(fd).unwrap();
        use core_net::socket::SocketState;
        use core_net::socket::TcpState;
        entry.state = SocketState::Tcp(TcpState::Established);
    }

    // 发送数据
    let data = b"Hello, World!";
    let sent = socket_mgr.send(fd, data, SendFlags::NONE).unwrap();
    assert_eq!(sent, 13);

    // 手动将数据移到接收缓冲区（模拟网络传输）
    {
        let entry = socket_mgr.get_entry_mut(fd).unwrap();
        let tx_data = entry.pop_tx().unwrap();
        entry.push_rx(tx_data);
    }

    // 接收数据
    let mut buf = [0u8; 64];
    let recv_len = socket_mgr.recv(fd, &mut buf, RecvFlags::NONE).unwrap();
    assert_eq!(recv_len, 13);
    assert_eq!(&buf[..13], b"Hello, World!");
}

#[test]
#[serial]
fn test_socket_sendto_recvfrom_udp() {
    let ctx = GlobalStateManager::create_context();
    let mut socket_mgr = ctx.socket_mgr.lock().unwrap();

    let fd = socket_mgr
        .socket(AddressFamily::AF_INET, SocketType::SOCK_DGRAM, SocketProtocol::Default)
        .unwrap();

    let addr = test_addr_v4(9090);
    socket_mgr.bind(fd, &addr).unwrap();

    // 发送数据
    let data = b"UDP Test";
    let dest_addr = test_addr_v4(8080);
    let sent = socket_mgr
        .sendto(fd, data, SendFlags::NONE, &dest_addr)
        .unwrap();
    assert_eq!(sent, 8);

    // 手动将数据移到接收缓冲区（模拟网络传输）
    {
        let entry = socket_mgr.get_entry_mut(fd).unwrap();
        let tx_data = entry.pop_tx().unwrap();
        entry.push_rx(tx_data);
    }

    // 接收数据
    let mut buf = [0u8; 64];
    let mut src_addr = None;
    let recv_len = socket_mgr
        .recvfrom(fd, &mut buf, RecvFlags::NONE, &mut src_addr)
        .unwrap();
    assert_eq!(recv_len, 8);
    assert_eq!(&buf[..8], b"UDP Test");
}

#[test]
#[serial]
fn test_socket_send_not_connected() {
    let ctx = GlobalStateManager::create_context();
    let mut socket_mgr = ctx.socket_mgr.lock().unwrap();

    let fd = socket_mgr
        .socket(AddressFamily::AF_INET, SocketType::SOCK_STREAM, SocketProtocol::Default)
        .unwrap();

    let data = b"Hello";
    let result = socket_mgr.send(fd, data, SendFlags::NONE);
    assert!(result.is_err());
}

#[test]
#[serial]
fn test_socket_recv_no_data() {
    let ctx = GlobalStateManager::create_context();
    let mut socket_mgr = ctx.socket_mgr.lock().unwrap();

    let fd = socket_mgr
        .socket(AddressFamily::AF_INET, SocketType::SOCK_DGRAM, SocketProtocol::Default)
        .unwrap();

    let addr = test_addr_v4(8080);
    socket_mgr.bind(fd, &addr).unwrap();

    let mut buf = [0u8; 64];
    let result = socket_mgr.recv(fd, &mut buf, RecvFlags::NONE);
    assert!(result.is_err());
}

// ========== 缓冲区管理测试组 ==========

#[test]
#[serial]
fn test_socket_buffer_operations() {
    let ctx = GlobalStateManager::create_context();
    let mut socket_mgr = ctx.socket_mgr.lock().unwrap();

    let fd = socket_mgr
        .socket(AddressFamily::AF_INET, SocketType::SOCK_DGRAM, SocketProtocol::Default)
        .unwrap();

    // 手动操作缓冲区
    {
        let entry = socket_mgr.get_entry_mut(fd).unwrap();
        assert!(entry.push_rx(vec![1, 2, 3]));
        assert!(entry.push_rx(vec![4, 5]));
        assert_eq!(entry.rx_buffer_used(), 5);

        let data = entry.pop_rx().unwrap();
        assert_eq!(data, vec![1, 2, 3]);
        assert_eq!(entry.rx_buffer_used(), 2);

        entry.clear_rx();
        assert_eq!(entry.rx_buffer_used(), 0);
    }
}

#[test]
#[serial]
fn test_socket_buffer_limit() {
    let ctx = GlobalStateManager::create_context();
    let mut socket_mgr = ctx.socket_mgr.lock().unwrap();

    let fd = socket_mgr
        .socket(AddressFamily::AF_INET, SocketType::SOCK_DGRAM, SocketProtocol::Default)
        .unwrap();

    // 设置较小的缓冲区限制
    {
        let entry = socket_mgr.get_entry_mut(fd).unwrap();
        entry.rx_buffer_size = 10;
    }

    // 添加数据
    {
        let entry = socket_mgr.get_entry_mut(fd).unwrap();
        assert!(entry.push_rx(vec![1, 2, 3, 4, 5]));
        assert_eq!(entry.rx_buffer_used(), 5);

        // 超过限制应该失败
        assert!(!entry.push_rx(vec![1, 2, 3, 4, 5, 6]));

        // 刚好到限制应该成功
        assert!(entry.push_rx(vec![1, 2, 3, 4, 5]));
        assert_eq!(entry.rx_buffer_used(), 10);
    }
}

// ========== 查找 Socket 测试组 ==========

#[test]
#[serial]
fn test_socket_lookup() {
    let ctx = GlobalStateManager::create_context();
    let mut socket_mgr = ctx.socket_mgr.lock().unwrap();

    let fd = socket_mgr
        .socket(AddressFamily::AF_INET, SocketType::SOCK_DGRAM, SocketProtocol::Default)
        .unwrap();

    let addr = test_addr_v4(8080);
    socket_mgr.bind(fd, &addr).unwrap();

    // 查找 Socket
    let found_fd = socket_mgr.lookup_socket(&addr, None);
    assert_eq!(found_fd, Some(fd));
}

#[test]
#[serial]
fn test_socket_lookup_not_found() {
    let ctx = GlobalStateManager::create_context();
    let mut socket_mgr = ctx.socket_mgr.lock().unwrap();

    let addr = test_addr_v4(8080);

    // 查找不存在的 Socket
    let found_fd = socket_mgr.lookup_socket(&addr, None);
    assert_eq!(found_fd, None);
}

// ========== 清理测试组 ==========

#[test]
#[serial]
fn test_socket_clear() {
    let ctx = GlobalStateManager::create_context();
    let mut socket_mgr = ctx.socket_mgr.lock().unwrap();

    socket_mgr
        .socket(AddressFamily::AF_INET, SocketType::SOCK_STREAM, SocketProtocol::Default)
        .unwrap();
    socket_mgr
        .socket(AddressFamily::AF_INET, SocketType::SOCK_DGRAM, SocketProtocol::Default)
        .unwrap();

    assert_eq!(socket_mgr.socket_count(), 2);

    socket_mgr.clear();
    assert_eq!(socket_mgr.socket_count(), 0);
}
