#[allow(unused_imports)]
use std::env;
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

#[cfg(test)]
mod tests {
    use super::*;

    // ========== parse_key_value 测试 ==========

    // ========== 测试辅助函数 ==========

    /// 创建测试用的 TOML 配置内容
    fn create_test_toml() -> String {
        r#"# 队列配置
[queue]
rxq_capacity = 256
txq_capacity = 512

# 网络接口配置
[[interfaces]]
name = "eth0"
mac_addr = "00:11:22:33:44:55"
ip_addr = "192.168.1.100"
netmask = "255.255.255.0"
gateway = "192.168.1.1"
mtu = 1500
state = "Up"

[[interfaces]]
name = "lo"
mac_addr = "00:00:00:00:00:00"
ip_addr = "127.0.0.1"
netmask = "255.0.0.0"
state = "Up"
"#.to_string()
    }

    /// 创建最小化的测试配置
    fn create_minimal_toml() -> String {
        r#"[queue]
rxq_capacity = 128
txq_capacity = 256

[[interfaces]]
name = "eth0"
mac_addr = "00:11:22:33:44:55"
ip_addr = "10.0.0.1"
netmask = "255.255.255.0"
"#.to_string()
    }

    // ========== parse_key_value 测试 ==========

    #[test]
    fn test_parse_key_value_valid() {
        let result = parse_key_value("name = \"eth0\"");
        assert!(result.is_some());
        let (key, value) = result.unwrap();
        assert_eq!(key, "name");
        assert_eq!(value, "eth0");
    }

    #[test]
    fn test_parse_key_value_with_spaces() {
        let result = parse_key_value("  name  =  \"eth0\"  ");
        assert!(result.is_some());
        let (key, value) = result.unwrap();
        assert_eq!(key, "name");
        assert_eq!(value, "eth0");
    }

    #[test]
    fn test_parse_key_value_no_quotes() {
        let result = parse_key_value("mtu = 1500");
        assert!(result.is_some());
        let (key, value) = result.unwrap();
        assert_eq!(key, "mtu");
        assert_eq!(value, "1500");
    }

    #[test]
    fn test_parse_key_value_invalid() {
        // 没有等号
        assert!(parse_key_value("name").is_none());
        // 空字符串
        assert!(parse_key_value("").is_none());
        // 只有等号返回空键值对（当前实现行为）
        let result = parse_key_value("=");
        assert!(result.is_some());
        assert_eq!(result.unwrap(), (String::new(), String::new()));
    }

    // ========== parse_state 测试 ==========

    #[test]
    fn test_parse_state_valid() {
        assert_eq!(parse_state("Up").unwrap(), InterfaceState::Up);
        assert_eq!(parse_state("Down").unwrap(), InterfaceState::Down);
        assert_eq!(parse_state("Testing").unwrap(), InterfaceState::Testing);
        assert_eq!(parse_state("Error").unwrap(), InterfaceState::Error);
    }

    #[test]
    fn test_parse_state_invalid() {
        assert!(parse_state("Invalid").is_err());
        assert!(parse_state("up").is_err()); // 大小写敏感
        assert!(parse_state("").is_err());
    }

    // ========== parse_toml_config 测试 ==========

    #[test]
    fn test_parse_toml_config_full() {
        let content = create_test_toml();
        let config = parse_toml_config(&content).unwrap();

        assert_eq!(config.rxq_capacity, 256);
        assert_eq!(config.txq_capacity, 512);
        assert_eq!(config.interfaces.len(), 2);

        // 验证第一个接口
        let iface0 = &config.interfaces[0];
        assert_eq!(iface0.name, "eth0");
        assert_eq!(iface0.mac_addr, MacAddr::new([0x00, 0x11, 0x22, 0x33, 0x44, 0x55]));
        assert_eq!(iface0.ip_addr, Ipv4Addr::new(192, 168, 1, 100));
        assert_eq!(iface0.netmask, Ipv4Addr::new(255, 255, 255, 0));
        assert_eq!(iface0.gateway, Some(Ipv4Addr::new(192, 168, 1, 1)));
        assert_eq!(iface0.mtu, Some(1500));
        assert_eq!(iface0.state, Some(InterfaceState::Up));

        // 验证第二个接口
        let iface1 = &config.interfaces[1];
        assert_eq!(iface1.name, "lo");
        assert_eq!(iface1.mac_addr, MacAddr::zero());
        assert_eq!(iface1.ip_addr, Ipv4Addr::new(127, 0, 0, 1));
        assert_eq!(iface1.netmask, Ipv4Addr::new(255, 0, 0, 0));
        assert_eq!(iface1.gateway, None);
        assert_eq!(iface1.mtu, None);
        assert_eq!(iface1.state, Some(InterfaceState::Up));
    }

