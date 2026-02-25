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

    /// 未指定地址 0.0.0.0（常量）
    pub const UNSPECIFIED: Ipv4Addr = Ipv4Addr { bytes: [0; 4] };

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

    /// 是否为组播地址 (224.0.0.0/4)
    pub fn is_multicast(&self) -> bool {
        // 组播地址范围: 224.0.0.0 ~ 239.255.255.255
        // 即第一字节的高4位为 1110 (0xE0)
        (self.bytes[0] & 0xF0) == 0xE0
    }

    /// 转换为字节数组引用
    pub const fn as_bytes(&self) -> &[u8; 4] {
        &self.bytes
    }

    /// 转换为字节数组（Vec）
    pub fn to_bytes(&self) -> [u8; 4] {
        self.bytes
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

// ========== IPv6 地址 ==========

/// IPv6 地址（128 位）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Ipv6Addr {
    pub bytes: [u8; 16],
}

impl Ipv6Addr {
    /// 从 8 个 16 位段创建 IPv6 地址
    #[allow(clippy::too_many_arguments)]
    pub const fn new(a: u16, b: u16, c: u16, d: u16,
                     e: u16, f: u16, g: u16, h: u16) -> Self {
        Ipv6Addr {
            bytes: [
                (a >> 8) as u8, a as u8,
                (b >> 8) as u8, b as u8,
                (c >> 8) as u8, c as u8,
                (d >> 8) as u8, d as u8,
                (e >> 8) as u8, e as u8,
                (f >> 8) as u8, f as u8,
                (g >> 8) as u8, g as u8,
                (h >> 8) as u8, h as u8,
            ],
        }
    }

    /// 从字节数组创建 IPv6 地址
    pub const fn from_bytes(bytes: [u8; 16]) -> Self {
        Ipv6Addr { bytes }
    }

    /// 未指定地址 ::
    pub const UNSPECIFIED: Ipv6Addr = Ipv6Addr { bytes: [0; 16] };

    /// 环回地址 ::1
    pub const LOOPBACK: Ipv6Addr = Ipv6Addr {
        bytes: [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1]
    };

    /// 所有节点组播地址 ff02::1
    pub const ALL_NODES_MULTICAST: Ipv6Addr = Ipv6Addr {
        bytes: [0xff, 0x02, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1]
    };

    /// 链路本地所有节点组播地址 ff02::1
    pub const LINK_LOCAL_ALL_NODES: Ipv6Addr = Ipv6Addr {
        bytes: [0xff, 0x02, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1]
    };

    /// 链路本地所有路由器组播地址 ff02::2
    pub const LINK_LOCAL_ALL_ROUTERS: Ipv6Addr = Ipv6Addr {
        bytes: [0xff, 0x02, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2]
    };

    /// 创建链路本地地址 fe80::/64
    pub fn link_local(eui64: [u8; 8]) -> Self {
        let mut bytes = [0u8; 16];
        bytes[0] = 0xfe;
        bytes[1] = 0x80;
        bytes[8..16].copy_from_slice(&eui64);
        Ipv6Addr { bytes }
    }

    /// 转换为字节数组引用
    pub const fn as_bytes(&self) -> &[u8; 16] {
        &self.bytes
    }

    /// 判断是否为未指定地址 ::
    pub fn is_unspecified(&self) -> bool {
        self.bytes == [0; 16]
    }

    /// 判断是否为环回地址 ::1
    pub fn is_loopback(&self) -> bool {
        self.bytes == [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1]
    }

    /// 判断是否为组播地址 (ff00::/8)
    pub fn is_multicast(&self) -> bool {
        self.bytes[0] == 0xff
    }

    /// 判断是否为链路本地地址 (fe80::/10)
    pub fn is_link_local(&self) -> bool {
        self.bytes[0] == 0xfe && (self.bytes[1] & 0xc0) == 0x80
    }

    /// 判断是否为站点本地地址 (已弃用, fec0::/10)
    pub fn is_site_local(&self) -> bool {
        self.bytes[0] == 0xfe && (self.bytes[1] & 0xc0) == 0xc0
    }

    /// 判断是否为全球单播地址 (2000::/3)
    pub fn is_global_unicast(&self) -> bool {
        (self.bytes[0] & 0xe0) == 0x20
    }

    /// 判断是否为单播地址（非组播）
    pub fn is_unicast(&self) -> bool {
        !self.is_multicast()
    }
}

