# 错误处理设计

## 1. 概述

CoreNet采用分层错误处理策略，每层定义自己的错误类型，最终统一转换为`CoreNetError`。使用Rust的标准错误处理机制，手动实现`Error` trait。

## 2. 错误类型层次

```
CoreNetError (顶层错误)
├── QueueError (队列错误)
├── InterfaceError (接口错误)
├── EthernetError (以太网层错误)
├── IpError (IP层错误)
├── TransportError (传输层错误)
└── SocketError (Socket错误)
```

## 3. 顶层错误定义

```rust
use std::fmt;

/// CoreNet统一错误类型
#[derive(Debug)]
pub enum CoreNetError {
    /// 队列错误
    Queue(QueueError),

    /// 网络接口错误
    Interface(InterfaceError),

    /// 以太网层错误
    Ethernet(EthernetError),

    /// IP层错误
    Ip(IpError),

    /// 传输层错误
    Transport(TransportError),

    /// Socket错误
    Socket(SocketError),

    /// 通用IO错误
    Io(io::Error),

    /// 解析错误
    Parse(ParseError),
}

impl fmt::Display for CoreNetError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CoreNetError::Queue(e) => write!(f, "Queue error: {}", e),
            CoreNetError::Interface(e) => write!(f, "Interface error: {}", e),
            CoreNetError::Ethernet(e) => write!(f, "Ethernet error: {}", e),
            CoreNetError::Ip(e) => write!(f, "IP error: {}", e),
            CoreNetError::Transport(e) => write!(f, "Transport error: {}", e),
            CoreNetError::Socket(e) => write!(f, "Socket error: {}", e),
            CoreNetError::Io(e) => write!(f, "IO error: {}", e),
            CoreNetError::Parse(e) => write!(f, "Parse error: {}", e),
        }
    }
}

impl std::error::Error for CoreNetError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            CoreNetError::Queue(e) => Some(e),
            CoreNetError::Interface(e) => Some(e),
            CoreNetError::Ethernet(e) => Some(e),
            CoreNetError::Ip(e) => Some(e),
            CoreNetError::Transport(e) => Some(e),
            CoreNetError::Socket(e) => Some(e),
            CoreNetError::Io(e) => Some(e),
            CoreNetError::Parse(e) => Some(e),
        }
    }
}

// From trait实现，方便错误转换
impl From<io::Error> for CoreNetError {
    fn from(err: io::Error) -> Self {
        CoreNetError::Io(err)
    }
}

impl From<QueueError> for CoreNetError {
    fn from(err: QueueError) -> Self {
        CoreNetError::Queue(err)
    }
}

impl From<InterfaceError> for CoreNetError {
    fn from(err: InterfaceError) -> Self {
        CoreNetError::Interface(err)
    }
}

// ... 其他From实现
```

## 4. 各层错误定义

### 4.1 队列错误

```rust
#[derive(Debug, Clone, PartialEq)]
pub enum QueueError {
    Full,
    Empty,
    Closed,
}

impl fmt::Display for QueueError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            QueueError::Full => write!(f, "Queue is full"),
            QueueError::Empty => write!(f, "Queue is empty"),
            QueueError::Closed => write!(f, "Queue is closed"),
        }
    }
}

impl std::error::Error for QueueError {}
```

### 4.2 接口错误

```rust
#[derive(Debug)]
pub enum InterfaceError {
    DeviceOpenFailed(String),
    IoctlFailed(String),
    WouldBlock,
    InvalidPacket,
    NotConfigured,
    InterfaceNotFound,
    BufferTooSmall,
}

impl fmt::Display for InterfaceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            InterfaceError::DeviceOpenFailed(msg) => {
                write!(f, "Failed to open device: {}", msg)
            }
            InterfaceError::IoctlFailed(msg) => {
                write!(f, "Ioctl failed: {}", msg)
            }
            InterfaceError::WouldBlock => {
                write!(f, "Operation would block")
            }
            InterfaceError::InvalidPacket => {
                write!(f, "Invalid packet")
            }
            InterfaceError::NotConfigured => {
                write!(f, "Interface not configured")
            }
            InterfaceError::InterfaceNotFound => {
                write!(f, "Interface not found")
            }
            InterfaceError::BufferTooSmall => {
                write!(f, "Buffer too small")
            }
        }
    }
}

impl std::error::Error for InterfaceError {}
```

### 4.3 以太网层错误

```rust
#[derive(Debug)]
pub enum EthernetError {
    InvalidHeader,
    InvalidLength,
    UnsupportedEtherType(u16),
    ChecksumError,
}

impl fmt::Display for EthernetError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EthernetError::InvalidHeader => {
                write!(f, "Invalid Ethernet header")
            }
            EthernetError::InvalidLength => {
                write!(f, "Invalid Ethernet frame length")
            }
            EthernetError::UnsupportedEtherType(ty) => {
                write!(f, "Unsupported EtherType: 0x{:04X}", ty)
            }
            EthernetError::ChecksumError => {
                write!(f, "Ethernet checksum error")
            }
        }
    }
}

impl std::error::Error for EthernetError {}

impl From<EthernetError> for CoreNetError {
    fn from(err: EthernetError) -> Self {
        CoreNetError::Ethernet(err)
    }
}
```

