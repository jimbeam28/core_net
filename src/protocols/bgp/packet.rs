// src/protocols/bgp/packet.rs
//
// BGP 报文解析和封装

use std::net::{IpAddr, Ipv4Addr};
use crate::protocols::bgp::{
    error::{BgpError, Result},
    message::*,
    BGP_HEADER_SIZE, BGP_MARKER_SIZE, BGP_MIN_MESSAGE_SIZE,
    BGP_MSG_OPEN, BGP_MSG_UPDATE, BGP_MSG_NOTIFICATION,
    BGP_MSG_KEEPALIVE, BGP_MSG_ROUTE_REFRESH, BGP_VERSION,
};
use crate::common::addr::Ipv4Addr as CoreIpv4Addr;

/// 解析 BGP 报文
///
/// # 参数
/// - `data`: BGP 报文字节数组
///
/// # 返回
/// 成功时返回解析的 BgpMessage，失败时返回 BgpError
pub fn parse_bgp_message(data: &[u8]) -> Result<BgpMessage> {
    // 检查最小长度
    if data.len() < BGP_MIN_MESSAGE_SIZE {
        return Err(BgpError::InvalidMessageLength(
            format!("expected at least {} bytes, got {}", BGP_MIN_MESSAGE_SIZE, data.len())
        ));
    }

    // 解析头部
    let header = parse_header(&data[..BGP_HEADER_SIZE + 1])?;

    // 检查长度
    if header.length as usize > data.len() {
        return Err(BgpError::InvalidMessageLength(
            format!("message length {} exceeds data size {}", header.length, data.len())
        ));
    }

    // 检查最大长度
    if header.length as usize > 4096 {
        return Err(BgpError::InvalidMessageLength(
            format!("message length {} exceeds maximum 4096", header.length)
        ));
    }

    // 验证 Marker
    if header.marker != BgpHeader::default_marker() {
        return Err(BgpError::InvalidMarker);
    }

    // 解析消息体
    let msg_data = &data[BGP_HEADER_SIZE + 1..header.length as usize];

    match header.msg_type {
        BGP_MSG_OPEN => {
            let open = parse_open(msg_data)?;
            Ok(BgpMessage::Open(open))
        }
        BGP_MSG_UPDATE => {
            let update = parse_update(msg_data)?;
            Ok(BgpMessage::Update(update))
        }
        BGP_MSG_NOTIFICATION => {
            let notification = parse_notification(msg_data)?;
            Ok(BgpMessage::Notification(notification))
        }
        BGP_MSG_KEEPALIVE => {
            // KEEPALIVE 只有头部，没有数据
            Ok(BgpMessage::Keepalive(BgpKeepalive))
        }
        BGP_MSG_ROUTE_REFRESH => {
            let route_refresh = parse_route_refresh(msg_data)?;
            Ok(BgpMessage::RouteRefresh(route_refresh))
        }
        _ => Err(BgpError::InvalidMessageType(header.msg_type)),
    }
}

/// 解析 BGP 头部
fn parse_header(data: &[u8]) -> Result<BgpHeader> {
    if data.len() < BGP_HEADER_SIZE + 1 {
        return Err(BgpError::InvalidMessageLength("header too short".to_string()));
    }

    let mut marker = [0u8; BGP_MARKER_SIZE];
    marker.copy_from_slice(&data[0..BGP_MARKER_SIZE]);

    let length = u16::from_be_bytes([data[16], data[17]]);
    let msg_type = data[18];

    Ok(BgpHeader {
        marker,
        length,
        msg_type,
    })
}

/// 解析 OPEN 报文
fn parse_open(data: &[u8]) -> Result<BgpOpen> {
    if data.len() < 10 {
        return Err(BgpError::InvalidMessageLength("OPEN too short".to_string()));
    }

    let version = data[0];
    if version != BGP_VERSION {
        return Err(BgpError::UnsupportedVersion(version));
    }

    let my_as = u16::from_be_bytes([data[1], data[2]]);
    let hold_time = u16::from_be_bytes([data[3], data[4]]);

    let bgp_id = CoreIpv4Addr::new(data[5], data[6], data[7], data[8]);

    let opt_param_len = data[9];
    let mut optional_parameters = Vec::new();

    if opt_param_len > 0 {
        let mut offset = 10;
        while offset < 10 + opt_param_len as usize {
            if offset + 2 > data.len() {
                break;
            }
            let param_type = data[offset];
            let param_len = data[offset + 1] as usize;

            if offset + 2 + param_len > data.len() {
                break;
            }

            let param_data = &data[offset + 2..offset + 2 + param_len];

            match param_type {
                1 => {
                    // 认证信息
                    if param_len > 0 {
                        optional_parameters.push(OptionalParameter::Authentication {
                            auth_code: param_data[0],
                            data: param_data[1..].to_vec(),
                        });
                    }
                }
                2 => {
                    // 能力通告
                    let capabilities = parse_capabilities(param_data)?;
                    optional_parameters.push(OptionalParameter::Capabilities { capabilities });
                }
                _ => {
                    // 忽略未知参数
                }
            }

            offset += 2 + param_len;
        }
    }

    Ok(BgpOpen {
        version,
        my_as,
        hold_time,
        bgp_identifier: bgp_id,
        optional_parameters,
    })
}

