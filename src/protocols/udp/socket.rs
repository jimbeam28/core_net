// src/protocols/udp/socket.rs
//
// UDP Socket API
// 提供应用层 UDP 通信接口

use crate::common::{CoreError, Result};
use crate::protocols::Ipv4Addr;
use crate::context::SystemContext;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

/// UDP Socket
///
/// 提供类似 POSIX 的 UDP socket 接口，支持：
/// - bind() 绑定本地端口
/// - set_callback() 设置接收回调
/// - close() 关闭 socket
///
/// 注意：sendto 功能由协议栈处理，应用层通过回调接收数据
#[derive(Clone)]
pub struct UdpSocket {
    /// 系统上下文
    context: SystemContext,
    /// 绑定的端口号（None 表示未绑定）
    port: Option<u16>,
    /// Socket 是否已关闭
    closed: Arc<AtomicBool>,
}

impl UdpSocket {
    /// 创建新的 UDP Socket
    ///
    /// # 参数
    /// - context: 系统上下文
    ///
    /// # 返回
    /// - UdpSocket: 新创建的 socket（未绑定状态）
    pub fn new(context: SystemContext) -> Self {
        Self {
            context,
            port: None,
            closed: Arc::new(AtomicBool::new(false)),
        }
    }

    /// 绑定到本地端口
    ///
    /// # 参数
    /// - port: 要绑定的端口号（0 表示自动分配）
    ///
    /// # 返回
    /// - Ok(u16): 实际绑定的端口号
    /// - Err: 绑定失败（端口已被占用或无效）
    pub fn bind(&mut self, port: u16) -> Result<u16> {
        if self.closed.load(Ordering::SeqCst) {
            return Err(CoreError::invalid_packet("Socket已关闭"));
        }

        if self.port.is_some() {
            return Err(CoreError::invalid_packet("Socket已绑定端口"));
        }

        let bound_port = self.context.udp_ports.lock().unwrap().bind(port)?;
        self.port = Some(bound_port);

        Ok(bound_port)
    }

    /// 设置接收回调
    ///
    /// 当接收到发送到此端口的数据时，将调用此回调。
    ///
    /// # 参数
    /// - callback: 接收回调函数
    ///
    /// # 返回
    /// - Ok(()): 设置成功
    /// - Err: 设置失败（socket 未绑定或已关闭）
    pub fn set_callback<F>(&self, callback: F) -> Result<()>
    where
        F: Fn(Ipv4Addr, u16, Vec<u8>) + Send + 'static,
    {
        if self.closed.load(Ordering::SeqCst) {
            return Err(CoreError::invalid_packet("Socket已关闭"));
        }

        let port = self.port.ok_or_else(|| {
            CoreError::invalid_packet("Socket未绑定端口")
        })?;

        let port_manager = self.context.udp_ports.lock().unwrap();
        let entry = port_manager.lookup(port).ok_or_else(|| {
            CoreError::invalid_packet("端口未绑定")
        })?;

        entry.set_callback(Box::new(callback));
        Ok(())
    }

    /// 移除接收回调
    ///
    /// # 返回
    /// - Ok(()): 移除成功
    /// - Err: 移除失败（socket 未绑定或已关闭）
    pub fn clear_callback(&self) -> Result<()> {
        if self.closed.load(Ordering::SeqCst) {
            return Err(CoreError::invalid_packet("Socket已关闭"));
        }

        let port = self.port.ok_or_else(|| {
            CoreError::invalid_packet("Socket未绑定端口")
        })?;

        let port_manager = self.context.udp_ports.lock().unwrap();
        let entry = port_manager.lookup(port).ok_or_else(|| {
            CoreError::invalid_packet("端口未绑定")
        })?;

        entry.clear_callback();
        Ok(())
    }

    /// 关闭 Socket
    ///
    /// 释放绑定的端口并清除回调。
    ///
    /// # 返回
    /// - Ok(()): 关闭成功
    /// - Err: 关闭失败
    pub fn close(&mut self) -> Result<()> {
        if self.closed.load(Ordering::SeqCst) {
            return Ok(()); // 已关闭
        }

        if let Some(port) = self.port {
            // 清除回调
            let port_manager = self.context.udp_ports.lock().unwrap();
            if let Some(entry) = port_manager.lookup(port) {
                entry.clear_callback();
            }
            drop(port_manager);

            // 解绑端口
            self.context.udp_ports.lock().unwrap().unbind(port)?;
        }

        self.closed.store(true, Ordering::SeqCst);
        self.port = None;

        Ok(())
    }

    /// 检查 Socket 是否已绑定
    ///
    /// # 返回
    /// - bool: 是否已绑定
    pub fn is_bound(&self) -> bool {
        self.port.is_some()
    }

