// src/protocols/ospf/election.rs
//
// OSPF DR/BDR 选举算法
// RFC 2328 Section 9.4: 选举指定路由器

use crate::common::Ipv4Addr;
use crate::protocols::ospf2::{
    neighbor::OspfNeighbor,
    interface::{OspfInterface, InterfaceState},
};
use crate::protocols::ospf::InterfaceType;

/// DR/BDR 选举结果
#[derive(Debug, Clone)]
pub struct ElectionResult {
    /// 新的 DR
    pub dr: Ipv4Addr,
    /// 新的 BDR
    pub bdr: Ipv4Addr,
    /// DR 是否改变
    pub dr_changed: bool,
    /// BDR 是否改变
    pub bdr_changed: bool,
    /// 是否需要发送 DD 报文
    pub need_exchange: bool,
}

/// DR/BDR 选举器
pub struct DrBdrElection;

impl DrBdrElection {
    /// 执行 DR/BDR 选举
    ///
    /// RFC 2328 Section 9.4: 选举指定路由器
    ///
    /// # 参数
    /// - `interface`: 要进行选举的接口
    /// - `local_router_id`: 本地路由器 ID
    /// - `neighbors`: 该接口上的所有邻居
    ///
    /// # 返回
    /// 返回选举结果
    pub fn elect_dr_bdr(
        interface: &mut OspfInterface,
        local_router_id: Ipv4Addr,
        neighbors: &[OspfNeighbor],
    ) -> ElectionResult {
        let old_dr = interface.dr;
        let old_bdr = interface.bdr;

        // 1. 构建候选集
        let mut candidates = Vec::new();

        // 添加自己（如果优先级 > 0 且接口不是点对点类型）
        if interface.is_eligible_for_dr() {
            candidates.push((
                local_router_id,
                interface.priority,
                interface.dr,
                interface.bdr,
            ));
        }

        // 添加已建立双向通信的邻居
        for neighbor in neighbors {
            if neighbor.state.is_two_way_established() && neighbor.priority > 0 {
                candidates.push((
                    neighbor.router_id,
                    neighbor.priority,
                    neighbor.dr,
                    neighbor.bdr,
                ));
            }
        }

        // 2. 选举 BDR（先于 DR）
        // BDR 选举规则：
        // - 排除当前 DR
        // - 选择优先级最高的
        // - 如果优先级相同，选择 Router ID 最大的
        let mut bdr = Ipv4Addr::UNSPECIFIED;
        let mut bdr_priority = 0;
        let mut bdr_id = Ipv4Addr::UNSPECIFIED;

        for (router_id, priority, current_dr, _) in &candidates {
            // 跳过当前 DR
            if *router_id == old_dr {
                continue;
            }

            // 选择优先级最高或 Router ID 最大的
            if *priority > bdr_priority || (*priority == bdr_priority && *router_id > bdr_id) {
                bdr_priority = *priority;
                bdr_id = *router_id;
                bdr = *router_id;
            }
        }

        // 如果没有找到 BDR 候选，保持当前 BDR
        if bdr == Ipv4Addr::UNSPECIFIED {
            bdr = old_bdr;
        }

        // 3. 选举 DR
        // DR 选举规则：
        // - 选择优先级最高的
        // - 如果优先级相同，选择 Router ID 最大的
        let mut dr = Ipv4Addr::UNSPECIFIED;
        let mut dr_priority = 0;
        let mut dr_id = Ipv4Addr::UNSPECIFIED;

        for (router_id, priority, _, _) in &candidates {
            if *priority > dr_priority || (*priority == dr_priority && *router_id > dr_id) {
                dr_priority = *priority;
                dr_id = *router_id;
                dr = *router_id;
            }
        }

        // 如果没有找到 DR 候选，保持当前 DR
        if dr == Ipv4Addr::UNSPECIFIED {
            dr = old_dr;
        }

        // 4. 更新接口的 DR/BDR
        interface.set_dr(dr);
        interface.set_bdr(bdr);

        // 5. 判断状态是否改变
        let dr_changed = dr != old_dr;
        let bdr_changed = bdr != old_bdr;

        // 6. 判断是否需要发送 DD 报文
        // 如果 DR/BDR 发生变化，或者路由器本身成为 DR/BDR
        let need_exchange = dr_changed || bdr_changed
            || (dr == local_router_id && interface.state != InterfaceState::DR)
            || (bdr == local_router_id && interface.state != InterfaceState::Backup);

        // 7. 更新接口状态
        Self::update_interface_state(interface, local_router_id);

        ElectionResult {
            dr,
            bdr,
            dr_changed,
            bdr_changed,
            need_exchange,
        }
    }

