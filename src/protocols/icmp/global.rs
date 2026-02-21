// src/protocols/icmp/global.rs
//
// ICMP 全局状态管理
// 用于跟踪待处理的 Echo 请求（匹配请求和响应）

use std::collections::HashMap;
use std::time::{Duration, Instant};

use crate::protocols::Ipv4Addr;

// ========== 待处理 Echo 条目 ==========

/// 待处理的 Echo 请求条目
#[derive(Debug, Clone)]
pub struct PendingEcho {
    /// 标识符
    pub identifier: u16,

    /// 序列号
    pub sequence: u16,

    /// 发送时间戳
    pub sent_at: Instant,

    /// 目标地址
    pub destination: Ipv4Addr,
}

impl PendingEcho {
    /// 创建新的待处理 Echo 请求
    ///
    /// # 参数
    /// - identifier: 标识符
    /// - sequence: 序列号
    /// - destination: 目标地址
    ///
    /// # 返回
    /// - PendingEcho: 新的待处理 Echo 请求
    pub fn new(identifier: u16, sequence: u16, destination: Ipv4Addr) -> Self {
        PendingEcho {
            identifier,
            sequence,
            sent_at: Instant::now(),
            destination,
        }
    }

    /// 检查是否超时
    ///
    /// # 参数
    /// - timeout: 超时时间
    ///
    /// # 返回
    /// - bool: true 表示已超时
    pub fn is_timeout(&self, timeout: Duration) -> bool {
        self.sent_at.elapsed() >= timeout
    }

    /// 计算往返时间
    ///
    /// # 返回
    /// - Duration: 从发送到现在经过的时间
    pub fn rtt(&self) -> Duration {
        self.sent_at.elapsed()
    }
}

// ========== Echo 管理器 ==========

/// Echo 请求管理器
pub struct EchoManager {
    /// 待处理的 Echo 请求
    pending: HashMap<(u16, u16), PendingEcho>,

    /// 默认超时时间
    default_timeout: Duration,

    /// 最大待处理数量
    max_pending: usize,
}

impl EchoManager {
    /// 创建新的 Echo 管理器
    ///
    /// # 参数
    /// - default_timeout: 默认超时时间
    /// - max_pending: 最大待处理数量
    ///
    /// # 返回
    /// - EchoManager: 新的 Echo 管理器
    pub fn new(default_timeout: Duration, max_pending: usize) -> Self {
        Self {
            pending: HashMap::new(),
            default_timeout,
            max_pending,
        }
    }

    /// 添加待处理的 Echo 请求
    ///
    /// # 参数
    /// - echo: 待处理的 Echo 请求
    ///
    /// # 返回
    /// - Ok(()): 成功添加
    /// - Err(String): 管理器已满
    pub fn add_pending(&mut self, echo: PendingEcho) -> Result<(), String> {
        // 清理超时的条目
        self.cleanup_timeouts();

        // 检查容量
        if self.pending.len() >= self.max_pending {
            return Err(format!(
                "Echo管理器已满: {} >= {}",
                self.pending.len(),
                self.max_pending
            ));
        }

        let key = (echo.identifier, echo.sequence);
        self.pending.insert(key, echo);
        Ok(())
    }

    /// 查找并移除待处理的 Echo 请求
    ///
    /// # 参数
    /// - identifier: 标识符
    /// - sequence: 序列号
    ///
    /// # 返回
    /// - Option<PendingEcho>: 找到的 Echo 请求（如果存在）
    pub fn remove_pending(&mut self, identifier: u16, sequence: u16) -> Option<PendingEcho> {
        let key = (identifier, sequence);
        self.pending.remove(&key)
    }

    /// 查找待处理的 Echo 请求（不移除）
    ///
    /// # 参数
    /// - identifier: 标识符
    /// - sequence: 序列号
    ///
    /// # 返回
    /// - Option<&PendingEcho>: 找到的 Echo 请求引用（如果存在）
    pub fn get_pending(&self, identifier: u16, sequence: u16) -> Option<&PendingEcho> {
        let key = (identifier, sequence);
        self.pending.get(&key)
    }

    /// 清理超时的请求
    pub fn cleanup_timeouts(&mut self) {
        self.pending.retain(|_, echo| !echo.is_timeout(self.default_timeout));
    }

    /// 获取待处理数量
    ///
    /// # 返回
    /// - usize: 当前待处理的 Echo 请求数量
    pub fn pending_count(&self) -> usize {
        self.pending.len()
    }

    /// 清空所有待处理请求
    pub fn clear(&mut self) {
        self.pending.clear();
    }
}

impl Default for EchoManager {
    fn default() -> Self {
        Self::new(Duration::from_secs(1), 100)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_echo_manager_add_remove() {
        let mut manager = EchoManager::default();

        let echo = PendingEcho::new(1234, 1, Ipv4Addr::new(192, 168, 1, 1));
        manager.add_pending(echo.clone()).unwrap();

        assert_eq!(manager.pending_count(), 1);

        let removed = manager.remove_pending(1234, 1);
        assert!(removed.is_some());
        assert_eq!(manager.pending_count(), 0);
    }

    #[test]
    fn test_echo_manager_cleanup() {
        let mut manager = EchoManager::new(Duration::from_millis(100), 100);

        let echo = PendingEcho::new(1234, 1, Ipv4Addr::new(192, 168, 1, 1));
        manager.add_pending(echo).unwrap();

        std::thread::sleep(Duration::from_millis(150));
        manager.cleanup_timeouts();

        assert_eq!(manager.pending_count(), 0);
    }
}
