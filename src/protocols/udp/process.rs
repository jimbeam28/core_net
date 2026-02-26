// src/protocols/udp/process.rs
//
// UDP 数据报处理逻辑

use crate::common::{CoreError, Packet, Result};
use crate::protocols::Ipv4Addr;
use crate::context::SystemContext;
use crate::protocols::ipsec::ikev2::{IKEV2_PORT, IKEV2_NAT_PORT, IkeMessage, IkeProcessor, IkeSaConfig, IkeRole, IkeAuthMethod, IkeDhGroup};
use crate::common::addr::IpAddr;

use super::packet::UdpDatagram;
use super::config::UdpConfig;

#[cfg(test)]
use super::UdpSocket;

/// UDP 处理结果
///
/// 表示 UDP 数据报处理后的结果类型。
#[derive(Debug, Clone, PartialEq)]
pub enum UdpProcessResult {
    /// 无需响应（数据报被静默处理）
    NoReply,

    /// 需要发送 ICMP 端口不可达响应（完整 IP 数据报）
    /// 包含原始 IP 数据报，用于构造 ICMP Destination Unreachable 消息
    PortUnreachable(Vec<u8>),

    /// 数据已交付给应用层（本地端口, 源 IP, 源端口, 数据）
    Delivered(u16, Ipv4Addr, u16, Vec<u8>),
}

/// 处理接收到的 UDP 数据报
///
/// # 参数
/// - packet: UDP 数据报（不包含 IP 头部）
/// - source_addr: 发送方 IP 地址
/// - dest_addr: 接收方 IP 地址（本接口 IP）
/// - original_ip_datagram: 原始 IP 数据报（包含 IP 头部），用于构造 ICMP 响应
/// - context: 系统上下文
/// - config: UDP 配置
///
/// # 返回
/// - Ok(UdpProcessResult): 处理结果
/// - Err(CoreError): 处理失败
///
/// # 处理流程
/// 1. 解析 UDP 头部
/// 2. 验证数据报长度
/// 3. 验证校验和（如果配置要求）
/// 4. 查找端口入口
/// 5. 如果端口已绑定且有回调，调用回调
/// 6. 如果端口未绑定且配置要求，返回 PortUnreachable（包含原始 IP 数据报）
pub fn process_udp_packet(
    packet: Packet,
    source_addr: Ipv4Addr,
    dest_addr: Ipv4Addr,
    original_ip_datagram: &[u8],
    context: &SystemContext,
    config: &UdpConfig,
) -> Result<UdpProcessResult> {
    // 读取数据用于解析
    let data = packet.peek(packet.remaining())
        .ok_or_else(|| CoreError::ParseError("读取 UDP 数据报失败".into()))?;

    // 解析 UDP 数据报
    let datagram = UdpDatagram::parse(data)?;

    // 验证长度字段与实际数据是否一致
    if datagram.header.length as usize > data.len() {
        return Err(CoreError::ParseError(format!(
            "UDP 长度字段 {} 大于实际数据长度 {}",
            datagram.header.length,
            data.len()
        )));
    }

    // 验证校验和
    if !datagram.verify_checksum(source_addr, dest_addr, config.enforce_checksum) {
        return Err(CoreError::invalid_packet("UDP 校验和错误"));
    }

    let dest_port = datagram.header.destination_port;

    // 检查是否为 IKEv2 端口
    if dest_port == IKEV2_PORT || dest_port == IKEV2_NAT_PORT {
        return process_ikev2_packet(
            datagram,
            source_addr,
            dest_addr,
            context,
        );
    }

    // 查找目标端口
    let port_entry = {
        let port_manager = context.udp_ports.lock().unwrap();
        port_manager.lookup(dest_port).cloned()
    };

    // 复制数据载荷
    let payload = datagram.payload.to_vec();

    match port_entry {
        Some(entry) => {
            // 端口已绑定
            if entry.has_callback() {
                // 调用应用层回调
                entry.invoke_callback(source_addr, datagram.header.source_port, payload.clone());
                Ok(UdpProcessResult::Delivered(dest_port, source_addr, datagram.header.source_port, payload))
            } else {
                // 端口已绑定但没有回调（端口预留状态）
                Ok(UdpProcessResult::NoReply)
            }
        }
        None => {
            // 端口未绑定
            if config.send_icmp_unreachable {
                // 返回完整 IP 数据报用于 ICMP 响应
                // 根据 RFC 792，ICMP Destination Unreachable 需要包含原始 IP 数据报
                // 的 IP 头部加上前 8 字节数据
                Ok(UdpProcessResult::PortUnreachable(original_ip_datagram.to_vec()))
            } else {
                Ok(UdpProcessResult::NoReply)
            }
        }
    }
}

