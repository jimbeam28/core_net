// src/socket/entry.rs
//
// Socket 表项和相关结构定义

use super::types::*;
use std::collections::{HashSet, VecDeque};

/// Socket 选项
#[derive(Debug, Clone)]
pub struct SocketOptions {
    /// SO_REUSEADDR
    pub reuse_addr: bool,
    /// SO_REUSEPORT
    pub reuse_port: bool,
    /// SO_BROADCAST
    pub broadcast: bool,
    /// SO_KEEPALIVE
    pub keepalive: bool,
    /// SO_RCVBUF
    pub rcvbuf: usize,
    /// SO_SNDBUF
    pub sndbuf: usize,
}

impl Default for SocketOptions {
    fn default() -> Self {
        Self {
            reuse_addr: false,
            reuse_port: false,
            broadcast: false,
            keepalive: false,
            rcvbuf: DEFAULT_SOCKET_BUFFER_SIZE,
            sndbuf: DEFAULT_SOCKET_BUFFER_SIZE,
        }
    }
}

/// Socket 状态（内部表示）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SocketState {
    /// TCP 连接状态
    Tcp(TcpState),
    /// UDP 无状态
    Udp,
}

/// 监听队列（用于 TCP 服务端）
pub struct ListenQueue {
    /// 最大挂起连接数
    pub backlog: usize,

    /// 已完成三次握手、等待 accept 的连接队列
    ///
    /// 每个 SocketFd 代表一个已建立的连接
    pub pending_connections: VecDeque<SocketFd>,

    /// 正在完成三次握手的连接队列（SYN RCVD 状态）
    pub half_open: HashSet<SocketFd>,
}

impl ListenQueue {
    /// 创建新的监听队列
    pub fn new(backlog: usize) -> Self {
        Self {
            backlog,
            pending_connections: VecDeque::new(),
            half_open: HashSet::new(),
        }
    }

    /// 检查是否已满
    pub fn is_full(&self) -> bool {
        self.pending_connections.len() >= self.backlog
    }

    /// 添加挂起连接
    pub fn add_pending(&mut self, fd: SocketFd) -> bool {
        if self.is_full() {
            return false;
        }
        self.pending_connections.push_back(fd);
        true
    }

    /// 取出挂起连接
    pub fn take_pending(&mut self) -> Option<SocketFd> {
        self.pending_connections.pop_front()
    }

    /// 获取挂起连接数量
    pub fn pending_count(&self) -> usize {
        self.pending_connections.len()
    }
}

/// Socket 表项
///
/// 每个 Socket 对应一个表项，包含其完整的状态信息
pub struct SocketEntry {
    /// Socket 文件描述符
    pub fd: SocketFd,

    /// 协议族
    pub family: AddressFamily,

    /// Socket 类型
    pub socket_type: SocketType,

    /// 协议
    pub protocol: SocketProtocol,

    /// Socket 状态（仅 TCP 有效）
    pub state: SocketState,

    /// 绑定的本地地址
    pub local_addr: Option<SocketAddr>,

    /// 连接的对端地址（仅面向连接的 Socket）
    pub peer_addr: Option<SocketAddr>,

    /// 接收缓冲区
    pub rx_buffer: VecDeque<Vec<u8>>,

    /// 发送缓冲区
    pub tx_buffer: VecDeque<Vec<u8>>,

    /// 接收缓冲区大小限制（字节）
    pub rx_buffer_size: usize,

    /// 发送缓冲区大小限制（字节）
    pub tx_buffer_size: usize,

    /// Socket 选项
    pub options: SocketOptions,

    /// 是否阻塞模式
    pub blocking: bool,

    /// 监听队列（仅 SOCK_STREAM 且状态为 Listen 时有效）
    pub listen_queue: Option<ListenQueue>,
}

impl SocketEntry {
    /// 创建新的 Socket 表项
    pub fn new(fd: SocketFd, family: AddressFamily, socket_type: SocketType, protocol: SocketProtocol) -> Self {
        let state = match socket_type {
            SocketType::SOCK_STREAM => SocketState::Tcp(TcpState::Closed),
            SocketType::SOCK_DGRAM => SocketState::Udp,
        };

        Self {
            fd,
            family,
            socket_type,
            protocol,
            state,
            local_addr: None,
            peer_addr: None,
            rx_buffer: VecDeque::new(),
            tx_buffer: VecDeque::new(),
            rx_buffer_size: DEFAULT_SOCKET_BUFFER_SIZE,
            tx_buffer_size: DEFAULT_SOCKET_BUFFER_SIZE,
            options: SocketOptions::default(),
            blocking: true,
            listen_queue: None,
        }
    }

    /// 检查是否已绑定
    pub fn is_bound(&self) -> bool {
        self.local_addr.is_some()
    }

    /// 检查是否已连接（仅 TCP）
    pub fn is_connected(&self) -> bool {
        matches!(self.state, SocketState::Tcp(TcpState::Established))
    }

    /// 检查是否正在监听（仅 TCP）
    pub fn is_listening(&self) -> bool {
        matches!(self.state, SocketState::Tcp(TcpState::Listen))
    }

    /// 获取接收缓冲区当前大小（字节）
    pub fn rx_buffer_used(&self) -> usize {
        self.rx_buffer.iter().map(|data| data.len()).sum()
    }

