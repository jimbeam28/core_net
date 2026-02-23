// src/socket/manager.rs
//
// Socket 管理器
// 管理所有 Socket 的创建、查找、销毁

use super::entry::{SocketEntry, SocketState, ListenQueue};
use super::error::{Result, SocketError};
use super::types::*;
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};
use crate::protocols::tcp::TcpSocketManager;
use crate::protocols::udp::UdpPortManager;

/// Socket 配置
pub struct SocketConfig {
    /// Socket 表最大容量
    pub max_sockets: usize,

    /// 默认接收缓冲区大小
    pub default_rx_buffer_size: usize,

    /// 默认发送缓冲区大小
    pub default_tx_buffer_size: usize,

    /// 最小接收缓冲区大小
    pub min_rx_buffer_size: usize,

    /// 最小发送缓冲区大小
    pub min_tx_buffer_size: usize,

    /// 最大接收缓冲区大小
    pub max_rx_buffer_size: usize,

    /// 最大发送缓冲区大小
    pub max_tx_buffer_size: usize,

    /// 默认监听队列长度
    pub default_listen_backlog: usize,
}

impl Default for SocketConfig {
    fn default() -> Self {
        Self {
            max_sockets: MAX_SOCKET_TABLE_SIZE,
            default_rx_buffer_size: DEFAULT_SOCKET_BUFFER_SIZE,
            default_tx_buffer_size: DEFAULT_SOCKET_BUFFER_SIZE,
            min_rx_buffer_size: 256,
            min_tx_buffer_size: 256,
            max_rx_buffer_size: 65536,
            max_tx_buffer_size: 65536,
            default_listen_backlog: 128,
        }
    }
}

/// Socket 管理器
///
/// 管理所有 Socket 的创建、查找、销毁
pub struct SocketManager {
    /// 配置
    config: SocketConfig,

    /// 下一个可分配的 fd
    next_fd: u32,

    /// Socket 表项映射
    sockets: HashMap<SocketFd, SocketEntry>,

    /// 已绑定的地址集合（用于检查地址冲突）
    ///
    /// Key: (协议族, 端口)
    /// Value: SocketFd 集合（支持 SO_REUSEADDR）
    bound_addresses: HashMap<(AddressFamily, u16), HashSet<SocketFd>>,

    /// TCP Socket 管理器引用（用于与 TCP 模块交互）
    tcp_socket_mgr: Arc<Mutex<TcpSocketManager>>,

    /// UDP 端口管理器引用（用于与 UDP 模块交互）
    udp_port_mgr: Arc<Mutex<UdpPortManager>>,
}

impl SocketManager {
    /// 创建新的 Socket 管理器
    pub fn new(
        tcp_socket_mgr: Arc<Mutex<TcpSocketManager>>,
        udp_port_mgr: Arc<Mutex<UdpPortManager>>,
    ) -> Self {
        Self {
            config: SocketConfig::default(),
            next_fd: SocketFd::FIRST_AVAILABLE.0,
            sockets: HashMap::new(),
            bound_addresses: HashMap::new(),
            tcp_socket_mgr,
            udp_port_mgr,
        }
    }

    /// 使用指定配置创建 Socket 管理器
    pub fn with_config(
        config: SocketConfig,
        tcp_socket_mgr: Arc<Mutex<TcpSocketManager>>,
        udp_port_mgr: Arc<Mutex<UdpPortManager>>,
    ) -> Self {
        Self {
            config,
            next_fd: SocketFd::FIRST_AVAILABLE.0,
            sockets: HashMap::new(),
            bound_addresses: HashMap::new(),
            tcp_socket_mgr,
            udp_port_mgr,
        }
    }

    /// 分配新的 SocketFd
    fn alloc_fd(&mut self) -> Result<SocketFd> {
        if self.sockets.len() >= self.config.max_sockets {
            return Err(SocketError::TableFull);
        }

        // 跳过保留的文件描述符
        while self.next_fd < 3 {
            self.next_fd = self.next_fd.wrapping_add(1);
        }

        let fd = SocketFd(self.next_fd);
        self.next_fd = self.next_fd.wrapping_add(1);

        // 检查是否重复（理论上不会发生，除非溢出）
        if self.sockets.contains_key(&fd) {
            return Err(SocketError::TableFull);
        }

        Ok(fd)
    }

