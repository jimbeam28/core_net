// src/common/timer.rs
//
// 通用定时器系统
// 为协议栈提供定时器支持，可用于ARP缓存老化、TCP重传等场景
//
// 设计原则：
// 1. 无依赖：只使用Rust标准库
// 2. 高效：使用最小堆实现，O(log n)的插入和删除
// 3. 通用：支持周期性定时和一次性定时
// 4. 线程安全：通过外部加锁保证线程安全（与SystemContext模式一致）

use std::collections::BinaryHeap;
use std::cmp::Ordering;
use std::time::{Instant, Duration};

/// 定时器ID
pub type TimerId = u64;

/// 定时器回调函数类型
/// 参数：定时器ID，用户数据
/// 返回：如果是周期性定时器，返回true表示继续，false表示停止
pub type TimerCallback = Box<dyn FnMut(TimerId, &mut dyn std::any::Any) -> bool>;

/// 定时器类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimerType {
    /// 一次性定时器，到期后自动删除
    OneShot,
    /// 周期性定时器，到期后重置
    Periodic,
}

/// 定时器条目
pub struct TimerEntry {
    /// 定时器ID
    pub id: TimerId,
    /// 到期时间
    pub expires_at: Instant,
    /// 定期间隔（仅用于周期性定时器）
    pub interval: Option<Duration>,
    /// 定时器类型
    pub timer_type: TimerType,
    /// 回调函数
    pub callback: TimerCallback,
    /// 用户数据（Box<dyn Any>）
    pub user_data: Box<dyn std::any::Any>,
}

/// 为了能在BinaryHeap中使用，需要实现Ord
/// 注意：我们希望在堆顶的是最先到期的定时器，所以顺序相反
impl Ord for TimerEntry {
    fn cmp(&self, other: &Self) -> Ordering {
        other.expires_at.cmp(&self.expires_at)
    }
}

impl PartialOrd for TimerEntry {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for TimerEntry {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id && self.expires_at == other.expires_at
    }
}

impl Eq for TimerEntry {}

/// 定时器管理器
pub struct TimerManager {
    /// 定时器堆（最小堆，按到期时间排序）
    timers: BinaryHeap<TimerEntry>,
    /// 下一个定时器ID
    next_id: TimerId,
    /// 当前时间（用于测试时模拟时间流逝）
    current_time: Option<Instant>,
}

impl TimerManager {
    /// 创建新的定时器管理器
    pub fn new() -> Self {
        TimerManager {
            timers: BinaryHeap::new(),
            next_id: 1,
            current_time: None,
        }
    }

    /// 添加一次性定时器
    ///
    /// # 参数
    /// - `delay`: 延迟时间
    /// - `callback`: 回调函数
    /// - `user_data`: 用户数据
    ///
    /// # 返回
    /// 定时器ID
    pub fn add_oneshot<F, T>(&mut self, delay: Duration, mut callback: F, user_data: T) -> TimerId
    where
        F: FnMut(TimerId, &mut T) + 'static,
        T: 'static,
    {
        let id = self.next_id;
        self.next_id += 1;

        let expires_at = self.now() + delay;

        // 包装回调函数，将dyn Any转换为具体类型
        let wrapped_callback: TimerCallback = Box::new(move |timer_id: TimerId, any_data: &mut dyn std::any::Any| {
            if let Some(data) = any_data.downcast_mut::<T>() {
                callback(timer_id, data);
            }
            false // 一次性定时器返回false
        });

        let entry = TimerEntry {
            id,
            expires_at,
            interval: None,
            timer_type: TimerType::OneShot,
            callback: wrapped_callback,
            user_data: Box::new(user_data),
        };

        self.timers.push(entry);
        id
    }