/// 封装 UDP 数据报
///
/// # 参数
/// - source_port: 源端口号
/// - destination_port: 目标端口号
/// - source_addr: 源 IP 地址
/// - dest_addr: 目标 IP 地址
/// - payload: 应用层数据
/// - calculate_checksum: 是否计算校验和
///
/// # 返回
/// - Vec<u8>: 完整的 UDP 数据报（包含头部和数据）
pub fn encapsulate_udp_datagram(
    source_port: u16,
    destination_port: u16,
    source_addr: Ipv4Addr,
    dest_addr: Ipv4Addr,
    payload: &[u8],
    calculate_checksum: bool,
) -> Vec<u8> {
    let datagram = UdpDatagram::create(source_port, destination_port, payload);
    datagram.to_bytes(source_addr, dest_addr, calculate_checksum)
}

/// 处理 IKEv2 数据包
///
/// IKEv2 使用 UDP 端口 500（标准）或 4500（NAT 穿透）
///
/// # 参数
/// - datagram: UDP 数据报
/// - source_addr: 源 IP 地址
/// - dest_addr: 目标 IP 地址
/// - context: 系统上下文
///
/// # 返回
/// - Ok(UdpProcessResult): 处理结果
/// - Err(CoreError): 处理失败
fn process_ikev2_packet(
    datagram: UdpDatagram,
    source_addr: Ipv4Addr,
    dest_addr: Ipv4Addr,
    context: &SystemContext,
) -> Result<UdpProcessResult> {
    use crate::common::addr::IpAddr;

    // 解析 IKE 消息
    let ike_bytes = datagram.payload;
    let ike_message = IkeMessage::from_bytes(ike_bytes)
        .map_err(|e| CoreError::ParseError(format!("IKEv2 解析失败: {}", e)))?;

    // 创建 IKE 处理器
    let config = IkeSaConfig::new(
        IkeRole::Responder,
        IpAddr::V4(dest_addr),
        IpAddr::V4(source_addr),
        IkeDhGroup::MODP2048,
        IkeAuthMethod::SHARED_KEY,
    );

    let processor = IkeProcessor::new(
        context.ike_manager.clone(),
        IpAddr::V4(dest_addr),
        config,
    );

    // 处理 IKE 消息
    let response = processor.process_message(&ike_message, IpAddr::V4(source_addr))
        .map_err(|e| CoreError::ParseError(format!("IKEv2 处理失败: {}", e)))?;

    // 如果有响应，需要发送（在实际实现中会通过某种机制通知发送层）
    if response.is_some() {
        // 简化实现：记录响应
        // 在真实实现中，需要将响应数据包放入发送队列
    }

    Ok(UdpProcessResult::NoReply)
}

