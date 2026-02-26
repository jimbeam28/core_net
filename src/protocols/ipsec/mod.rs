// src/protocols/ipsec/mod.rs
//
// IPsec (IP Security) 协议实现
// 包含 AH (Authentication Header) 和 ESP (Encapsulating Security Payload) 协议

pub mod ah;
pub mod esp;
pub mod sa;
pub mod ikev2;

// IPsec 协议号
pub const IP_PROTO_AH: u8 = 51;   // Authentication Header
pub const IP_PROTO_ESP: u8 = 50;  // Encapsulating Security Payload

// 默认配置
pub const DEFAULT_REPLAY_WINDOW_SIZE: usize = 64;
pub const DEFAULT_ICV_SIZE: usize = 12;  // HMAC-SHA1-96
pub const ESP_PAD_ALIGN: usize = 4;     // ESP 填充对齐（字节）

// 重新导出主要类型
pub use ah::{AhHeader, AhPacket, AH_HEADER_MIN_LEN};
pub use esp::{EspHeader, EspTrailer, EspPacket, ESP_HEADER_MIN_LEN};
pub use sa::{
    SecurityAssociation, SaDirection, IpsecMode, IpsecProtocol,
    SecurityPolicy, PolicyAction, TrafficSelector,
    CipherTransform, AuthTransform,
    SadEntry, SpdEntry,
    SadManager, SpdManager,
    ReplayWindow,
    SaState,
    SaConfig,
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

/// 恒定时间比较 - 防止时序攻击
///
/// 用于比较密码学敏感数据（如 ICV、HMAC），避免通过计时分析泄露信息
pub fn constant_time_compare(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }

    let mut result = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        result |= x ^ y;
    }
    result == 0
}

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

/// IPsec 模式处理辅助函数
pub mod mode {
    use super::*;

    /// 处理传输模式解封装
    ///
    /// 传输模式：IPsec 头位于原始 IP 头和上层协议之间
    /// 解封装后需要将上层协议提交给原始 IP 头处理
    ///
    /// # 参数
    /// - `ipsec_data`: IPsec 载荷（解密后的上层协议数据）
    /// - `ip_hdr`: 原始 IP 头
    ///
    /// # 返回
    /// 上层协议号和载荷数据
    pub fn decapsulate_transport(
        ipsec_data: &[u8],
        _ip_hdr: &crate::protocols::ip::Ipv4Header,
    ) -> IpsecResult<(u8, Vec<u8>)> {
        // 传输模式下，IPsec 载荷的第一个字节就是上层协议号
        if ipsec_data.is_empty() {
            return Err(IpsecError::InvalidLength);
        }
        let next_header = ipsec_data[0];
        let payload = ipsec_data.to_vec();
        Ok((next_header, payload))
    }

    /// 处理隧道模式解封装
    ///
    /// 隧道模式：整个原始 IP 包被封装
    /// 解封装后得到完整的内层 IP 包
    ///
    /// # 参数
    /// - `ipsec_data`: IPsec 载荷（解密后的内层 IP 包）
    ///
    /// # 返回
    /// 内层 IP 包的完整字节数据
    pub fn decapsulate_tunnel(ipsec_data: &[u8]) -> IpsecResult<Vec<u8>> {
        // 隧道模式下，IPsec 载荷就是完整的内层 IP 包
        if ipsec_data.is_empty() {
            return Err(IpsecError::InvalidLength);
        }

        // 验证这是有效的 IP 包（IP 版本 4）
        let first_byte = ipsec_data[0];
        if first_byte >> 4 != 4 {
            return Err(IpsecError::ParseError("不是有效的 IPv4 包".to_string()));
        }

        Ok(ipsec_data.to_vec())
    }

    /// 传输模式封装
    ///
    /// 将上层协议数据封装为 IPsec 载荷
    ///
    /// # 参数
    /// - `payload`: 上层协议数据（包含协议头）
    /// - `next_header`: 上层协议号
    pub fn encapsulate_transport(payload: &[u8], next_header: u8) -> Vec<u8> {
        let mut result = Vec::with_capacity(1 + payload.len());
        result.push(next_header);
        result.extend_from_slice(payload);
        result
    }

    /// 隧道模式封装
    ///
    /// 将完整的 IP 包封装为 IPsec 载荷
    ///
    /// # 参数
    /// - `inner_packet`: 内层 IP 包（完整字节数据）
    pub fn encapsulate_tunnel(inner_packet: &[u8]) -> Vec<u8> {
        inner_packet.to_vec()
    }