impl fmt::Display for Ipv6Addr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // 将 16 字节转换为 8 个 u16 段
        let segments: [u16; 8] = [
            u16::from_be_bytes([self.bytes[0], self.bytes[1]]),
            u16::from_be_bytes([self.bytes[2], self.bytes[3]]),
            u16::from_be_bytes([self.bytes[4], self.bytes[5]]),
            u16::from_be_bytes([self.bytes[6], self.bytes[7]]),
            u16::from_be_bytes([self.bytes[8], self.bytes[9]]),
            u16::from_be_bytes([self.bytes[10], self.bytes[11]]),
            u16::from_be_bytes([self.bytes[12], self.bytes[13]]),
            u16::from_be_bytes([self.bytes[14], self.bytes[15]]),
        ];

        // 特殊情况：全部为零 -> ::
        if segments.iter().all(|&s| s == 0) {
            return write!(f, "::");
        }

        // 找到最长的连续零段（至少2段才压缩）
        let mut longest_zero_start = 0;
        let mut longest_zero_len = 0;
        let mut current_zero_start = 0;
        let mut current_zero_len = 0;

        for (i, &seg) in segments.iter().enumerate() {
            if seg == 0 {
                if current_zero_len == 0 {
                    current_zero_start = i;
                }
                current_zero_len += 1;
            } else {
                if current_zero_len > longest_zero_len && current_zero_len >= 2 {
                    longest_zero_start = current_zero_start;
                    longest_zero_len = current_zero_len;
                }
                current_zero_len = 0;
            }
        }
        // 检查最后一段
        if current_zero_len > longest_zero_len && current_zero_len >= 2 {
            longest_zero_start = current_zero_start;
            longest_zero_len = current_zero_len;
        }

        // 如果没有找到至少2段的连续零，则不压缩
        if longest_zero_len < 2 {
            // 不使用压缩
            for (i, &seg) in segments.iter().enumerate() {
                write!(f, "{:x}", seg)?;
                if i < 7 { write!(f, ":")?; }
            }
            return Ok(());
        }

        // 使用压缩格式
        for &seg in segments.iter().take(longest_zero_start) {
            write!(f, "{:x}", seg)?;
            write!(f, ":")?;
        }

        // 写入 ::（如果压缩段在开头，前面没有冒号，需要两个；否则前面已经有一个了）
        if longest_zero_start == 0 {
            write!(f, "::")?;
        } else {
            write!(f, ":")?;
        }

        for (i, &seg) in segments.iter().enumerate().skip(longest_zero_start + longest_zero_len) {
            write!(f, "{:x}", seg)?;
            if i < 7 { write!(f, ":")?; }
        }

        Ok(())
    }
}

impl FromStr for Ipv6Addr {
    type Err = AddrError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // 处理压缩的 ::
        let parts: Vec<&str> = s.split("::").collect();

        let segments: Vec<u16> = if parts.len() == 1 {
            // 没有 :: 的情况
            s.split(':')
                .map(|p| u16::from_str_radix(p, 16)
                    .map_err(|_| AddrError::InvalidIpAddr(s.to_string())))
                .collect::<Result<Vec<_>, _>>()?
        } else if parts.len() == 2 {
            // 有 :: 的情况
            let left: Vec<u16> = if parts[0].is_empty() {
                Vec::new()
            } else {
                parts[0].split(':')
                    .map(|p| u16::from_str_radix(p, 16)
                        .map_err(|_| AddrError::InvalidIpAddr(s.to_string())))
                    .collect::<Result<Vec<_>, _>>()?
            };

            let right: Vec<u16> = if parts[1].is_empty() {
                Vec::new()
            } else {
                parts[1].split(':')
                    .map(|p| u16::from_str_radix(p, 16)
                        .map_err(|_| AddrError::InvalidIpAddr(s.to_string())))
                    .collect::<Result<Vec<_>, _>>()?
            };

            // 计算需要填充的零段数量
            let zeros_needed = 8 - left.len() - right.len();
            if zeros_needed > 8 {
                return Err(AddrError::InvalidIpAddr(s.to_string()));
            }

            // 合并：左段 + 零段 + 右段
            left.iter().copied()
                .chain(std::iter::repeat_n(0, zeros_needed))
                .chain(right.iter().copied())
                .collect()
        } else {
            return Err(AddrError::InvalidIpAddr(s.to_string()));
        };

        if segments.len() != 8 {
            return Err(AddrError::InvalidIpAddr(s.to_string()));
        }

        let mut bytes = [0u8; 16];
        for (i, segment) in segments.iter().enumerate() {
            bytes[i * 2] = (segment >> 8) as u8;
            bytes[i * 2 + 1] = (*segment & 0xFF) as u8;
        }

        Ok(Ipv6Addr::from_bytes(bytes))
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

// ========== IP 地址枚举 ==========

/// IP 地址枚举（IPv4 或 IPv6）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum IpAddr {
    /// IPv4 地址
    V4(Ipv4Addr),
    /// IPv6 地址
    V6(Ipv6Addr),
}

impl IpAddr {
    /// 判断是否为 IPv4 地址
    pub fn is_ipv4(&self) -> bool {
        matches!(self, IpAddr::V4(_))
    }

    /// 判断是否为 IPv6 地址
    pub fn is_ipv6(&self) -> bool {
        matches!(self, IpAddr::V6(_))
    }

    /// 判断是否为未指定地址
    pub fn is_unspecified(&self) -> bool {
        match self {
            IpAddr::V4(addr) => addr.is_unspecified(),
            IpAddr::V6(addr) => addr.is_unspecified(),
        }
    }

    /// 判断是否为环回地址
    pub fn is_loopback(&self) -> bool {
        match self {
            IpAddr::V4(addr) => addr.is_loopback(),
            IpAddr::V6(addr) => addr.is_loopback(),
        }
    }
}

impl fmt::Display for IpAddr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            IpAddr::V4(addr) => write!(f, "{}", addr),
            IpAddr::V6(addr) => write!(f, "{}", addr),
        }
    }
}

impl From<Ipv4Addr> for IpAddr {
    fn from(addr: Ipv4Addr) -> Self {
        IpAddr::V4(addr)
    }
}

impl From<Ipv6Addr> for IpAddr {
    fn from(addr: Ipv6Addr) -> Self {
        IpAddr::V6(addr)
    }
}
