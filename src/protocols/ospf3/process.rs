// src/protocols/ospf3/process.rs
//
// OSPFv3 报文处理逻辑

use crate::common::{Packet, Ipv6Addr};
use crate::context::SystemContext;
use crate::protocols::ospf::NeighborState;
use super::error::{Ospfv3Error, Ospfv3Result};
use super::packet::*;
use super::lsa::*;
use super::interface::Ospfv3Interface;

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
pub struct Ospfv3Processor<'a> {
    /// 路由器 ID (32-bit)
    pub router_id: u32,
    /// 系统上下文引用
    context: &'a SystemContext,
}

impl<'a> Ospfv3Processor<'a> {
    /// 创建新的 OSPFv3 处理器（使用 SystemContext）
    pub fn with_context(router_id: u32, context: &'a SystemContext) -> Self {
        Self {
            router_id,
            context,
        }
    }

    /// 创建新的 OSPFv3 处理器（向后兼容，但不推荐使用）
    #[deprecated(note = "请使用 with_context 方法")]
    pub fn new(_router_id: u32) -> Self {
        panic!("Ospfv3Processor::new() 已废弃，请使用 with_context() 方法并传入 SystemContext");
    }

    /// 添加接口（已废弃，接口现在由 OspfManager 管理）
    #[deprecated(note = "接口现在由 OspfManager 管理")]
    pub fn add_interface(&mut self, _interface: Ospfv3Interface) {
        // 不再需要，接口通过 OspfManager 管理
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

        // 从 OspfManager 获取接口信息
        let mut ospf_mgr = self.context.ospf_manager.lock()
            .map_err(|e| Ospfv3Error::Other { reason: format!("锁定 OSPF 管理器失败: {}", e) })?;

        let interface = ospf_mgr.get_v3_interface(ifindex)
            .ok_or_else(|| Ospfv3Error::Other {
                reason: format!("Interface {} not found in OSPF manager", ifindex),
            })?;

        // 验证 Hello 参数
        interface.validate_hello_params(
            hello.hello_interval,
            hello.router_dead_interval,
        )?;

        let dead_interval = interface.dead_interval;
        let current_dr = interface.dr;
        let current_bdr = interface.bdr;
        let local_is_dr = interface.is_dr(self.router_id);
        let local_is_bdr = interface.is_bdr(self.router_id);

        // 获取或创建邻居（使用 OspfManager）
        let neighbor = ospf_mgr.get_or_create_v3_neighbor(ifindex, header.router_id, source_ip, dead_interval);
        let mut neighbor = neighbor.lock()
            .map_err(|e| Ospfv3Error::Other { reason: format!("锁定邻居失败: {}", e) })?;

        // 更新邻居状态
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

        // 执行 DR/BDR 选举（简化实现）
        // TODO: 实现完整的 DR/BDR 选举算法

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

    /// 获取接口（从 OspfManager）
    fn get_interface(&self, ifindex: u32) -> Ospfv3Result<Ospfv3Interface> {
        let ospf_mgr = self.context.ospf_manager.lock()
            .map_err(|e| Ospfv3Error::Other { reason: format!("锁定 OSPF 管理器失败: {}", e) })?;

        ospf_mgr.get_v3_interface(ifindex)
            .ok_or_else(|| Ospfv3Error::Other {
                reason: format!("Interface {} not found", ifindex),
            })
            .cloned()
    }
}

// 移除了以下已废弃的方法：
// - elect_dr_bdr_internal (现在通过 OspfManager 处理)
// - get_priority (已整合到 OspfManager 的选举逻辑)

/// 处理 OSPFv3 报文（入口函数）
pub fn process_ospfv3_packet(
    packet: &mut Packet,
    ifindex: u32,
    source_ip: Ipv6Addr,
    context: &SystemContext,
) -> Ospfv3Result<Ospfv3ProcessResult> {
    // 从 IPv6 地址获取路由器 ID（简化实现）
    let ipv6_bytes = source_ip.as_bytes();
    let router_id = u32::from_be_bytes([
        ipv6_bytes[12], ipv6_bytes[13], ipv6_bytes[14], ipv6_bytes[15]
    ]);

    // 创建使用 context 的处理器
    let mut processor = Ospfv3Processor::with_context(router_id, context);
    processor.process_packet(packet, ifindex, source_ip)
}
