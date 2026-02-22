// src/protocols/ip/packet.rs
//
// IPv4 数据报处理逻辑

use crate::common::{CoreError, Packet, Result as CoreResult};
use crate::protocols::Ipv4Addr;
use crate::context::SystemContext;

use super::header::Ipv4Header;
use super::protocol::Ipv4Protocol;
use super::error::IpError;
use super::verify_checksum;

/// IP 处理结果
///
/// 表示 IP 数据报处理后的结果类型。
#[derive(Debug, Clone, PartialEq)]
pub enum IpProcessResult {
    /// 无需响应（数据报被静默丢弃）
    NoReply,

    /// 需要发送 ICMP 错误响应（Vec<u8> 为完整的 IP 数据报）
    Reply(Vec<u8>),

    /// 交付给上层协议（包含 IP 头部和负载数据）
    DeliverToProtocol {
        /// IP 头部
        ip_hdr: Ipv4Header,
        /// 上层协议数据（不含 IP 头部）
        data: Vec<u8>,
    },
}

/// IP 处理专用 Result 类型
pub type IpResult<T> = std::result::Result<T, IpError>;

/// 处理 IP 数据报
///
/// # 参数
/// - packet: 可变引用的 Packet（已去除以太网头部）
/// - ifindex: 接口索引
/// - context: 系统上下文，用于访问接口信息
///
/// # 返回
/// - Ok(IpProcessResult): 处理结果
/// - Err(IpError): 处理失败
///
/// # 处理流程
/// 1. 解析 IP 头部
/// 2. 验证校验和
/// 3. 检查分片标志（不支持分片，直接丢弃）
/// 4. 检查目的地址是否为本机地址
/// 5. 根据协议字段分发到上层协议
pub fn process_ip_packet(
    packet: &mut Packet,
    ifindex: u32,
    context: &SystemContext,
) -> IpResult<IpProcessResult> {
    // 1. 解析 IP 头部
    let ip_hdr = Ipv4Header::from_packet(packet)
        .map_err(|e| match e {
            CoreError::UnsupportedProtocol(msg) if msg.contains("版本") => {
                IpError::invalid_version(6) // 假设是 IPv6
            }
            _ => IpError::invalid_packet(e.to_string()),
        })?;

    // 2. 验证校验和
    verify_header_checksum(packet, ip_hdr.header_len())
        .map_err(|e| IpError::invalid_packet(e.to_string()))?;

    // 3. 检查是否为分片数据报（当前版本不支持分片和重组）
    if ip_hdr.is_fragmented() {
        return Err(IpError::fragmented_packet(
            ip_hdr.has_mf_flag(),
            ip_hdr.fragment_offset(),
        ));
    }

    // 4. 检查目的地址是否为本机地址
    let is_local = is_local_address(context, ip_hdr.dest_addr, ifindex)?;

    if !is_local {
        // 不是发送给本机的报文
        return Ok(IpProcessResult::NoReply);
    }

    // 5. 根据协议字段分发
    let protocol = Ipv4Protocol::from(ip_hdr.protocol);

    match protocol {
        Ipv4Protocol::Icmp => {
            // 提取数据部分（不含 IP 头部）
            let data = extract_payload(packet, ip_hdr.header_len())?;
            Ok(IpProcessResult::DeliverToProtocol { ip_hdr, data })
        }
        Ipv4Protocol::Udp => {
            // 提取数据部分（不含 IP 头部）
            let data = extract_payload(packet, ip_hdr.header_len())?;
            Ok(IpProcessResult::DeliverToProtocol { ip_hdr, data })
        }
        Ipv4Protocol::Tcp => {
            // 提取数据部分（不含 IP 头部）
            let data = extract_payload(packet, ip_hdr.header_len())?;
            Ok(IpProcessResult::DeliverToProtocol { ip_hdr, data })
        }
        _ => {
            // 协议不支持，需要返回 ICMP 协议不可达
            Err(IpError::unsupported_protocol(ip_hdr.protocol))
        }
    }
}

