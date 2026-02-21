// src/protocols/ipv6/config.rs
//
// IPv6 协议配置

/// IPv6 协议配置
#[derive(Debug, Clone)]
pub struct Ipv6Config {
    /// 默认跳数限制 (默认: 64)
    pub default_hop_limit: u8,

    /// 最小链路 MTU (默认: 1280, RFC 8200 要求)
    pub min_mtu: u16,

    /// 允许的最大扩展头数量 (默认: 0, 暂不支持)
    pub max_extension_headers: usize,

    /// 是否启用分片 (默认: false, 暂不支持)
    pub enable_fragmentation: bool,

    /// 是否启用 Path MTU Discovery (默认: false, 暂不支持)
    pub enable_pmtud: bool,

    /// 最大数据包大小 (默认: 65535)
    pub max_packet_size: u16,
}

impl Default for Ipv6Config {
    fn default() -> Self {
        Self {
            default_hop_limit: 64,
            min_mtu: 1280,
            max_extension_headers: 0,
            enable_fragmentation: false,
            enable_pmtud: false,
            max_packet_size: 65535,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default() {
        let config = Ipv6Config::default();
        assert_eq!(config.default_hop_limit, 64);
        assert_eq!(config.min_mtu, 1280);
        assert_eq!(config.max_extension_headers, 0);
        assert!(!config.enable_fragmentation);
        assert!(!config.enable_pmtud);
        assert_eq!(config.max_packet_size, 65535);
    }
}
