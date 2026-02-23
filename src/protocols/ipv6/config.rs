// src/protocols/ipv6/config.rs
//
// IPv6 协议配置

use super::extension::ExtensionConfig;
use super::fragment::{DEFAULT_MAX_REASSEMBLY_ENTRIES, DEFAULT_REASSEMBLY_TIMEOUT};

/// IPv6 协议配置
///
/// 包含 IPv6 协议处理的所有配置参数，包括扩展头、分片、路由等。
#[derive(Debug, Clone)]
pub struct Ipv6Config {
    // ========== 基础配置 ==========
    /// 默认跳数限制 (默认: 64)
    pub default_hop_limit: u8,

    /// 最小链路 MTU (默认: 1280, RFC 8200 要求)
    pub min_mtu: u16,

    /// 最大数据包大小 (默认: 65535)
    pub max_packet_size: u16,

    // ========== 扩展头部配置 ==========
    /// 是否允许扩展头部
    pub allow_extension_headers: bool,

    /// 最大扩展头数量 (默认: 8)
    pub max_extension_headers: usize,

    /// 最大扩展头总长度 (默认: 2048)
    pub max_extension_headers_length: usize,

    // ========== 逐跳选项配置 ==========
    /// 是否处理逐跳选项头
    pub process_hop_by_hop: bool,

    /// 是否支持 Jumbo Payload
    pub allow_jumbo_payload: bool,

    /// Router Alert 速率限制（每秒）
    pub router_alert_rate_limit: u32,

    // ========== 分片配置 ==========
    /// 是否支持分片
    pub enable_fragmentation: bool,

    /// 是否支持重组
    pub enable_reassembly: bool,

    /// 最大分片缓存条目数
    pub max_reassembly_entries: usize,

    /// 重组超时时间（秒）
    pub reassembly_timeout: u64,

    /// 每个数据包的最大分片数
    pub max_fragments_per_packet: usize,

    /// 是否拒绝原子分片（单个分片）
    pub reject_atomic_fragments: bool,

    // ========== 路由头配置 ==========
    /// 是否接受路由头
    pub accept_routing_header: bool,

    /// 允许的路由类型
    pub allowed_routing_types: Vec<u8>,

    // ========== 目的选项配置 ==========
    /// 是否处理目的选项头
    pub process_destination_options: bool,

    // ========== 安全配置 ==========
    /// 是否验证所有扩展头长度
    pub verify_all_lengths: bool,

    /// 扩展头处理速率限制（每秒）
    pub extension_header_rate_limit: u32,
}

impl Default for Ipv6Config {
    fn default() -> Self {
        Self {
            // 基础配置
            default_hop_limit: 64,
            min_mtu: 1280,
            max_packet_size: 65535,

            // 扩展头部配置
            allow_extension_headers: true,
            max_extension_headers: 8,
            max_extension_headers_length: 2048,

            // 逐跳选项配置
            process_hop_by_hop: true,
            allow_jumbo_payload: false,
            router_alert_rate_limit: 100,

            // 分片配置
            enable_fragmentation: false,
            enable_reassembly: false,
            max_reassembly_entries: DEFAULT_MAX_REASSEMBLY_ENTRIES,
            reassembly_timeout: DEFAULT_REASSEMBLY_TIMEOUT,
            max_fragments_per_packet: 64,
            reject_atomic_fragments: true,

            // 路由头配置
            accept_routing_header: false,
            allowed_routing_types: vec![2, 3], // Type 2 和 Type 3

            // 目的选项配置
            process_destination_options: true,

            // 安全配置
            verify_all_lengths: true,
            extension_header_rate_limit: 1000,
        }
    }
}

impl Ipv6Config {
    /// 创建新的配置
    pub fn new() -> Self {
        Self::default()
    }

    /// 获取扩展头配置
    pub fn extension_config(&self) -> ExtensionConfig {
        ExtensionConfig {
            max_extension_headers: self.max_extension_headers,
            max_extension_headers_length: self.max_extension_headers_length,
            process_hop_by_hop: self.process_hop_by_hop,
            process_destination_options: self.process_destination_options,
            accept_routing_header: self.accept_routing_header,
            enable_fragmentation: self.enable_fragmentation,
            reject_atomic_fragments: self.reject_atomic_fragments,
            verify_all_lengths: self.verify_all_lengths,
        }
    }

    /// 设置是否允许扩展头
    pub fn with_allow_extension_headers(mut self, allow: bool) -> Self {
        self.allow_extension_headers = allow;
        self
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

    /// 设置是否接受路由头
    pub fn with_accept_routing_header(mut self, accept: bool) -> Self {
        self.accept_routing_header = accept;
        self
    }

    /// 设置是否允许 Jumbo Payload
    pub fn with_allow_jumbo_payload(mut self, allow: bool) -> Self {
        self.allow_jumbo_payload = allow;
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

        // 扩展头配置
        assert!(config.allow_extension_headers);
        assert_eq!(config.max_extension_headers, 8);
        assert_eq!(config.max_extension_headers_length, 2048);

        // 逐跳选项配置
        assert!(config.process_hop_by_hop);
        assert!(!config.allow_jumbo_payload);
        assert_eq!(config.router_alert_rate_limit, 100);

        // 分片配置
        assert!(!config.enable_fragmentation);
        assert!(!config.enable_reassembly);
        assert_eq!(config.max_reassembly_entries, 256);
        assert_eq!(config.reassembly_timeout, 60);
        assert_eq!(config.max_fragments_per_packet, 64);
        assert!(config.reject_atomic_fragments);

        // 路由头配置
        assert!(!config.accept_routing_header);
        assert_eq!(config.allowed_routing_types, vec![2, 3]);

        // 目的选项配置
        assert!(config.process_destination_options);

        // 安全配置
        assert!(config.verify_all_lengths);
        assert_eq!(config.extension_header_rate_limit, 1000);
    }

    #[test]
    fn test_extension_config() {
        let config = Ipv6Config::default();
        let ext_config = config.extension_config();

        assert_eq!(ext_config.max_extension_headers, 8);
        assert_eq!(ext_config.max_extension_headers_length, 2048);
        assert!(ext_config.process_hop_by_hop);
        assert!(ext_config.process_destination_options);
        assert!(!ext_config.accept_routing_header);
        assert!(!ext_config.enable_fragmentation);
        assert!(ext_config.reject_atomic_fragments);
        assert!(ext_config.verify_all_lengths);
    }

    #[test]
    fn test_config_builders() {
        let config = Ipv6Config::new()
            .with_allow_extension_headers(true)
            .with_enable_fragmentation(true)
            .with_enable_reassembly(true)
            .with_accept_routing_header(true)
            .with_allow_jumbo_payload(true);

        assert!(config.allow_extension_headers);
        assert!(config.enable_fragmentation);
        assert!(config.enable_reassembly);
        assert!(config.accept_routing_header);
        assert!(config.allow_jumbo_payload);
    }
}
