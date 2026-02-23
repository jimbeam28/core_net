// src/protocols/ipv6/extension.rs
//
// IPv6 扩展头定义和处理

use crate::common::{CoreError, Packet, Result};
use crate::protocols::Ipv6Addr;

use super::protocol::IpProtocol;

// --- 扩展头常量 ---

/// 扩展头最小长度（8字节）
pub const EXTENSION_HEADER_MIN_LEN: usize = 8;

/// 默认最大扩展头数量
pub const DEFAULT_MAX_EXTENSION_HEADERS: usize = 8;

/// 默认最大扩展头链长度
pub const DEFAULT_MAX_EXTENSION_HEADERS_LENGTH: usize = 2048;

// --- 通用扩展头头部 ---

/// 通用扩展头头部（RFC 8200）
///
/// 所有 IPv6 扩展头都遵循这个通用格式。
/// 扩展头长度以 8 字节为单位，不包括前 8 字节。
#[repr(C, packed)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExtensionHeader {
    /// 下一头部类型
    pub next_header: u8,
    /// 扩展头长度（以 8 字节为单位，不包括前 8 字节）
    pub header_length: u8,
}

impl ExtensionHeader {
    /// 获取扩展头总长度（字节数）
    pub fn total_length(&self) -> usize {
        ((self.header_length as usize) + 1) * 8
    }

    /// 从字节流解析扩展头
    pub fn from_bytes(data: &[u8]) -> Result<Self> {
        if data.len() < 2 {
            return Err(CoreError::parse_error("扩展头数据不足"));
        }

        Ok(ExtensionHeader {
            next_header: data[0],
            header_length: data[1],
        })
    }

    /// 序列化为字节数组
    pub fn to_bytes(&self) -> [u8; 2] {
        [self.next_header, self.header_length]
    }
}

// --- 逐跳选项头 (Hop-by-Hop Options Header) ---

/// 逐跳选项头（Next Header = 0）
///
/// 必须由路径上每个节点处理，必须紧跟在 IPv6 基本头部之后。
#[repr(C, packed)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HopByHopHeader {
    /// 下一头部
    pub next_header: u8,
    /// 扩展头长度
    pub header_length: u8,
}

impl HopByHopHeader {
    /// 创建新的逐跳选项头
    pub fn new(next_header: u8, options_length: usize) -> Self {
        // 计算头部长度（不包括前8字节）
        let header_length = if options_length == 0 {
            0
        } else {
            options_length.div_ceil(8) - 1
        };

        HopByHopHeader {
            next_header,
            header_length: header_length as u8,
        }
    }

    /// 获取扩展头总长度
    pub fn total_length(&self) -> usize {
        ((self.header_length as usize) + 1) * 8
    }

    /// 从字节流解析
    pub fn from_bytes(data: &[u8]) -> Result<Self> {
        if data.len() < 2 {
            return Err(CoreError::parse_error("逐跳选项头数据不足"));
        }

        Ok(HopByHopHeader {
            next_header: data[0],
            header_length: data[1],
        })
    }

    /// 序列化为字节数组
    pub fn to_bytes(&self) -> [u8; 2] {
        [self.next_header, self.header_length]
    }
}

// --- 路由头 (Routing Header) ---

/// 路由头（Next Header = 43）
///
/// 用于指定数据包经过的中间路由。
#[repr(C, packed)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RoutingHeader {
    /// 下一头部
    pub next_header: u8,
    /// 扩展头长度
    pub header_length: u8,
    /// 路由类型
    pub routing_type: u8,
    /// 剩余段数
    pub segments_left: u8,
}

impl RoutingHeader {
    /// 创建新的路由头
    pub fn new(next_header: u8, routing_type: u8, segments_left: u8, data_length: usize) -> Self {
        let header_length = if data_length == 0 {
            0
        } else {
            data_length.div_ceil(8) - 1
        };

        RoutingHeader {
            next_header,
            header_length: header_length as u8,
            routing_type,
            segments_left,
        }
    }

    /// 获取扩展头总长度
    pub fn total_length(&self) -> usize {
        ((self.header_length as usize) + 1) * 8
    }

    /// 从字节流解析
    pub fn from_bytes(data: &[u8]) -> Result<Self> {
        if data.len() < 4 {
            return Err(CoreError::parse_error("路由头数据不足"));
        }

        Ok(RoutingHeader {
            next_header: data[0],
            header_length: data[1],
            routing_type: data[2],
            segments_left: data[3],
        })
    }

    /// 序列化为字节数组
    pub fn to_bytes(&self) -> [u8; 4] {
        [
            self.next_header,
            self.header_length,
            self.routing_type,
            self.segments_left,
        ]
    }

