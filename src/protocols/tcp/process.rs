// src/protocols/tcp/process.rs
//
// TCP 报文处理逻辑

use crate::common::Packet;
use crate::protocols::Ipv4Addr;
use crate::protocols::ip::{add_ipv4_pseudo_header, fold_carry};
use crate::context::SystemContext;
use std::sync::{Arc, Mutex};

use super::config::TcpConfig;
use super::tcb::{Tcb, TcpConnectionId, TcpState};
use super::segment::TcpSegment;
use super::header::TcpHeader;
use super::connection::{TcpConnectionManager, TcpOption};
use super::error::TcpError;

/// TCP 处理结果
#[derive(Debug, Clone, PartialEq)]
pub enum TcpProcessResult {
    /// 无需响应
    NoReply,

    /// 需要发送 TCP 响应
    Reply(Vec<u8>),

    /// 数据已交付给应用层（连接 ID, 数据，无需回复）
    Delivered(TcpConnectionId, Vec<u8>),

    /// 需要发送 TCP 响应，同时数据已交付给应用层（连接 ID, 响应, 数据）
    ReplyAndDelivered(TcpConnectionId, Vec<u8>, Vec<u8>),

    /// 连接已建立
    ConnectionEstablished(TcpConnectionId),

    /// 连接已关闭
    ConnectionClosed(TcpConnectionId),
}

/// 处理接收到的 TCP 报文
///
/// # 参数
/// - packet: TCP 报文（不包含 IP 头部）
/// - source_addr: 发送方 IP 地址
/// - dest_addr: 接收方 IP 地址（本接口 IP）
/// - context: 系统上下文
/// - config: TCP 配置
///
/// # 返回
/// - Ok(TcpProcessResult): 处理结果
/// - Err(TcpError): 处理失败
pub fn process_tcp_packet(
    packet: Packet,
    source_addr: Ipv4Addr,
    dest_addr: Ipv4Addr,
    context: &SystemContext,
    config: &TcpConfig,
) -> std::result::Result<TcpProcessResult, TcpError> {
    // 获取连接管理器的可变引用
    let mut conn_manager = context.tcp_connections.lock()
        .map_err(|e| TcpError::Other(format!("锁定 TCP 管理器失败: {}", e)))?;

    // 读取数据用于解析
    let data = packet.peek(packet.remaining())
        .ok_or_else(|| TcpError::ParseError("读取 TCP 报文失败".to_string()))?;

    // 解析 TCP 报文段
    let segment = TcpSegment::parse(data)?;

    // 验证校验和
    if !segment.verify_checksum(source_addr, dest_addr) {
        return Err(TcpError::ChecksumError);
    }

    let header = segment.header;

    // 查找现有连接
    let conn_id = TcpConnectionId::new(dest_addr, header.destination_port, source_addr, header.source_port);

    // 检查是否有监听端口
    if let Some(listen_tcb) = conn_manager.find_listen(header.destination_port) {
        return handle_syn_for_listen(listen_tcb, source_addr, header.source_port, &segment, config, &mut conn_manager);
    }

    // 查找现有连接
    let tcb = conn_manager.find(&conn_id);

    if let Some(tcb) = tcb {
        let mut tcb_guard = tcb.lock()
            .map_err(|e| TcpError::Other(format!("锁定 TCB 失败: {}", e)))?;
        return process_segment_with_tcb(&mut tcb_guard, &segment, source_addr, dest_addr, config);
    }

    // 连接不存在
    Err(TcpError::ConnectionNotExist)
}

