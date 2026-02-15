/// 系统配置结构，定义资源创建参数
#[derive(Debug, Clone, PartialEq)]
pub struct SystemConfig {
    /// 接口配置文件路径
    pub interface_config_path: String,

    /// 每个接口的接收队列容量
    pub rxq_capacity: usize,

    /// 每个接口的发送队列容量
    pub txq_capacity: usize,
}

impl Default for SystemConfig {
    fn default() -> Self {
        Self {
            rxq_capacity: 256,
            txq_capacity: 256,
            interface_config_path: "src/config/interface.toml".to_string(),
        }
    }
}
