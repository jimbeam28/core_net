// src/protocols/ip/error.rs
//
// IPv4 协议错误类型

use crate::protocols::Ipv4Addr;
use crate::common::CoreError;

/// IPv4 协议处理错误类型
///
/// 定义了 IP 协议处理过程中可能遇到的所有错误情况。
/// 实现 `From<IpError> for CoreError` 以便与其他模块集成。
#[derive(Debug)]
pub enum IpError {
    /// 版本号不匹配
    InvalidVersion {
        expected: u8,
        found: u8,
    },

    /// 首部长度无效
    InvalidHeaderLength {
        ihl: u8,
    },

    /// 校验和错误
    ChecksumError {
        expected: u16,
        calculated: u16,
    },

    /// 数据报长度不足
    PacketTooShort {
        expected: usize,
        found: usize,
    },

    /// 分片数据报（当前版本不支持分片和重组）
    FragmentedPacket {
        mf_flag: bool,
        fragment_offset: u16,
    },

    /// 分片重叠
    FragmentOverlap {
        offset: u16,
    },

    /// 分片数量超过限制
    TooManyFragments {
        count: usize,
        max: usize,
    },

    /// 重组表已满
    ReassemblyTableFull,

    /// 重组超时
    ReassemblyTimeout {
        id: u16,
    },

    /// 协议不支持
    UnsupportedProtocol {
        protocol: u8,
    },

    /// 目的地址不可达
    DestinationUnreachable {
        addr: Ipv4Addr,
    },

    /// TTL 超时
    TtlExceeded {
        ttl: u8,
    },

    /// 数据报长度超过 MTU 且 DF 标志置位
    FragmentationNeeded {
        mtu: u16,
        length: u16,
    },

    /// 无效数据包
    InvalidPacket {
        message: String,
    },
}

impl IpError {
    /// 创建版本号错误
    pub fn invalid_version(found: u8) -> Self {
        IpError::InvalidVersion { expected: 4, found }
    }

    /// 创建首部长度错误
    pub fn invalid_header_length(ihl: u8) -> Self {
        IpError::InvalidHeaderLength { ihl }
    }

    /// 创建校验和错误
    pub fn checksum_error(expected: u16, calculated: u16) -> Self {
        IpError::ChecksumError { expected, calculated }
    }

    /// 创建数据报长度不足错误
    pub fn packet_too_short(expected: usize, found: usize) -> Self {
        IpError::PacketTooShort { expected, found }
    }

    /// 创建分片数据报错误
    pub fn fragmented_packet(mf_flag: bool, fragment_offset: u16) -> Self {
        IpError::FragmentedPacket { mf_flag, fragment_offset }
    }

    /// 创建分片重叠错误
    pub fn fragment_overlap(offset: u16) -> Self {
        IpError::FragmentOverlap { offset }
    }

    /// 创建分片数量过多错误
    pub fn too_many_fragments(count: usize, max: usize) -> Self {
        IpError::TooManyFragments { count, max }
    }

    /// 创建重组表已满错误
    pub fn reassembly_table_full() -> Self {
        IpError::ReassemblyTableFull
    }

    /// 创建重组超时错误
    pub fn reassembly_timeout(id: u16) -> Self {
        IpError::ReassemblyTimeout { id }
    }

    /// 创建协议不支持错误
    pub fn unsupported_protocol(protocol: u8) -> Self {
        IpError::UnsupportedProtocol { protocol }
    }

    /// 创建目的地址不可达错误
    pub fn destination_unreachable(addr: Ipv4Addr) -> Self {
        IpError::DestinationUnreachable { addr }
    }

    /// 创建 TTL 超时错误
    pub fn ttl_exceeded(ttl: u8) -> Self {
        IpError::TtlExceeded { ttl }
    }

    /// 创建需要分片错误
    pub fn fragmentation_needed(mtu: u16, length: u16) -> Self {
        IpError::FragmentationNeeded { mtu, length }
    }

    /// 创建无效数据包错误
    pub fn invalid_packet(msg: impl Into<String>) -> Self {
        IpError::InvalidPacket {
            message: msg.into(),
        }
    }
}

