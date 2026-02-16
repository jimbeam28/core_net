use std::fs;

use crate::common::AddrError;
use crate::interface::iface::InterfaceConfig;
use crate::interface::manager::InterfaceManager;
use crate::interface::types::{InterfaceError, InterfaceState, Ipv4Addr, MacAddr};

/// 接口模块默认配置文件路径
pub const DEFAULT_CONFIG_PATH: &str = "src/interface/interface.toml";

/// 接口模块配置（包含队列配置和接口列表）
#[derive(Debug, Clone)]
pub struct InterfaceModuleConfig {
    /// 接收队列容量
    pub rxq_capacity: usize,
    /// 发送队列容量
    pub txq_capacity: usize,
    /// 接口配置列表
    pub interfaces: Vec<InterfaceConfig>,
}

impl Default for InterfaceModuleConfig {
    fn default() -> Self {
        Self {
            rxq_capacity: 256,
            txq_capacity: 256,
            interfaces: Vec::new(),
        }
    }
}

/// 从默认配置文件加载接口
///
/// 从默认配置文件路径 (src/interface/interface.toml) 加载接口配置
///
/// # 返回
/// - Ok(manager): 加载成功的接口管理器
/// - Err(error): 加载失败
///
/// # 配置文件格式 (TOML)
/// ```toml
/// [queue]
/// rxq_capacity = 256
/// txq_capacity = 256
///
/// [[interfaces]]
/// name = "eth0"
/// mac_addr = "00:11:22:33:44:55"
/// ip_addr = "192.168.1.100"
/// netmask = "255.255.255.0"
/// gateway = "192.168.1.1"
/// mtu = 1500
/// state = "Up"
/// ```
pub fn load_default_config() -> Result<InterfaceManager, InterfaceError> {
    let content = fs::read_to_string(DEFAULT_CONFIG_PATH).map_err(|e| {
        InterfaceError::ConfigReadFailed(format!("读取文件失败: {}", e))
    })?;

    let module_config = parse_toml_config(&content)?;

    let mut manager = InterfaceManager::new(module_config.rxq_capacity, module_config.txq_capacity);

    for config in module_config.interfaces {
        manager.add_from_config(config)?;
    }

    Ok(manager)
}

/// 保存配置到文件
///
/// # 参数
/// - manager: 接口管理器
/// - path: 配置文件路径
/// - rxq_capacity: 接收队列容量
/// - txq_capacity: 发送队列容量
pub fn save_config(
    manager: &InterfaceManager,
    path: &str,
    rxq_capacity: usize,
    txq_capacity: usize,
) -> Result<(), InterfaceError> {
    let mut content = String::new();

    // 写入队列配置
    content.push_str("# 队列配置\n");
    content.push_str("[queue]\n");
    content.push_str(&format!("rxq_capacity = {}\n", rxq_capacity));
    content.push_str(&format!("txq_capacity = {}\n\n", txq_capacity));

    // 写入接口配置
    content.push_str("# 网络接口配置\n");
    for iface in manager.interfaces() {
        content.push_str("[[interfaces]]\n");
        content.push_str(&format!("name = \"{}\"\n", iface.name()));
        content.push_str(&format!("mac_addr = \"{}\"\n", iface.mac_addr));
        content.push_str(&format!("ip_addr = \"{}\"\n", iface.ip_addr));
        content.push_str(&format!("netmask = \"{}\"\n", iface.netmask));
        if let Some(gateway) = iface.gateway {
            content.push_str(&format!("gateway = \"{}\"\n", gateway));
        }
        content.push_str(&format!("mtu = {}\n", iface.mtu));
        content.push_str(&format!("state = \"{:?}\"\n\n", iface.state));
    }

    fs::write(path, content)
        .map_err(|e| InterfaceError::ConfigWriteFailed(format!("写入文件失败: {}", e)))
}

