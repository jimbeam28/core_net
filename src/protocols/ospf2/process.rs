// src/protocols/ospf2/process.rs
//
// OSPFv2 报文处理逻辑

use crate::common::{Packet, Ipv4Addr};
use crate::context::SystemContext;
use crate::protocols::ospf::LsaFlooder;
use super::error::{OspfError, OspfResult};
use super::packet::*;
use super::lsa::*;
use super::interface::OspfInterface;
use super::neighbor::NeighborState;

/// OSPF 处理结果
#[derive(Debug, Clone)]
pub enum OspfProcessResult {
    /// 无响应
    NoReply,

    /// 需要发送响应报文
    Reply(Vec<u8>),

    /// 需要洪泛 LSA
    FloodLsa {lsa: Lsa, exclude_interface: Option<u32>},

    /// 触发 SPF 计算
    ScheduleSpfCalculation,

    /// 数据库同步完成
    DatabaseSynced,
}

/// OSPFv2 处理器
pub struct OspfProcessor<'a> {
    /// 路由器 ID
    pub router_id: Ipv4Addr,

    /// 系统上下文引用
    context: &'a SystemContext,
}

impl<'a> OspfProcessor<'a> {
    /// 创建新的 OSPF 处理器（使用 SystemContext）
    pub fn with_context(router_id: Ipv4Addr, context: &'a SystemContext) -> Self {
        Self {
            router_id,
            context,
        }
    }

    /// 创建新的 OSPF 处理器（向后兼容，但不推荐使用）
    #[deprecated(note = "请使用 with_context 方法")]
    pub fn new(router_id: Ipv4Addr) -> Self {
        // 这个方法现在只是临时实现，需要调用者提供 context
        // 实际使用中应该使用 with_context
        panic!("OspfProcessor::new() 已废弃，请使用 with_context() 方法并传入 SystemContext");
    }

    /// 添加接口（已废弃，接口现在由 OspfManager 管理）
    #[deprecated(note = "接口现在由 OspfManager 管理")]
    pub fn add_interface(&mut self, _interface: OspfInterface) {
        // 不再需要，接口通过 OspfManager 管理
    }

    /// 处理接收到的 OSPF 报文
    pub fn process_packet(&mut self, packet: &mut Packet, ifindex: u32, source_ip: Ipv4Addr) -> OspfResult<OspfProcessResult> {
        // 解析 OSPF 头部
        let data = packet.as_remaining_slice();
        let header = OspfHeader::from_bytes(data)?;

        // 验证基本参数
        self.validate_header(&header)?;

        // 根据报文类型分发处理
        match header.packet_type {
            OspfType::Hello => self.process_hello(header, data, ifindex, source_ip),
            OspfType::DatabaseDescription => self.process_dd(header, data, ifindex),
            OspfType::LinkStateRequest => self.process_lsr(header, data, ifindex),
            OspfType::LinkStateUpdate => self.process_lsu(header, data, ifindex),
            OspfType::LinkStateAck => self.process_lsack(header, data, ifindex),
        }
    }

    /// 验证 OSPF 头部
    fn validate_header(&self, header: &OspfHeader) -> OspfResult<()> {
        // 验证版本
        if header.version != 2 {
            return Err(OspfError::parse_error(
                "version",
                format!("expected 2, got {}", header.version),
            ));
        }

        // 验证区域 ID
        if header.area_id != Ipv4Addr::new(0, 0, 0, 0) {
            // 非骨干区域需要额外的验证（简化实现）
        }

        Ok(())
    }