    /// 创建 Socket
    pub fn socket(
        &mut self,
        domain: AddressFamily,
        socket_type: SocketType,
        protocol: SocketProtocol,
    ) -> Result<SocketFd> {
        // 验证协议族与 Socket 类型的组合，并确定实际协议
        let actual_protocol = match (domain, socket_type, protocol) {
            (AddressFamily::AF_INET, SocketType::SOCK_STREAM, SocketProtocol::Default) => SocketProtocol::TCP,
            (AddressFamily::AF_INET, SocketType::SOCK_STREAM, SocketProtocol::TCP) => SocketProtocol::TCP,
            (AddressFamily::AF_INET, SocketType::SOCK_DGRAM, SocketProtocol::Default) => SocketProtocol::UDP,
            (AddressFamily::AF_INET, SocketType::SOCK_DGRAM, SocketProtocol::UDP) => SocketProtocol::UDP,
            (AddressFamily::AF_INET6, SocketType::SOCK_STREAM, SocketProtocol::Default) => SocketProtocol::TCP,
            (AddressFamily::AF_INET6, SocketType::SOCK_STREAM, SocketProtocol::TCP) => SocketProtocol::TCP,
            (AddressFamily::AF_INET6, SocketType::SOCK_DGRAM, SocketProtocol::Default) => SocketProtocol::UDP,
            (AddressFamily::AF_INET6, SocketType::SOCK_DGRAM, SocketProtocol::UDP) => SocketProtocol::UDP,
            _ => return Err(SocketError::InvalidProtocol),
        };

        // 分配 SocketFd
        let fd = self.alloc_fd()?;

        // 创建 SocketEntry（使用实际协议）
        let entry = SocketEntry::new(fd, domain, socket_type, actual_protocol);
        self.sockets.insert(fd, entry);

        Ok(fd)
    }

    /// 绑定地址
    pub fn bind(&mut self, fd: SocketFd, addr: &SocketAddr) -> Result<()> {
        // 先检查是否已绑定和获取需要的信息
        let (family, socket_type, reuse_addr) = {
            let entry = self.sockets.get(&fd).ok_or(SocketError::InvalidFd)?;
            if entry.is_bound() {
                return Err(SocketError::AlreadyBound);
            }
            (entry.family, entry.socket_type, entry.options.reuse_addr)
        };

        // 验证协议族与地址类型匹配
        match (family, addr) {
            (AddressFamily::AF_INET, SocketAddr::V4(_)) => {}
            (AddressFamily::AF_INET6, SocketAddr::V6(_)) => {}
            _ => return Err(SocketError::InvalidProtocol),
        }

        let port = addr.port();

        // 检查端口冲突（除非设置了 SO_REUSEADDR）
        let key = (family, port);
        if let Some(fds) = self.bound_addresses.get(&key) {
            if !fds.is_empty() && !reuse_addr {
                // 检查是否有其他 Socket 绑定了相同端口
                for &other_fd in fds {
                    if let Some(other_entry) = self.sockets.get(&other_fd) {
                        if !other_entry.options.reuse_addr {
                            return Err(SocketError::AddrInUse);
                        }
                    }
                }
            }
        }

        // 更新 bound_addresses
        self.bound_addresses
            .entry(key)
            .or_insert_with(HashSet::new)
            .insert(fd);

        // 更新 local_addr
        let entry = self.sockets.get_mut(&fd).ok_or(SocketError::InvalidFd)?;
        entry.local_addr = Some(addr.clone());

        Ok(())
    }

    /// 开始监听
    pub fn listen(&mut self, fd: SocketFd, backlog: usize) -> Result<()> {
        // 查找 SocketEntry
        let entry = self.sockets.get_mut(&fd).ok_or(SocketError::InvalidFd)?;

        // 验证 Socket 类型为 SOCK_STREAM
        if entry.socket_type != SocketType::SOCK_STREAM {
            return Err(SocketError::NotStream);
        }

        // 验证 Socket 已绑定
        if !entry.is_bound() {
            return Err(SocketError::NotBound);
        }

        // 创建 ListenQueue
        entry.listen_queue = Some(ListenQueue::new(backlog));

        // 更新状态为 Listen
        entry.state = SocketState::Tcp(TcpState::Listen);

        Ok(())
    }

    /// 接受连接
    pub fn accept(&mut self, fd: SocketFd) -> Result<SocketFd> {
        // 查找 SocketEntry
        let entry = self.sockets.get_mut(&fd).ok_or(SocketError::InvalidFd)?;

        // 验证 Socket 状态为 Listen
        if !entry.is_listening() {
            return Err(SocketError::NotListening);
        }

        // 从监听队列取出连接
        let listen_queue = entry.listen_queue.as_mut().ok_or(SocketError::NotListening)?;
        let conn_fd = listen_queue.take_pending().ok_or(SocketError::WouldBlock)?;

        Ok(conn_fd)
    }

