// src/protocols/ipv6/options.rs
//
// IPv6 扩展头选项处理

use crate::common::{CoreError, Result};

// --- 选项类型常量 ---

/// Pad1 选项类型
pub const OPTION_TYPE_PAD1: u8 = 0x00;

/// PadN 选项类型
pub const OPTION_TYPE_PADN: u8 = 0x01;

/// Router Alert 选项类型
pub const OPTION_TYPE_ROUTER_ALERT: u8 = 0x05;

/// Jumbo Payload 选项类型
pub const OPTION_TYPE_JUMBO_PAYLOAD: u8 = 0xC2;

// --- 选项类型 ---

/// 选项类型
///
/// IPv6 扩展头选项的类型字段编码。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OptionType(pub u8);

impl OptionType {
    /// 创建新的选项类型
    pub fn new(value: u8) -> Self {
        OptionType(value)
    }

    /// 获取选项类型值（低 5 位）
    pub fn option_type_value(&self) -> u8 {
        self.0 & 0x1F
    }

    /// 获取 Action 字段（高 2 位）
    ///
    /// Action 字段指示当节点无法识别选项时的行为：
    /// - 00: 跳过此选项，继续处理
    /// - 01: 丢弃数据包，不发送 ICMPv6
    /// - 10: 丢弃数据包，发送 ICMPv6 Parameter Problem
    /// - 11: 丢弃数据包，发送 ICMPv6 Parameter Problem（指向选项类型字段）
    pub fn action(&self) -> u8 {
        (self.0 >> 6) & 0x03
    }

    /// 获取 Change 位（第 5 位）
    ///
    /// C 位指示选项数据在传输过程中是否可能改变：
    /// - 0: 选项数据不能改变
    /// - 1: 选项数据可以改变
    pub fn change_flag(&self) -> bool {
        (self.0 & 0x20) != 0
    }

    /// 判断是否需要丢弃数据包
    pub fn should_discard(&self) -> bool {
        self.action() >= 1
    }

    /// 判断是否需要发送 ICMPv6 错误消息
    pub fn should_send_icmp(&self) -> bool {
        self.action() >= 2
    }

    /// Pad1 选项
    pub const PAD1: Self = Self(OPTION_TYPE_PAD1);

    /// PadN 选项
    pub const PADN: Self = Self(OPTION_TYPE_PADN);

    /// Router Alert 选项
    pub const ROUTER_ALERT: Self = Self(OPTION_TYPE_ROUTER_ALERT);

    /// 判断是否为 Pad1 选项
    pub fn is_pad1(&self) -> bool {
        self.0 == OPTION_TYPE_PAD1
    }

    /// 判断是否为 PadN 选项
    pub fn is_padn(&self) -> bool {
        self.option_type_value() == 0x01
    }

    /// 判断是否为填充选项（Pad1 或 PadN）
    pub fn is_padding(&self) -> bool {
        self.is_pad1() || self.is_padn()
    }
}

impl std::fmt::Display for OptionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.0 {
            OPTION_TYPE_PAD1 => write!(f, "Pad1"),
            OPTION_TYPE_PADN => write!(f, "PadN"),
            OPTION_TYPE_ROUTER_ALERT => write!(f, "RouterAlert"),
            _ => write!(f, "OptionType({:#04x})", self.0),
        }
    }
}

// --- 通用选项 ---

/// 通用选项结构
///
/// 所有选项都遵循这个通用格式（除了 Pad1）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Option {
    /// 选项类型
    pub option_type: OptionType,
    /// 选项数据
    pub data: Vec<u8>,
}

impl Option {
    /// 创建新的选项
    pub fn new(option_type: OptionType, data: Vec<u8>) -> Self {
        Option {
            option_type,
            data,
        }
    }

    /// 获取选项总长度（包括类型和长度字段）
    pub fn total_length(&self) -> usize {
        if self.option_type.is_pad1() {
            1
        } else {
            2 + self.data.len()
        }
    }