/// 解析能力列表
fn parse_capabilities(data: &[u8]) -> Result<Vec<BgpCapability>> {
    let mut capabilities = Vec::new();
    let mut offset = 0;

    while offset + 4 <= data.len() {
        let cap_code = data[offset];
        let cap_len = u16::from_be_bytes([data[offset + 1], data[offset + 2]]) as usize;

        if offset + 4 + cap_len > data.len() {
            break;
        }

        let cap_data = &data[offset + 4..offset + 4 + cap_len];

        let capability = match cap_code {
            1 => {
                // Multi-Protocol (MP-BGP)
                if cap_len >= 4 {
                    let afi = u16::from_be_bytes([cap_data[0], cap_data[1]]);
                    let _reserved = cap_data[2];
                    let safi = cap_data[3];
                    BgpCapability::MultiProtocol { afi, safi }
                } else {
                    BgpCapability::Unknown {
                        code: cap_code,
                        data: cap_data.to_vec(),
                    }
                }
            }
            2 => BgpCapability::RouteRefresh,
            4 => {
                // 4-Octet AS Number
                if cap_len >= 4 {
                    let as_number = u32::from_be_bytes([
                        cap_data[0], cap_data[1], cap_data[2], cap_data[3],
                    ]);
                    BgpCapability::FourOctetAsNumber { as_number }
                } else {
                    BgpCapability::Unknown {
                        code: cap_code,
                        data: cap_data.to_vec(),
                    }
                }
            }
            6 => BgpCapability::CapabilityNegotiation,
            _ => BgpCapability::Unknown {
                code: cap_code,
                data: cap_data.to_vec(),
            },
        };

        capabilities.push(capability);
        offset += 4 + cap_len;
    }

    Ok(capabilities)
}

/// 解析 UPDATE 报文
fn parse_update(data: &[u8]) -> Result<BgpUpdate> {
    if data.len() < 4 {
        return Err(BgpError::InvalidMessageLength("UPDATE too short".to_string()));
    }

    let mut offset = 0;

    // 解析 Withdrawn Routes Length
    let withdrawn_len = u16::from_be_bytes([data[offset], data[offset + 1]]) as usize;
    offset += 2;

    // 解析 Withdrawn Routes
    let mut withdrawn_routes = Vec::new();
    if withdrawn_len > 0 {
        let withdrawn_end = offset + withdrawn_len;
        if withdrawn_end > data.len() {
            return Err(BgpError::InvalidMessageLength("withdrawn routes overflow".to_string()));
        }

        while offset < withdrawn_end {
            let (prefix, bytes_read) = parse_prefix(&data[offset..])?;
            withdrawn_routes.push(prefix);
            offset += bytes_read;
        }
    }

    // 解析 Total Path Attribute Length
    if offset + 2 > data.len() {
        return Err(BgpError::InvalidMessageLength("path attribute length overflow".to_string()));
    }

    let path_attr_len = u16::from_be_bytes([data[offset], data[offset + 1]]) as usize;
    offset += 2;

    // 解析 Path Attributes
    let mut path_attributes = Vec::new();
    if path_attr_len > 0 {
        let path_attr_end = offset + path_attr_len;
        if path_attr_end > data.len() {
            return Err(BgpError::InvalidMessageLength("path attributes overflow".to_string()));
        }

        while offset < path_attr_end {
            let (attr, bytes_read) = parse_path_attribute(&data[offset..])?;
            path_attributes.push(attr);
            offset += bytes_read;
        }
    }

    // 解析 NLRI
    let mut nlri = Vec::new();
    while offset < data.len() {
        let (prefix, bytes_read) = parse_prefix(&data[offset..])?;
        nlri.push(prefix);
        offset += bytes_read;
    }

    Ok(BgpUpdate {
        withdrawn_routes,
        path_attributes,
        nlri,
    })
}