    /// 检查是否为 Type 0 路由头（已废弃）
    pub fn is_type0(&self) -> bool {
        self.routing_type == 0
    }
}

/// Type 2 路由头（Mobile IPv6 家乡地址）
///
/// 用于移动 IPv6，携带移动节点的家乡地址。
#[repr(C, packed)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RoutingHeaderType2 {
    pub next_header: u8,
    pub header_length: u8,
    pub routing_type: u8,
    pub segments_left: u8,
    pub reserved: [u8; 4],
    pub home_address: Ipv6Addr,
}

impl RoutingHeaderType2 {
    /// 创建新的 Type 2 路由头
    pub fn new(next_header: u8, home_address: Ipv6Addr) -> Self {
        RoutingHeaderType2 {
            next_header,
            header_length: 2, // 固定为 2（24 字节）
            routing_type: 2,
            segments_left: 0,
            reserved: [0; 4],
            home_address,
        }
    }

    /// 从字节流解析
    pub fn from_bytes(data: &[u8]) -> Result<Self> {
        if data.len() < 24 {
            return Err(CoreError::parse_error("Type 2 路由头数据不足"));
        }

        let mut home_addr_bytes = [0u8; 16];
        home_addr_bytes.copy_from_slice(&data[8..24]);

        Ok(RoutingHeaderType2 {
            next_header: data[0],
            header_length: data[1],
            routing_type: data[2],
            segments_left: data[3],
            reserved: [data[4], data[5], data[6], data[7]],
            home_address: Ipv6Addr::from_bytes(home_addr_bytes),
        })
    }

    /// 序列化为字节数组
    pub fn to_bytes(&self) -> [u8; 24] {
        let mut bytes = [0u8; 24];
        bytes[0] = self.next_header;
        bytes[1] = self.header_length;
        bytes[2] = self.routing_type;
        bytes[3] = self.segments_left;
        bytes[4..8].copy_from_slice(&self.reserved);
        bytes[8..24].copy_from_slice(&self.home_address.bytes);
        bytes
    }
}

// --- 分片头 (Fragment Header) ---

/// 分片头（Next Header = 44）
///
/// 用于数据包的分片和重组，固定 8 字节。
#[repr(C, packed)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FragmentHeader {
    /// 下一头部
    pub next_header: u8,
    /// 保留字段
    pub reserved: u8,
    /// 片偏移 (高 13 位) + 保留 (高 2 位) + M 标志 (低 1 位)
    pub offset_res_m: u16,
    /// 分片标识符
    pub identification: u32,
}

impl FragmentHeader {
    /// 分片头固定长度
    pub const HEADER_SIZE: usize = 8;

    /// 创建新的分片头
    pub fn new(
        next_header: u8,
        fragment_offset: u16,
        more_fragments: bool,
        identification: u32,
    ) -> Self {
        let offset_res_m = (fragment_offset << 3) | (if more_fragments { 1 } else { 0 });

        FragmentHeader {
            next_header,
            reserved: 0,
            offset_res_m,
            identification,
        }
    }

    /// 获取片偏移（以 8 字节为单位）
    pub fn fragment_offset(&self) -> u16 {
        (self.offset_res_m & 0xFFF8) >> 3
    }

    /// 获取 M 标志（更多分片）
    pub fn more_fragments(&self) -> bool {
        (self.offset_res_m & 0x01) != 0
    }

    /// 检查是否为原子分片（单个分片，RFC 8200 禁止）
    pub fn is_atomic_fragment(&self) -> bool {
        self.fragment_offset() == 0 && !self.more_fragments()
    }

    /// 从字节流解析
    pub fn from_bytes(data: &[u8]) -> Result<Self> {
        if data.len() < 8 {
            return Err(CoreError::parse_error("分片头数据不足"));
        }

        let offset_res_m = u16::from_be_bytes([data[2], data[3]]);
        let identification = u32::from_be_bytes([
            data[4], data[5], data[6], data[7]
        ]);

        Ok(FragmentHeader {
            next_header: data[0],
            reserved: data[1],
            offset_res_m,
            identification,
        })
    }

    /// 序列化为字节数组
    pub fn to_bytes(&self) -> [u8; 8] {
        let mut bytes = [0u8; 8];
        bytes[0] = self.next_header;
        bytes[1] = self.reserved;

        let offset_res_m_bytes = self.offset_res_m.to_be_bytes();
        bytes[2] = offset_res_m_bytes[0];
        bytes[3] = offset_res_m_bytes[1];

        let id_bytes = self.identification.to_be_bytes();
        bytes[4] = id_bytes[0];
        bytes[5] = id_bytes[1];
        bytes[6] = id_bytes[2];
        bytes[7] = id_bytes[3];

        bytes
    }
}

