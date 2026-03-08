// src/protocols/ospf2/process.rs
//
// OSPFv2 报文处理逻辑（简化版）

use crate::common::Packet;
use crate::context::SystemContext;
use super::error::OspfResult;

/// OSPF 处理结果（简化版）
#[derive(Debug, Clone)]
pub enum OspfProcessResult {
    /// 无响应
    NoReply,
    /// 需要发送响应报文
    Reply(Vec<u8>),
}

/// 处理 OSPFv2 报文（简化版接口）
///
/// 仅解析报文头部并打印日志，不进行完整处理
pub fn process_ospfv2_packet(
    _packet: &mut Packet,
    _ifindex: u32,
    _context: &SystemContext,
    _verbose: bool,
) -> OspfResult<OspfProcessResult> {
    // 简化实现：直接返回无响应
    Ok(OspfProcessResult::NoReply)
}
