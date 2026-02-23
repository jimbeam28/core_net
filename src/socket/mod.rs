// src/socket/mod.rs
//
// Socket API 模块
// 提供类似于 POSIX Socket API 的网络接口

pub mod entry;
pub mod error;
pub mod manager;
pub mod types;

// 重新导出常用类型
pub use error::{Result, SocketError};
pub use entry::{SocketEntry, SocketState, SocketOptions, ListenQueue};
pub use manager::{SocketConfig, SocketManager};
pub use types::{
    AddressFamily, IpAddr, RecvFlags, SendFlags, SocketAddr,
    SocketAddrV4, SocketAddrV6, SocketFd, SocketProtocol, SocketType, TcpState,
};

// 重新导出地址类型
pub use crate::common::addr::{Ipv4Addr, Ipv6Addr};

// Socket API 函数（便利函数，使用全局 SocketManager）

/// 创建一个新的 Socket
///
/// # 参数
/// - `_domain`: 协议族（AfInet/AfInet6）
/// - `_socket_type`: Socket 类型（SockStream/SockDgram）
/// - `_protocol`: 协议编号（通常为 0 表示自动选择）
///
/// # 返回
/// - 成功：返回 SocketFd
/// - 失败：返回 SocketError
///
/// # 注意
///
/// 这是一个便利函数，需要先初始化全局 SocketManager。
/// 推荐直接使用 `SocketManager` 的方法。
pub fn socket(
    _domain: AddressFamily,
    _socket_type: SocketType,
    _protocol: SocketProtocol,
) -> Result<SocketFd> {
    // TODO: 实现全局 SocketManager 支持
    Err(SocketError::Other("Global SocketManager not initialized".to_string()))
}

/// 绑定 Socket 到本地地址
///
/// # 参数
/// - `_fd`: Socket 文件描述符
/// - `_addr`: 本地地址
///
/// # 返回
/// - 成功：Ok(())
/// - 失败：返回 SocketError
pub fn bind(_fd: SocketFd, _addr: &SocketAddr) -> Result<()> {
    // TODO: 实现全局 SocketManager 支持
    Err(SocketError::Other("Global SocketManager not initialized".to_string()))
}

/// 将 Socket 置为监听模式（仅面向连接的 Socket）
///
/// # 参数
/// - `_fd`: Socket 文件描述符
/// - `_backlog`: 挂起连接队列的最大长度
pub fn listen(_fd: SocketFd, _backlog: usize) -> Result<()> {
    // TODO: 实现全局 SocketManager 支持
    Err(SocketError::Other("Global SocketManager not initialized".to_string()))
}

/// 接受一个挂起的连接（仅面向连接的 Socket）
pub fn accept(_fd: SocketFd) -> Result<SocketFd> {
    // TODO: 实现全局 SocketManager 支持
    Err(SocketError::Other("Global SocketManager not initialized".to_string()))
}

/// 发起到对端的连接（仅面向连接的 Socket）
pub fn connect(_fd: SocketFd, _addr: &SocketAddr) -> Result<()> {
    // TODO: 实现全局 SocketManager 支持
    Err(SocketError::Other("Global SocketManager not initialized".to_string()))
}

/// 发送数据（面向连接的 Socket）
pub fn send(_fd: SocketFd, _buf: &[u8], _flags: SendFlags) -> Result<usize> {
    // TODO: 实现全局 SocketManager 支持
    Err(SocketError::Other("Global SocketManager not initialized".to_string()))
}

/// 发送数据（无连接的 Socket）
pub fn sendto(_fd: SocketFd, _buf: &[u8], _flags: SendFlags, _dest_addr: &SocketAddr) -> Result<usize> {
    // TODO: 实现全局 SocketManager 支持
    Err(SocketError::Other("Global SocketManager not initialized".to_string()))
}

/// 接收数据（面向连接的 Socket）
pub fn recv(_fd: SocketFd, _buf: &mut [u8], _flags: RecvFlags) -> Result<usize> {
    // TODO: 实现全局 SocketManager 支持
    Err(SocketError::Other("Global SocketManager not initialized".to_string()))
}

/// 接收数据（无连接的 Socket）
pub fn recvfrom(
    _fd: SocketFd,
    _buf: &mut [u8],
    _flags: RecvFlags,
    _src_addr: &mut Option<SocketAddr>,
) -> Result<usize> {
    // TODO: 实现全局 SocketManager 支持
    Err(SocketError::Other("Global SocketManager not initialized".to_string()))
}

/// 关闭 Socket
pub fn close(_fd: SocketFd) -> Result<()> {
    // TODO: 实现全局 SocketManager 支持
    Err(SocketError::Other("Global SocketManager not initialized".to_string()))
}
