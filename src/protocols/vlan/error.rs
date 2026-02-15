// src/common/protocols/vlan/error.rs
//
// VLAN模块错误类型定义

/// VLAN处理错误
#[derive(Debug)]
pub enum VlanError {
    /// 无效的VLAN ID
    InvalidVlanId { vid: u16 },

    /// 无效的PCP值 (超过7)
    InvalidPcp { pcp: u8 },

    /// 不支持的TPID
    UnsupportedTpid { tpid: u16 },

    /// 报文长度不足，无法解析VLAN标签
    InsufficientPacketLength { expected: usize, actual: usize },

    /// 双层VLAN标签暂不支持
    DoubleTagNotSupported,

    /// VLAN标签解析错误
    ParseError(String),
}

impl std::fmt::Display for VlanError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidVlanId { vid } =>
                write!(f, "无效的VLAN ID: {} (有效范围: 1-4094)", vid),
            Self::InvalidPcp { pcp } =>
                write!(f, "无效的PCP值: {} (有效范围: 0-7)", pcp),
            Self::UnsupportedTpid { tpid } =>
                write!(f, "不支持的TPID: 0x{:04x}", tpid),
            Self::InsufficientPacketLength { expected, actual } =>
                write!(f, "报文长度不足: 期望{}字节, 实际{}字节", expected, actual),
            Self::DoubleTagNotSupported =>
                write!(f, "双层VLAN标签暂不支持"),
            Self::ParseError(msg) =>
                write!(f, "VLAN解析错误: {}", msg),
        }
    }
}

impl std::error::Error for VlanError {}