/// 解析 TOML 格式的完整配置（包含队列配置和接口列表）
fn parse_toml_config(content: &str) -> Result<InterfaceModuleConfig, InterfaceError> {
    let mut config = InterfaceModuleConfig::default();
    let mut current_interface: Option<InterfaceConfig> = None;
    let mut in_interfaces_section = false;

    for line in content.lines() {
        let line = line.trim();

        // 跳过空行和注释
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        // 检查是否是 [queue] 部分
        if line == "[queue]" {
            in_interfaces_section = false;
            continue;
        }

        // 检查是否是 [[interfaces]] 部分
        if line.starts_with("[[interfaces]]") {
            in_interfaces_section = true;
            if let Some(iface) = current_interface.take() {
                config.interfaces.push(iface);
            }
            current_interface = Some(InterfaceConfig {
                name: String::new(),
                mac_addr: MacAddr::zero(),
                ip_addr: Ipv4Addr::unspecified(),
                netmask: Ipv4Addr::new(255, 255, 255, 0),
                gateway: None,
                mtu: None,
                state: None,
            });
            continue;
        }

        // 解析键值对
        if let Some((key, value)) = parse_key_value(line) {
            if in_interfaces_section {
                // 接口配置
                if let Some(ref mut iface) = current_interface {
                    match key.as_str() {
                        "name" => {
                            iface.name = value;
                        }
                        "mac_addr" => {
                            iface.mac_addr = value.parse().map_err(|e: AddrError| match e {
                                AddrError::InvalidMacAddr(s) => InterfaceError::InvalidMacAddr(s),
                                AddrError::InvalidIpAddr(s) => InterfaceError::InvalidIpAddr(s),
                            })?;
                        }
                        "ip_addr" => {
                            iface.ip_addr = value.parse().map_err(|e: AddrError| match e {
                                AddrError::InvalidMacAddr(s) => InterfaceError::InvalidMacAddr(s),
                                AddrError::InvalidIpAddr(s) => InterfaceError::InvalidIpAddr(s),
                            })?;
                        }
                        "netmask" => {
                            iface.netmask = value.parse().map_err(|e: AddrError| match e {
                                AddrError::InvalidMacAddr(s) => InterfaceError::InvalidMacAddr(s),
                                AddrError::InvalidIpAddr(s) => InterfaceError::InvalidIpAddr(s),
                            })?;
                        }
                        "gateway" => {
                            iface.gateway = Some(value.parse().map_err(|e: AddrError| match e {
                                AddrError::InvalidMacAddr(s) => InterfaceError::InvalidMacAddr(s),
                                AddrError::InvalidIpAddr(s) => InterfaceError::InvalidIpAddr(s),
                            })?);
                        }
                        "mtu" => {
                            iface.mtu = Some(
                                value
                                    .parse::<u16>()
                                    .map_err(|_| InterfaceError::InvalidMtu(0))?,
                            );
                        }
                        "state" => {
                            iface.state = Some(parse_state(&value)?);
                        }
                        _ => {
                            return Err(InterfaceError::InvalidFormat(format!(
                                "未知字段: {}",
                                key
                            )))
                        }
                    }
                }
            } else {
                // 队列配置
                match key.as_str() {
                    "rxq_capacity" => {
                        config.rxq_capacity = value
                            .parse::<usize>()
                            .map_err(|_| InterfaceError::InvalidFormat(format!(
                                "无效的 rxq_capacity: {}",
                                value
                            )))?;
                    }
                    "txq_capacity" => {
                        config.txq_capacity = value
                            .parse::<usize>()
                            .map_err(|_| InterfaceError::InvalidFormat(format!(
                                "无效的 txq_capacity: {}",
                                value
                            )))?;
                    }
                    _ => {
                        // 忽略其他字段
                    }
                }
            }
        }
    }

    // 添加最后一个接口
    if let Some(iface) = current_interface {
        config.interfaces.push(iface);
    }

    Ok(config)
}

/// 解析键值对
fn parse_key_value(line: &str) -> Option<(String, String)> {
    let parts: Vec<&str> = line.splitn(2, '=').collect();
    if parts.len() != 2 {
        return None;
    }

    let key = parts[0].trim().to_string();
    let value = parts[1].trim().trim_matches('"').to_string();
    Some((key, value))
}

/// 解析状态字符串
fn parse_state(s: &str) -> Result<InterfaceState, InterfaceError> {
    match s {
        "Up" => Ok(InterfaceState::Up),
        "Down" => Ok(InterfaceState::Down),
        "Testing" => Ok(InterfaceState::Testing),
        "Error" => Ok(InterfaceState::Error),
        _ => Err(InterfaceError::InvalidFormat(format!("无效的接口状态: {}", s))),
    }
}
