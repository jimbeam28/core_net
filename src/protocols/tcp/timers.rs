// src/protocols/tcp/timers.rs
//
// TCP 定时器实现
// 包括重传定时器、TimeWait 定时器和 Keepalive 定时器

use std::time::{Duration, Instant};
use std::collections::VecDeque;

/// TCP 定时器类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimerType {
    /// 重传定时器
    Retransmission,
    /// TimeWait 定时器（2MSL）
    TimeWait,
    /// Keepalive 定时器
    Keepalive,
    /// 延迟 ACK 定时器
    DelayedAck,
}

/// 定时器事件
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TimerEvent {
    /// 定时器类型
    pub timer_type: TimerType,
    /// 连接 ID
    pub connection_id: u64,
}

/// TCP 定时器管理器
///
/// 负责管理所有 TCP 连接的定时器。
#[derive(Debug)]
pub struct TcpTimerManager {
    /// 定时器队列：(过期时间, 事件)
    timers: VecDeque<(Instant, TimerEvent)>,
}

impl TcpTimerManager {
    /// 创建新的定时器管理器
    pub fn new() -> Self {
        Self {
            timers: VecDeque::new(),
        }
    }

    /// 添加定时器
    ///
    /// # 参数
    /// - delay: 延迟时间
    /// - timer_type: 定时器类型
    /// - connection_id: 连接 ID
    ///
    /// # 返回
    /// - Instant: 定时器过期时间
    pub fn add_timer(&mut self, delay: Duration, timer_type: TimerType, connection_id: u64) -> Instant {
        let expire_time = Instant::now() + delay;
        let event = TimerEvent {
            timer_type,
            connection_id,
        };

        // 按过期时间插入（保持有序）
        let mut inserted = false;
        for i in 0..self.timers.len() {
            if self.timers[i].0 > expire_time {
                self.timers.insert(i, (expire_time, event));
                inserted = true;
                break;
            }
        }

        if !inserted {
            self.timers.push_back((expire_time, event));
        }

        expire_time
    }

    /// 取消指定连接的所有定时器
    ///
    /// # 参数
    /// - connection_id: 连接 ID
    pub fn cancel_timers(&mut self, connection_id: u64) {
        self.timers.retain(|(_, event)| event.connection_id != connection_id);
    }

    /// 取消指定连接的特定类型定时器
    ///
    /// # 参数
    /// - connection_id: 连接 ID
    /// - timer_type: 定时器类型
    pub fn cancel_timer_type(&mut self, connection_id: u64, timer_type: TimerType) {
        self.timers.retain(|(_, event)| {
            !(event.connection_id == connection_id && event.timer_type == timer_type)
        });
    }

    /// 获取最近的定时器过期时间
    ///
    /// # 返回
    /// - Some(Duration): 距离最近定时器过期的时间
    /// - None: 没有活动定时器
    pub fn next_timeout(&self) -> Option<Duration> {
        self.timers.front().map(|(expire_time, _)| {
            let now = Instant::now();
            if *expire_time > now {
                *expire_time - now
            } else {
                Duration::from_secs(0)
            }
        })
    }

    /// 检查并获取已过期的定时器
    ///
    /// # 返回
    /// - Vec<TimerEvent>: 所有已过期的定时器事件
    pub fn check_timeouts(&mut self) -> Vec<TimerEvent> {
        let now = Instant::now();
        let mut expired = Vec::new();

        while let Some((expire_time, event)) = self.timers.front() {
            if *expire_time <= now {
                expired.push(*event);
                self.timers.pop_front();
            } else {
                break;
            }
        }

        expired
    }

    /// 获取活动定时器数量
    pub fn active_count(&self) -> usize {
        self.timers.len()
    }

    /// 清空所有定时器
    pub fn clear(&mut self) {
        self.timers.clear();
    }
}

impl Default for TcpTimerManager {
    fn default() -> Self {
        Self::new()
    }
}

/// TCP 定时器配置
#[derive(Debug, Clone, Copy)]
pub struct TcpTimerConfig {
    /// 初始重传超时时间（毫秒）
    pub initial_rto: u32,
    /// 最小重传超时时间（毫秒）
    pub min_rto: u32,
    /// 最大重传超时时间（毫秒）
    pub max_rto: u32,
    /// 最大重传次数
    pub max_retransmit: u32,
    /// TimeWait 状态持续时间（2MSL，毫秒）
    pub time_wait_msl: u32,
    /// Keepalive 间隔时间（秒）
    pub keepalive_interval: u32,
    /// Keepalive 探测次数
    pub keepalive_probes: u32,
}

impl Default for TcpTimerConfig {
    fn default() -> Self {
        Self {
            initial_rto: 1000,      // 1 秒
            min_rto: 200,           // 200 毫秒
            max_rto: 120000,        // 120 秒
            max_retransmit: 12,     // 12 次
            time_wait_msl: 1000,    // 1 秒（2MSL = 2 秒）
            keepalive_interval: 7200, // 2 小时
            keepalive_probes: 9,     // 9 次
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_timer_manager_add() {
        let mut manager = TcpTimerManager::new();
        let delay = Duration::from_millis(100);

        let expire = manager.add_timer(delay, TimerType::Retransmission, 1);
        assert_eq!(manager.active_count(), 1);

        // 验证过期时间大致正确（允许一些误差）
        let now = Instant::now();
        let time_until = expire.saturating_duration_since(now);
        assert!(time_until <= delay);
    }

    #[test]
    fn test_timer_manager_cancel() {
        let mut manager = TcpTimerManager::new();

        manager.add_timer(Duration::from_millis(100), TimerType::Retransmission, 1);
        manager.add_timer(Duration::from_millis(200), TimerType::TimeWait, 1);
        manager.add_timer(Duration::from_millis(300), TimerType::Retransmission, 2);

        assert_eq!(manager.active_count(), 3);

        // 取消连接 1 的所有定时器
        manager.cancel_timers(1);
        assert_eq!(manager.active_count(), 1);

        // 取消特定类型的定时器
        manager.cancel_timer_type(2, TimerType::Retransmission);
        assert_eq!(manager.active_count(), 0);
    }

    #[test]
    fn test_timer_manager_next_timeout() {
        let mut manager = TcpTimerManager::new();

        // 没有定时器时返回 None
        assert!(manager.next_timeout().is_none());

        manager.add_timer(Duration::from_millis(100), TimerType::Retransmission, 1);
        assert!(manager.next_timeout().is_some());
    }

    #[test]
    fn test_timer_manager_ordering() {
        let mut manager = TcpTimerManager::new();

        // 按非顺序添加定时器
        manager.add_timer(Duration::from_millis(300), TimerType::Retransmission, 3);
        manager.add_timer(Duration::from_millis(100), TimerType::Retransmission, 1);
        manager.add_timer(Duration::from_millis(200), TimerType::Retransmission, 2);

        // 验证顺序
        assert_eq!(manager.active_count(), 3);

        // 第一个应该是连接 1（100ms）
        if let Some((_, event)) = manager.timers.front() {
            assert_eq!(event.connection_id, 1);
        }
    }

    #[test]
    fn test_timer_config_default() {
        let config = TcpTimerConfig::default();
        assert_eq!(config.initial_rto, 1000);
        assert_eq!(config.min_rto, 200);
        assert_eq!(config.max_rto, 120000);
        assert_eq!(config.max_retransmit, 12);
        assert_eq!(config.time_wait_msl, 1000);
        assert_eq!(config.keepalive_interval, 7200);
        assert_eq!(config.keepalive_probes, 9);
    }
}
