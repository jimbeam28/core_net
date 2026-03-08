// src/protocols/icmpv6/process.rs
//
// ICMPv6 报文处理逻辑（精简版）

use crate::common::Packet;
use crate::protocols::Ipv6Addr;

use super::packet::*;
use super::neighbor::*;
use super::*;
use super::checksum::verify_icmpv6_checksum;

/// ICMPv6 处理结果
#[derive(Debug, Clone, PartialEq)]
pub enum Icmpv6ProcessResult {
    /// 无需响应
    NoReply,
    /// 需要发送 ICMPv6 响应报文
    Reply(Vec<u8>),
    /// 处理完成
    Processed,
}

/// ICMPv6 处理上下文（精简版）
pub struct Icmpv6Context {
    /// 邻居缓存（简化版）
    pub neighbor_cache: NeighborCache,
    /// 配置
    pub config: Icmpv6Config,
}

impl Icmpv6Context {
    pub fn new(config: Icmpv6Config) -> Self {
        Icmpv6Context {
            neighbor_cache: NeighborCache::new(config.max_neighbor_cache_entries),
            config,
        }
    }
}

impl Default for Icmpv6Context {
    fn default() -> Self {
        Self::new(ICMPV6_CONFIG_DEFAULT)
    }
}

/// 处理接收到的 ICMPv6 报文（精简版）
pub fn process_icmpv6_packet(
    mut packet: Packet,
    source_addr: Ipv6Addr,
    dest_addr: Ipv6Addr,
    hop_limit: u8,
    our_mac: Option<crate::protocols::MacAddr>,
    context: &mut Icmpv6Context,
    verbose: bool,
) -> Icmpv6Result<Icmpv6ProcessResult> {
    // 读取数据用于校验和验证
    let data = packet.peek(packet.remaining()).unwrap_or(&[]);

    // 验证 ICMPv6 校验和
    if !verify_icmpv6_checksum(source_addr, dest_addr, data, 2) {
        if verbose {
            println!("ICMPv6: 校验和错误，静默丢弃");
        }
        return Ok(Icmpv6ProcessResult::NoReply);
    }

    // 解析 ICMPv6 报文
    let icmpv6_packet = Icmpv6Packet::from_packet(&mut packet)?;

    if verbose {
        println!("ICMPv6: Type={} Source={} Dest={} HopLimit={}",
            icmpv6_packet.get_type(), source_addr, dest_addr, hop_limit);
    }

    // 根据类型处理
    match icmpv6_packet {
        Icmpv6Packet::Echo(echo) => {
            handle_icmpv6_echo_packet(echo, source_addr, dest_addr, context, verbose)
        }
        Icmpv6Packet::NeighborSolicitation(ns) => {
            handle_neighbor_solicitation(ns, source_addr, dest_addr, hop_limit, our_mac, context, verbose)
        }
        Icmpv6Packet::NeighborAdvertisement(na) => {
            handle_neighbor_advertisement(na, source_addr, hop_limit, context, verbose)
        }
        _ => {
            // 其他类型简化处理
            if verbose {
                println!("ICMPv6: 收到 {:?} 报文", icmpv6_packet.get_type());
            }
            Ok(Icmpv6ProcessResult::Processed)
        }
    }
}