/// 封装 IP 数据报
///
/// # 参数
/// - source_addr: 源 IP 地址
/// - dest_addr: 目的 IP 地址
/// - protocol: 上层协议号
/// - payload: 上层协议数据
///
/// # 返回
/// - Vec<u8>: 完整的 IP 数据报（包含头部和数据）
pub fn encapsulate_ip_datagram(
    source_addr: Ipv4Addr,
    dest_addr: Ipv4Addr,
    protocol: u8,
    payload: &[u8],
) -> Vec<u8> {
    let header = Ipv4Header::new(source_addr, dest_addr, protocol, payload.len());
    let mut packet = header.to_bytes();
    packet.extend_from_slice(payload);
    packet
}

/// 验证 IP 头部校验和
fn verify_header_checksum(packet: &Packet, header_len: usize) -> CoreResult<()> {
    // 保存当前offset
    let original_offset = packet.offset;

    // IP数据报从当前offset减去header_len开始（因为from_packet已经读取过了）
    // 实际上，offset现在指向IP头部之后的位置
    // 所以IP头部在 [original_offset - header_len, original_offset)
    let ip_header_start = original_offset.saturating_sub(header_len);

    // 从原始数据中获取IP头部
    let header_data = packet.data.get(ip_header_start..original_offset)
        .ok_or_else(|| CoreError::parse_error("IP头部数据不足"))?;

    // 验证校验和（校验和字段在偏移 10 处）
    if !verify_checksum(header_data, 10) {
        return Err(CoreError::invalid_packet("IP校验和错误"));
    }

    Ok(())
}

/// 检查目的地址是否为本机地址
fn is_local_address(context: &SystemContext, dest_addr: Ipv4Addr, ifindex: u32) -> IpResult<bool> {
    let interfaces = context.interfaces.lock()
        .map_err(|_| IpError::destination_unreachable(dest_addr))?;

    // 检查是否有接口配置了此地址
    let is_local = interfaces.get_by_index(ifindex)
        .map(|iface| iface.ip_addr == dest_addr)
        .unwrap_or(false);

    // 特殊地址检查
    if dest_addr.is_broadcast() || dest_addr.is_loopback() {
        return Ok(true); // 广播和回环地址也需要处理
    }

    Ok(is_local)
}

/// 提取 IP 负载数据
fn extract_payload(packet: &Packet, _header_len: usize) -> IpResult<Vec<u8>> {
    // 此时 packet.offset 已经在 IP 头部之后
    // packet.remaining() 就是负载数据的长度
    let payload_len = packet.remaining();

    if payload_len == 0 {
        return Ok(Vec::new());
    }

    let payload_data = packet.peek(payload_len)
        .ok_or(IpError::PacketTooShort {
            expected: payload_len,
            found: 0,
        })?;

    Ok(payload_data.to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encapsulate_ip_datagram() {
        let src = Ipv4Addr::new(192, 168, 1, 1);
        let dst = Ipv4Addr::new(192, 168, 1, 2);
        let payload = vec![0x08, 0x00, 0x01, 0x02]; // ICMP 示例

        let packet = encapsulate_ip_datagram(src, dst, 1, &payload);

        // 验证包头
        assert_eq!(packet[0], 0x45); // Version=4, IHL=5
        assert_eq!(packet[9], 1); // Protocol=ICMP

        // 验证地址
        assert_eq!(&packet[12..16], &[192, 168, 1, 1]);
        assert_eq!(&packet[16..20], &[192, 168, 1, 2]);

        // 验证负载
        assert_eq!(&packet[20..], &payload[..]);
    }

    #[test]
    fn test_ipv4_protocol_from_u8() {
        assert_eq!(Ipv4Protocol::from(1), Ipv4Protocol::Icmp);
        assert_eq!(Ipv4Protocol::from(6), Ipv4Protocol::Tcp);
        assert_eq!(Ipv4Protocol::from(17), Ipv4Protocol::Udp);
        assert_eq!(Ipv4Protocol::from(255), Ipv4Protocol::Unknown(255));
    }

    #[test]
    fn test_ipv4_protocol_is_supported() {
        assert!(Ipv4Protocol::Icmp.is_supported());
        assert!(Ipv4Protocol::Tcp.is_supported());
        assert!(Ipv4Protocol::Udp.is_supported());
    }
}
