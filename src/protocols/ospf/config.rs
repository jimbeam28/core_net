// src/protocols/ospf/config.rs
//
// OSPF 配置定义

use crate::common::Ipv4Addr;

// OSPF 默认常量

/// 默认 Hello 间隔（秒）
pub const HELLO_INTERVAL_DEFAULT: u16 = 10;

/// 默认路由器死亡间隔（秒）
pub const DEAD_INTERVAL_DEFAULT: u32 = 40;

/// 默认路由器优先级
pub const PRIORITY_DEFAULT: u8 = 1;

/// 默认重传间隔（秒）
pub const RETRANSMIT_INTERVAL_DEFAULT: u32 = 5;

/// 默认传输延迟（秒）
pub const TRANSMIT_DELAY_DEFAULT: u32 = 1;

/// 默认 SPF 计算延迟（秒）
pub const SPF_DELAY_DEFAULT: u32 = 5;

/// 默认 SPF 计算最小间隔（秒）
pub const SPF_HOLD_TIME_DEFAULT: u32 = 10;

/// 默认 LSA 生成间隔（秒）
pub const LSA_GENERATION_INTERVAL_DEFAULT: u32 = 5;

/// 默认最大邻居数
pub const MAX_NEIGHBORS_DEFAULT: usize = 100;

/// LSA 最大年龄（秒）
pub const LSA_MAX_AGE: u16 = 3600;

/// LSA 刷新间隔（秒）
pub const LSA_REFRESH_INTERVAL: u16 = 1800;

/// OSPF 认证算法
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthAlgorithm {
    /// HMAC-MD5
    HmacMd5,
    /// HMAC-SHA1
    HmacSha1,
    /// HMAC-SHA256
    HmacSha256,
    /// HMAC-SHA384
    HmacSha384,
    /// HMAC-SHA512
    HmacSha512,
}

impl AuthAlgorithm {
    pub fn name(&self) -> &'static str {
        match self {
            AuthAlgorithm::HmacMd5 => "HMAC-MD5",
            AuthAlgorithm::HmacSha1 => "HMAC-SHA1",
            AuthAlgorithm::HmacSha256 => "HMAC-SHA256",
            AuthAlgorithm::HmacSha384 => "HMAC-SHA384",
            AuthAlgorithm::HmacSha512 => "HMAC-SHA512",
        }
    }

    /// 密钥长度（字节）
    pub fn key_length(&self) -> usize {
        match self {
            AuthAlgorithm::HmacMd5 => 16,
            AuthAlgorithm::HmacSha1 => 20,
            AuthAlgorithm::HmacSha256 => 32,
            AuthAlgorithm::HmacSha384 => 48,
            AuthAlgorithm::HmacSha512 => 64,
        }
    }
}

/// 加密认证配置
#[derive(Debug, Clone)]
pub struct CryptoAuthConfig {
    /// 认证算法
    pub algorithm: AuthAlgorithm,
    /// 认证密钥 ID
    pub key_id: u8,
    /// 认证密钥
    pub key: Vec<u8>,
}

impl CryptoAuthConfig {
    pub fn new(algorithm: AuthAlgorithm, key_id: u8, key: Vec<u8>) -> Self {
        Self {
            algorithm,
            key_id,
            key,
        }
    }

    /// 验证密钥长度
    pub fn is_valid_key_length(&self) -> bool {
        self.key.len() >= self.algorithm.key_length()
    }
}

/// OSPF 接口配置
#[derive(Debug, Clone)]
pub struct OspfInterfaceConfig {
    /// 接口名称
    pub name: String,

    /// 区域 ID
    pub area_id: Ipv4Addr,

    /// 接口类型
    pub if_type: super::types::InterfaceType,

    /// Hello 间隔（秒）
    pub hello_interval: u16,

    /// 路由器死亡间隔（秒）
    pub dead_interval: u32,

    /// 路由器优先级（0-255），0 表示不参与 DR 选举
    pub priority: u8,

    /// 接口 Cost，None 表示根据带宽自动计算
    pub cost: Option<u32>,

    /// 重传间隔（秒）
    pub retransmit_interval: u32,

    /// 传输延迟（秒）
    pub transmit_delay: u32,

    /// 是否被动接口（只接收不发送）
    pub passive: bool,

    /// 认证类型 (0=None, 1=Simple, 2=Crypto)
    pub auth_type: u16,

    /// 简单认证密码（Type 1）
    pub auth_key: Option<String>,

    /// 加密认证配置（Type 2）
    pub crypto_auth: Option<CryptoAuthConfig>,
}

impl OspfInterfaceConfig {
    pub fn new(name: String, area_id: Ipv4Addr) -> Self {
        Self {
            name,
            area_id,
            if_type: super::types::InterfaceType::Broadcast,
            hello_interval: HELLO_INTERVAL_DEFAULT,
            dead_interval: DEAD_INTERVAL_DEFAULT,
            priority: PRIORITY_DEFAULT,
            cost: None,
            retransmit_interval: RETRANSMIT_INTERVAL_DEFAULT,
            transmit_delay: TRANSMIT_DELAY_DEFAULT,
            passive: false,
            auth_type: 0,
            auth_key: None,
            crypto_auth: None,
        }
    }

