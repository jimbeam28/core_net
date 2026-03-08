// src/protocols/ipv6/mod.rs
//
// IPv6 协议模块（精简版）
// 实现 IPv6 数据包解析、封装、验证
// 不支持扩展头链处理

mod header;
mod protocol;
mod error;
mod packet;
mod fragment;

pub use header::{
    Ipv6Header,
    IPV6_VERSION,
    IPV6_HEADER_LEN,
    IPV6_MIN_MTU,
    DEFAULT_HOP_LIMIT,
};

pub use protocol::IpProtocol;

pub use error::Ipv6Error;

pub use packet::{
    Ipv6ProcessResult,
    Ipv6Result,
    process_ipv6_packet,
    encapsulate_ipv6_packet,
};

// 分片重组相关导出（保留基础功能）
pub use fragment::{
    ReassemblyKey,
    FragmentInfo,
    ReassemblyEntry,
    FragmentCache,
    ReassemblyError,
    FragmentPacket,
    create_fragments_simple,
    DEFAULT_MAX_REASSEMBLY_ENTRIES,
    DEFAULT_REASSEMBLY_TIMEOUT,
};

// ==================== config.rs 内容 ====================

/// IPv6 协议配置（精简版）
#[derive(Debug, Clone)]
pub struct Ipv6Config {
    /// 默认跳数限制 (默认: 64)
    pub default_hop_limit: u8,

    /// 最小链路 MTU (默认: 1280, RFC 8200 要求)
    pub min_mtu: u16,

    /// 最大数据包大小 (默认: 65535)
    pub max_packet_size: u16,

    /// 是否支持分片
    pub enable_fragmentation: bool,

    /// 是否支持重组
    pub enable_reassembly: bool,

    /// 最大分片缓存条目数
    pub max_reassembly_entries: usize,

    /// 重组超时时间（秒）
    pub reassembly_timeout: u64,
}

impl Default for Ipv6Config {
    fn default() -> Self {
        Self {
            default_hop_limit: 64,
            min_mtu: 1280,
            max_packet_size: 65535,
            enable_fragmentation: false,
            enable_reassembly: false,
            max_reassembly_entries: DEFAULT_MAX_REASSEMBLY_ENTRIES,
            reassembly_timeout: DEFAULT_REASSEMBLY_TIMEOUT,
        }
    }
}

impl Ipv6Config {
    /// 创建新的配置
    pub fn new() -> Self {
        Self::default()
    }

    /// 设置是否启用分片
    pub fn with_enable_fragmentation(mut self, enable: bool) -> Self {
        self.enable_fragmentation = enable;
        self
    }

    /// 设置是否启用重组
    pub fn with_enable_reassembly(mut self, enable: bool) -> Self {
        self.enable_reassembly = enable;
        self
    }
}

#[cfg(test)]
mod config_tests {
    use super::*;

    #[test]
    fn test_config_default() {
        let config = Ipv6Config::default();

        assert_eq!(config.default_hop_limit, 64);
        assert_eq!(config.min_mtu, 1280);
        assert_eq!(config.max_packet_size, 65535);
        assert!(!config.enable_fragmentation);
        assert!(!config.enable_reassembly);
        assert_eq!(config.max_reassembly_entries, 256);
        assert_eq!(config.reassembly_timeout, 60);
    }

    #[test]
    fn test_config_builders() {
        let config = Ipv6Config::new()
            .with_enable_fragmentation(true)
            .with_enable_reassembly(true);

        assert!(config.enable_fragmentation);
        assert!(config.enable_reassembly);
    }
}