// --- 目的选项头 (Destination Options Header) ---

/// 目的选项头（Next Header = 60）
///
/// 格式与逐跳选项头相同，但仅由目的节点处理。
#[repr(C, packed)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DestinationOptionsHeader {
    /// 下一头部
    pub next_header: u8,
    /// 扩展头长度
    pub header_length: u8,
}

impl DestinationOptionsHeader {
    /// 创建新的目的选项头
    pub fn new(next_header: u8, options_length: usize) -> Self {
        let header_length = if options_length == 0 {
            0
        } else {
            options_length.div_ceil(8) - 1
        };

        DestinationOptionsHeader {
            next_header,
            header_length: header_length as u8,
        }
    }

    /// 获取扩展头总长度
    pub fn total_length(&self) -> usize {
        ((self.header_length as usize) + 1) * 8
    }

    /// 从字节流解析
    pub fn from_bytes(data: &[u8]) -> Result<Self> {
        if data.len() < 2 {
            return Err(CoreError::parse_error("目的选项头数据不足"));
        }

        Ok(DestinationOptionsHeader {
            next_header: data[0],
            header_length: data[1],
        })
    }

    /// 序列化为字节数组
    pub fn to_bytes(&self) -> [u8; 2] {
        [self.next_header, self.header_length]
    }
}

// --- 扩展头枚举 ---

/// IPv6 扩展头枚举
///
/// 包含所有类型的扩展头及其数据。
#[derive(Debug, Clone, PartialEq)]
pub enum ExtensionHeaderType {
    /// 逐跳选项头
    HopByHop {
        header: HopByHopHeader,
        options: Vec<u8>,
    },

    /// 路由头
    Routing {
        header: RoutingHeader,
        data: Vec<u8>,
    },

    /// 分片头
    Fragment {
        header: FragmentHeader,
    },

    /// 目的选项头
    DestinationOptions {
        header: DestinationOptionsHeader,
        options: Vec<u8>,
    },

    /// IPSec 认证头（AH）
    AuthenticationHeader {
        header: Vec<u8>,
    },

    /// IPSec 封装安全载荷（ESP）
    EncapsulatingSecurityPayload {
        data: Vec<u8>,
    },
}

impl ExtensionHeaderType {
    /// 获取下一头部类型
    pub fn next_header(&self) -> u8 {
        match self {
            ExtensionHeaderType::HopByHop { header, .. } => header.next_header,
            ExtensionHeaderType::Routing { header, .. } => header.next_header,
            ExtensionHeaderType::Fragment { header } => header.next_header,
            ExtensionHeaderType::DestinationOptions { header, .. } => header.next_header,
            ExtensionHeaderType::AuthenticationHeader { header } => header[0],
            ExtensionHeaderType::EncapsulatingSecurityPayload { .. } => {
                // ESP 没有 next_header 字段在头部
                50 // ESP protocol number
            }
        }
    }

    /// 获取扩展头总长度
    pub fn total_length(&self) -> usize {
        match self {
            ExtensionHeaderType::HopByHop { header, .. } => header.total_length(),
            ExtensionHeaderType::Routing { header, .. } => header.total_length(),
            ExtensionHeaderType::Fragment { .. } => FragmentHeader::HEADER_SIZE,
            ExtensionHeaderType::DestinationOptions { header, .. } => header.total_length(),
            ExtensionHeaderType::AuthenticationHeader { header } => header.len(),
            ExtensionHeaderType::EncapsulatingSecurityPayload { data } => data.len(),
        }
    }
}

// --- 扩展头解析 ---

