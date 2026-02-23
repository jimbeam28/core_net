// src/protocols/icmpv6/process.rs
//
// ICMPv6 报文处理逻辑
// RFC 4443: ICMPv6 处理规范
// RFC 4861: 邻居发现处理

use crate::common::{Packet, Result};
use crate::protocols::Ipv6Addr;
use crate::context::SystemContext;

use super::types::*;
use super::packet::*;
use super::neighbor::*;
use super::config::*;
use super::error::{Icmpv6Error, Icmpv6Result};
use super::checksum::verify_icmpv6_checksum;

/// ICMPv6 处理结果
#[derive(Debug, Clone, PartialEq)]
pub enum Icmpv6ProcessResult {
    /// 无需响应
    NoReply,

    /// 需要发送 ICMPv6 响应报文
    Reply(Vec<u8>),

    /// 处理完成（无需发送响应）
    Processed,
}

/// ICMPv6 处理上下文
pub struct Icmpv6Context {
    /// 邻居缓存
    pub neighbor_cache: NeighborCache,
    /// 路由器列表
    pub router_list: RouterList,
    /// 前缀列表
    pub prefix_list: PrefixList,
    /// PMTU 缓存
    pub pmtu_cache: PmtuCache,
    /// Echo 管理器
    pub echo_manager: EchoManager,
    /// 配置
    pub config: Icmpv6Config,
}

impl Icmpv6Context {
    pub fn new(config: Icmpv6Config) -> Self {
        Icmpv6Context {
            neighbor_cache: NeighborCache::new(
                config.max_neighbor_cache_entries,
                config.default_reachable_time,
            ),
            router_list: RouterList::new(),
            prefix_list: PrefixList::new(),
            pmtu_cache: PmtuCache::new(config.pmtu_cache_timeout),
            echo_manager: EchoManager::new(
                config.max_pending_echoes,
                config.echo_timeout.as_millis() as u32,
            ),
            config,
        }
    }

    /// 处理定时器超时
    pub fn handle_timeouts(&mut self) {
        self.neighbor_cache.handle_timeouts();
        self.router_list.remove_expired();
        self.prefix_list.remove_expired();
        self.pmtu_cache.remove_expired();
        self.echo_manager.cleanup_timeouts();
    }
}

impl Default for Icmpv6Context {
    fn default() -> Self {
        Self::new(ICMPV6_CONFIG_DEFAULT)
    }
}

/// 处理接收到的 ICMPv6 报文
///
/// # 参数
/// - packet: ICMPv6 报文（不包含 IPv6 头部）
/// - source_addr: 发送方 IPv6 地址
/// - dest_addr: 接收方 IPv6 地址（本接口 IPv6）
/// - context: ICMPv6 处理上下文
/// - verbose: 是否打印详细信息
///
/// # 返回
/// - Ok(Icmpv6ProcessResult): 处理结果
/// - Err(CoreError): 处理失败
pub fn process_icmpv6_packet(
    mut packet: Packet,
    source_addr: Ipv6Addr,
    dest_addr: Ipv6Addr,
    context: &mut Icmpv6Context,
    verbose: bool,
) -> Icmpv6Result<Icmpv6ProcessResult> {
    // 读取数据用于校验和验证
    let data = packet.peek(packet.remaining()).unwrap_or(&[]);

    // 验证 ICMPv6 校验和（包含 IPv6 伪头部）
    // 校验和错误时静默丢弃
    if !verify_icmpv6_checksum(source_addr, dest_addr, data, 2) {
        if verbose {
            println!("ICMPv6: 校验和错误，静默丢弃");
        }
        return Ok(Icmpv6ProcessResult::NoReply);
    }

    // 解析 ICMPv6 报文
    let icmpv6_packet = Icmpv6Packet::from_packet(&mut packet)?;

    if verbose {
        println!("ICMPv6: Type={} Source={} Dest={}",
            icmpv6_packet.get_type(), source_addr, dest_addr);
    }

    // 根据类型处理
    match icmpv6_packet {
        Icmpv6Packet::Echo(echo) => {
            handle_icmpv6_echo_packet(echo, source_addr, dest_addr, context, verbose)
        }
        Icmpv6Packet::DestUnreachable(dest_unreach) => {
            handle_dest_unreachable(dest_unreach, source_addr, context, verbose)
        }
        Icmpv6Packet::PacketTooBig(packet_too_big) => {
            handle_packet_too_big(packet_too_big, source_addr, context, verbose)
        }
        Icmpv6Packet::TimeExceeded(time_exceeded) => {
            handle_time_exceeded(time_exceeded, source_addr, context, verbose)
        }
        Icmpv6Packet::ParameterProblem(param_problem) => {
            handle_parameter_problem(param_problem, source_addr, context, verbose)
        }
        Icmpv6Packet::RouterSolicitation(rs) => {
            handle_router_solicitation(rs, source_addr, dest_addr, context, verbose)
        }
        Icmpv6Packet::RouterAdvertisement(ra) => {
            handle_router_advertisement(ra, source_addr, context, verbose)
        }
        Icmpv6Packet::NeighborSolicitation(ns) => {
            handle_neighbor_solicitation(ns, source_addr, dest_addr, context, verbose)
        }
        Icmpv6Packet::NeighborAdvertisement(na) => {
            handle_neighbor_advertisement(na, source_addr, context, verbose)
        }
        Icmpv6Packet::Redirect(redirect) => {
            handle_redirect(redirect, source_addr, context, verbose)
        }
    }
}