    /// 处理 Hello 报文
    fn process_hello(&mut self, header: OspfHeader, data: &[u8], ifindex: u32, source_ip: Ipv4Addr) -> OspfResult<OspfProcessResult> {
        // 跳过 OSPF 头部
        let hello_data = &data.get(OspfHeader::LENGTH..).unwrap_or(&[]);

        // 解析 Hello 报文
        let hello = OspfHello::from_bytes(hello_data)?;

        // 从 OspfManager 获取接口信息
        let mut ospf_mgr = self.context.ospf_manager.lock()
            .map_err(|e| OspfError::Other { reason: format!("锁定 OSPF 管理器失败: {}", e) })?;

        let interface = ospf_mgr.get_v2_interface(ifindex)
            .ok_or_else(|| OspfError::Other {
                reason: format!("Interface {} not found in OSPF manager", ifindex),
            })?;

        // 验证 Hello 参数
        interface.validate_hello_params(
            hello.hello_interval,
            hello.router_dead_interval,
            hello.network_mask,
        )?;

        let dead_interval = interface.dead_interval;
        let current_dr = interface.dr;
        let current_bdr = interface.bdr;
        let local_is_dr = interface.is_dr(self.router_id);
        let local_is_bdr = interface.is_bdr(self.router_id);

        // 释放接口锁后获取或创建邻居
        drop(interface);

        // 获取或创建邻居（使用 OspfManager）
        let neighbor = ospf_mgr.get_or_create_v2_neighbor(header.router_id, source_ip, dead_interval);
        let mut neighbor = neighbor.lock()
            .map_err(|e| OspfError::Other { reason: format!("锁定邻居失败: {}", e) })?;

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

        // 释放邻居锁后执行 DR/BDR 选举
        drop(neighbor);

        // 执行 DR/BDR 选举（简化实现）
        // TODO: 实现完整的 DR/BDR 选举算法

        Ok(OspfProcessResult::NoReply)
    }

    /// 处理 Database Description 报文
    ///
    /// RFC 2328 Section 10.6: 发送和接收数据库描述报文
    fn process_dd(&mut self, header: OspfHeader, data: &[u8], ifindex: u32) -> OspfResult<OspfProcessResult> {
        let dd_data = &data.get(OspfHeader::LENGTH..).unwrap_or(&[]);
        let dd = OspfDatabaseDescription::from_bytes(dd_data)?;

        // 获取邻居
        let mut ospf_mgr = self.context.ospf_manager.lock()
            .map_err(|e| OspfError::Other { reason: format!("锁定 OSPF 管理器失败: {}", e) })?;

        let neighbor = ospf_mgr.get_v2_neighbor(header.router_id)
            .ok_or_else(|| OspfError::Other { reason: format!("邻居 {} 未找到", header.router_id) })?;
        let mut neighbor = neighbor.lock()
            .map_err(|e| OspfError::Other { reason: format!("锁定邻居失败: {}", e) })?;

        // 1. 检查邻居状态
        if neighbor.state < NeighborState::ExStart {
            return Err(OspfError::Other { reason: "邻居状态错误，需要至少是 ExStart".to_string() });
        }

        // 2. 处理 ExStart 状态（Master/Slave 协商）
        if neighbor.state == NeighborState::ExStart {
            // 处理 I 位（Initialize）
            if dd.i_bit {
                // Master/Slave 协商
                // 比较 Router ID 和 DD 序列号来确定 Master/Slave
                let should_become_master = if dd.ms_bit {
                    // 对方声称是 Master
                    if dd.dd_sequence_number > neighbor.dd_seq_number {
                        // 对方序号更大，我们成为 Slave
                        neighbor.is_master = false;
                        false
                    } else if dd.dd_sequence_number < neighbor.dd_seq_number {
                        // 我们序号更大，保持 Master
                        neighbor.is_master = true;
                        true
                    } else {
                        // 序号相同，比较 Router ID
                        neighbor.is_master = self.router_id > header.router_id;
                        neighbor.is_master
                    }
                } else {
                    // 对方声称是 Slave，我们成为 Master
                    neighbor.is_master = true;
                    true
                };

                // 设置初始序列号
                neighbor.dd_seq_number = if should_become_master {
                    neighbor.dd_seq_number
                } else {
                    dd.dd_sequence_number
                };

                // 转换到 Exchange 状态
                neighbor.state = NeighborState::Exchange;
                return Ok(OspfProcessResult::Reply(self.build_empty_dd(
                    header.router_id,
                    ifindex,
                    neighbor.is_master,
                    neighbor.dd_seq_number,
                    true,  // M bit: 更多 LSA 后续发送
                    true,  // I bit: 初始化
                )?));
            }
        }

        // 3. 处理 Exchange 状态
        if neighbor.state == NeighborState::Exchange {
            // 验证序列号
            if neighbor.is_master {
                // Master：接收的序列号应该等于 ours + 1
                if dd.dd_sequence_number != neighbor.dd_seq_number + 1 {
                    // 序列号不匹配，可能是重复报文
                    return Ok(OspfProcessResult::NoReply);
                }
            } else {
                // Slave：接收的序列号应该等于 ours
                if dd.dd_sequence_number != neighbor.dd_seq_number {
                    return Ok(OspfProcessResult::NoReply);
                }
            }

            // 更新序列号
            neighbor.dd_seq_number = dd.dd_sequence_number;

            // 处理 LSA 头部列表
            let mut has_more_instances = false;
            for lsa_hdr in &dd.lsa_headers {
                // 检查是否需要请求此 LSA
                if self.should_request_lsa(&ospf_mgr, lsa_hdr) {
                    neighbor.add_lsa_request(
                        lsa_hdr.lsa_type,
                        lsa_hdr.link_state_id,
                        lsa_hdr.advertising_router,
                    );
                }
                has_more_instances = true;
            }

            // 检查是否 DD 交换完成
            if !dd.m_bit && !dd.i_bit {
                // DD 交换完成
                if neighbor.has_pending_requests() {
                    neighbor.state = NeighborState::Loading;
                } else {
                    neighbor.state = NeighborState::Full;
                }
            }

            // 发送响应（如果需要）
            if dd.i_bit || (neighbor.is_master && !dd.m_bit) {
                // 发送下一个 DD 报文
                neighbor.increment_dd_sequence();
                return Ok(OspfProcessResult::Reply(self.build_empty_dd(
                    header.router_id,
                    ifindex,
                    neighbor.is_master,
                    neighbor.dd_seq_number,
                    false,  // M bit: 没有更多 LSA
                    false,  // I bit: 不是初始化
                )?));
            }
        }

        Ok(OspfProcessResult::NoReply)
    }

