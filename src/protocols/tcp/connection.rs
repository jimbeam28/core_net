// src/protocols/tcp/connection.rs
//
// TCP 连接管理器

use super::tcb::{Tcb, TcpConnectionId, TcpState};
use super::config::TcpConfig;
use super::TcpError;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// TCP 连接管理器
///
/// 管理所有 TCP 连接的 TCB，提供连接查找、添加、更新、删除等功能。
#[derive(Debug)]
pub struct TcpConnectionManager {
    /// 连接表（四元组 -> TCB）
    connections: HashMap<TcpConnectionId, Arc<Mutex<Tcb>>>,

    /// Socket ID 映射（socket_id -> 四元组）
    socket_id_map: HashMap<u64, TcpConnectionId>,

    /// 监听端口表（端口 -> TCB）
    listen_sockets: HashMap<u16, Arc<Mutex<Tcb>>>,

    /// 配置
    config: TcpConfig,

    /// 下一个临时端口号
    next_ephemeral_port: u16,

    /// 下一个 socket ID
    next_socket_id: u64,
}

impl TcpConnectionManager {
    /// 创建新的连接管理器
    pub fn new(config: TcpConfig) -> Self {
        Self {
            connections: HashMap::new(),
            socket_id_map: HashMap::new(),
            listen_sockets: HashMap::new(),
            config,
            next_ephemeral_port: 32768, // 临时端口范围起始
            next_socket_id: 1,
        }
    }

    /// 查找连接
    pub fn find(&self, id: &TcpConnectionId) -> Option<Arc<Mutex<Tcb>>> {
        self.connections.get(id).cloned()
    }

    /// 查找监听端口
    pub fn find_listen(&self, port: u16) -> Option<Arc<Mutex<Tcb>>> {
        self.listen_sockets.get(&port).cloned()
    }

    /// 添加连接
    pub fn add(&mut self, tcb: Tcb) -> Result<(), TcpError> {
        let id = tcb.id.clone();

        // 检查连接数量限制
        if self.connections.len() >= self.config.max_connections {
            return Err(TcpError::BufferFull);
        }

        // 分配 socket_id
        let socket_id = self.next_socket_id;
        self.next_socket_id = self.next_socket_id.wrapping_add(1);

        // 存储 socket_id 到连接 ID 的映射
        self.socket_id_map.insert(socket_id, id.clone());

        self.connections.insert(id, Arc::new(Mutex::new(tcb)));
        Ok(())
    }

    /// 通过 socket ID 查找连接
    pub fn find_by_socket_id(&self, socket_id: u64) -> Option<Arc<Mutex<Tcb>>> {
        if let Some(conn_id) = self.socket_id_map.get(&socket_id) {
            self.connections.get(conn_id).cloned()
        } else {
            None
        }
    }

    /// 获取连接的 socket ID
    pub fn get_socket_id(&self, conn_id: &TcpConnectionId) -> Option<u64> {
        // 反向查找 socket_id
        for (&socket_id, id) in &self.socket_id_map {
            if id == conn_id {
                return Some(socket_id);
            }
        }
        None
    }

    /// 添加监听端口
    pub fn add_listen(&mut self, tcb: Tcb) -> Result<(), TcpError> {
        let port = tcb.id.local_port;

        if self.listen_sockets.contains_key(&port) {
            return Err(TcpError::Other(format!("端口 {} 已在监听", port)));
        }

        self.listen_sockets.insert(port, Arc::new(Mutex::new(tcb)));
        Ok(())
    }

    /// 移除连接
    ///
    /// 返回被移除连接的 Arc<Mutex<Tcb>>，如果连接不存在则返回 None。
    /// 注意：如果连接仍被其他线程引用，TCB 不会被立即销毁。
    pub fn remove(&mut self, id: &TcpConnectionId) -> Option<Arc<Mutex<Tcb>>> {
        // 先查找并移除 socket_id 映射
        let socket_id = self.get_socket_id(id);
        if let Some(sid) = socket_id {
            self.socket_id_map.remove(&sid);
        }

        self.connections.remove(id)
    }

    /// 移除监听端口
    ///
    /// 返回被移除监听端口的 Arc<Mutex<Tcb>>，如果端口不存在则返回 None。
    pub fn remove_listen(&mut self, port: u16) -> Option<Arc<Mutex<Tcb>>> {
        self.listen_sockets.remove(&port)
    }