/// 处理 ICMPv6 Echo 报文
fn handle_icmpv6_echo_packet(
    echo: Icmpv6Echo,
    source_addr: Ipv6Addr,
    dest_addr: Ipv6Addr,
    context: &mut Icmpv6Context,
    verbose: bool,
) -> Icmpv6Result<Icmpv6ProcessResult> {
    if echo.is_request() {
        // 处理 Echo Request
        if !context.config.enable_echo_reply {
            return Ok(Icmpv6ProcessResult::NoReply);
        }

        if verbose {
            println!("ICMPv6: 收到 Echo Request ID={} Seq={} from {} to {}",
                echo.identifier, echo.sequence, source_addr, dest_addr);
        }

        // 创建 Echo Reply
        let reply = echo.make_reply();
        let mut bytes = reply.to_bytes_without_checksum();

        // 计算并设置 ICMPv6 校验和（源地址和目的地址互换）
        let checksum = super::checksum::calculate_icmpv6_checksum(
            dest_addr,  // 响应的源地址是本机地址
            source_addr, // 响应的目的地址是原始发送方
            &bytes
        );
        bytes[2] = (checksum >> 8) as u8;
        bytes[3] = (checksum & 0xFF) as u8;

        if verbose {
            println!("ICMPv6: 发送 Echo Reply ID={} Seq={}",
                reply.identifier, reply.sequence);
        }
        Ok(Icmpv6ProcessResult::Reply(bytes))
    } else if echo.is_reply() {
        // 处理 Echo Reply
        if verbose {
            println!("ICMPv6: 收到 Echo Reply ID={} Seq={} from {}",
                echo.identifier, echo.sequence, source_addr);
        }

        // 尝试匹配待处理的 Echo 请求
        if let Some(pending) = context.echo_manager.match_reply(echo.identifier, echo.sequence) {
            let rtt = pending.rtt_ms();
            if verbose {
                println!("ICMPv6: Echo Reply 匹配成功 ID={} Seq={} RTT={}ms",
                    echo.identifier, echo.sequence, rtt);
            }
        }

        Ok(Icmpv6ProcessResult::Processed)
    } else {
        Ok(Icmpv6ProcessResult::NoReply)
    }
}

/// 处理 Destination Unreachable
fn handle_dest_unreachable(
    _dest_unreach: Icmpv6DestUnreachable,
    _source_addr: Ipv6Addr,
    _context: &mut Icmpv6Context,
    verbose: bool,
) -> Icmpv6Result<Icmpv6ProcessResult> {
    if verbose {
        println!("ICMPv6: 收到 Destination Unreachable");
    }
    // Destination Unreachable 是错误消息，不需要响应
    Ok(Icmpv6ProcessResult::Processed)
}

