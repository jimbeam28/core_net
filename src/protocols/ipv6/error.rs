// src/protocols/ipv6/error.rs
//
// IPv6 协议错误类型

use crate::common::CoreError;

/// IPv6 协议处理错误类型
///
/// 定义了 IPv6 协议处理过程中可能遇到的所有错误情况。
/// 实现 `From<Ipv6Error> for CoreError` 以便与其他模块集成。
#[derive(Debug)]
pub enum Ipv6Error {
    /// 版本号不匹配
    InvalidVersion {
        expected: u8,
        found: u8,
    },

    /// 头部长度无效
    InvalidHeaderLength {
        length: usize,
    },

    /// 数据包长度不足
    PacketTooShort {
        expected: usize,
        found: usize,
    },

    /// 负载长度无效
    InvalidPayloadLength {
        length: u16,
    },

    /// 不支持扩展头部
    ExtensionHeaderNotSupported {
        next_header: u8,
    },

    /// 协议不支持
    UnsupportedProtocol {
        protocol: u8,
    },

    /// 目的地址不可达
    DestinationUnreachable {
        addr: String,
    },

    /// Hop Limit 超时
    HopLimitExceeded {
        hop_limit: u8,
    },

    /// 源地址无效
    InvalidSourceAddress {
        addr: String,
    },

    /// 数据包长度超过 MTU
    PacketTooLarge {
        length: u16,
        mtu: u16,
    },
}

impl Ipv6Error {
    /// 创建版本号错误
    pub fn invalid_version(found: u8) -> Self {
        Ipv6Error::InvalidVersion { expected: 6, found }
    }

    /// 创建头部长度错误
    pub fn invalid_header_length(length: usize) -> Self {
        Ipv6Error::InvalidHeaderLength { length }
    }

    /// 创建数据包长度不足错误
    pub fn packet_too_short(expected: usize, found: usize) -> Self {
        Ipv6Error::PacketTooShort { expected, found }
    }

    /// 创建负载长度无效错误
    pub fn invalid_payload_length(length: u16) -> Self {
        Ipv6Error::InvalidPayloadLength { length }
    }

    /// 创建扩展头不支持错误
    pub fn extension_header_not_supported(next_header: u8) -> Self {
        Ipv6Error::ExtensionHeaderNotSupported { next_header }
    }

    /// 创建协议不支持错误
    pub fn unsupported_protocol(protocol: u8) -> Self {
        Ipv6Error::UnsupportedProtocol { protocol }
    }

    /// 创建目的地址不可达错误
    pub fn destination_unreachable(addr: String) -> Self {
        Ipv6Error::DestinationUnreachable { addr }
    }

    /// 创建 Hop Limit 超时错误
    pub fn hop_limit_exceeded(hop_limit: u8) -> Self {
        Ipv6Error::HopLimitExceeded { hop_limit }
    }

    /// 创建源地址无效错误
    pub fn invalid_source_address(addr: String) -> Self {
        Ipv6Error::InvalidSourceAddress { addr }
    }

    /// 创建数据包过大错误
    pub fn packet_too_large(length: u16, mtu: u16) -> Self {
        Ipv6Error::PacketTooLarge { length, mtu }
    }
}

impl std::fmt::Display for Ipv6Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Ipv6Error::InvalidVersion { expected, found } => {
                write!(f, "IPv6版本无效: 期望={}, 发现={}", expected, found)
            }
            Ipv6Error::InvalidHeaderLength { length } => {
                write!(f, "IPv6头部长度无效: 长度={}", length)
            }
            Ipv6Error::PacketTooShort { expected, found } => {
                write!(f, "IPv6数据包长度不足: 期望={}, 发现={}", expected, found)
            }
            Ipv6Error::InvalidPayloadLength { length } => {
                write!(f, "IPv6负载长度无效: 长度={}", length)
            }
            Ipv6Error::ExtensionHeaderNotSupported { next_header } => {
                write!(f, "IPv6扩展头不支持: NextHeader={}", next_header)
            }
            Ipv6Error::UnsupportedProtocol { protocol } => {
                write!(f, "IPv6协议不支持: Protocol={}", protocol)
            }
            Ipv6Error::DestinationUnreachable { addr } => {
                write!(f, "IPv6目的地址不可达: {}", addr)
            }
            Ipv6Error::HopLimitExceeded { hop_limit } => {
                write!(f, "IPv6 Hop Limit超时: HopLimit={}", hop_limit)
            }
            Ipv6Error::InvalidSourceAddress { addr } => {
                write!(f, "IPv6源地址无效: {}", addr)
            }
            Ipv6Error::PacketTooLarge { length, mtu } => {
                write!(f, "IPv6数据包过大: 长度={}, MTU={}", length, mtu)
            }
        }
    }
}

impl std::error::Error for Ipv6Error {}

// 转换为 CoreError
impl From<Ipv6Error> for CoreError {
    fn from(err: Ipv6Error) -> Self {
        match err {
            Ipv6Error::InvalidVersion { .. } | Ipv6Error::InvalidHeaderLength { .. } => {
                CoreError::InvalidPacket(err.to_string())
            }
            Ipv6Error::PacketTooShort { .. } => {
                CoreError::InvalidPacket(err.to_string())
            }
            Ipv6Error::InvalidPayloadLength { .. } => {
                CoreError::InvalidPacket(err.to_string())
            }
            Ipv6Error::ExtensionHeaderNotSupported { .. } => {
                CoreError::UnsupportedProtocol(err.to_string())
            }
            Ipv6Error::UnsupportedProtocol { .. } => {
                CoreError::UnsupportedProtocol(err.to_string())
            }
            Ipv6Error::DestinationUnreachable { .. } => {
                CoreError::InvalidPacket(err.to_string())
            }
            Ipv6Error::HopLimitExceeded { .. } => {
                CoreError::InvalidPacket(err.to_string())
            }
            Ipv6Error::InvalidSourceAddress { .. } => {
                CoreError::InvalidPacket(err.to_string())
            }
            Ipv6Error::PacketTooLarge { .. } => {
                CoreError::InvalidPacket(err.to_string())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = Ipv6Error::invalid_version(4);
        assert!(err.to_string().contains("版本无效"));

        let err = Ipv6Error::unsupported_protocol(99);
        assert!(err.to_string().contains("协议不支持"));

        let err = Ipv6Error::extension_header_not_supported(0);
        assert!(err.to_string().contains("扩展头不支持"));
    }

    #[test]
    fn test_error_to_core_error() {
        let ipv6_err = Ipv6Error::unsupported_protocol(6);
        let core_err: CoreError = ipv6_err.into();
        match core_err {
            CoreError::UnsupportedProtocol(msg) => {
                assert!(msg.contains("协议不支持"));
            }
            _ => panic!("Expected UnsupportedProtocol"),
        }
    }
}
