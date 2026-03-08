// src/protocols/ipv6/packet.rs
//
// IPv6 数据包处理逻辑（精简版）

use crate::common::Packet;
use crate::protocols::Ipv6Addr;
use crate::context::SystemContext;

use super::header::Ipv6Header;
use super::protocol::IpProtocol;
use super::error::Ipv6Error;

/// IPv6 处理结果
#[derive(Debug, Clone, PartialEq)]
pub enum Ipv6ProcessResult {
    /// 无需响应
    NoReply,
    /// 需要发送响应
    Reply(Vec<u8>),
    /// 交付给上层协议
    DeliverToProtocol { header: Ipv6Header, data: Vec<u8> },
    /// 需要分片重组（暂不支持）
    NeedsReassembly {
        source_addr: Ipv6Addr,
        dest_addr: Ipv6Addr,
        identification: u32,
        fragment_data: Vec<u8>,
        next_header: u8,
    },
}

/// IPv6 处理专用 Result 类型
pub type Ipv6Result<T> = std::result::Result<T, Ipv6Error>;

/// 处理 IPv6 数据包（精简版）
///
/// 不处理扩展头链，直接根据 next_header 分发
pub fn process_ipv6_packet(
    packet: &mut Packet,
    ifindex: u32,
    context: &SystemContext,
) -> Ipv6Result<Ipv6ProcessResult> {
    // 1. 解析 IPv6 基本头部
    let ip_hdr = Ipv6Header::from_packet(packet)
        .map_err(|_| Ipv6Error::PacketTooShort {
            expected: 40,
            found: packet.remaining(),
        })?;

    // 2. 检查 Hop Limit
    if ip_hdr.hop_limit == 0 {
        return Err(Ipv6Error::hop_limit_exceeded(0));
    }

    // 3. 检查源地址是否为组播地址
    if ip_hdr.source_addr.is_multicast() {
        return Err(Ipv6Error::invalid_source_address(ip_hdr.source_addr.to_string()));
    }

    // 4. 检查目的地址是否为本机地址
    let is_local = is_local_address(context, ip_hdr.destination_addr, ifindex)?;

    if !is_local {
        return Ok(Ipv6ProcessResult::NoReply);
    }

    // 5. 简化处理：不遍历扩展头链，直接根据 next_header 分发
    match ip_hdr.next_header {
        IpProtocol::IcmpV6 => {
            let data = extract_payload(packet, ip_hdr.payload_length as usize)?;
            Ok(Ipv6ProcessResult::DeliverToProtocol {
                header: ip_hdr,
                data,
            })
        }
        IpProtocol::Ospf => {
            // OSPFv3 直接交付
            let data = extract_payload(packet, ip_hdr.payload_length as usize)?;
            Ok(Ipv6ProcessResult::DeliverToProtocol {
                header: ip_hdr,
                data,
            })
        }
        _ => {
            // 扩展头或其他协议，直接返回不支持
            Ok(Ipv6ProcessResult::NoReply)
        }
    }
}

/// 封装 IPv6 数据包
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
        .map_err(|_| Ipv6Error::destination_unreachable(dest_addr.to_string()))?;

    let is_local = interfaces.get_by_index(ifindex)
        .map(|iface| {
            if iface.ipv6_addr() == dest_addr {
                return true;
            }
            if dest_addr.is_loopback() || dest_addr == Ipv6Addr::LINK_LOCAL_ALL_NODES {
                return true;
            }
            false
        })
        .unwrap_or(false);

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
        let payload = vec![0x80, 0x00, 0x00, 0x00];

        let packet = encapsulate_ipv6_packet(src, dst, IpProtocol::IcmpV6, &payload, 64);

        assert_eq!(packet[0] >> 4, 6);
        assert_eq!(u16::from_be_bytes([packet[4], packet[5]]), 4);
        assert_eq!(packet[6], 58);
        assert_eq!(packet[7], 64);
    }
}
