// src/protocols/tcp/tcb.rs
//
// TCP 传输控制块（TCB）和连接状态定义

use crate::protocols::Ipv4Addr;
use super::error::TcpError;

/// TCP 连接四元组（标识符）
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TcpConnectionId {
    /// 本地 IP 地址
    pub local_ip: Ipv4Addr,
    /// 本地端口
    pub local_port: u16,
    /// 远程 IP 地址
    pub remote_ip: Ipv4Addr,
    /// 远程端口
    pub remote_port: u16,
}

impl TcpConnectionId {
    /// 创建新的连接标识符
    pub fn new(local_ip: Ipv4Addr, local_port: u16, remote_ip: Ipv4Addr, remote_port: u16) -> Self {
        Self {
            local_ip,
            local_port,
            remote_ip,
            remote_port,
        }
    }

    /// 创建监听连接 ID（远程地址为 0）
    pub fn listen(local_ip: Ipv4Addr, local_port: u16) -> Self {
        Self {
            local_ip,
            local_port,
            remote_ip: Ipv4Addr::new(0, 0, 0, 0),
            remote_port: 0,
        }
    }

    /// 检查是否为监听连接
    pub fn is_listen(&self) -> bool {
        self.remote_ip.is_zero() && self.remote_port == 0
    }
}

/// TCP 连接状态
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TcpState {
    /// 关闭状态
    Closed = 0,
    /// 监听状态
    Listen = 1,
    /// 同步已发送
    SynSent = 2,
    /// 同步已接收
    SynReceived = 3,
    /// 已建立连接
    Established = 4,
    /// 结束等待1
    FinWait1 = 5,
    /// 结束等待2
    FinWait2 = 6,
    /// 正在关闭
    Closing = 7,
    /// 时间等待
    TimeWait = 8,
    /// 关闭等待
    CloseWait = 9,
    /// 最后确认
    LastAck = 10,
}

impl TcpState {
    /// 获取状态名称
    pub const fn name(&self) -> &'static str {
        match self {
            TcpState::Closed => "CLOSED",
            TcpState::Listen => "LISTEN",
            TcpState::SynSent => "SYN_SENT",
            TcpState::SynReceived => "SYN_RCVD",
            TcpState::Established => "ESTABLISHED",
            TcpState::FinWait1 => "FIN_WAIT_1",
            TcpState::FinWait2 => "FIN_WAIT_2",
            TcpState::Closing => "CLOSING",
            TcpState::TimeWait => "TIME_WAIT",
            TcpState::CloseWait => "CLOSE_WAIT",
            TcpState::LastAck => "LAST_ACK",
        }
    }

    /// 检查是否为已建立连接状态
    pub const fn is_established(&self) -> bool {
        matches!(self, TcpState::Established)
    }

    /// 检查是否为连接建立过程中的状态
    pub const fn is_connecting(&self) -> bool {
        matches!(
            self,
            TcpState::SynSent | TcpState::SynReceived
        )
    }

    /// 检查是否为连接关闭过程中的状态
    pub const fn is_closing(&self) -> bool {
        matches!(
            self,
            TcpState::FinWait1
                | TcpState::FinWait2
                | TcpState::Closing
                | TcpState::TimeWait
                | TcpState::CloseWait
                | TcpState::LastAck
        )
    }

    /// 检查是否可以接收数据
    pub const fn can_receive(&self) -> bool {
        matches!(
            self,
            TcpState::Established | TcpState::FinWait1 | TcpState::FinWait2
        )
    }

    /// 检查是否可以发送数据
    pub const fn can_send(&self) -> bool {
        matches!(
            self,
            TcpState::Established | TcpState::CloseWait
        )
    }
}

impl std::fmt::Display for TcpState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name())
    }
}

/// 已发送但未确认的数据段
#[derive(Debug, Clone)]
pub struct SentSegment {
    /// 起始序列号
    pub seq: u32,
    /// 数据长度
    pub len: usize,
    /// 发送时间（微秒）
    pub send_time: u64,
}

impl SentSegment {
    /// 创建新的已发送段记录
    pub fn new(seq: u32, len: usize, send_time: u64) -> Self {
        Self {
            seq,
            len,
            send_time,
        }
    }
}