    /// 获取发送缓冲区当前大小（字节）
    pub fn tx_buffer_used(&self) -> usize {
        self.tx_buffer.iter().map(|data| data.len()).sum()
    }

    /// 检查接收缓冲区是否有空间
    pub fn has_rx_space(&self) -> bool {
        self.rx_buffer_used() < self.rx_buffer_size
    }

    /// 检查发送缓冲区是否有空间
    pub fn has_tx_space(&self) -> bool {
        self.tx_buffer_used() < self.tx_buffer_size
    }

    /// 添加数据到接收缓冲区
    pub fn push_rx(&mut self, data: Vec<u8>) -> bool {
        if self.rx_buffer_used() + data.len() > self.rx_buffer_size {
            return false;
        }
        self.rx_buffer.push_back(data);
        true
    }

    /// 添加数据到发送缓冲区
    pub fn push_tx(&mut self, data: Vec<u8>) -> bool {
        if self.tx_buffer_used() + data.len() > self.tx_buffer_size {
            return false;
        }
        self.tx_buffer.push_back(data);
        true
    }

    /// 从接收缓冲区取出数据
    pub fn pop_rx(&mut self) -> Option<Vec<u8>> {
        self.rx_buffer.pop_front()
    }

    /// 从发送缓冲区取出数据
    pub fn pop_tx(&mut self) -> Option<Vec<u8>> {
        self.tx_buffer.pop_front()
    }

    /// 清空接收缓冲区
    pub fn clear_rx(&mut self) {
        self.rx_buffer.clear();
    }

    /// 清空发送缓冲区
    pub fn clear_tx(&mut self) {
        self.tx_buffer.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_socket_entry_new() {
        let fd = SocketFd::FIRST_AVAILABLE;
        let entry = SocketEntry::new(
            fd,
            AddressFamily::AF_INET,
            SocketType::SOCK_STREAM,
            SocketProtocol::TCP,
        );

        assert_eq!(entry.fd, fd);
        assert_eq!(entry.family, AddressFamily::AF_INET);
        assert_eq!(entry.socket_type, SocketType::SOCK_STREAM);
        assert_eq!(entry.protocol, SocketProtocol::TCP);
        assert!(matches!(entry.state, SocketState::Tcp(TcpState::Closed)));
        assert!(!entry.is_bound());
        assert!(!entry.is_connected());
        assert!(entry.blocking);
    }

    #[test]
    fn test_socket_entry_udp() {
        let entry = SocketEntry::new(
            SocketFd::FIRST_AVAILABLE,
            AddressFamily::AF_INET,
            SocketType::SOCK_DGRAM,
            SocketProtocol::UDP,
        );

        assert!(matches!(entry.state, SocketState::Udp));
        assert!(!entry.is_bound());
    }

    #[test]
    fn test_socket_entry_buffer_operations() {
        let mut entry = SocketEntry::new(
            SocketFd::FIRST_AVAILABLE,
            AddressFamily::AF_INET,
            SocketType::SOCK_DGRAM,
            SocketProtocol::UDP,
        );

        // 添加数据
        assert!(entry.push_rx(vec![1, 2, 3]));
        assert!(entry.push_rx(vec![4, 5]));
        assert_eq!(entry.rx_buffer_used(), 5);

        // 取出数据
        let data = entry.pop_rx();
        assert_eq!(data, Some(vec![1, 2, 3]));
        assert_eq!(entry.rx_buffer_used(), 2);

        // 清空
        entry.clear_rx();
        assert_eq!(entry.rx_buffer_used(), 0);
    }

    #[test]
    fn test_socket_entry_buffer_limit() {
        let mut entry = SocketEntry::new(
            SocketFd::FIRST_AVAILABLE,
            AddressFamily::AF_INET,
            SocketType::SOCK_DGRAM,
            SocketProtocol::UDP,
        );
        entry.rx_buffer_size = 10;

        // 添加 5 字节
        assert!(entry.push_rx(vec![1, 2, 3, 4, 5]));
        assert_eq!(entry.rx_buffer_used(), 5);

        // 添加 6 字节应该失败（总共 11 字节超过限制）
        assert!(!entry.push_rx(vec![1, 2, 3, 4, 5, 6]));

        // 添加 5 字节应该成功（总共 10 字节）
        assert!(entry.push_rx(vec![1, 2, 3, 4, 5]));
        assert_eq!(entry.rx_buffer_used(), 10);
    }

    #[test]
    fn test_listen_queue() {
        let mut queue = ListenQueue::new(2);

        assert!(!queue.is_full());
        assert_eq!(queue.pending_count(), 0);

        // 添加连接
        assert!(queue.add_pending(SocketFd(3)));
        assert_eq!(queue.pending_count(), 1);

        assert!(queue.add_pending(SocketFd(4)));
        assert_eq!(queue.pending_count(), 2);

        // 已满
        assert!(queue.is_full());
        assert!(!queue.add_pending(SocketFd(5)));

        // 取出连接
        let fd = queue.take_pending();
        assert_eq!(fd, Some(SocketFd(3)));
        assert_eq!(queue.pending_count(), 1);

        // 不再满
        assert!(!queue.is_full());
    }
}