/// 解析 IP 前缀
fn parse_prefix(data: &[u8]) -> Result<(IpPrefix, usize)> {
    if data.is_empty() {
        return Err(BgpError::InvalidMessageLength("prefix data empty".to_string()));
    }

    let prefix_len = data[0];
    let byte_len = (prefix_len as usize).div_ceil(8);

    if 1 + byte_len > data.len() {
        return Err(BgpError::InvalidMessageLength("prefix overflow".to_string()));
    }

    // 解析 IP 地址
    let mut octets = [0u8; 4];
    octets[..byte_len].copy_from_slice(&data[1..=byte_len]);

    // 清理多余位
    if !prefix_len.is_multiple_of(8) {
        let mask = 0xFF << (8 - (prefix_len % 8));
        octets[byte_len - 1] &= mask;
    }

    let prefix = IpAddr::V4(Ipv4Addr::new(octets[0], octets[1], octets[2], octets[3]));

    Ok((IpPrefix::new(prefix, prefix_len), 1 + byte_len))
}

/// 解析路径属性
fn parse_path_attribute(data: &[u8]) -> Result<(PathAttribute, usize)> {
    if data.len() < 3 {
        return Err(BgpError::InvalidMessageLength("path attribute too short".to_string()));
    }

    let flags = data[0];
    let type_code = data[1];
    let extended_length = flags & 0x10 != 0;

    // 解析长度（可选扩展长度）
    let (length, offset) = if extended_length {
        if data.len() < 4 {
            return Err(BgpError::InvalidMessageLength("extended length but data too short".to_string()));
        }
        let len = u16::from_be_bytes([data[2], data[3]]) as usize;
        (len, 4)
    } else {
        let len = data[2] as usize;
        (len, 3)
    };

    if offset + length > data.len() {
        return Err(BgpError::InvalidMessageLength("path attribute data overflow".to_string()));
    }

    let attr_data = &data[offset..offset + length];

    let attr = match type_code {
        1 => {
            // ORIGIN
            let origin = if !attr_data.is_empty() { attr_data[0] } else { 0 };
            PathAttribute::Origin { origin }
        }
        2 => {
            // AS_PATH
            let (as_sequence, as_set) = parse_as_path(attr_data)?;
            PathAttribute::AsPath { as_sequence, as_set }
        }
        3 => {
            // NEXT_HOP
            if attr_data.len() >= 4 {
                let next_hop = CoreIpv4Addr::new(attr_data[0], attr_data[1], attr_data[2], attr_data[3]);
                PathAttribute::NextHop { next_hop }
            } else {
                return Err(BgpError::InvalidPathAttribute("NEXT_HOP too short".to_string()));
            }
        }
        4 => {
            // MED
            if attr_data.len() >= 4 {
                let med = u32::from_be_bytes([attr_data[0], attr_data[1], attr_data[2], attr_data[3]]);
                PathAttribute::MultiExitDisc { med }
            } else {
                return Err(BgpError::InvalidPathAttribute("MED too short".to_string()));
            }
        }
        5 => {
            // LOCAL_PREF
            if attr_data.len() >= 4 {
                let local_pref = u32::from_be_bytes([attr_data[0], attr_data[1], attr_data[2], attr_data[3]]);
                PathAttribute::LocalPref { local_pref }
            } else {
                return Err(BgpError::InvalidPathAttribute("LOCAL_PREF too short".to_string()));
            }
        }
        6 => PathAttribute::AtomicAggregate,
        7 => {
            // AGGREGATOR
            if attr_data.len() >= 6 {
                let as_number = u16::from_be_bytes([attr_data[0], attr_data[1]]) as u32;
                let router_id = CoreIpv4Addr::new(attr_data[2], attr_data[3], attr_data[4], attr_data[5]);
                PathAttribute::Aggregator { as_number, router_id }
            } else {
                return Err(BgpError::InvalidPathAttribute("AGGREGATOR too short".to_string()));
            }
        }
        8 => {
            // COMMUNITY
            let mut communities = Vec::new();
            let mut i = 0;
            while i + 4 <= attr_data.len() {
                let community = u32::from_be_bytes([
                    attr_data[i], attr_data[i + 1],
                    attr_data[i + 2], attr_data[i + 3],
                ]);
                communities.push(community);
                i += 4;
            }
            PathAttribute::Community { communities }
        }
        14 => {
            // MP_REACH_NLRI (简化实现)
            PathAttribute::MpReachNlri {
                afi: 0,
                safi: 0,
                next_hop: vec![],
                nlri: vec![],
            }
        }
        15 => {
            // MP_UNREACH_NLRI (简化实现)
            PathAttribute::MpUnreachNlri {
                afi: 0,
                safi: 0,
                nlri: vec![],
            }
        }
        _ => {
            // 忽略未知属性
            return Err(BgpError::InvalidPathAttribute(format!("unknown attribute type {}", type_code)));
        }
    };

    // 计算总长度（包括头部）
    let total_len = offset + length;

    Ok((attr, total_len))
}

