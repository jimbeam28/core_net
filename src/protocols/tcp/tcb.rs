// src/protocols/tcp/tcb.rs
//
// TCP 传输控制块（TCB）和连接状态定义

use crate::protocols::Ipv4Addr;

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
    /// 使用简单的基于时间的 ISN 生成算法。
    /// 实际实现应使用更安全的随机数生成器（RFC 6528）。
    pub fn generate_isn() -> u32 {
        // 简化实现：使用计数器
        // 实际应使用基于时间戳和加密哈希的算法
        use std::sync::atomic::{AtomicU32, Ordering};
        static ISN_COUNTER: AtomicU32 = AtomicU32::new(1);

        // 以 1 秒为单位的 ISN（简化版）
        ISN_COUNTER.fetch_add(1, Ordering::Relaxed)
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
        let isn1 = Tcb::generate_isn();
        let isn2 = Tcb::generate_isn();

        // ISN 应该递增
        assert!(isn2 != isn1); // 可能回绕
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
}
