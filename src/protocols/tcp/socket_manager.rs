// src/protocols/tcp/socket_manager.rs
//
// TCP Socket 管理器
// 管理 TCP Socket 的生命周期

use super::socket::TcpSocket;
use crate::protocols::{Ipv4Addr, Ipv6Addr};
use crate::common::addr::IpAddr;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// TCP 连接四元组
///
/// 用于唯一标识一个 TCP 连接：
/// - 源 IP 地址和端口
/// - 目的 IP 地址和端口
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ConnectionTuple {
    /// 源 IP 地址
    pub src_ip: IpAddr,
    /// 源端口号
    pub src_port: u16,
    /// 目的 IP 地址
    pub dst_ip: IpAddr,
    /// 目的端口号
    pub dst_port: u16,
}

impl ConnectionTuple {
    /// 创建新的连接四元组
    pub fn new(src_ip: IpAddr, src_port: u16, dst_ip: IpAddr, dst_port: u16) -> Self {
        Self {
            src_ip,
            src_port,
            dst_ip,
            dst_port,
        }
    }

    /// 创建 IPv4 连接四元组
    pub fn new_v4(src_ip: Ipv4Addr, src_port: u16, dst_ip: Ipv4Addr, dst_port: u16) -> Self {
        Self {
            src_ip: IpAddr::V4(src_ip),
            src_port,
            dst_ip: IpAddr::V4(dst_ip),
            dst_port,
        }
    }

    /// 创建 IPv6 连接四元组
    pub fn new_v6(src_ip: Ipv6Addr, src_port: u16, dst_ip: Ipv6Addr, dst_port: u16) -> Self {
        Self {
            src_ip: IpAddr::V6(src_ip),
            src_port,
            dst_ip: IpAddr::V6(dst_ip),
            dst_port,
        }
    }

    /// 反转四元组（用于响应方向）
    pub fn reverse(&self) -> Self {
        Self {
            src_ip: self.dst_ip,
            src_port: self.dst_port,
            dst_ip: self.src_ip,
            dst_port: self.src_port,
        }
    }
}

/// TCP Socket 管理器
///
/// 管理所有 TCP Socket，提供 Socket 创建、查找和删除功能。
///
/// 使用两种索引方式：
/// 1. Socket ID -> Socket（用于应用层查找）
/// 2. 连接四元组 -> Socket ID（用于网络层路由）
#[derive(Debug)]
pub struct TcpSocketManager {
    /// Socket 表（socket_id -> TcpSocket）
    sockets: HashMap<u64, Arc<Mutex<TcpSocket>>>,

    /// 连接表（四元组 -> socket_id）
    connections: HashMap<ConnectionTuple, u64>,

    /// 下一个 Socket ID
    next_socket_id: u64,
}

impl TcpSocketManager {
    /// 创建新的 Socket 管理器
    pub fn new() -> Self {
        Self {
            sockets: HashMap::new(),
            connections: HashMap::new(),
            next_socket_id: 1,
        }
    }

    /// 创建新的 Socket
    ///
    /// # 返回
    /// - Arc<Mutex<TcpSocket>>: 新创建的 Socket
    pub fn create_socket(&mut self) -> Arc<Mutex<TcpSocket>> {
        let socket_id = self.next_socket_id;
        self.next_socket_id = self.next_socket_id.wrapping_add(1);

        let socket = TcpSocket::new(socket_id);
        let socket_arc = Arc::new(Mutex::new(socket));
        self.sockets.insert(socket_id, socket_arc.clone());

        socket_arc
    }

    /// 绑定连接四元组到 Socket
    ///
    /// 当 TCP 连接建立后（SYN-RCVD 或 ESTABLISHED），调用此方法将四元组绑定到 Socket。
    ///
    /// # 参数
    /// - tuple: 连接四元组
    /// - socket_id: Socket ID
    ///
    /// # 返回
    /// - Ok(()): 绑定成功
    /// - Err(String): 绑定失败（如 Socket ID 不存在）
    pub fn bind_connection(&mut self, tuple: ConnectionTuple, socket_id: u64) -> Result<(), String> {
        // 检查 Socket 是否存在
        if !self.sockets.contains_key(&socket_id) {
            return Err(format!("Socket ID {} 不存在", socket_id));
        }

        // 插入连接映射
        self.connections.insert(tuple, socket_id);
        Ok(())
    }

    /// 查找 Socket（通过 Socket ID）
    ///
    /// # 参数
    /// - socket_id: Socket ID
    ///
    /// # 返回
    /// - Option<Arc<Mutex<TcpSocket>>>: Socket（如果存在）
    pub fn find(&self, socket_id: u64) -> Option<Arc<Mutex<TcpSocket>>> {
        self.sockets.get(&socket_id).cloned()
    }