/// 处理 Packet Too Big
fn handle_packet_too_big(
    packet_too_big: Icmpv6PacketTooBig,
    _source_addr: Ipv6Addr,
    context: &mut Icmpv6Context,
    verbose: bool,
) -> Icmpv6Result<Icmpv6ProcessResult> {
    if context.config.enable_pmtu_discovery {
        // 从原始数据报中提取目标地址
        // 这里简化处理，实际应该解析原始 IPv6 头
        if verbose {
            println!("ICMPv6: 收到 Packet Too Big MTU={}", packet_too_big.mtu);
        }
        // 更新 PMTU 缓存（需要从原始数据报中提取目标地址）
        // context.pmtu_cache.update(dest_addr, packet_too_big.mtu);
    }
    Ok(Icmpv6ProcessResult::Processed)
}

/// 处理 Time Exceeded
fn handle_time_exceeded(
    _time_exceeded: Icmpv6TimeExceeded,
    _source_addr: Ipv6Addr,
    _context: &mut Icmpv6Context,
    verbose: bool,
) -> Icmpv6Result<Icmpv6ProcessResult> {
    if verbose {
        println!("ICMPv6: 收到 Time Exceeded");
    }
    // Time Exceeded 是错误消息，不需要响应
    Ok(Icmpv6ProcessResult::Processed)
}

/// 处理 Parameter Problem
fn handle_parameter_problem(
    _param_problem: Icmpv6ParameterProblem,
    _source_addr: Ipv6Addr,
    _context: &mut Icmpv6Context,
    verbose: bool,
) -> Icmpv6Result<Icmpv6ProcessResult> {
    if verbose {
        println!("ICMPv6: 收到 Parameter Problem");
    }
    // Parameter Problem 是错误消息，不需要响应
    Ok(Icmpv6ProcessResult::Processed)
}

/// 处理 Router Solicitation
fn handle_router_solicitation(
    _rs: Icmpv6RouterSolicitation,
    _source_addr: Ipv6Addr,
    _dest_addr: Ipv6Addr,
    _context: &mut Icmpv6Context,
    verbose: bool,
) -> Icmpv6Result<Icmpv6ProcessResult> {
    if verbose {
        println!("ICMPv6: 收到 Router Solicitation");
    }
    // 当前版本：不发送 Router Advertisement
    Ok(Icmpv6ProcessResult::NoReply)
}

/// 处理 Router Advertisement
fn handle_router_advertisement(
    ra: Icmpv6RouterAdvertisement,
    source_addr: Ipv6Addr,
    context: &mut Icmpv6Context,
    verbose: bool,
) -> Icmpv6Result<Icmpv6ProcessResult> {
    if !context.config.accept_router_advertisements {
        if verbose {
            println!("ICMPv6: 忽略 Router Advertisement（配置禁用）");
        }
        return Ok(Icmpv6ProcessResult::NoReply);
    }

    if verbose {
        println!("ICMPv6: 收到 Router Advertisement from {}", source_addr);
        println!("  HopLimit={} ReachableTime={} RetransTimer={}",
            ra.cur_hop_limit, ra.reachable_time, ra.retrans_timer);
    }

    // 解析选项
    for opt in &ra.options {
        match opt.get_type() {
            Some(Icmpv6OptionType::SourceLinkLayerAddr) => {
                if opt.data.len() >= 6 {
                    let mut mac_bytes = [0u8; 6];
                    mac_bytes.copy_from_slice(&opt.data[..6]);
                    let mac = crate::protocols::MacAddr::new(mac_bytes);

                    // 添加路由器到列表
                    let router = DefaultRouterEntry::new(source_addr, mac, ra.lifetime);
                    context.router_list.add_or_update(router);

                    if verbose {
                        println!("  路由器: {} MAC={}", source_addr, mac);
                    }
                }
            }
            Some(Icmpv6OptionType::PrefixInfo) => {
                // 解析前缀信息选项（需要至少 30 字节）
                if opt.data.len() >= 30 {
                    let prefix_len = opt.data[2];
                    let mut prefix_bytes = [0u8; 16];
                    prefix_bytes.copy_from_slice(&opt.data[4..20]);
                    let prefix = Ipv6Addr::from_bytes(prefix_bytes);

                    let valid_lifetime = u32::from_be_bytes([
                        opt.data[4], opt.data[5], opt.data[6], opt.data[7]
                    ]);
                    let preferred_lifetime = u32::from_be_bytes([
                        opt.data[8], opt.data[9], opt.data[10], opt.data[11]
                    ]);

                    let prefix_entry = PrefixEntry::new(
                        prefix,
                        prefix_len,
                        valid_lifetime,
                        preferred_lifetime,
                    );
                    context.prefix_list.add_or_update(prefix_entry);

                    if verbose {
                        println!("  前缀: {}/{} Valid={} Preferred={}",
                            prefix, prefix_len, valid_lifetime, preferred_lifetime);
                    }
                }
            }
            Some(Icmpv6OptionType::Mtu) => {
                if opt.data.len() >= 4 {
                    let mtu = u32::from_be_bytes([
                        opt.data[0], opt.data[1], opt.data[2], opt.data[3]
                    ]);
                    if verbose {
                        println!("  MTU: {}", mtu);
                    }
                }
            }
            _ => {
                if context.config.drop_unknown_options {
                    if verbose {
                        println!("  未知选项类型: {}", opt.option_type);
                    }
                }
            }
        }
    }

    Ok(Icmpv6ProcessResult::Processed)
}

