// src/socket/types.rs
//
// Socket API 类型定义

use crate::common::addr::{Ipv4Addr, Ipv6Addr};

// ========== Socket 描述符 ==========

/// Socket 文件描述符
///
/// 内部维护一个递增的整数，类似于 Linux 的 fd 分配机制
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SocketFd(pub u32);

impl SocketFd {
    /// 无效的 Socket 描述符
    pub const INVALID: Self = Self(u32::MAX);

    /// 标准输入（保留，本实现不使用）
    pub const STDIN: Self = Self(0);

    /// 标准输出（保留，本实现不使用）
    pub const STDOUT: Self = Self(1);

    /// 标准错误（保留，本实现不使用）
    pub const STDERR: Self = Self(2);

    /// 第一个可用的 Socket 描述符
    pub const FIRST_AVAILABLE: Self = Self(3);
}

// ========== 协议族与类型 ==========

/// 协议族 (Address Family)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AddressFamily {
    /// IPv4 协议族
    AF_INET,
    /// IPv6 协议族
    AF_INET6,
}

/// Socket 类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SocketType {
    /// 流式套接字 (TCP)
    SOCK_STREAM,
    /// 数据报套接字 (UDP)
    SOCK_DGRAM,
}

/// Socket 协议
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SocketProtocol {
    /// 默认协议 (0)
    Default,
    /// ICMP 协议 (1)
    ICMP,
    /// TCP 协议 (6)
    TCP,
    /// UDP 协议 (17)
    UDP,
}

// ========== Socket 地址 ==========

/// Socket 地址枚举
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SocketAddr {
    V4(SocketAddrV4),
    V6(SocketAddrV6),
}

impl SocketAddr {
    /// 获取 IP 地址
    pub fn ip(&self) -> IpAddr {
        match self {
            Self::V4(addr) => IpAddr::V4(addr.ip),
            Self::V6(addr) => IpAddr::V6(addr.ip),
        }
    }

    /// 获取端口号
    pub fn port(&self) -> u16 {
        match self {
            Self::V4(addr) => addr.port,
            Self::V6(addr) => addr.port,
        }
    }
}

/// IPv4 Socket 地址
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SocketAddrV4 {
    /// IP 地址
    pub ip: Ipv4Addr,
    /// 端口号
    pub port: u16,
}

impl SocketAddrV4 {
    /// 创建新的 IPv4 Socket 地址
    pub fn new(ip: Ipv4Addr, port: u16) -> Self {
        Self { ip, port }
    }
}

/// IPv6 Socket 地址
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SocketAddrV6 {
    /// IP 地址
    pub ip: Ipv6Addr,
    /// 端口号
    pub port: u16,
    /// 流标签
    pub flowinfo: u32,
    /// 范围 ID
    pub scope_id: u32,
}

impl SocketAddrV6 {
    /// 创建新的 IPv6 Socket 地址
    pub fn new(ip: Ipv6Addr, port: u16) -> Self {
        Self {
            ip,
            port,
            flowinfo: 0,
            scope_id: 0,
        }
    }
}

// ========== IP 地址枚举 ==========

/// IP 地址枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IpAddr {
    V4(Ipv4Addr),
    V6(Ipv6Addr),
}

// ========== TCP 状态 ==========

/// TCP 连接状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TcpState {
    /// 不存在
    Closed,
    /// 正在建立连接
    Listen,
    /// SYN 已发送
    SynSent,
    /// SYN 已接收
    SynReceived,
    /// 连接已建立
    Established,
    /// 正在关闭
    FinWait1,
    /// 半关闭状态
    FinWait2,
    /// 对方已关闭
    CloseWait,
    /// FIN 已发送
    Closing,
    /// 等待 FIN
    LastAck,
    /// 等待远程关闭
    TimeWait,
}

// ========== 发送/接收标志 ==========

/// 发送标志
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SendFlags {
    /// MSG_DONTROUTE - 不使用路由（本实现暂不支持）
    pub dont_route: bool,

    /// MSG_OOB - 发送带外数据（本实现暂不支持）
    pub oob: bool,
}

impl SendFlags {
    /// 无标志
    pub const NONE: Self = Self {
        dont_route: false,
        oob: false,
    };
}

impl Default for SendFlags {
    fn default() -> Self {
        Self::NONE
    }
}

/// 接收标志
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RecvFlags {
    /// MSG_PEEK - 查看数据但不从队列移除（本实现暂不支持）
    pub peek: bool,

    /// MSG_WAITALL - 等待请求的全部数据（本实现暂不支持）
    pub wait_all: bool,
}

impl RecvFlags {
    /// 无标志
    pub const NONE: Self = Self {
        peek: false,
        wait_all: false,
    };
}

impl Default for RecvFlags {
    fn default() -> Self {
        Self::NONE
    }
}

// ========== 配置常量 ==========

/// 默认 Socket 缓冲区大小
pub const DEFAULT_SOCKET_BUFFER_SIZE: usize = 8192;

/// Socket 表最大容量
pub const MAX_SOCKET_TABLE_SIZE: usize = 1024;
