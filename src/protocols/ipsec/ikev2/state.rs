// src/protocols/ipsec/ikev2/state.rs
//
// IKEv2 状态机实现

use super::*;
use std::collections::VecDeque;

// ========== 发起方状态 ==========

/// IKE 发起方状态
#[derive(Debug, Clone, PartialEq)]
pub enum IkeInitiatorState {
    /// 初始状态
    Idle,
    /// 已发送 IKE_SA_INIT 请求
    InitSent,
    /// 已发送 IKE_AUTH 请求
    AuthSent,
    /// IKE SA 已建立
    Established,
    /// 重密钥中
    Rekeying,
    /// 删除中
    Deleting,
}

impl IkeInitiatorState {
    /// 转换到下一个状态
    pub fn transition(&mut self, event: &IkeEvent) -> IkeResult<()> {
        let current = std::mem::replace(self, Self::Idle);
        let new_state = match (current, event) {
            (Self::Idle, IkeEvent::SendInitRequest) => Self::InitSent,
            (Self::InitSent, IkeEvent::ReceiveInitResponse) => Self::AuthSent,
            (Self::AuthSent, IkeEvent::ReceiveAuthResponse) => Self::Established,
            (Self::Established, IkeEvent::StartRekey) => Self::Rekeying,
            (Self::Rekeying, IkeEvent::RekeyComplete) => Self::Established,
            (Self::Established, IkeEvent::DeleteSa) => Self::Deleting,
            (Self::Deleting, IkeEvent::DeleteComplete) => Self::Idle,
            (current, event) => return Err(IkeError::SaStateError(format!(
                "无效的状态转换: {:?} + {:?}", current, event
            ))),
        };
        *self = new_state;
        Ok(())
    }

    /// 检查是否可以发送数据
    pub fn can_send(&self) -> bool {
        matches!(self, Self::Established)
    }
}

// ========== 响应方状态 ==========

/// IKE 响应方状态
#[derive(Debug, Clone, PartialEq)]
pub enum IkeResponderState {
    /// 初始状态
    Idle,
    /// 已接收 IKE_SA_INIT 请求
    InitReceived,
    /// 已发送 IKE_SA_INIT 响应
    InitResponded,
    /// 已接收 IKE_AUTH 请求
    AuthReceived,
    /// IKE SA 已建立
    Established,
    /// 重密钥中
    Rekeying,
    /// 删除中
    Deleting,
}

impl IkeResponderState {
    /// 转换到下一个状态
    pub fn transition(&mut self, event: &IkeEvent) -> IkeResult<()> {
        let current = std::mem::replace(self, Self::Idle);
        let new_state = match (current, event) {
            (Self::Idle, IkeEvent::ReceiveInitRequest) => Self::InitResponded,
            (Self::InitResponded, IkeEvent::ReceiveAuthRequest) => Self::Established,
            (Self::Established, IkeEvent::ReceiveRekeyRequest) => Self::Rekeying,
            (Self::Rekeying, IkeEvent::RekeyComplete) => Self::Established,
            (Self::Established, IkeEvent::ReceiveDeleteRequest) => Self::Deleting,
            (Self::Deleting, IkeEvent::DeleteComplete) => Self::Idle,
            (current, event) => return Err(IkeError::SaStateError(format!(
                "无效的状态转换: {:?} + {:?}", current, event
            ))),
        };
        *self = new_state;
        Ok(())
    }

    /// 检查是否可以处理数据
    pub fn can_process(&self) -> bool {
        matches!(self, Self::Established)
    }
}

// ========== IKE 事件 ==========

/// IKE 状态机事件
#[derive(Debug, Clone, PartialEq)]
pub enum IkeEvent {
    /// 发送 IKE_SA_INIT 请求
    SendInitRequest,
    /// 接收 IKE_SA_INIT 响应
    ReceiveInitResponse,
    /// 接收 IKE_SA_INIT 请求
    ReceiveInitRequest,
    /// 接收 IKE_AUTH 请求
    ReceiveAuthRequest,
    /// 接收 IKE_AUTH 响应
    ReceiveAuthResponse,
    /// 发送 IKE_AUTH 请求
    SendAuthRequest,
    /// 开始重密钥
    StartRekey,
    /// 接收重密钥请求
    ReceiveRekeyRequest,
    /// 重密钥完成
    RekeyComplete,
    /// 删除 SA
    DeleteSa,
    /// 接收删除请求
    ReceiveDeleteRequest,
    /// 删除完成
    DeleteComplete,
    /// 超时
    Timeout,
}

// ========== IKE 状态机 ==========

/// IKEv2 状态机
#[derive(Debug, Clone)]
pub struct IkeStateMachine {
    /// 角色
    role: IkeRole,
    /// 当前状态（发起方）
    initiator_state: IkeInitiatorState,
    /// 当前状态（响应方）
    responder_state: IkeResponderState,
    /// 事件队列
    event_queue: VecDeque<IkeEvent>,
    /// 重传计数
    retransmit_count: u32,
    /// 最大重传次数
    max_retransmit: u32,
}

impl IkeStateMachine {
    /// 创建新的状态机
    pub fn new(role: IkeRole) -> Self {
        Self {
            role,
            initiator_state: IkeInitiatorState::Idle,
            responder_state: IkeResponderState::Idle,
            event_queue: VecDeque::new(),
            retransmit_count: 0,
            max_retransmit: 3,
        }
    }