    /// 添加周期性定时器
    ///
    /// # 参数
    /// - `interval`: 定期间隔
    /// - `callback`: 回调函数，返回true继续，false停止
    /// - `user_data`: 用户数据
    ///
    /// # 返回
    /// 定时器ID
    pub fn add_periodic<F, T>(&mut self, interval: Duration, mut callback: F, user_data: T) -> TimerId
    where
        F: FnMut(TimerId, &mut T) -> bool + 'static,
        T: 'static,
    {
        let id = self.next_id;
        self.next_id += 1;

        let expires_at = self.now() + interval;

        // 包装回调函数
        let wrapped_callback: TimerCallback = Box::new(move |timer_id: TimerId, any_data: &mut dyn std::any::Any| {
            if let Some(data) = any_data.downcast_mut::<T>() {
                callback(timer_id, data)
            } else {
                false
            }
        });

        let entry = TimerEntry {
            id,
            expires_at,
            interval: Some(interval),
            timer_type: TimerType::Periodic,
            callback: wrapped_callback,
            user_data: Box::new(user_data),
        };

        self.timers.push(entry);
        id
    }

    /// 取消定时器
    ///
    /// # 参数
    /// - `timer_id`: 定时器ID
    ///
    /// # 返回
    /// true表示成功取消，false表示定时器不存在或已到期
    pub fn cancel(&mut self, timer_id: TimerId) -> bool {
        // 由于BinaryHeap不支持直接删除，我们需要重建堆
        let mut found = false;
        let mut new_timers: BinaryHeap<TimerEntry> = BinaryHeap::new();

        while let Some(entry) = self.timers.pop() {
            if entry.id == timer_id {
                found = true;
                // 不加入新堆，相当于删除
            } else {
                new_timers.push(entry);
            }
        }

        self.timers = new_timers;
        found
    }

    /// 处理到期的定时器
    ///
    /// # 返回
    /// 处理的定时器数量
    pub fn process_expired(&mut self) -> usize {
        let now = self.now();
        let mut count = 0;
        let mut reinsert = Vec::new();

        while let Some(entry) = self.timers.peek() {
            // 检查最早的定时器是否到期
            let should_pop = entry.expires_at <= now;
            if !should_pop {
                break;
            }

            // 弹出到期的定时器
            let mut expired_entry = self.timers.pop().unwrap();

            // 执行回调
            let should_continue = (expired_entry.callback)(
                expired_entry.id,
                &mut *expired_entry.user_data
            );

            count += 1;

            // 如果是周期性定时器且回调返回true，重新加入队列
            if expired_entry.timer_type == TimerType::Periodic
                && should_continue
                && let Some(interval) = expired_entry.interval
            {
                expired_entry.expires_at = now + interval;
                reinsert.push(expired_entry);
            }
        }

        // 将需要重新插入的周期性定时器加入队列
        for entry in reinsert {
            self.timers.push(entry);
        }

        count
    }

    /// 获取下一个到期时间
    ///
    /// # 返回
    /// Some(Instant)：下一个定时器的到期时间
    /// None：没有定时器
    pub fn next_expiry(&self) -> Option<Instant> {
        self.timers.peek().map(|entry| entry.expires_at)
    }

    /// 检查是否有定时器到期
    ///
    /// # 返回
    /// true表示有定时器到期
    pub fn has_expired(&self) -> bool {
        if let Some(entry) = self.timers.peek() {
            entry.expires_at <= self.now()
        } else {
            false
        }
    }

    /// 获取定时器数量
    pub fn len(&self) -> usize {
        self.timers.len()
    }

    /// 检查是否为空
    pub fn is_empty(&self) -> bool {
        self.timers.is_empty()
    }

    /// 清空所有定时器
    pub fn clear(&mut self) {
        self.timers.clear();
    }

    /// 获取当前时间
    fn now(&self) -> Instant {
        self.current_time.unwrap_or_else(Instant::now)
    }

    /// 设置当前时间（用于测试）
    #[cfg(test)]
    pub fn set_time(&mut self, time: Instant) {
        self.current_time = Some(time);
    }

    /// 快进时间（用于测试）
    #[cfg(test)]
    pub fn advance(&mut self, duration: Duration) {
        let new_time = self.now() + duration;
        self.set_time(new_time);
    }
}

impl Default for TimerManager {
    fn default() -> Self {
        Self::new()
    }
}

/// 协议定时器 trait
/// 协议模块实现此trait来与定时器系统集成
pub trait ProtocolTimer {
    /// 定时器到期时的处理函数
    ///
    /// # 参数
    /// - `timer_id`: 定时器ID
    /// - `context`: 系统上下文（可选）
    ///
    /// # 返回
    /// true表示继续定时（周期性），false表示停止
    fn on_timer_expired(&mut self, timer_id: TimerId) -> bool;
}