    /// 发起连接
    pub fn connect(&mut self, fd: SocketFd, addr: &SocketAddr) -> Result<()> {
        // 查找 SocketEntry
        let entry = self.sockets.get_mut(&fd).ok_or(SocketError::InvalidFd)?;

        // 验证 Socket 类型为 SOCK_STREAM
        if entry.socket_type != SocketType::SOCK_STREAM {
            return Err(SocketError::NotStream);
        }

        // 验证是否已连接
        if entry.is_connected() {
            return Err(SocketError::AlreadyConnected);
        }

        // 验证协议族与地址类型匹配
        match (entry.family, addr) {
            (AddressFamily::AF_INET, SocketAddr::V4(_)) => {}
            (AddressFamily::AF_INET6, SocketAddr::V6(_)) => {}
            _ => return Err(SocketError::InvalidProtocol),
        }

        // TODO: 触发 TCP 三次握手（与 TCP 模块交互）

        // 更新 peer_addr 和状态
        entry.peer_addr = Some(addr.clone());
        entry.state = SocketState::Tcp(TcpState::SynSent);

        // 阻塞模式下等待连接建立（简化处理，直接设置为 Established）
        if entry.blocking {
            entry.state = SocketState::Tcp(TcpState::Established);
            Ok(())
        } else {
            Err(SocketError::InProgress)
        }
    }

    /// 发送数据（面向连接）
    pub fn send(&mut self, fd: SocketFd, buf: &[u8], _flags: SendFlags) -> Result<usize> {
        // 查找 SocketEntry
        let entry = self.sockets.get_mut(&fd).ok_or(SocketError::InvalidFd)?;

        // 验证 Socket 状态为 Established
        if !entry.is_connected() {
            return Err(SocketError::NotConnected);
        }

        // 检查发送缓冲区空间
        if !entry.has_tx_space() {
            return Err(if entry.blocking {
                SocketError::NoBufferSpace
            } else {
                SocketError::WouldBlock
            });
        }

        // 将数据加入发送缓冲区
        let data = buf.to_vec();
        let len = data.len();
        if !entry.push_tx(data) {
            return Err(SocketError::NoBufferSpace);
        }

        // TODO: 触发 TCP 层发送

        Ok(len)
    }

    /// 发送数据（无连接）
    pub fn sendto(
        &mut self,
        fd: SocketFd,
        buf: &[u8],
        _flags: SendFlags,
        dest_addr: &SocketAddr,
    ) -> Result<usize> {
        // 查找 SocketEntry
        let entry = self.sockets.get_mut(&fd).ok_or(SocketError::InvalidFd)?;

        // 验证 Socket 已绑定
        if !entry.is_bound() {
            return Err(SocketError::NotBound);
        }

        // 检查发送缓冲区空间
        if !entry.has_tx_space() {
            return Err(if entry.blocking {
                SocketError::NoBufferSpace
            } else {
                SocketError::WouldBlock
            });
        }

        // 将数据加入发送缓冲区
        let data = buf.to_vec();
        let len = data.len();
        if !entry.push_tx(data) {
            return Err(SocketError::NoBufferSpace);
        }

        // TODO: 触发 UDP 层封装和发送

        Ok(len)
    }

    /// 接收数据（面向连接）
    pub fn recv(&mut self, fd: SocketFd, buf: &mut [u8], _flags: RecvFlags) -> Result<usize> {
        // 查找 SocketEntry
        let entry = self.sockets.get_mut(&fd).ok_or(SocketError::InvalidFd)?;

        // 验证 Socket 状态
        if entry.socket_type == SocketType::SOCK_STREAM && !entry.is_connected() {
            return Err(SocketError::NotConnected);
        }

        // 从接收缓冲区取出数据
        let data = entry.pop_rx().ok_or(if entry.blocking {
            SocketError::Other("No data available".to_string())
        } else {
            SocketError::WouldBlock
        })?;

        // 复制数据到用户缓冲区
        let len = buf.len().min(data.len());
        buf[..len].copy_from_slice(&data[..len]);

        Ok(len)
    }

