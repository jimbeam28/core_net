// src/protocols/ip/config.rs
//
// IPv4 协议配置参数

/// IPv4 协议配置参数
///
/// 定义了 IPv4 协议处理的各种配置选项。
/// 当前版本不支持分片和重组，因此不包含相关配置。
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
}

/// 默认 IPv4 配置
pub const IPV4_CONFIG_DEFAULT: Ipv4Config = Ipv4Config {
    default_ttl: 64,
    min_mtu: 576,
    default_mtu: 1500,
    verify_checksum: true,
    process_options: true,
    icmp_error_rate_limit: 100,
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
    }

    #[test]
    fn test_config_builder() {
        let config = Ipv4Config::new()
            .with_default_ttl(128)
            .with_min_mtu(1280)
            .with_verify_checksum(false);

        assert_eq!(config.default_ttl, 128);
        assert_eq!(config.min_mtu, 1280);
        assert!(!config.verify_checksum);
    }

    #[test]
    fn test_const_default_config() {
        assert_eq!(IPV4_CONFIG_DEFAULT.default_ttl, 64);
        assert_eq!(IPV4_CONFIG_DEFAULT.min_mtu, 576);
    }
}