/// TCP 传输控制块（TCB）
///
/// 存储每个 TCP 连接的完整状态信息。
#[derive(Debug, Clone)]
pub struct Tcb {
    /// 连接标识符
    pub id: TcpConnectionId,
    /// 连接状态
    pub state: TcpState,

    // ========== 发送状态变量 ==========
    /// 发送但未确认的最小序列号
    pub snd_una: u32,
    /// 下一个要发送的序列号
    pub snd_nxt: u32,
    /// 发送窗口大小
    pub snd_wnd: u16,
    /// 紧急指针
    pub snd_up: u32,
    /// 初始发送序列号
    pub iss: u32,

    // ========== 接收状态变量 ==========
    /// 期望接收的下一个序列号
    pub rcv_nxt: u32,
    /// 接收窗口大小
    pub rcv_wnd: u16,
    /// 初始接收序列号
    pub irs: u32,

    // ========== RTT 估计 ==========
    /// 平滑往返时间（微秒）
    pub srtt: u32,
    /// 往返时间方差（微秒）
    pub rttvar: u32,
    /// 重传超时时间（毫秒）
    pub rto: u32,

    // ========== 拥塞控制 ==========
    /// 拥塞窗口大小（字节）
    pub cwnd: u32,
    /// 慢启动阈值（字节）
    pub ssthresh: u32,

    // ========== 选项协商 ==========
    /// 最大分段大小（MSS）
    pub mss: u16,
    /// 窗口缩放因子
    pub window_scale: u8,
    /// 是否支持 SACK
    pub sack_permitted: bool,
    /// 是否支持时间戳
    pub timestamps: bool,

    // ========== 统计信息 ==========
    /// 重传次数
    pub retransmit_count: u32,

    // ========== 缓冲区 ==========
    /// 发送缓冲区（环形队列）
    pub send_buffer: Vec<u8>,
    /// 接收缓冲区（环形队列）
    pub recv_buffer: Vec<u8>,
    /// 接收缓冲区已读取偏移
    pub recv_buffer_read: usize,
    /// 重传队列（记录已发送但未确认的数据段）
    pub retransmit_queue: Vec<SentSegment>,
}

impl Tcb {
    /// 创建新的 TCB
    pub fn new(id: TcpConnectionId) -> Self {
        Self {
            id,
            state: TcpState::Closed,
            snd_una: 0,
            snd_nxt: 0,
            snd_wnd: 0,
            snd_up: 0,
            iss: 0,
            rcv_nxt: 0,
            rcv_wnd: 0,
            irs: 0,
            srtt: 0,
            rttvar: 0,
            rto: 1000, // 默认 1 秒
            cwnd: 14600, // 默认 10 * MSS
            ssthresh: u32::MAX,
            mss: 1460,
            window_scale: 0,
            sack_permitted: false,
            timestamps: false,
            retransmit_count: 0,
            send_buffer: Vec::with_capacity(65536),  // 默认 64KB
            recv_buffer: Vec::with_capacity(65536),  // 默认 64KB
            recv_buffer_read: 0,
            retransmit_queue: Vec::new(),
        }
    }

    /// 创建监听 TCB
    pub fn listen(local_ip: Ipv4Addr, local_port: u16, window_size: u16) -> Self {
        let mut tcb = Self::new(TcpConnectionId::listen(local_ip, local_port));
        tcb.state = TcpState::Listen;
        tcb.rcv_wnd = window_size;
        tcb
    }

    /// 初始化发送状态变量（主动打开）
    pub fn init_send_state(&mut self, iss: u32) {
        self.iss = iss;
        self.snd_una = iss;
        self.snd_nxt = iss;
        self.snd_wnd = 0;
    }

    /// 初始化接收状态变量（收到 SYN）
    pub fn init_recv_state(&mut self, irs: u32, window_size: u16) {
        self.irs = irs;
        self.rcv_nxt = irs.wrapping_add(1);
        self.rcv_wnd = window_size;
    }