    /// 分配临时端口
    pub fn allocate_ephemeral_port(&mut self) -> Result<u16, TcpError> {
        let start = self.next_ephemeral_port;
        loop {
            let port = self.next_ephemeral_port;
            self.next_ephemeral_port = if self.next_ephemeral_port == 65535 {
                32768
            } else {
                self.next_ephemeral_port + 1
            };

            // 检查端口是否可用
            if !self.listen_sockets.contains_key(&port) {
                // 检查是否已有连接使用该端口
                let port_in_use = self.connections.keys().any(|id| id.local_port == port);
                if !port_in_use {
                    return Ok(port);
                }
            }

            // 防止无限循环
            if self.next_ephemeral_port == start {
                return Err(TcpError::Other("无可用临时端口".to_string()));
            }
        }
    }

    /// 获取连接数量
    pub fn connection_count(&self) -> usize {
        self.connections.len()
    }

    /// 获取监听端口数量
    pub fn listen_count(&self) -> usize {
        self.listen_sockets.len()
    }

    /// 清理所有连接
    pub fn clear(&mut self) {
        self.connections.clear();
        self.socket_id_map.clear();
        self.listen_sockets.clear();
    }

    /// 清理已关闭的连接
    pub fn cleanup_closed(&mut self) {
        // 收集需要移除的连接 ID 和 socket ID
        let mut to_remove = Vec::new();
        let mut socket_ids_to_remove = Vec::new();

        for (socket_id, conn_id) in &self.socket_id_map {
            if let Some(tcb) = self.connections.get(conn_id)
                && let Ok(guard) = tcb.lock()
                && guard.state == TcpState::Closed
            {
                to_remove.push(conn_id.clone());
                socket_ids_to_remove.push(*socket_id);
            }
        }

        // 移除已关闭的连接
        for conn_id in to_remove {
            self.connections.remove(&conn_id);
        }
        for socket_id in socket_ids_to_remove {
            self.socket_id_map.remove(&socket_id);
        }
    }

    /// 查找或创建连接（用于被动打开）
    pub fn find_or_create_passive(
        &mut self,
        local_ip: crate::protocols::Ipv4Addr,
        local_port: u16,
        remote_ip: crate::protocols::Ipv4Addr,
        remote_port: u16,
    ) -> Result<Arc<Mutex<Tcb>>, TcpError> {
        let id = TcpConnectionId::new(local_ip, local_port, remote_ip, remote_port);

        // 尝试查找现有连接
        if let Some(tcb) = self.find(&id) {
            return Ok(tcb);
        }

        // 检查半连接数量限制
        let half_open_count = self.connections.values().filter(|tcb| {
            if let Ok(guard) = tcb.lock() {
                guard.state == TcpState::SynReceived
            } else {
                false
            }
        }).count();

        if half_open_count >= self.config.max_half_connections {
            return Err(TcpError::BufferFull);
        }

        // 创建新连接
        let mut tcb = Tcb::new(id.clone());
        tcb.state = TcpState::SynReceived;
        tcb.rcv_wnd = self.config.default_window_size;
        tcb.mss = self.config.max_segment_size;

        // 分配 socket_id
        let socket_id = self.next_socket_id;
        self.next_socket_id = self.next_socket_id.wrapping_add(1);

        // 存储 socket_id 到连接 ID 的映射
        self.socket_id_map.insert(socket_id, id.clone());

        let tcb_arc = Arc::new(Mutex::new(tcb));
        self.connections.insert(id, tcb_arc.clone());

        Ok(tcb_arc)
    }

    /// 获取配置
    pub const fn config(&self) -> &TcpConfig {
        &self.config
    }
}

impl Default for TcpConnectionManager {
    fn default() -> Self {
        Self::new(TcpConfig::default())
    }
}

/// TCP 选项解析
#[derive(Debug, Clone, PartialEq)]
pub enum TcpOption {
    /// 最大分段大小
    MaxSegmentSize { mss: u16 },
    /// 窗口缩放
    WindowScale { shift: u8 },
    /// SACK 允许
    SackPermitted,
    /// SACK 块
    Sack { blocks: Vec<(u32, u32)> },
    /// 时间戳
    Timestamps { ts_val: u32, ts_ecr: u32 },
    /// 无操作（填充）
    Nop,
    /// 行尾（选项结束）
    End,
}

