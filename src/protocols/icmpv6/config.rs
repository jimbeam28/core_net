// src/protocols/icmpv6/config.rs
//
// ICMPv6 配置参数

use std::time::Duration;

/// ICMPv6 配置参数
#[derive(Debug, Clone)]
pub struct Icmpv6Config {
    // ========== 基本功能 ==========
    /// 是否响应 Echo Request (ping6)
    pub enable_echo_reply: bool,

    /// Echo 请求超时时间
    pub echo_timeout: Duration,

    /// 最大待处理 Echo 请求数量
    pub max_pending_echoes: usize,

    // ========== 邻居发现 ==========
    /// 是否接受 Router Advertisement
    pub accept_router_advertisements: bool,

    /// 是否发送 Router Solicitation
    pub send_router_solicitation: bool,

    /// Router Solicitation 延迟（秒）
    pub router_solicitation_delay: u32,

    /// 最大 Router Solicitation 重传次数
    pub max_rs_retransmissions: u32,

    /// 邻居缓存最大条目数
    pub max_neighbor_cache_entries: usize,

    /// 默认可达时间（毫秒）
    pub default_reachable_time: u32,

    /// 默认重传定时器（毫秒）
    pub default_retrans_timer: u32,

    /// 是否启用重复地址检测 (DAD)
    pub enable_dad: bool,

    /// DAD 传输次数
    pub dad_transmits: u32,

    /// DAD 超时时间（秒）
    pub dad_timeout: u32,

    // ========== 安全 ==========
    /// 是否接受 Redirect 消息
    pub accept_redirects: bool,

    /// 是否验证 NDP 消息的 Hop Limit = 255
    pub verify_hop_limit: bool,

    /// NDP 消息速率限制（每秒）
    pub ndp_rate_limit: u32,

    /// 是否丢弃包含未知选项的 NDP 消息
    pub drop_unknown_options: bool,

    // ========== PMTU ==========
    /// 是否启用路径 MTU 发现
    pub enable_pmtu_discovery: bool,

    /// PMTU 缓存超时时间（分钟）
    pub pmtu_cache_timeout: u32,

    // ========== MLD ==========
    /// 是否启用 MLD (Multicast Listener Discovery)
    pub enable_mld: bool,

    /// MLD 版本 (1 或 2)
    pub mld_version: u32,

    // ========== 速率限制 ==========
    /// ICMPv6 Error 消息发送速率限制（每秒）
    pub error_rate_limit: u32,
}

impl Default for Icmpv6Config {
    fn default() -> Self {
        Self {
            enable_echo_reply: true,
            echo_timeout: Duration::from_secs(1),
            max_pending_echoes: 100,

            accept_router_advertisements: true,
            send_router_solicitation: true,
            router_solicitation_delay: 1,
            max_rs_retransmissions: 3,
            max_neighbor_cache_entries: 256,
            default_reachable_time: 30000,
            default_retrans_timer: 1000,
            enable_dad: true,
            dad_transmits: 1,
            dad_timeout: 1,

            accept_redirects: false,
            verify_hop_limit: true,
            ndp_rate_limit: 10,
            drop_unknown_options: false,

            enable_pmtu_discovery: true,
            pmtu_cache_timeout: 10,

            enable_mld: false,
            mld_version: 2,

            error_rate_limit: 10,
        }
    }
}

/// 默认 ICMPv6 配置
pub const ICMPV6_CONFIG_DEFAULT: Icmpv6Config = Icmpv6Config {
    enable_echo_reply: true,
    echo_timeout: Duration::from_secs(1),
    max_pending_echoes: 100,

    accept_router_advertisements: true,
    send_router_solicitation: true,
    router_solicitation_delay: 1,
    max_rs_retransmissions: 3,
    max_neighbor_cache_entries: 256,
    default_reachable_time: 30000,
    default_retrans_timer: 1000,
    enable_dad: true,
    dad_transmits: 1,
    dad_timeout: 1,

    accept_redirects: false,
    verify_hop_limit: true,
    ndp_rate_limit: 10,
    drop_unknown_options: false,

    enable_pmtu_discovery: true,
    pmtu_cache_timeout: 10,

    enable_mld: false,
    mld_version: 2,

    error_rate_limit: 10,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = Icmpv6Config::default();

        assert!(config.enable_echo_reply);
        assert!(config.accept_router_advertisements);
        assert!(config.enable_dad);
        assert!(!config.accept_redirects);
        assert!(config.verify_hop_limit);
    }

    #[test]
    fn test_config_const() {
        const { assert!(ICMPV6_CONFIG_DEFAULT.enable_echo_reply) };
        const { assert!(ICMPV6_CONFIG_DEFAULT.accept_router_advertisements) };
        assert_eq!(ICMPV6_CONFIG_DEFAULT.max_neighbor_cache_entries, 256);
    }
}