    /// 设置接口类型
    pub fn with_interface_type(mut self, if_type: super::types::InterfaceType) -> Self {
        self.if_type = if_type;
        self
    }

    /// 设置 Hello 间隔
    pub fn with_hello_interval(mut self, interval: u16) -> Self {
        self.hello_interval = interval;
        self
    }

    /// 设置死亡间隔
    pub fn with_dead_interval(mut self, interval: u32) -> Self {
        self.dead_interval = interval;
        self
    }

    /// 设置优先级
    pub fn with_priority(mut self, priority: u8) -> Self {
        self.priority = priority;
        self
    }

    /// 设置 Cost
    pub fn with_cost(mut self, cost: u32) -> Self {
        self.cost = Some(cost);
        self
    }

    /// 设置被动模式
    pub fn with_passive(mut self, passive: bool) -> Self {
        self.passive = passive;
        self
    }

    /// 设置认证密钥（简单认证）
    pub fn with_auth_key(mut self, key: String) -> Self {
        self.auth_type = 1;
        self.auth_key = Some(key);
        self
    }

    /// 设置加密认证
    pub fn with_crypto_auth(mut self, auth: CryptoAuthConfig) -> Self {
        self.auth_type = 2;
        self.crypto_auth = Some(auth);
        self
    }

    /// 验证配置
    pub fn validate(&self) -> Result<(), String> {
        // Hello 间隔必须小于死亡间隔
        if self.hello_interval as u32 >= self.dead_interval {
            return Err(format!(
                "Hello interval ({}) must be less than Dead interval ({})",
                self.hello_interval, self.dead_interval
            ));
        }

        // 认证配置检查
        if self.auth_type == 2
            && let Some(ref crypto) = self.crypto_auth
            && !crypto.is_valid_key_length()
        {
            return Err(format!(
                "Invalid key length for algorithm {:?}",
                crypto.algorithm
            ));
        }

        Ok(())
    }

    /// 是否需要认证
    pub fn requires_auth(&self) -> bool {
        self.auth_type > 0
    }
}

impl Default for OspfInterfaceConfig {
    fn default() -> Self {
        Self::new("eth0".to_string(), Ipv4Addr::new(0, 0, 0, 0))
    }
}

/// OSPF 全局配置
#[derive(Debug, Clone)]
pub struct OspfConfig {
    /// 路由器 ID（如果未配置，自动选择最大 IP 地址）
    pub router_id: Option<Ipv4Addr>,

    /// 是否启用 SPF 计算（用于调试）
    pub spf_enabled: bool,

    /// SPF 计算延迟时间（秒）
    pub spf_delay: u32,

    /// SPF 计算最小间隔时间（秒）
    pub spf_hold_time: u32,

    /// LSA 生成间隔时间（秒）
    pub lsa_generation_interval: u32,

    /// 单个接口最大邻居数
    pub max_neighbors: usize,

    /// OSPF 启用的接口列表
    pub interfaces: Vec<OspfInterfaceConfig>,

    /// 默认认证配置
    pub default_auth_type: u16,

    /// 默认加密认证配置
    pub default_crypto_auth: Option<CryptoAuthConfig>,
}

impl OspfConfig {
    pub fn new() -> Self {
        Self {
            router_id: None,
            spf_enabled: true,
            spf_delay: SPF_DELAY_DEFAULT,
            spf_hold_time: SPF_HOLD_TIME_DEFAULT,
            lsa_generation_interval: LSA_GENERATION_INTERVAL_DEFAULT,
            max_neighbors: MAX_NEIGHBORS_DEFAULT,
            interfaces: Vec::new(),
            default_auth_type: 0,
            default_crypto_auth: None,
        }
    }

    /// 设置路由器 ID
    pub fn with_router_id(mut self, router_id: Ipv4Addr) -> Self {
        self.router_id = Some(router_id);
        self
    }

    /// 添加接口配置
    pub fn with_interface(mut self, interface: OspfInterfaceConfig) -> Self {
        self.interfaces.push(interface);
        self
    }

    /// 禁用 SPF 计算（用于调试）
    pub fn with_spf_disabled(mut self) -> Self {
        self.spf_enabled = false;
        self
    }

    /// 验证配置
    pub fn validate(&self) -> Result<(), String> {
        // 验证每个接口配置
        for (idx, iface) in self.interfaces.iter().enumerate() {
            iface.validate().map_err(|e| {
                format!("Interface {} validation failed: {}", iface.name, e)
            })?;

            // 检查接口名称唯一性
            for other in &self.interfaces[idx + 1..] {
                if iface.name == other.name {
                    return Err(format!("Duplicate interface name: {}", iface.name));
                }
            }
        }

        Ok(())
    }