/// 处理 Neighbor Solicitation
fn handle_neighbor_solicitation(
    ns: Icmpv6NeighborSolicitation,
    source_addr: Ipv6Addr,
    _dest_addr: Ipv6Addr,
    context: &mut Icmpv6Context,
    verbose: bool,
) -> Icmpv6Result<Icmpv6ProcessResult> {
    if verbose {
        println!("ICMPv6: 收到 Neighbor Solicitation Target={}", ns.target_address);
    }

    // 检查目标地址是否为本机地址
    // 这里简化处理，实际需要查询接口地址列表
    // 如果目标地址是本机地址，发送 Neighbor Advertisement

    // 提取源链路层地址选项
    for opt in &ns.options {
        if let Some(Icmpv6OptionType::SourceLinkLayerAddr) = opt.get_type() {
            if opt.data.len() >= 6 {
                let mut mac_bytes = [0u8; 6];
                mac_bytes.copy_from_slice(&opt.data[..6]);
                let mac = crate::protocols::MacAddr::new(mac_bytes);

                // 更新邻居缓存
                context.neighbor_cache.update(
                    source_addr,
                    mac,
                    false,
                    NeighborCacheState::Stale,
                )?;

                if verbose {
                    println!("  更新邻居缓存: {} -> {}", source_addr, mac);
                }
            }
            break;
        }
    }

    // 当前版本：不自动发送 Neighbor Advertisement
    Ok(Icmpv6ProcessResult::NoReply)
}

/// 处理 Neighbor Advertisement
fn handle_neighbor_advertisement(
    na: Icmpv6NeighborAdvertisement,
    source_addr: Ipv6Addr,
    context: &mut Icmpv6Context,
    verbose: bool,
) -> Icmpv6Result<Icmpv6ProcessResult> {
    if verbose {
        println!("ICMPv6: 收到 Neighbor Advertisement Target={}", na.target_address);
        println!("  Router={} Solicited={} Override={}",
            na.router_flag(), na.solicited_flag(), na.override_flag());
    }

    // 提取目标链路层地址选项
    let mut link_layer_addr = None;
    for opt in &na.options {
        if let Some(Icmpv6OptionType::TargetLinkLayerAddr) = opt.get_type() {
            if opt.data.len() >= 6 {
                let mut mac_bytes = [0u8; 6];
                mac_bytes.copy_from_slice(&opt.data[..6]);
                link_layer_addr = Some(crate::protocols::MacAddr::new(mac_bytes));
            }
            break;
        }
    }

    // 更新邻居缓存
    let is_router = na.router_flag();
    let state = if na.solicited_flag() {
        NeighborCacheState::Reachable
    } else {
        NeighborCacheState::Stale
    };

    if let Some(mac) = link_layer_addr {
        context.neighbor_cache.update(
            na.target_address,
            mac,
            is_router,
            state,
        )?;

        if verbose {
            println!("  更新邻居缓存: {} -> {} State={}",
                na.target_address, mac, state);
        }
    } else if na.override_flag() {
        // Override 标志设置但没有链路层地址，删除现有条目
        context.neighbor_cache.mark_stale(na.target_address)?;
    }

    Ok(Icmpv6ProcessResult::Processed)
}

