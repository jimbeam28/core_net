// src/protocols/ipsec/ikev2/mod.rs
//
// IKEv2 (Internet Key Exchange Version 2) 协议实现
// RFC 7296: Internet Key Exchange Protocol Version 2 (IKEv2)

// ⚠️ 警告：这是简化的教学实现
// - 加密和认证使用简化的操作，不提供真实安全性
// - 仅用于学习 IKEv2 协议原理
// - 禁止在生产环境中使用
// - 生产环境应使用标准加密库（如 RustCrypto 的 aes-gcm、hmac、sha2 等）

pub mod message;
pub mod payload;
pub mod sa;
pub mod crypto;
pub mod state;
pub mod processor;

use crate::common::addr::IpAddr;

// ========== IKEv2 常量 ==========

/// IKEv2 标准端口
pub const IKEV2_PORT: u16 = 500;

/// NAT 穿透端口
pub const IKEV2_NAT_PORT: u16 = 4500;

/// IKEv2 协议版本 (2.0)
pub const IKEV2_VERSION: u8 = 0x20;

/// IKE 消息头部最小长度
pub const IKE_HEADER_LEN: usize = 28;

/// SPI 长度
pub const SPI_LEN: usize = 8;

// ========== 交换类型 ==========

/// IKEv2 交换类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum IkeExchangeType {
    /// IKE_SA_INIT
    IkeSaInit = 34,
    /// IKE_AUTH
    IkeAuth = 35,
    /// CREATE_CHILD_SA
    CreateChildSa = 36,
    /// INFORMATIONAL
    Informational = 37,
}

impl IkeExchangeType {
    /// 从 u8 创建交换类型
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            34 => Some(Self::IkeSaInit),
            35 => Some(Self::IkeAuth),
            36 => Some(Self::CreateChildSa),
            37 => Some(Self::Informational),
            _ => None,
        }
    }

    /// 转换为 u8
    pub fn as_u8(self) -> u8 {
        self as u8
    }
}

// ========== IKEv2 标志位 ==========

/// IKEv2 消息标志位
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct IkeFlags {
    /// 发起方标志 (Bit 3)
    pub initiator: bool,
    /// 版本标志 (Bit 4) - IKEv2 中必须为 0
    pub version: bool,
    /// 响应标志 (Bit 5)
    pub response: bool,
}

impl IkeFlags {
    /// 从 u8 创建标志位
    pub fn from_u8(value: u8) -> Self {
        Self {
            initiator: (value & 0x08) != 0,
            version: (value & 0x10) != 0,
            response: (value & 0x20) != 0,
        }
    }

    /// 转换为 u8
    pub fn as_u8(self) -> u8 {
        let mut flags = 0u8;
        if self.initiator {
            flags |= 0x08;
        }
        if self.version {
            flags |= 0x10;
        }
        if self.response {
            flags |= 0x20;
        }
        flags
    }

    /// 创建请求消息标志
    pub fn request(is_initiator: bool) -> Self {
        Self {
            initiator: is_initiator,
            version: false,
            response: false,
        }
    }

    /// 创建响应消息标志
    pub fn response(is_initiator: bool) -> Self {
        Self {
            initiator: is_initiator,
            version: false,
            response: true,
        }
    }
}

// ========== Payload 类型 ==========

/// IKEv2 Payload 类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum IkePayloadType {
    None = 0,
    SA = 33,
    KE = 34,
    IDi = 35,
    IDr = 36,
    CERT = 37,
    CERTREQ = 38,
    AUTH = 39,
    Nonce = 40,  // Ni 和 Nr 使用相同的类型码
    Notify = 41,
    Delete = 42,
    Vendor = 43,
    TSi = 44,
    TSr = 45,
}

impl IkePayloadType {
    /// 从 u8 创建 Payload 类型
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0 => Some(Self::None),
            33 => Some(Self::SA),
            34 => Some(Self::KE),
            35 => Some(Self::IDi),
            36 => Some(Self::IDr),
            37 => Some(Self::CERT),
            38 => Some(Self::CERTREQ),
            39 => Some(Self::AUTH),
            40 => Some(Self::Nonce), // Ni 和 Nr 共享相同的类型码
            41 => Some(Self::Notify),
            42 => Some(Self::Delete),
            43 => Some(Self::Vendor),
            44 => Some(Self::TSi),
            45 => Some(Self::TSr),
            _ => None,
        }
    }

    /// 转换为 u8
    pub fn as_u8(self) -> u8 {
        self as u8
    }
}

// ========== 协议 ID ==========

/// IKEv2 协议 ID
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum IkeProtocolId {
    /// IKE
    Ike = 1,
    /// AH
    Ah = 2,
    /// ESP
    Esp = 3,
}

