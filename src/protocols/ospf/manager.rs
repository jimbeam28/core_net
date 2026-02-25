// src/protocols/ospf/manager.rs
//
// OSPF 管理器 - 维护 OSPF 协议的持久状态
// 负责管理接口、邻居、LSDB 和定时器

use crate::common::Ipv4Addr;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use crate::protocols::ospf2::interface::OspfInterface;
use crate::protocols::ospf2::neighbor::OspfNeighbor;
use crate::protocols::ospf2::lsdb::LinkStateDatabase;
use crate::protocols::ospf3::interface::Ospfv3Interface;
use crate::protocols::ospf3::neighbor::Ospfv3Neighbor;
use crate::protocols::ospf3::lsdb::LinkStateDatabasev3;
use crate::common::Ipv6Addr;

/// OSPF 定时器类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OspfTimerType {
    /// Hello 定时器
    Hello,
    /// Wait 定时器（DR/BDR 选举）
    Wait,
    /// Inactivity 定时器（邻居超时）
    Inactivity,
    /// 重传定时器
    Rxmt,
    /// LSA 刷新定时器
    LsaRefresh,
}

/// OSPF 定时器
#[derive(Debug, Clone)]
pub struct OspfTimer {
    /// 定时器 ID
    pub id: u64,
    /// 定时器类型
    pub timer_type: OspfTimerType,
    /// 接口索引（对于接口级别定时器）
    pub ifindex: Option<u32>,
    /// 邻居 Router ID（对于邻居级别定时器）
    pub neighbor_id: Option<u32>,
    /// 到期时间
    pub expiry: Instant,
    /// 是否是 OSPFv3
    pub is_v3: bool,
}

/// 接口定时器集合
#[derive(Debug, Clone, Default)]
pub struct InterfaceTimers {
    /// Hello 定时器 ID
    pub hello_id: Option<u64>,
    /// Wait 定时器 ID
    pub wait_id: Option<u64>,
}

/// 邻居定时器集合
#[derive(Debug, Clone, Default)]
pub struct NeighborTimers {
    /// Inactivity 定时器 ID
    pub inactivity_id: Option<u64>,
    /// 重传定时器 ID
    pub rxmt_id: Option<u64>,
}

/// 定时器事件（用于回调）
#[derive(Debug, Clone)]
pub struct OspfTimerEvent {
    /// 定时器类型
    pub timer_type: OspfTimerType,
    /// 接口索引
    pub ifindex: Option<u32>,
    /// 邻居 Router ID（对于邻居级别事件）
    pub neighbor_id: Option<u32>,
    /// 是否是 OSPFv3
    pub is_v3: bool,
}

/// OSPF 定时器管理器
#[derive(Debug)]
pub struct OspfTimerManager {
    /// 定时器 ID 计数器
    next_id: u64,
    /// 活跃的定时器
    timers: HashMap<u64, OspfTimer>,
    /// 接口定时器索引
    interface_timers: HashMap<(u32, bool), InterfaceTimers>,  // (ifindex, is_v3) -> timers
    /// 邻居定时器索引
    neighbor_timers: HashMap<(u32, u32, bool), NeighborTimers>,  // (ifindex, neighbor_id, is_v3) -> timers
}

impl Default for OspfTimerManager {
    fn default() -> Self {
        Self::new()
    }
}

impl OspfTimerManager {
    /// 创建新的定时器管理器
    pub fn new() -> Self {
        Self {
            next_id: 1,
            timers: HashMap::new(),
            interface_timers: HashMap::new(),
            neighbor_timers: HashMap::new(),
        }
    }

    /// 添加 Hello 定时器
    pub fn add_hello_timer(&mut self, ifindex: u32, delay_sec: u16, is_v3: bool) -> u64 {
        let id = self.next_id;
        self.next_id = self.next_id.wrapping_add(1);

        let timer = OspfTimer {
            id,
            timer_type: OspfTimerType::Hello,
            ifindex: Some(ifindex),
            neighbor_id: None,
            expiry: Instant::now() + std::time::Duration::from_secs(delay_sec as u64),
            is_v3,
        };

        self.timers.insert(id, timer);

        let key = (ifindex, is_v3);
        let entry = self.interface_timers.entry(key).or_default();
        entry.hello_id = Some(id);

        id
    }