/// 解析 AS_PATH
fn parse_as_path(data: &[u8]) -> Result<(Vec<u32>, Vec<u32>)> {
    let mut as_sequence = Vec::new();
    let mut as_set = Vec::new();
    let mut offset = 0;

    while offset + 2 <= data.len() {
        let segment_type = data[offset];
        let segment_length = data[offset + 1] as usize;
        offset += 2;

        if offset + segment_length * 4 > data.len() {
            break;
        }

        let segment_as: Vec<u32> = (0..segment_length)
            .map(|i| {
                let start = offset + i * 4;
                u32::from_be_bytes([data[start], data[start + 1], data[start + 2], data[start + 3]])
            })
            .collect();

        match segment_type {
            1 => as_sequence.extend(segment_as),
            2 => as_set.extend(segment_as),
            _ => {}
        }

        offset += segment_length * 4;
    }

    Ok((as_sequence, as_set))
}

/// 解析 NOTIFICATION 报文
fn parse_notification(data: &[u8]) -> Result<BgpNotification> {
    let error_code = if !data.is_empty() { data[0] } else { 0 };
    let error_subcode = if data.len() > 1 { data[1] } else { 0 };
    let notification_data = if data.len() > 2 { data[2..].to_vec() } else { vec![] };

    Ok(BgpNotification {
        error_code,
        error_subcode,
        data: notification_data,
    })
}

/// 解析 ROUTE-REFRESH 报文
fn parse_route_refresh(data: &[u8]) -> Result<BgpRouteRefresh> {
    if data.len() < 4 {
        return Err(BgpError::InvalidMessageLength("ROUTE-REFRESH too short".to_string()));
    }

    let afi = u16::from_be_bytes([data[0], data[1]]);
    let reserved = data[2];
    let safi = data[3];

    Ok(BgpRouteRefresh {
        afi,
        reserved,
        safi,
    })
}

/// 封装 BGP 报文
///
/// # 参数
/// - `msg`: BGP 报文
///
/// # 返回
/// 返回封装后的字节数组
pub fn encapsulate_bgp_message(msg: &BgpMessage) -> Vec<u8> {
    match msg {
        BgpMessage::Open(open) => encapsulate_open(open),
        BgpMessage::Update(update) => encapsulate_update(update),
        BgpMessage::Notification(notification) => encapsulate_notification(notification),
        BgpMessage::Keepalive(_) => encapsulate_keepalive(),
        BgpMessage::RouteRefresh(rr) => encapsulate_route_refresh(rr),
    }
}

/// 封装 OPEN 报文
fn encapsulate_open(open: &BgpOpen) -> Vec<u8> {
    let mut msg_data = Vec::new();

    msg_data.push(open.version);
    msg_data.extend_from_slice(&open.my_as.to_be_bytes());
    msg_data.extend_from_slice(&open.hold_time.to_be_bytes());
    msg_data.extend_from_slice(&open.bgp_identifier.bytes);

    // 计算可选参数长度
    let opt_params = encapsulate_optional_parameters(&open.optional_parameters);
    msg_data.push(opt_params.len() as u8);
    msg_data.extend_from_slice(&opt_params);

    // 封装头部
    let total_len = (BGP_HEADER_SIZE + 1) + msg_data.len();
    let mut packet = Vec::with_capacity(total_len);

    // Marker
    packet.extend_from_slice(&BgpHeader::default_marker());
    // Length
    packet.extend_from_slice(&(total_len as u16).to_be_bytes());
    // Type
    packet.push(BGP_MSG_OPEN);
    // Data
    packet.extend_from_slice(&msg_data);

    packet
}

