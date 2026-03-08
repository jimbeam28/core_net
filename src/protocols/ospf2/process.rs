// src/protocols/ospf2/process.rs
//
// OSPFv2 报文处理逻辑（精简版）

use crate::common::Packet;
use crate::context::SystemContext;
use super::error::{OspfError, OspfResult};
use super::packet::*;

/// OSPF 处理结果（精简版）
#[derive(Debug, Clone)]
pub enum OspfProcessResult {
    /// 无响应
    NoReply,
    /// 需要发送响应报文
    Reply(Vec<u8>),
}

/// 处理 OSPFv2 报文（精简版）
///
/// 仅解析报文头部并打印日志，不进行完整处理
pub fn process_ospfv2_packet(
    packet: &mut Packet,
    _ifindex: u32,
    context: &SystemContext,
    verbose: bool,
) -> OspfResult<OspfProcessResult> {
    // 验证 OSPF 管理器可访问（简化处理）
    drop(context.ospf_manager.lock().map_err(|_| OspfError::LockError)?);

    // 尝试解析 OSPF 头部
    let data = packet.peek(packet.remaining()).unwrap_or(&[]);
    match OspfHeader::from_bytes(data) {
        Ok(header) => {
            if verbose {
                println!("OSPFv2: 收到 {:?} 报文 from router {}",
                    header.packet_type, header.router_id);
            }

            // 简化处理：只处理 Hello 报文的基本字段
            match header.packet_type {
                OspfType::Hello => {
                    if verbose {
                        println!("OSPFv2: Hello 报文（简化处理）");
                    }
                }
                _ => {
                    if verbose {
                        println!("OSPFv2: {:?} 报文（简化处理，不处理）", header.packet_type);
                    }
                }
            }

            Ok(OspfProcessResult::NoReply)
        }
        Err(e) => {
            if verbose {
                println!("OSPFv2: 报文解析失败: {:?}", e);
            }
            Err(e)
        }
    }
}
