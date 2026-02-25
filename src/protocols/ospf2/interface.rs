// src/protocols/ospf2/interface.rs
//
// OSPFv2 接口状态机

use crate::common::Ipv4Addr;
use super::error::OspfResult;

/// 接口状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InterfaceState {
    Down,
    Loopback,
    Waiting,
    PointToPoint,
    DROther,
    DR,
    Backup,
}

impl InterfaceState {
    pub fn name(&self) -> &'static str {
        match self {
            InterfaceState::Down => "Down",
            InterfaceState::Loopback => "Loopback",
            InterfaceState::Waiting => "Waiting",
            InterfaceState::PointToPoint => "Point-to-Point",
            InterfaceState::DROther => "DROther",
            InterfaceState::DR => "DR",
            InterfaceState::Backup => "Backup",
        }
    }

    pub fn can_send_hello(&self) -> bool {
        !matches!(self, InterfaceState::Down | InterfaceState::Loopback)
    }
}

/// OSPFv2 接口
#[derive(Debug, Clone)]
pub struct OspfInterface {
    /// 接口名称
    pub name: String,

    /// 接口索引
    pub ifindex: u32,

    /// 接口状态
    pub state: InterfaceState,

    /// 接口 IP 地址
    pub ip_addr: Ipv4Addr,

    /// 接口掩码
    pub mask: Ipv4Addr,

    /// 区域 ID
    pub area_id: Ipv4Addr,

    /// 接口类型
    pub if_type: crate::protocols::ospf::types::InterfaceType,

    /// Hello 间隔
    pub hello_interval: u16,

    /// 路由器死亡间隔
    pub dead_interval: u32,

    /// 路由器优先级
    pub priority: u8,

    /// 指定路由器
    pub dr: Ipv4Addr,

    /// 备份指定路由器
    pub bdr: Ipv4Addr,

    /// 接口 Cost
    pub cost: u32,

    /// Hello 定时器
    pub hello_timer: Option<std::time::Instant>,

    /// Wait 定时器
    pub wait_timer: Option<std::time::Instant>,
}

impl OspfInterface {
    pub fn new(
        name: String,
        ifindex: u32,
        ip_addr: Ipv4Addr,
        mask: Ipv4Addr,
        area_id: Ipv4Addr,
    ) -> Self {
        Self {
            name,
            ifindex,
            state: InterfaceState::Down,
            ip_addr,
            mask,
            area_id,
            if_type: crate::protocols::ospf::types::InterfaceType::Broadcast,
            hello_interval: 10,
            dead_interval: 40,
            priority: 1,
            dr: Ipv4Addr::UNSPECIFIED,
            bdr: Ipv4Addr::UNSPECIFIED,
            cost: 1,
            hello_timer: None,
            wait_timer: None,
        }
    }

    /// 启动接口
    pub fn up(&mut self) {
        if self.state == InterfaceState::Down {
            if self.if_type.is_point_to_point() {
                self.state = InterfaceState::PointToPoint;
            } else {
                self.state = InterfaceState::Waiting;
                // 启动 Wait 定时器
                self.wait_timer = Some(
                    std::time::Instant::now()
                        .checked_add(std::time::Duration::from_secs(self.dead_interval as u64))
                        .unwrap_or(std::time::Instant::now())
                );
            }
            // 启动 Hello 定时器
            self.reset_hello_timer();
        }
    }

    /// 关闭接口
    pub fn down(&mut self) {
        self.state = InterfaceState::Down;
        self.dr = Ipv4Addr::UNSPECIFIED;
        self.bdr = Ipv4Addr::UNSPECIFIED;
        self.hello_timer = None;
        self.wait_timer = None;
    }

    /// 重置 Hello 定时器
    pub fn reset_hello_timer(&mut self) {
        self.hello_timer = Some(
            std::time::Instant::now()
                .checked_add(std::time::Duration::from_secs(self.hello_interval as u64))
                .unwrap_or(std::time::Instant::now())
        );
    }

    /// 检查 Hello 定时器是否超时
    pub fn is_hello_timer_expired(&self) -> bool {
        if let Some(timer) = self.hello_timer {
            std::time::Instant::now() > timer
        } else {
            false
        }
    }