    /// 查找 Socket（通过连接四元组）
    ///
    /// 根据传入的 IP 地址和端口号查找对应的 Socket。
    /// 尝试正向匹配，如果失败则尝试反向匹配（用于响应报文）。
    ///
    /// # 参数
    /// - src_ip: 源 IP 地址
    /// - src_port: 源端口号
    /// - dst_ip: 目的 IP 地址
    /// - dst_port: 目的端口号
    ///
    /// # 返回
    /// - Option<Arc<Mutex<TcpSocket>>>: Socket（如果存在）
    pub fn find_by_connection(
        &self,
        src_ip: IpAddr,
        src_port: u16,
        dst_ip: IpAddr,
        dst_port: u16,
    ) -> Option<Arc<Mutex<TcpSocket>>> {
        // 首先尝试正向匹配
        let tuple = ConnectionTuple::new(src_ip, src_port, dst_ip, dst_port);
        if let Some(&socket_id) = self.connections.get(&tuple) {
            return self.sockets.get(&socket_id).cloned();
        }

        // 尝试反向匹配（响应报文可能方向相反）
        let reverse_tuple = tuple.reverse();
        if let Some(&socket_id) = self.connections.get(&reverse_tuple) {
            return self.sockets.get(&socket_id).cloned();
        }

        None
    }

    /// 查找 Socket（通过 IPv4 连接四元组）
    pub fn find_by_connection_v4(
        &self,
        src_ip: Ipv4Addr,
        src_port: u16,
        dst_ip: Ipv4Addr,
        dst_port: u16,
    ) -> Option<Arc<Mutex<TcpSocket>>> {
        self.find_by_connection(
            IpAddr::V4(src_ip),
            src_port,
            IpAddr::V4(dst_ip),
            dst_port,
        )
    }

    /// 查找 Socket（通过 IPv6 连接四元组）
    pub fn find_by_connection_v6(
        &self,
        src_ip: Ipv6Addr,
        src_port: u16,
        dst_ip: Ipv6Addr,
        dst_port: u16,
    ) -> Option<Arc<Mutex<TcpSocket>>> {
        self.find_by_connection(
            IpAddr::V6(src_ip),
            src_port,
            IpAddr::V6(dst_ip),
            dst_port,
        )
    }

    /// 移除 Socket
    ///
    /// # 参数
    /// - socket_id: Socket ID
    ///
    /// # 返回
    /// - Option<Arc<Mutex<TcpSocket>>>: 被移除的 Socket（如果存在）
    pub fn remove_socket(&mut self, socket_id: u64) -> Option<Arc<Mutex<TcpSocket>>> {
        // 首先移除所有与此 Socket 关联的连接
        self.connections.retain(|_, &mut id| id != socket_id);

        // 然后移除 Socket
        self.sockets.remove(&socket_id)
    }

    /// 解除连接绑定
    ///
    /// # 参数
    /// - tuple: 连接四元组
    ///
    /// # 返回
    /// - Option<u64>: 被解除绑定的 Socket ID（如果存在）
    pub fn unbind_connection(&mut self, tuple: &ConnectionTuple) -> Option<u64> {
        self.connections.remove(tuple)
    }

    /// 获取 Socket 数量
    pub fn socket_count(&self) -> usize {
        self.sockets.len()
    }

    /// 获取连接数量
    pub fn connection_count(&self) -> usize {
        self.connections.len()
    }

    /// 清理所有 Socket 和连接
    pub fn clear(&mut self) {
        self.sockets.clear();
        self.connections.clear();
    }
}