    /// 获取绑定的端口号
    ///
    /// # 返回
    /// - Option<u16>: 绑定的端口号（如果已绑定）
    pub fn local_port(&self) -> Option<u16> {
        self.port
    }

    /// 检查 Socket 是否已关闭
    ///
    /// # 返回
    /// - bool: 是否已关闭
    pub fn is_closed(&self) -> bool {
        self.closed.load(Ordering::SeqCst)
    }

    /// 检查端口是否有回调
    ///
    /// # 返回
    /// - bool: 是否有回调
    pub fn has_callback(&self) -> bool {
        if let Some(port) = self.port {
            if let Ok(port_manager) = self.context.udp_ports.lock() {
                if let Some(entry) = port_manager.lookup(port) {
                    return entry.has_callback();
                }
            }
        }
        false
    }
}

impl Drop for UdpSocket {
    fn drop(&mut self) {
        // 自动关闭 socket
        let _ = self.close();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_socket_new() {
        let ctx = SystemContext::new();
        let socket = UdpSocket::new(ctx);

        assert!(!socket.is_bound());
        assert!(!socket.is_closed());
        assert!(socket.local_port().is_none());
    }

    #[test]
    fn test_socket_bind() {
        let ctx = SystemContext::new();
        let mut socket = UdpSocket::new(ctx.clone());

        let port = socket.bind(0).unwrap();
        assert!(port >= crate::protocols::udp::EPHEMERAL_PORT_MIN);
        assert!(socket.is_bound());
        assert_eq!(socket.local_port(), Some(port));
    }

    #[test]
    fn test_socket_bind_specific_port() {
        let ctx = SystemContext::new();
        let mut socket = UdpSocket::new(ctx.clone());

        let port = socket.bind(8080).unwrap();
        assert_eq!(port, 8080);
        assert_eq!(socket.local_port(), Some(8080));
    }

    #[test]
    fn test_socket_bind_already_bound() {
        let ctx = SystemContext::new();
        let mut socket = UdpSocket::new(ctx);

        socket.bind(8080).unwrap();
        let result = socket.bind(9090);
        assert!(result.is_err());
    }

    #[test]
    fn test_socket_bind_port_conflict() {
        let ctx = SystemContext::new();
        let mut socket1 = UdpSocket::new(ctx.clone());
        let mut socket2 = UdpSocket::new(ctx);

        socket1.bind(8080).unwrap();
        let result = socket2.bind(8080);
        assert!(result.is_err());
    }

    #[test]
    fn test_socket_close() {
        let ctx = SystemContext::new();
        let mut socket = UdpSocket::new(ctx.clone());

        socket.bind(8080).unwrap();
        assert!(socket.is_bound());

        socket.close().unwrap();
        assert!(!socket.is_bound());
        assert!(socket.is_closed());
        assert!(socket.local_port().is_none());
    }

    #[test]
    fn test_socket_close_twice() {
        let ctx = SystemContext::new();
        let mut socket = UdpSocket::new(ctx.clone());

        socket.bind(8080).unwrap();
        socket.close().unwrap();
        let result = socket.close();
        assert!(result.is_ok());
    }

    #[test]
    fn test_socket_set_callback() {
        let ctx = SystemContext::new();
        let mut socket = UdpSocket::new(ctx.clone());

        socket.bind(8080).unwrap();

        let result = socket.set_callback(|_src_addr, _src_port, _data| {
            // 回调逻辑
        });
        assert!(result.is_ok());
        assert!(socket.has_callback());
    }

    #[test]
    fn test_socket_set_callback_not_bound() {
        let ctx = SystemContext::new();
        let socket = UdpSocket::new(ctx);

        let result = socket.set_callback(|_src_addr, _src_port, _data| {});
        assert!(result.is_err());
    }

    #[test]
    fn test_socket_clear_callback() {
        let ctx = SystemContext::new();
        let mut socket = UdpSocket::new(ctx.clone());

        socket.bind(8080).unwrap();
        socket.set_callback(|_src_addr, _src_port, _data| {}).unwrap();
        assert!(socket.has_callback());

        socket.clear_callback().unwrap();
        assert!(!socket.has_callback());
    }

    #[test]
    fn test_socket_clone() {
        let ctx = SystemContext::new();
        let mut socket1 = UdpSocket::new(ctx.clone());
        let socket2 = socket1.clone();

        // 两个 socket 共享 closed 状态
        socket1.close().unwrap();
        assert!(socket2.is_closed());
    }

    #[test]
    fn test_socket_closed_operations() {
        let ctx = SystemContext::new();
        let mut socket = UdpSocket::new(ctx.clone());

        socket.bind(8080).unwrap();
        socket.close().unwrap();

        // 关闭后无法设置回调
        let result = socket.set_callback(|_src_addr, _src_port, _data| {});
        assert!(result.is_err());
    }
}
