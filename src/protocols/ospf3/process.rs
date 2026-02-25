// src/protocols/ospf3/process.rs
//
// OSPFv3 报文处理逻辑

use crate::common::{Packet, Ipv6Addr};
use crate::protocols::ospf::{InterfaceState, NeighborState};
use super::error::{Ospfv3Error, Ospfv3Result};
use super::packet::*;
use super::lsa::*;
use super::interface::Ospfv3Interface;
use super::neighbor::Ospfv3Neighbor;
use super::lsdb::LinkStateDatabasev3;

/// OSPFv3 处理结果
#[derive(Debug, Clone)]
pub enum Ospfv3ProcessResult {
    /// 无响应
    NoReply,
    /// 需要发送响应报文
    Reply(Vec<u8>),
    /// 需要洪泛 LSA
    FloodLsa { lsa: Lsa, exclude_interface: Option<u32> },
    /// 触发 SPF 计算
    ScheduleSpfCalculation,
    /// 数据库同步完成
    DatabaseSynced,
}

/// OSPFv3 处理器
pub struct Ospfv3Processor {
    /// 路由器 ID (32-bit)
    pub router_id: u32,
    /// 接口列表（按接口索引）
    pub interfaces: Vec<Ospfv3Interface>,
    /// 邻居列表（按 Router ID）
    pub neighbors: Vec<Ospfv3Neighbor>,
    /// 链路状态数据库
    pub lsdb: LinkStateDatabasev3,
}

impl Ospfv3Processor {
    /// 创建新的 OSPFv3 处理器
    pub fn new(router_id: u32) -> Self {
        Self {
            router_id,
            interfaces: Vec::new(),
            neighbors: Vec::new(),
            lsdb: LinkStateDatabasev3::new(),
        }
    }

    /// 添加接口
    pub fn add_interface(&mut self, interface: Ospfv3Interface) {
        self.interfaces.push(interface);
    }

    /// 处理接收到的 OSPFv3 报文
    pub fn process_packet(&mut self, packet: &mut Packet, ifindex: u32, source_ip: Ipv6Addr) -> Ospfv3Result<Ospfv3ProcessResult> {
        // 解析 OSPFv3 头部
        let data = packet.as_remaining_slice();
        let header = Ospfv3Header::from_bytes(data)?;

        // 验证基本参数
        self.validate_header(&header)?;

        // 根据报文类型分发处理
        match header.packet_type {
            1 => self.process_hello(header, data, ifindex, source_ip),
            2 => self.process_dd(header, data, ifindex),
            3 => self.process_lsr(header, data, ifindex),
            4 => self.process_lsu(header, data, ifindex),
            5 => self.process_lsack(header, data, ifindex),
            _ => Err(Ospfv3Error::invalid_packet_type(header.packet_type)),
        }
    }

    /// 验证 OSPFv3 头部
    fn validate_header(&self, header: &Ospfv3Header) -> Ospfv3Result<()> {
        if header.version != 3 {
            return Err(Ospfv3Error::VersionMismatch {
                expected: 3,
                actual: header.version,
            });
        }
        Ok(())
    }

    /// 处理 Hello 报文
    fn process_hello(&mut self, header: Ospfv3Header, data: &[u8], ifindex: u32, source_ip: Ipv6Addr) -> Ospfv3Result<Ospfv3ProcessResult> {
        let hello_data = &data.get(Ospfv3Header::LENGTH..).unwrap_or(&[]);
        let hello = Ospfv3Hello::from_bytes(hello_data)?;

        // 获取接口信息用于验证和后续处理
        let interface_idx = self.interfaces.iter()
            .position(|iface| iface.ifindex == ifindex)
            .ok_or_else(|| Ospfv3Error::Other {
                reason: format!("Interface {} not found", ifindex)
            })?;

        // 验证 Hello 参数
        self.interfaces[interface_idx].validate_hello_params(
            hello.hello_interval,
            hello.router_dead_interval,
        )?;

        let dead_interval = self.interfaces[interface_idx].dead_interval;
        let current_dr = self.interfaces[interface_idx].dr;
        let current_bdr = self.interfaces[interface_idx].bdr;
        let local_is_dr = self.interfaces[interface_idx].is_dr(self.router_id);
        let local_is_bdr = self.interfaces[interface_idx].is_bdr(self.router_id);

        // 查找或创建邻居
        let neighbor_idx = if let Some(idx) = self.neighbors.iter().position(|n| n.router_id == header.router_id) {
            idx
        } else {
            let neighbor = Ospfv3Neighbor::new(header.router_id, source_ip, dead_interval);
            self.neighbors.push(neighbor);
            self.neighbors.len() - 1
        };

        // 更新邻居状态
        {
            let neighbor = &mut self.neighbors[neighbor_idx];
            neighbor.priority = hello.router_priority;
            neighbor.dr = hello.designated_router;
            neighbor.bdr = hello.backup_designated_router;
            neighbor.reset_inactivity_timer(dead_interval);

            // 状态转换
            match neighbor.state {
                NeighborState::Down => {
                    neighbor.state = NeighborState::Init;
                }
                NeighborState::Init => {
                    if hello.neighbors.contains(&self.router_id) {
                        neighbor.state = NeighborState::TwoWay;
                        let neighbor_is_dr = current_dr == header.router_id;
                        let neighbor_is_bdr = current_bdr == header.router_id;

                        if neighbor.needs_adjacency(local_is_dr, local_is_bdr, neighbor_is_dr, neighbor_is_bdr) {
                            neighbor.state = NeighborState::ExStart;
                            neighbor.init_dd_sequence();
                        }
                    }
                }
                _ => {}
            }
        }

        // 执行 DR/BDR 选举
        self.elect_dr_bdr_internal(interface_idx);

        Ok(Ospfv3ProcessResult::NoReply)
    }