impl Default for TcpSocketManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_socket_manager_new() {
        let mgr = TcpSocketManager::new();
        assert_eq!(mgr.socket_count(), 0);
        assert_eq!(mgr.connection_count(), 0);
    }

    #[test]
    fn test_socket_manager_default() {
        let mgr = TcpSocketManager::default();
        assert_eq!(mgr.socket_count(), 0);
        assert_eq!(mgr.connection_count(), 0);
    }

    #[test]
    fn test_create_socket() {
        let mut mgr = TcpSocketManager::new();

        let socket = mgr.create_socket();
        assert_eq!(mgr.socket_count(), 1);

        // 验证 Socket ID 已设置
        let socket_guard = socket.lock().unwrap();
        assert!(socket_guard.id() > 0);
    }

    #[test]
    fn test_create_multiple_sockets() {
        let mut mgr = TcpSocketManager::new();

        let socket1 = mgr.create_socket();
        let socket2 = mgr.create_socket();

        assert_eq!(mgr.socket_count(), 2);

        // Socket ID 应该不同
        let id1 = socket1.lock().unwrap().id();
        let id2 = socket2.lock().unwrap().id();
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_find_socket() {
        let mut mgr = TcpSocketManager::new();

        let socket = mgr.create_socket();
        let socket_id = socket.lock().unwrap().id();

        let found = mgr.find(socket_id);
        assert!(found.is_some());
    }

    #[test]
    fn test_find_socket_not_exist() {
        let mgr = TcpSocketManager::new();

        let found = mgr.find(999);
        assert!(found.is_none());
    }

    #[test]
    fn test_bind_and_find_by_connection() {
        let mut mgr = TcpSocketManager::new();

        let socket = mgr.create_socket();
        let socket_id = socket.lock().unwrap().id();

        let src_ip = Ipv4Addr::new(192, 168, 1, 10);
        let dst_ip = Ipv4Addr::new(192, 168, 1, 20);
        let tuple = ConnectionTuple::new_v4(src_ip, 1234, dst_ip, 5678);

        // 绑定连接
        assert!(mgr.bind_connection(tuple, socket_id).is_ok());
        assert_eq!(mgr.connection_count(), 1);

        // 通过连接查找
        let found = mgr.find_by_connection_v4(src_ip, 1234, dst_ip, 5678);
        assert!(found.is_some());
        assert_eq!(found.unwrap().lock().unwrap().id(), socket_id);
    }

    #[test]
    fn test_find_by_connection_reverse() {
        let mut mgr = TcpSocketManager::new();

        let socket = mgr.create_socket();
        let socket_id = socket.lock().unwrap().id();

        let src_ip = Ipv4Addr::new(192, 168, 1, 10);
        let dst_ip = Ipv4Addr::new(192, 168, 1, 20);
        let tuple = ConnectionTuple::new_v4(src_ip, 1234, dst_ip, 5678);

        // 绑定连接
        assert!(mgr.bind_connection(tuple, socket_id).is_ok());

        // 尝试反向查找（应该也能找到）
        let found = mgr.find_by_connection_v4(dst_ip, 5678, src_ip, 1234);
        assert!(found.is_some());
        assert_eq!(found.unwrap().lock().unwrap().id(), socket_id);
    }

    #[test]
    fn test_bind_connection_invalid_socket() {
        let mut mgr = TcpSocketManager::new();

        let tuple = ConnectionTuple::new_v4(
            Ipv4Addr::new(192, 168, 1, 10),
            1234,
            Ipv4Addr::new(192, 168, 1, 20),
            5678,
        );

        let result = mgr.bind_connection(tuple, 999);
        assert!(result.is_err());
    }

    #[test]
    fn test_remove_socket() {
        let mut mgr = TcpSocketManager::new();

        let socket = mgr.create_socket();
        let socket_id = socket.lock().unwrap().id();

        // 绑定连接
        let tuple = ConnectionTuple::new_v4(
            Ipv4Addr::new(192, 168, 1, 10),
            1234,
            Ipv4Addr::new(192, 168, 1, 20),
            5678,
        );
        mgr.bind_connection(tuple, socket_id).unwrap();

        assert_eq!(mgr.socket_count(), 1);
        assert_eq!(mgr.connection_count(), 1);

        // 移除 Socket
        let removed = mgr.remove_socket(socket_id);
        assert!(removed.is_some());
        assert_eq!(mgr.socket_count(), 0);
        assert_eq!(mgr.connection_count(), 0);
    }

    #[test]
    fn test_unbind_connection() {
        let mut mgr = TcpSocketManager::new();

        let socket = mgr.create_socket();
        let socket_id = socket.lock().unwrap().id();

        let tuple = ConnectionTuple::new_v4(
            Ipv4Addr::new(192, 168, 1, 10),
            1234,
            Ipv4Addr::new(192, 168, 1, 20),
            5678,
        );
        mgr.bind_connection(tuple, socket_id).unwrap();

        assert_eq!(mgr.connection_count(), 1);

        // 解除绑定
        let unbound_id = mgr.unbind_connection(&tuple);
        assert_eq!(unbound_id, Some(socket_id));
        assert_eq!(mgr.connection_count(), 0);

        // Socket 应该仍然存在
        assert_eq!(mgr.socket_count(), 1);
    }

    #[test]
    fn test_clear() {
        let mut mgr = TcpSocketManager::new();

        mgr.create_socket();
        mgr.create_socket();

        let socket_id = mgr.create_socket().lock().unwrap().id();

        let tuple = ConnectionTuple::new_v4(
            Ipv4Addr::new(192, 168, 1, 10),
            1234,
            Ipv4Addr::new(192, 168, 1, 20),
            5678,
        );
        mgr.bind_connection(tuple, socket_id).unwrap();

        assert_eq!(mgr.socket_count(), 3);
        assert_eq!(mgr.connection_count(), 1);

        mgr.clear();
        assert_eq!(mgr.socket_count(), 0);
        assert_eq!(mgr.connection_count(), 0);
    }

    #[test]
    fn test_connection_tuple_reverse() {
        let tuple = ConnectionTuple::new_v4(
            Ipv4Addr::new(192, 168, 1, 10),
            1234,
            Ipv4Addr::new(192, 168, 1, 20),
            5678,
        );

        let reversed = tuple.reverse();

        assert_eq!(reversed.src_ip, IpAddr::V4(Ipv4Addr::new(192, 168, 1, 20)));
        assert_eq!(reversed.src_port, 5678);
        assert_eq!(reversed.dst_ip, IpAddr::V4(Ipv4Addr::new(192, 168, 1, 10)));
        assert_eq!(reversed.dst_port, 1234);
    }
}