impl TcpOption {
    /// 解析 TCP 选项
    pub fn parse_options(data: &[u8]) -> Result<Vec<TcpOption>, TcpError> {
        let mut options = Vec::new();
        let mut i = 0;

        while i < data.len() {
            let kind = data[i];

            match kind {
                0 => {
                    // END
                    options.push(TcpOption::End);
                    break;
                }
                1 => {
                    // NOP
                    options.push(TcpOption::Nop);
                    i += 1;
                }
                2 => {
                    // MSS (Kind=2, Length=4)
                    if i + 3 >= data.len() {
                        return Err(TcpError::ParseError("MSS 选项长度不足".to_string()));
                    }
                    let length = data[i + 1] as usize;
                    if length != 4 {
                        return Err(TcpError::ParseError(format!("无效的 MSS 长度: {}", length)));
                    }
                    let mss = u16::from_be_bytes([data[i + 2], data[i + 3]]);
                    options.push(TcpOption::MaxSegmentSize { mss });
                    i += length;
                }
                3 => {
                    // Window Scale (Kind=3, Length=3)
                    if i + 2 >= data.len() {
                        return Err(TcpError::ParseError("Window Scale 选项长度不足".to_string()));
                    }
                    let length = data[i + 1] as usize;
                    if length != 3 {
                        return Err(TcpError::ParseError(format!("无效的 Window Scale 长度: {}", length)));
                    }
                    let shift = data[i + 2];
                    options.push(TcpOption::WindowScale { shift });
                    i += length;
                }
                4 => {
                    // SACK Permitted (Kind=4, Length=2)
                    if i + 1 >= data.len() {
                        return Err(TcpError::ParseError("SACK Permitted 选项长度不足".to_string()));
                    }
                    let length = data[i + 1] as usize;
                    if length != 2 {
                        return Err(TcpError::ParseError(format!("无效的 SACK Permitted 长度: {}", length)));
                    }
                    options.push(TcpOption::SackPermitted);
                    i += length;
                }
                8 => {
                    // Timestamps (Kind=8, Length=10)
                    if i + 9 >= data.len() {
                        return Err(TcpError::ParseError("Timestamps 选项长度不足".to_string()));
                    }
                    let length = data[i + 1] as usize;
                    if length != 10 {
                        return Err(TcpError::ParseError(format!("无效的 Timestamps 长度: {}", length)));
                    }
                    let ts_val = u32::from_be_bytes([data[i + 2], data[i + 3], data[i + 4], data[i + 5]]);
                    let ts_ecr = u32::from_be_bytes([data[i + 6], data[i + 7], data[i + 8], data[i + 9]]);
                    options.push(TcpOption::Timestamps { ts_val, ts_ecr });
                    i += length;
                }
                _ => {
                    // 未知选项，尝试跳过
                    if i + 1 < data.len() {
                        let length = data[i + 1] as usize;
                        if length >= 2 {
                            i += length;
                        } else {
                            i += 2; // 最小长度
                        }
                    } else {
                        break;
                    }
                }
            }
        }

        Ok(options)
    }

