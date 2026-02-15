use std::fmt;

// 从 common 模块导入地址类型
pub use crate::common::{MacAddr, Ipv4Addr};

/// 接口错误类型
#[derive(Debug)]
pub enum InterfaceError {
    /// 接口名称重复
    DuplicateName(String),

    /// 接口未找到
    InterfaceNotFound,

    /// 配置文件读取失败
    ConfigReadFailed(String),

    /// 配置文件解析失败
    ConfigParseFailed(String),

    /// 配置文件写入失败
    ConfigWriteFailed(String),

    /// MAC地址格式无效
    InvalidMacAddr(String),

    /// IP地址格式无效
    InvalidIpAddr(String),

    /// MTU值无效
    InvalidMtu(u16),

    /// 配置文件格式错误
    InvalidFormat(String),

    /// 互斥锁锁定失败
    MutexLockFailed(String),
}

impl fmt::Display for InterfaceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            InterfaceError::DuplicateName(name) => {
                write!(f, "接口名称已存在: {}", name)
            }
            InterfaceError::InterfaceNotFound => {
                write!(f, "接口未找到")
            }
            InterfaceError::ConfigReadFailed(msg) => {
                write!(f, "配置文件读取失败: {}", msg)
            }
            InterfaceError::ConfigParseFailed(msg) => {
                write!(f, "配置文件解析失败: {}", msg)
            }
            InterfaceError::ConfigWriteFailed(msg) => {
                write!(f, "配置文件写入失败: {}", msg)
            }
            InterfaceError::InvalidMacAddr(addr) => {
                write!(f, "无效的MAC地址格式: {}", addr)
            }
            InterfaceError::InvalidIpAddr(addr) => {
                write!(f, "无效的IP地址格式: {}", addr)
            }
            InterfaceError::InvalidMtu(mtu) => {
                write!(f, "无效的MTU值: {}", mtu)
            }
            InterfaceError::InvalidFormat(msg) => {
                write!(f, "配置文件格式错误: {}", msg)
            }
            InterfaceError::MutexLockFailed(msg) => {
                write!(f, "互斥锁锁定失败: {}", msg)
            }
        }
    }
}

impl std::error::Error for InterfaceError {}

/// 网络接口状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InterfaceState {
    /// 接口已启用，可以收发数据
    Up,
    /// 接口已禁用
    Down,
    /// 接口处于测试模式
    Testing,
    /// 接口发生错误
    Error,
}

/// 接口类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InterfaceType {
    /// 以太网接口
    Ethernet,
    /// 本地回环接口
    Loopback,
    /// 虚拟接口
    Virtual,
}