    #[test]
    fn test_parse_toml_config_minimal() {
        let content = create_minimal_toml();
        let config = parse_toml_config(&content).unwrap();

        assert_eq!(config.rxq_capacity, 128);
        assert_eq!(config.txq_capacity, 256);
        assert_eq!(config.interfaces.len(), 1);

        let iface = &config.interfaces[0];
        assert_eq!(iface.name, "eth0");
        assert_eq!(iface.gateway, None);
        assert_eq!(iface.mtu, None);
        assert_eq!(iface.state, None);
    }

    #[test]
    fn test_parse_toml_config_with_comments() {
        // 注意：简单 TOML 解析器不支持行内注释，只支持整行注释
        let content = r#"
# 这是一个注释
[queue]
rxq_capacity = 256
txq_capacity = 256

# 另一个注释
[[interfaces]]
name = "eth0"
mac_addr = "00:11:22:33:44:55"
ip_addr = "192.168.1.1"
netmask = "255.255.255.0"
"#.to_string();

        let config = parse_toml_config(&content).unwrap();
        assert_eq!(config.interfaces.len(), 1);
    }

    #[test]
    fn test_parse_toml_config_empty_sections() {
        let content = r#"
[queue]
rxq_capacity = 256
txq_capacity = 256

[[interfaces]]
name = "eth0"
mac_addr = "00:11:22:33:44:55"
ip_addr = "192.168.1.1"
netmask = "255.255.255.0"

"#.to_string();

        let config = parse_toml_config(&content).unwrap();
        assert_eq!(config.interfaces.len(), 1);
    }

    #[test]
    fn test_parse_toml_config_invalid_mac() {
        let content = r#"
[[interfaces]]
name = "eth0"
mac_addr = "invalid"
ip_addr = "192.168.1.1"
netmask = "255.255.255.0"
"#.to_string();

        let result = parse_toml_config(&content);
        assert!(result.is_err());
        match result {
            Err(InterfaceError::InvalidMacAddr(_)) => {}
            _ => panic!("Expected InvalidMacAddr error"),
        }
    }

    #[test]
    fn test_parse_toml_config_invalid_ip() {
        let content = r#"
[[interfaces]]
name = "eth0"
mac_addr = "00:11:22:33:44:55"
ip_addr = "invalid"
netmask = "255.255.255.0"
"#.to_string();

        let result = parse_toml_config(&content);
        assert!(result.is_err());
        match result {
            Err(InterfaceError::InvalidIpAddr(_)) => {}
            _ => panic!("Expected InvalidIpAddr error"),
        }
    }

    #[test]
    fn test_parse_toml_config_invalid_mtu() {
        let content = r#"
[[interfaces]]
name = "eth0"
mac_addr = "00:11:22:33:44:55"
ip_addr = "192.168.1.1"
netmask = "255.255.255.0"
mtu = "invalid"
"#.to_string();

        let result = parse_toml_config(&content);
        assert!(result.is_err());
        match result {
            Err(InterfaceError::InvalidMtu(_)) => {}
            _ => panic!("Expected InvalidMtu error"),
        }
    }