    /// 检查是否需要请求 LSA
    fn should_request_lsa(&self, ospf_mgr: &crate::protocols::ospf::OspfManager, lsa_hdr: &LsaHeader) -> bool {
        // 查找本地 LSDB 中的 LSA
        let key = (lsa_hdr.lsa_type, lsa_hdr.link_state_id, lsa_hdr.advertising_router);
        if let Some(entry) = ospf_mgr.v2_lsdb.lsas.get(&key) {
            // 比较序列号
            if lsa_hdr.sequence_number > entry.header.sequence_number {
                return true;
            }
            // 序列号相同但校验和不同
            if lsa_hdr.sequence_number == entry.header.sequence_number && lsa_hdr.checksum != entry.header.checksum {
                return true;
            }
            false
        } else {
            // 本地没有此 LSA，需要请求
            true
        }
    }

    /// 构建空的 DD 报文（用于 ExStart 阶段）
    fn build_empty_dd(
        &self,
        neighbor_id: Ipv4Addr,
        ifindex: u32,
        is_master: bool,
        dd_seq_number: u32,
        m_bit: bool,
        i_bit: bool,
    ) -> OspfResult<Vec<u8>> {
        // 获取接口信息
        let interface = self.get_interface(ifindex)?;

        // 构建空的 DD 报文
        let mut dd = OspfDatabaseDescription::new(0, dd_seq_number);  // MTU: 0 表示不使用

        dd.options = 0;  // 简化实现
        dd.i_bit = i_bit;
        dd.m_bit = m_bit;
        dd.ms_bit = is_master;

        // 构建 OSPF 头部
        let mut header = OspfHeader::new(OspfType::DatabaseDescription, self.router_id, interface.area_id);
        let dd_bytes = dd.to_bytes();
        header.length = (OspfHeader::LENGTH + dd_bytes.len()) as u16;

        // 计算校验和
        header.calculate_checksum(&dd_bytes);

        // 组装完整报文
        let mut packet = header.to_bytes();
        packet.extend_from_slice(&dd_bytes);

        Ok(packet)
    }