/// 封装可选参数
fn encapsulate_optional_parameters(params: &[OptionalParameter]) -> Vec<u8> {
    let mut data = Vec::new();

    for param in params {
        match param {
            OptionalParameter::Authentication { auth_code, data: auth_data } => {
                data.push(1); // 类型
                data.push((1 + auth_data.len()) as u8);
                data.push(*auth_code);
                data.extend_from_slice(auth_data);
            }
            OptionalParameter::Capabilities { capabilities } => {
                let cap_data = encapsulate_capabilities(capabilities);
                data.push(2); // 类型
                data.extend_from_slice(&(cap_data.len() as u16).to_be_bytes());
                data.extend_from_slice(&cap_data);
            }
        }
    }

    data
}

/// 封装能力列表
fn encapsulate_capabilities(capabilities: &[BgpCapability]) -> Vec<u8> {
    let mut data = Vec::new();

    for cap in capabilities {
        match cap {
            BgpCapability::MultiProtocol { afi, safi } => {
                data.push(1); // code
                data.extend_from_slice(&2u16.to_be_bytes()); // length (大端序)
                data.push(afi.to_be_bytes()[0]);
                data.push(afi.to_be_bytes()[1]);
                data.push(0); // reserved
                data.push(*safi);
            }
            BgpCapability::RouteRefresh => {
                data.push(2); // code
                data.extend_from_slice(&0u16.to_be_bytes()); // length
            }
            BgpCapability::FourOctetAsNumber { as_number } => {
                data.push(4); // code
                data.extend_from_slice(&4u16.to_be_bytes()); // length
                data.extend_from_slice(&as_number.to_be_bytes());
            }
            BgpCapability::CapabilityNegotiation => {
                data.push(6); // code
                data.extend_from_slice(&0u16.to_be_bytes()); // length
            }
            BgpCapability::Unknown { code: _, data: _cap_data } => {
                // 忽略未知能力
            }
        }
    }

    data
}

/// 封装 UPDATE 报文
fn encapsulate_update(update: &BgpUpdate) -> Vec<u8> {
    let mut msg_data = Vec::new();

    // 封装 Withdrawn Routes
    let withdrawn_data = encapsulate_prefix_list(&update.withdrawn_routes);
    msg_data.extend_from_slice(&(withdrawn_data.len() as u16).to_be_bytes());
    msg_data.extend_from_slice(&withdrawn_data);

    // 封装 Path Attributes
    let path_attr_data = encapsulate_path_attributes(&update.path_attributes);
    msg_data.extend_from_slice(&(path_attr_data.len() as u16).to_be_bytes());
    msg_data.extend_from_slice(&path_attr_data);

    // 封装 NLRI
    let nlri_data = encapsulate_prefix_list(&update.nlri);
    msg_data.extend_from_slice(&nlri_data);

    // 封装头部
    let total_len = (BGP_HEADER_SIZE + 1) + msg_data.len();
    let mut packet = Vec::with_capacity(total_len);

    packet.extend_from_slice(&BgpHeader::default_marker());
    packet.extend_from_slice(&(total_len as u16).to_be_bytes());
    packet.push(BGP_MSG_UPDATE);
    packet.extend_from_slice(&msg_data);

    packet
}

/// 封装前缀列表
fn encapsulate_prefix_list(prefixes: &[IpPrefix]) -> Vec<u8> {
    let mut data = Vec::new();

    for prefix in prefixes {
        data.extend_from_slice(&encapsulate_prefix(prefix));
    }

    data
}

/// 封装单个前缀
fn encapsulate_prefix(prefix: &IpPrefix) -> Vec<u8> {
    let mut data = Vec::new();
    data.push(prefix.prefix_len);

    if let IpAddr::V4(ipv4) = prefix.prefix {
        let octets = ipv4.octets();
        let byte_len = (prefix.prefix_len as usize).div_ceil(8);
        data.extend_from_slice(&octets[..byte_len]);
    }

    data
}

/// 封装路径属性
fn encapsulate_path_attributes(attrs: &[PathAttribute]) -> Vec<u8> {
    let mut data = Vec::new();

    for attr in attrs {
        data.extend_from_slice(&encapsulate_path_attribute(attr));
    }

    data
}