    /// 从字节流解析选项
    pub fn from_bytes(data: &[u8]) -> Result<(Option, usize)> {
        if data.is_empty() {
            return Err(CoreError::parse_error("选项数据为空"));
        }

        // 检查 Pad1 选项
        if data[0] == OPTION_TYPE_PAD1 {
            return Ok((Option {
                option_type: OptionType::PAD1,
                data: Vec::new(),
            }, 1));
        }

        // 其他选项需要至少 2 字节（类型 + 长度）
        if data.len() < 2 {
            return Err(CoreError::parse_error("选项数据不足"));
        }

        let option_type = OptionType(data[0]);
        let opt_data_len = data[1] as usize;

        // 检查数据长度
        if data.len() < 2 + opt_data_len {
            return Err(CoreError::parse_error("选项数据长度不足"));
        }

        let option_data = data[2..2 + opt_data_len].to_vec();

        Ok((Option {
            option_type,
            data: option_data,
        }, 2 + opt_data_len))
    }

    /// 序列化为字节数组
    pub fn to_bytes(&self) -> Vec<u8> {
        if self.option_type.is_pad1() {
            vec![OPTION_TYPE_PAD1]
        } else {
            let mut bytes = Vec::with_capacity(2 + self.data.len());
            bytes.push(self.option_type.0);
            bytes.push(self.data.len() as u8);
            bytes.extend_from_slice(&self.data);
            bytes
        }
    }
}

// --- Router Alert 选项 ---

/// Router Alert 选项数据
///
/// 用于通知路由器需要检查数据包。
#[repr(C, packed)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RouterAlertOption {
    /// 选项类型 = 5
    pub option_type: u8,
    /// 选项长度 = 2
    pub option_length: u8,
    /// Alert 值
    pub alert_value: u16,
}

impl RouterAlertOption {
    /// Router Alert 选项固定长度
    pub const LENGTH: usize = 4;

    /// 数据包需要路由器检查（MLDv1 或 RSVP）
    pub const ALERT_VALUE_MLD_OR_RSVP: u16 = 0;

    /// 数据包包含 MLDv1 消息
    pub const ALERT_VALUE_MLDV1: u16 = 1;

    /// 数据包包含 RSVP 消息
    pub const ALERT_VALUE_RSVP: u16 = 2;

    /// 数据包包含 Active Networks 消息
    pub const ALERT_VALUE_ACTIVE_NETWORKS: u16 = 3;

    /// 创建新的 Router Alert 选项
    pub fn new(alert_value: u16) -> Self {
        RouterAlertOption {
            option_type: OPTION_TYPE_ROUTER_ALERT,
            option_length: 2,
            alert_value,
        }
    }

    /// 从字节流解析
    pub fn from_bytes(data: &[u8]) -> Result<Self> {
        if data.len() < Self::LENGTH {
            return Err(CoreError::parse_error("Router Alert 选项数据不足"));
        }

        if data[0] != OPTION_TYPE_ROUTER_ALERT {
            return Err(CoreError::parse_error(
                format!("无效的 Router Alert 选项类型: {}", data[0])
            ));
        }

        if data[1] != 2 {
            return Err(CoreError::parse_error(
                format!("无效的 Router Alert 选项长度: {}", data[1])
            ));
        }

        let alert_value = u16::from_be_bytes([data[2], data[3]]);

        Ok(RouterAlertOption {
            option_type: data[0],
            option_length: data[1],
            alert_value,
        })
    }

    /// 序列化为字节数组
    pub fn to_bytes(&self) -> [u8; Self::LENGTH] {
        let mut bytes = [0u8; Self::LENGTH];
        bytes[0] = self.option_type;
        bytes[1] = self.option_length;

        let alert_bytes = self.alert_value.to_be_bytes();
        bytes[2] = alert_bytes[0];
        bytes[3] = alert_bytes[1];

        bytes
    }
}

// --- Jumbo Payload 选项 ---

/// Jumbo Payload 选项数据
///
/// 用于大于 65535 字节的负载（需要逐跳选项头）。
#[repr(C, packed)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct JumboPayloadOption {
    /// 选项类型 = 0xC2
    pub option_type: u8,
    /// 选项长度 = 4
    pub option_length: u8,
    /// Jumbo 长度（大于 65535）
    pub jumbo_length: u32,
}

