// src/protocols/tcp/socket_manager.rs
//
// TCP Socket 管理器
// 管理 TCP Socket 的生命周期

use super::socket::TcpSocket;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// TCP Socket 管理器
///
/// 管理所有 TCP Socket，提供 Socket 创建、查找和删除功能。
#[derive(Debug)]
pub struct TcpSocketManager {
    /// Socket 表（socket_id -> TcpSocket）
    sockets: HashMap<u64, Arc<Mutex<TcpSocket>>>,

    /// 下一个 Socket ID
    next_socket_id: u64,
}

impl TcpSocketManager {
    /// 创建新的 Socket 管理器
    pub fn new() -> Self {
        Self {
            sockets: HashMap::new(),
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

    /// 查找 Socket
    ///
    /// # 参数
    /// - socket_id: Socket ID
    ///
    /// # 返回
    /// - Option<Arc<Mutex<TcpSocket>>>: Socket（如果存在）
    pub fn find(&self, socket_id: u64) -> Option<Arc<Mutex<TcpSocket>>> {
        self.sockets.get(&socket_id).cloned()
    }

    /// 移除 Socket
    ///
    /// # 参数
    /// - socket_id: Socket ID
    ///
    /// # 返回
    /// - Option<Arc<Mutex<TcpSocket>>>: 被移除的 Socket（如果存在）
    pub fn remove_socket(&mut self, socket_id: u64) -> Option<Arc<Mutex<TcpSocket>>> {
        self.sockets.remove(&socket_id)
    }

    /// 获取 Socket 数量
    pub fn socket_count(&self) -> usize {
        self.sockets.len()
    }

    /// 清理所有 Socket
    pub fn clear(&mut self) {
        self.sockets.clear();
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
    }

    #[test]
    fn test_socket_manager_default() {
        let mgr = TcpSocketManager::default();
        assert_eq!(mgr.socket_count(), 0);
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
    fn test_remove_socket() {
        let mut mgr = TcpSocketManager::new();

        let socket = mgr.create_socket();
        let socket_id = socket.lock().unwrap().id();

        assert_eq!(mgr.socket_count(), 1);

        let removed = mgr.remove_socket(socket_id);
        assert!(removed.is_some());
        assert_eq!(mgr.socket_count(), 0);
    }

    #[test]
    fn test_clear() {
        let mut mgr = TcpSocketManager::new();

        mgr.create_socket();
        mgr.create_socket();
        assert_eq!(mgr.socket_count(), 2);

        mgr.clear();
        assert_eq!(mgr.socket_count(), 0);
    }
}