/// 创建 ICMP 端口不可达消息
///
/// # 参数
/// - original_ip_datagram: 原始 IP 数据报（包含 IP 头部）
///
/// # 返回
/// - Vec<u8>: ICMP Destination Unreachable 消息（IP 层封装）
pub fn create_port_unreachable(original_ip_datagram: &[u8]) -> Vec<u8> {
    use crate::protocols::icmp;

    // ICMP 端口不可达：Type=3, Code=3
    icmp::create_dest_unreachable(3, original_ip_datagram.to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::SystemContext;

    #[test]
    fn test_process_udp_basic() {
        let src_ip = Ipv4Addr::new(192, 168, 1, 10);
        let dst_ip = Ipv4Addr::new(192, 168, 1, 100);

        // 创建 UDP 数据报
        let udp_bytes = encapsulate_udp_datagram(
            1234,
            5678,
            src_ip,
            dst_ip,
            b"Hello",
            false, // 不计算校验和
        );

        let packet = Packet::from_bytes(udp_bytes);
        let ctx = SystemContext::new();
        let config = UdpConfig::new().with_enforce_checksum(false); // 不强制验证校验和

        // 绑定端口并设置回调
        let mut socket = UdpSocket::new(ctx.clone());
        socket.bind(5678).unwrap();
        socket.set_callback(|_src_addr, _src_port, _data| {}).unwrap();

        let result = process_udp_packet(packet, src_ip, dst_ip, &[], &ctx, &config).unwrap();

        match result {
            UdpProcessResult::Delivered(_local_port, _src_addr, _src_port, data) => {
                assert_eq!(data, b"Hello");
            }
            _ => panic!("Expected Delivered result"),
        }
    }

    #[test]
    fn test_process_udp_checksum_validation() {
        let src_ip = Ipv4Addr::new(192, 168, 1, 10);
        let dst_ip = Ipv4Addr::new(192, 168, 1, 100);

        // 创建带校验和的 UDP 数据报
        let udp_bytes = encapsulate_udp_datagram(
            1234,
            5678,
            src_ip,
            dst_ip,
            b"Test",
            true,
        );

        let packet = Packet::from_bytes(udp_bytes);
        let ctx = SystemContext::new();
        let config = UdpConfig::new().with_enforce_checksum(true);

        let result = process_udp_packet(packet, src_ip, dst_ip, &[], &ctx, &config);
        assert!(result.is_ok());
    }

    #[test]
    fn test_process_udp_checksum_error() {
        let src_ip = Ipv4Addr::new(192, 168, 1, 10);
        let dst_ip = Ipv4Addr::new(192, 168, 1, 100);

        // 创建 UDP 数据报（不计算校验和）
        let mut udp_bytes = encapsulate_udp_datagram(
            1234,
            5678,
            src_ip,
            dst_ip,
            b"Test",
            false,
        );

        // 修改校验和为错误的值
        udp_bytes[6] = 0xFF;
        udp_bytes[7] = 0xFF;

        let packet = Packet::from_bytes(udp_bytes);
        let ctx = SystemContext::new();
        let config = UdpConfig::new().with_enforce_checksum(true);

        let result = process_udp_packet(packet, src_ip, dst_ip, &[], &ctx, &config);
        assert!(result.is_err());
    }

    #[test]
    fn test_encapsulate_udp_datagram() {
        let src_ip = Ipv4Addr::new(192, 168, 1, 1);
        let dst_ip = Ipv4Addr::new(192, 168, 1, 2);

        let bytes = encapsulate_udp_datagram(
            8080,
            53,
            src_ip,
            dst_ip,
            b"ABC",
            false,
        );

        assert_eq!(bytes.len(), 11); // 8 + 3
        assert_eq!(bytes[0..2], 8080u16.to_be_bytes());
        assert_eq!(bytes[2..4], 53u16.to_be_bytes());
        assert_eq!(bytes[4..6], 11u16.to_be_bytes());
        assert_eq!(&bytes[8..], b"ABC");
    }

    #[test]
    fn test_encapsulate_udp_with_checksum() {
        let src_ip = Ipv4Addr::new(192, 168, 1, 1);
        let dst_ip = Ipv4Addr::new(192, 168, 1, 2);

        let bytes = encapsulate_udp_datagram(
            1234,
            5678,
            src_ip,
            dst_ip,
            b"Test",
            true,
        );

        // 校验和不应为 0
        let checksum = u16::from_be_bytes([bytes[6], bytes[7]]);
        assert_ne!(checksum, 0);

        // 验证校验和
        let datagram = UdpDatagram::parse(&bytes).unwrap();
        assert!(datagram.verify_checksum(src_ip, dst_ip, true));
    }

    #[test]
    fn test_process_udp_minimal() {
        let src_ip = Ipv4Addr::new(192, 168, 1, 10);
        let dst_ip = Ipv4Addr::new(192, 168, 1, 100);

        // 最小 UDP 数据报（仅头部，无数据）
        let udp_bytes = encapsulate_udp_datagram(
            1234,
            5678,
            src_ip,
            dst_ip,
            b"",
            false,
        );

        let packet = Packet::from_bytes(udp_bytes);
        let ctx = SystemContext::new();
        let config = UdpConfig::new().with_enforce_checksum(false); // 不强制验证校验和

        // 绑定端口并设置回调
        let mut socket = UdpSocket::new(ctx.clone());
        socket.bind(5678).unwrap();
        socket.set_callback(|_src_addr, _src_port, _data| {}).unwrap();

        let result = process_udp_packet(packet, src_ip, dst_ip, &[], &ctx, &config).unwrap();

        match result {
            UdpProcessResult::Delivered(_local_port, _src_addr, _src_port, data) => {
                assert!(data.is_empty());
            }
            _ => panic!("Expected Delivered result"),
        }
    }

    #[test]
    fn test_process_udp_length_mismatch() {
        let src_ip = Ipv4Addr::new(192, 168, 1, 10);
        let dst_ip = Ipv4Addr::new(192, 168, 1, 100);

        // 创建一个长度字段不匹配的数据报
        let bytes = vec![
            0x04, 0xD2, // Source Port: 1234
            0x16, 0x2E, // Dest Port: 5678
            0x00, 0x14, // Length: 20
            0x00, 0x00, // Checksum
            // 只有 4 字节数据，但长度声明是 20
            0x01, 0x02, 0x03, 0x04,
        ];

        let packet = Packet::from_bytes(bytes);
        let ctx = SystemContext::new();
        let config = UdpConfig::new();

        let result = process_udp_packet(packet, src_ip, dst_ip, &[], &ctx, &config);
        assert!(result.is_err());
    }

    #[test]
    fn test_create_port_unreachable() {
        let original_datagram = vec![
            0x45, 0x00, 0x00, 0x1c, // IP header
            0x00, 0x00, 0x00, 0x00,
            0x40, 0x11, 0x00, 0x00,
            0xc0, 0xa8, 0x01, 0x0a,
            0xc0, 0xa8, 0x01, 0x64,
            // UDP header
            0x04, 0xD2, 0x16, 0x2E,
            0x00, 0x08, 0x00, 0x00,
        ];

        let icmp_msg = create_port_unreachable(&original_datagram);

        // ICMP Type should be 3 (Destination Unreachable)
        assert_eq!(icmp_msg[0], 3);
        // ICMP Code should be 3 (Port Unreachable)
        assert_eq!(icmp_msg[1], 3);
    }
}
