// src/protocols/icmp/process.rs
//
// ICMP 报文处理逻辑

use crate::common::{CoreError, Packet, Result};
use crate::protocols::Ipv4Addr;
use crate::protocols::ip::verify_checksum;
use crate::context::SystemContext;

use super::packet::{IcmpPacket, IcmpEcho};
use super::types::*;
use super::echo::{handle_echo_request, handle_echo_reply, EchoProcessResult};

/// ICMP 处理结果
#[derive(Debug, Clone, PartialEq)]
pub enum IcmpProcessResult {
    /// 无需响应
    NoReply,

    /// 需要发送 ICMP 响应报文
    Reply(Vec<u8>),

    /// 处理完成（无需发送响应）
    Processed,
}

/// 处理接收到的 ICMP 报文
///
/// # 参数
/// - packet: ICMP 报文（不包含 IP 头部）
/// - source_addr: 发送方 IP 地址
/// - dest_addr: 接收方 IP 地址（本接口 IP）
/// - context: 系统上下文（包含 Echo 管理器）
/// - verbose: 是否打印详细信息
///
/// # 返回
/// - Ok(IcmpProcessResult): 处理结果
/// - Err(CoreError): 处理失败
pub fn process_icmp_packet(
    mut packet: Packet,
    source_addr: Ipv4Addr,
    dest_addr: Ipv4Addr,
    context: &SystemContext,
    verbose: bool,
) -> Result<IcmpProcessResult> {
    // 读取数据用于校验和验证
    let data = packet.peek(packet.remaining()).unwrap_or(&[]);

    // 验证校验和
    if !verify_checksum(data, 2) {
        return Err(CoreError::invalid_packet("ICMP校验和错误"));
    }

    // 解析 ICMP 报文
    let icmp_packet = IcmpPacket::from_packet(&mut packet)?;

    if verbose {
        println!("ICMP: Type={} Source={} Dest={}",
            icmp_packet.get_type(), source_addr, dest_addr);
    }

    // 根据类型处理
    match icmp_packet {
        IcmpPacket::Echo(echo) => {
            handle_echo_packet(echo, source_addr, dest_addr, context, verbose)
        }
        IcmpPacket::DestUnreachable(_) => {
            // Destination Unreachable 是错误消息，不需要响应
            if verbose {
                println!("ICMP: 收到 Destination Unreachable");
            }
            Ok(IcmpProcessResult::Processed)
        }
        IcmpPacket::TimeExceeded(_) => {
            // Time Exceeded 是错误消息，不需要响应
            if verbose {
                println!("ICMP: 收到 Time Exceeded");
            }
            Ok(IcmpProcessResult::Processed)
        }
    }
}

/// 处理 Echo 报文
fn handle_echo_packet(
    echo: IcmpEcho,
    source_addr: Ipv4Addr,
    dest_addr: Ipv4Addr,
    context: &SystemContext,
    verbose: bool,
) -> Result<IcmpProcessResult> {
    if echo.is_request() {
        // 处理 Echo Request
        if verbose {
            println!("ICMP: 收到 Echo Request ID={} Seq={} from {}",
                echo.identifier, echo.sequence, source_addr);
        }

        match handle_echo_request(&echo, dest_addr)? {
            EchoProcessResult::Reply(reply) => {
                if verbose {
                    println!("ICMP: 发送 Echo Reply ID={} Seq={}",
                        reply.identifier, reply.sequence);
                }
                Ok(IcmpProcessResult::Reply(reply.to_bytes()))
            }
            EchoProcessResult::NoReply => Ok(IcmpProcessResult::NoReply),
            _ => Ok(IcmpProcessResult::NoReply),
        }
    } else if echo.is_reply() {
        // 处理 Echo Reply
        if verbose {
            println!("ICMP: 收到 Echo Reply ID={} Seq={} from {}",
                echo.identifier, echo.sequence, source_addr);
        }

        match handle_echo_reply(&echo, &context.icmp_echo)? {
            EchoProcessResult::Matched { identifier, sequence, rtt_ms } => {
                if verbose {
                    println!("ICMP: Echo Reply 匹配成功 ID={} Seq={} RTT={}ms",
                        identifier, sequence, rtt_ms);
                }
                Ok(IcmpProcessResult::Processed)
            }
            EchoProcessResult::NoReply => Ok(IcmpProcessResult::NoReply),
            _ => Ok(IcmpProcessResult::NoReply),
        }
    } else {
        Ok(IcmpProcessResult::NoReply)
    }
}