    /// 创建 ESP 封装包
    ///
    /// # 参数
    /// - `sa`: 安全关联（可变引用，需要更新序列号）
    /// - `payload`: 原始载荷数据
    /// - `next_header`: 上层协议号
    pub fn create_esp_packet(
        sa: &mut super::SecurityAssociation,
        payload: Vec<u8>,
        next_header: u8,
    ) -> Result<super::esp::EspPacket, IpsecError> {
        let block_size = sa.cipher.as_ref().map_or(1, |c| c.block_size());
        let cipher = sa.cipher.clone();
        let key = sa.cipher_key.clone();
        let seq = sa.next_sequence() as u32;

        let packet = super::esp::EspPacket::create_encrypted(
            sa.spi,
            seq,
            payload,
            next_header,
            block_size,
            cipher.as_ref(),
            key.as_deref().unwrap_or(&[]),
        );

        Ok(packet)
    }

    /// 创建 AH 封装包
    ///
    /// # 参数
    /// - `sa`: 安全关联（可变引用，需要更新序列号）
    /// - `payload`: 原始载荷数据（包含上层协议头）
    /// - `next_header`: 上层协议号
    pub fn create_ah_packet(
        sa: &mut super::SecurityAssociation,
        payload: Vec<u8>,
        next_header: u8,
    ) -> Result<super::ah::AhPacket, IpsecError> {
        let icv_len = sa.auth.icv_size();
        let seq = sa.next_sequence() as u32;

        // 创建 AH 头
        let header = super::ah::AhHeader::new(
            next_header,
            sa.spi,
            seq,
            icv_len,
        );

        // 计算 ICV（简化实现，实际应该计算完整包的 ICV）
        let icv = super::ah::AhPacket::compute_icv(&payload, &sa.auth_key, icv_len);

        Ok(super::ah::AhPacket {
            header,
            icv,
            payload,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocols::ip::Ipv4Header;

    #[test]
    fn test_constant_time_compare() {
        // 相同数据
        assert!(constant_time_compare(b"hello", b"hello"));

        // 不同数据
        assert!(!constant_time_compare(b"hello", b"world"));

        // 不同长度
        assert!(!constant_time_compare(b"hello", b"hello!"));

        // 空数据
        assert!(constant_time_compare(b"", b""));
    }

    #[test]
    fn test_mode_transport_encapsulate() {
        let payload = b"TCP data";
        let next_header = 6; // TCP

        let encapsulated = mode::encapsulate_transport(payload, next_header);

        assert_eq!(encapsulated[0], next_header);
        assert_eq!(&encapsulated[1..], payload);
    }

    #[test]
    fn test_mode_transport_decapsulate() {
        let payload = b"TCP data";
        let mut encapsulated = Vec::new();
        encapsulated.push(6); // TCP
        encapsulated.extend_from_slice(payload);

        let (next_header, decapsulated) = mode::decapsulate_transport(&encapsulated, &Ipv4Header::new(
            crate::protocols::Ipv4Addr::new(192, 168, 1, 1),
            crate::protocols::Ipv4Addr::new(192, 168, 1, 2),
            6,
            payload.len() + 20,
        )).unwrap();

        assert_eq!(next_header, 6);
        assert_eq!(decapsulated, encapsulated);
    }

    #[test]
    fn test_mode_tunnel_encapsulate() {
        let inner_packet = vec![
            0x45, 0x00, 0x00, 0x1c, // Version=4, IHL=5, Total Length=28
            0x00, 0x01, 0x00, 0x00, // ID=1, Flags=0
            0x40, 0x06, 0x00, 0x00, // TTL=64, Protocol=6 (TCP), Checksum=0
            0xc0, 0xa8, 0x01, 0x01, // Source=192.168.1.1
            0xc0, 0xa8, 0x01, 0x02, // Dest=192.168.1.2
        ];

        let encapsulated = mode::encapsulate_tunnel(&inner_packet);

        assert_eq!(encapsulated, inner_packet);
    }

    #[test]
    fn test_mode_tunnel_decapsulate() {
        let inner_packet = vec![
            0x45, 0x00, 0x00, 0x1c, // Version=4, IHL=5, Total Length=28
            0x00, 0x01, 0x00, 0x00, // ID=1, Flags=0
            0x40, 0x06, 0x00, 0x00, // TTL=64, Protocol=6 (TCP), Checksum=0
            0xc0, 0xa8, 0x01, 0x01, // Source=192.168.1.1
            0xc0, 0xa8, 0x01, 0x02, // Dest=192.168.1.2
        ];

        let decapsulated = mode::decapsulate_tunnel(&inner_packet).unwrap();

        assert_eq!(decapsulated, inner_packet);
    }

    #[test]
    fn test_mode_tunnel_invalid_ip() {
        let invalid_packet = vec![0x00, 0x00, 0x00, 0x00]; // IP version 0

        let result = mode::decapsulate_tunnel(&invalid_packet);
        assert!(result.is_err());
    }
}