    /// 生成初始序列号（ISN）
    ///
    /// 使用符合 RFC 6528 的 ISN 生成算法。
    ///
    /// ISN = (M + F(local_ip, local_port, remote_ip, remote_port, secret)) mod 2^32
    ///
    /// 其中：
    /// - M 是一个计数器（基于微秒时间戳）
    /// - F 是一个哈希函数（使用简化的 FNV-1a 变体）
    /// - secret 是一个随机密钥（基于进程启动时间）
    ///
    /// # 参数
    /// - local_ip: 本地 IP 地址
    /// - local_port: 本地端口号
    /// - remote_ip: 远程 IP 地址
    /// - remote_port: 远程端口号
    ///
    /// # 返回
    /// - u32: 初始序列号
    pub fn generate_isn(
        local_ip: crate::protocols::Ipv4Addr,
        local_port: u16,
        remote_ip: crate::protocols::Ipv4Addr,
        remote_port: u16,
    ) -> u32 {
        use std::sync::atomic::{AtomicU32, Ordering};
        use std::time::{SystemTime, UNIX_EPOCH};

        // 计数器 M：基于微秒时间戳
        let since_epoch = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default();
        let micros = since_epoch.as_micros() as u64;
        let m = (micros & 0xFFFFFFFF) as u32;

        // 密钥 secret：基于进程启动时间（每进程唯一）
        // 使用黄金比例分数作为额外混合
        static SECRET_KEY: AtomicU32 = AtomicU32::new(0);
        let mut secret = SECRET_KEY.load(Ordering::Relaxed);
        if secret == 0 {
            // 首次初始化：使用启动时间和黄金比例分数
            let init_secret = (since_epoch.as_nanos() as u32).wrapping_add(0x9e3779b9);
            secret = init_secret;
            // 尝试存储（可能被其他线程抢先，这不影响正确性）
            let _ = SECRET_KEY.compare_exchange(0, init_secret, Ordering::Relaxed, Ordering::Relaxed);
        }

        // 哈希函数 F：使用简化的 FNV-1a 变体
        // F(local_ip, local_port, remote_ip, remote_port, secret)
        let f = Self::isn_hash_function(
            local_ip,
            local_port,
            remote_ip,
            remote_port,
            secret,
        );

        // ISN = (M + F) mod 2^32
        m.wrapping_add(f)
    }

    /// ISN 哈希函数（基于 FNV-1a）
    ///
    /// 将所有输入混合成一个 32 位哈希值
    fn isn_hash_function(
        local_ip: crate::protocols::Ipv4Addr,
        local_port: u16,
        remote_ip: crate::protocols::Ipv4Addr,
        remote_port: u16,
        secret: u32,
    ) -> u32 {
        // 使用 FNV-1a 32 位哈希算法的简化版本
        // FNV-1a 偏移基数：2166136261
        // FNV-1a 质数：16777619

        const FNV_OFFSET_BASIS: u32 = 2166136261;
        const FNV_PRIME: u32 = 16777619;

        let mut hash = FNV_OFFSET_BASIS;

        // 混入 secret
        hash ^= (secret >> 24) & 0xFF;
        hash = hash.wrapping_mul(FNV_PRIME);
        hash ^= (secret >> 16) & 0xFF;
        hash = hash.wrapping_mul(FNV_PRIME);
        hash ^= (secret >> 8) & 0xFF;
        hash = hash.wrapping_mul(FNV_PRIME);
        hash ^= secret & 0xFF;
        hash = hash.wrapping_mul(FNV_PRIME);

        // 混入 local_ip（4 字节）
        for byte in local_ip.bytes {
            hash ^= byte as u32;
            hash = hash.wrapping_mul(FNV_PRIME);
        }

        // 混入 local_port（2 字节）
        hash ^= ((local_port >> 8) & 0xFF) as u32;
        hash = hash.wrapping_mul(FNV_PRIME);
        hash ^= (local_port & 0xFF) as u32;
        hash = hash.wrapping_mul(FNV_PRIME);

        // 混入 remote_ip（4 字节）
        for byte in remote_ip.bytes {
            hash ^= byte as u32;
            hash = hash.wrapping_mul(FNV_PRIME);
        }

        // 混入 remote_port（2 字节）
        hash ^= ((remote_port >> 8) & 0xFF) as u32;
        hash = hash.wrapping_mul(FNV_PRIME);
        hash ^= (remote_port & 0xFF) as u32;
        hash = hash.wrapping_mul(FNV_PRIME);

        hash
    }

