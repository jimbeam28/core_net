// src/protocols/ipsec/ikev2/processor.rs
//
// IKEv2 协议处理逻辑

use super::*;
use super::crypto::{generate_dh_keypair, generate_random_nonce, compute_dh_shared, compute_key_material, generate_random_spi};
use std::sync::{Arc, Mutex};

// ========== IKEv2 协议处理器 ==========

/// IKEv2 协议处理器
pub struct IkeProcessor {
    /// SA 管理器
    sa_manager: Arc<Mutex<IkeSaManager>>,
    /// 本地地址
    local_addr: IpAddr,
    /// 配置
    config: IkeSaConfig,
}

impl IkeProcessor {
    /// 创建新的 IKEv2 处理器
    pub fn new(
        sa_manager: Arc<Mutex<IkeSaManager>>,
        local_addr: IpAddr,
        config: IkeSaConfig,
    ) -> Self {
        Self {
            sa_manager,
            local_addr,
            config,
        }
    }

    /// 处理接收到的 IKE 消息
    pub fn process_message(&self, message: &IkeMessage, remote_addr: IpAddr) -> IkeResult<Option<IkeMessage>> {
        // 查找或创建 IKE SA
        let (sa_entry, is_new) = self.lookup_or_create_sa(message, remote_addr)?;

        // 验证消息
        self.validate_message(message, &sa_entry)?;

        // 根据交换类型处理
        let response = match message.exchange_type() {
            IkeExchangeType::IkeSaInit => {
                self.handle_init_exchange(message, &sa_entry, is_new)?
            }
            IkeExchangeType::IkeAuth => {
                self.handle_auth_exchange(message, &sa_entry)?
            }
            IkeExchangeType::CreateChildSa => {
                self.handle_create_child_sa(message, &sa_entry)?
            }
            IkeExchangeType::Informational => {
                self.handle_informational(message, &sa_entry)?
            }
        };

        // 更新 SA
        self.update_sa(message, &sa_entry)?;

        Ok(response)
    }

    /// 查找或创建 IKE SA
    fn lookup_or_create_sa(&self, message: &IkeMessage, remote_addr: IpAddr) -> IkeResult<(IkeSaEntry, bool)> {
        let mut manager = self.sa_manager.lock().unwrap();

        // 尝试通过 SPI 查找现有 SA
        let spi = if message.header.responder_spi == [0u8; 8] {
            message.header.initiator_spi
        } else {
            message.header.responder_spi
        };

        if let Some(existing) = manager.get_by_spi(&spi) {
            return Ok((existing.clone(), false));
        }

        // 创建新的 SA（仅当接收到有效的请求时）
        if message.is_response() {
            return Err(IkeError::SaNotFound);
        }

        let initiator_spi = message.header.initiator_spi;
        let mut config = self.config.clone();
        config.local_addr = self.local_addr;
        config.remote_addr = remote_addr;

        let mut sa_entry = IkeSaEntry::new(config, initiator_spi);

        // 如果是响应方，设置状态
        if !message.header.flags.initiator {
            sa_entry.set_responder_spi(generate_random_spi());
            sa_entry.set_state(IkeSaState::InitSent);
        }

        Ok((sa_entry, true))
    }

    /// 验证消息
    fn validate_message(&self, message: &IkeMessage, sa_entry: &IkeSaEntry) -> IkeResult<()> {
        // 验证版本
        if message.header.version != IKEV2_VERSION {
            return Err(IkeError::ParseError("不支持的 IKE 版本".to_string()));
        }

        // 验证 SPI
        if message.header.initiator_spi == [0u8; 8] {
            return Err(IkeError::InvalidSpi);
        }

        // 验证消息长度
        if message.header.length as usize > 65535 {
            return Err(IkeError::InvalidLength);
        }

        Ok(())
    }

    /// 更新 SA
    fn update_sa(&self, message: &IkeMessage, sa_entry: &IkeSaEntry) -> IkeResult<()> {
        let mut manager = self.sa_manager.lock().unwrap();
        if let Some(sa) = manager.get_mut(&sa_entry.id) {
            // 更新 SA 状态（在具体交换处理中设置）
        }
        Ok(())
    }