    /// 更新接口状态
    fn update_interface_state(interface: &mut OspfInterface, local_router_id: Ipv4Addr) {
        match interface.if_type {
            InterfaceType::Broadcast => {
                if interface.is_dr(local_router_id) {
                    interface.state = InterfaceState::DR;
                } else if interface.is_bdr(local_router_id) {
                    interface.state = InterfaceState::Backup;
                } else {
                    interface.state = InterfaceState::DROther;
                }
            }
            InterfaceType::PointToPoint => {
                interface.state = InterfaceState::PointToPoint;
            }
            _ => {
                // 其他类型保持当前状态
            }
        }
    }

    /// 判断路由器是否有资格参与 DR 选举
    pub fn is_eligible(interface: &OspfInterface, router_id: Ipv4Addr) -> bool {
        // 优先级必须大于 0
        if interface.priority == 0 {
            return false;
        }

        // 接口状态必须允许选举
        match interface.state {
            InterfaceState::DR => {
                // 当前 DR 始终有资格
                true
            }
            InterfaceState::Backup => {
                // 当前 BDR 始终有资格
                true
            }
            InterfaceState::DROther => {
                // DROther 有资格
                true
            }
            InterfaceState::Waiting => {
                // Waiting 状态有资格
                true
            }
            InterfaceState::PointToPoint => {
                // 点对点网络不进行 DR/BDR 选举
                false
            }
            _ => {
                // 其他状态无资格
                false
            }
        }
    }

    /// 判断是否需要建立邻接关系
    ///
    /// RFC 2328 Section 10.4: 是否需要建立邻接关系
    pub fn needs_adjacency(
        local_is_dr: bool,
        local_is_bdr: bool,
        neighbor_is_dr: bool,
        neighbor_is_bdr: bool,
    ) -> bool {
        // DR 与 BDR 之间必须建立邻接关系
        if (local_is_dr && neighbor_is_bdr) || (local_is_bdr && neighbor_is_dr) {
            return true;
        }

        // DR/BDR 与所有非 DR/BDR 路由器建立邻接关系
        if local_is_dr || local_is_bdr {
            return !neighbor_is_dr && !neighbor_is_bdr;
        }

        // 非 DR/BDR 路由器与 DR/BDR 建立邻接关系
        if neighbor_is_dr || neighbor_is_bdr {
            return true;
        }

        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_elect_dr_bdr_empty_candidates() {
        let mut iface = OspfInterface::new(
            "eth0".to_string(),
            1,
            Ipv4Addr::new(192, 168, 1, 1),
            Ipv4Addr::new(255, 255, 255, 0),
            Ipv4Addr::new(0, 0, 0, 0),
        );
        iface.priority = 1;
        iface.state = InterfaceState::Waiting;

        let local_router_id = Ipv4Addr::new(1, 1, 1, 1);
        let neighbors: Vec<OspfNeighbor> = Vec::new();

        let result = DrBdrElection::elect_dr_bdr(&mut iface, local_router_id, &neighbors);

        // 没有候选时，应该选举自己为 DR/BDR
        assert_eq!(result.dr, local_router_id);
        assert_eq!(result.bdr, local_router_id);
    }

    #[test]
    fn test_is_eligible() {
        let mut iface = OspfInterface::new(
            "eth0".to_string(),
            1,
            Ipv4Addr::new(192, 168, 1, 1),
            Ipv4Addr::new(255, 255, 255, 0),
            Ipv4Addr::new(0, 0, 0, 0),
        );
        iface.priority = 1;

        // DROther 状态有资格
        iface.state = InterfaceState::DROther;
        assert!(DrBdrElection::is_eligible(&iface, Ipv4Addr::new(1, 1, 1, 1)));

        // 优先级为 0 时无资格
        iface.priority = 0;
        assert!(!DrBdrElection::is_eligible(&iface, Ipv4Addr::new(1, 1, 1, 1)));

        // 点对点网络无资格
        iface.priority = 1;
        iface.if_type = InterfaceType::PointToPoint;
        iface.state = InterfaceState::PointToPoint;
        assert!(!DrBdrElection::is_eligible(&iface, Ipv4Addr::new(1, 1, 1, 1)));
    }

    #[test]
    fn test_needs_adjacency() {
        // DR 与 BDR 之间需要邻接关系
        assert!(DrBdrElection::needs_adjacency(true, false, false, true));
        assert!(DrBdrElection::needs_adjacency(false, true, true, false));

        // DR/BDR 与非 DR/BDR 路由器需要邻接关系
        assert!(DrBdrElection::needs_adjacency(true, false, false, false));
        assert!(DrBdrElection::needs_adjacency(false, true, false, false));

        // 非 DR/BDR 路由器与 DR/BDR 需要邻接关系
        assert!(DrBdrElection::needs_adjacency(false, false, true, false));
        assert!(DrBdrElection::needs_adjacency(false, false, false, true));

        // 两个非 DR/BDR 路由器之间不需要邻接关系
        assert!(!DrBdrElection::needs_adjacency(false, false, false, false));
    }
}
