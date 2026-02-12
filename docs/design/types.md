# 通用类型定义

## 1. 概述

定义网络协议栈中通用的数据类型，包括MAC地址、IP地址、端口号等。

## 2. MAC地址

```rust
/// MAC地址（6字节）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MacAddr {
    pub bytes: [u8; 6],
}

impl MacAddr {
    /// 创建新的MAC地址
    pub const fn new(a: u8, b: u8, c: u8, d: u8, e: u8, f: u8) -> Self;

    /// 从字节数组创建
    pub fn from_bytes(bytes: &[u8; 6]) -> Self;

    /// 广播地址
    pub const BROADCAST: Self = MacAddr { bytes: [0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF] };

    /// 零地址
    pub const ZERO: Self = MacAddr { bytes: [0x00, 0x00, 0x00, 0x00, 0x00, 0x00] };

    /// 是否为广播地址
    pub fn is_broadcast(&self) -> bool;

    /// 是否为多播地址
    pub fn is_multicast(&self) -> bool;

    /// 是否为本地地址
    pub fn is_local(&self) -> bool;
}

impl Display for MacAddr {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{:02X}:{:02X}:{:02X}:{:02X}:{:02X}:{:02X}",
            self.bytes[0], self.bytes[1], self.bytes[2],
            self.bytes[3], self.bytes[4], self.bytes[5])
    }
}

impl FromStr for MacAddr {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Error> {
        // 支持 "XX:XX:XX:XX:XX:XX" 或 "XX-XX-XX-XX-XX-XX" 格式
        // ...
    }
}
```

## 3. IP地址

```rust
/// IP地址枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum IpAddr {
    V4(Ipv4Addr),
    V6(Ipv6Addr),
}

/// IPv4地址（4字节）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Ipv4Addr {
    pub bytes: [u8; 4],
}

/// IPv6地址（16字节）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Ipv6Addr {
    pub bytes: [u8; 16],
}

impl Ipv4Addr {
    /// 创建新的IPv4地址
    pub const fn new(a: u8, b: u8, c: u8, d: u8) -> Self;

    /// 从u32创建
    pub const fn from_u32(addr: u32) -> Self;

    /// 转换为u32（大端序）
    pub const fn to_u32(self) -> u32;

    /// 特殊地址
    pub const UNSPECIFIED: Self = Ipv4Addr { bytes: [0, 0, 0, 0] };
    pub const LOCALHOST: Self = Ipv4Addr { bytes: [127, 0, 0, 1] };
    pub const BROADCAST: Self = Ipv4Addr { bytes: [255, 255, 255, 255] };

    /// 是否为特定网段的地址
    pub fn is_in_network(&self, network: &Self, prefix: u8) -> bool;
}

impl Ipv6Addr {
    /// 创建新的IPv6地址
    pub const fn new(
        a: u16, b: u16, c: u16, d: u16,
        e: u16, f: u16, g: u16, h: u16,
    ) -> Self;

    /// 从16字节数组创建
    pub const fn from_bytes(bytes: &[u8; 16]) -> Self;

    /// 特殊地址
    pub const UNSPECIFIED: Self = Ipv6Addr { bytes: [0; 16] };
    pub const LOCALHOST: Self = Ipv6Addr { bytes: [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1] };

    /// 是否为IPv4映射地址
    pub fn is_ipv4_mapped(&self) -> bool;
}

impl Display for IpAddr {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            IpAddr::V4(addr) => write!(f, "{}", addr),
            IpAddr::V6(addr) => write!(f, "{}", addr),
        }
    }
}

impl FromStr for IpAddr {
    type Err = ParseError;
    // ...
}
```

## 4. 协议类型

```rust
/// 以太网类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum EtherType {
    IPv4 = 0x0800,
    ARP = 0x0806,
    IPv6 = 0x86DD,
    VLAN = 0x8100,
}

impl EtherType {
    /// 从u16解析
    pub fn from_u16(value: u16) -> Option<Self>;

    /// 转换为u16
    pub const fn to_u16(self) -> u16;
}

/// IP协议号
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum IpProtocol {
    ICMP = 1,
    TCP = 6,
    UDP = 17,
    ICMPv6 = 58,
    // ...
}

impl IpProtocol {
    /// 从u8解析
    pub fn from_u8(value: u8) -> Option<Self>;

    /// 转换为u8
    pub const fn to_u8(self) -> u8;
}

/// IP版本
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IpVersion {
    V4 = 4,
    V6 = 6,
}
```

## 5. 端口号

```rust
/// 端口号类型
pub type Port = u16;

/// 知名端口号常量
pub mod well_known_ports {
    use super::Port;

    pub const DNS: Port = 53;
    pub const HTTP: Port = 80;
    pub const HTTPS: Port = 443;
    pub const SSH: Port = 22;
    pub const FTP: Port = 21;
    pub const TELNET: Port = 23;

    // 临时端口范围
    pub const EPHEMERAL_START: Port = 49152;
    pub const EPHEMERAL_END: Port = 65535;
}
```

## 6. 网络掩码和前缀

```rust
/// IPv4网络掩码
pub struct Ipv4Net {
    pub addr: Ipv4Addr,
    pub prefix: u8,
}

impl Ipv4Net {
    pub fn new(addr: Ipv4Addr, prefix: u8) -> Result<Self, NetError>;

    /// 获取网络地址
    pub fn network(&self) -> Ipv4Addr;

    /// 获取广播地址
    pub fn broadcast(&self) -> Ipv4Addr;

    /// 获取掩码
    pub fn netmask(&self) -> Ipv4Addr;

    /// 检查地址是否在网段内
    pub fn contains(&self, addr: &Ipv4Addr) -> bool;
}
```

## 7. 校验和

```rust
/// 计算互联网校验和（RFC 1071）
pub fn checksum(data: &[u8]) -> u16 {
    // 将数据按16位分组求和
    let mut sum: u32 = 0;

    for chunk in data.chunks(2) {
        let word = if chunk.len() == 2 {
            u16::from_be_bytes([chunk[0], chunk[1]]) as u32
        } else {
            (chunk[0] as u32) << 8
        };
        sum = sum.wrapping_add(word);
    }

    // 处理进位
    while sum >> 16 != 0 {
        sum = (sum & 0xFFFF) + (sum >> 16);
    }

    // 取反
    !sum as u16
}

/// 验证校验和
pub fn verify_checksum(data: &[u8]) -> bool {
    checksum(data) == 0
}
```

## 8. 字节序转换

```rust
/// 网络字节序（大端序）转换
pub trait NetEndian {
    fn to_be(self) -> Self;
    fn to_le(self) -> Self;
    fn from_be(v: Self) -> Self;
    fn from_le(v: Self) -> Self;
}

impl NetEndian for u16 {
    fn to_be(self) -> Self { self.to_be() }
    // ...
}

impl NetEndian for u32 {
    // ...
}
```