/// 解析扩展头链
///
/// 从 Packet 中解析扩展头链，返回：
/// - 上层协议类型
/// - 所有扩展头的列表
/// - 剩余的负载数据起始位置（相对于 IPv6 头部之后）
pub fn parse_extension_chain(
    packet: &mut Packet,
    initial_next_header: IpProtocol,
    config: &ExtensionConfig,
) -> Result<ExtensionChainResult> {
    let mut next_header = initial_next_header;
    let mut headers = Vec::new();
    let mut extension_chain_length = 0;

    loop {
        match next_header {
            IpProtocol::HopByHopOptions => {
                // 检查扩展头数量
                if headers.len() >= config.max_extension_headers {
                    return Err(CoreError::InvalidPacket(format!(
                        "扩展头数量超过限制: {} >= {}",
                        headers.len(),
                        config.max_extension_headers
                    )));
                }

                // 读取扩展头
                let ext_bytes = packet.peek(2)
                    .ok_or_else(|| CoreError::parse_error("读取逐跳选项头失败"))?;

                let header = HopByHopHeader::from_bytes(ext_bytes)?;
                let total_len = header.total_length();

                // 检查扩展头链长度
                extension_chain_length += total_len;
                if extension_chain_length > config.max_extension_headers_length {
                    return Err(CoreError::InvalidPacket(format!(
                        "扩展头链过长: {} > {}",
                        extension_chain_length,
                        config.max_extension_headers_length
                    )));
                }

                // 读取完整扩展头数据
                let full_data = packet.read(total_len)
                    .ok_or_else(|| CoreError::parse_error("读取逐跳选项数据失败"))?;

                // 提取选项部分（跳过前2字节头部）
                let options = full_data[2..].to_vec();

                next_header = IpProtocol::from(header.next_header);
                headers.push(ExtensionHeaderType::HopByHop {
                    header,
                    options,
                });
            }

            IpProtocol::Ipv6DestOptions => {
                // 处理目的选项头（逻辑与逐跳选项类似）
                if headers.len() >= config.max_extension_headers {
                    return Err(CoreError::InvalidPacket("扩展头数量超过限制".into()));
                }

                let ext_bytes = packet.peek(2)
                    .ok_or_else(|| CoreError::parse_error("读取目的选项头失败"))?;

                let header = DestinationOptionsHeader::from_bytes(ext_bytes)?;
                let total_len = header.total_length();

                extension_chain_length += total_len;
                if extension_chain_length > config.max_extension_headers_length {
                    return Err(CoreError::InvalidPacket("扩展头链过长".into()));
                }

                let full_data = packet.read(total_len)
                    .ok_or_else(|| CoreError::parse_error("读取目的选项数据失败"))?;
                let options = full_data[2..].to_vec();

                next_header = IpProtocol::from(header.next_header);
                headers.push(ExtensionHeaderType::DestinationOptions {
                    header,
                    options,
                });
            }

            IpProtocol::Ipv6Route => {
                if headers.len() >= config.max_extension_headers {
                    return Err(CoreError::InvalidPacket("扩展头数量超过限制".into()));
                }

                let ext_bytes = packet.peek(4)
                    .ok_or_else(|| CoreError::parse_error("读取路由头失败"))?;

                let routing_header = RoutingHeader::from_bytes(ext_bytes)?;

                // 检查 Type 0（已废弃）
                if routing_header.is_type0() {
                    return Err(CoreError::UnsupportedProtocol(
                        "路由头 Type 0 已废弃 (RFC 5095)".into()
                    ));
                }

                let total_len = routing_header.total_length();

                extension_chain_length += total_len;
                if extension_chain_length > config.max_extension_headers_length {
                    return Err(CoreError::InvalidPacket("扩展头链过长".into()));
                }

                let data = packet.read(total_len)
                    .ok_or_else(|| CoreError::parse_error("读取路由头数据失败"))?;

                next_header = IpProtocol::from(routing_header.next_header);
                headers.push(ExtensionHeaderType::Routing {
                    header: routing_header,
                    data: data.to_vec(),
                });
            }

            IpProtocol::Ipv6Fragment => {
                if headers.len() >= config.max_extension_headers {
                    return Err(CoreError::InvalidPacket("扩展头数量超过限制".into()));
                }

                let ext_bytes = packet.peek(8)
                    .ok_or_else(|| CoreError::parse_error("读取分片头失败"))?;

                let frag_header = FragmentHeader::from_bytes(ext_bytes)?;

                // 检查原子分片
                if frag_header.is_atomic_fragment() && config.reject_atomic_fragments {
                    let id = frag_header.identification;
                    return Err(CoreError::InvalidPacket(
                        format!("原子分片被禁止: ID={}", id)
                    ));
                }

                extension_chain_length += 8;
                if extension_chain_length > config.max_extension_headers_length {
                    return Err(CoreError::InvalidPacket("扩展头链过长".into()));
                }

                // 读取并丢弃分片头
                packet.read(8)
                    .ok_or_else(|| CoreError::parse_error("读取分片头失败"))?;

                next_header = IpProtocol::from(frag_header.next_header);
                headers.push(ExtensionHeaderType::Fragment {
                    header: frag_header,
                });

                // 分片头后必须是最后一个扩展头
                break;
            }

            IpProtocol::Esp => {
                // ESP 处理（暂不支持）
                return Err(CoreError::UnsupportedProtocol("IPSec ESP 暂不支持".into()));
            }

            IpProtocol::Ah => {
                // AH 处理（暂不支持）
                return Err(CoreError::UnsupportedProtocol("IPSec AH 暂不支持".into()));
            }

            _ => {
                // 不是扩展头，退出循环
                break;
            }
        }
    }

    Ok(ExtensionChainResult {
        next_header,
        headers,
        chain_length: extension_chain_length,
    })
}