    /// 检查 Wait 定时器是否超时
    pub fn is_wait_timer_expired(&self) -> bool {
        if let Some(timer) = self.wait_timer {
            std::time::Instant::now() > timer
        } else {
            false
        }
    }

    /// 是否是 DR
    pub fn is_dr(&self, router_id: Ipv4Addr) -> bool {
        self.dr == router_id
    }

    /// 是否是 BDR
    pub fn is_bdr(&self, router_id: Ipv4Addr) -> bool {
        self.bdr == router_id
    }

    /// 设置 DR
    pub fn set_dr(&mut self, dr: Ipv4Addr) {
        self.dr = dr;
    }

    /// 设置 BDR
    pub fn set_bdr(&mut self, bdr: Ipv4Addr) {
        self.bdr = bdr;
    }

    /// 验证 Hello 报文参数
    pub fn validate_hello_params(
        &self,
        hello_interval: u16,
        dead_interval: u32,
        mask: Ipv4Addr,
    ) -> OspfResult<()> {
        if hello_interval != self.hello_interval {
            return Err(super::error::OspfError::ConfigError {
                parameter: "hello_interval".to_string(),
                reason: format!("mismatch: expected {}, got {}", self.hello_interval, hello_interval),
            });
        }

        if dead_interval != self.dead_interval {
            return Err(super::error::OspfError::ConfigError {
                parameter: "dead_interval".to_string(),
                reason: format!("mismatch: expected {}, got {}", self.dead_interval, dead_interval),
            });
        }

        if mask != self.mask {
            return Err(super::error::OspfError::ConfigError {
                parameter: "network_mask".to_string(),
                reason: format!("mismatch: expected {}, got {}", self.mask, mask),
            });
        }

        Ok(())
    }

    /// 是否参与 DR 选举
    pub fn is_eligible_for_dr(&self) -> bool {
        self.priority > 0 && !self.if_type.is_point_to_point()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocols::ospf::types::InterfaceType;

    #[test]
    fn test_interface_state_can_send_hello() {
        assert!(!InterfaceState::Down.can_send_hello());
        assert!(!InterfaceState::Loopback.can_send_hello());
        assert!(InterfaceState::Waiting.can_send_hello());
        assert!(InterfaceState::DR.can_send_hello());
    }

    #[test]
    fn test_ospf_interface_new() {
        let iface = OspfInterface::new(
            "eth0".to_string(),
            1,
            Ipv4Addr::new(192, 168, 1, 1),
            Ipv4Addr::new(255, 255, 255, 0),
            Ipv4Addr::new(0, 0, 0, 0),
        );

        assert_eq!(iface.name, "eth0");
        assert_eq!(iface.state, InterfaceState::Down);
        assert_eq!(iface.ip_addr, Ipv4Addr::new(192, 168, 1, 1));
    }

    #[test]
    fn test_ospf_interface_up_down() {
        let mut iface = OspfInterface::new(
            "eth0".to_string(),
            1,
            Ipv4Addr::new(192, 168, 1, 1),
            Ipv4Addr::new(255, 255, 255, 0),
            Ipv4Addr::new(0, 0, 0, 0),
        );

        iface.up();
        assert_eq!(iface.state, InterfaceState::Waiting);
        assert!(iface.hello_timer.is_some());
        assert!(iface.wait_timer.is_some());

        iface.down();
        assert_eq!(iface.state, InterfaceState::Down);
        assert!(iface.hello_timer.is_none());
        assert!(iface.wait_timer.is_none());
    }

    #[test]
    fn test_ospf_interface_point_to_point() {
        let mut iface = OspfInterface::new(
            "eth0".to_string(),
            1,
            Ipv4Addr::new(192, 168, 1, 1),
            Ipv4Addr::new(255, 255, 255, 255),
            Ipv4Addr::new(0, 0, 0, 0),
        );
        iface.if_type = InterfaceType::PointToPoint;

        iface.up();
        assert_eq!(iface.state, InterfaceState::PointToPoint);
    }
}