/// 处理 SYN 报文（针对监听端口）
fn handle_syn_for_listen(
    listen_tcb: Arc<Mutex<Tcb>>,
    source_ip: Ipv4Addr,
    source_port: u16,
    segment: &TcpSegment,
    config: &TcpConfig,
    conn_manager: &mut TcpConnectionManager,
) -> std::result::Result<TcpProcessResult, TcpError> {
    let header = segment.header;

    if !header.is_syn() {
        return Err(TcpError::ConnectionNotExist);
    }

    let tcb_guard = listen_tcb.lock()
        .map_err(|e| TcpError::Other(format!("锁定 TCB 失败: {}", e)))?;

    let local_ip = tcb_guard.id.local_ip;
    let local_port = tcb_guard.id.local_port;
    drop(tcb_guard);

    // 创建新连接的 TCB 并保存到连接管理器
    // 使用符合 RFC 6528 的 ISN 生成算法
    let iss = Tcb::generate_isn(local_ip, local_port, source_ip, source_port);
    let irs = header.sequence_number;

    // 创建新连接 TCB
    let new_conn_id = TcpConnectionId::new(local_ip, local_port, source_ip, source_port);
    let mut tcb = Tcb::new(new_conn_id.clone());
    tcb.state = TcpState::SynReceived;
    tcb.init_recv_state(irs, config.default_window_size);
    tcb.init_send_state(iss);
    tcb.mss = config.max_segment_size;

    // 保存新连接
    conn_manager.add(tcb.clone())?;

    // 构建 SYN-ACK（携带 MSS 选项）
    let options = vec![
        TcpOption::MaxSegmentSize { mss: config.max_segment_size },
    ];
    let options_bytes = TcpOption::serialize_options(&options);

    let response_header = TcpHeader::syn_ack(
        local_port,
        source_port,
        iss,
        irs.wrapping_add(1),
        config.default_window_size,
    );

    let response_bytes = encapsulate_tcp_segment(&response_header, &options_bytes, local_ip, source_ip);

    Ok(TcpProcessResult::Reply(response_bytes))
}

/// 使用现有 TCB 处理报文段
fn process_segment_with_tcb(
    tcb: &mut Tcb,
    segment: &TcpSegment,
    _source_ip: Ipv4Addr,
    _dest_ip: Ipv4Addr,
    _config: &TcpConfig,
) -> std::result::Result<TcpProcessResult, TcpError> {
    let header = segment.header;

    // 检查 RST 标志
    if header.is_rst() {
        tcb.state = TcpState::Closed;
        return Ok(TcpProcessResult::ConnectionClosed(tcb.id.clone()));
    }

    match tcb.state {
        TcpState::SynReceived => {
            // 等待 ACK 完成三次握手
            if header.is_ack() && header.acknowledgment_number == tcb.iss.wrapping_add(1) {
                tcb.state = TcpState::Established;
                return Ok(TcpProcessResult::ConnectionEstablished(tcb.id.clone()));
            }
        }
        TcpState::Established => {
            // 处理 ACK（在数据处理之前，因为 ACK 可能更新拥塞窗口）
            if header.is_ack() {
                let old_una = tcb.snd_una;
                if header.acknowledgment_number > tcb.snd_una {
                    let acked = header.acknowledgment_number - old_una;

                    // 更新拥塞控制
                    if tcb.cwnd < tcb.ssthresh {
                        tcb.slow_start(tcb.mss as u32);
                    } else {
                        tcb.congestion_avoidance(tcb.mss as u32);
                    }

                    tcb.snd_una = header.acknowledgment_number;

                    // 从重传队列中移除已确认的段
                    tcb.remove_acked_from_retransmit_queue(header.acknowledgment_number);

                    // 重置重传计数
                    if acked > 0 {
                        tcb.reset_retransmit_count();
                    }
                }
            }

            // 处理 FIN（对方请求关闭）
            if header.is_fin() {
                tcb.rcv_nxt = header.sequence_number.wrapping_add(1);
                tcb.state = TcpState::CloseWait;

                // 发送 ACK
                let ack_header = TcpHeader::ack(
                    tcb.id.local_port,
                    tcb.id.remote_port,
                    tcb.snd_nxt,
                    tcb.rcv_nxt,
                    tcb.rcv_wnd,
                );
                let response = encapsulate_tcp_segment(&ack_header, &[], tcb.id.local_ip, tcb.id.remote_ip);
                return Ok(TcpProcessResult::Reply(response));
            }

            // 处理数据接收
            if !segment.payload.is_empty() {
                // 验证序列号
                if header.sequence_number == tcb.rcv_nxt {
                    // 更新接收序列号
                    tcb.rcv_nxt = tcb.rcv_nxt.wrapping_add(segment.payload.len() as u32);

                    // 将数据写入接收缓冲区
                    tcb.write_to_recv_buffer(segment.payload)?;

                    // 发送 ACK
                    let ack_header = TcpHeader::ack(
                        tcb.id.local_port,
                        tcb.id.remote_port,
                        tcb.snd_nxt,
                        tcb.rcv_nxt,
                        tcb.rcv_wnd,
                    );
                    let response = encapsulate_tcp_segment(&ack_header, &[], tcb.id.local_ip, tcb.id.remote_ip);

                    // 返回数据（Delivered）
                    let data = segment.payload.to_vec();
                    let conn_id = tcb.id.clone();
                    return Ok(TcpProcessResult::ReplyAndDelivered(conn_id, response, data));
                }
            }
        }
        TcpState::FinWait1 => {
            if header.is_ack() && header.acknowledgment_number == tcb.snd_nxt.wrapping_add(1) {
                tcb.state = TcpState::FinWait2;
            }
        }
        TcpState::CloseWait => {
            // 等待应用层关闭
        }
        TcpState::LastAck => {
            if header.is_ack() {
                tcb.state = TcpState::Closed;
                return Ok(TcpProcessResult::ConnectionClosed(tcb.id.clone()));
            }
        }
        _ => {}
    }

    Ok(TcpProcessResult::NoReply)
}