    /// 处理 Link State Request 报文
    ///
    /// RFC 2328 Section 10.7: 接收链路状态请求报文
    fn process_lsr(&mut self, header: OspfHeader, data: &[u8], ifindex: u32) -> OspfResult<OspfProcessResult> {
        let lsr_data = &data.get(OspfHeader::LENGTH..).unwrap_or(&[]);
        let lsr = OspfLinkStateRequest::from_bytes(lsr_data)?;

        // 获取邻居
        let mut ospf_mgr = self.context.ospf_manager.lock()
            .map_err(|e| OspfError::Other { reason: format!("锁定 OSPF 管理器失败: {}", e) })?;

        let neighbor = ospf_mgr.get_v2_neighbor(header.router_id)
            .ok_or_else(|| OspfError::Other { reason: format!("邻居 {} 未找到", header.router_id) })?;
        let neighbor = neighbor.lock()
            .map_err(|e| OspfError::Other { reason: format!("锁定邻居失败: {}", e) })?;

        // 1. 检查邻居状态
        if neighbor.state < NeighborState::Exchange {
            return Err(OspfError::Other { reason: "邻居状态错误，需要至少 Exchange".to_string() });
        }

        // 释放邻居锁
        drop(neighbor);

        // 2. 解析 LSA 请求列表并查找请求的 LSA
        let mut found_lsas = Vec::new();
        for req in &lsr.requests {
            let key = (req.lsa_type, req.link_state_id, req.advertising_router);
            if let Some(entry) = ospf_mgr.v2_lsdb.lsas.get(&key) {
                // 从完整数据重建 LSA
                if let Ok(lsa) = self.parse_lsa_from_bytes(&entry.data) {
                    found_lsas.push(lsa);
                }
            }
        }

        // 3. 构建并发送 LSU 响应
        if !found_lsas.is_empty() {
            return Ok(OspfProcessResult::Reply(self.build_lsu_response(
                header.router_id,
                ifindex,
                &found_lsas,
            )?));
        }

        // 如果没有找到任何请求的 LSA，返回错误
        Err(OspfError::Other { reason: "未找到请求的 LSA".to_string() })
    }

    /// 从字节数据解析 LSA
    fn parse_lsa_from_bytes(&self, data: &[u8]) -> OspfResult<Lsa> {
        // 简化实现：仅解析 LSA 头部，返回最小化的 LSA
        if data.len() < LsaHeader::LENGTH {
            return Err(OspfError::ParseError {
                field: "LSA".to_string(),
                reason: "数据长度不足".to_string(),
            });
        }

        let header = LsaHeader::from_bytes(data)?;

        match header.lsa_type {
            1 => {
                // Router LSA - 创建空的 LSA
                let mut router_lsa = RouterLsa::new(
                    header.link_state_id,
                    header.advertising_router,
                );
                router_lsa.header = header;
                Ok(Lsa::Router(router_lsa))
            }
            2 => {
                // Network LSA - 创建空的 LSA
                let mut network_lsa = NetworkLsa::new(
                    header.link_state_id,
                    header.advertising_router,
                    Ipv4Addr::new(255, 255, 255, 0),  // 默认掩码
                );
                network_lsa.header = header;
                Ok(Lsa::Network(network_lsa))
            }
            3 => {
                // Summary LSA (Type 3) - 创建空的 LSA
                let lsa_type = LsaType::SummaryNetworkLsa;
                let mut summary_lsa = SummaryLsa::new(
                    lsa_type,
                    header.link_state_id,
                    header.advertising_router,
                    Ipv4Addr::new(255, 255, 255, 0),  // 默认掩码
                    0,  // 默认 metric
                );
                summary_lsa.header = header;
                Ok(Lsa::SummaryNetwork(summary_lsa))
            }
            4 => {
                // Summary LSA (Type 4) - 创建空的 LSA
                let lsa_type = LsaType::SummaryAsbrLsa;
                let mut summary_lsa = SummaryLsa::new(
                    lsa_type,
                    header.link_state_id,
                    header.advertising_router,
                    Ipv4Addr::new(255, 255, 255, 0),  // 默认掩码
                    0,  // 默认 metric
                );
                summary_lsa.header = header;
                Ok(Lsa::SummaryAsbr(summary_lsa))
            }
            5 => {
                // AS External LSA - 创建空的 LSA
                let mut external_lsa = AsExternalLsa::new(
                    header.link_state_id,
                    header.advertising_router,
                    Ipv4Addr::new(0, 0, 0, 0),  // 默认转发地址
                    0,  // 默认 metric
                    false,  // 默认 e_bit
                );
                external_lsa.header = header;
                Ok(Lsa::ASExternal(external_lsa))
            }
            _ => Err(OspfError::ParseError {
                field: "LSA".to_string(),
                reason: format!("未知的 LSA 类型: {}", header.lsa_type),
            }),
        }
    }

