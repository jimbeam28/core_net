// src/protocols/icmp/process.rs
//
// ICMP 报文处理逻辑

use crate::common::{CoreError, Packet, Result};
use crate::protocols::{Ipv4Addr, Ipv6Addr};
use crate::protocols::ip::{verify_checksum, verify_icmpv6_checksum};
use crate::context::SystemContext;

use super::packet::{IcmpPacket, IcmpEcho, IcmpV6Packet, IcmpV6Echo, is_broadcast_addr, is_multicast_addr};
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

    // 验证校验和 - 校验和错误时静默丢弃，不返回错误
    // RFC 792: 不应该对 ICMP 错误报文发送 ICMP 错误报文
    if !verify_checksum(data, 2) {
        if verbose {
            println!("ICMP: 校验和错误，静默丢弃");
        }
        return Ok(IcmpProcessResult::NoReply);
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
        IcmpPacket::DestUnreachable(dest_unreach) => {
            // Destination Unreachable 是错误消息，不需要响应
            if verbose {
                println!("ICMP: 收到 Destination Unreachable Code={}", dest_unreach.code);
            }
            Ok(IcmpProcessResult::Processed)
        }
        IcmpPacket::TimeExceeded(time_exceeded) => {
            // Time Exceeded 是错误消息，不需要响应
            if verbose {
                println!("ICMP: 收到 Time Exceeded Code={}", time_exceeded.code);
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

        // 检查目标地址是否为广播/多播地址
        // RFC 1122: 不应该响应广播地址的 Echo Request
        if is_broadcast_addr(dest_addr.as_bytes()) || is_multicast_addr(dest_addr.as_bytes()) {
            if verbose {
                println!("ICMP: 忽略发往广播/多播地址的 Echo Request");
            }
            return Ok(IcmpProcessResult::NoReply);
        }

        if verbose {
            println!("ICMP: 收到 Echo Request ID={} Seq={} from {} to {}",
                echo.identifier, echo.sequence, source_addr, dest_addr);
        }

        match handle_echo_request(&echo, dest_addr)? {
            EchoProcessResult::Reply(reply) => {
                // 检查速率限制
                let mut echo_manager = context.icmp_echo.lock()
                    .map_err(|e| CoreError::parse_error(format!("锁定Echo管理器失败: {}", e)))?;

                if !echo_manager.can_send_echo_reply() {
                    if verbose {
                        println!("ICMP: 超过速率限制，不发送 Echo Reply");
                    }
                    return Ok(IcmpProcessResult::NoReply);
                }

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

        match handle_echo_reply(&echo, source_addr, &context.icmp_echo)? {
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

// ========== ICMPv6 处理函数 ==========

/// 处理接收到的 ICMPv6 报文
///
/// # 参数
/// - packet: ICMPv6 报文（不包含 IPv6 头部）
/// - source_addr: 发送方 IPv6 地址
/// - dest_addr: 接收方 IPv6 地址（本接口 IPv6）
/// - _context: 系统上下文（包含 Echo 管理器）
/// - verbose: 是否打印详细信息
///
/// # 返回
/// - Ok(IcmpProcessResult): 处理结果
/// - Err(CoreError): 处理失败
///
/// # ICMPv6 校验和
/// ICMPv6 校验和计算需要包含 IPv6 伪头部（RFC 4443, RFC 8200）：
/// - 源 IPv6 地址（16 字节）
/// - 目的 IPv6 地址（16 字节）
/// - 上层包长度（4 字节）
/// - 下一头部值：58（ICMPv6）
pub fn process_icmpv6_packet(
    mut packet: Packet,
    source_addr: Ipv6Addr,
    dest_addr: Ipv6Addr,
    _context: &SystemContext,
    verbose: bool,
) -> Result<IcmpProcessResult> {
    // 读取数据用于校验和验证
    let data = packet.peek(packet.remaining()).unwrap_or(&[]);

    // 验证 ICMPv6 校验和（需要包含 IPv6 伪头部）
    if !verify_icmpv6_checksum(source_addr, dest_addr, data) {
        if verbose {
            println!("ICMPv6: 校验和错误，静默丢弃");
        }
        return Ok(IcmpProcessResult::NoReply);
    }

    // 解析 ICMPv6 报文
    let icmpv6_packet = IcmpV6Packet::from_packet(&mut packet)?;

    if verbose {
        println!("ICMPv6: Type={} Source={} Dest={}",
            icmpv6_packet.get_type(), source_addr, dest_addr);
    }

    // 根据类型处理
    match icmpv6_packet {
        IcmpV6Packet::Echo(echo) => {
            handle_icmpv6_echo_packet(echo, source_addr, dest_addr, verbose)
        }
    }
}

/// 处理 ICMPv6 Echo 报文
fn handle_icmpv6_echo_packet(
    echo: IcmpV6Echo,
    source_addr: Ipv6Addr,
    dest_addr: Ipv6Addr,
    verbose: bool,
) -> Result<IcmpProcessResult> {
    if echo.is_request() {
        // 处理 Echo Request
        if verbose {
            println!("ICMPv6: 收到 Echo Request ID={} Seq={} from {} to {}",
                echo.identifier, echo.sequence, source_addr, dest_addr);
        }

        // 创建 Echo Reply（交换源地址和目的地址）
        let reply = echo.make_reply();

        if verbose {
            println!("ICMPv6: 发送 Echo Reply ID={} Seq={}",
                reply.identifier, reply.sequence);
        }

        // 使用正确的 ICMPv6 校验和（包含伪头部）
        // 注意：响应报文的源地址是请求的目的地址，目的地址是请求的源地址
        Ok(IcmpProcessResult::Reply(
            reply.to_bytes_with_addrs(dest_addr, source_addr)
        ))
    } else if echo.is_reply() {
        // 处理 Echo Reply
        if verbose {
            println!("ICMPv6: 收到 Echo Reply ID={} Seq={} from {}",
                echo.identifier, echo.sequence, source_addr);
        }
        Ok(IcmpProcessResult::Processed)
    } else {
        Ok(IcmpProcessResult::NoReply)
    }
}

/// 创建 ICMPv6 Echo Request
///
/// # 参数
/// - identifier: 标识符
/// - sequence: 序列号
/// - data: 负载数据
/// - source_addr: 源 IPv6 地址
/// - dest_addr: 目的 IPv6 地址
///
/// # 返回
/// - Vec<u8>: 编码后的 ICMPv6 报文（包含正确的伪头部校验和）
pub fn create_icmpv6_echo_request(
    identifier: u16,
    sequence: u16,
    data: Vec<u8>,
    source_addr: Ipv6Addr,
    dest_addr: Ipv6Addr,
) -> Vec<u8> {
    let echo = IcmpV6Echo::new_request(identifier, sequence, data);
    echo.to_bytes_with_addrs(source_addr, dest_addr)
}

/// 创建 ICMPv6 Echo Reply
///
/// # 参数
/// - identifier: 标识符
/// - sequence: 序列号
/// - data: 负载数据
/// - source_addr: 源 IPv6 地址
/// - dest_addr: 目的 IPv6 地址
///
/// # 返回
/// - Vec<u8>: 编码后的 ICMPv6 报文（包含正确的伪头部校验和）
pub fn create_icmpv6_echo_reply(
    identifier: u16,
    sequence: u16,
    data: Vec<u8>,
    source_addr: Ipv6Addr,
    dest_addr: Ipv6Addr,
) -> Vec<u8> {
    let echo = IcmpV6Echo::new_reply(identifier, sequence, data);
    echo.to_bytes_with_addrs(source_addr, dest_addr)
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
        // Need at least IP header (20 bytes) + 8 bytes data
        let original = vec![
            0x45, 0x00, 0x00, 0x1c,  // Version/IHL, TOS, Total Length
            0x00, 0x00, 0x00, 0x00,  // ID, Flags/Fragment
            0x40, 0x01, 0x00, 0x00,  // TTL, Protocol, Checksum
            0xc0, 0xa8, 0x01, 0x01,  // Source IP
            0xc0, 0xa8, 0x01, 0x02,  // Dest IP
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,  // 8 bytes data
        ];
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