/// 封装 TCP 报文
///
/// # 参数
/// - header: TCP 头部
/// - options: TCP 选项（可选）
/// - source_addr: 源 IP 地址
/// - dest_addr: 目标 IP 地址
///
/// # 返回
/// - Vec<u8>: 完整的 TCP 报文（包含头部、选项和数据）
pub fn encapsulate_tcp_segment(
    header: &TcpHeader,
    options: &[u8],
    source_addr: Ipv4Addr,
    dest_addr: Ipv4Addr,
) -> Vec<u8> {
    let mut bytes = Vec::new();

    // 序列化头部
    let mut header_bytes = header.serialize();

    // 添加选项（如果有）
    if !options.is_empty() {
        // 更新数据偏移
        let data_offset = 5 + options.len().div_ceil(4);
        header_bytes[12] &= 0xF0;
        header_bytes[12] |= (data_offset as u8) & 0x0F;

        bytes.extend_from_slice(&header_bytes);
        bytes.extend_from_slice(options);

        // 填充到 4 字节边界
        while bytes.len() % 4 != 0 {
            bytes.push(0);
        }
    } else {
        bytes.extend_from_slice(&header_bytes);
    }

    // 计算校验和
    let checksum = calculate_tcp_checksum(&bytes, source_addr, dest_addr);
    bytes[16] = (checksum >> 8) as u8;
    bytes[17] = (checksum & 0xFF) as u8;

    bytes
}

/// 计算 TCP 校验和（包含伪头部）
fn calculate_tcp_checksum(data: &[u8], source_ip: Ipv4Addr, dest_ip: Ipv4Addr) -> u16 {
    let mut sum = 0u32;

    // 伪头部
    add_ipv4_pseudo_header(&mut sum, source_ip, dest_ip);
    sum += u32::from(6u16) << 8; // Protocol

    let tcp_len = data.len() as u16;
    sum += u32::from(tcp_len >> 8) << 8;
    sum += u32::from(tcp_len & 0xFF) << 8;

    // TCP 数据
    let mut i = 0;
    while i + 1 < data.len() {
        let word = u16::from_be_bytes([data[i], data[i + 1]]);
        sum += u32::from(word);
        i += 2;
    }
    if i < data.len() {
        sum += u32::from(data[i]) << 8;
    }

    // 处理进位
    !fold_carry(sum)
}

/// 创建 SYN 报文
///
/// 用于 TCP 连接建立的三次握手，发送 SYN 报文。
///
/// # 参数
/// - source_port: 源端口
/// - destination_port: 目的端口
/// - source_addr: 源 IP 地址
/// - dest_addr: 目的 IP 地址
/// - seq: 初始序列号
/// - window_size: 窗口大小
///
/// # 返回
/// - Vec<u8>: 编码后的 TCP SYN 报文
pub fn create_syn(
    source_port: u16,
    destination_port: u16,
    source_addr: Ipv4Addr,
    dest_addr: Ipv4Addr,
    seq: u32,
    window_size: u16,
) -> Vec<u8> {
    let header = TcpHeader::syn(source_port, destination_port, seq, window_size);
    encapsulate_tcp_segment(&header, &[], source_addr, dest_addr)
}

/// 创建 ACK 报文
///
/// 用于 TCP 连接中确认已接收数据。
///
/// # 参数
/// - source_port: 源端口
/// - destination_port: 目的端口
/// - source_addr: 源 IP 地址
/// - dest_addr: 目的 IP 地址
/// - seq: 发送序列号
/// - ack: 确认号
/// - window_size: 窗口大小
///
/// # 返回
/// - Vec<u8>: 编码后的 TCP ACK 报文
pub fn create_ack(
    source_port: u16,
    destination_port: u16,
    source_addr: Ipv4Addr,
    dest_addr: Ipv4Addr,
    seq: u32,
    ack: u32,
    window_size: u16,
) -> Vec<u8> {
    let header = TcpHeader::ack(source_port, destination_port, seq, ack, window_size);
    encapsulate_tcp_segment(&header, &[], source_addr, dest_addr)
}