    /// 构建 LSU 响应报文
    fn build_lsu_response(
        &self,
        neighbor_id: Ipv4Addr,
        ifindex: u32,
        lsas: &[Lsa],
    ) -> OspfResult<Vec<u8>> {
        // 获取接口信息
        let interface = self.get_interface(ifindex)?;

        // 构建 LSU 报文
        let mut lsu = OspfLinkStateUpdate::new();
        for lsa in lsas {
            lsu.add_lsa(lsa.clone());
        }

        // 构建 OSPF 头部
        let mut header = OspfHeader::new(OspfType::LinkStateUpdate, self.router_id, interface.area_id);
        let lsu_bytes = lsu.to_bytes();
        header.length = (OspfHeader::LENGTH + lsu_bytes.len()) as u16;

        // 计算校验和
        header.calculate_checksum(&lsu_bytes);

        // 组装完整报文
        let mut packet = header.to_bytes();
        packet.extend_from_slice(&lsu_bytes);

        Ok(packet)
    }

    /// 处理 Link State Update 报文
    ///
    /// RFC 2328 Section 13: 洪泛链路状态更新
    fn process_lsu(&mut self, header: OspfHeader, data: &[u8], ifindex: u32) -> OspfResult<OspfProcessResult> {
        // 跳过 OSPF 头部和 LSA 数量字段
        let lsu_data = &data.get(OspfHeader::LENGTH + 4..).unwrap_or(&[]);

        // 解析 LSA 数量
        let lsa_count = u32::from_be_bytes([
            data[OspfHeader::LENGTH],
            data[OspfHeader::LENGTH + 1],
            data[OspfHeader::LENGTH + 2],
            data[OspfHeader::LENGTH + 3],
        ]) as usize;

        // 获取 OSPF 管理器和洪泛器
        let mut ospf_mgr = self.context.ospf_manager.lock()
            .map_err(|e| OspfError::Other { reason: format!("锁定 OSPF 管理器失败: {}", e) })?;

        let mut flooder = LsaFlooder::new();
        let mut trigger_spf = false;
        let mut lsas_to_ack = Vec::new();

        // 解析并处理每个 LSA
        let mut offset = 0;
        for _ in 0..lsa_count {
            if offset >= lsu_data.len() {
                break;
            }

            // 解析 LSA 头部
            let lsa_header = LsaHeader::from_bytes(&lsu_data[offset..])?;
            let lsa_length = (lsa_header.length as usize) * 4;  // LSA 长度以 4 字节为单位

            if offset + lsa_length > lsu_data.len() {
                break;
            }

            // 解析完整 LSA
            let lsa_data = &lsu_data[offset..offset + lsa_length];
            let lsa = self.parse_lsa_from_bytes(lsa_data)?;

            // 记录需要确认的 LSA 头部
            lsas_to_ack.push(lsa_header.clone());

            // 使用洪泛器处理 LSA
            let interfaces: Vec<OspfInterface> = ospf_mgr.v2_interfaces.clone();
            let neighbors = ospf_mgr.v2_neighbors.clone();

            match flooder.process_received_lsa(
                lsa,
                ifindex,
                header.router_id,
                &mut ospf_mgr.v2_lsdb,
                &interfaces,
                &neighbors,
            ) {
                crate::protocols::ospf::FloodResult::InstalledAndFlood { targets, trigger_spf: should_trigger } => {
                    if should_trigger {
                        trigger_spf = true;
                    }
                    // 这里应该触发 LSU 洪泛到目标邻居
                    // 简化实现：在实际发送时由引擎层处理
                }
                crate::protocols::ospf::FloodResult::InstalledWithoutFlooding => {
                    // LSA 已安装但不需要洪泛
                }
                crate::protocols::ospf::FloodResult::AlreadyPresent => {
                    // LSA 已存在
                }
                crate::protocols::ospf::FloodResult::Invalid => {
                    // LSA 无效，丢弃
                }
            }

            offset += lsa_length;
        }

        // 发送 LSAck 确认
        if !lsas_to_ack.is_empty() {
            self.send_lsack_for_lsu(header.router_id);
        }

        // 如果需要，触发 SPF 计算
        if trigger_spf {
            return Ok(OspfProcessResult::ScheduleSpfCalculation);
        }

        Ok(OspfProcessResult::NoReply)
    }