    #[test]
    fn test_parse_toml_config_invalid_state() {
        let content = r#"
[[interfaces]]
name = "eth0"
mac_addr = "00:11:22:33:44:55"
ip_addr = "192.168.1.1"
netmask = "255.255.255.0"
state = "InvalidState"
"#.to_string();

        let result = parse_toml_config(&content);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_toml_config_invalid_queue_capacity() {
        let content = r#"
[queue]
rxq_capacity = invalid
txq_capacity = 256
"#.to_string();

        let result = parse_toml_config(&content);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_toml_config_unknown_field() {
        let content = r#"
[[interfaces]]
name = "eth0"
mac_addr = "00:11:22:33:44:55"
ip_addr = "192.168.1.1"
netmask = "255.255.255.0"
unknown_field = "value"
"#.to_string();

        let result = parse_toml_config(&content);
        assert!(result.is_err());
        match result {
            Err(InterfaceError::InvalidFormat(msg)) => {
                assert!(msg.contains("未知字段") || msg.contains("unknown_field"));
            }
            _ => panic!("Expected InvalidFormat error"),
        }
    }

    // ========== save_config 测试 ==========

    #[test]
    fn test_save_config() {
        let mut manager = InterfaceManager::new(256, 512);

        // 添加测试接口
        let config1 = InterfaceConfig {
            name: "eth0".to_string(),
            mac_addr: MacAddr::new([0x00, 0x11, 0x22, 0x33, 0x44, 0x55]),
            ip_addr: Ipv4Addr::new(192, 168, 1, 100),
            netmask: Ipv4Addr::new(255, 255, 255, 0),
            gateway: Some(Ipv4Addr::new(192, 168, 1, 1)),
            mtu: Some(1500),
            state: Some(InterfaceState::Up),
        };
        manager.add_from_config(config1).unwrap();

        let config2 = InterfaceConfig {
            name: "lo".to_string(),
            mac_addr: MacAddr::zero(),
            ip_addr: Ipv4Addr::new(127, 0, 0, 1),
            netmask: Ipv4Addr::new(255, 0, 0, 0),
            gateway: None,
            mtu: None,
            state: Some(InterfaceState::Up),
        };
        manager.add_from_config(config2).unwrap();

        // 创建临时文件路径
        let temp_dir = env::temp_dir();
        let temp_file_path = temp_dir.join("test_interface_config.toml");

        // 保存配置
        let result = save_config(&manager, temp_file_path.to_str().unwrap(), 256, 512);
        assert!(result.is_ok());

        // 读取保存的内容
        let saved_content = fs::read_to_string(&temp_file_path).unwrap();

        // 验证内容包含关键字段
        assert!(saved_content.contains("[queue]"));
        assert!(saved_content.contains("rxq_capacity = 256"));
        assert!(saved_content.contains("txq_capacity = 512"));
        assert!(saved_content.contains("[[interfaces]]"));
        assert!(saved_content.contains("name = \"eth0\""));
        assert!(saved_content.contains("name = \"lo\""));
        assert!(saved_content.contains("00:11:22:33:44:55"));
        assert!(saved_content.contains("192.168.1.100"));
        assert!(saved_content.contains("127.0.0.1"));

        // 清理临时文件
        let _ = fs::remove_file(&temp_file_path);
    }

    #[test]
    fn test_save_config_roundtrip() {
        // 创建原始管理器
        let original_toml = create_test_toml();
        let original_manager = {
            let module_config = parse_toml_config(&original_toml).unwrap();
            let mut mgr = InterfaceManager::new(module_config.rxq_capacity, module_config.txq_capacity);
            for config in module_config.interfaces {
                mgr.add_from_config(config).unwrap();
            }
            mgr
        };

        // 创建临时文件路径
        let temp_dir = env::temp_dir();
        let temp_file_path = temp_dir.join("test_interface_config_roundtrip.toml");

        // 保存配置
        save_config(&original_manager, temp_file_path.to_str().unwrap(), 256, 512).unwrap();

        // 重新加载
        let reloaded_content = fs::read_to_string(&temp_file_path).unwrap();
        let reloaded_config = parse_toml_config(&reloaded_content).unwrap();

        // 验证队列配置
        assert_eq!(reloaded_config.rxq_capacity, 256);
        assert_eq!(reloaded_config.txq_capacity, 512);

        // 验证接口数量
        assert_eq!(reloaded_config.interfaces.len(), 2);

        // 清理临时文件
        let _ = fs::remove_file(&temp_file_path);
    }

    // ========== InterfaceModuleConfig 测试 ==========

    #[test]
    fn test_interface_module_config_default() {
        let config = InterfaceModuleConfig::default();
        assert_eq!(config.rxq_capacity, 256);
        assert_eq!(config.txq_capacity, 256);
        assert!(config.interfaces.is_empty());
    }
}
