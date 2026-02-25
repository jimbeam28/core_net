// src/protocols/ospf3/config.rs
//
// OSPFv3 配置

use std::fmt;
use crate::common::Ipv6Addr;
use crate::protocols::ospf::{InterfaceType};

/// OSPFv3 接口配置
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ospfv3InterfaceConfig {
    /// 接口名称
    pub name: String,
    /// 接口索引
    pub ifindex: u32,
    /// 区域 ID
    pub area_id: Ipv6Addr,
    /// 接口类型
    pub if_type: InterfaceType,
    /// Hello 间隔（秒）
    pub hello_interval: u16,
    /// 路由器死亡间隔（秒）
    pub dead_interval: u32,
    /// 路由器优先级
    pub priority: u8,
    /// 传输开销
    pub cost: Option<u32>,
    /// 被动接口（不发送 OSPF 报文）
    pub passive: bool,
}

impl Ospfv3InterfaceConfig {
    pub fn new(name: String, area_id: Ipv6Addr) -> Self {
        Self {
            name,
            ifindex: 0,
            area_id,
            if_type: InterfaceType::Broadcast,
            hello_interval: 10,
            dead_interval: 40,
            priority: 1,
            cost: None,
            passive: false,
        }
    }

    pub fn with_interface_type(mut self, if_type: InterfaceType) -> Self {
        self.if_type = if_type;
        self
    }

    pub fn with_hello_interval(mut self, interval: u16) -> Self {
        self.hello_interval = interval;
        self
    }

    pub fn with_dead_interval(mut self, interval: u32) -> Self {
        self.dead_interval = interval;
        self
    }

    pub fn with_priority(mut self, priority: u8) -> Self {
        self.priority = priority;
        self
    }

    pub fn with_cost(mut self, cost: u32) -> Self {
        self.cost = Some(cost);
        self
    }

    pub fn with_passive(mut self, passive: bool) -> Self {
        self.passive = passive;
        self
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.hello_interval < 1 {
            return Err("Hello 间隔必须 >= 1".to_string());
        }

        if self.dead_interval < 1 {
            return Err("死亡间隔必须 >= 1".to_string());
        }

        if self.dead_interval <= self.hello_interval as u32 {
            return Err("死亡间隔必须大于 Hello 间隔".to_string());
        }

        Ok(())
    }
}

impl fmt::Display for Ospfv3InterfaceConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "OSPFv3Interface({}, area={})", self.name, self.area_id)
    }
}