/// 创建 FIN 报文
///
/// 用于 TCP 连接关闭，发送方不再发送数据。
///
/// # 参数
/// - source_port: 源端口
/// - destination_port: 目的端口
/// - source_addr: 源 IP 地址
/// - dest_addr: 目的 IP 地址
/// - seq: 发送序列号
/// - ack: 确认号
/// - window_size: 窗口大小
///
/// # 返回
/// - Vec<u8>: 编码后的 TCP FIN 报文
pub fn create_fin(
    source_port: u16,
    destination_port: u16,
    source_addr: Ipv4Addr,
    dest_addr: Ipv4Addr,
    seq: u32,
    ack: u32,
    window_size: u16,
) -> Vec<u8> {
    let header = TcpHeader::fin(source_port, destination_port, seq, ack, window_size);
    encapsulate_tcp_segment(&header, &[], source_addr, dest_addr)
}

/// 创建 RST 报文
///
/// 用于 TCP 连接复位，异常关闭连接。
///
/// # 参数
/// - source_port: 源端口
/// - destination_port: 目的端口
/// - source_addr: 源 IP 地址
/// - dest_addr: 目的 IP 地址
/// - seq: 发送序列号
/// - ack: 确认号
///
/// # 返回
/// - Vec<u8>: 编码后的 TCP RST 报文
pub fn create_rst(
    source_port: u16,
    destination_port: u16,
    source_addr: Ipv4Addr,
    dest_addr: Ipv4Addr,
    seq: u32,
    ack: u32,
) -> Vec<u8> {
    let header = TcpHeader::rst(source_port, destination_port, seq, ack);
    encapsulate_tcp_segment(&header, &[], source_addr, dest_addr)
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::flags;

    #[test]
    fn test_create_syn() {
        let src_ip = Ipv4Addr::new(192, 168, 1, 10);
        let dst_ip = Ipv4Addr::new(192, 168, 1, 100);

        let syn = create_syn(1234, 80, src_ip, dst_ip, 1000, 8192);

        assert_eq!(syn.len(), 20); // 基本头部
        // 源端口
        assert_eq!(syn[0], 0x04);
        assert_eq!(syn[1], 0xD2);
    }

    #[test]
    fn test_create_ack() {
        let src_ip = Ipv4Addr::new(192, 168, 1, 10);
        let dst_ip = Ipv4Addr::new(192, 168, 1, 100);

        let ack = create_ack(1234, 80, src_ip, dst_ip, 1001, 2000, 8192);

        assert_eq!(ack.len(), 20);
        // 应该有 ACK 标志
        assert!(ack[13] & flags::ACK != 0);
    }

    #[test]
    fn test_create_fin() {
        let src_ip = Ipv4Addr::new(192, 168, 1, 10);
        let dst_ip = Ipv4Addr::new(192, 168, 1, 100);

        let fin = create_fin(1234, 80, src_ip, dst_ip, 5000, 4000, 8192);

        assert_eq!(fin.len(), 20);
        // 应该有 FIN 和 ACK 标志
        assert!(fin[13] & flags::FIN != 0);
        assert!(fin[13] & flags::ACK != 0);
    }

    #[test]
    fn test_create_rst() {
        let src_ip = Ipv4Addr::new(192, 168, 1, 10);
        let dst_ip = Ipv4Addr::new(192, 168, 1, 100);

        let rst = create_rst(1234, 80, src_ip, dst_ip, 0, 0);

        assert_eq!(rst.len(), 20);
        // 应该有 RST 和 ACK 标志
        assert!(rst[13] & flags::RST != 0);
        assert!(rst[13] & flags::ACK != 0);
    }

    #[test]
    fn test_encapsulate_tcp_segment() {
        let header = TcpHeader::ack(1234, 5678, 1000, 500, 8192);
        let src_ip = Ipv4Addr::new(192, 168, 1, 1);
        let dst_ip = Ipv4Addr::new(192, 168, 1, 2);

        let bytes = encapsulate_tcp_segment(&header, &[], src_ip, dst_ip);

        assert_eq!(bytes.len(), 20);
        // 验证源端口
        assert_eq!(bytes[0..2], 1234u16.to_be_bytes());
        // 验证校验和已计算（不为 0）
        let checksum = u16::from_be_bytes([bytes[16], bytes[17]]);
        assert_ne!(checksum, 0);
    }

    #[test]
    fn test_process_result_no_reply() {
        let result = TcpProcessResult::NoReply;
        assert_eq!(result, TcpProcessResult::NoReply);
    }
}
