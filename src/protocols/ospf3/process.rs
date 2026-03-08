// src/protocols/ospf3/process.rs
//
// OSPFv3 报文处理逻辑（简化版）

use crate::common::Packet;
use crate::context::SystemContext;
use super::error::Ospfv3Result;

/// OSPFv3 处理结果（简化版）
#[derive(Debug, Clone)]
pub enum Ospfv3ProcessResult {
    /// 无响应
    NoReply,
    /// 需要发送响应报文
    Reply(Vec<u8>),
}

/// 处理 OSPFv3 报文（简化版接口）
///
/// 仅解析报文头部并打印日志，不进行完整处理
pub fn process_ospfv3_packet(
    _packet: &mut Packet,
    _ifindex: u32,
    _context: &SystemContext,
    _verbose: bool,
) -> Ospfv3Result<Ospfv3ProcessResult> {
    // 简化实现：直接返回无响应
    Ok(Ospfv3ProcessResult::NoReply)
}