    /// 接收数据（无连接）
    pub fn recvfrom(
        &mut self,
        fd: SocketFd,
        buf: &mut [u8],
        _flags: RecvFlags,
        src_addr: &mut Option<SocketAddr>,
    ) -> Result<usize> {
        // 查找 SocketEntry
        let entry = self.sockets.get_mut(&fd).ok_or(SocketError::InvalidFd)?;

        // 从接收缓冲区取出数据
        let data = entry.pop_rx().ok_or(if entry.blocking {
            SocketError::Other("No data available".to_string())
        } else {
            SocketError::WouldBlock
        })?;

        // TODO: 设置源地址
        // src_addr 暂时保持为 None，因为缓冲区中没有存储源地址

        // 复制数据到用户缓冲区
        let len = buf.len().min(data.len());
        buf[..len].copy_from_slice(&data[..len]);

        Ok(len)
    }

    /// 关闭 Socket
    pub fn close(&mut self, fd: SocketFd) -> Result<()> {
        // 查找 SocketEntry
        let entry = self.sockets.remove(&fd).ok_or(SocketError::InvalidFd)?;

        // 从 bound_addresses 移除
        if let Some(local_addr) = &entry.local_addr {
            let key = (entry.family, local_addr.port());
            if let Some(fds) = self.bound_addresses.get_mut(&key) {
                fds.remove(&fd);
                if fds.is_empty() {
                    self.bound_addresses.remove(&key);
                }
            }
        }

        // TODO: TCP 关闭流程
        if matches!(entry.state, SocketState::Tcp(TcpState::Established | TcpState::Listen)) {
            // 发送 FIN
        }

        Ok(())
    }

    /// 根据 SocketFd 获取 SocketEntry（内部使用）
    pub fn get_entry(&self, fd: SocketFd) -> Option<&SocketEntry> {
        self.sockets.get(&fd)
    }

    /// 根据 SocketFd 获取可变 SocketEntry（内部使用）
    pub fn get_entry_mut(&mut self, fd: SocketFd) -> Option<&mut SocketEntry> {
        self.sockets.get_mut(&fd)
    }

    /// 根据 IP 和端口查找 Socket（用于接收到数据时分发）
    pub fn lookup_socket(
        &self,
        local_addr: &SocketAddr,
        _peer_addr: Option<&SocketAddr>,
    ) -> Option<SocketFd> {
        let key = (match local_addr {
            SocketAddr::V4(_) => AddressFamily::AF_INET,
            SocketAddr::V6(_) => AddressFamily::AF_INET6,
        }, local_addr.port());

        // 查找绑定到该地址的 Socket
        if let Some(fds) = self.bound_addresses.get(&key) {
            // 返回第一个匹配的 Socket（简化处理）
            for fd in fds {
                if let Some(entry) = self.sockets.get(fd) {
                    if entry.local_addr.as_ref() == Some(local_addr) {
                        return Some(*fd);
                    }
                }
            }
        }

        None
    }

    /// 获取 Socket 数量
    pub fn socket_count(&self) -> usize {
        self.sockets.len()
    }

    /// 清空所有 Socket
    pub fn clear(&mut self) {
        self.sockets.clear();
        self.bound_addresses.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};
    use crate::common::addr::Ipv4Addr;

    fn create_test_manager() -> SocketManager {
        let tcp_mgr = Arc::new(Mutex::new(TcpSocketManager::new()));
        let udp_mgr = Arc::new(Mutex::new(UdpPortManager::new()));
        SocketManager::new(tcp_mgr, udp_mgr)
    }

    #[test]
    fn test_socket_manager_new() {
        let mgr = create_test_manager();
        assert_eq!(mgr.socket_count(), 0);
    }

    #[test]
    fn test_socket_create_tcp() {
        let mut mgr = create_test_manager();

        let fd = mgr.socket(AddressFamily::AF_INET, SocketType::SOCK_STREAM, SocketProtocol::Default).unwrap();
        assert_eq!(fd.0, SocketFd::FIRST_AVAILABLE.0);
        assert_eq!(mgr.socket_count(), 1);

        // 验证 Socket 属性
        let entry = mgr.get_entry(fd).unwrap();
        assert_eq!(entry.family, AddressFamily::AF_INET);
        assert_eq!(entry.socket_type, SocketType::SOCK_STREAM);
        assert!(matches!(entry.state, SocketState::Tcp(TcpState::Closed)));
    }

    #[test]
    fn test_socket_create_udp() {
        let mut mgr = create_test_manager();

        let fd = mgr.socket(AddressFamily::AF_INET, SocketType::SOCK_DGRAM, SocketProtocol::Default).unwrap();
        assert_eq!(mgr.socket_count(), 1);

        let entry = mgr.get_entry(fd).unwrap();
        assert_eq!(entry.socket_type, SocketType::SOCK_DGRAM);
        assert!(matches!(entry.state, SocketState::Udp));
    }