    /// 添加 Wait 定时器
    pub fn add_wait_timer(&mut self, ifindex: u32, delay_sec: u32, is_v3: bool) -> u64 {
        let id = self.next_id;
        self.next_id = self.next_id.wrapping_add(1);

        let timer = OspfTimer {
            id,
            timer_type: OspfTimerType::Wait,
            ifindex: Some(ifindex),
            neighbor_id: None,
            expiry: Instant::now() + std::time::Duration::from_secs(delay_sec as u64),
            is_v3,
        };

        self.timers.insert(id, timer);

        let key = (ifindex, is_v3);
        let entry = self.interface_timers.entry(key).or_default();
        entry.wait_id = Some(id);

        id
    }

    /// 添加 Inactivity 定时器
    pub fn add_inactivity_timer(
        &mut self,
        ifindex: u32,
        neighbor_id: u32,
        delay_sec: u32,
        is_v3: bool,
    ) -> u64 {
        let id = self.next_id;
        self.next_id = self.next_id.wrapping_add(1);

        let timer = OspfTimer {
            id,
            timer_type: OspfTimerType::Inactivity,
            ifindex: Some(ifindex),
            neighbor_id: Some(neighbor_id),
            expiry: Instant::now() + std::time::Duration::from_secs(delay_sec as u64),
            is_v3,
        };

        self.timers.insert(id, timer);

        let key = (ifindex, neighbor_id, is_v3);
        let entry = self.neighbor_timers.entry(key).or_default();
        entry.inactivity_id = Some(id);

        id
    }

    /// 添加重传定时器
    pub fn add_rxmt_timer(
        &mut self,
        ifindex: u32,
        neighbor_id: u32,
        delay_sec: u32,
        is_v3: bool,
    ) -> u64 {
        let id = self.next_id;
        self.next_id = self.next_id.wrapping_add(1);

        let timer = OspfTimer {
            id,
            timer_type: OspfTimerType::Rxmt,
            ifindex: Some(ifindex),
            neighbor_id: Some(neighbor_id),
            expiry: Instant::now() + std::time::Duration::from_secs(delay_sec as u64),
            is_v3,
        };

        self.timers.insert(id, timer);

        let key = (ifindex, neighbor_id, is_v3);
        let entry = self.neighbor_timers.entry(key).or_default();
        entry.rxmt_id = Some(id);

        id
    }

    /// 重置 Inactivity 定时器
    pub fn reset_inactivity_timer(&mut self, ifindex: u32, neighbor_id: u32, dead_interval: u32, is_v3: bool) {
        // 移除旧的定时器
        let key = (ifindex, neighbor_id, is_v3);
        if let Some(entry) = self.neighbor_timers.get_mut(&key) {
            if let Some(old_id) = entry.inactivity_id.take() {
                self.timers.remove(&old_id);
            }
        }

        // 添加新的定时器
        self.add_inactivity_timer(ifindex, neighbor_id, dead_interval, is_v3);
    }

    /// 获取已到期的定时器事件
    pub fn get_expired_timers(&mut self) -> Vec<OspfTimerEvent> {
        let now = Instant::now();
        let mut expired = Vec::new();
        let mut expired_ids = Vec::new();

        for (id, timer) in &self.timers {
            if now > timer.expiry {
                expired.push(OspfTimerEvent {
                    timer_type: timer.timer_type,
                    ifindex: timer.ifindex,
                    neighbor_id: timer.neighbor_id,
                    is_v3: timer.is_v3,
                });
                expired_ids.push(*id);
            }
        }

        // 移除已到期的定时器
        for id in expired_ids {
            self.remove_timer(id);
        }

        expired
    }

