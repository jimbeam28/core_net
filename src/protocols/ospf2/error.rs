// src/protocols/ospf2/error.rs
//
// OSPFv2 错误类型定义

use std::fmt;

/// OSPFv2 错误类型
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OspfError {
    /// 解析错误
    ParseError {
        field: String,
        reason: String,
    },

    /// 报文长度不足
    PacketTooShort {
        expected: usize,
        actual: usize,
    },

    /// 无效的报文类型
    InvalidPacketType {
        packet_type: u8,
    },

    /// 无效的 LSA 类型
    InvalidLsaType {
        lsa_type: u8,
    },

    /// 无效的 LSA 年龄
    InvalidLsaAge {
        age: u16,
    },

    /// 无效的 LSA 序列号
    InvalidLsaSequenceNumber {
        sequence: u32,
    },

    /// 无效的 Router ID
    InvalidRouterId {
        router_id: crate::common::Ipv4Addr,
    },

    /// 无效的 Area ID
    InvalidAreaId {
        area_id: crate::common::Ipv4Addr,
    },

    /// 认证失败
    AuthenticationFailed {
        reason: String,
    },

    /// 校验和错误
    ChecksumError {
        expected: u16,
        actual: u16,
    },

    /// 邻居状态错误
    NeighborStateError {
        neighbor_id: crate::common::Ipv4Addr,
        current_state: String,
        expected_state: String,
    },

    /// 接口状态错误
    InterfaceStateError {
        interface_name: String,
        current_state: String,
        expected_state: String,
    },

    /// LSA 不存在
    LsaNotFound {
        lsa_type: u8,
        link_state_id: crate::common::Ipv4Addr,
        advertising_router: crate::common::Ipv4Addr,
    },

    /// SPF 计算错误
    SpfCalculationError {
        reason: String,
    },

    /// 配置错误
    ConfigError {
        parameter: String,
        reason: String,
    },

    /// 超时错误
    TimeoutError {
        timer_name: String,
    },

    /// 锁错误
    LockError,

    /// 其他错误
    Other {
        reason: String,
    },
}

impl OspfError {
    /// 创建解析错误
    pub fn parse_error(field: impl Into<String>, reason: impl Into<String>) -> Self {
        Self::ParseError {
            field: field.into(),
            reason: reason.into(),
        }
    }

    /// 创建报文长度错误
    pub fn packet_too_short(expected: usize, actual: usize) -> Self {
        Self::PacketTooShort { expected, actual }
    }

    /// 创建无效报文类型错误
    pub fn invalid_packet_type(packet_type: u8) -> Self {
        Self::InvalidPacketType { packet_type }
    }

    /// 创建认证失败错误
    pub fn authentication_failed(reason: impl Into<String>) -> Self {
        Self::AuthenticationFailed {
            reason: reason.into(),
        }
    }

    /// 创建校验和错误
    pub fn checksum_error(expected: u16, actual: u16) -> Self {
        Self::ChecksumError { expected, actual }
    }
}

impl fmt::Display for OspfError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ParseError { field, reason } => {
                write!(f, "Parse error for field '{}': {}", field, reason)
            }
            Self::PacketTooShort { expected, actual } => {
                write!(f, "Packet too short: expected {} bytes, got {} bytes", expected, actual)
            }
            Self::InvalidPacketType { packet_type } => {
                write!(f, "Invalid packet type: {}", packet_type)
            }
            Self::InvalidLsaType { lsa_type } => {
                write!(f, "Invalid LSA type: {}", lsa_type)
            }
            Self::InvalidLsaAge { age } => {
                write!(f, "Invalid LSA age: {}", age)
            }
            Self::InvalidLsaSequenceNumber { sequence } => {
                write!(f, "Invalid LSA sequence number: 0x{:08X}", sequence)
            }
            Self::InvalidRouterId { router_id } => {
                write!(f, "Invalid Router ID: {}", router_id)
            }
            Self::InvalidAreaId { area_id } => {
                write!(f, "Invalid Area ID: {}", area_id)
            }
            Self::AuthenticationFailed { reason } => {
                write!(f, "Authentication failed: {}", reason)
            }
            Self::ChecksumError { expected, actual } => {
                write!(f, "Checksum error: expected 0x{:04X}, got 0x{:04X}", expected, actual)
            }
            Self::NeighborStateError { neighbor_id, current_state, expected_state } => {
                write!(f, "Neighbor {} state error: current={}, expected={}",
                    neighbor_id, current_state, expected_state)
            }
            Self::InterfaceStateError { interface_name, current_state, expected_state } => {
                write!(f, "Interface {} state error: current={}, expected={}",
                    interface_name, current_state, expected_state)
            }
            Self::LsaNotFound { lsa_type, link_state_id, advertising_router } => {
                write!(f, "LSA not found: type={}, link_state_id={}, advertising_router={}",
                    lsa_type, link_state_id, advertising_router)
            }
            Self::SpfCalculationError { reason } => {
                write!(f, "SPF calculation error: {}", reason)
            }
            Self::ConfigError { parameter, reason } => {
                write!(f, "Configuration error for '{}': {}", parameter, reason)
            }
            Self::TimeoutError { timer_name } => {
                write!(f, "Timeout error: timer '{}'", timer_name)
            }
            Self::LockError => {
                write!(f, "Lock error")
            }
            Self::Other { reason } => {
                write!(f, "OSPF error: {}", reason)
            }
        }
    }
}

impl std::error::Error for OspfError {}

/// OSPFv2 结果类型
pub type OspfResult<T> = Result<T, OspfError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ospf_error_display() {
        let err = OspfError::parse_error("version", "invalid version number");
        assert_eq!(err.to_string(), "Parse error for field 'version': invalid version number");

        let err = OspfError::packet_too_short(24, 10);
        assert_eq!(err.to_string(), "Packet too short: expected 24 bytes, got 10 bytes");
    }

    #[test]
    fn test_authentication_failed_error() {
        let err = OspfError::authentication_failed("invalid password");
        assert_eq!(err.to_string(), "Authentication failed: invalid password");
    }

    #[test]
    fn test_checksum_error() {
        let err = OspfError::checksum_error(0x1234, 0x5678);
        assert_eq!(err.to_string(), "Checksum error: expected 0x1234, got 0x5678");
    }
}