    #[test]
    fn test_socket_bind() {
        let mut mgr = create_test_manager();

        let fd = mgr.socket(AddressFamily::AF_INET, SocketType::SOCK_DGRAM, SocketProtocol::Default).unwrap();
        let addr = SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), 8080));

        mgr.bind(fd, &addr).unwrap();

        let entry = mgr.get_entry(fd).unwrap();
        assert!(entry.is_bound());
        assert_eq!(entry.local_addr, Some(addr));
    }

    #[test]
    fn test_socket_bind_already_bound() {
        let mut mgr = create_test_manager();

        let fd = mgr.socket(AddressFamily::AF_INET, SocketType::SOCK_DGRAM, SocketProtocol::Default).unwrap();
        let addr = SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), 8080));

        mgr.bind(fd, &addr).unwrap();
        let result = mgr.bind(fd, &addr);
        assert!(matches!(result, Err(SocketError::AlreadyBound)));
    }

    #[test]
    fn test_socket_bind_addr_in_use() {
        let mut mgr = create_test_manager();

        let fd1 = mgr.socket(AddressFamily::AF_INET, SocketType::SOCK_DGRAM, SocketProtocol::Default).unwrap();
        let fd2 = mgr.socket(AddressFamily::AF_INET, SocketType::SOCK_DGRAM, SocketProtocol::Default).unwrap();
        let addr = SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), 8080));

        mgr.bind(fd1, &addr).unwrap();
        let result = mgr.bind(fd2, &addr);
        assert!(matches!(result, Err(SocketError::AddrInUse)));
    }

    #[test]
    fn test_socket_listen() {
        let mut mgr = create_test_manager();

        let fd = mgr.socket(AddressFamily::AF_INET, SocketType::SOCK_STREAM, SocketProtocol::Default).unwrap();
        let addr = SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), 8080));

        mgr.bind(fd, &addr).unwrap();
        mgr.listen(fd, 128).unwrap();

        let entry = mgr.get_entry(fd).unwrap();
        assert!(entry.is_listening());
        assert!(entry.listen_queue.is_some());
    }

    #[test]
    fn test_socket_listen_not_bound() {
        let mut mgr = create_test_manager();

        let fd = mgr.socket(AddressFamily::AF_INET, SocketType::SOCK_STREAM, SocketProtocol::Default).unwrap();
        let result = mgr.listen(fd, 128);
        assert!(matches!(result, Err(SocketError::NotBound)));
    }

    #[test]
    fn test_socket_send_recv() {
        let mut mgr = create_test_manager();

        let fd = mgr.socket(AddressFamily::AF_INET, SocketType::SOCK_STREAM, SocketProtocol::Default).unwrap();

        // 手动设置状态为 Established（模拟已连接）
        let entry = mgr.get_entry_mut(fd).unwrap();
        entry.state = SocketState::Tcp(TcpState::Established);

        // 发送数据
        let data = b"Hello";
        mgr.send(fd, data, SendFlags::NONE).unwrap();

        // 手动将数据移到接收缓冲区（模拟网络传输）
        let entry = mgr.get_entry_mut(fd).unwrap();
        let tx_data = entry.pop_tx().unwrap();
        entry.push_rx(tx_data);

        // 接收数据
        let mut buf = [0u8; 64];
        let len = mgr.recv(fd, &mut buf, RecvFlags::NONE).unwrap();
        assert_eq!(len, 5);
        assert_eq!(&buf[..5], b"Hello");
    }

    #[test]
    fn test_socket_close() {
        let mut mgr = create_test_manager();

        let fd = mgr.socket(AddressFamily::AF_INET, SocketType::SOCK_DGRAM, SocketProtocol::Default).unwrap();
        assert_eq!(mgr.socket_count(), 1);

        mgr.close(fd).unwrap();
        assert_eq!(mgr.socket_count(), 0);
        assert!(mgr.get_entry(fd).is_none());
    }

    #[test]
    fn test_socket_close_invalid_fd() {
        let mut mgr = create_test_manager();
        let result = mgr.close(SocketFd::INVALID);
        assert!(matches!(result, Err(SocketError::InvalidFd)));
    }

    #[test]
    fn test_clear() {
        let mut mgr = create_test_manager();

        mgr.socket(AddressFamily::AF_INET, SocketType::SOCK_STREAM, SocketProtocol::Default).unwrap();
        mgr.socket(AddressFamily::AF_INET, SocketType::SOCK_DGRAM, SocketProtocol::Default).unwrap();
        assert_eq!(mgr.socket_count(), 2);

        mgr.clear();
        assert_eq!(mgr.socket_count(), 0);
    }
}
