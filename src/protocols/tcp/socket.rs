// src/protocols/tcp/socket.rs
//
// TCP Socket API
// 提供应用层 TCP 通信接口

use crate::protocols::Ipv4Addr;
use super::tcb::TcpConnectionId;
use super::error::TcpError;
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

    /// 设置本地地址
    pub(crate) fn set_local_addr(&mut self, ip: Ipv4Addr, port: u16) {
        self.local_addr = Some((ip, port));
    }

    /// 设置远程地址
    pub(crate) fn set_remote_addr(&mut self, ip: Ipv4Addr, port: u16) {
        self.remote_addr = Some((ip, port));
    }

    /// 设置连接 ID
    pub(crate) fn set_connection_id(&mut self, id: TcpConnectionId) {
        self.connection_id = Some(id);
    }

    /// 设置为监听状态
    pub(crate) fn set_listening(&mut self, listening: bool) {
        self.is_listening = listening;
    }

    /// 设置为已连接状态
    pub(crate) fn set_connected(&mut self, connected: bool) {
        self.is_connected = connected;
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

    /// 触发事件回调
    pub(crate) fn trigger_event(&self, event: TcpEvent) {
        if self.callback_set.load(Ordering::SeqCst) {
            if let Some(callback) = self.callback.lock().unwrap().as_ref() {
                callback(event);
            }
        }
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
    fn test_socket_set_local_addr() {
        let mut socket = TcpSocket::new(1);

        socket.set_local_addr(Ipv4Addr::new(192, 168, 1, 100), 8080);

        assert!(socket.is_bound());
        assert_eq!(socket.local_addr(), Some((Ipv4Addr::new(192, 168, 1, 100), 8080)));
    }

    #[test]
    fn test_socket_set_remote_addr() {
        let mut socket = TcpSocket::new(1);

        socket.set_remote_addr(Ipv4Addr::new(192, 168, 1, 1), 80);

        assert_eq!(socket.remote_addr(), Some((Ipv4Addr::new(192, 168, 1, 1), 80)));
    }

    #[test]
    fn test_socket_set_connection_id() {
        let mut socket = TcpSocket::new(1);

        let conn_id = TcpConnectionId::new(
            Ipv4Addr::new(192, 168, 1, 100), 8080,
            Ipv4Addr::new(192, 168, 1, 1), 80,
        );
        socket.set_connection_id(conn_id.clone());

        assert_eq!(socket.connection_id(), Some(conn_id));
    }

    #[test]
    fn test_socket_set_listening() {
        let mut socket = TcpSocket::new(1);

        socket.set_listening(true);

        assert!(socket.is_listening());
    }

    #[test]
    fn test_socket_set_connected() {
        let mut socket = TcpSocket::new(1);

        socket.set_connected(true);

        assert!(socket.is_connected());
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

        socket.set_listening(true);
        socket.set_connected(true);

        socket.close().unwrap();

        assert!(socket.is_closed());
        assert!(!socket.is_listening());
        assert!(!socket.is_connected());
    }

    #[test]
    fn test_socket_close_twice() {
        let mut socket = TcpSocket::new(1);

        socket.close().unwrap();
        let result = socket.close();
        assert!(result.is_ok());
    }

    #[test]
    fn test_socket_trigger_event() {
        let socket = TcpSocket::new(1);

        // 使用 Arc<Mutex<Cell>> 来跟踪回调是否被调用
        use std::sync::atomic::{AtomicBool, Ordering};
        let called = Arc::new(AtomicBool::new(false));
        let called_clone = called.clone();

        socket.set_callback(Box::new(move |_event| {
            called_clone.store(true, Ordering::SeqCst);
        }));

        let conn_id = TcpConnectionId::new(
            Ipv4Addr::new(192, 168, 1, 100), 8080,
            Ipv4Addr::new(192, 168, 1, 1), 80,
        );
        socket.trigger_event(TcpEvent::Connected(conn_id.clone()));

        assert!(called.load(Ordering::SeqCst));
    }
}
