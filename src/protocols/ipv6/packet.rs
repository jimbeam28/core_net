// src/protocols/ipv6/packet.rs
//
// IPv6 数据包处理逻辑

use crate::common::{CoreError, Packet};
use crate::protocols::Ipv6Addr;
use crate::context::SystemContext;

use super::header::Ipv6Header;
use super::protocol::IpProtocol;
use super::error::Ipv6Error;

/// IPv6 处理结果
///
/// 表示 IPv6 数据包处理后的结果类型。
#[derive(Debug, Clone, PartialEq)]
pub enum Ipv6ProcessResult {
    /// 无需响应（数据包被静默丢弃）
    NoReply,

    /// 需要发送 ICMPv6 错误响应（Vec<u8> 为完整的 IPv6 数据包）
    Reply(Vec<u8>),

    /// 交付给上层协议（Vec<u8> 为上层协议数据，不含 IPv6 头部）
    DeliverToProtocol(Vec<u8>),
}

/// IPv6 处理专用 Result 类型
pub type Ipv6Result<T> = std::result::Result<T, Ipv6Error>;

/// 处理 IPv6 数据包
///
/// # 参数
/// - packet: 可变引用的 Packet（已去除以太网头部）
/// - ifindex: 接口索引
/// - context: 系统上下文，用于访问接口信息
///
/// # 返回
/// - Ok(Ipv6ProcessResult): 处理结果
/// - Err(Ipv6Error): 处理失败
///
/// # 处理流程
/// 1. 解析 IPv6 头部
/// 2. 验证版本号
/// 3. 检查 Hop Limit
/// 4. 检查目的地址是否为本机地址
/// 5. 根据 Next Header 字段分发到上层协议
pub fn process_ipv6_packet(
    packet: &mut Packet,
    ifindex: u32,
    context: &SystemContext,
) -> Ipv6Result<Ipv6ProcessResult> {
    // 1. 解析 IPv6 头部
    let ip_hdr = Ipv6Header::from_packet(packet)
        .map_err(|e| match e {
            CoreError::UnsupportedProtocol(msg) if msg.contains("版本") => {
                Ipv6Error::invalid_version(4)
            }
            _ => Ipv6Error::PacketTooShort {
                expected: 40,
                found: packet.remaining(),
            },
        })?;

    // 2. 检查 Hop Limit
    if ip_hdr.hop_limit == 0 {
        return Err(Ipv6Error::hop_limit_exceeded(0));
    }

    // 3. 检查源地址是否为组播地址（违反规范）
    if ip_hdr.source_addr.is_multicast() {
        return Err(Ipv6Error::invalid_source_address(ip_hdr.source_addr.to_string()));
    }

    // 4. 检查扩展头（当前版本不支持）
    if ip_hdr.next_header.is_extension_header() {
        return Err(Ipv6Error::extension_header_not_supported(
            u8::from(ip_hdr.next_header)
        ));
    }

    // 5. 检查目的地址是否为本机地址
    let is_local = is_local_address(context, ip_hdr.destination_addr, ifindex)?;

    if !is_local {
        // 不是发送给本机的报文（不支持转发）
        return Ok(Ipv6ProcessResult::NoReply);
    }

    // 6. 根据 Next Header 分发
    match ip_hdr.next_header {
        IpProtocol::IcmpV6 => {
            // 提取数据部分（不含 IPv6 头部）
            let data = extract_payload(packet, ip_hdr.payload_length as usize)?;
            Ok(Ipv6ProcessResult::DeliverToProtocol(data))
        }
        _ => {
            // 协议不支持
            Err(Ipv6Error::unsupported_protocol(u8::from(ip_hdr.next_header)))
        }
    }
}

/// 封装 IPv6 数据包
///
/// # 参数
/// - source_addr: 源 IPv6 地址
/// - dest_addr: 目的 IPv6 地址
/// - next_header: 上层协议号
/// - payload: 上层协议数据
/// - hop_limit: 跳数限制（默认 64）
///
/// # 返回
/// - Vec<u8>: 完整的 IPv6 数据包（包含头部和数据）
pub fn encapsulate_ipv6_packet(
    source_addr: Ipv6Addr,
    dest_addr: Ipv6Addr,
    next_header: IpProtocol,
    payload: &[u8],
    hop_limit: u8,
) -> Vec<u8> {
    let header = Ipv6Header::new(
        source_addr,
        dest_addr,
        payload.len() as u16,
        next_header,
        hop_limit,
    );
    let mut packet = header.to_bytes().to_vec();
    packet.extend_from_slice(payload);
    packet
}

/// 检查目的地址是否为本机地址
fn is_local_address(
    context: &SystemContext,
    dest_addr: Ipv6Addr,
    ifindex: u32,
) -> Ipv6Result<bool> {
    let interfaces = context.interfaces.lock()
        .map_err(|_| Ipv6Error::DestinationUnreachable {
            addr: dest_addr.to_string(),
        })?;

    // 检查是否有接口配置了此地址
    let is_local = interfaces.get_by_index(ifindex)
        .map(|iface| {
            // 检查地址是否匹配接口配置的 IPv6 地址
            if iface.ipv6_addr() == dest_addr {
                return true;
            }

            // 特殊地址检查
            if dest_addr.is_loopback() || dest_addr == Ipv6Addr::LINK_LOCAL_ALL_NODES {
                return true;
            }

            false
        })
        .unwrap_or(false);

    // 组播地址也需要处理
    if dest_addr.is_multicast() {
        return Ok(true);
    }

    Ok(is_local)
}

/// 提取 IPv6 负载数据
fn extract_payload(packet: &Packet, payload_length: usize) -> Ipv6Result<Vec<u8>> {
    let remaining = packet.remaining();
    if remaining < payload_length {
        return Err(Ipv6Error::PacketTooShort {
            expected: payload_length,
            found: remaining,
        });
    }

    let payload_data = packet.peek(payload_length)
        .ok_or(Ipv6Error::PacketTooShort {
            expected: payload_length,
            found: 0,
        })?;

    Ok(payload_data.to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encapsulate_ipv6_packet() {
        let src = Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 1);
        let dst = Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 2);
        let payload = vec![0x80, 0x00, 0x00, 0x00]; // ICMPv6 示例

        let packet = encapsulate_ipv6_packet(src, dst, IpProtocol::IcmpV6, &payload, 64);

        // 验证包头
        assert_eq!(packet[0] >> 4, 6); // Version=6
        assert_eq!(packet[6], 58); // Next Header=ICMPv6

        // 验证地址
        assert_eq!(&packet[8..24], &src.bytes[..]);
        assert_eq!(&packet[24..40], &dst.bytes[..]);

        // 验证负载
        assert_eq!(&packet[40..], &payload[..]);
    }

    #[test]
    fn test_ipv6_process_result_no_reply() {
        let result = Ipv6ProcessResult::NoReply;
        assert_eq!(result, Ipv6ProcessResult::NoReply);
    }

    #[test]
    fn test_ipv6_process_result_deliver() {
        let data = vec![0x01, 0x02, 0x03];
        let result = Ipv6ProcessResult::DeliverToProtocol(data.clone());
        match result {
            Ipv6ProcessResult::DeliverToProtocol(d) => assert_eq!(d, data),
            _ => panic!("Expected DeliverToProtocol"),
        }
    }
}
