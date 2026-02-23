// src/protocols/ipv6/packet.rs
//
// IPv6 数据包处理逻辑

use crate::common::{CoreError, Packet};
use crate::protocols::Ipv6Addr;
use crate::context::SystemContext;

use super::header::Ipv6Header;
use super::protocol::IpProtocol;
use super::error::Ipv6Error;
use super::config::Ipv6Config;
use super::extension::{parse_extension_chain, ExtensionConfig, ExtensionHeaderType};
use super::fragment::{ReassemblyKey, FragmentInfo, ReassemblyError};

/// IPv6 处理结果
///
/// 表示 IPv6 数据包处理后的结果类型。
#[derive(Debug, Clone, PartialEq)]
pub enum Ipv6ProcessResult {
    /// 无需响应（数据包被静默丢弃）
    NoReply,

    /// 需要发送 ICMPv6 错误响应（Vec<u8> 为完整的 IPv6 数据包）
    Reply(Vec<u8>),

    /// 交付给上层协议（包含 IPv6 头部信息和负载数据）
    DeliverToProtocol { header: Ipv6Header, data: Vec<u8> },

    /// 需要分片重组（包含分片信息）
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
/// 1. 解析 IPv6 基本头部
/// 2. 验证版本号
/// 3. 检查 Hop Limit
/// 4. 检查源地址是否为组播地址（违反规范）
/// 5. 处理扩展头链（如果启用）
/// 6. 检查目的地址是否为本机地址
/// 7. 根据 Next Header 字段分发到上层协议
pub fn process_ipv6_packet(
    packet: &mut Packet,
    ifindex: u32,
    context: &SystemContext,
) -> Ipv6Result<Ipv6ProcessResult> {
    // 1. 解析 IPv6 基本头部
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

    // 4. 检查目的地址是否为本机地址
    let is_local = is_local_address(context, ip_hdr.destination_addr, ifindex)?;

    if !is_local {
        // 不是发送给本机的报文（不支持转发）
        return Ok(Ipv6ProcessResult::NoReply);
    }

    // 5. 获取配置
    let config = get_config(context);
    let extension_config = config.extension_config();

    // 6. 处理扩展头链（如果启用）
    let final_next_header = if config.allow_extension_headers {
        process_extension_chain(packet, ip_hdr.next_header, &extension_config, context, &ip_hdr)?
    } else {
        // 扩展头未启用，检查 Next Header 是否为扩展头类型
        if ip_hdr.next_header.is_extension_header() {
            return Err(Ipv6Error::extension_header_not_supported(
                u8::from(ip_hdr.next_header)
            ));
        }
        ip_hdr.next_header
    };

    // 7. 根据 Next Header 分发
    match final_next_header {
        IpProtocol::IcmpV6 => {
            // 提取数据部分（不含 IPv6 头部）
            let data = extract_payload(packet, ip_hdr.payload_length as usize)?;
            Ok(Ipv6ProcessResult::DeliverToProtocol {
                header: ip_hdr,
                data,
            })
        }
        IpProtocol::Tcp => {
            // TODO: TCP 支持
            Err(Ipv6Error::unsupported_protocol(u8::from(final_next_header)))
        }
        IpProtocol::Udp => {
            // TODO: UDP 支持
            Err(Ipv6Error::unsupported_protocol(u8::from(final_next_header)))
        }
        _ => {
            // 协议不支持
            Err(Ipv6Error::unsupported_protocol(u8::from(final_next_header)))
        }
    }
}

/// 处理扩展头链
///
/// 解析扩展头链，返回最终的上层协议类型。
/// 如果遇到分片头，处理分片逻辑。
fn process_extension_chain(
    packet: &mut Packet,
    initial_next_header: IpProtocol,
    config: &ExtensionConfig,
    context: &SystemContext,
    ip_hdr: &Ipv6Header,
) -> Ipv6Result<IpProtocol> {

    // 尝试解析扩展头链
    match parse_extension_chain(packet, initial_next_header, config) {
        Ok(chain_result) => {
            // 检查是否有分片头
            let fragment_header = chain_result.headers.iter().find_map(|ext| {
                if let ExtensionHeaderType::Fragment { header } = ext {
                    Some(header)
                } else {
                    None
                }
            });

            if let Some(frag_hdr) = fragment_header {
                // 处理分片
                if !config.enable_fragmentation {
                    return Err(Ipv6Error::unsupported_protocol(44));
                }

                return process_fragment(packet, frag_hdr, ip_hdr, context);
            }

            Ok(chain_result.next_header)
        }
        Err(e) => {
            // 解析失败，转换为 Ipv6Error
            match e {
                CoreError::UnsupportedProtocol(msg) => {
                    if msg.contains("Type 0") {
                        Err(Ipv6Error::RoutingHeaderType0Deprecated)
                    } else {
                        Err(Ipv6Error::unsupported_protocol(0))
                    }
                }
                CoreError::InvalidPacket(msg) => {
                    if msg.contains("扩展头数量超过限制") {
                        Err(Ipv6Error::too_many_extension_headers(
                            0, // 无法提取具体数字
                            config.max_extension_headers,
                        ))
                    } else if msg.contains("扩展头链过长") {
                        Err(Ipv6Error::extension_chain_too_long(
                            0,
                            config.max_extension_headers_length,
                        ))
                    } else if msg.contains("原子分片") {
                        Err(Ipv6Error::atomic_fragment(0))
                    } else {
                        Err(Ipv6Error::invalid_header_length_simple())
                    }
                }
                _ => Err(Ipv6Error::invalid_header_length(0)),
            }
        }
    }
}

/// 处理分片数据包
///
/// 将分片添加到重组缓存，如果重组完成则返回重组后的数据。
fn process_fragment(
    packet: &mut Packet,
    frag_hdr: &super::extension::FragmentHeader,
    ip_hdr: &Ipv6Header,
    context: &SystemContext,
) -> Result<IpProtocol, Ipv6Error> {
    // 提取分片数据
    let remaining = packet.remaining();
    let fragment_data = packet.read(remaining)
        .ok_or_else(|| Ipv6Error::PacketTooShort {
            expected: remaining,
            found: 0,
        })?
        .to_vec();

    // 创建重组键
    let key = ReassemblyKey::new(
        ip_hdr.source_addr,
        ip_hdr.destination_addr,
        frag_hdr.identification,
    );

    // 创建分片信息
    let fragment = FragmentInfo::new(
        frag_hdr.fragment_offset(),
        frag_hdr.more_fragments(),
        fragment_data,
    );

    // 添加到分片缓存
    let mut cache = context.ipv6_fragment_cache.lock()
        .map_err(|_| Ipv6Error::reassembly_error())?;

    match cache.add_fragment(key, fragment) {
        Ok(Some(_reassembled_data)) => {
            // 重组完成，返回下一头部类型
            // 注意：重组后的数据会被缓存，上层协议需要处理
            Ok(IpProtocol::from(frag_hdr.next_header))
        }
        Ok(None) => {
            // 重组未完成，返回特殊标记
            Err(Ipv6Error::reassembly_incomplete())
        }
        Err(ReassemblyError::TooManyFragments { .. }) => {
            Err(Ipv6Error::reassembly_too_many_fragments())
        }
        Err(ReassemblyError::FragmentOverlap { .. }) => {
            Err(Ipv6Error::reassembly_fragment_overlap())
        }
        Err(ReassemblyError::Incomplete) => {
            Err(Ipv6Error::reassembly_incomplete())
        }
        Err(ReassemblyError::InvalidFragmentData) => {
            Err(Ipv6Error::invalid_header_length_simple())
        }
        Err(ReassemblyError::Timeout) => {
            Err(Ipv6Error::reassembly_timeout())
        }
        Err(ReassemblyError::InconsistentTotalLength { .. }) => {
            Err(Ipv6Error::invalid_header_length_simple())
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
        .map_err(|_| Ipv6Error::destination_unreachable(dest_addr.to_string()))?;

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

/// 获取 IPv6 配置
///
/// 从系统上下文中获取 IPv6 配置，如果没有则返回默认配置。
fn get_config(_context: &SystemContext) -> Ipv6Config {
    // TODO: 从 SystemContext 获取配置
    // 当前返回默认配置
    Ipv6Config::default()
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
        let src = Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 1);
        let dst = Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 2);
        let header = Ipv6Header::new(src, dst, 8, IpProtocol::IcmpV6, 64);
        let data = vec![0x01, 0x02, 0x03];
        let result = Ipv6ProcessResult::DeliverToProtocol {
            header,
            data: data.clone(),
        };
        match result {
            Ipv6ProcessResult::DeliverToProtocol { header: _, data: d } => assert_eq!(d, data),
            _ => panic!("Expected DeliverToProtocol"),
        }
    }

    #[test]
    fn test_ipv6_process_result_needs_reassembly() {
        let src = Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 1);
        let dst = Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 2);
        let result = Ipv6ProcessResult::NeedsReassembly {
            source_addr: src,
            dest_addr: dst,
            identification: 12345,
            fragment_data: vec![0x01, 0x02, 0x03],
            next_header: 58,
        };

        match result {
            Ipv6ProcessResult::NeedsReassembly {
                source_addr,
                dest_addr,
                identification,
                fragment_data,
                next_header,
            } => {
                assert_eq!(source_addr, src);
                assert_eq!(dest_addr, dst);
                assert_eq!(identification, 12345);
                assert_eq!(fragment_data, vec![0x01, 0x02, 0x03]);
                assert_eq!(next_header, 58);
            }
            _ => panic!("Expected NeedsReassembly"),
        }
    }
}
