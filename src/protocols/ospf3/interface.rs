// src/protocols/ospf3/interface.rs
//
// OSPFv3 接口状态和逻辑

use crate::common::Ipv6Addr;
use crate::protocols::ospf::{InterfaceState, InterfaceType};
use std::time::Instant;

/// OSPFv3 接口
#[derive(Debug, Clone)]
pub struct Ospfv3Interface {
    /// 接口名称
    pub name: String,
    /// 接口索引
    pub ifindex: u32,
    /// 接口 IPv6 地址
    pub ip_addr: Ipv6Addr,
    /// 区域 ID (32-bit)
    pub area_id: u32,
    /// 接口类型
    pub if_type: InterfaceType,
    /// 接口状态
    pub state: InterfaceState,
    /// Hello 间隔（秒）
    pub hello_interval: u16,
    /// 路由器死亡间隔（秒）
    pub dead_interval: u32,
    /// 路由器优先级
    pub priority: u8,
    /// 传输开销
    pub cost: u32,
    /// 指定路由器 ID (32-bit)
    pub dr: u32,
    /// 备份指定路由器 ID (32-bit)
    pub bdr: u32,
    /// Hello 定时器
    pub hello_timer: Option<Instant>,
    /// Wait 定时器
    pub wait_timer: Option<Instant>,
    /// 被动接口
    pub passive: bool,
}

impl Ospfv3Interface {
    pub const DEFAULT_COST: u32 = 1;
    pub const DEFAULT_PRIORITY: u8 = 1;

    pub fn new(
        name: String,
        ifindex: u32,
        ip_addr: Ipv6Addr,
        area_id: u32,
    ) -> Self {
        Self {
            name,
            ifindex,
            ip_addr,
            area_id,
            if_type: InterfaceType::Broadcast,
            state: InterfaceState::Down,
            hello_interval: 10,
            dead_interval: 40,
            priority: Self::DEFAULT_PRIORITY,
            cost: Self::DEFAULT_COST,
            dr: 0,
            bdr: 0,
            hello_timer: None,
            wait_timer: None,
            passive: false,
        }
    }

    /// 启动接口
    pub fn up(&mut self) {
        if self.state == InterfaceState::Down {
            if self.if_type.is_point_to_point() {
                self.state = InterfaceState::PointToPoint;
            } else {
                self.state = InterfaceState::Waiting;
                self.wait_timer = Some(
                    Instant::now()
                        .checked_add(std::time::Duration::from_secs(self.dead_interval as u64))
                        .unwrap_or(Instant::now())
                );
            }
            self.reset_hello_timer();
        }
    }

    /// 关闭接口
    pub fn down(&mut self) {
        self.state = InterfaceState::Down;
        self.dr = 0;
        self.bdr = 0;
        self.hello_timer = None;
        self.wait_timer = None;
    }

    /// 重置 Hello 定时器
    pub fn reset_hello_timer(&mut self) {
        self.hello_timer = Some(
            Instant::now()
                .checked_add(std::time::Duration::from_secs(self.hello_interval as u64))
                .unwrap_or(Instant::now())
        );
    }

    /// 检查 Hello 定时器是否超时
    pub fn is_hello_timer_expired(&self) -> bool {
        if let Some(timer) = self.hello_timer {
            Instant::now() > timer
        } else {
            false
        }
    }

    /// 设置 DR
    pub fn set_dr(&mut self, dr: u32) {
        self.dr = dr;
    }

    /// 设置 BDR
    pub fn set_bdr(&mut self, bdr: u32) {
        self.bdr = bdr;
    }

    /// 是否是 DR
    pub fn is_dr(&self, router_id: u32) -> bool {
        self.dr == router_id
    }

    /// 是否是 BDR
    pub fn is_bdr(&self, router_id: u32) -> bool {
        self.bdr == router_id
    }

    /// 是否有资格参与 DR 选举
    pub fn is_eligible_for_dr(&self) -> bool {
        self.priority > 0 && !self.if_type.is_point_to_point()
    }

    /// 验证 Hello 参数
    pub fn validate_hello_params(
        &self,
        hello_interval: u16,
        dead_interval: u32,
    ) -> super::error::Ospfv3Result<()> {
        use super::error::Ospfv3Error;

        if hello_interval != self.hello_interval {
            return Err(Ospfv3Error::HelloMismatch {
                expected: self.hello_interval,
                received: hello_interval,
            });
        }

        if dead_interval != self.dead_interval {
            return Err(Ospfv3Error::DeadIntervalMismatch {
                expected: self.dead_interval,
                received: dead_interval,
            });
        }

        Ok(())
    }
}
