// src/protocols/bgp/process.rs
//
// BGP 报文处理逻辑（精简版）

use crate::context::SystemContext;
use std::net::IpAddr;

use super::error::Result;
use super::message::{BgpMessage, BgpKeepalive};
use super::packet::{parse_bgp_message, encapsulate_bgp_message};

/// BGP 处理结果（精简版）
#[derive(Debug, Clone, PartialEq)]
pub enum BgpProcessResult {
    /// 无需响应
    NoReply,
    /// 需要发送 BGP 响应
    Reply(Vec<u8>),
    /// 连接已建立
    ConnectionEstablished,
    /// 连接已关闭
    ConnectionClosed,
}

/// 处理接收到的 BGP 报文（精简版）
///
/// 仅解析报文并打印日志，不进行完整处理
pub fn process_bgp_packet(
    data: &[u8],
    source_addr: IpAddr,
    _context: &SystemContext,
    verbose: bool,
) -> Result<BgpProcessResult> {
    // 解析 BGP 报文
    let message = parse_bgp_message(data)?;

    if verbose {
        println!("BGP: 收到报文 from {}: {:?}", source_addr, message);
    }

    match message {
        BgpMessage::Open(open) => {
            if verbose {
                println!("BGP: Open 报文 - AS: {}, Hold Time: {}, ID: {}",
                    open.my_as, open.hold_time, open.bgp_identifier);
            }
            // 简化处理：发送 Keepalive 响应
            let keepalive = BgpMessage::Keepalive(BgpKeepalive);
            let response = encapsulate_bgp_message(&keepalive);
            Ok(BgpProcessResult::Reply(response))
        }
        BgpMessage::Update(update) => {
            if verbose {
                println!("BGP: Update 报文 - {} 个前缀",
                    update.nlri.len());
            }
            Ok(BgpProcessResult::NoReply)
        }
        BgpMessage::Notification(notif) => {
            if verbose {
                println!("BGP: Notification 报文 - Code: {}, Subcode: {}",
                    notif.error_code, notif.error_subcode);
            }
            Ok(BgpProcessResult::ConnectionClosed)
        }
        BgpMessage::Keepalive(_) => {
            if verbose {
                println!("BGP: Keepalive 报文");
            }
            Ok(BgpProcessResult::NoReply)
        }
        _ => {
            if verbose {
                println!("BGP: 其他类型报文");
            }
            Ok(BgpProcessResult::NoReply)
        }
    }
}
