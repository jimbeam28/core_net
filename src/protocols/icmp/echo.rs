// src/protocols/icmp/echo.rs
//
// ICMP Echo Request/Reply 处理逻辑

use crate::common::{CoreError, Result};
use crate::protocols::Ipv4Addr;

use super::packet::IcmpEcho;
use super::global::{get_or_init_global_echo_manager, PendingEcho};

/// Echo 处理结果
#[derive(Debug, Clone, PartialEq)]
pub enum EchoProcessResult {
    /// 无需响应（例如：收到的不是 Request）
    NoReply,

    /// 需要发送 Reply
    Reply(IcmpEcho),

    /// 收到 Reply，匹配成功
    Matched {
        identifier: u16,
        sequence: u16,
        rtt_ms: u64,
    },
}

/// 处理接收到的 Echo Request
///
/// # 参数
/// - echo: 接收到的 Echo 报文
/// - our_ip: 本接口的 IP 地址
///
/// # 返回
/// - Ok(EchoProcessResult): 处理结果
/// - Err(CoreError): 处理失败
pub fn handle_echo_request(echo: &IcmpEcho, _our_ip: Ipv4Addr) -> Result<EchoProcessResult> {
    // 只处理 Echo Request
    if !echo.is_request() {
        return Ok(EchoProcessResult::NoReply);
    }

    // 生成对应的 Echo Reply
    let reply = echo.make_reply();

    Ok(EchoProcessResult::Reply(reply))
}

/// 处理接收到的 Echo Reply
///
/// # 参数
/// - echo: 接收到的 Echo Reply
///
/// # 返回
/// - Ok(EchoProcessResult): 处理结果
/// - Err(CoreError): 处理失败
pub fn handle_echo_reply(echo: &IcmpEcho) -> Result<EchoProcessResult> {
    if !echo.is_reply() {
        return Ok(EchoProcessResult::NoReply);
    }

    // 查找对应的待处理请求
    let manager = get_or_init_global_echo_manager();
    let mut guard = manager.lock()
        .map_err(|e| CoreError::parse_error(format!("锁定Echo管理器失败: {}", e)))?;

    if let Some(pending) = guard.remove_pending(echo.identifier, echo.sequence) {
        let rtt = pending.rtt();
        let rtt_ms = rtt.as_millis() as u64;

        return Ok(EchoProcessResult::Matched {
            identifier: echo.identifier,
            sequence: echo.sequence,
            rtt_ms,
        });
    }

    // 未找到匹配的请求（可能是重复或延迟的响应）
    Ok(EchoProcessResult::NoReply)
}

/// 注册 Echo Request（发送前调用）
///
/// # 参数
/// - identifier: Echo 标识符
/// - sequence: Echo 序列号
/// - destination: 目标 IP 地址
///
/// # 返回
/// - Ok(()): 注册成功
/// - Err(CoreError): 注册失败
pub fn register_echo_request(
    identifier: u16,
    sequence: u16,
    destination: Ipv4Addr,
) -> Result<()> {
    let pending = PendingEcho::new(identifier, sequence, destination);

    let manager = get_or_init_global_echo_manager();
    let mut guard = manager.lock()
        .map_err(|e| CoreError::parse_error(format!("锁定Echo管理器失败: {}", e)))?;

    guard.add_pending(pending)
        .map_err(|e| CoreError::parse_error(format!("注册Echo请求失败: {}", e)))?;

    Ok(())
}

/// 清理超时的 Echo 请求
pub fn cleanup_echo_timeouts() {
    let manager = get_or_init_global_echo_manager();
    if let Ok(mut guard) = manager.lock() {
        guard.cleanup_timeouts();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_handle_echo_request() {
        let request = IcmpEcho::new_request(1234, 1, vec![0x42; 32]);
        let our_ip = Ipv4Addr::new(192, 168, 1, 100);

        let result = handle_echo_request(&request, our_ip).unwrap();

        match result {
            EchoProcessResult::Reply(reply) => {
                assert_eq!(reply.type_, 0); // Echo Reply
                assert_eq!(reply.identifier, 1234);
                assert_eq!(reply.sequence, 1);
            }
            _ => panic!("Expected Reply"),
        }
    }

    #[test]
    fn test_handle_echo_reply_no_match() {
        let reply = IcmpEcho::new_reply(1234, 1, vec![0x42; 32]);

        let result = handle_echo_reply(&reply).unwrap();

        // 没有注册对应的请求，应该返回 NoReply
        assert_eq!(result, EchoProcessResult::NoReply);
    }

    #[test]
    fn test_register_and_match() {
        let dest = Ipv4Addr::new(192, 168, 1, 1);

        // 注册请求
        register_echo_request(1234, 1, dest).unwrap();

        // 处理响应
        let reply = IcmpEcho::new_reply(1234, 1, vec![0x42; 32]);
        let result = handle_echo_reply(&reply).unwrap();

        match result {
            EchoProcessResult::Matched { identifier, sequence, rtt_ms } => {
                assert_eq!(identifier, 1234);
                assert_eq!(sequence, 1);
                assert!(rtt_ms < 100); // 应该很快
            }
            _ => panic!("Expected Matched"),
        }
    }
}
