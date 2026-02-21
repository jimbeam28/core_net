// src/protocols/udp/config.rs
//
// UDP 协议配置

/// UDP 协议配置
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UdpConfig {
    /// 是否强制验证校验和（IPv4 中可选，默认启用）
    pub enforce_checksum: bool,
    /// 是否在端口不可达时发送 ICMP 消息
    pub send_icmp_unreachable: bool,
    /// 最大 UDP 数据报大小（受 MTU 限制）
    pub max_datagram_size: u16,
}

impl Default for UdpConfig {
    fn default() -> Self {
        Self {
            enforce_checksum: true,
            send_icmp_unreachable: true,
            max_datagram_size: 1472, // 1500 (以太网 MTU) - 20 (IP 头部) - 8 (UDP 头部)
        }
    }
}

impl UdpConfig {
    /// 创建新的 UDP 配置
    pub fn new() -> Self {
        Self::default()
    }

    /// 设置是否强制验证校验和
    pub fn with_enforce_checksum(mut self, enforce: bool) -> Self {
        self.enforce_checksum = enforce;
        self
    }

    /// 设置是否发送 ICMP 不可达消息
    pub fn with_send_icmp_unreachable(mut self, send: bool) -> Self {
        self.send_icmp_unreachable = send;
        self
    }

    /// 设置最大数据报大小
    pub fn with_max_datagram_size(mut self, size: u16) -> Self {
        self.max_datagram_size = size;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default() {
        let config = UdpConfig::default();
        assert!(config.enforce_checksum);
        assert!(config.send_icmp_unreachable);
        assert_eq!(config.max_datagram_size, 1472);
    }

    #[test]
    fn test_config_new() {
        let config = UdpConfig::new();
        assert_eq!(config, UdpConfig::default());
    }

    #[test]
    fn test_config_with_enforce_checksum() {
        let config = UdpConfig::new().with_enforce_checksum(false);
        assert!(!config.enforce_checksum);
        assert!(config.send_icmp_unreachable);
    }

    #[test]
    fn test_config_with_send_icmp_unreachable() {
        let config = UdpConfig::new().with_send_icmp_unreachable(false);
        assert!(config.enforce_checksum);
        assert!(!config.send_icmp_unreachable);
    }

    #[test]
    fn test_config_with_max_datagram_size() {
        let config = UdpConfig::new().with_max_datagram_size(1024);
        assert_eq!(config.max_datagram_size, 1024);
    }

    #[test]
    fn test_config_chain() {
        let config = UdpConfig::new()
            .with_enforce_checksum(false)
            .with_send_icmp_unreachable(false)
            .with_max_datagram_size(512);

        assert!(!config.enforce_checksum);
        assert!(!config.send_icmp_unreachable);
        assert_eq!(config.max_datagram_size, 512);
    }
}
