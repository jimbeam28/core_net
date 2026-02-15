use std::fs;

use crate::common::AddrError;
use crate::interface::iface::InterfaceConfig;
use crate::interface::manager::InterfaceManager;
use crate::interface::types::{InterfaceError, InterfaceState, Ipv4Addr, MacAddr};

/// 从配置文件加载接口
///
/// # 参数
/// - path: 配置文件路径
/// - rxq_capacity: 每个接口的接收队列容量
/// - txq_capacity: 每个接口的发送队列容量
///
/// # 返回
/// - Ok(manager): 加载成功的接口管理器
/// - Err(error): 加载失败
///
/// # 配置文件格式 (TOML)
/// ```toml
/// [[interfaces]]
/// name = "eth0"
/// mac_addr = "00:11:22:33:44:55"
/// ip_addr = "192.168.1.100"
/// netmask = "255.255.255.0"
/// gateway = "192.168.1.1"
/// mtu = 1500
/// state = "Up"
/// ```
pub fn load_config(path: &str, rxq_capacity: usize, txq_capacity: usize) -> Result<InterfaceManager, InterfaceError> {
    let content = fs::read_to_string(path).map_err(|e| {
        InterfaceError::ConfigReadFailed(format!("读取文件失败: {}", e))
    })?;

    let mut manager = InterfaceManager::new(rxq_capacity, txq_capacity);
    let interfaces = parse_toml_interfaces(&content)?;

    for config in interfaces {
        manager.add_from_config(config)?;
    }

    Ok(manager)
}

/// 保存配置到文件
///
/// # 参数
/// - manager: 接口管理器
/// - path: 配置文件路径
pub fn save_config(manager: &InterfaceManager, path: &str) -> Result<(), InterfaceError> {
    let mut content = String::new();

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

/// 解析 TOML 格式的接口配置
fn parse_toml_interfaces(content: &str) -> Result<Vec<InterfaceConfig>, InterfaceError> {
    let mut configs = Vec::new();

    let mut current_config: Option<InterfaceConfig> = None;
    let mut in_interface_block = false;

    for line in content.lines() {
        let line = line.trim();

        // 跳过空行和注释
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        // 检查是否是新的接口块 [[interfaces]]
        if line.starts_with("[[interfaces]]") {
            if let Some(config) = current_config.take() {
                configs.push(config);
            }
            current_config = Some(InterfaceConfig {
                name: String::new(),
                mac_addr: MacAddr::zero(),
                ip_addr: Ipv4Addr::unspecified(),
                netmask: Ipv4Addr::new(255, 255, 255, 0),
                gateway: None,
                mtu: None,
                state: None,
            });
            in_interface_block = true;
            continue;
        }

        // 解析键值对
        if let Some(ref mut config) = current_config {
            if let Some((key, value)) = parse_key_value(line) {
                match key.as_str() {
                    "name" => {
                        config.name = value;
                    }
                    "mac_addr" => {
                        config.mac_addr = value.parse().map_err(|e: AddrError| match e {
                            AddrError::InvalidMacAddr(s) => InterfaceError::InvalidMacAddr(s),
                            AddrError::InvalidIpAddr(s) => InterfaceError::InvalidIpAddr(s),
                        })?;
                    }
                    "ip_addr" => {
                        config.ip_addr = value.parse().map_err(|e: AddrError| match e {
                            AddrError::InvalidMacAddr(s) => InterfaceError::InvalidMacAddr(s),
                            AddrError::InvalidIpAddr(s) => InterfaceError::InvalidIpAddr(s),
                        })?;
                    }
                    "netmask" => {
                        config.netmask = value.parse().map_err(|e: AddrError| match e {
                            AddrError::InvalidMacAddr(s) => InterfaceError::InvalidMacAddr(s),
                            AddrError::InvalidIpAddr(s) => InterfaceError::InvalidIpAddr(s),
                        })?;
                    }
                    "gateway" => {
                        config.gateway = Some(value.parse().map_err(|e: AddrError| match e {
                            AddrError::InvalidMacAddr(s) => InterfaceError::InvalidMacAddr(s),
                            AddrError::InvalidIpAddr(s) => InterfaceError::InvalidIpAddr(s),
                        })?);
                    }
                    "mtu" => {
                        config.mtu = Some(
                            value
                                .parse::<u16>()
                                .map_err(|_| InterfaceError::InvalidMtu(0))?,
                        );
                    }
                    "state" => {
                        config.state = Some(parse_state(&value)?);
                    }
                    _ => {
                        return Err(InterfaceError::InvalidFormat(format!(
                            "未知字段: {}",
                            key
                        )))
                    }
                }
            }
        }
    }

    // 添加最后一个配置
    if let Some(config) = current_config {
        configs.push(config);
    }

    Ok(configs)
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