    /// 移除定时器
    pub fn remove_timer(&mut self, id: u64) {
        if let Some(timer) = self.timers.remove(&id) {
            // 从索引中移除
            if let Some(ifindex) = timer.ifindex {
                if let Some(neighbor_id) = timer.neighbor_id {
                    // 邻居定时器
                    let key = (ifindex, neighbor_id, timer.is_v3);
                    if let Some(entry) = self.neighbor_timers.get_mut(&key) {
                        match timer.timer_type {
                            OspfTimerType::Inactivity => {
                                if entry.inactivity_id == Some(id) {
                                    entry.inactivity_id = None;
                                }
                            }
                            OspfTimerType::Rxmt => {
                                if entry.rxmt_id == Some(id) {
                                    entry.rxmt_id = None;
                                }
                            }
                            _ => {}
                        }
                    }
                } else {
                    // 接口定时器
                    let key = (ifindex, timer.is_v3);
                    if let Some(entry) = self.interface_timers.get_mut(&key) {
                        match timer.timer_type {
                            OspfTimerType::Hello => {
                                if entry.hello_id == Some(id) {
                                    entry.hello_id = None;
                                }
                            }
                            OspfTimerType::Wait => {
                                if entry.wait_id == Some(id) {
                                    entry.wait_id = None;
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
        }
    }

    /// 取消接口的所有定时器
    pub fn cancel_interface_timers(&mut self, ifindex: u32, is_v3: bool) {
        let key = (ifindex, is_v3);
        if let Some(entry) = self.interface_timers.remove(&key) {
            if let Some(id) = entry.hello_id {
                self.timers.remove(&id);
            }
            if let Some(id) = entry.wait_id {
                self.timers.remove(&id);
            }
        }
    }

    /// 取消邻居的所有定时器
    pub fn cancel_neighbor_timers(&mut self, ifindex: u32, neighbor_id: u32, is_v3: bool) {
        let key = (ifindex, neighbor_id, is_v3);
        if let Some(entry) = self.neighbor_timers.remove(&key) {
            if let Some(id) = entry.inactivity_id {
                self.timers.remove(&id);
            }
            if let Some(id) = entry.rxmt_id {
                self.timers.remove(&id);
            }
        }
    }
}

/// OSPF 管理器 - 维护 OSPFv2 和 OSPFv3 的持久状态
pub struct OspfManager {
    /// 路由器 ID
    pub router_id: u32,

    /// OSPFv2 接口列表
    pub v2_interfaces: Vec<OspfInterface>,

    /// OSPFv3 接口列表
    pub v3_interfaces: Vec<Ospfv3Interface>,

    /// OSPFv2 邻居映射 (router_id -> neighbor)
    pub v2_neighbors: HashMap<Ipv4Addr, Arc<Mutex<OspfNeighbor>>>,

    /// OSPFv3 邻居映射 (ifindex, router_id -> neighbor)
    pub v3_neighbors: HashMap<(u32, u32), Arc<Mutex<Ospfv3Neighbor>>>,  // (ifindex, router_id)

    /// OSPFv2 链路状态数据库
    pub v2_lsdb: LinkStateDatabase,

    /// OSPFv3 链路状态数据库
    pub v3_lsdb: LinkStateDatabasev3,

    /// 定时器管理器
    pub timers: OspfTimerManager,
}

impl Default for OspfManager {
    fn default() -> Self {
        Self::new(0)
    }
}

impl OspfManager {
    /// 创建新的 OSPF 管理器
    pub fn new(router_id: u32) -> Self {
        Self {
            router_id,
            v2_interfaces: Vec::new(),
            v3_interfaces: Vec::new(),
            v2_neighbors: HashMap::new(),
            v3_neighbors: HashMap::new(),
            v2_lsdb: LinkStateDatabase::new(),
            v3_lsdb: LinkStateDatabasev3::new(),
            timers: OspfTimerManager::new(),
        }
    }

    /// 设置路由器 ID
    pub fn set_router_id(&mut self, router_id: u32) {
        self.router_id = router_id;
    }

    // ========== OSPFv2 接口管理 ==========

    /// 添加 OSPFv2 接口
    pub fn add_v2_interface(&mut self, interface: OspfInterface) {
        self.v2_interfaces.push(interface);
    }

    /// 获取 OSPFv2 接口
    pub fn get_v2_interface(&self, ifindex: u32) -> Option<&OspfInterface> {
        self.v2_interfaces.iter().find(|iface| iface.ifindex == ifindex)
    }

    /// 获取可变 OSPFv2 接口
    pub fn get_v2_interface_mut(&mut self, ifindex: u32) -> Option<&mut OspfInterface> {
        self.v2_interfaces.iter_mut().find(|iface| iface.ifindex == ifindex)
    }

    /// 移除 OSPFv2 接口
    pub fn remove_v2_interface(&mut self, ifindex: u32) -> bool {
        if let Some(pos) = self.v2_interfaces.iter().position(|iface| iface.ifindex == ifindex) {
            self.v2_interfaces.remove(pos);
            // 取消接口定时器
            self.timers.cancel_interface_timers(ifindex, false);
            true
        } else {
            false
        }
    }

    // ========== OSPFv3 接口管理 ==========

    /// 添加 OSPFv3 接口
    pub fn add_v3_interface(&mut self, interface: Ospfv3Interface) {
        self.v3_interfaces.push(interface);
    }

    /// 获取 OSPFv3 接口
    pub fn get_v3_interface(&self, ifindex: u32) -> Option<&Ospfv3Interface> {
        self.v3_interfaces.iter().find(|iface| iface.ifindex == ifindex)
    }

    /// 获取可变 OSPFv3 接口
    pub fn get_v3_interface_mut(&mut self, ifindex: u32) -> Option<&mut Ospfv3Interface> {
        self.v3_interfaces.iter_mut().find(|iface| iface.ifindex == ifindex)
    }

    /// 移除 OSPFv3 接口
    pub fn remove_v3_interface(&mut self, ifindex: u32) -> bool {
        if let Some(pos) = self.v3_interfaces.iter().position(|iface| iface.ifindex == ifindex) {
            self.v3_interfaces.remove(pos);
            // 取消接口定时器
            self.timers.cancel_interface_timers(ifindex, true);
            true
        } else {
            false
        }
    }

    // ========== OSPFv2 邻居管理 ==========

    /// 获取或创建 OSPFv2 邻居
    pub fn get_or_create_v2_neighbor(
        &mut self,
        router_id: Ipv4Addr,
        ip_addr: Ipv4Addr,
        dead_interval: u32,
    ) -> Arc<Mutex<OspfNeighbor>> {
        if let Some(neighbor) = self.v2_neighbors.get(&router_id) {
            // 重置 Inactivity 定时器
            neighbor.lock().unwrap().reset_inactivity_timer(dead_interval);
            neighbor.clone()
        } else {
            let neighbor = Arc::new(Mutex::new(OspfNeighbor::new(router_id, ip_addr, dead_interval)));
            self.v2_neighbors.insert(router_id, neighbor.clone());
            neighbor
        }
    }

    /// 获取 OSPFv2 邻居
    pub fn get_v2_neighbor(&self, router_id: Ipv4Addr) -> Option<Arc<Mutex<OspfNeighbor>>> {
        self.v2_neighbors.get(&router_id).cloned()
    }

    /// 移除 OSPFv2 邻居
    pub fn remove_v2_neighbor(&mut self, router_id: Ipv4Addr) -> bool {
        if let Some(neighbor) = self.v2_neighbors.remove(&router_id) {
            // 取消邻居定时器（需要遍历接口查找）
            // 简化实现：直接返回
            drop(neighbor);
            true
        } else {
            false
        }
    }

    /// 获取指定接口的所有 OSPFv2 邻居
    pub fn get_v2_neighbors_by_interface(&self, _ifindex: u32) -> Vec<Arc<Mutex<OspfNeighbor>>> {
        // 当前实现没有将邻居与接口关联
        // TODO: 在邻居数据结构中添加 ifindex 字段
        self.v2_neighbors.values().cloned().collect()
    }

    // ========== OSPFv3 邻居管理 ==========

    /// 获取或创建 OSPFv3 邻居
    pub fn get_or_create_v3_neighbor(
        &mut self,
        ifindex: u32,
        router_id: u32,
        link_local_addr: Ipv6Addr,
        dead_interval: u32,
    ) -> Arc<Mutex<Ospfv3Neighbor>> {
        let key = (ifindex, router_id);
        if let Some(neighbor) = self.v3_neighbors.get(&key) {
            // 重置 Inactivity 定时器
            neighbor.lock().unwrap().reset_inactivity_timer(dead_interval);
            neighbor.clone()
        } else {
            let neighbor = Arc::new(Mutex::new(Ospfv3Neighbor::new(router_id, link_local_addr, dead_interval)));
            self.v3_neighbors.insert(key, neighbor.clone());
            neighbor
        }
    }

    /// 获取 OSPFv3 邻居
    pub fn get_v3_neighbor(&self, ifindex: u32, router_id: u32) -> Option<Arc<Mutex<Ospfv3Neighbor>>> {
        self.v3_neighbors.get(&(ifindex, router_id)).cloned()
    }

    /// 移除 OSPFv3 邻居
    pub fn remove_v3_neighbor(&mut self, ifindex: u32, router_id: u32) -> bool {
        let key = (ifindex, router_id);
        if let Some(neighbor) = self.v3_neighbors.remove(&key) {
            // 取消邻居定时器
            self.timers.cancel_neighbor_timers(ifindex, router_id, true);
            drop(neighbor);
            true
        } else {
            false
        }
    }

    /// 获取指定接口的所有 OSPFv3 邻居
    pub fn get_v3_neighbors_by_interface(&self, ifindex: u32) -> Vec<Arc<Mutex<Ospfv3Neighbor>>> {
        self.v3_neighbors
            .iter()
            .filter(|((idx, _), _)| *idx == ifindex)
            .map(|(_, neighbor)| neighbor.clone())
            .collect()
    }

    // ========== LSDB 管理 ==========

    /// 清空 OSPFv2 LSDB
    pub fn clear_v2_lsdb(&mut self) {
        self.v2_lsdb.clear();
    }

    /// 清空 OSPFv3 LSDB
    pub fn clear_v3_lsdb(&mut self) {
        self.v3_lsdb.clear();
    }

    // ========== 定时器处理 ==========

    /// 获取所有已到期的定时器事件
    pub fn get_expired_timers(&mut self) -> Vec<OspfTimerEvent> {
        self.timers.get_expired_timers()
    }

    /// 重置邻居 Inactivity 定时器（OSPFv2）
    pub fn reset_v2_inactivity_timer(&mut self, ifindex: u32, router_id: Ipv4Addr, dead_interval: u32) {
        let router_id_u32 = ((router_id.bytes[0] as u32) << 24) |
                            ((router_id.bytes[1] as u32) << 16) |
                            ((router_id.bytes[2] as u32) << 8) |
                            (router_id.bytes[3] as u32);
        self.timers.reset_inactivity_timer(ifindex, router_id_u32, dead_interval, false);
    }

    /// 重置邻居 Inactivity 定时器（OSPFv3）
    pub fn reset_v3_inactivity_timer(&mut self, ifindex: u32, router_id: u32, dead_interval: u32) {
        self.timers.reset_inactivity_timer(ifindex, router_id, dead_interval, true);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ospf_manager_new() {
        let manager = OspfManager::new(0x01010101);
        assert_eq!(manager.router_id, 0x01010101);
        assert!(manager.v2_interfaces.is_empty());
        assert!(manager.v2_neighbors.is_empty());
    }

    #[test]
    fn test_ospf_manager_set_router_id() {
        let mut manager = OspfManager::new(0);
        manager.set_router_id(0x02020202);
        assert_eq!(manager.router_id, 0x02020202);
    }

    #[test]
    fn test_ospf_timer_manager_hello_timer() {
        let mut tm = OspfTimerManager::new();

        let id1 = tm.add_hello_timer(1, 10, false);
        assert!(id1 > 0);

        let id2 = tm.add_hello_timer(2, 10, true);
        assert!(id2 > id1);

        // 检查接口定时器索引
        let key = (1, false);
        assert!(tm.interface_timers.contains_key(&key));
        assert_eq!(tm.interface_timers[&key].hello_id, Some(id1));
    }

    #[test]
    fn test_ospf_timer_manager_inactivity_timer() {
        let mut tm = OspfTimerManager::new();

        let id = tm.add_inactivity_timer(1, 0x01010101, 40, false);
        assert!(id > 0);

        // 检查邻居定时器索引
        let key = (1, 0x01010101, false);
        assert!(tm.neighbor_timers.contains_key(&key));
        assert_eq!(tm.neighbor_timers[&key].inactivity_id, Some(id));

        // 重置定时器
        tm.reset_inactivity_timer(1, 0x01010101, 40, false);
        let new_id = tm.neighbor_timers[&key].inactivity_id.unwrap();
        assert!(new_id != id);
    }

    #[test]
    fn test_ospf_manager_v2_interface() {
        let mut manager = OspfManager::new(0x01010101);

        let iface = OspfInterface::new(
            "eth0".to_string(),
            1,
            Ipv4Addr::new(192, 168, 1, 1),
            Ipv4Addr::new(255, 255, 255, 0),
            Ipv4Addr::new(0, 0, 0, 0),
        );

        manager.add_v2_interface(iface);
        assert_eq!(manager.v2_interfaces.len(), 1);
        assert!(manager.get_v2_interface(1).is_some());
        assert!(manager.get_v2_interface(2).is_none());
    }

    #[test]
    fn test_ospf_manager_v2_neighbor() {
        let mut manager = OspfManager::new(0x01010101);

        let router_id = Ipv4Addr::new(1, 1, 1, 2);
        let ip_addr = Ipv4Addr::new(192, 168, 1, 2);

        let neighbor1 = manager.get_or_create_v2_neighbor(router_id, ip_addr, 40);
        assert_eq!(Arc::strong_count(&neighbor1), 2); // manager + neighbor1

        let neighbor2 = manager.get_or_create_v2_neighbor(router_id, ip_addr, 40);
        assert!(Arc::ptr_eq(&neighbor1, &neighbor2));

        assert_eq!(manager.v2_neighbors.len(), 1);
    }

    #[test]
    fn test_ospf_timer_cancel_interface_timers() {
        let mut tm = OspfTimerManager::new();

        let hello_id = tm.add_hello_timer(1, 10, false);
        let wait_id = tm.add_wait_timer(1, 40, false);

        tm.cancel_interface_timers(1, false);

        // 验证定时器已被移除
        assert!(!tm.timers.contains_key(&hello_id));
        assert!(!tm.timers.contains_key(&wait_id));
        assert!(!tm.interface_timers.contains_key(&(1, false)));
    }
}