### 4.4 IP层错误

```rust
#[derive(Debug)]
pub enum IpError {
    InvalidHeader,
    InvalidLength,
    ChecksumError,
    UnsupportedVersion(u8),
    FragmentationFailed,
    NoRouteToHost,
    PacketTooSmall,
}

impl fmt::Display for IpError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            IpError::InvalidHeader => write!(f, "Invalid IP header"),
            IpError::InvalidLength => write!(f, "Invalid IP packet length"),
            IpError::ChecksumError => write!(f, "IP checksum error"),
            IpError::UnsupportedVersion(v) => {
                write!(f, "Unsupported IP version: {}", v)
            }
            IpError::FragmentationFailed => {
                write!(f, "Fragmentation failed")
            }
            IpError::NoRouteToHost => {
                write!(f, "No route to host")
            }
            IpError::PacketTooSmall => {
                write!(f, "Packet too small")
            }
        }
    }
}

impl std::error::Error for IpError {}

impl From<IpError> for CoreNetError {
    fn from(err: IpError) -> Self {
        CoreNetError::Ip(err)
    }
}
```

### 4.5 传输层错误

```rust
#[derive(Debug)]
pub enum TransportError {
    InvalidHeader,
    InvalidPort,
    ChecksumError,
    ConnectionNotFound,
    ConnectionReset,
    ConnectionTimedOut,
}

impl fmt::Display for TransportError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TransportError::InvalidHeader => {
                write!(f, "Invalid transport header")
            }
            TransportError::InvalidPort => {
                write!(f, "Invalid port number")
            }
            TransportError::ChecksumError => {
                write!(f, "Transport checksum error")
            }
            TransportError::ConnectionNotFound => {
                write!(f, "Connection not found")
            }
            TransportError::ConnectionReset => {
                write!(f, "Connection reset")
            }
            TransportError::ConnectionTimedOut => {
                write!(f, "Connection timed out")
            }
        }
    }
}

impl std::error::Error for TransportError {}

impl From<TransportError> for CoreNetError {
    fn from(err: TransportError) -> Self {
        CoreNetError::Transport(err)
    }
}
```

### 4.6 解析错误

```rust
#[derive(Debug)]
pub enum ParseError {
    InvalidFormat(String),
    UnexpectedEndOfData,
    InvalidValue(String),
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParseError::InvalidFormat(msg) => {
                write!(f, "Invalid format: {}", msg)
            }
            ParseError::UnexpectedEndOfData => {
                write!(f, "Unexpected end of data")
            }
            ParseError::InvalidValue(msg) => {
                write!(f, "Invalid value: {}", msg)
            }
        }
    }
}

impl std::error::Error for ParseError {}

impl From<ParseError> for CoreNetError {
    fn from(err: ParseError) -> Self {
        CoreNetError::Parse(err)
    }
}
```

## 5. Result类型别名

```rust
/// 通用Result类型
pub type Result<T> = std::result::Result<T, CoreNetError>;

/// 队列Result
pub type QueueResult<T> = std::result::Result<T, QueueError>;

/// 接口Result
pub type InterfaceResult<T> = std::result::Result<T, InterfaceError>;

/// 以太网Result
pub type EthernetResult<T> = std::result::Result<T, EthernetError>;

/// IP Result
pub type IpResult<T> = std::result::Result<T, IpError>;

/// 传输层Result
pub type TransportResult<T> = std::result::Result<T, TransportError>;
```

## 6. 错误处理最佳实践

### 6.1 在协议层使用特定错误

```rust
// 在协议层内部使用特定错误类型
fn parse_ipv4_header(packet: &mut Packet) -> IpResult<Ipv4Header> {
    if packet.remaining() < 20 {
        return Err(IpError::PacketTooSmall);
    }
    // ...
    Ok(header)
}

// 自动转换为CoreNetError
fn process_packet(packet: &mut Packet) -> Result<()> {
    let header = parse_ipv4_header(packet)?;  // 自动转换
    // ...
    Ok(())
}
```

### 6.2 错误上下文

```rust
// 提供更多上下文信息
fn send_packet(iface: &dyn NetworkInterface, packet: &Packet) -> Result<()> {
    iface.send(packet).map_err(|e| {
        CoreNetError::Interface(InterfaceError::DeviceOpenFailed(
            format!("Failed to send packet on {}: {}", iface.name(), e)
        ))
    })?;
    Ok(())
}
```

### 6.3 错误日志

```rust
// 简单的日志宏
macro_rules! log_error {
    ($($arg:tt)*) => {
        eprintln!("[ERROR] {}", format!($($arg)*));
    };
}

fn handle_error(err: &CoreNetError) {
    match err {
        CoreNetError::Queue(QueueError::Full) => {
            log_error!("RX queue full, packet dropped");
        }
        CoreNetError::Ip(IpError::ChecksumError) => {
            log_error!("IP checksum error, packet dropped");
        }
        _ => {
            log_error!("Error: {}", err);
        }
    }
}
```