    /// 获取或自动选择 Router ID
    ///
    /// 如果未配置，选择所有接口中最大的 IP 地址
    pub fn get_or_select_router_id(&self) -> Option<Ipv4Addr> {
        if let Some(rid) = self.router_id {
            return Some(rid);
        }

        // 选择所有接口中最大的 IP 地址
        self.interfaces.iter()
            .filter_map(|_iface| {
                // 这里假设接口有 IP 地址，实际实现需要从接口管理器获取
                None  // 需要外部提供接口 IP 信息
            })
            .max()
    }
}

impl Default for OspfConfig {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocols::ospf::types::InterfaceType;

    #[test]
    fn test_ospf_interface_config_new() {
        let config = OspfInterfaceConfig::new("eth0".to_string(), Ipv4Addr::new(0, 0, 0, 0));
        assert_eq!(config.name, "eth0");
        assert_eq!(config.area_id, Ipv4Addr::new(0, 0, 0, 0));
        assert_eq!(config.hello_interval, HELLO_INTERVAL_DEFAULT);
        assert_eq!(config.dead_interval, DEAD_INTERVAL_DEFAULT);
    }

    #[test]
    fn test_ospf_interface_config_builder() {
        let config = OspfInterfaceConfig::new("eth0".to_string(), Ipv4Addr::new(0, 0, 0, 1))
            .with_interface_type(InterfaceType::PointToPoint)
            .with_hello_interval(20)
            .with_dead_interval(80)
            .with_priority(100)
            .with_cost(10)
            .with_passive(true);

        assert_eq!(config.if_type, InterfaceType::PointToPoint);
        assert_eq!(config.hello_interval, 20);
        assert_eq!(config.dead_interval, 80);
        assert_eq!(config.priority, 100);
        assert_eq!(config.cost, Some(10));
        assert!(config.passive);
    }

    #[test]
    fn test_ospf_interface_config_validate() {
        // 有效配置
        let config = OspfInterfaceConfig::new("eth0".to_string(), Ipv4Addr::new(0, 0, 0, 0))
            .with_hello_interval(10)
            .with_dead_interval(40);
        assert!(config.validate().is_ok());

        // Hello 间隔 >= 死亡间隔
        let config = OspfInterfaceConfig::new("eth0".to_string(), Ipv4Addr::new(0, 0, 0, 0))
            .with_hello_interval(40)
            .with_dead_interval(40);
        assert!(config.validate().is_err());

        // 优先级最大值 (u8::MAX = 255) 是有效的
        let config = OspfInterfaceConfig::new("eth0".to_string(), Ipv4Addr::new(0, 0, 0, 0))
            .with_priority(255);
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_ospf_config_new() {
        let config = OspfConfig::new();
        assert!(config.router_id.is_none());
        assert!(config.spf_enabled);
        assert_eq!(config.spf_delay, SPF_DELAY_DEFAULT);
        assert!(config.interfaces.is_empty());
    }

    #[test]
    fn test_ospf_config_builder() {
        let iface = OspfInterfaceConfig::new("eth0".to_string(), Ipv4Addr::new(0, 0, 0, 1));
        let config = OspfConfig::new()
            .with_router_id(Ipv4Addr::new(1, 1, 1, 1))
            .with_spf_disabled()
            .with_interface(iface);

        assert_eq!(config.router_id, Some(Ipv4Addr::new(1, 1, 1, 1)));
        assert!(!config.spf_enabled);
        assert_eq!(config.interfaces.len(), 1);
    }

    #[test]
    fn test_ospf_config_validate_duplicate_interface() {
        let iface1 = OspfInterfaceConfig::new("eth0".to_string(), Ipv4Addr::new(0, 0, 0, 1));
        let iface2 = OspfInterfaceConfig::new("eth0".to_string(), Ipv4Addr::new(0, 0, 0, 2));

        let config = OspfConfig::new()
            .with_interface(iface1)
            .with_interface(iface2);

        assert!(config.validate().is_err());
    }

    #[test]
    fn test_auth_algorithm_key_length() {
        assert_eq!(AuthAlgorithm::HmacMd5.key_length(), 16);
        assert_eq!(AuthAlgorithm::HmacSha1.key_length(), 20);
        assert_eq!(AuthAlgorithm::HmacSha256.key_length(), 32);
        assert_eq!(AuthAlgorithm::HmacSha384.key_length(), 48);
        assert_eq!(AuthAlgorithm::HmacSha512.key_length(), 64);
    }

    #[test]
    fn test_crypto_auth_config_is_valid_key_length() {
        let auth = CryptoAuthConfig::new(
            AuthAlgorithm::HmacMd5,
            1,
            vec![0u8; 16],
        );
        assert!(auth.is_valid_key_length());

        let auth = CryptoAuthConfig::new(
            AuthAlgorithm::HmacMd5,
            1,
            vec![0u8; 8],
        );
        assert!(!auth.is_valid_key_length());
    }
}