    /// 处理 IKE_SA_INIT 交换
    fn handle_init_exchange(&self, message: &IkeMessage, sa_entry: &IkeSaEntry, is_new: bool) -> IkeResult<Option<IkeMessage>> {
        if message.is_response() {
            // 处理 IKE_SA_INIT 响应（发起方）
            self.handle_init_response(message, sa_entry)
        } else {
            // 处理 IKE_SA_INIT 请求（响应方）
            self.handle_init_request(message, sa_entry, is_new)
        }
    }

    /// 处理 IKE_SA_INIT 请求
    fn handle_init_request(&self, message: &IkeMessage, sa_entry: &IkeSaEntry, is_new: bool) -> IkeResult<Option<IkeMessage>> {
        // 提取发起方参数
        let sa_payload = message.get_payload(IkePayloadType::SA)
            .and_then(|p| if let IkePayload::SA(sa) = p { Some(sa) } else { None });
        let ke_payload = message.get_payload(IkePayloadType::KE)
            .and_then(|p| if let IkePayload::KE(ke) = p { Some(ke) } else { None });
        let nonce_payload = message.get_payload(IkePayloadType::Nonce)
            .and_then(|p| if let IkePayload::Nonce(n) = p { Some(n) } else { None });

        if sa_payload.is_none() || ke_payload.is_none() || nonce_payload.is_none() {
            return Err(IkeError::ParseError("缺少必需的 Payload".to_string()));
        }

        // 选择协商参数（简化：接受第一个提议）
        let selected_proposal = sa_payload.unwrap().proposals.first()
            .ok_or_else(|| IkeError::ParseError("没有提议".to_string()))?;

        // 生成响应方参数
        let responder_spi = generate_random_spi();
        let (dh_private, dh_public) = generate_dh_keypair(self.config.dh_group)?;
        let nr = generate_random_nonce(32);

        // 计算 DH 共享密钥
        let ke = ke_payload.unwrap();
        let dh_shared = compute_dh_shared(self.config.dh_group, &ke.public_key, &dh_private)?;

        // 派生密钥材料
        let ni = &nonce_payload.unwrap().nonce_data;
        let keymat = compute_key_material(
            ni,
            &nr,
            &dh_shared,
            &message.header.initiator_spi,
            &responder_spi,
            32,  // 加密密钥长度
            32,  // 认证密钥长度
        )?;

        // 构建响应消息
        let mut response_payloads = Vec::new();

        // SA Payload（接受提议）
        response_payloads.push(IkePayload::SA(IkeSaPayload {
            next_payload: IkePayloadType::KE,
            critical: false,
            proposals: vec![selected_proposal.clone()],
        }));

        // KE Payload
        response_payloads.push(IkePayload::KE(IkeKePayload {
            next_payload: IkePayloadType::Nonce,
            critical: false,
            dh_group: self.config.dh_group,
            public_key: dh_public,
        }));

        // Nonce Payload
        response_payloads.push(IkePayload::Nonce(IkeNoncePayload {
            next_payload: IkePayloadType::None,
            critical: false,
            nonce_data: nr,
        }));

        let mut header = message.header.response(IkePayloadType::SA, 0);
        header.responder_spi = responder_spi;

        let response = IkeMessage::new(header, response_payloads);

        Ok(Some(response))
    }

    /// 处理 IKE_SA_INIT 响应
    fn handle_init_response(&self, message: &IkeMessage, sa_entry: &IkeSaEntry) -> IkeResult<Option<IkeMessage>> {
        // 提取响应方参数
        let ke_payload = message.get_payload(IkePayloadType::KE)
            .and_then(|p| if let IkePayload::KE(ke) = p { Some(ke) } else { None });
        let nonce_payload = message.get_payload(IkePayloadType::Nonce)
            .and_then(|p| if let IkePayload::Nonce(n) = p { Some(n) } else { None });

        if ke_payload.is_none() || nonce_payload.is_none() {
            return Err(IkeError::ParseError("缺少必需的 Payload".to_string()));
        }

        // 在真实实现中，这里会计算 DH 共享密钥和派生密钥材料
        // 简化实现中，我们只是记录状态

        // 准备发送 IKE_AUTH 请求（将在下一步处理）
        Ok(None) // 返回 None 表示需要继续处理
    }

    /// 处理 IKE_AUTH 交换
    fn handle_auth_exchange(&self, message: &IkeMessage, sa_entry: &IkeSaEntry) -> IkeResult<Option<IkeMessage>> {
        if message.is_response() {
            self.handle_auth_response(message, sa_entry)
        } else {
            self.handle_auth_request(message, sa_entry)
        }
    }

