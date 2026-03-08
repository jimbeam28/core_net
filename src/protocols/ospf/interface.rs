// src/protocols/ospf/interface.rs
//
// OSPF 接口共享逻辑接口定义（简化版）

use crate::protocols::ospf::types::{InterfaceState, InterfaceType};
use std::time::Instant;

/// OSPF 接口常量
pub struct OspfInterfaceConstants;

impl OspfInterfaceConstants {
    /// 默认 Hello 间隔（秒）
    pub const DEFAULT_HELLO_INTERVAL: u16 = 10;
    /// 默认 Router Dead Interval（秒）
    pub const DEFAULT_DEAD_INTERVAL: u32 = 40;
    /// 默认路由器优先级
    pub const DEFAULT_PRIORITY: u8 = 1;
    /// 默认传输开销
    pub const DEFAULT_COST: u32 = 1;
}

/// OSPF 接口定时器状态（共享）
#[derive(Debug, Clone)]
pub struct SharedInterfaceTimers {
    /// Hello 定时器到期时间
    pub hello_expiry: Option<Instant>,
    /// Wait 定时器到期时间
    pub wait_expiry: Option<Instant>,
    /// 重传定时器到期时间
    pub rxmt_expiry: Option<Instant>,
}

impl SharedInterfaceTimers {
    /// 创建新的定时器状态
    pub fn new() -> Self {
        Self {
            hello_expiry: None,
            wait_expiry: None,
            rxmt_expiry: None,
        }
    }
}

impl Default for SharedInterfaceTimers {
    fn default() -> Self {
        Self::new()
    }
}

/// OSPF 接口 DR/BDR 状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DrBdrState<T> {
    /// 指定路由器 ID
    pub dr: T,
    /// 备份指定路由器 ID
    pub bdr: T,
}

impl<T: Copy + PartialEq> DrBdrState<T> {
    /// 创建新的 DR/BDR 状态
    pub fn new(dr: T, bdr: T) -> Self {
        Self { dr, bdr }
    }
}

/// OSPF Hello 参数验证结果
#[derive(Debug, Clone)]
pub struct HelloValidation {
    /// Hello 间隔是否匹配
    pub hello_match: bool,
    /// Dead Interval 是否匹配
    pub dead_match: bool,
}

impl HelloValidation {
    /// 验证 Hello 参数
    pub fn validate(
        local_hello: u16,
        local_dead: u32,
        received_hello: u16,
        received_dead: u32,
    ) -> Self {
        Self {
            hello_match: local_hello == received_hello,
            dead_match: local_dead == received_dead,
        }
    }

    /// 检查是否全部匹配
    pub fn is_valid(&self) -> bool {
        self.hello_match && self.dead_match
    }
}

/// OSPF 接口配置（共享）
#[derive(Debug, Clone)]
pub struct SharedInterfaceConfig {
    /// Hello 间隔（秒）
    pub hello_interval: u16,
    /// Router Dead Interval（秒）
    pub dead_interval: u32,
    /// 路由器优先级（0-255）
    pub priority: u8,
    /// 传输开销
    pub cost: u32,
    /// 是否是被动接口
    pub passive: bool,
    /// 重传间隔（秒）
    pub rxmt_interval: u32,
    /// 传输延迟（秒）
    pub transmit_delay: u32,
}

impl SharedInterfaceConfig {
    /// 创建默认配置
    pub fn default_config() -> Self {
        Self {
            hello_interval: OspfInterfaceConstants::DEFAULT_HELLO_INTERVAL,
            dead_interval: OspfInterfaceConstants::DEFAULT_DEAD_INTERVAL,
            priority: OspfInterfaceConstants::DEFAULT_PRIORITY,
            cost: OspfInterfaceConstants::DEFAULT_COST,
            passive: false,
            rxmt_interval: 5,
            transmit_delay: 1,
        }
    }
}

impl Default for SharedInterfaceConfig {
    fn default() -> Self {
        Self::default_config()
    }
}

/// OSPF 接口共享行为 trait
pub trait OspfInterfaceCommon {
    /// 获取接口名称
    fn name(&self) -> &str;
    /// 获取接口索引
    fn ifindex(&self) -> u32;
    /// 获取接口类型
    fn if_type(&self) -> InterfaceType;
    /// 获取接口状态
    fn state(&self) -> InterfaceState;
    /// 设置接口状态
    fn set_state(&mut self, state: InterfaceState);
    /// 获取 Hello 间隔
    fn hello_interval(&self) -> u16;
    /// 获取 Dead Interval
    fn dead_interval(&self) -> u32;
    /// 获取路由器优先级
    fn priority(&self) -> u8;
}
