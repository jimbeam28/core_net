// src/protocols/tcp/config.rs
//
// TCP 协议配置

/// TCP 协议配置
#[derive(Debug, Clone)]
pub struct TcpConfig {
    // ========== 基本配置 ==========

    /// 最大分段大小（MSS），默认 1460 字节（以太网 MTU 1500 - IP 20 - TCP 20）
    pub max_segment_size: u16,

    /// 默认接收窗口大小，默认 65535 字节
    pub default_window_size: u16,

    /// 最小接收窗口大小，默认 1460 字节（1 MSS）
    pub min_window_size: u16,

    // ========== 超时配置 ==========

    /// 初始重传超时时间（RTO），默认 1 秒
    pub initial_rto: u32,

    /// 最小 RTO，默认 200ms（RFC2988 建议）
    pub min_rto: u32,

    /// 最大 RTO，默认 120 秒（RFC2988 建议）
    pub max_rto: u32,

    /// TIME_WAIT 状态持续时间（2MSL），默认 60 秒
    pub time_wait_duration: u32,

    /// 延迟 ACK 定时器，默认 200ms
    pub delayed_ack_timeout: u32,

    // ========== 拥塞控制配置 ==========

    /// 初始拥塞窗口，默认 10 * MSS（RFC6928）
    pub initial_cwnd: u32,

    /// 初始慢启动阈值，默认无限大
    pub initial_ssthresh: u32,

    // ========== 连接限制 ==========

    /// 最大连接数（包括 TIME_WAIT），默认 1000
    pub max_connections: usize,

    /// 最大半连接数（SYN_RCVD），默认 100
    pub max_half_connections: usize,

    /// 最大重传次数，默认 12 次（约 9 分钟）
    pub max_retransmit_attempts: u32,

    // ========== 功能开关 ==========

    /// 是否启用窗口缩放，默认 true
    pub enable_window_scale: bool,

    /// 是否启用 SACK，默认 true
    pub enable_sack: bool,

    /// 是否启用时间戳，默认 true
    pub enable_timestamps: bool,

    /// 是否启用延迟 ACK，默认 true
    pub enable_delayed_ack: bool,

    /// 是否启用 SYN Cookies（防御 SYN Flood），默认 false
    pub enable_syn_cookies: bool,
}

impl TcpConfig {
    /// 创建新的 TCP 配置（使用默认值）
    pub fn new() -> Self {
        Self::default()
    }

    /// 设置 MSS
    pub fn with_max_segment_size(mut self, mss: u16) -> Self {
        self.max_segment_size = mss;
        self
    }

    /// 设置默认窗口大小
    pub fn with_default_window_size(mut self, size: u16) -> Self {
        self.default_window_size = size;
        self
    }

    /// 设置初始 RTO
    pub fn with_initial_rto(mut self, rto: u32) -> Self {
        self.initial_rto = rto;
        self
    }

    /// 设置最大重传次数
    pub fn with_max_retransmit_attempts(mut self, attempts: u32) -> Self {
        self.max_retransmit_attempts = attempts;
        self
    }
}

impl Default for TcpConfig {
    fn default() -> Self {
        Self {
            max_segment_size: 1460,
            default_window_size: 65535,
            min_window_size: 1460,
            initial_rto: 1000,
            min_rto: 200,
            max_rto: 120000,
            time_wait_duration: 60000,
            delayed_ack_timeout: 200,
            initial_cwnd: 14600,
            initial_ssthresh: u32::MAX,
            max_connections: 1000,
            max_half_connections: 100,
            max_retransmit_attempts: 12,
            enable_window_scale: true,
            enable_sack: true,
            enable_timestamps: true,
            enable_delayed_ack: true,
            enable_syn_cookies: false,
        }
    }
}

/// 默认 TCP 配置
pub const TCP_CONFIG_DEFAULT: TcpConfig = TcpConfig {
    max_segment_size: 1460,
    default_window_size: 65535,
    min_window_size: 1460,
    initial_rto: 1000,
    min_rto: 200,
    max_rto: 120000,
    time_wait_duration: 60000,
    delayed_ack_timeout: 200,
    initial_cwnd: 14600,
    initial_ssthresh: u32::MAX,
    max_connections: 1000,
    max_half_connections: 100,
    max_retransmit_attempts: 12,
    enable_window_scale: true,
    enable_sack: true,
    enable_timestamps: true,
    enable_delayed_ack: true,
    enable_syn_cookies: false,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default() {
        let config = TcpConfig::default();
        assert_eq!(config.max_segment_size, 1460);
        assert_eq!(config.default_window_size, 65535);
        assert_eq!(config.initial_rto, 1000);
    }

    #[test]
    fn test_config_new() {
        let config = TcpConfig::new();
        assert_eq!(config.max_segment_size, 1460);
    }

    #[test]
    fn test_config_with_max_segment_size() {
        let config = TcpConfig::new().with_max_segment_size(1400);
        assert_eq!(config.max_segment_size, 1400);
    }

    #[test]
    fn test_config_with_default_window_size() {
        let config = TcpConfig::new().with_default_window_size(32768);
        assert_eq!(config.default_window_size, 32768);
    }

    #[test]
    fn test_config_with_initial_rto() {
        let config = TcpConfig::new().with_initial_rto(2000);
        assert_eq!(config.initial_rto, 2000);
    }

    #[test]
    fn test_config_with_max_retransmit_attempts() {
        let config = TcpConfig::new().with_max_retransmit_attempts(8);
        assert_eq!(config.max_retransmit_attempts, 8);
    }

    #[test]
    fn test_config_const_default() {
        assert_eq!(TCP_CONFIG_DEFAULT.max_segment_size, 1460);
        assert_eq!(TCP_CONFIG_DEFAULT.default_window_size, 65535);
    }
}