/// 处理 Redirect
fn handle_redirect(
    _redirect: Icmpv6Redirect,
    _source_addr: Ipv6Addr,
    context: &mut Icmpv6Context,
    verbose: bool,
) -> Icmpv6Result<Icmpv6ProcessResult> {
    if !context.config.accept_redirects {
        if verbose {
            println!("ICMPv6: 忽略 Redirect（配置禁用）");
        }
        return Ok(Icmpv6ProcessResult::NoReply);
    }

    if verbose {
        println!("ICMPv6: 收到 Redirect");
    }
    // 当前版本：不处理 Redirect
    Ok(Icmpv6ProcessResult::Processed)
}

/// 创建 ICMPv6 Echo Request（带正确的校验和）
///
/// # 参数
/// - src_addr: 源 IPv6 地址
/// - dst_addr: 目的 IPv6 地址
/// - identifier: 标识符
/// - sequence: 序列号
/// - data: 负载数据
///
/// # 返回
/// - Vec<u8>: 编码后的 ICMPv6 报文（包含正确的校验和）
pub fn create_icmpv6_echo_request(
    src_addr: Ipv6Addr,
    dst_addr: Ipv6Addr,
    identifier: u16,
    sequence: u16,
    data: Vec<u8>,
) -> Vec<u8> {
    let echo = Icmpv6Echo::new_request(identifier, sequence, data);
    let mut bytes = echo.to_bytes_without_checksum();

    // 计算并设置 ICMPv6 校验和
    let checksum = super::checksum::calculate_icmpv6_checksum(src_addr, dst_addr, &bytes);
    bytes[2] = (checksum >> 8) as u8;
    bytes[3] = (checksum & 0xFF) as u8;

    bytes
}

/// 创建 ICMPv6 Echo Reply（带正确的校验和）
///
/// # 参数
/// - src_addr: 源 IPv6 地址
/// - dst_addr: 目的 IPv6 地址
/// - identifier: 标识符
/// - sequence: 序列号
/// - data: 负载数据
///
/// # 返回
/// - Vec<u8>: 编码后的 ICMPv6 报文（包含正确的校验和）
pub fn create_icmpv6_echo_reply(
    src_addr: Ipv6Addr,
    dst_addr: Ipv6Addr,
    identifier: u16,
    sequence: u16,
    data: Vec<u8>,
) -> Vec<u8> {
    let echo = Icmpv6Echo::new_reply(identifier, sequence, data);
    let mut bytes = echo.to_bytes_without_checksum();

    // 计算并设置 ICMPv6 校验和
    let checksum = super::checksum::calculate_icmpv6_checksum(src_addr, dst_addr, &bytes);
    bytes[2] = (checksum >> 8) as u8;
    bytes[3] = (checksum & 0xFF) as u8;

    bytes
}

/// 创建 ICMPv6 Neighbor Solicitation
///
/// # 参数
/// - target_addr: 目标 IPv6 地址
/// - source_link_layer: 源链路层地址（可选）
///
/// # 返回
/// - Vec<u8>: 编码后的 ICMPv6 报文
pub fn create_icmpv6_neighbor_solicitation(
    target_addr: Ipv6Addr,
    source_link_layer: Option<crate::protocols::MacAddr>,
) -> Vec<u8> {
    let mut ns = Icmpv6NeighborSolicitation::new(target_addr);

    if let Some(mac) = source_link_layer {
        let opt = Icmpv6Option::source_link_layer_addr(&mac.bytes);
        ns.options.push(opt);
    }

    ns.to_bytes()
}