impl IkeProtocolId {
    /// 从 u8 创建协议 ID
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            1 => Some(Self::Ike),
            2 => Some(Self::Ah),
            3 => Some(Self::Esp),
            _ => None,
        }
    }

    /// 转换为 u8
    pub fn as_u8(self) -> u8 {
        self as u8
    }
}

// ========== Transform 类型 ==========

/// IKEv2 Transform 类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum IkeTransformType {
    /// 加密算法
    Encryption = 1,
    /// PRF
    Prf = 2,
    /// 完整性算法
    Integrity = 3,
    /// DH 组
    DhGroup = 4,
    /// 扩展序列号
    ESN = 5,
}

impl IkeTransformType {
    /// 从 u8 创建 Transform 类型
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            1 => Some(Self::Encryption),
            2 => Some(Self::Prf),
            3 => Some(Self::Integrity),
            4 => Some(Self::DhGroup),
            5 => Some(Self::ESN),
            _ => None,
        }
    }

    /// 转换为 u8
    pub fn as_u8(self) -> u8 {
        self as u8
    }
}

// ========== ID 类型 ==========

/// IKEv2 ID 类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum IkeIdType {
    /// IPv4 地址
    IdIpv4Addr = 1,
    /// FQDN
    IdFqdn = 2,
    /// RFC822 邮箱
    IdRfc822Addr = 3,
    /// IPv6 地址
    IdIpv6Addr = 5,
    /// DER ASN1 DN
    IdDerAsn1Dn = 9,
    /// DER ASN1 GN
    IdDerAsn1Gn = 10,
}

impl IkeIdType {
    /// 从 u8 创建 ID 类型
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            1 => Some(Self::IdIpv4Addr),
            2 => Some(Self::IdFqdn),
            3 => Some(Self::IdRfc822Addr),
            5 => Some(Self::IdIpv6Addr),
            9 => Some(Self::IdDerAsn1Dn),
            10 => Some(Self::IdDerAsn1Gn),
            _ => None,
        }
    }

    /// 转换为 u8
    pub fn as_u8(self) -> u8 {
        self as u8
    }
}

// ========== 认证方法 ==========

/// IKEv2 认证方法
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum IkeAuthMethod {
    /// 预共享密钥
    SharedKey = 1,
    /// RSA 签名
    RsaSig = 2,
    /// DSA 签名
    DssSig = 3,
    /// ECDSA 签名 (SHA-256)
    EcdsaSha256 = 9,
    /// ECDSA 签名 (SHA-384)
    EcdsaSha384 = 10,
    /// ECDSA 签名 (SHA-512)
    EcdsaSha512 = 11,
}

impl IkeAuthMethod {
    /// 从 u8 创建认证方法
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            1 => Some(Self::SharedKey),
            2 => Some(Self::RsaSig),
            3 => Some(Self::DssSig),
            9 => Some(Self::EcdsaSha256),
            10 => Some(Self::EcdsaSha384),
            11 => Some(Self::EcdsaSha512),
            _ => None,
        }
    }

    /// 转换为 u8
    pub fn as_u8(self) -> u8 {
        self as u8
    }
}

// ========== DH 组 ==========

/// IKEv2 DH 组
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum IkeDhGroup {
    /// 2048-bit MODP Group (RFC 3526)
    MODP2048 = 14,
    /// 3072-bit MODP Group (RFC 3526)
    MODP3072 = 15,
    /// 4096-bit MODP Group (RFC 3526)
    MODP4096 = 16,
    /// 256-bit ECP Group (RFC 5903)
    ECP256 = 19,
    /// 384-bit ECP Group (RFC 5903)
    ECP384 = 20,
}

impl IkeDhGroup {
    /// 从 u16 创建 DH 组
    pub fn from_u16(value: u16) -> Option<Self> {
        match value {
            14 => Some(Self::MODP2048),
            15 => Some(Self::MODP3072),
            16 => Some(Self::MODP4096),
            19 => Some(Self::ECP256),
            20 => Some(Self::ECP384),
            _ => None,
        }
    }

    /// 转换为 u16
    pub fn as_u16(self) -> u16 {
        self as u16
    }
}

// ========== IKEv2 错误类型 ==========

/// IKEv2 错误类型
#[derive(Debug, Clone, PartialEq)]
pub enum IkeError {
    /// 解析错误
    ParseError(String),
    /// 无效的消息长度
    InvalidLength,
    /// 无效的 SPI
    InvalidSpi,
    /// 不支持的交换类型
    UnsupportedExchangeType(u8),
    /// 不支持的 Payload 类型
    UnsupportedPayloadType(u8),
    /// 认证失败
    AuthenticationFailed,
    /// SA 不存在
    SaNotFound,
    /// SA 状态错误
    SaStateError(String),
    /// 加密错误
    CryptoError(String),
    /// DH 计算错误
    DhError(String),
    /// 重放检测
    ReplayDetected,
    /// 超时
    Timeout,
    /// 其他错误
    Other(String),
}

