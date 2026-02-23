// src/protocols/ip/config.rs
//
// IPv4 协议配置参数

use crate::protocols::ip::fragment::FragmentOverlapPolicy;

/// IPv4 协议配置参数
///
/// 定义了 IPv4 协议处理的各种配置选项。
#[derive(Debug, Clone, PartialEq)]
pub struct Ipv4Config {
    /// 默认 TTL 值
    pub default_ttl: u8,

    /// 最小 MTU（RFC 规定至少 576 字节）
    pub min_mtu: u16,

    /// 默认 MTU（标准以太网）
    pub default_mtu: u16,

    /// 是否验证校验和
    pub verify_checksum: bool,

    /// 是否处理 IP 选项
    pub process_options: bool,

    /// ICMP 错误消息速率限制（每秒）
    pub icmp_error_rate_limit: u32,

    // ========== 分片和重组相关参数 ==========

    /// 是否允许分片（全局开关，可被 DF 标志覆盖）
    pub allow_fragmentation: bool,

    /// 发送时默认 DF 标志
    pub df_flag: bool,

    /// 重组超时时间（秒）
    /// RFC 1122 推荐至少 30 秒
    pub reassembly_timeout_secs: u32,

    /// 最大重组条目数
    pub max_reassembly_entries: usize,

    /// 每个数据报最大分片数
    pub max_fragments_per_datagram: usize,

    /// 是否检测分片重叠
    pub detect_fragment_overlap: bool,

    /// 分片重叠处理策略
    pub fragment_overlap_policy: FragmentOverlapPolicy,
}

impl Default for Ipv4Config {
    fn default() -> Self {
        Self {
            default_ttl: 64,
            min_mtu: 576,
            default_mtu: 1500,
            verify_checksum: true,
            process_options: true,
            icmp_error_rate_limit: 100,
            allow_fragmentation: true,
            df_flag: false,
            reassembly_timeout_secs: 30,
            max_reassembly_entries: 64,
            max_fragments_per_datagram: 16,
            detect_fragment_overlap: true,
            fragment_overlap_policy: FragmentOverlapPolicy::Drop,
        }
    }
}

impl Ipv4Config {
    /// 创建新的 IPv4 配置
    pub const fn new() -> Self {
        Self {
            default_ttl: 64,
            min_mtu: 576,
            default_mtu: 1500,
            verify_checksum: true,
            process_options: true,
            icmp_error_rate_limit: 100,
            allow_fragmentation: true,
            df_flag: false,
            reassembly_timeout_secs: 30,
            max_reassembly_entries: 64,
            max_fragments_per_datagram: 16,
            detect_fragment_overlap: true,
            fragment_overlap_policy: FragmentOverlapPolicy::Drop,
        }
    }

    /// 设置默认 TTL
    pub const fn with_default_ttl(mut self, ttl: u8) -> Self {
        self.default_ttl = ttl;
        self
    }

    /// 设置最小 MTU
    pub const fn with_min_mtu(mut self, mtu: u16) -> Self {
        self.min_mtu = mtu;
        self
    }

    /// 设置默认 MTU
    pub const fn with_default_mtu(mut self, mtu: u16) -> Self {
        self.default_mtu = mtu;
        self
    }

    /// 设置是否验证校验和
    pub const fn with_verify_checksum(mut self, verify: bool) -> Self {
        self.verify_checksum = verify;
        self
    }

    /// 设置是否处理 IP 选项
    pub const fn with_process_options(mut self, process: bool) -> Self {
        self.process_options = process;
        self
    }

    /// 设置 ICMP 错误消息速率限制
    pub const fn with_icmp_error_rate_limit(mut self, limit: u32) -> Self {
        self.icmp_error_rate_limit = limit;
        self
    }

    /// 设置是否允许分片
    pub const fn with_allow_fragmentation(mut self, allow: bool) -> Self {
        self.allow_fragmentation = allow;
        self
    }

    /// 设置默认 DF 标志
    pub const fn with_df_flag(mut self, df: bool) -> Self {
        self.df_flag = df;
        self
    }

    /// 设置重组超时时间
    pub const fn with_reassembly_timeout(mut self, timeout_secs: u32) -> Self {
        self.reassembly_timeout_secs = timeout_secs;
        self
    }

    /// 设置最大重组条目数
    pub const fn with_max_reassembly_entries(mut self, max: usize) -> Self {
        self.max_reassembly_entries = max;
        self
    }

    /// 设置最大分片数
    pub const fn with_max_fragments_per_datagram(mut self, max: usize) -> Self {
        self.max_fragments_per_datagram = max;
        self
    }

    /// 设置是否检测分片重叠
    pub const fn with_detect_fragment_overlap(mut self, detect: bool) -> Self {
        self.detect_fragment_overlap = detect;
        self
    }

    /// 设置分片重叠处理策略
    pub const fn with_fragment_overlap_policy(mut self, policy: FragmentOverlapPolicy) -> Self {
        self.fragment_overlap_policy = policy;
        self
    }
}

/// 默认 IPv4 配置
pub const IPV4_CONFIG_DEFAULT: Ipv4Config = Ipv4Config {
    default_ttl: 64,
    min_mtu: 576,
    default_mtu: 1500,
    verify_checksum: true,
    process_options: true,
    icmp_error_rate_limit: 100,
    allow_fragmentation: true,
    df_flag: false,
    reassembly_timeout_secs: 30,
    max_reassembly_entries: 64,
    max_fragments_per_datagram: 16,
    detect_fragment_overlap: true,
    fragment_overlap_policy: FragmentOverlapPolicy::Drop,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = Ipv4Config::default();
        assert_eq!(config.default_ttl, 64);
        assert_eq!(config.min_mtu, 576);
        assert_eq!(config.default_mtu, 1500);
        assert!(config.verify_checksum);
        assert!(config.process_options);
        assert_eq!(config.icmp_error_rate_limit, 100);
        assert!(config.allow_fragmentation);
        assert!(!config.df_flag);
        assert_eq!(config.reassembly_timeout_secs, 30);
        assert_eq!(config.max_reassembly_entries, 64);
        assert_eq!(config.max_fragments_per_datagram, 16);
        assert!(config.detect_fragment_overlap);
    }

    #[test]
    fn test_config_builder() {
        let config = Ipv4Config::new()
            .with_default_ttl(128)
            .with_min_mtu(1280)
            .with_verify_checksum(false)
            .with_allow_fragmentation(false)
            .with_df_flag(true);

        assert_eq!(config.default_ttl, 128);
        assert_eq!(config.min_mtu, 1280);
        assert!(!config.verify_checksum);
        assert!(!config.allow_fragmentation);
        assert!(config.df_flag);
    }

    #[test]
    fn test_const_default_config() {
        assert_eq!(IPV4_CONFIG_DEFAULT.default_ttl, 64);
        assert_eq!(IPV4_CONFIG_DEFAULT.min_mtu, 576);
    }

    #[test]
    fn test_fragmentation_config() {
        let config = Ipv4Config::new()
            .with_reassembly_timeout(60)
            .with_max_reassembly_entries(128)
            .with_max_fragments_per_datagram(32)
            .with_detect_fragment_overlap(false)
            .with_fragment_overlap_policy(FragmentOverlapPolicy::First);

        assert_eq!(config.reassembly_timeout_secs, 60);
        assert_eq!(config.max_reassembly_entries, 128);
        assert_eq!(config.max_fragments_per_datagram, 32);
        assert!(!config.detect_fragment_overlap);
        assert_eq!(config.fragment_overlap_policy, FragmentOverlapPolicy::First);
    }
}