impl JumboPayloadOption {
    /// Jumbo Payload 选项固定长度
    pub const LENGTH: usize = 6;

    /// 最小 Jumbo 长度
    pub const MIN_JUMBO_LENGTH: u32 = 65536;

    /// 创建新的 Jumbo Payload 选项
    pub fn new(jumbo_length: u32) -> Result<Self> {
        if jumbo_length < Self::MIN_JUMBO_LENGTH {
            return Err(CoreError::InvalidPacket(format!(
                "Jumbo 长度必须大于 65535: {}", jumbo_length
            )));
        }

        Ok(JumboPayloadOption {
            option_type: OPTION_TYPE_JUMBO_PAYLOAD,
            option_length: 4,
            jumbo_length,
        })
    }

    /// 从字节流解析
    pub fn from_bytes(data: &[u8]) -> Result<Self> {
        if data.len() < Self::LENGTH {
            return Err(CoreError::parse_error("Jumbo Payload 选项数据不足"));
        }

        if data[0] != OPTION_TYPE_JUMBO_PAYLOAD {
            return Err(CoreError::parse_error(
                format!("无效的 Jumbo Payload 选项类型: {}", data[0])
            ));
        }

        if data[1] != 4 {
            return Err(CoreError::parse_error(
                format!("无效的 Jumbo Payload 选项长度: {}", data[1])
            ));
        }

        let jumbo_length = u32::from_be_bytes([data[2], data[3], data[4], data[5]]);

        if jumbo_length < Self::MIN_JUMBO_LENGTH {
            return Err(CoreError::InvalidPacket(format!(
                "Jumbo 长度必须大于 65535: {}", jumbo_length
            )));
        }

        Ok(JumboPayloadOption {
            option_type: data[0],
            option_length: data[1],
            jumbo_length,
        })
    }

    /// 序列化为字节数组
    pub fn to_bytes(&self) -> [u8; Self::LENGTH] {
        let mut bytes = [0u8; Self::LENGTH];
        bytes[0] = self.option_type;
        bytes[1] = self.option_length;

        let length_bytes = self.jumbo_length.to_be_bytes();
        bytes[2] = length_bytes[0];
        bytes[3] = length_bytes[1];
        bytes[4] = length_bytes[2];
        bytes[5] = length_bytes[3];

        bytes
    }
}

// --- 选项解析 ---

/// 选项解析结果
#[derive(Debug, Clone, PartialEq)]
pub struct OptionsParseResult {
    /// 解析的选项列表
    pub options: Vec<Option>,
    /// 总解析长度（字节）
    pub total_length: usize,
}

/// 解析选项数据
///
/// 从扩展头的选项数据中解析所有选项。
pub fn parse_options(data: &[u8]) -> Result<OptionsParseResult> {
    let mut options = Vec::new();
    let mut offset = 0;

    while offset < data.len() {
        match Option::from_bytes(&data[offset..]) {
            Ok((option, length)) => {
                options.push(option);
                offset += length;
            }
            Err(_e) => {
                // 解析失败，返回已解析的部分
                return Ok(OptionsParseResult {
                    options,
                    total_length: offset,
                });
            }
        }
    }

    Ok(OptionsParseResult {
        options,
        total_length: offset,
    })
}

// --- PadN 选项 ---