/// 扩展头链解析结果
#[derive(Debug, Clone)]
pub struct ExtensionChainResult {
    /// 最终的上层协议类型
    pub next_header: IpProtocol,
    /// 所有解析的扩展头
    pub headers: Vec<ExtensionHeaderType>,
    /// 扩展头链的总长度
    pub chain_length: usize,
}

// --- 扩展头配置 ---

/// 扩展头处理配置
#[derive(Debug, Clone)]
pub struct ExtensionConfig {
    /// 最大扩展头数量
    pub max_extension_headers: usize,

    /// 最大扩展头链长度
    pub max_extension_headers_length: usize,

    /// 是否处理逐跳选项
    pub process_hop_by_hop: bool,

    /// 是否处理目的选项
    pub process_destination_options: bool,

    /// 是否接受路由头
    pub accept_routing_header: bool,

    /// 是否支持分片
    pub enable_fragmentation: bool,

    /// 是否拒绝原子分片
    pub reject_atomic_fragments: bool,

    /// 是否验证所有长度
    pub verify_all_lengths: bool,
}

impl Default for ExtensionConfig {
    fn default() -> Self {
        ExtensionConfig {
            max_extension_headers: DEFAULT_MAX_EXTENSION_HEADERS,
            max_extension_headers_length: DEFAULT_MAX_EXTENSION_HEADERS_LENGTH,
            process_hop_by_hop: true,
            process_destination_options: true,
            accept_routing_header: false,
            enable_fragmentation: false,
            reject_atomic_fragments: true,
            verify_all_lengths: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extension_header_total_length() {
        // header_length = 0 表示总长度为 8 字节
        let ext = ExtensionHeader {
            next_header: 58,
            header_length: 0,
        };
        assert_eq!(ext.total_length(), 8);

        // header_length = 1 表示总长度为 16 字节
        let ext = ExtensionHeader {
            next_header: 58,
            header_length: 1,
        };
        assert_eq!(ext.total_length(), 16);
    }

    #[test]
    fn test_fragment_header() {
        let frag = FragmentHeader::new(58, 0, true, 12345);

        assert_eq!(frag.next_header, 58);
        assert_eq!(frag.fragment_offset(), 0);
        assert!(frag.more_fragments());
        assert!(!frag.is_atomic_fragment());
        let identification = frag.identification;
        assert_eq!(identification, 12345);
    }

    #[test]
    fn test_atomic_fragment_detection() {
        // 原子分片：offset=0, M=0
        let frag = FragmentHeader::new(58, 0, false, 12345);
        assert!(frag.is_atomic_fragment());

        // 非原子分片
        let frag = FragmentHeader::new(58, 0, true, 12345);
        assert!(!frag.is_atomic_fragment());

        let frag = FragmentHeader::new(58, 8, false, 12345);
        assert!(!frag.is_atomic_fragment());
    }

    #[test]
    fn test_routing_header_type0_detection() {
        let routing = RoutingHeader::new(58, 0, 0, 0);
        assert!(routing.is_type0());

        let routing = RoutingHeader::new(58, 2, 0, 0);
        assert!(!routing.is_type0());
    }

    #[test]
    fn test_fragment_header_serialize() {
        let frag = FragmentHeader::new(58, 123, true, 0xABCDEF01);
        let bytes = frag.to_bytes();

        assert_eq!(bytes[0], 58); // next_header
        assert_eq!(bytes[1], 0); // reserved

        // 检查 offset_res_m
        let offset_res_m = u16::from_be_bytes([bytes[2], bytes[3]]);
        assert_eq!(offset_res_m >> 3, 123); // offset
        assert_eq!(offset_res_m & 0x01, 1); // M flag

        // 检查 identification
        let id = u32::from_be_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]);
        assert_eq!(id, 0xABCDEF01);
    }

    #[test]
    fn test_extension_config_default() {
        let config = ExtensionConfig::default();
        assert_eq!(config.max_extension_headers, 8);
        assert_eq!(config.max_extension_headers_length, 2048);
        assert!(config.process_hop_by_hop);
        assert!(config.reject_atomic_fragments);
        assert!(!config.enable_fragmentation);
    }
}