    /// 处理 IKE_AUTH 请求
    fn handle_auth_request(&self, message: &IkeMessage, sa_entry: &IkeSaEntry) -> IkeResult<Option<IkeMessage>> {
        // 验证认证 Payload
        let auth_payload = message.get_payload(IkePayloadType::AUTH)
            .and_then(|p| if let IkePayload::AUTH(a) = p { Some(a) } else { None });

        if auth_payload.is_none() {
            return Err(IkeError::AuthenticationFailed);
        }

        // 在真实实现中，这里会验证 AUTH payload
        // 简化实现中，我们假设认证成功

        // 构建响应消息
        let id_payload = IkeIdPayload::new(
            IkePayloadType::AUTH,
            IkeIdType::ID_IPV4_ADDR,
            vec![0x0A, 0x00, 0x00, 0x01], // 10.0.0.1
        );

        let auth_data = vec![0u8; 32]; // 简化：假的认证数据
        let auth_payload = IkeAuthPayload::new(
            IkePayloadType::None,
            IkeAuthMethod::SHARED_KEY,
            auth_data,
        );

        let mut header = message.header.response(IkePayloadType::IDr, 0);

        let response = IkeMessage::new(
            header,
            vec![
                IkePayload::IDr(id_payload),
                IkePayload::AUTH(auth_payload),
            ],
        );

        Ok(Some(response))
    }

    /// 处理 IKE_AUTH 响应
    fn handle_auth_response(&self, message: &IkeMessage, sa_entry: &IkeSaEntry) -> IkeResult<Option<IkeMessage>> {
        // 验证认证 Payload
        let auth_payload = message.get_payload(IkePayloadType::AUTH)
            .and_then(|p| if let IkePayload::AUTH(a) = p { Some(a) } else { None });

        if auth_payload.is_none() {
            return Err(IkeError::AuthenticationFailed);
        }

        // IKE SA 建立完成
        Ok(None)
    }

    /// 处理 CREATE_CHILD_SA 交换
    fn handle_create_child_sa(&self, message: &IkeMessage, sa_entry: &IkeSaEntry) -> IkeResult<Option<IkeMessage>> {
        // 简化实现：返回确认响应
        let header = message.header.response(IkePayloadType::None, IKE_HEADER_LEN as u32);
        let response = IkeMessage::new(header, vec![]);
        Ok(Some(response))
    }

    /// 处理 INFORMATIONAL 交换
    fn handle_informational(&self, message: &IkeMessage, sa_entry: &IkeSaEntry) -> IkeResult<Option<IkeMessage>> {
        // 检查是否有删除 Payload
        let delete_payload = message.get_payload(IkePayloadType::Delete);

        if delete_payload.is_some() {
            // 处理 SA 删除
            // 简化实现：返回确认
            let header = message.header.response(IkePayloadType::None, IKE_HEADER_LEN as u32);
            let response = IkeMessage::new(header, vec![]);
            return Ok(Some(response));
        }

        // 其他 INFORMATIONAL 消息不需要响应
        Ok(None)
    }

