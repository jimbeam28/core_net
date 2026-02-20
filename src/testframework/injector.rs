//! 报文注入器
//!
//! 负责将测试报文放入指定接口的接收队列。

use crate::common::Packet;
use crate::interface::InterfaceManager;
use crate::context::SystemContext;
use crate::testframework::error::{HarnessError, HarnessResult};

/// 报文注入器
pub struct PacketInjector<'a> {
    /// 接口管理器的可变引用（直接模式）
    interfaces: Option<&'a mut InterfaceManager>,

    /// 系统上下文的引用（SystemContext 模式）
    context: Option<&'a SystemContext>,
}

impl<'a> PacketInjector<'a> {
    /// 创建新的注入器（直接模式）
    ///
    /// # 参数
    /// - interfaces: 接口管理器的可变引用
    pub fn new(interfaces: &'a mut InterfaceManager) -> Self {
        Self {
            interfaces: Some(interfaces),
            context: None,
        }
    }

    /// 使用系统上下文创建注入器
    ///
    /// # 参数
    /// - context: 系统上下文的引用
    pub fn with_context(context: &'a SystemContext) -> Self {
        Self {
            interfaces: None,
            context: Some(context),
        }
    }

    /// 向指定接口注入单个报文
    ///
    /// # 参数
    /// - interface_name: 接口名称
    /// - packet: 要注入的报文
    ///
    /// # 返回
    /// - Ok(()): 注入成功
    /// - Err(HarnessError): 注入失败（接口未找到、队列满等）
    pub fn inject(
        &mut self,
        interface_name: &str,
        packet: Packet,
    ) -> HarnessResult<()> {
        if let Some(interfaces) = &mut self.interfaces {
            // 直接模式：使用接口管理器的可变引用
            let iface = interfaces.get_by_name_mut(interface_name)?;
            iface.rxq.enqueue(packet).map_err(|e| HarnessError::QueueError(format!("{:?}", e)))?;
            Ok(())
        } else if let Some(context) = self.context {
            // SystemContext 模式：使用上下文中的接口
            let mut interfaces = context.interfaces.lock()
                .map_err(|e| HarnessError::QueueError(format!("锁定接口管理器失败: {}", e)))?;
            let iface = interfaces.get_by_name_mut(interface_name)?;
            iface.rxq.enqueue(packet).map_err(|e| HarnessError::QueueError(format!("{:?}", e)))?;
            Ok(())
        } else {
            Err(HarnessError::InterfaceError("注入器未正确初始化".to_string()))
        }
    }

    /// 向指定接口注入多个报文
    ///
    /// # 参数
    /// - interface_name: 接口名称
    /// - packets: 要注入的报文列表
    ///
    /// # 返回
    /// - Ok(count): 成功注入的报文数量
    /// - Err(HarnessError): 注入失败
    pub fn inject_multiple(
        &mut self,
        interface_name: &str,
        packets: Vec<Packet>,
    ) -> HarnessResult<usize> {
        let mut count = 0;

        for packet in packets {
            self.inject(interface_name, packet)?;
            count += 1;
        }

        Ok(count)
    }
}