/// 创建 ICMPv6 Neighbor Advertisement
///
/// # 参数
/// - target_addr: 目标 IPv6 地址
/// - router: 是否为路由器
/// - solicited: 是否为响应 NS
/// - override_: 是否覆盖现有缓存
/// - target_link_layer: 目标链路层地址（可选）
///
/// # 返回
/// - Vec<u8>: 编码后的 ICMPv6 报文
pub fn create_icmpv6_neighbor_advertisement(
    target_addr: Ipv6Addr,
    router: bool,
    solicited: bool,
    override_: bool,
    target_link_layer: Option<crate::protocols::MacAddr>,
) -> Vec<u8> {
    let mut na = Icmpv6NeighborAdvertisement::new(target_addr, router, solicited, override_);

    if let Some(mac) = target_link_layer {
        let opt = Icmpv6Option::target_link_layer_addr(&mac.bytes);
        na.options.push(opt);
    }

    na.to_bytes()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocols::MacAddr;

    #[test]
    fn test_create_echo_request() {
        let src = Ipv6Addr::new(0xfe80, 0, 0, 0, 0, 0, 0, 1);
        let dst = Ipv6Addr::new(0xfe80, 0, 0, 0, 0, 0, 0, 2);
        let data = vec![0x42; 32];
        let packet = create_icmpv6_echo_request(src, dst, 1234, 1, data.clone());

        assert_eq!(packet[0], 128); // Echo Request
        assert_eq!(packet[1], 0);
    }

    #[test]
    fn test_create_echo_reply() {
        let src = Ipv6Addr::new(0xfe80, 0, 0, 0, 0, 0, 0, 1);
        let dst = Ipv6Addr::new(0xfe80, 0, 0, 0, 0, 0, 0, 2);
        let data = vec![0x42; 32];
        let packet = create_icmpv6_echo_reply(src, dst, 1234, 1, data);

        assert_eq!(packet[0], 129); // Echo Reply
        assert_eq!(packet[1], 0);
    }

    #[test]
    fn test_process_echo_request() {
        let source = Ipv6Addr::new(0xfe80, 0, 0, 0, 0, 0, 0, 1);
        let dest = Ipv6Addr::new(0xfe80, 0, 0, 0, 0, 0, 0, 2);
        let echo_bytes = create_icmpv6_echo_request(source, dest, 1234, 1, vec![0x42; 32]);
        let packet = Packet::from_bytes(echo_bytes);
        let mut context = Icmpv6Context::default();

        let result = process_icmpv6_packet(packet, source, dest, &mut context, false).unwrap();

        match result {
            Icmpv6ProcessResult::Reply(reply_bytes) => {
                assert_eq!(reply_bytes[0], 129); // Echo Reply
            }
            _ => panic!("Expected Reply"),
        }
    }

    #[test]
    fn test_icmpv6_context_default() {
        let context = Icmpv6Context::default();
        assert!(context.config.enable_echo_reply);
        assert_eq!(context.neighbor_cache.len(), 0);
        assert_eq!(context.echo_manager.pending_count(), 0);
    }

    #[test]
    fn test_neighbor_solicitation_creation() {
        let target = Ipv6Addr::new(0xfe80, 0, 0, 0, 0, 0, 0, 1);
        let mac = MacAddr::new([0x00, 0x11, 0x22, 0x33, 0x44, 0x55]);

        let packet = create_icmpv6_neighbor_solicitation(target, Some(mac));

        assert_eq!(packet[0], 135); // Neighbor Solicitation
    }

    #[test]
    fn test_neighbor_advertisement_creation() {
        let target = Ipv6Addr::new(0xfe80, 0, 0, 0, 0, 0, 0, 1);
        let mac = MacAddr::new([0x00, 0x11, 0x22, 0x33, 0x44, 0x55]);

        let packet = create_icmpv6_neighbor_advertisement(
            target, true, true, false, Some(mac)
        );

        assert_eq!(packet[0], 136); // Neighbor Advertisement
    }
}