    /// 检查序列号是否在接收窗口内
    pub fn is_seq_in_window(&self, seq: u32) -> bool {
        // 简化实现：检查 seq 是否等于 rcv_nxt
        // 实际应考虑窗口大小和序列号回绕
        seq == self.rcv_nxt
    }

    /// 获取发送窗口大小（取通告窗口和拥塞窗口的较小值）
    pub fn effective_window(&self) -> u32 {
        let snd_wnd = self.snd_wnd as u32;
        std::cmp::min(snd_wnd, self.cwnd)
    }

    /// 获取可用发送窗口
    pub fn available_window(&self) -> u32 {
        let outstanding = self.snd_nxt.wrapping_sub(self.snd_una);
        let eff = self.effective_window();
        eff.saturating_sub(outstanding)
    }

    /// 更新 RTO 估计（RFC 2988）
    pub fn update_rto(&mut self, rtt_ms: u32) {
        if self.srtt == 0 {
            // 第一次测量
            self.srtt = rtt_ms;
            self.rttvar = rtt_ms / 2;
        } else {
            // 后续测量
            let delta = self.srtt.abs_diff(rtt_ms);
            self.rttvar = (3 * self.rttvar + delta) / 4;
            self.srtt = (7 * self.srtt + rtt_ms) / 8;
        }

        // 计算 RTO
        self.rto = self.srtt + std::cmp::max(200u32, 4 * self.rttvar);
        self.rto = self.rto.clamp(200u32, 120000u32);
    }

    /// 重置拥塞控制（超时重传后）
    pub fn reset_congestion_control(&mut self, mss: u32) {
        self.ssthresh = std::cmp::max(self.effective_window() / 2, 2 * mss);
        self.cwnd = mss;
    }

    /// 慢启动（每收到一个 ACK）
    pub fn slow_start(&mut self, mss: u32) {
        self.cwnd += mss;
    }

    /// 拥塞避免（每收到一个 ACK）
    pub fn congestion_avoidance(&mut self, mss: u32) {
        if self.cwnd < self.ssthresh {
            // 仍在慢启动
            self.cwnd += mss;
        } else {
            // 拥塞避免
            self.cwnd += (mss * mss) / self.cwnd.max(1);
        }
    }

    /// 快恢复（3 个重复 ACK）
    pub fn fast_retransmit(&mut self, mss: u32) {
        self.ssthresh = std::cmp::max(self.effective_window() / 2, 2 * mss);
        self.cwnd = self.ssthresh + 3 * mss;
    }

    /// 快恢复中（收到重复 ACK）
    pub fn fast_recovery(&mut self, mss: u32) {
        self.cwnd += mss;
    }

    /// 快恢复完成（收到新数据的 ACK）
    pub fn finish_fast_recovery(&mut self) {
        self.cwnd = self.ssthresh;
    }

    /// 检查是否需要重传
    pub fn should_retransmit(&self, max_attempts: u32) -> bool {
        self.retransmit_count < max_attempts
    }

    /// 增加重传计数
    pub fn increment_retransmit(&mut self) {
        self.retransmit_count += 1;
        // 指数退避
        self.rto = std::cmp::min(self.rto * 2, 120000u32);
    }

    /// 重置重传计数
    pub fn reset_retransmit_count(&mut self) {
        self.retransmit_count = 0;
    }

    // ========== 缓冲区管理方法 ==========

    /// 写入数据到接收缓冲区
    ///
    /// # 参数
    /// - data: 要写入的数据
    ///
    /// # 返回
    /// - Ok(()): 写入成功
    /// - Err(TcpError): 缓冲区已满
    pub fn write_to_recv_buffer(&mut self, data: &[u8]) -> Result<(), TcpError> {
        // 检查可用空间（基于当前未读取数据量）
        let unread_count = self.recv_buffer.len() - self.recv_buffer_read;
        let available = self.recv_buffer.capacity() - unread_count;
        if data.len() > available {
            return Err(TcpError::BufferFull);
        }

        // 写入数据到缓冲区末尾
        self.recv_buffer.extend_from_slice(data);

        Ok(())
    }

