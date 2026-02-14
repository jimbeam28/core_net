/// 系统配置结构，定义资源创建参数
#[derive(Debug, Clone, PartialEq)]
pub struct SystemConfig {
    /// 接收队列容量
    pub rxq_capacity: usize,

    /// 发送队列容量
    pub txq_capacity: usize,
}

impl Default for SystemConfig {
    fn default() -> Self {
        Self {
            rxq_capacity: 256,
            txq_capacity: 256,
        }
    }
}