/// 处理 ICMPv6 Echo 报文
fn handle_icmpv6_echo_packet(
    echo: Icmpv6Echo,
    source_addr: Ipv6Addr,
    dest_addr: Ipv6Addr,
    _context: &mut Icmpv6Context,
    verbose: bool,
) -> Icmpv6Result<Icmpv6ProcessResult> {
    if echo.is_request() {
        if verbose {
            println!("ICMPv6: 收到 Echo Request ID={} Seq={} from {} to {}",
                echo.identifier, echo.sequence, source_addr, dest_addr);
        }

        // 创建 Echo Reply
        let reply = echo.make_reply();
        let mut bytes = reply.to_bytes_without_checksum();

        // 计算并设置 ICMPv6 校验和
        let checksum = super::checksum::calculate_icmpv6_checksum(
            dest_addr,
            source_addr,
            &bytes
        );
        bytes[2] = (checksum >> 8) as u8;
        bytes[3] = (checksum & 0xFF) as u8;

        if verbose {
            println!("ICMPv6: 发送 Echo Reply ID={} Seq={}",
                reply.identifier, reply.sequence);
        }
        Ok(Icmpv6ProcessResult::Reply(bytes))
    } else {
        if verbose {
            println!("ICMPv6: 收到 Echo Reply ID={} Seq={}",
                echo.identifier, echo.sequence);
        }
        Ok(Icmpv6ProcessResult::Processed)
    }
}

/// 处理 Neighbor Solicitation（简化版）
fn handle_neighbor_solicitation(
    _ns: Icmpv6NeighborSolicitation,
    source_addr: Ipv6Addr,
    dest_addr: Ipv6Addr,
    hop_limit: u8,
    our_mac: Option<crate::protocols::MacAddr>,
    context: &mut Icmpv6Context,
    verbose: bool,
) -> Icmpv6Result<Icmpv6ProcessResult> {
    // 验证 Hop Limit
    if context.config.verify_hop_limit && hop_limit != 255 {
        if verbose {
            println!("ICMPv6: 丢弃 Neighbor Solicitation（Hop Limit={} != 255）", hop_limit);
        }
        return Ok(Icmpv6ProcessResult::NoReply);
    }

    // 简化处理：只记录日志
    if verbose {
        println!("ICMPv6: 收到 Neighbor Solicitation from {} to {}",
            source_addr, dest_addr);
    }

    // 如果有 MAC 地址，发送 Neighbor Advertisement 响应
    if let Some(mac) = our_mac {
        // 简化：只更新邻居缓存
        context.neighbor_cache.update(source_addr, mac);

        // 构造 Neighbor Advertisement 响应
        let mut na = Icmpv6NeighborAdvertisement::new(
            dest_addr,
            false, // Not a router
            true,  // Solicited
            false, // No override
        );

        // 添加目标链路层地址选项
        na.options.push(Icmpv6Option::target_link_layer_addr(&mac.bytes));

        let bytes = na.to_bytes();

        if verbose {
            println!("ICMPv6: 发送 Neighbor Advertisement to {}", source_addr);
        }
        return Ok(Icmpv6ProcessResult::Reply(bytes));
    }

    Ok(Icmpv6ProcessResult::NoReply)
}

/// 处理 Neighbor Advertisement（简化版）
fn handle_neighbor_advertisement(
    na: Icmpv6NeighborAdvertisement,
    source_addr: Ipv6Addr,
    hop_limit: u8,
    context: &mut Icmpv6Context,
    verbose: bool,
) -> Icmpv6Result<Icmpv6ProcessResult> {
    // 验证 Hop Limit
    if context.config.verify_hop_limit && hop_limit != 255 {
        if verbose {
            println!("ICMPv6: 丢弃 Neighbor Advertisement（Hop Limit={} != 255）", hop_limit);
        }
        return Ok(Icmpv6ProcessResult::NoReply);
    }

    // 简化处理：只更新邻居缓存
    // 从选项中提取 MAC 地址 (Type 2 = Target Link-Layer Address)
    for option in &na.options {
        if option.option_type == 2 && option.data.len() >= 6 {
            let mac_bytes = [option.data[0], option.data[1], option.data[2],
                             option.data[3], option.data[4], option.data[5]];
            context.neighbor_cache.update(source_addr, crate::protocols::MacAddr::new(mac_bytes));
            break;
        }
    }

    if verbose {
        println!("ICMPv6: 收到 Neighbor Advertisement from {}", source_addr);
    }

    Ok(Icmpv6ProcessResult::Processed)
}