/// 封装单个路径属性
fn encapsulate_path_attribute(attr: &PathAttribute) -> Vec<u8> {
    let mut data = Vec::new();

    // 标志位（可选、传递、部分、扩展长度）
    let flags = match attr {
        PathAttribute::Origin { .. } => 0x40, // Well-known, mandatory
        PathAttribute::AsPath { .. } => 0x40,
        PathAttribute::NextHop { .. } => 0x40,
        PathAttribute::MultiExitDisc { .. } => 0x80, // Optional, non-transitive
        PathAttribute::LocalPref { .. } => 0x40,
        PathAttribute::AtomicAggregate => 0x40,
        PathAttribute::Aggregator { .. } => 0xC0, // Optional, transitive
        PathAttribute::Community { .. } => 0xC0,
        PathAttribute::MpReachNlri { .. } => 0x80,
        PathAttribute::MpUnreachNlri { .. } => 0x80,
    };

    let (type_code, attr_data) = match attr {
        PathAttribute::Origin { origin } => {
            (1, vec![*origin])
        }
        PathAttribute::AsPath { as_sequence, as_set } => {
            (2, encapsulate_as_path(as_sequence, as_set))
        }
        PathAttribute::NextHop { next_hop } => {
            (3, next_hop.bytes.to_vec())
        }
        PathAttribute::MultiExitDisc { med } => {
            (4, med.to_be_bytes().to_vec())
        }
        PathAttribute::LocalPref { local_pref } => {
            (5, local_pref.to_be_bytes().to_vec())
        }
        PathAttribute::AtomicAggregate => {
            (6, vec![])
        }
        PathAttribute::Aggregator { as_number, router_id } => {
            let mut d = Vec::new();
            d.extend_from_slice(&(*as_number as u16).to_be_bytes());
            d.extend_from_slice(&router_id.bytes);
            (7, d)
        }
        PathAttribute::Community { communities } => {
            let mut d = Vec::new();
            for c in communities {
                d.extend_from_slice(&c.to_be_bytes());
            }
            (8, d)
        }
        PathAttribute::MpReachNlri { .. } => {
            // 简化实现，返回空数据
            (14, vec![])
        }
        PathAttribute::MpUnreachNlri { .. } => {
            // 简化实现，返回空数据
            (15, vec![])
        }
    };

    data.push(flags);
    data.push(type_code);
    data.push(attr_data.len() as u8);
    data.extend_from_slice(&attr_data);

    data
}

/// 封装 AS_PATH
fn encapsulate_as_path(as_sequence: &[u32], as_set: &[u32]) -> Vec<u8> {
    let mut data = Vec::new();

    // AS_SEQUENCE
    if !as_sequence.is_empty() {
        data.push(1); // 类型
        data.push(as_sequence.len() as u8);
        for as_num in as_sequence {
            data.extend_from_slice(&as_num.to_be_bytes());
        }
    }

    // AS_SET
    if !as_set.is_empty() {
        data.push(2); // 类型
        data.push(as_set.len() as u8);
        for as_num in as_set {
            data.extend_from_slice(&as_num.to_be_bytes());
        }
    }

    data
}

/// 封装 NOTIFICATION 报文
fn encapsulate_notification(notification: &BgpNotification) -> Vec<u8> {
    let mut msg_data = Vec::new();
    msg_data.push(notification.error_code);
    msg_data.push(notification.error_subcode);
    msg_data.extend_from_slice(&notification.data);

    let total_len = (BGP_HEADER_SIZE + 1) + msg_data.len();
    let mut packet = Vec::with_capacity(total_len);

    packet.extend_from_slice(&BgpHeader::default_marker());
    packet.extend_from_slice(&(total_len as u16).to_be_bytes());
    packet.push(BGP_MSG_NOTIFICATION);
    packet.extend_from_slice(&msg_data);

    packet
}

/// 封装 KEEPALIVE 报文
fn encapsulate_keepalive() -> Vec<u8> {
    let total_len = BGP_HEADER_SIZE + 1;
    let mut packet = Vec::with_capacity(total_len);

    packet.extend_from_slice(&BgpHeader::default_marker());
    packet.extend_from_slice(&(total_len as u16).to_be_bytes());
    packet.push(BGP_MSG_KEEPALIVE);

    packet
}

/// 封装 ROUTE-REFRESH 报文
fn encapsulate_route_refresh(rr: &BgpRouteRefresh) -> Vec<u8> {
    let mut msg_data = Vec::new();
    msg_data.extend_from_slice(&rr.afi.to_be_bytes());
    msg_data.push(rr.reserved);
    msg_data.push(rr.safi);

    let total_len = (BGP_HEADER_SIZE + 1) + msg_data.len();
    let mut packet = Vec::with_capacity(total_len);

    packet.extend_from_slice(&BgpHeader::default_marker());
    packet.extend_from_slice(&(total_len as u16).to_be_bytes());
    packet.push(BGP_MSG_ROUTE_REFRESH);
    packet.extend_from_slice(&msg_data);

    packet
}