    /// 获取当前状态（统一接口）
    pub fn state(&self) -> IkeSaState {
        match self.role {
            IkeRole::Initiator => match self.initiator_state {
                IkeInitiatorState::Idle => IkeSaState::Idle,
                IkeInitiatorState::InitSent => IkeSaState::InitSent,
                IkeInitiatorState::AuthSent => IkeSaState::AuthSent,
                IkeInitiatorState::Established => IkeSaState::Established,
                IkeInitiatorState::Rekeying => IkeSaState::Established,
                IkeInitiatorState::Deleting => IkeSaState::Deleted,
            },
            IkeRole::Responder => match self.responder_state {
                IkeResponderState::Idle => IkeSaState::Idle,
                IkeResponderState::InitReceived => IkeSaState::InitSent,
                IkeResponderState::InitResponded => IkeSaState::InitSent,
                IkeResponderState::AuthReceived => IkeSaState::AuthSent,
                IkeResponderState::Established => IkeSaState::Established,
                IkeResponderState::Rekeying => IkeSaState::Established,
                IkeResponderState::Deleting => IkeSaState::Deleted,
            },
        }
    }

    /// 处理事件
    pub fn handle_event(&mut self, event: IkeEvent) -> IkeResult<()> {
        self.event_queue.push_back(event);

        while let Some(event) = self.event_queue.pop_front() {
            match self.role {
                IkeRole::Initiator => {
                    self.initiator_state.transition(&event)?;
                }
                IkeRole::Responder => {
                    self.responder_state.transition(&event)?;
                }
            }

            // 重置重传计数
            self.retransmit_count = 0;
        }

        Ok(())
    }

    /// 获取发起方状态
    pub fn initiator_state(&self) -> &IkeInitiatorState {
        &self.initiator_state
    }

    /// 获取响应方状态
    pub fn responder_state(&self) -> &IkeResponderState {
        &self.responder_state
    }

    /// 增加重传计数
    pub fn increment_retransmit(&mut self) {
        self.retransmit_count += 1;
    }

    /// 获取重传计数
    pub fn retransmit_count(&self) -> u32 {
        self.retransmit_count
    }

    /// 检查是否超过最大重传次数
    pub fn is_retransmit_exceeded(&self) -> bool {
        self.retransmit_count >= self.max_retransmit
    }

    /// 设置最大重传次数
    pub fn set_max_retransmit(&mut self, max: u32) {
        self.max_retransmit = max;
    }
}

impl Default for IkeStateMachine {
    fn default() -> Self {
        Self::new(IkeRole::Initiator)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_initiator_state_transitions() {
        let mut state = IkeInitiatorState::Idle;

        state.transition(&IkeEvent::SendInitRequest).unwrap();
        assert_eq!(state, IkeInitiatorState::InitSent);

        state.transition(&IkeEvent::ReceiveInitResponse).unwrap();
        assert_eq!(state, IkeInitiatorState::AuthSent);

        state.transition(&IkeEvent::ReceiveAuthResponse).unwrap();
        assert_eq!(state, IkeInitiatorState::Established);

        assert!(state.can_send());
    }

    #[test]
    fn test_responder_state_transitions() {
        let mut state = IkeResponderState::Idle;

        state.transition(&IkeEvent::ReceiveInitRequest).unwrap();
        assert_eq!(state, IkeResponderState::InitResponded);

        state.transition(&IkeEvent::ReceiveAuthRequest).unwrap();
        assert_eq!(state, IkeResponderState::Established);

        assert!(state.can_process());
    }

    #[test]
    fn test_invalid_transition() {
        let mut state = IkeInitiatorState::Idle;

        let result = state.transition(&IkeEvent::ReceiveAuthResponse);
        assert!(result.is_err());
    }

    #[test]
    fn test_state_machine() {
        let mut sm = IkeStateMachine::new(IkeRole::Initiator);

        assert_eq!(sm.state(), IkeSaState::Idle);

        sm.handle_event(IkeEvent::SendInitRequest).unwrap();
        assert_eq!(sm.state(), IkeSaState::InitSent);

        sm.handle_event(IkeEvent::ReceiveInitResponse).unwrap();
        assert_eq!(sm.state(), IkeSaState::AuthSent);

        sm.handle_event(IkeEvent::ReceiveAuthResponse).unwrap();
        assert_eq!(sm.state(), IkeSaState::Established);
    }

    #[test]
    fn test_retransmit() {
        let mut sm = IkeStateMachine::new(IkeRole::Initiator);

        assert_eq!(sm.retransmit_count(), 0);
        assert!(!sm.is_retransmit_exceeded());

        sm.increment_retransmit();
        sm.increment_retransmit();

        assert_eq!(sm.retransmit_count(), 2);
        assert!(!sm.is_retransmit_exceeded());

        sm.increment_retransmit();
        assert!(sm.is_retransmit_exceeded());
    }

    #[test]
    fn test_responder_state_machine() {
        let mut sm = IkeStateMachine::new(IkeRole::Responder);

        assert_eq!(sm.state(), IkeSaState::Idle);

        sm.handle_event(IkeEvent::ReceiveInitRequest).unwrap();
        assert_eq!(sm.state(), IkeSaState::InitSent);

        sm.handle_event(IkeEvent::ReceiveAuthRequest).unwrap();
        assert_eq!(sm.state(), IkeSaState::Established);
    }
}
