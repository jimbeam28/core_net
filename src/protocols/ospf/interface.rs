// src/protocols/ospf/interface.rs
//
// OSPF 接口共享逻辑
// 定义 OSPFv2 和 OSPFv3 共享的接口行为 trait

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

    /// 最小 Hello 间隔
    pub const MIN_HELLO_INTERVAL: u16 = 1;

    /// 最大 Hello 间隔
    pub const MAX_HELLO_INTERVAL: u16 = 65535;

    /// 最小 Dead Interval
    pub const MIN_DEAD_INTERVAL: u32 = 2;

    /// Wait Timer = Router Dead Interval
    pub fn wait_interval(dead_interval: u32) -> u32 {
        dead_interval
    }
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

    /// 重置 Hello 定时器
    pub fn reset_hello_timer(&mut self, interval: u16) {
        self.hello_expiry = Some(
            Instant::now()
                .checked_add(std::time::Duration::from_secs(interval as u64))
                .unwrap_or(Instant::now())
        );
    }

    /// 检查 Hello 定时器是否超时
    pub fn is_hello_expired(&self) -> bool {
        if let Some(expiry) = self.hello_expiry {
            Instant::now() > expiry
        } else {
            false
        }
    }

    /// 启动 Wait 定时器
    pub fn start_wait_timer(&mut self, dead_interval: u32) {
        self.wait_expiry = Some(
            Instant::now()
                .checked_add(std::time::Duration::from_secs(dead_interval as u64))
                .unwrap_or(Instant::now())
        );
    }

    /// 检查 Wait 定时器是否超时
    pub fn is_wait_expired(&self) -> bool {
        if let Some(expiry) = self.wait_expiry {
            Instant::now() > expiry
        } else {
            false
        }
    }

    /// 清除 Wait 定时器
    pub fn clear_wait_timer(&mut self) {
        self.wait_expiry = None;
    }

    /// 启动重传定时器
    pub fn start_rxmt_timer(&mut self, interval: u32) {
        self.rxmt_expiry = Some(
            Instant::now()
                .checked_add(std::time::Duration::from_secs(interval as u64))
                .unwrap_or(Instant::now())
        );
    }

    /// 检查重传定时器是否超时
    pub fn is_rxmt_expired(&self) -> bool {
        if let Some(expiry) = self.rxmt_expiry {
            Instant::now() > expiry
        } else {
            false
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

    /// 设置 DR
    pub fn set_dr(&mut self, dr: T) {
        self.dr = dr;
    }

    /// 设置 BDR
    pub fn set_bdr(&mut self, bdr: T) {
        self.bdr = bdr;
    }

    /// 检查是否是 DR
    pub fn is_dr(&self, router_id: T) -> bool {
        self.dr == router_id
    }

    /// 检查是否是 BDR
    pub fn is_bdr(&self, router_id: T) -> bool {
        self.bdr == router_id
    }

    /// 清空 DR/BDR
    pub fn clear(&mut self)
    where
        T: Default,
    {
        self.dr = T::default();
        self.bdr = T::default();
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

    /// 验证配置参数
    pub fn validate(&self) -> Result<(), String> {
        if self.hello_interval < OspfInterfaceConstants::MIN_HELLO_INTERVAL
            || self.hello_interval > OspfInterfaceConstants::MAX_HELLO_INTERVAL
        {
            return Err(format!(
                "Hello interval must be between {} and {}",
                OspfInterfaceConstants::MIN_HELLO_INTERVAL,
                OspfInterfaceConstants::MAX_HELLO_INTERVAL
            ));
        }

        if self.dead_interval < OspfInterfaceConstants::MIN_DEAD_INTERVAL {
            return Err(format!(
                "Dead interval must be at least {}",
                OspfInterfaceConstants::MIN_DEAD_INTERVAL
            ));
        }

        // Dead Interval 应该是 Hello Interval 的倍数（通常是 4 倍）
        if self.dead_interval < self.hello_interval as u32 * 2 {
            return Err(
                "Dead interval should be at least 2x hello interval".to_string()
            );
        }

        if self.priority > 255 {
            return Err("Priority must be between 0 and 255".to_string());
        }

        Ok(())
    }
}

impl Default for SharedInterfaceConfig {
    fn default() -> Self {
        Self::default_config()
    }
}

/// OSPF 接口共享行为 trait
///
/// 此 trait 定义了 OSPFv2 和 OSPFv3 接口的共同行为
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

    /// 判断是否有资格参与 DR 选举
    fn is_eligible_for_dr(&self) -> bool {
        self.priority() > 0 && !self.if_type().is_point_to_point()
    }

    /// 判断接口是否启用
    fn is_up(&self) -> bool {
        self.state() != InterfaceState::Down
    }

    /// 验证 Hello 参数
    fn validate_hello_params(
        &self,
        hello_interval: u16,
        dead_interval: u32,
    ) -> Result<(), String> {
        if hello_interval != self.hello_interval() {
            return Err(format!(
                "Hello interval mismatch: expected {}, received {}",
                self.hello_interval(),
                hello_interval
            ));
        }

        if dead_interval != self.dead_interval() {
            return Err(format!(
                "Dead interval mismatch: expected {}, received {}",
                self.dead_interval(),
                dead_interval
            ));
        }

        Ok(())
    }

    /// 判断是否需要发送 Hello 报文
    fn should_send_hello(&self) -> bool {
        // 被动接口不发送 Hello
        if self.is_passive() {
            return false;
        }

        // 接口必须处于 Up 状态
        self.is_up()
    }

    /// 判断是否是被动接口
    fn is_passive(&self) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_interface_timers() {
        let mut timers = SharedInterfaceTimers::new();

        // 测试 Hello 定时器
        timers.reset_hello_timer(10);
        assert!(!timers.is_hello_expired());

        // 测试 Wait 定时器
        timers.start_wait_timer(40);
        assert!(!timers.is_wait_expired());

        timers.clear_wait_timer();
        assert!(!timers.is_wait_expired());

        // 测试重传定时器
        timers.start_rxmt_timer(5);
        assert!(!timers.is_rxmt_expired());
    }

    #[test]
    fn test_dr_bdr_state() {
        let mut state = DrBdrState::new(1u32, 2u32);

        assert!(state.is_dr(1));
        assert!(state.is_bdr(2));
        assert!(!state.is_dr(2));

        state.set_dr(3);
        assert!(state.is_dr(3));

        state.clear();
        assert!(!state.is_dr(3));
    }

    #[test]
    fn test_hello_validation() {
        let validation = HelloValidation::validate(10, 40, 10, 40);
        assert!(validation.is_valid());

        let validation = HelloValidation::validate(10, 40, 5, 40);
        assert!(!validation.is_valid());
        assert!(!validation.hello_match);

        let validation = HelloValidation::validate(10, 40, 10, 30);
        assert!(!validation.is_valid());
        assert!(!validation.dead_match);
    }

    #[test]
    fn test_interface_config_default() {
        let config = SharedInterfaceConfig::default_config();
        assert_eq!(config.hello_interval, 10);
        assert_eq!(config.dead_interval, 40);
        assert_eq!(config.priority, 1);
        assert_eq!(config.cost, 1);
        assert!(!config.passive);
    }

    #[test]
    fn test_interface_config_validate() {
        let config = SharedInterfaceConfig::default_config();
        assert!(config.validate().is_ok());

        // Hello interval 太小
        let mut invalid = config.clone();
        invalid.hello_interval = 0;
        assert!(invalid.validate().is_err());

        // Dead interval 太小
        invalid.hello_interval = 10;
        invalid.dead_interval = 1;
        assert!(invalid.validate().is_err());

        // Priority 最大值测试（255 是有效的）
        let mut valid_priority = config.clone();
        valid_priority.priority = 255;
        assert!(valid_priority.validate().is_ok());
    }

    #[test]
    fn test_constants() {
        assert_eq!(OspfInterfaceConstants::DEFAULT_HELLO_INTERVAL, 10);
        assert_eq!(OspfInterfaceConstants::DEFAULT_DEAD_INTERVAL, 40);
        assert_eq!(OspfInterfaceConstants::DEFAULT_PRIORITY, 1);
        assert_eq!(OspfInterfaceConstants::wait_interval(40), 40);
    }
}