/// 创建 PadN 选项（用于填充）
pub fn create_padn(length: usize) -> Option {
    // PadN 选项：类型=1, 长度字段, 填充数据
    // 总长度 = 2 + length
    let data_length = length.saturating_sub(2);
    let data = vec![0u8; data_length];

    Option {
        option_type: OptionType::PADN,
        data,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_option_type_action() {
        // Action = 00: 跳过选项
        let opt = OptionType(0x00);
        assert_eq!(opt.action(), 0);
        assert!(!opt.should_discard());

        // Action = 01: 丢弃数据包
        let opt = OptionType(0x40);
        assert_eq!(opt.action(), 1);
        assert!(opt.should_discard());
        assert!(!opt.should_send_icmp());

        // Action = 10: 丢弃并发送 ICMP
        let opt = OptionType(0x80);
        assert_eq!(opt.action(), 2);
        assert!(opt.should_discard());
        assert!(opt.should_send_icmp());

        // Action = 11: 丢弃并发送 ICMP（指向选项类型）
        let opt = OptionType(0xC0);
        assert_eq!(opt.action(), 3);
        assert!(opt.should_discard());
        assert!(opt.should_send_icmp());
    }

    #[test]
    fn test_option_type_change_flag() {
        // C = 0: 数据不能改变
        let opt = OptionType(0x00);
        assert!(!opt.change_flag());

        // C = 1: 数据可以改变
        let opt = OptionType(0x20);
        assert!(opt.change_flag());
    }

    #[test]
    fn test_pad1_option() {
        let opt = Option::new(OptionType::PAD1, Vec::new());
        assert!(opt.option_type.is_pad1());
        assert_eq!(opt.total_length(), 1);

        let bytes = opt.to_bytes();
        assert_eq!(bytes, vec![0x00]);
    }

    #[test]
    fn test_pad1_parse() {
        let data = vec![0x00];
        let (opt, length) = Option::from_bytes(&data).unwrap();

        assert!(opt.option_type.is_pad1());
        assert_eq!(length, 1);
    }

    #[test]
    fn test_router_alert_option() {
        let opt = RouterAlertOption::new(RouterAlertOption::ALERT_VALUE_MLDV1);

        assert_eq!(opt.option_type, OPTION_TYPE_ROUTER_ALERT);
        assert_eq!(opt.option_length, 2);
        let alert_value = opt.alert_value;
        assert_eq!(alert_value, RouterAlertOption::ALERT_VALUE_MLDV1);

        let bytes = opt.to_bytes();
        assert_eq!(bytes.len(), RouterAlertOption::LENGTH);
        assert_eq!(bytes[0], OPTION_TYPE_ROUTER_ALERT);
        assert_eq!(bytes[1], 2);

        let parsed = RouterAlertOption::from_bytes(&bytes).unwrap();
        let alert_value = parsed.alert_value;
        let opt_alert_value = opt.alert_value;
        assert_eq!(alert_value, opt_alert_value);
    }

    #[test]
    fn test_jumbo_payload_option() {
        let opt = JumboPayloadOption::new(100000).unwrap();

        assert_eq!(opt.option_type, OPTION_TYPE_JUMBO_PAYLOAD);
        assert_eq!(opt.option_length, 4);
        let jumbo_length = opt.jumbo_length;
        assert_eq!(jumbo_length, 100000);

        let bytes = opt.to_bytes();
        assert_eq!(bytes.len(), JumboPayloadOption::LENGTH);

        let parsed = JumboPayloadOption::from_bytes(&bytes).unwrap();
        let parsed_length = parsed.jumbo_length;
        assert_eq!(parsed_length, 100000);
    }

    #[test]
    fn test_jumbo_payload_too_small() {
        let result = JumboPayloadOption::new(65535);
        assert!(result.is_err());
    }

    #[test]
    fn test_create_padn() {
        let pad = create_padn(6); // 总长度 6 字节
        assert!(pad.option_type.is_padn());
        assert_eq!(pad.total_length(), 6);
        assert_eq!(pad.data.len(), 4); // 类型 + 长度 = 2 字节
    }

    #[test]
    fn test_parse_options() {
        let mut data = Vec::new();

        // Pad1
        data.push(0x00);

        // Router Alert
        data.extend_from_slice(&[0x05, 0x02, 0x00, 0x00]);

        // PadN
        data.extend_from_slice(&[0x01, 0x04, 0x00, 0x00, 0x00, 0x00]);

        let result = parse_options(&data).unwrap();

        assert_eq!(result.options.len(), 3);
        // Pad1 (1) + Router Alert (4) + PadN (6) = 11
        assert_eq!(result.total_length, 11);
    }

    #[test]
    fn test_option_padding_detection() {
        let pad1 = OptionType::PAD1;
        assert!(pad1.is_padding());

        let padn = OptionType::PADN;
        assert!(padn.is_padding());

        let router_alert = OptionType::ROUTER_ALERT;
        assert!(!router_alert.is_padding());
    }
}
