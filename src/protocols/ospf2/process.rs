// src/protocols/ospf2/process.rs
//
// OSPFv2 报文处理逻辑

use crate::common::{Packet, Ipv4Addr};
use super::error::{OspfError, OspfResult};
use super::packet::*;
use super::lsa::*;
use super::interface::{OspfInterface, InterfaceState};
use super::neighbor::{OspfNeighbor, NeighborState};
use super::lsdb::LinkStateDatabase;

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
pub struct OspfProcessor {
    /// 路由器 ID
    pub router_id: Ipv4Addr,

    /// 接口列表（按接口索引）
    pub interfaces: Vec<OspfInterface>,

    /// 邻居列表（按 Router ID）
    pub neighbors: Vec<OspfNeighbor>,

    /// 链路状态数据库
    pub lsdb: LinkStateDatabase,
}

impl OspfProcessor {
    /// 创建新的 OSPF 处理器
    pub fn new(router_id: Ipv4Addr) -> Self {
        Self {
            router_id,
            interfaces: Vec::new(),
            neighbors: Vec::new(),
            lsdb: LinkStateDatabase::new(),
        }
    }

    /// 添加接口
    pub fn add_interface(&mut self, interface: OspfInterface) {
        self.interfaces.push(interface);
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

        // 获取接口信息用于验证和后续处理
        let interface_idx = self.interfaces.iter()
            .position(|iface| iface.ifindex == ifindex)
            .ok_or_else(|| OspfError::Other {
                reason: format!("Interface {} not found", ifindex),
            })?;

        // 验证 Hello 参数
        self.interfaces[interface_idx].validate_hello_params(
            hello.hello_interval,
            hello.router_dead_interval,
            hello.network_mask,
        )?;

        let dead_interval = self.interfaces[interface_idx].dead_interval;
        let current_dr = self.interfaces[interface_idx].dr;
        let current_bdr = self.interfaces[interface_idx].bdr;
        let local_is_dr = self.interfaces[interface_idx].is_dr(self.router_id);
        let local_is_bdr = self.interfaces[interface_idx].is_bdr(self.router_id);

        // 查找或创建邻居索引
        let neighbor_idx = if let Some(idx) = self.neighbors.iter().position(|n| n.router_id == header.router_id) {
            idx
        } else {
            // 创建新邻居
            let neighbor = OspfNeighbor::new(header.router_id, source_ip, dead_interval);
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

        Ok(OspfProcessResult::NoReply)
    }

    /// 处理 Database Description 报文
    fn process_dd(&mut self, _header: OspfHeader, _data: &[u8], _ifindex: u32) -> OspfResult<OspfProcessResult> {
        // DD 报文处理逻辑
        // 1. 检查邻居状态
        // 2. 处理 I/M/MS 位
        // 3. 比较 LSA 头部
        // 4. 更新请求列表

        Ok(OspfProcessResult::NoReply)
    }

    /// 处理 Link State Request 报文
    fn process_lsr(&mut self, _header: OspfHeader, _data: &[u8], _ifindex: u32) -> OspfResult<OspfProcessResult> {
        // LSR 报文处理逻辑
        // 1. 解析请求列表
        // 2. 查找请求的 LSA
        // 3. 发送 LSU 响应

        Ok(OspfProcessResult::NoReply)
    }

    /// 处理 Link State Update 报文
    fn process_lsu(&mut self, header: OspfHeader, data: &[u8], ifindex: u32) -> OspfResult<OspfProcessResult> {
        // LSU 报文处理逻辑
        // 1. 解析 LSA 数量
        // 2. 处理每个 LSA
        // 3. 更新 LSDB
        // 4. 洪泛 LSA
        // 5. 发送确认
        // 6. 触发 SPF 计算

        let _interface = self.get_interface(ifindex)?;

        // 跳过 OSPF 头部和 LSA 数量字段
        let _lsu_data = &data.get(OspfHeader::LENGTH + 4..).unwrap_or(&[]);

        // 简化实现：只发送确认
        self.send_lsack_for_lsu(header.router_id);

        Ok(OspfProcessResult::ScheduleSpfCalculation)
    }

    /// 处理 Link State Acknowledgment 报文
    fn process_lsack(&mut self, _header: OspfHeader, _data: &[u8], _ifindex: u32) -> OspfResult<OspfProcessResult> {
        // LSAck 报文处理逻辑
        // 1. 解析 LSA 头部列表
        // 2. 从重传列表中移除已确认的 LSA

        Ok(OspfProcessResult::NoReply)
    }

    /// 查找或创建邻居
    fn find_or_create_neighbor(&mut self, router_id: Ipv4Addr, ip_addr: Ipv4Addr, dead_interval: u32) -> &mut OspfNeighbor {
        // 查找现有邻居
        if let Some(idx) = self.neighbors.iter().position(|n| n.router_id == router_id) {
            return &mut self.neighbors[idx];
        }

        // 创建新邻居
        let neighbor = OspfNeighbor::new(router_id, ip_addr, dead_interval);
        self.neighbors.push(neighbor);
        self.neighbors.last_mut().unwrap()
    }

    /// 从 Hello 报文更新邻居状态
    fn update_neighbor_from_hello(&mut self, neighbor: &mut OspfNeighbor, hello: &OspfHello, source_id: Ipv4Addr, interface: &OspfInterface) {
        // 更新邻居的基本信息
        neighbor.priority = hello.router_priority;
        neighbor.dr = hello.designated_router;
        neighbor.bdr = hello.backup_designated_router;

        // 重置 Inactivity Timer
        neighbor.reset_inactivity_timer(interface.dead_interval);

        // 状态转换
        match neighbor.state {
            NeighborState::Down => {
                // 收到 Hello，进入 Init 状态
                neighbor.state = NeighborState::Init;
            }
            NeighborState::Init => {
                // 检查邻居列表中是否包含自己
                if hello.neighbors.contains(&self.router_id) {
                    // 双向通信已建立
                    neighbor.state = NeighborState::TwoWay;

                    // 判断是否需要建立邻接关系
                    let local_is_dr = interface.is_dr(self.router_id);
                    let local_is_bdr = interface.is_bdr(self.router_id);
                    let neighbor_is_dr = interface.is_dr(source_id);
                    let neighbor_is_bdr = interface.is_bdr(source_id);

                    if neighbor.needs_adjacency(local_is_dr, local_is_bdr, neighbor_is_dr, neighbor_is_bdr) {
                        // 开始数据库交换
                        neighbor.state = NeighborState::ExStart;
                        neighbor.init_dd_sequence();
                    }
                }
            }
            _ => {
                // 其他状态下只需重置定时器
            }
        }
    }

    /// 执行 DR/BDR 选举
    fn elect_dr_bdr(&mut self, interface: &mut OspfInterface) {
        // 简化的 DR/BDR 选举算法
        // 1. 获取所有有资格的路由器（Priority > 0）
        let mut candidates: Vec<(Ipv4Addr, u8)> = vec![];

        // 添加自己
        if interface.is_eligible_for_dr() {
            candidates.push((self.router_id, interface.priority));
        }

        // 添加邻居
        for neighbor in &self.neighbors {
            if neighbor.state.is_two_way_established() && neighbor.priority > 0 {
                candidates.push((neighbor.router_id, neighbor.priority));
            }
        }

        // 选举 DR
        let mut dr = Ipv4Addr::UNSPECIFIED;
        let mut dr_priority = 0;
        let mut dr_id = Ipv4Addr::UNSPECIFIED;

        // 选举 BDR
        let mut bdr = Ipv4Addr::UNSPECIFIED;
        let mut bdr_priority = 0;
        let mut bdr_id = Ipv4Addr::UNSPECIFIED;

        for (router_id, priority) in &candidates {
            // DR 选举
            if *priority > dr_priority || (*priority == dr_priority && *router_id > dr_id) {
                dr_priority = *priority;
                dr_id = *router_id;
                dr = *router_id;
            }

            // BDR 选举（排除已选为 DR 的）
            if *router_id != dr && (*priority > bdr_priority || (*priority == bdr_priority && *router_id > bdr_id)) {
                bdr_priority = *priority;
                bdr_id = *router_id;
                bdr = *router_id;
            }
        }

        // 更新接口的 DR/BDR
        interface.set_dr(dr);
        interface.set_bdr(bdr);

        // 更新接口状态
        if interface.is_dr(self.router_id) {
            interface.state = InterfaceState::DR;
        } else if interface.is_bdr(self.router_id) {
            interface.state = InterfaceState::Backup;
        } else {
            interface.state = InterfaceState::DROther;
        }
    }

    /// 执行 DR/BDR 选举（内部版本，使用索引）
    fn elect_dr_bdr_internal(&mut self, interface_idx: usize) {
        // 简化的 DR/BDR 选举算法
        // 1. 获取所有有资格的路由器（Priority > 0）
        let mut candidates: Vec<(Ipv4Addr, u8)> = vec![];

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

        // 选举 DR
        let mut dr = Ipv4Addr::UNSPECIFIED;
        let mut dr_priority = 0;
        let mut dr_id = Ipv4Addr::UNSPECIFIED;

        // 选举 BDR
        let mut bdr = Ipv4Addr::UNSPECIFIED;
        let mut bdr_priority = 0;
        let mut bdr_id = Ipv4Addr::UNSPECIFIED;

        for (router_id, priority) in &candidates {
            // DR 选举
            if *priority > dr_priority || (*priority == dr_priority && *router_id > dr_id) {
                dr_priority = *priority;
                dr_id = *router_id;
                dr = *router_id;
            }

            // BDR 选举（排除已选为 DR 的）
            if *router_id != dr && (*priority > bdr_priority || (*priority == bdr_priority && *router_id > bdr_id)) {
                bdr_priority = *priority;
                bdr_id = *router_id;
                bdr = *router_id;
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

    /// 发送 LSAck
    fn send_lsack_for_lsu(&mut self, _destination: Ipv4Addr) {
        // 发送直接确认
    }

    /// 获取接口
    fn get_interface(&self, ifindex: u32) -> OspfResult<&OspfInterface> {
        self.interfaces.iter()
            .find(|iface| iface.ifindex == ifindex)
            .ok_or_else(|| OspfError::Other {
                reason: format!("Interface {} not found", ifindex),
            })
    }

    /// 获取可变接口
    fn get_interface_mut(&mut self, ifindex: u32) -> OspfResult<&mut OspfInterface> {
        self.interfaces.iter_mut()
            .find(|iface| iface.ifindex == ifindex)
            .ok_or_else(|| OspfError::Other {
                reason: format!("Interface {} not found", ifindex),
            })
    }
}

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
    router_id: Ipv4Addr,
    source_ip: Ipv4Addr,
) -> OspfResult<OspfProcessResult> {
    let mut processor = OspfProcessor::new(router_id);
    processor.process_packet(packet, ifindex, source_ip)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ospf_processor_new() {
        let processor = OspfProcessor::new(Ipv4Addr::new(1, 1, 1, 1));
        assert_eq!(processor.router_id, Ipv4Addr::new(1, 1, 1, 1));
        assert!(processor.interfaces.is_empty());
        assert!(processor.neighbors.is_empty());
    }

    #[test]
    fn test_ospf_processor_add_interface() {
        let mut processor = OspfProcessor::new(Ipv4Addr::new(1, 1, 1, 1));

        let interface = OspfInterface::new(
            "eth0".to_string(),
            1,
            Ipv4Addr::new(192, 168, 1, 1),
            Ipv4Addr::new(255, 255, 255, 0),
            Ipv4Addr::new(0, 0, 0, 0),
        );

        processor.add_interface(interface);
        assert_eq!(processor.interfaces.len(), 1);
    }

    #[test]
    fn test_ospf_header_validate() {
        let processor = OspfProcessor::new(Ipv4Addr::new(1, 1, 1, 1));

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