impl std::fmt::Display for IkeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IkeError::ParseError(msg) => write!(f, "解析错误: {}", msg),
            IkeError::InvalidLength => write!(f, "无效的消息长度"),
            IkeError::InvalidSpi => write!(f, "无效的 SPI"),
            IkeError::UnsupportedExchangeType(t) => write!(f, "不支持的交换类型: {}", t),
            IkeError::UnsupportedPayloadType(t) => write!(f, "不支持的 Payload 类型: {}", t),
            IkeError::AuthenticationFailed => write!(f, "认证失败"),
            IkeError::SaNotFound => write!(f, "SA 不存在"),
            IkeError::SaStateError(msg) => write!(f, "SA 状态错误: {}", msg),
            IkeError::CryptoError(msg) => write!(f, "加密错误: {}", msg),
            IkeError::DhError(msg) => write!(f, "DH 计算错误: {}", msg),
            IkeError::ReplayDetected => write!(f, "检测到重放"),
            IkeError::Timeout => write!(f, "超时"),
            IkeError::Other(msg) => write!(f, "其他错误: {}", msg),
        }
    }
}

impl std::error::Error for IkeError {}

/// IKEv2 Result 类型
pub type IkeResult<T> = Result<T, IkeError>;

// ========== 重新导出主要类型 ==========

pub use message::{IkeHeader, IkeMessage};
pub use payload::{
    IkePayload, IkePayloadHeader,
    IkeSaPayload, IkeProposal, IkeTransform,
    IkeKePayload, IkeIdPayload, IkeAuthPayload,
    IkeNoncePayload, IkeNotifyPayload, IkeDeletePayload,
    IkeTsPayload, TrafficSelector, TsType,
};
pub use sa::{
    IkeSaEntry, IkeSaId, IkeSaState, IkeRole,
    IkeKeyMaterial, IkeSaConfig,
    IkeSaManager,
};
pub use crypto::{
    IkeCrypto, IkePseudoRandomFunction,
    generate_random_spi, generate_random_nonce,
    compute_dh_shared, compute_key_material,
};
pub use state::{
    IkeStateMachine, IkeInitiatorState, IkeResponderState,
};
pub use processor::{IkeProcessor};

// ========== IPsec 模块集成 ==========

impl From<IkeError> for super::IpsecError {
    fn from(err: IkeError) -> Self {
        match err {
            IkeError::ParseError(msg) => super::IpsecError::ParseError(msg),
            IkeError::InvalidLength => super::IpsecError::InvalidLength,
            IkeError::AuthenticationFailed => super::IpsecError::AuthError("认证失败".to_string()),
            IkeError::CryptoError(msg) => super::IpsecError::CryptoError(msg),
            _ => super::IpsecError::Other(format!("IKEv2: {:?}", err)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exchange_type_conversion() {
        assert_eq!(IkeExchangeType::from_u8(34), Some(IkeExchangeType::IkeSaInit));
        assert_eq!(IkeExchangeType::IkeSaInit.as_u8(), 34);
        assert_eq!(IkeExchangeType::from_u8(99), None);
    }

    #[test]
    fn test_payload_type_conversion() {
        assert_eq!(IkePayloadType::from_u8(33), Some(IkePayloadType::SA));
        assert_eq!(IkePayloadType::SA.as_u8(), 33);
        assert_eq!(IkePayloadType::from_u8(99), None);
    }

    #[test]
    fn test_flags() {
        let flags = IkeFlags::request(true);
        assert!(flags.initiator);
        assert!(!flags.response);

        let value = flags.as_u8();
        let decoded = IkeFlags::from_u8(value);
        assert_eq!(decoded.initiator, flags.initiator);
        assert_eq!(decoded.response, flags.response);
    }

    #[test]
    fn test_dh_group_conversion() {
        assert_eq!(IkeDhGroup::from_u16(14), Some(IkeDhGroup::MODP2048));
        assert_eq!(IkeDhGroup::MODP2048.as_u16(), 14);
        assert_eq!(IkeDhGroup::from_u16(99), None);
    }

    #[test]
    fn test_auth_method_conversion() {
        assert_eq!(IkeAuthMethod::from_u8(1), Some(IkeAuthMethod::SharedKey));
        assert_eq!(IkeAuthMethod::SharedKey.as_u8(), 1);
        assert_eq!(IkeAuthMethod::from_u8(99), None);
    }

    #[test]
    fn test_constants() {
        assert_eq!(IKEV2_PORT, 500);
        assert_eq!(IKEV2_NAT_PORT, 4500);
        assert_eq!(IKEV2_VERSION, 0x20);
        assert_eq!(IKE_HEADER_LEN, 28);
        assert_eq!(SPI_LEN, 8);
    }
}
