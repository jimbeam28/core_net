// src/protocols/tcp/socket.rs
//
// TCP Socket API
// 提供应用层 TCP 通信接口

use crate::protocols::Ipv4Addr;
use super::tcb::TcpConnectionId;
use super::TcpError;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

/// TCP 事件类型
#[derive(Debug, Clone)]
pub enum TcpEvent {
    /// 连接已建立
    Connected(TcpConnectionId),
    /// 数据已接收
    DataReceived(TcpConnectionId, Vec<u8>),
    /// 连接已关闭
    Closed(TcpConnectionId),
    /// 连接错误
    Error(TcpConnectionId, String),
}

/// TCP 事件回调
pub type TcpCallback = Box<dyn Fn(TcpEvent) + Send>;

/// TCP Socket
///
/// 提供类似 POSIX 的 TCP socket 接口，支持：
/// - bind() 绑定本地端口
/// - listen() 监听端口
/// - connect() 主动连接
/// - send() 发送数据
/// - recv() 接收数据
/// - close() 关闭 socket
/// - set_callback() 设置事件回调
pub struct TcpSocket {
    /// Socket ID
    id: u64,

    /// 本地地址（IP 和端口）
    local_addr: Option<(Ipv4Addr, u16)>,

    /// 远程地址（IP 和端口）
    remote_addr: Option<(Ipv4Addr, u16)>,

    /// 连接 ID（如果已建立）
    connection_id: Option<TcpConnectionId>,

    /// 事件回调
    callback: Arc<Mutex<Option<TcpCallback>>>,
    /// 回调是否已设置（用于线程安全检查）
    callback_set: AtomicBool,

    /// Socket 是否已关闭
    closed: Arc<AtomicBool>,

    /// 是否为监听 Socket
    is_listening: bool,

    /// 是否为已连接的 Socket
    is_connected: bool,
}

impl TcpSocket {
    /// 创建新的 TCP Socket（内部使用）
    ///
    /// # 参数
    /// - id: Socket ID
    ///
    /// # 返回
    /// - TcpSocket: 新创建的 socket
    pub(crate) fn new(id: u64) -> Self {
        Self {
            id,
            local_addr: None,
            remote_addr: None,
            connection_id: None,
            callback: Arc::new(Mutex::new(None)),
            callback_set: AtomicBool::new(false),
            closed: Arc::new(AtomicBool::new(false)),
            is_listening: false,
            is_connected: false,
        }
    }

    /// 获取 Socket ID
    pub fn id(&self) -> u64 {
        self.id
    }

    /// 检查 Socket 是否已绑定
    pub fn is_bound(&self) -> bool {
        self.local_addr.is_some()
    }

    /// 获取本地地址
    pub fn local_addr(&self) -> Option<(Ipv4Addr, u16)> {
        self.local_addr
    }

    /// 获取远程地址
    pub fn remote_addr(&self) -> Option<(Ipv4Addr, u16)> {
        self.remote_addr
    }

    /// 检查是否为监听 Socket
    pub fn is_listening(&self) -> bool {
        self.is_listening
    }

    /// 检查是否为已连接的 Socket
    pub fn is_connected(&self) -> bool {
        self.is_connected
    }

    /// 检查 Socket 是否已关闭
    pub fn is_closed(&self) -> bool {
        self.closed.load(Ordering::SeqCst)
    }

    /// 获取连接 ID
    pub fn connection_id(&self) -> Option<TcpConnectionId> {
        self.connection_id.clone()
    }

    /// 设置事件回调
    ///
    /// # 参数
    /// - callback: 事件回调函数
    pub fn set_callback(&self, callback: TcpCallback) {
        *self.callback.lock().unwrap() = Some(callback);
        self.callback_set.store(true, Ordering::SeqCst);
    }

    /// 移除事件回调
    pub fn clear_callback(&self) {
        *self.callback.lock().unwrap() = None;
        self.callback_set.store(false, Ordering::SeqCst);
    }

    /// 检查是否有回调
    pub fn has_callback(&self) -> bool {
        self.callback_set.load(Ordering::SeqCst)
    }

    /// 关闭 Socket
    pub fn close(&mut self) -> Result<(), TcpError> {
        if self.closed.load(Ordering::SeqCst) {
            return Ok(()); // 已关闭
        }

        self.closed.store(true, Ordering::SeqCst);
        self.is_listening = false;
        self.is_connected = false;

        Ok(())
    }
}

impl std::fmt::Debug for TcpSocket {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TcpSocket")
            .field("id", &self.id)
            .field("local_addr", &self.local_addr)
            .field("remote_addr", &self.remote_addr)
            .field("connection_id", &self.connection_id)
            .field("callback_set", &self.callback_set.load(Ordering::SeqCst))
            .field("closed", &self.closed.load(Ordering::SeqCst))
            .field("is_listening", &self.is_listening)
            .field("is_connected", &self.is_connected)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_socket_new() {
        let socket = TcpSocket::new(12345);

        assert_eq!(socket.id(), 12345);
        assert!(!socket.is_bound());
        assert!(!socket.is_listening());
        assert!(!socket.is_connected());
        assert!(!socket.is_closed());
        assert!(!socket.has_callback());
        assert!(socket.local_addr().is_none());
        assert!(socket.remote_addr().is_none());
    }

    #[test]
    fn test_socket_callback() {
        let socket = TcpSocket::new(1);

        assert!(!socket.has_callback());

        socket.set_callback(Box::new(|_event| {
            // 回调逻辑
        }));

        assert!(socket.has_callback());
    }

    #[test]
    fn test_socket_clear_callback() {
        let socket = TcpSocket::new(1);

        socket.set_callback(Box::new(|_event| {}));
        assert!(socket.has_callback());

        socket.clear_callback();
        assert!(!socket.has_callback());
    }

    #[test]
    fn test_socket_close() {
        let mut socket = TcpSocket::new(1);

        socket.close().unwrap();

        assert!(socket.is_closed());
    }

    #[test]
    fn test_socket_close_twice() {
        let mut socket = TcpSocket::new(1);

        socket.close().unwrap();
        let result = socket.close();
        assert!(result.is_ok());
    }
}