    /// 处理 Link State Acknowledgment 报文
    ///
    /// RFC 2328 Section 10.8: 发送链路状态确认报文
    fn process_lsack(&mut self, header: OspfHeader, data: &[u8], ifindex: u32) -> OspfResult<OspfProcessResult> {
        let lsack_data = &data.get(OspfHeader::LENGTH..).unwrap_or(&[]);
        let lsack = OspfLinkStateAck::from_bytes(lsack_data)?;

        // 获取邻居
        let mut ospf_mgr = self.context.ospf_manager.lock()
            .map_err(|e| OspfError::Other { reason: format!("锁定 OSPF 管理器失败: {}", e) })?;

        let neighbor = ospf_mgr.get_v2_neighbor(header.router_id)
            .ok_or_else(|| OspfError::Other { reason: format!("邻居 {} 未找到", header.router_id) })?;
        let mut neighbor = neighbor.lock()
            .map_err(|e| OspfError::Other { reason: format!("锁定邻居失败: {}", e) })?;

        // 1. 检查邻居状态
        if neighbor.state < NeighborState::Exchange {
            return Err(OspfError::Other { reason: "邻居状态错误，需要至少 Exchange".to_string() });
        }

        // 2. 从重传列表中移除已确认的 LSA
        for lsa_hdr in &lsack.lsa_headers {
            neighbor.remove_retransmission(lsa_hdr);
        }

        Ok(OspfProcessResult::NoReply)
    }

    /// 发送 LSAck
    fn send_lsack_for_lsu(&mut self, destination: Ipv4Addr) {
        // 简化实现：记录需要发送确认
        // 在实际实现中，应该构建 LSAck 报文并发送
        // 这里只是占位，实际的发送应该在引擎层完成
        let _ = destination;
    }

    /// 构建 LSAck 报文
    fn build_lsack_packet(
        &self,
        _destination: Ipv4Addr,
        ifindex: u32,
        lsa_headers: &[LsaHeader],
    ) -> OspfResult<Vec<u8>> {
        // 获取接口信息
        let interface = self.get_interface(ifindex)?;

        // 构建 LSAck 报文
        let mut lsack = OspfLinkStateAck::new();
        for lsa_hdr in lsa_headers {
            lsack.add_lsa_header(lsa_hdr.clone());
        }

        // 构建 OSPF 头部
        let mut header = OspfHeader::new(OspfType::LinkStateAck, self.router_id, interface.area_id);
        let lsack_bytes = lsack.to_bytes();
        header.length = (OspfHeader::LENGTH + lsack_bytes.len()) as u16;

        // 计算校验和
        header.calculate_checksum(&lsack_bytes);

        // 组装完整报文
        let mut packet = header.to_bytes();
        packet.extend_from_slice(&lsack_bytes);

        Ok(packet)
    }

    /// 获取接口（从 OspfManager）
    fn get_interface(&self, ifindex: u32) -> OspfResult<OspfInterface> {
        let ospf_mgr = self.context.ospf_manager.lock()
            .map_err(|e| OspfError::Other { reason: format!("锁定 OSPF 管理器失败: {}", e) })?;

        ospf_mgr.get_v2_interface(ifindex)
            .ok_or_else(|| OspfError::Other {
                reason: format!("Interface {} not found", ifindex),
            })
            .map(|iface| iface.clone())
    }

    /// 获取可变接口引用（通过回调）
    fn with_interface_mut<F, R>(&self, ifindex: u32, f: F) -> OspfResult<R>
    where
        F: FnOnce(&mut OspfInterface) -> OspfResult<R>,
    {
        let mut ospf_mgr = self.context.ospf_manager.lock()
            .map_err(|e| OspfError::Other { reason: format!("锁定 OSPF 管理器失败: {}", e) })?;

        let iface = ospf_mgr.get_v2_interface_mut(ifindex)
            .ok_or_else(|| OspfError::Other {
                reason: format!("Interface {} not found", ifindex),
            })?;

        f(iface)
    }
}

// 移除了以下已废弃的方法：
// - find_or_create_neighbor (现在通过 OspfManager 处理)
// - update_neighbor_from_hello (已整合到 process_hello 中)
// - elect_dr_bdr (现在通过 OspfManager 处理)
// - elect_dr_bdr_internal (现在通过 OspfManager 处理)

