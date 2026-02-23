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

    /// 无效头部长度（简化版本）
    InvalidHeaderLengthSimple,

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

    /// 扩展头数量超过限制
    TooManyExtensionHeaders {
        count: usize,
        max: usize,
    },

    /// 扩展头长度无效
    InvalidExtensionHeaderLength {
        header_type: u8,
        length: usize,
    },

    /// 扩展头链过长
    ExtensionChainTooLong {
        total_length: usize,
        max: usize,
    },

    /// 分片重组超时
    FragmentReassemblyTimeout {
        id: u32,
    },

    /// 分片重叠
    FragmentOverlap {
        id: u32,
    },

    /// 原子分片（RFC 8200 禁止）
    AtomicFragment {
        id: u32,
    },

    /// 路由头类型不支持
    RoutingHeaderNotSupported {
        routing_type: u8,
    },

    /// 路由头类型0已废弃
    RoutingHeaderType0Deprecated,

    /// 选项类型未知且 Action 要求丢弃
    UnknownOptionWithDropAction {
        option_type: u8,
    },

    /// Jumbo Payload 不支持
    JumboPayloadNotSupported,

    /// 重组错误（通用错误）
    ReassemblyError,

    /// 重组未完成
    ReassemblyIncomplete,

    /// 分片数量过多
    ReassemblyTooManyFragments,

    /// 分片重叠
    ReassemblyFragmentOverlap,

    /// 重组超时
    ReassemblyTimeout,
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

    /// 创建无效头部长度错误（简化版本）
    pub fn invalid_header_length_simple() -> Self {
        Ipv6Error::InvalidHeaderLengthSimple
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

    /// 创建扩展头数量超限错误
    pub fn too_many_extension_headers(count: usize, max: usize) -> Self {
        Ipv6Error::TooManyExtensionHeaders { count, max }
    }

    /// 创建扩展头长度无效错误
    pub fn invalid_extension_header_length(header_type: u8, length: usize) -> Self {
        Ipv6Error::InvalidExtensionHeaderLength { header_type, length }
    }

    /// 创建扩展头链过长错误
    pub fn extension_chain_too_long(total_length: usize, max: usize) -> Self {
        Ipv6Error::ExtensionChainTooLong { total_length, max }
    }

    /// 创建分片重组超时错误
    pub fn fragment_reassembly_timeout(id: u32) -> Self {
        Ipv6Error::FragmentReassemblyTimeout { id }
    }

    /// 创建分片重叠错误
    pub fn fragment_overlap(id: u32) -> Self {
        Ipv6Error::FragmentOverlap { id }
    }

    /// 创建原子分片错误
    pub fn atomic_fragment(id: u32) -> Self {
        Ipv6Error::AtomicFragment { id }
    }

    /// 创建路由头不支持错误
    pub fn routing_header_not_supported(routing_type: u8) -> Self {
        Ipv6Error::RoutingHeaderNotSupported { routing_type }
    }

    /// 创建路由头类型0已废弃错误
    pub fn routing_header_type0_deprecated() -> Self {
        Ipv6Error::RoutingHeaderType0Deprecated
    }

    /// 创建未知选项丢弃错误
    pub fn unknown_option_with_drop_action(option_type: u8) -> Self {
        Ipv6Error::UnknownOptionWithDropAction { option_type }
    }

    /// 创建 Jumbo Payload 不支持错误
    pub fn jumbo_payload_not_supported() -> Self {
        Ipv6Error::JumboPayloadNotSupported
    }

    /// 创建重组错误
    pub fn reassembly_error() -> Self {
        Ipv6Error::ReassemblyError
    }

    /// 创建重组未完成错误
    pub fn reassembly_incomplete() -> Self {
        Ipv6Error::ReassemblyIncomplete
    }

    /// 创建分片数量过多错误
    pub fn reassembly_too_many_fragments() -> Self {
        Ipv6Error::ReassemblyTooManyFragments
    }

    /// 创建分片重叠错误
    pub fn reassembly_fragment_overlap() -> Self {
        Ipv6Error::ReassemblyFragmentOverlap
    }

    /// 创建重组超时错误
    pub fn reassembly_timeout() -> Self {
        Ipv6Error::ReassemblyTimeout
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
            Ipv6Error::InvalidHeaderLengthSimple => {
                write!(f, "IPv6头部长度无效")
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
            Ipv6Error::TooManyExtensionHeaders { count, max } => {
                write!(f, "IPv6扩展头数量超过限制: 数量={}, 最大={}", count, max)
            }
            Ipv6Error::InvalidExtensionHeaderLength { header_type, length } => {
                write!(f, "IPv6扩展头长度无效: 类型={}, 长度={}", header_type, length)
            }
            Ipv6Error::ExtensionChainTooLong { total_length, max } => {
                write!(f, "IPv6扩展头链过长: 总长度={}, 最大={}", total_length, max)
            }
            Ipv6Error::FragmentReassemblyTimeout { id } => {
                write!(f, "IPv6分片重组超时: ID={}", id)
            }
            Ipv6Error::FragmentOverlap { id } => {
                write!(f, "IPv6分片重叠: ID={}", id)
            }
            Ipv6Error::AtomicFragment { id } => {
                write!(f, "IPv6原子分片禁止: ID={}", id)
            }
            Ipv6Error::RoutingHeaderNotSupported { routing_type } => {
                write!(f, "IPv6路由头不支持: 类型={}", routing_type)
            }
            Ipv6Error::RoutingHeaderType0Deprecated => {
                write!(f, "IPv6路由头Type 0已废弃(RFC 5095)")
            }
            Ipv6Error::UnknownOptionWithDropAction { option_type } => {
                write!(f, "IPv6未知选项要求丢弃: 选项类型={}", option_type)
            }
            Ipv6Error::JumboPayloadNotSupported => {
                write!(f, "IPv6 Jumbo Payload不支持")
            }
            Ipv6Error::ReassemblyError => {
                write!(f, "IPv6分片重组错误")
            }
            Ipv6Error::ReassemblyIncomplete => {
                write!(f, "IPv6分片重组未完成")
            }
            Ipv6Error::ReassemblyTooManyFragments => {
                write!(f, "IPv6分片数量过多")
            }
            Ipv6Error::ReassemblyFragmentOverlap => {
                write!(f, "IPv6分片重叠")
            }
            Ipv6Error::ReassemblyTimeout => {
                write!(f, "IPv6分片重组超时")
            }
        }
    }
}

