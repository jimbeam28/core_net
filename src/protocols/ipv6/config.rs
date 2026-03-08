// src/protocols/ipv6/config.rs
//
// IPv6 协议配置（精简版）

use super::fragment::{DEFAULT_MAX_REASSEMBLY_ENTRIES, DEFAULT_REASSEMBLY_TIMEOUT};

/// IPv6 协议配置（精简版）
///
/// 包含 IPv6 协议处理的基础配置参数。
#[derive(Debug, Clone)]
pub struct Ipv6Config {
    // ========== 基础配置 ==========
    /// 默认跳数限制 (默认: 64)
    pub default_hop_limit: u8,

    /// 最小链路 MTU (默认: 1280, RFC 8200 要求)
    pub min_mtu: u16,

    /// 最大数据包大小 (默认: 65535)
    pub max_packet_size: u16,

    // ========== 分片配置 ==========
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
            // 基础配置
            default_hop_limit: 64,
            min_mtu: 1280,
            max_packet_size: 65535,

            // 分片配置
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
mod tests {
    use super::*;

    #[test]
    fn test_config_default() {
        let config = Ipv6Config::default();

        // 基础配置
        assert_eq!(config.default_hop_limit, 64);
        assert_eq!(config.min_mtu, 1280);
        assert_eq!(config.max_packet_size, 65535);

        // 分片配置
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
