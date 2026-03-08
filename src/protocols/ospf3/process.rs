// src/protocols/ospf3/process.rs
//
// OSPFv3 报文处理逻辑（精简版）

use crate::common::Packet;
use crate::context::SystemContext;
use super::error::{Ospfv3Error, Ospfv3Result};
use super::packet::*;

/// OSPFv3 处理结果（精简版）
#[derive(Debug, Clone)]
pub enum Ospfv3ProcessResult {
    /// 无响应
    NoReply,
    /// 需要发送响应报文
    Reply(Vec<u8>),
}

/// 处理 OSPFv3 报文（精简版）
///
/// 仅解析报文头部并打印日志，不进行完整处理
pub fn process_ospfv3_packet(
    packet: &mut Packet,
    _ifindex: u32,
    context: &SystemContext,
    verbose: bool,
) -> Ospfv3Result<Ospfv3ProcessResult> {
    // 验证 OSPF 管理器可访问（简化处理）
    drop(context.ospf_manager.lock().map_err(|_| Ospfv3Error::LockError)?);

    // 尝试解析 OSPFv3 头部
    let data = packet.peek(packet.remaining()).unwrap_or(&[]);
    match Ospfv3Header::from_bytes(data) {
        Ok(header) => {
            if verbose {
                println!("OSPFv3: 收到 {:?} 报文 from router {}",
                    header.packet_type, header.router_id);
            }

            // 简化处理：只处理 Hello 报文的基本字段
            if header.packet_type == 1 { // Hello
                if verbose {
                    println!("OSPFv3: Hello 报文（简化处理）");
                }
            } else if verbose {
                println!("OSPFv3: 类型 {} 报文（简化处理，不处理）", header.packet_type);
            }

            Ok(Ospfv3ProcessResult::NoReply)
        }
        Err(e) => {
            if verbose {
                println!("OSPFv3: 报文解析失败: {:?}", e);
            }
            Err(e)
        }
    }
}
