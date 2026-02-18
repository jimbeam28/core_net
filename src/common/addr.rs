// src/common/addr.rs
//
// 公共网络地址类型定义
// 包含 MAC 地址、IP 地址等网络协议中常用的地址类型

use std::fmt;
use std::str::FromStr;

// ========== MAC 地址 ==========

/// MAC 地址（以太网硬件地址）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MacAddr {
    pub bytes: [u8; 6],
}

impl MacAddr {
    /// 创建新的 MAC 地址
    pub const fn new(bytes: [u8; 6]) -> Self {
        MacAddr { bytes }
    }

    /// 创建广播 MAC 地址 ff:ff:ff:ff:ff:ff
    pub const fn broadcast() -> Self {
        MacAddr {
            bytes: [0xff, 0xff, 0xff, 0xff, 0xff, 0xff],
        }
    }

    /// 创建零 MAC 地址
    pub const fn zero() -> Self {
        MacAddr { bytes: [0; 6] }
    }

    /// 是否为广播地址
    pub fn is_broadcast(&self) -> bool {
        self.bytes == [0xff, 0xff, 0xff, 0xff, 0xff, 0xff]
    }

    /// 是否为零地址
    pub fn is_zero(&self) -> bool {
        self.bytes == [0; 6]
    }

    /// 是否为多播地址
    pub fn is_multicast(&self) -> bool {
        self.bytes[0] & 0x01 == 0x01
    }

    /// 转换为字节数组引用
    pub const fn as_bytes(&self) -> &[u8; 6] {
        &self.bytes
    }
}

impl fmt::Display for MacAddr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}",
            self.bytes[0],
            self.bytes[1],
            self.bytes[2],
            self.bytes[3],
            self.bytes[4],
            self.bytes[5]
        )
    }
}

impl FromStr for MacAddr {
    type Err = AddrError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts: Vec<&str> = s.split(':').collect();
        if parts.len() != 6 {
            return Err(AddrError::InvalidMacAddr(s.to_string()));
        }

        let mut bytes = [0u8; 6];
        for (i, part) in parts.iter().enumerate() {
            bytes[i] = u8::from_str_radix(part, 16)
                .map_err(|_| AddrError::InvalidMacAddr(s.to_string()))?;
        }

        Ok(MacAddr::new(bytes))
    }
}

// ========== IPv4 地址 ==========

/// IPv4 地址
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Ipv4Addr {
    pub bytes: [u8; 4],
}

impl Ipv4Addr {
    /// 创建新的 IPv4 地址
    pub const fn new(a: u8, b: u8, c: u8, d: u8) -> Self {
        Ipv4Addr { bytes: [a, b, c, d] }
    }

    /// 从字节数组创建 IPv4 地址
    pub const fn from_bytes(bytes: [u8; 4]) -> Self {
        Ipv4Addr { bytes }
    }

    /// 从 u32 创建 IPv4 地址
    pub fn from_u32(addr: u32) -> Self {
        Ipv4Addr {
            bytes: addr.to_be_bytes(),
        }
    }

    /// 转换为 u32
    pub fn to_u32(&self) -> u32 {
        u32::from_be_bytes(self.bytes)
    }

    /// 创建本地回环地址 127.0.0.1
    pub const fn localhost() -> Self {
        Ipv4Addr {
            bytes: [127, 0, 0, 1],
        }
    }

    /// 创建零地址 0.0.0.0
    pub const fn unspecified() -> Self {
        Ipv4Addr { bytes: [0; 4] }
    }

    /// 创建广播地址 255.255.255.255
    pub const fn broadcast() -> Self {
        Ipv4Addr {
            bytes: [255, 255, 255, 255],
        }
    }

    /// 是否为零地址
    pub fn is_zero(&self) -> bool {
        self.bytes == [0; 4]
    }

    /// 是否为未指定地址 0.0.0.0
    pub fn is_unspecified(&self) -> bool {
        self.is_zero()
    }

    /// 是否为广播地址 255.255.255.255
    pub fn is_broadcast(&self) -> bool {
        self.bytes == [0xff, 0xff, 0xff, 0xff]
    }

    /// 是否为本地回环地址
    pub fn is_loopback(&self) -> bool {
        self.bytes[0] == 127
    }

    /// 转换为字节数组引用
    pub const fn as_bytes(&self) -> &[u8; 4] {
        &self.bytes
    }
}

impl fmt::Display for Ipv4Addr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}.{}.{}.{}",
            self.bytes[0], self.bytes[1], self.bytes[2], self.bytes[3]
        )
    }
}

impl FromStr for Ipv4Addr {
    type Err = AddrError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts: Vec<&str> = s.split('.').collect();
        if parts.len() != 4 {
            return Err(AddrError::InvalidIpAddr(s.to_string()));
        }

        let mut bytes = [0u8; 4];
        for (i, part) in parts.iter().enumerate() {
            bytes[i] = part
                .parse::<u8>()
                .map_err(|_| AddrError::InvalidIpAddr(s.to_string()))?;
        }

        Ok(Ipv4Addr::new(bytes[0], bytes[1], bytes[2], bytes[3]))
    }
}

// ========== 地址错误类型 ==========

/// 地址解析错误
#[derive(Debug)]
pub enum AddrError {
    /// 无效的 MAC 地址格式
    InvalidMacAddr(String),

    /// 无效的 IP 地址格式
    InvalidIpAddr(String),
}

impl fmt::Display for AddrError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AddrError::InvalidMacAddr(addr) => {
                write!(f, "无效的MAC地址格式: {}", addr)
            }
            AddrError::InvalidIpAddr(addr) => {
                write!(f, "无效的IP地址格式: {}", addr)
            }
        }
    }
}

impl std::error::Error for AddrError {}
