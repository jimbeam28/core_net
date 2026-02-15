// src/common/addr.rs
//
// 公共网络地址类型定义
// 包含 MAC 地址、IP 地址等网络协议中常用的地址类型

use std::fmt;

// ========== MAC 地址 ==========

/// MAC 地址（以太网硬件地址）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MacAddr {
    pub bytes: [u8; 6],
}

impl MacAddr {
    /// 创建新的 MAC 地址
    pub fn new(bytes: [u8; 6]) -> Self {
        MacAddr { bytes }
    }

    /// 创建广播 MAC 地址 ff:ff:ff:ff:ff:ff
    pub fn broadcast() -> Self {
        MacAddr { bytes: [0xff, 0xff, 0xff, 0xff, 0xff, 0xff] }
    }

    /// 创建零 MAC 地址
    pub fn zero() -> Self {
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
}

impl fmt::Display for MacAddr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}",
            self.bytes[0], self.bytes[1], self.bytes[2],
            self.bytes[3], self.bytes[4], self.bytes[5]
        )
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
    pub fn new(bytes: [u8; 4]) -> Self {
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
    pub fn localhost() -> Self {
        Ipv4Addr { bytes: [127, 0, 0, 1] }
    }

    /// 创建零地址 0.0.0.0
    pub fn zero() -> Self {
        Ipv4Addr { bytes: [0; 4] }
    }

    /// 是否为零地址
    pub fn is_zero(&self) -> bool {
        self.bytes == [0; 4]
    }

    /// 是否为广播地址 255.255.255.255
    pub fn is_broadcast(&self) -> bool {
        self.bytes == [0xff, 0xff, 0xff, 0xff]
    }

    /// 是否为本地回环地址
    pub fn is_loopback(&self) -> bool {
        self.bytes[0] == 127
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
