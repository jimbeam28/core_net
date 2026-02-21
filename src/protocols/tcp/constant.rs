// src/protocols/tcp/constant.rs
//
// TCP 协议常量定义

/// TCP 协议号（在 IP 协议字段中的值）
pub const IP_PROTO_TCP: u8 = 6;

/// TCP 头部最小大小（20 字节）
pub const TCP_MIN_HEADER_LEN: usize = 20;

/// TCP 头部最大大小（60 字节）
pub const TCP_MAX_HEADER_LEN: usize = 60;

/// TCP 头部最小数据偏移值（5，表示 20 字节）
pub const TCP_MIN_DATA_OFFSET: u8 = 5;

/// 知名端口号
pub mod well_known_ports {
    /// FTP 数据
    pub const FTP_DATA: u16 = 20;
    /// FTP 控制
    pub const FTP_CONTROL: u16 = 21;
    /// SSH
    pub const SSH: u16 = 22;
    /// Telnet
    pub const TELNET: u16 = 23;
    /// SMTP
    pub const SMTP: u16 = 25;
    /// DNS
    pub const DNS: u16 = 53;
    /// HTTP
    pub const HTTP: u16 = 80;
    /// HTTPS
    pub const HTTPS: u16 = 443;
}

/// TCP 标志位常量
pub mod flags {
    /// FIN 标志位（结束连接）
    pub const FIN: u8 = 0x01;
    /// SYN 标志位（同步序列号）
    pub const SYN: u8 = 0x02;
    /// RST 标志位（重置连接）
    pub const RST: u8 = 0x04;
    /// PSH 标志位（推送数据）
    pub const PSH: u8 = 0x08;
    /// ACK 标志位（确认号有效）
    pub const ACK: u8 = 0x10;
    /// URG 标志位（紧急指针有效）
    pub const URG: u8 = 0x20;
    /// ECE 标志位（ECN-Echo）
    pub const ECE: u8 = 0x40;
    /// CWR 标志位（拥塞窗口减少）
    pub const CWR: u8 = 0x80;
}

/// TCP 选项类型
pub mod option_kind {
    /// 行尾（选项结束）
    pub const END: u8 = 0;
    /// 无操作（填充）
    pub const NOP: u8 = 1;
    /// 最大分段大小（MSS）
    pub const MSS: u8 = 2;
    /// 窗口缩放
    pub const WINDOW_SCALE: u8 = 3;
    /// SACK 允许
    pub const SACK_PERMITTED: u8 = 4;
    /// SACK 选项
    pub const SACK: u8 = 5;
    /// 时间戳
    pub const TIMESTAMPS: u8 = 8;
}
