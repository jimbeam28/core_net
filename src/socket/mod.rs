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
/// - `domain`: 协议族（AF_INET/AF_INET6）
/// - `type`: Socket 类型（SOCK_STREAM/SOCK_DGRAM）
/// - `protocol`: 协议编号（通常为 0 表示自动选择）
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
    domain: AddressFamily,
    socket_type: SocketType,
    protocol: SocketProtocol,
) -> Result<SocketFd> {
    // TODO: 实现全局 SocketManager 支持
    Err(SocketError::Other("Global SocketManager not initialized".to_string()))
}

/// 绑定 Socket 到本地地址
///
/// # 参数
/// - `fd`: Socket 文件描述符
/// - `addr`: 本地地址
///
/// # 返回
/// - 成功：Ok(())
/// - 失败：返回 SocketError
pub fn bind(fd: SocketFd, addr: &SocketAddr) -> Result<()> {
    // TODO: 实现全局 SocketManager 支持
    Err(SocketError::Other("Global SocketManager not initialized".to_string()))
}

/// 将 Socket 置为监听模式（仅面向连接的 Socket）
///
/// # 参数
/// - `fd`: Socket 文件描述符
/// - `backlog`: 挂起连接队列的最大长度
pub fn listen(fd: SocketFd, backlog: usize) -> Result<()> {
    // TODO: 实现全局 SocketManager 支持
    Err(SocketError::Other("Global SocketManager not initialized".to_string()))
}

/// 接受一个挂起的连接（仅面向连接的 Socket）
pub fn accept(fd: SocketFd) -> Result<SocketFd> {
    // TODO: 实现全局 SocketManager 支持
    Err(SocketError::Other("Global SocketManager not initialized".to_string()))
}

/// 发起到对端的连接（仅面向连接的 Socket）
pub fn connect(fd: SocketFd, addr: &SocketAddr) -> Result<()> {
    // TODO: 实现全局 SocketManager 支持
    Err(SocketError::Other("Global SocketManager not initialized".to_string()))
}

/// 发送数据（面向连接的 Socket）
pub fn send(fd: SocketFd, buf: &[u8], flags: SendFlags) -> Result<usize> {
    // TODO: 实现全局 SocketManager 支持
    Err(SocketError::Other("Global SocketManager not initialized".to_string()))
}

/// 发送数据（无连接的 Socket）
pub fn sendto(fd: SocketFd, buf: &[u8], flags: SendFlags, dest_addr: &SocketAddr) -> Result<usize> {
    // TODO: 实现全局 SocketManager 支持
    Err(SocketError::Other("Global SocketManager not initialized".to_string()))
}

/// 接收数据（面向连接的 Socket）
pub fn recv(fd: SocketFd, buf: &mut [u8], flags: RecvFlags) -> Result<usize> {
    // TODO: 实现全局 SocketManager 支持
    Err(SocketError::Other("Global SocketManager not initialized".to_string()))
}

/// 接收数据（无连接的 Socket）
pub fn recvfrom(
    fd: SocketFd,
    buf: &mut [u8],
    flags: RecvFlags,
    src_addr: &mut Option<SocketAddr>,
) -> Result<usize> {
    // TODO: 实现全局 SocketManager 支持
    Err(SocketError::Other("Global SocketManager not initialized".to_string()))
}

/// 关闭 Socket
pub fn close(fd: SocketFd) -> Result<()> {
    // TODO: 实现全局 SocketManager 支持
    Err(SocketError::Other("Global SocketManager not initialized".to_string()))
}