    /// 序列化选项
    pub fn serialize_options(options: &[TcpOption]) -> Vec<u8> {
        let mut bytes = Vec::new();

        for opt in options {
            match opt {
                TcpOption::End => {
                    bytes.push(0);
                }
                TcpOption::Nop => {
                    bytes.push(1);
                }
                TcpOption::MaxSegmentSize { mss } => {
                    bytes.push(2); // Kind
                    bytes.push(4); // Length
                    bytes.extend_from_slice(&mss.to_be_bytes());
                }
                TcpOption::WindowScale { shift } => {
                    bytes.push(3); // Kind
                    bytes.push(3); // Length
                    bytes.push(*shift);
                }
                TcpOption::SackPermitted => {
                    bytes.push(4); // Kind
                    bytes.push(2); // Length
                }
                TcpOption::Sack { blocks } => {
                    bytes.push(5); // Kind
                    bytes.push((2 + blocks.len() * 8) as u8); // Length
                    for (left, right) in blocks {
                        bytes.extend_from_slice(&left.to_be_bytes());
                        bytes.extend_from_slice(&right.to_be_bytes());
                    }
                }
                TcpOption::Timestamps { ts_val, ts_ecr } => {
                    bytes.push(8); // Kind
                    bytes.push(10); // Length
                    bytes.extend_from_slice(&ts_val.to_be_bytes());
                    bytes.extend_from_slice(&ts_ecr.to_be_bytes());
                }
            }
        }

        // 填充到 4 字节边界
        while bytes.len() % 4 != 0 {
            bytes.push(1); // NOP
        }

        bytes
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_connection_manager_new() {
        let mgr = TcpConnectionManager::new(TcpConfig::default());
        assert_eq!(mgr.connection_count(), 0);
        assert_eq!(mgr.listen_count(), 0);
    }

    #[test]
    fn test_connection_manager_default() {
        let mgr = TcpConnectionManager::default();
        assert_eq!(mgr.connection_count(), 0);
    }

    #[test]
    fn test_add_listen() {
        let mut mgr = TcpConnectionManager::new(TcpConfig::default());
        let tcb = Tcb::listen(crate::protocols::Ipv4Addr::new(192, 168, 1, 100), 80, 65535);

        let result = mgr.add_listen(tcb);
        assert!(result.is_ok());
        assert_eq!(mgr.listen_count(), 1);
    }

    #[test]
    fn test_find_listen() {
        let mut mgr = TcpConnectionManager::new(TcpConfig::default());
        let tcb = Tcb::listen(crate::protocols::Ipv4Addr::new(192, 168, 1, 100), 80, 65535);

        mgr.add_listen(tcb).unwrap();
        let found = mgr.find_listen(80);
        assert!(found.is_some());
    }

    #[test]
    fn test_remove_listen() {
        let mut mgr = TcpConnectionManager::new(TcpConfig::default());
        let tcb = Tcb::listen(crate::protocols::Ipv4Addr::new(192, 168, 1, 100), 80, 65535);

        mgr.add_listen(tcb).unwrap();
        let removed = mgr.remove_listen(80);
        assert!(removed.is_some());
        assert_eq!(mgr.listen_count(), 0);
    }

    #[test]
    fn test_allocate_ephemeral_port() {
        let mut mgr = TcpConnectionManager::new(TcpConfig::default());

        let port1 = mgr.allocate_ephemeral_port().unwrap();
        assert!(port1 >= 32768);

        let port2 = mgr.allocate_ephemeral_port().unwrap();
        assert!(port2 >= 32768);
        assert_ne!(port1, port2);
    }

    #[test]
    fn test_parse_options_mss() {
        let data = [2, 4, 0x05, 0xB4]; // MSS=1460
        let options = TcpOption::parse_options(&data).unwrap();

        assert_eq!(options.len(), 1);
        assert_eq!(options[0], TcpOption::MaxSegmentSize { mss: 1460 });
    }

    #[test]
    fn test_parse_options_multiple() {
        let data = [
            2, 4, 0x05, 0xB4,  // MSS
            1,                // NOP
            3, 3, 0x00,       // Window Scale
        ];
        let options = TcpOption::parse_options(&data).unwrap();

        assert_eq!(options.len(), 3);
        assert_eq!(options[0], TcpOption::MaxSegmentSize { mss: 1460 });
        assert_eq!(options[1], TcpOption::Nop);
        assert_eq!(options[2], TcpOption::WindowScale { shift: 0 });
    }

    #[test]
    fn test_serialize_options_mss() {
        let options = vec![TcpOption::MaxSegmentSize { mss: 1460 }];
        let bytes = TcpOption::serialize_options(&options);

        assert_eq!(bytes.len(), 4);
        assert_eq!(bytes[0], 2); // Kind
        assert_eq!(bytes[1], 4); // Length
    }

    #[test]
    fn test_serialize_options_padding() {
        let options = vec![TcpOption::MaxSegmentSize { mss: 1460 }, TcpOption::Nop];
        let bytes = TcpOption::serialize_options(&options);

        // 应该填充到 4 字节边界
        assert_eq!(bytes.len() % 4, 0);
    }
}