impl std::error::Error for Ipv6Error {}

// 转换为 CoreError
impl From<Ipv6Error> for CoreError {
    fn from(err: Ipv6Error) -> Self {
        match err {
            Ipv6Error::InvalidVersion { .. }
            | Ipv6Error::InvalidHeaderLength { .. }
            | Ipv6Error::InvalidHeaderLengthSimple => {
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
            Ipv6Error::TooManyExtensionHeaders { .. } => {
                CoreError::InvalidPacket(err.to_string())
            }
            Ipv6Error::InvalidExtensionHeaderLength { .. } => {
                CoreError::InvalidPacket(err.to_string())
            }
            Ipv6Error::ExtensionChainTooLong { .. } => {
                CoreError::InvalidPacket(err.to_string())
            }
            Ipv6Error::FragmentReassemblyTimeout { .. } => {
                CoreError::InvalidPacket(err.to_string())
            }
            Ipv6Error::FragmentOverlap { .. } => {
                CoreError::InvalidPacket(err.to_string())
            }
            Ipv6Error::AtomicFragment { .. } => {
                CoreError::InvalidPacket(err.to_string())
            }
            Ipv6Error::RoutingHeaderNotSupported { .. } => {
                CoreError::UnsupportedProtocol(err.to_string())
            }
            Ipv6Error::RoutingHeaderType0Deprecated => {
                CoreError::UnsupportedProtocol(err.to_string())
            }
            Ipv6Error::UnknownOptionWithDropAction { .. } => {
                CoreError::InvalidPacket(err.to_string())
            }
            Ipv6Error::JumboPayloadNotSupported => {
                CoreError::UnsupportedProtocol(err.to_string())
            }
            Ipv6Error::ReassemblyError
            | Ipv6Error::ReassemblyIncomplete
            | Ipv6Error::ReassemblyTooManyFragments
            | Ipv6Error::ReassemblyFragmentOverlap
            | Ipv6Error::ReassemblyTimeout => {
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