    /// 从接收缓冲区读取数据
    ///
    /// # 参数
    /// - buf: 输出缓冲区
    ///
    /// # 返回
    /// - usize: 实际读取的字节数
    pub fn read_from_recv_buffer(&mut self, buf: &mut [u8]) -> usize {
        let available = self.available_recv_data();
        let to_read = buf.len().min(available);

        // 从缓冲区复制数据
        if to_read > 0 {
            buf[..to_read].copy_from_slice(&self.recv_buffer[self.recv_buffer_read..self.recv_buffer_read + to_read]);
            self.recv_buffer_read += to_read;

            // 清理已读取的数据（如果读指针超过一半）
            if self.recv_buffer_read > self.recv_buffer.capacity() / 2 {
                self.recv_buffer.drain(0..self.recv_buffer_read);
                self.recv_buffer_read = 0;
            }
        }

        to_read
    }

    /// 获取接收缓冲区中可读取的数据量
    pub fn available_recv_data(&self) -> usize {
        self.recv_buffer.len().saturating_sub(self.recv_buffer_read)
    }

    /// 添加段到重传队列
    pub fn add_to_retransmit_queue(&mut self, seq: u32, len: usize) {
        // 简化实现：使用当前时间
        let send_time = 0; // 实际应使用真实时间戳
        self.retransmit_queue.push(SentSegment::new(seq, len, send_time));
    }

    /// 从重传队列中移除已确认的段
    pub fn remove_acked_from_retransmit_queue(&mut self, ack: u32) {
        self.retransmit_queue.retain(|seg| {
            // 保留序列号 >= ACK 的段（未完全确认）
            seg.seq.wrapping_add(seg.len as u32) > ack
        });
    }

    /// 获取发送缓冲区可用空间
    pub fn available_send_space(&self) -> usize {
        self.send_buffer.capacity() - self.send_buffer.len()
    }