    /// 发起 IKE SA 建立（发起方）
    pub fn initiate_sa(&self, remote_addr: IpAddr) -> IkeResult<IkeMessage> {
        let initiator_spi = generate_random_spi();
        let mut config = self.config.clone();
        config.local_addr = self.local_addr;
        config.remote_addr = remote_addr;

        // 创建 IKE SA 条目
        let sa_entry = IkeSaEntry::new(config, initiator_spi);

        // 生成 DH 密钥对和 Nonce
        let (dh_private, dh_public) = generate_dh_keypair(self.config.dh_group)?;
        let ni = generate_random_nonce(32);

        // 构建 SA Payload
        let proposal = IkeProposal {
            is_last: true,
            proposal_num: 1,
            protocol_id: IkeProtocolId::Ike,
            spi_size: 0,
            num_transforms: 4,
            spi: vec![],
            transforms: vec![
                IkeTransform {
                    is_last: false,
                    transform_type: IkeTransformType::Encryption,
                    transform_id: 14, // AES-CBC
                    attributes: vec![],
                },
                IkeTransform {
                    is_last: false,
                    transform_type: IkeTransformType::Prf,
                    transform_id: 2, // PRF_HMAC_SHA1
                    attributes: vec![],
                },
                IkeTransform {
                    is_last: false,
                    transform_type: IkeTransformType::Integrity,
                    transform_id: 2, // HMAC_SHA1
                    attributes: vec![],
                },
                IkeTransform {
                    is_last: true,
                    transform_type: IkeTransformType::DhGroup,
                    transform_id: self.config.dh_group.as_u16(),
                    attributes: vec![],
                },
            ],
        };

        let sa_payload = IkeSaPayload {
            next_payload: IkePayloadType::KE,
            critical: false,
            proposals: vec![proposal],
        };

        // 构建 KE Payload
        let ke_payload = IkeKePayload {
            next_payload: IkePayloadType::Nonce,
            critical: false,
            dh_group: self.config.dh_group,
            public_key: dh_public,
        };

        // 构建 Nonce Payload
        let nonce_payload = IkeNoncePayload {
            next_payload: IkePayloadType::None,
            critical: false,
            nonce_data: ni,
        };

        // 构建消息
        let mut header = IkeHeader::init_request(initiator_spi);
        header.next_payload = IkePayloadType::SA.as_u8();

        let message = IkeMessage::new(
            header,
            vec![
                IkePayload::SA(sa_payload),
                IkePayload::KE(ke_payload),
                IkePayload::Nonce(nonce_payload),
            ],
        );

        // 保存 SA
        let mut manager = self.sa_manager.lock().unwrap();
        manager.add(sa_entry);

        Ok(message)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::addr::Ipv4Addr;

    #[test]
    fn test_create_init_request() {
        let sa_manager = Arc::new(Mutex::new(IkeSaManager::new()));
        let config = IkeSaConfig::new(
            IkeRole::Initiator,
            IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1)),
            IpAddr::V4(Ipv4Addr::new(192, 168, 1, 2)),
            IkeDhGroup::MODP2048,
            IkeAuthMethod::SHARED_KEY,
        );

        let processor = IkeProcessor::new(
            sa_manager.clone(),
            IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1)),
            config,
        );

        let message = processor.initiate_sa(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 2))).unwrap();

        assert_eq!(message.header.exchange_type, IkeExchangeType::IkeSaInit);
        assert!(!message.header.is_response());
        assert!(!message.payloads.is_empty());
    }

    #[test]
    fn test_process_init_request() {
        let sa_manager = Arc::new(Mutex::new(IkeSaManager::new()));
        let config = IkeSaConfig::new(
            IkeRole::Responder,
            IpAddr::V4(Ipv4Addr::new(192, 168, 1, 2)),
            IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1)),
            IkeDhGroup::MODP2048,
            IkeAuthMethod::SHARED_KEY,
        );

        let processor = IkeProcessor::new(
            sa_manager.clone(),
            IpAddr::V4(Ipv4Addr::new(192, 168, 1, 2)),
            config,
        );

        // 创建 IKE_SA_INIT 请求
        let initiator_spi = generate_random_spi();
        let (_, dh_public) = generate_dh_keypair(IkeDhGroup::MODP2048).unwrap();
        let ni = generate_random_nonce(32);

        let proposal = IkeProposal {
            is_last: true,
            proposal_num: 1,
            protocol_id: IkeProtocolId::Ike,
            spi_size: 0,
            num_transforms: 1,
            spi: vec![],
            transforms: vec![IkeTransform {
                is_last: true,
                transform_type: IkeTransformType::DhGroup,
                transform_id: 14,
                attributes: vec![],
            }],
        };

        let sa_payload = IkeSaPayload {
            next_payload: IkePayloadType::KE,
            critical: false,
            proposals: vec![proposal],
        };

        let ke_payload = IkeKePayload {
            next_payload: IkePayloadType::Nonce,
            critical: false,
            dh_group: IkeDhGroup::MODP2048,
            public_key: dh_public,
        };

        let nonce_payload = IkeNoncePayload {
            next_payload: IkePayloadType::None,
            critical: false,
            nonce_data: ni,
        };

        let mut header = IkeHeader::init_request(initiator_spi);
        header.next_payload = IkePayloadType::SA.as_u8();

        let message = IkeMessage::new(
            header,
            vec![
                IkePayload::SA(sa_payload),
                IkePayload::KE(ke_payload),
                IkePayload::Nonce(nonce_payload),
            ],
        );

        let response = processor.process_message(
            &message,
            IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1)),
        ).unwrap();

        assert!(response.is_some());
        assert!(response.as_ref().unwrap().is_response());
    }
}