/// 定时器句柄
/// 用于在SystemContext中管理定时器
pub struct TimerHandle {
    pub manager: TimerManager,
}

impl TimerHandle {
    pub fn new() -> Self {
        TimerHandle {
            manager: TimerManager::new(),
        }
    }

    /// 处理所有到期定时器
    pub fn process_timers(&mut self) -> usize {
        self.manager.process_expired()
    }

    /// 检查是否有定时器到期
    pub fn has_expired(&self) -> bool {
        self.manager.has_expired()
    }
}

impl Default for TimerHandle {
    fn default() -> Self {
        Self::new()
    }
}

// 安全性：TimerHandle 在单线程环境中使用，通过 Arc<Mutex<T>> 保护访问
// 这是教学性质的协议栈实现，不涉及真正的多线程并发
unsafe impl Send for TimerHandle {}

// 安全性：同上，通过 Arc<Mutex<T>> 保护访问
unsafe impl Sync for TimerHandle {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_timer_oneshot() {
        let mut manager = TimerManager::new();

        manager.add_oneshot(
            Duration::from_secs(1),
            |_id, _data: &mut bool| {
                // 定时器回调
            },
            false
        );

        // 时间未到
        manager.advance(Duration::from_millis(500));
        assert_eq!(manager.process_expired(), 0);

        // 时间到了
        manager.advance(Duration::from_millis(600));
        assert_eq!(manager.process_expired(), 1);
    }

    #[test]
    fn test_timer_periodic() {
        let mut manager = TimerManager::new();
        let counter = std::sync::Arc::new(std::sync::atomic::AtomicU32::new(0));
        let counter_clone = counter.clone();

        manager.add_periodic(
            Duration::from_secs(1),
            move |_id, count: &mut u32| {
                *count += 1;
                counter_clone.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                *count < 3 // 执行3次后停止
            },
            0u32
        );

        // 第1次
        manager.advance(Duration::from_secs(1));
        assert_eq!(manager.process_expired(), 1);
        assert_eq!(counter.load(std::sync::atomic::Ordering::SeqCst), 1);

        // 第2次
        manager.advance(Duration::from_secs(1));
        assert_eq!(manager.process_expired(), 1);
        assert_eq!(counter.load(std::sync::atomic::Ordering::SeqCst), 2);

        // 第3次（最后一次）
        manager.advance(Duration::from_secs(1));
        assert_eq!(manager.process_expired(), 1);
        assert_eq!(counter.load(std::sync::atomic::Ordering::SeqCst), 3);

        // 没有了
        manager.advance(Duration::from_secs(1));
        assert_eq!(manager.process_expired(), 0);
    }

    #[test]
    fn test_timer_cancel() {
        let mut manager = TimerManager::new();

        let id = manager.add_oneshot(
            Duration::from_secs(1),
            |_id, _data: &mut ()| {},
            ()
        );

        assert_eq!(manager.len(), 1);
        assert!(manager.cancel(id));
        assert_eq!(manager.len(), 0);
        assert!(!manager.cancel(id)); // 已经取消了
    }

    #[test]
    fn test_timer_multiple() {
        let mut manager = TimerManager::new();
        let order = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));

        let order1 = order.clone();
        manager.add_oneshot(Duration::from_secs(2), move |_id, _data: &mut ()| {
            order1.lock().unwrap().push(2);
        }, ());

        let order2 = order.clone();
        manager.add_oneshot(Duration::from_secs(1), move |_id, _data: &mut ()| {
            order2.lock().unwrap().push(1);
        }, ());

        let order3 = order.clone();
        manager.add_oneshot(Duration::from_secs(3), move |_id, _data: &mut ()| {
            order3.lock().unwrap().push(3);
        }, ());

        manager.advance(Duration::from_secs(5));
        assert_eq!(manager.process_expired(), 3);

        let result = order.lock().unwrap();
        assert_eq!(*result, vec![1, 2, 3]); // 按到期顺序执行
    }
}
