// src/protocols/ipsec/mod.rs
//
// IPsec (IP Security) 协议实现
// 包含 AH (Authentication Header) 和 ESP (Encapsulating Security Payload) 协议

pub mod ah;
pub mod esp;
pub mod sa;

// IPsec 协议号
pub const IP_PROTO_AH: u8 = 51;   // Authentication Header
pub const IP_PROTO_ESP: u8 = 50;  // Encapsulating Security Payload

// 默认配置
pub const DEFAULT_REPLAY_WINDOW_SIZE: usize = 64;
pub const DEFAULT_ICV_SIZE: usize = 12;  // HMAC-SHA1-96
pub const ESP_PAD_ALIGN: usize = 4;     // ESP 填充对齐（字节）

// 重新导出主要类型
pub use ah::{AhHeader, AhPacket};
pub use esp::{EspHeader, EspTrailer, EspPacket};
pub use sa::{
    SecurityAssociation, SaDirection, IpsecMode, IpsecProtocol,
    SecurityPolicy, PolicyAction, TrafficSelector,
    CipherTransform, AuthTransform,
    SadEntry, SpdEntry,
    SadManager, SpdManager,
    ReplayWindow,
    SaState,
};

/// IPsec 错误类型
#[derive(Debug, Clone, PartialEq)]
pub enum IpsecError {
    /// 解析错误
    ParseError(String),
    /// 无效的 SA
    InvalidSa,
    /// SA 不存在
    SaNotFound,
    /// 无效的 SPI
    InvalidSpi,
    /// ICV 验证失败
    IcvMismatch,
    /// 重放攻击检测
    ReplayDetected,
    /// 加密错误
    CryptoError(String),
    /// 认证错误
    AuthError(String),
    /// 策略不匹配
    PolicyMismatch,
    /// 无效的报文长度
    InvalidLength,
    /// 不支持的加密算法
    UnsupportedCipher,
    /// 不支持的认证算法
    UnsupportedAuth,
    /// 其他错误
    Other(String),
}

impl std::fmt::Display for IpsecError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IpsecError::ParseError(msg) => write!(f, "解析错误: {}", msg),
            IpsecError::InvalidSa => write!(f, "无效的 SA"),
            IpsecError::SaNotFound => write!(f, "SA 不存在"),
            IpsecError::InvalidSpi => write!(f, "无效的 SPI"),
            IpsecError::IcvMismatch => write!(f, "ICV 验证失败"),
            IpsecError::ReplayDetected => write!(f, "检测到重放攻击"),
            IpsecError::CryptoError(msg) => write!(f, "加密错误: {}", msg),
            IpsecError::AuthError(msg) => write!(f, "认证错误: {}", msg),
            IpsecError::PolicyMismatch => write!(f, "策略不匹配"),
            IpsecError::InvalidLength => write!(f, "无效的报文长度"),
            IpsecError::UnsupportedCipher => write!(f, "不支持的加密算法"),
            IpsecError::UnsupportedAuth => write!(f, "不支持的认证算法"),
            IpsecError::Other(msg) => write!(f, "其他错误: {}", msg),
        }
    }
}

impl std::error::Error for IpsecError {}

/// IPsec Result 类型
pub type IpsecResult<T> = Result<T, IpsecError>;

// 从 CoreError 转换
impl From<crate::common::CoreError> for IpsecError {
    fn from(err: crate::common::CoreError) -> Self {
        match err {
            crate::common::CoreError::ParseError(msg) => IpsecError::ParseError(msg),
            crate::common::CoreError::InvalidPacket(msg) => IpsecError::ParseError(msg),
            _ => IpsecError::Other(format!("{:?}", err)),
        }
    }
}