    /// 处理 Database Description 报文
    fn process_dd(&mut self, _header: Ospfv3Header, _data: &[u8], _ifindex: u32) -> Ospfv3Result<Ospfv3ProcessResult> {
        Ok(Ospfv3ProcessResult::NoReply)
    }

    /// 处理 Link State Request 报文
    fn process_lsr(&mut self, _header: Ospfv3Header, _data: &[u8], _ifindex: u32) -> Ospfv3Result<Ospfv3ProcessResult> {
        Ok(Ospfv3ProcessResult::NoReply)
    }

    /// 处理 Link State Update 报文
    fn process_lsu(&mut self, header: Ospfv3Header, data: &[u8], ifindex: u32) -> Ospfv3Result<Ospfv3ProcessResult> {
        let _interface = self.get_interface(ifindex)?;

        // 跳过 OSPF 头部和 LSA 数量字段
        let _lsu_data = &data.get(Ospfv3Header::LENGTH + 4..).unwrap_or(&[]);

        // 简化实现：只发送确认
        self.send_lsack_for_lsu(header.router_id);

        Ok(Ospfv3ProcessResult::ScheduleSpfCalculation)
    }

    /// 处理 Link State Acknowledgment 报文
    fn process_lsack(&mut self, _header: Ospfv3Header, _data: &[u8], _ifindex: u32) -> Ospfv3Result<Ospfv3ProcessResult> {
        Ok(Ospfv3ProcessResult::NoReply)
    }

    /// 发送 LSAck
    fn send_lsack_for_lsu(&mut self, _destination: u32) {
        // 发送直接确认
    }

    /// 获取接口
    fn get_interface(&self, ifindex: u32) -> Ospfv3Result<&Ospfv3Interface> {
        self.interfaces.iter()
            .find(|iface| iface.ifindex == ifindex)
            .ok_or_else(|| Ospfv3Error::Other {
                reason: format!("Interface {} not found", ifindex),
            })
    }

    /// 执行 DR/BDR 选举（内部版本，使用索引）
    fn elect_dr_bdr_internal(&mut self, interface_idx: usize) {
        let mut candidates: Vec<(u32, u8)> = vec![];

        // 添加自己
        if self.interfaces[interface_idx].is_eligible_for_dr() {
            candidates.push((self.router_id, self.interfaces[interface_idx].priority));
        }

        // 添加邻居
        for neighbor in &self.neighbors {
            if neighbor.state.is_two_way_established() && neighbor.priority > 0 {
                candidates.push((neighbor.router_id, neighbor.priority));
            }
        }

        // 简化的选举逻辑：选择优先级最高的作为 DR
        let mut dr: u32 = 0;
        let mut bdr: u32 = 0;

        for (router_id, priority) in &candidates {
            if dr == 0 || *priority > self.get_priority(&self.interfaces, &self.neighbors, dr) {
                bdr = dr;
                dr = *router_id;
            }
        }

        // 更新接口的 DR/BDR
        self.interfaces[interface_idx].set_dr(dr);
        self.interfaces[interface_idx].set_bdr(bdr);

        // 更新接口状态
        if self.interfaces[interface_idx].is_dr(self.router_id) {
            self.interfaces[interface_idx].state = InterfaceState::DR;
        } else if self.interfaces[interface_idx].is_bdr(self.router_id) {
            self.interfaces[interface_idx].state = InterfaceState::Backup;
        } else {
            self.interfaces[interface_idx].state = InterfaceState::DROther;
        }
    }

    /// 获取路由器的优先级
    fn get_priority(&self, _interfaces: &[Ospfv3Interface], _neighbors: &[Ospfv3Neighbor], router_id: u32) -> u8 {
        if router_id == self.router_id {
            self.interfaces.iter()
                .find(|iface| iface.ifindex > 0)
                .map(|iface| iface.priority)
                .unwrap_or(1)
        } else {
            self.neighbors.iter()
                .find(|n| n.router_id == router_id)
                .map(|n| n.priority)
                .unwrap_or(1)
        }
    }
}

/// 处理 OSPFv3 报文（入口函数）
pub fn process_ospfv3_packet(
    packet: &mut Packet,
    ifindex: u32,
    router_id: u32,
    source_ip: Ipv6Addr,
) -> Ospfv3Result<Ospfv3ProcessResult> {
    let mut processor = Ospfv3Processor::new(router_id);
    processor.process_packet(packet, ifindex, source_ip)
}