/// 封装 OSPFv2 Hello 报文
pub fn encapsulate_ospfv2_hello(
    router_id: Ipv4Addr,
    area_id: Ipv4Addr,
    interface: &OspfInterface,
    neighbors: &[Ipv4Addr],
) -> Vec<u8> {
    // 构建 Hello 报文
    let mut hello = OspfHello::new(
        interface.mask,
        interface.hello_interval,
        interface.dead_interval,
        interface.priority,
    );

    hello.designated_router = interface.dr;
    hello.backup_designated_router = interface.bdr;

    for neighbor in neighbors {
        hello.add_neighbor(*neighbor);
    }

    // 构建 OSPF 头部
    let mut header = OspfHeader::new(OspfType::Hello, router_id, area_id);

    // 计算总长度
    let hello_bytes = hello.to_bytes();
    header.length = (OspfHeader::LENGTH + hello_bytes.len()) as u16;

    // 计算校验和
    header.calculate_checksum(&hello_bytes);

    // 组装完整报文
    let mut packet = header.to_bytes();
    packet.extend_from_slice(&hello_bytes);

    packet
}

/// 处理 OSPFv2 报文（入口函数）
pub fn process_ospfv2_packet(
    packet: &mut Packet,
    ifindex: u32,
    source_ip: Ipv4Addr,
    context: &SystemContext,
) -> OspfResult<OspfProcessResult> {
    // 获取路由器 ID
    let router_id = {
        let interfaces = context.interfaces.lock()
            .map_err(|e| OspfError::Other { reason: format!("锁定接口管理器失败: {}", e) })?;
        let iface = interfaces.get_by_index(ifindex)
            .map_err(|e| OspfError::Other { reason: format!("获取接口失败: {}", e) })?;
        // 使用接口 IP 作为路由器 ID（简化实现）
        iface.ip_addr
    };

    // 创建使用 context 的处理器
    let mut processor = OspfProcessor::with_context(router_id, context);
    processor.process_packet(packet, ifindex, source_ip)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::SystemContext;
    use std::sync::{Arc, Mutex};

    #[test]
    fn test_ospf_processor_with_context() {
        let context = SystemContext::new();
        let router_id = Ipv4Addr::new(1, 1, 1, 1);
        let processor = OspfProcessor::with_context(router_id, &context);
        assert_eq!(processor.router_id, router_id);
        // 检查 OSPF 管理器是否存在且为空
        let ospf_mgr = context.ospf_manager.lock().unwrap();
        assert!(ospf_mgr.v2_interfaces.is_empty());
        assert!(ospf_mgr.v2_neighbors.is_empty());
    }

    #[test]
    fn test_ospf_header_validate() {
        let context = SystemContext::new();
        let processor = OspfProcessor::with_context(Ipv4Addr::new(1, 1, 1, 1), &context);

        let header = OspfHeader::new(
            OspfType::Hello,
            Ipv4Addr::new(1, 1, 1, 1),
            Ipv4Addr::new(0, 0, 0, 0),
        );

        assert!(processor.validate_header(&header).is_ok());
    }

    #[test]
    fn test_ospf_hello_round_trip() {
        let hello = OspfHello::new(
            Ipv4Addr::new(255, 255, 255, 0),
            10,
            40,
            1,
        );

        let bytes = hello.to_bytes();
        let parsed = OspfHello::from_bytes(&bytes).unwrap();

        assert_eq!(parsed.network_mask, hello.network_mask);
        assert_eq!(parsed.hello_interval, hello.hello_interval);
    }

    #[test]
    fn test_encapsulate_ospfv2_hello() {
        let mut interface = OspfInterface::new(
            "eth0".to_string(),
            1,
            Ipv4Addr::new(192, 168, 1, 1),
            Ipv4Addr::new(255, 255, 255, 0),
            Ipv4Addr::new(0, 0, 0, 0),
        );

        let packet = encapsulate_ospfv2_hello(
            Ipv4Addr::new(1, 1, 1, 1),
            Ipv4Addr::new(0, 0, 0, 0),
            &interface,
            &[],
        );

        assert!(packet.len() >= OspfHeader::LENGTH);

        let header = OspfHeader::from_bytes(&packet).unwrap();
        assert_eq!(header.packet_type, OspfType::Hello);
    }
}