impl std::fmt::Display for IpError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IpError::InvalidVersion { expected, found } => {
                write!(f, "IP版本无效: 期望={}, 发现={}", expected, found)
            }
            IpError::InvalidHeaderLength { ihl } => {
                write!(f, "IP首部长度无效: IHL={}", ihl)
            }
            IpError::ChecksumError { expected, calculated } => {
                write!(f, "IP校验和错误: 期望={:04x}, 计算={:04x}", expected, calculated)
            }
            IpError::PacketTooShort { expected, found } => {
                write!(f, "IP数据报长度不足: 期望={}, 发现={}", expected, found)
            }
            IpError::FragmentedPacket { mf_flag, fragment_offset } => {
                write!(f, "IP分片数据报(不支持): MF={}, Offset={}", mf_flag, fragment_offset)
            }
            IpError::FragmentOverlap { offset } => {
                write!(f, "IP分片重叠: Offset={}", offset)
            }
            IpError::TooManyFragments { count, max } => {
                write!(f, "IP分片数量超过限制: count={}, max={}", count, max)
            }
            IpError::ReassemblyTableFull => {
                write!(f, "IP重组表已满")
            }
            IpError::ReassemblyTimeout { id } => {
                write!(f, "IP重组超时: ID={}", id)
            }
            IpError::UnsupportedProtocol { protocol } => {
                write!(f, "IP协议不支持: Protocol={}", protocol)
            }
            IpError::DestinationUnreachable { addr } => {
                write!(f, "IP目的地址不可达: {}", addr)
            }
            IpError::TtlExceeded { ttl } => {
                write!(f, "IP TTL超时: TTL={}", ttl)
            }
            IpError::FragmentationNeeded { mtu, length } => {
                write!(f, "IP需要分片但DF置位: MTU={}, Length={}", mtu, length)
            }
            IpError::InvalidPacket { message } => {
                write!(f, "IP数据包无效: {}", message)
            }
        }
    }
}

impl std::error::Error for IpError {}

// 转换为 CoreError
impl From<IpError> for CoreError {
    fn from(err: IpError) -> Self {
        match err {
            IpError::InvalidVersion { .. } | IpError::InvalidHeaderLength { .. } => {
                CoreError::InvalidPacket(err.to_string())
            }
            IpError::ChecksumError { .. } => {
                CoreError::InvalidPacket(err.to_string())
            }
            IpError::PacketTooShort { .. } => {
                CoreError::InvalidPacket(err.to_string())
            }
            IpError::FragmentedPacket { .. } => {
                CoreError::UnsupportedProtocol(err.to_string())
            }
            IpError::FragmentOverlap { .. } => {
                CoreError::InvalidPacket(err.to_string())
            }
            IpError::TooManyFragments { .. } => {
                CoreError::InvalidPacket(err.to_string())
            }
            IpError::ReassemblyTableFull => {
                CoreError::InvalidPacket(err.to_string())
            }
            IpError::ReassemblyTimeout { .. } => {
                CoreError::InvalidPacket(err.to_string())
            }
            IpError::UnsupportedProtocol { .. } => {
                CoreError::UnsupportedProtocol(err.to_string())
            }
            IpError::DestinationUnreachable { .. } => {
                CoreError::InvalidPacket(err.to_string())
            }
            IpError::TtlExceeded { .. } => {
                CoreError::InvalidPacket(err.to_string())
            }
            IpError::FragmentationNeeded { .. } => {
                CoreError::InvalidPacket(err.to_string())
            }
            IpError::InvalidPacket { .. } => {
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
        let err = IpError::invalid_version(6);
        assert!(err.to_string().contains("版本无效"));

        let err = IpError::unsupported_protocol(99);
        assert!(err.to_string().contains("协议不支持"));

        let err = IpError::fragmented_packet(true, 185);
        assert!(err.to_string().contains("分片数据报"));
    }

    #[test]
    fn test_error_to_core_error() {
        let ip_err = IpError::unsupported_protocol(6);
        let core_err: CoreError = ip_err.into();
        match core_err {
            CoreError::UnsupportedProtocol(msg) => {
                assert!(msg.contains("协议不支持"));
            }
            _ => panic!("Expected UnsupportedProtocol"),
        }
    }
}
