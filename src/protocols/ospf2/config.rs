// src/protocols/ospf2/config.rs
//
// OSPFv2 配置定义

use crate::common::Ipv4Addr;
use super::error::OspfResult;

// 默认常量

/// 默认 Hello 间隔（秒）
pub const DEFAULT_HELLO_INTERVAL: u16 = 10;

/// 默认路由器死亡间隔（秒）
pub const DEFAULT_DEAD_INTERVAL: u32 = 40;

/// 默认路由器优先级
pub const DEFAULT_PRIORITY: u8 = 1;

/// 默认重传间隔（秒）
pub const DEFAULT_RETRANSMIT_INTERVAL: u32 = 5;

/// 默认传输延迟（秒）
pub const DEFAULT_TRANSMIT_DELAY: u32 = 1;

/// 默认接口 Cost（自动计算）
pub const DEFAULT_AUTO_COST: u32 = 1;

/// OSPFv2 接口配置
#[derive(Debug, Clone)]
pub struct OspfV2InterfaceConfig {
    /// 接口名称
    pub name: String,

    /// 区域 ID
    pub area_id: Ipv4Addr,

    /// 接口 IP 地址
    pub ip_addr: Option<Ipv4Addr>,

    /// 接口掩码
    pub mask: Option<Ipv4Addr>,

    /// Hello 间隔（秒）
    pub hello_interval: u16,

    /// 路由器死亡间隔（秒）
    pub dead_interval: u32,

    /// 路由器优先级（0-255）
    pub priority: u8,

    /// 接口 Cost
    pub cost: u32,

    /// 重传间隔（秒）
    pub retransmit_interval: u32,

    /// 传输延迟（秒）
    pub transmit_delay: u32,

    /// 是否被动接口
    pub passive: bool,

    /// 认证类型
    pub auth_type: u16,

    /// 认证密钥
    pub auth_key: Option<String>,
}

impl OspfV2InterfaceConfig {
    pub fn new(name: String, area_id: Ipv4Addr) -> Self {
        Self {
            name,
            area_id,
            ip_addr: None,
            mask: None,
            hello_interval: DEFAULT_HELLO_INTERVAL,
            dead_interval: DEFAULT_DEAD_INTERVAL,
            priority: DEFAULT_PRIORITY,
            cost: DEFAULT_AUTO_COST,
            retransmit_interval: DEFAULT_RETRANSMIT_INTERVAL,
            transmit_delay: DEFAULT_TRANSMIT_DELAY,
            passive: false,
            auth_type: 0,
            auth_key: None,
        }
    }

    pub fn validate(&self) -> OspfResult<()> {
        // Hello 间隔必须小于死亡间隔
        if self.hello_interval as u32 >= self.dead_interval {
            return Err(super::error::OspfError::ConfigError {
                parameter: "hello_interval".to_string(),
                reason: format!("must be less than dead_interval ({})", self.dead_interval),
            });
        }

        Ok(())
    }
}

impl Default for OspfV2InterfaceConfig {
    fn default() -> Self {
        Self::new("eth0".to_string(), Ipv4Addr::new(0, 0, 0, 0))
    }
}

/// OSPFv2 全局配置
#[derive(Debug, Clone)]
pub struct OspfV2Config {
    /// 路由器 ID
    pub router_id: Option<Ipv4Addr>,

    /// 是否启用 SPF 计算
    pub spf_enabled: bool,

    /// SPF 计算延迟（秒）
    pub spf_delay: u32,

    /// SPF 计算最小间隔（秒）
    pub spf_hold_time: u32,

    /// LSA 生成间隔（秒）
    pub lsa_generation_interval: u32,

    /// 最大邻居数
    pub max_neighbors: usize,

    /// 接口配置列表
    pub interfaces: Vec<OspfV2InterfaceConfig>,
}

impl OspfV2Config {
    pub fn new() -> Self {
        Self {
            router_id: None,
            spf_enabled: true,
            spf_delay: 5,
            spf_hold_time: 10,
            lsa_generation_interval: 5,
            max_neighbors: 100,
            interfaces: Vec::new(),
        }
    }

    pub fn validate(&self) -> OspfResult<()> {
        for (idx, iface) in self.interfaces.iter().enumerate() {
            iface.validate()?;

            // 检查接口名称唯一性
            for other in &self.interfaces[idx + 1..] {
                if iface.name == other.name {
                    return Err(super::error::OspfError::ConfigError {
                        parameter: "interface_name".to_string(),
                        reason: format!("duplicate interface name: {}", iface.name),
                    });
                }
            }
        }

        Ok(())
    }
}

impl Default for OspfV2Config {
    fn default() -> Self {
        Self::new()
    }
}