    /// 写入数据到发送缓冲区
    ///
    /// # 参数
    /// - data: 要写入的数据
    ///
    /// # 返回
    /// - Ok(usize): 实际写入的字节数
    /// - Err(TcpError): 写入失败
    pub fn write_to_send_buffer(&mut self, data: &[u8]) -> Result<usize, TcpError> {
        let available = self.available_send_space();
        let to_write = data.len().min(available);

        if to_write == 0 {
            return Err(TcpError::BufferFull);
        }

        self.send_buffer.extend_from_slice(&data[..to_write]);
        Ok(to_write)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_connection_id_new() {
        let local_ip = Ipv4Addr::new(192, 168, 1, 100);
        let remote_ip = Ipv4Addr::new(192, 168, 1, 10);
        let id = TcpConnectionId::new(local_ip, 8080, remote_ip, 12345);

        assert_eq!(id.local_ip, local_ip);
        assert_eq!(id.local_port, 8080);
        assert_eq!(id.remote_ip, remote_ip);
        assert_eq!(id.remote_port, 12345);
        assert!(!id.is_listen());
    }

    #[test]
    fn test_connection_id_listen() {
        let local_ip = Ipv4Addr::new(192, 168, 1, 100);
        let id = TcpConnectionId::listen(local_ip, 80);

        assert_eq!(id.local_ip, local_ip);
        assert_eq!(id.local_port, 80);
        assert!(id.remote_ip.is_zero());
        assert_eq!(id.remote_port, 0);
        assert!(id.is_listen());
    }

    #[test]
    fn test_state_name() {
        assert_eq!(TcpState::Closed.name(), "CLOSED");
        assert_eq!(TcpState::Listen.name(), "LISTEN");
        assert_eq!(TcpState::Established.name(), "ESTABLISHED");
        assert_eq!(TcpState::TimeWait.name(), "TIME_WAIT");
    }

    #[test]
    fn test_state_checks() {
        assert!(TcpState::Established.is_established());
        assert!(TcpState::Established.can_receive());
        assert!(TcpState::Established.can_send());

        assert!(TcpState::SynSent.is_connecting());
        assert!(TcpState::SynReceived.is_connecting());

        assert!(TcpState::FinWait1.is_closing());
        assert!(TcpState::TimeWait.is_closing());
        assert!(TcpState::CloseWait.is_closing());

        assert!(!TcpState::Closed.can_receive());
        assert!(!TcpState::Closed.can_send());
    }

    #[test]
    fn test_tcb_new() {
        let id = TcpConnectionId::listen(Ipv4Addr::new(192, 168, 1, 100), 80);
        let tcb = Tcb::new(id.clone());

        assert_eq!(tcb.id, id);
        assert_eq!(tcb.state, TcpState::Closed);
        assert_eq!(tcb.mss, 1460);
        assert_eq!(tcb.cwnd, 14600);
    }

    #[test]
    fn test_tcb_listen() {
        let local_ip = Ipv4Addr::new(192, 168, 1, 100);
        let tcb = Tcb::listen(local_ip, 80, 65535);

        assert_eq!(tcb.state, TcpState::Listen);
        assert_eq!(tcb.rcv_wnd, 65535);
        assert!(tcb.id.is_listen());
    }

    #[test]
    fn test_tcb_init_send_state() {
        let mut tcb = Tcb::new(TcpConnectionId::listen(Ipv4Addr::new(192, 168, 1, 100), 80));
        tcb.init_send_state(1000);

        assert_eq!(tcb.iss, 1000);
        assert_eq!(tcb.snd_una, 1000);
        assert_eq!(tcb.snd_nxt, 1000);
    }

    #[test]
    fn test_tcb_init_recv_state() {
        let mut tcb = Tcb::new(TcpConnectionId::listen(Ipv4Addr::new(192, 168, 1, 100), 80));
        tcb.init_recv_state(5000, 8192);

        assert_eq!(tcb.irs, 5000);
        assert_eq!(tcb.rcv_nxt, 5001);
        assert_eq!(tcb.rcv_wnd, 8192);
    }

    #[test]
    fn test_tcb_generate_isn() {
        let local_ip = Ipv4Addr::new(192, 168, 1, 10);
        let local_port = 1234;
        let remote_ip = Ipv4Addr::new(192, 168, 1, 20);
        let remote_port = 5678;

        let isn1 = Tcb::generate_isn(local_ip, local_port, remote_ip, remote_port);

        // 稍微延迟以确保时间戳不同
        std::thread::sleep(std::time::Duration::from_millis(1));

        let isn2 = Tcb::generate_isn(local_ip, local_port, remote_ip, remote_port);

        // ISN 应该递增（由于时间戳不同）
        assert_ne!(isn1, isn2);

        // 相同参数应该产生不同的 ISN（由于时间戳）
        // 不同参数应该产生不同的 ISN（由于哈希输入不同）
        let remote_ip2 = Ipv4Addr::new(192, 168, 1, 21);
        let isn3 = Tcb::generate_isn(local_ip, local_port, remote_ip2, remote_port);
        assert_ne!(isn1, isn3);
    }

    #[test]
    fn test_tcb_effective_window() {
        let mut tcb = Tcb::new(TcpConnectionId::listen(Ipv4Addr::new(192, 168, 1, 100), 80));
        tcb.snd_wnd = 32768;
        tcb.cwnd = 16384;

        // 有效窗口 = min(snd_wnd, cwnd)
        assert_eq!(tcb.effective_window(), 16384);
    }

    #[test]
    fn test_tcb_available_window() {
        let mut tcb = Tcb::new(TcpConnectionId::listen(Ipv4Addr::new(192, 168, 1, 100), 80));
        tcb.snd_una = 1000;
        tcb.snd_nxt = 1200;
        tcb.snd_wnd = 8192;
        tcb.cwnd = 8192;

        // 可用窗口 = effective - (snd_nxt - snd_una)
        assert_eq!(tcb.available_window(), 8192 - 200);
    }

    #[test]
    fn test_tcb_update_rto() {
        let mut tcb = Tcb::new(TcpConnectionId::listen(Ipv4Addr::new(192, 168, 1, 100), 80));

        // 第一次测量
        tcb.update_rto(200);
        assert_eq!(tcb.srtt, 200);
        assert_eq!(tcb.rttvar, 100);

        // 第二次测量
        tcb.update_rto(250);
        assert!(tcb.srtt > 0);
        assert!(tcb.rto >= 200); // 应该被限制在最小值
    }

    #[test]
    fn test_tcb_slow_start() {
        let mut tcb = Tcb::new(TcpConnectionId::listen(Ipv4Addr::new(192, 168, 1, 100), 80));
        tcb.cwnd = 14600;
        tcb.ssthresh = u32::MAX;

        tcb.slow_start(1460);
        assert_eq!(tcb.cwnd, 16060);
    }

    #[test]
    fn test_tcb_congestion_avoidance() {
        let mut tcb = Tcb::new(TcpConnectionId::listen(Ipv4Addr::new(192, 168, 1, 100), 80));
        tcb.cwnd = 20000;
        tcb.ssthresh = 15000;

        // cwnd > ssthresh，进入拥塞避免
        tcb.congestion_avoidance(1460);
        assert!(tcb.cwnd > 20000);
        assert!(tcb.cwnd < 21500); // 应该增长较慢
    }

    #[test]
    fn test_tcb_fast_retransmit() {
        let mut tcb = Tcb::new(TcpConnectionId::listen(Ipv4Addr::new(192, 168, 1, 100), 80));
        tcb.snd_wnd = 32768;
        tcb.cwnd = 30000;

        tcb.fast_retransmit(1460);
        assert_eq!(tcb.ssthresh, 15000); // cwnd / 2
        assert_eq!(tcb.cwnd, 15000 + 3 * 1460);
    }

    #[test]
    fn test_tcb_retransmit_count() {
        let mut tcb = Tcb::new(TcpConnectionId::listen(Ipv4Addr::new(192, 168, 1, 100), 80));

        assert!(tcb.should_retransmit(12));

        tcb.increment_retransmit();
        assert_eq!(tcb.retransmit_count, 1);

        tcb.reset_retransmit_count();
        assert_eq!(tcb.retransmit_count, 0);
    }

    #[test]
    fn test_tcb_recv_buffer_write_read() {
        let mut tcb = Tcb::new(TcpConnectionId::listen(Ipv4Addr::new(192, 168, 1, 100), 80));

        // 写入数据
        let data = b"Hello TCP";
        tcb.write_to_recv_buffer(data).unwrap();

        assert_eq!(tcb.available_recv_data(), 9);

        // 读取数据
        let mut buf = [0u8; 20];
        let n = tcb.read_from_recv_buffer(&mut buf);
        assert_eq!(n, 9);
        assert_eq!(&buf[..9], data);
        assert_eq!(tcb.available_recv_data(), 0);
    }

    #[test]
    fn test_tcb_recv_buffer_multiple_writes() {
        let mut tcb = Tcb::new(TcpConnectionId::listen(Ipv4Addr::new(192, 168, 1, 100), 80));

        tcb.write_to_recv_buffer(b"First ").unwrap();
        tcb.write_to_recv_buffer(b"Second").unwrap();

        // "First " (6 bytes) + "Second" (6 bytes) = 12 bytes
        assert_eq!(tcb.available_recv_data(), 12);

        let mut buf = [0u8; 20];
        let n = tcb.read_from_recv_buffer(&mut buf);
        assert_eq!(n, 12);
        assert_eq!(&buf[..12], b"First Second");
    }

    #[test]
    fn test_tcb_send_buffer() {
        let mut tcb = Tcb::new(TcpConnectionId::listen(Ipv4Addr::new(192, 168, 1, 100), 80));

        // 检查初始可用空间
        assert!(tcb.available_send_space() > 0);

        // 写入数据
        let data = b"Hello TCP";
        let n = tcb.write_to_send_buffer(data).unwrap();
        assert_eq!(n, 9);

        // 可用空间减少
        assert!(tcb.available_send_space() < 65536);
    }

    #[test]
    fn test_tcb_retransmit_queue() {
        let mut tcb = Tcb::new(TcpConnectionId::listen(Ipv4Addr::new(192, 168, 1, 100), 80));

        // 添加段到重传队列
        tcb.add_to_retransmit_queue(1000, 100);
        tcb.add_to_retransmit_queue(1100, 200);

        assert_eq!(tcb.retransmit_queue.len(), 2);

        // 确认部分数据
        tcb.remove_acked_from_retransmit_queue(1050);
        assert_eq!(tcb.retransmit_queue.len(), 2); // 第一个段未完全确认

        // 确认第一个段
        tcb.remove_acked_from_retransmit_queue(1100);
        assert_eq!(tcb.retransmit_queue.len(), 1); // 第一个段已确认

        // 确认所有数据
        tcb.remove_acked_from_retransmit_queue(1300);
        assert_eq!(tcb.retransmit_queue.len(), 0);
    }
}