/// 创建 Echo Request
///
/// # 参数
/// - identifier: 标识符
/// - sequence: 序列号
/// - data: 负载数据
///
/// # 返回
/// - Vec<u8>: 编码后的 ICMP 报文
pub fn create_echo_request(identifier: u16, sequence: u16, data: Vec<u8>) -> Vec<u8> {
    let echo = IcmpEcho::new_request(identifier, sequence, data);
    echo.to_bytes()
}

/// 创建 Echo Reply
///
/// # 参数
/// - identifier: 标识符
/// - sequence: 序列号
/// - data: 负载数据
///
/// # 返回
/// - Vec<u8>: 编码后的 ICMP 报文
pub fn create_echo_reply(identifier: u16, sequence: u16, data: Vec<u8>) -> Vec<u8> {
    let echo = IcmpEcho::new_reply(identifier, sequence, data);
    echo.to_bytes()
}

/// 创建 Destination Unreachable 报文
///
/// # 参数
/// - code: 错误代码（0=网络不可达, 1=主机不可达, 2=协议不可达, 3=端口不可达等）
/// - original_datagram: 触发错误的原始 IP 数据报（至少包含 IP 头部和前 8 字节）
///
/// # 返回
/// - Vec<u8>: 编码后的 ICMP Destination Unreachable 报文
pub fn create_dest_unreachable(code: u8, original_datagram: Vec<u8>) -> Vec<u8> {
    use super::packet::IcmpDestUnreachable;

    let dest_unreachable = IcmpDestUnreachable {
        type_: ICMP_TYPE_DEST_UNREACHABLE,
        code,
        checksum: 0,
        original_datagram,
    };

    dest_unreachable.to_bytes()
}

/// 创建 Time Exceeded 报文
///
/// # 参数
/// - code: 错误代码（0=TTL 超时, 1=分片重组超时）
/// - original_datagram: 触发错误的原始 IP 数据报（至少包含 IP 头部和前 8 字节）
///
/// # 返回
/// - Vec<u8>: 编码后的 ICMP Time Exceeded 报文
pub fn create_time_exceeded(code: u8, original_datagram: Vec<u8>) -> Vec<u8> {
    use super::packet::IcmpTimeExceeded;

    let time_exceeded = IcmpTimeExceeded {
        type_: ICMP_TYPE_TIME_EXCEEDED,
        code,
        checksum: 0,
        original_datagram,
    };

    time_exceeded.to_bytes()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::SystemContext;

    #[test]
    fn test_create_echo_request() {
        let data = vec![0x42; 32];
        let packet = create_echo_request(1234, 1, data.clone());

        assert_eq!(packet[0], ICMP_TYPE_ECHO_REQUEST);
        assert_eq!(packet[1], 0);
    }

    #[test]
    fn test_create_echo_reply() {
        let data = vec![0x42; 32];
        let packet = create_echo_reply(1234, 1, data);

        assert_eq!(packet[0], ICMP_TYPE_ECHO_REPLY);
        assert_eq!(packet[1], 0);
    }

    #[test]
    fn test_create_dest_unreachable() {
        let original = vec
![0x45u8, 0x00, 0x00, 0x1c, 0x00, 0x00, 0x00, 0x00];
        let packet = create_dest_unreachable(0, original);

        assert_eq!(packet[0], ICMP_TYPE_DEST_UNREACHABLE);
        assert_eq!(packet[1], 0);
    }

    #[test]
    fn test_process_echo_request() {
        let echo_bytes = create_echo_request(1234, 1, vec![0x42; 32]);
        let packet = Packet::from_bytes(echo_bytes);
        let source = Ipv4Addr::new(192, 168, 1, 1);
        let dest = Ipv4Addr::new(192, 168, 1, 100);
        let ctx = SystemContext::new();

        let result = process_icmp_packet(packet, source, dest, &ctx, false).unwrap();

        match result {
            IcmpProcessResult::Reply(reply_bytes) => {
                assert_eq!(reply_bytes[0], ICMP_TYPE_ECHO_REPLY);
            }
            _ => panic!("Expected Reply"),
        }
    }
}
