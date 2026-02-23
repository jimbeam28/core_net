// src/common/packet.rs
//
// 报文描述符设计
// Packet是CoreNet的核心数据结构，用于在协议栈各层之间传递报文数据

/// 报文描述符
#[derive(Debug, Clone)]
pub struct Packet {
    /// 数据缓冲区
    pub data: Vec<u8>,
    /// 当前读取偏移量
    pub offset: usize,
    /// VLAN ID (从VLAN标签解析而来，0表示无VLAN)
    pub vlan_id: u16,
    /// 接口索引 (报文来自哪个接口，0表示未知)
    pub ifindex: u32,
}

impl Packet {
    // ========== 创建相关 ==========

    /// 创建新的空Packet
    pub fn new() -> Self {
        Packet {
            data: Vec::new(),
            offset: 0,
            vlan_id: 0,
            ifindex: 0,
        }
    }

    /// 从已有数据创建Packet
    pub fn from_bytes(data: Vec<u8>) -> Self {
        Packet {
            data,
            offset: 0,
            vlan_id: 0,
            ifindex: 0,
        }
    }

    // ========== 偏移相关 ==========

    /// 获取剩余可读取总长度
    pub fn remaining(&self) -> usize {
        self.data.len().saturating_sub(self.offset)
    }

    /// 检查是否有足够的数据可读
    pub fn has_remaining(&self, len: usize) -> bool {
        self.remaining() >= len
    }

    /// 读取指定字节数，不移动offset
    pub fn peek(&self, len: usize) -> Option<&[u8]> {
        if !self.has_remaining(len) {
            return None;
        }
        Some(&self.data[self.offset..self.offset + len])
    }

    /// 读取指定字节数，移动offset
    pub fn read(&mut self, len: usize) -> Option<&[u8]> {
        if !self.has_remaining(len) {
            return None;
        }
        let start = self.offset;
        self.offset += len;
        Some(&self.data[start..self.offset])
    }

    /// 跳过指定字节数
    pub fn skip(&mut self, len: usize) -> bool {
        if !self.has_remaining(len) {
            return false;
        }
        self.offset += len;
        true
    }

    /// 重置offset到指定位置
    pub fn seek(&mut self, offset: usize) -> bool {
        if offset > self.data.len() {
            return false;
        }
        self.offset = offset;
        true
    }

    // ========== 清空相关 ==========

    /// 清空所有数据
    pub fn clear(&mut self) {
        self.data.clear();
        self.offset = 0;
        self.vlan_id = 0;
        self.ifindex = 0;
    }

    // ========== 复制相关 ==========

    /// 复制Packet（深拷贝数据）
    pub fn clone_data(&self) -> Self {
        Packet {
            data: self.data.clone(),
            offset: self.offset,
            vlan_id: self.vlan_id,
            ifindex: self.ifindex,
        }
    }

    // ========== 数据操作 ==========

    /// 追加数据到buffer末尾
    pub fn extend(&mut self, data: &[u8]) {
        self.data.extend_from_slice(data);
    }

    /// 获取所有数据
    pub fn as_slice(&self) -> &[u8] {
        &self.data
    }

    /// 获取剩余可读数据的切片
    pub fn as_remaining_slice(&self) -> &[u8] {
        &self.data[self.offset..]
    }

    // ========== 查询相关 ==========

    /// 判断报文数据是否为空
    ///
    /// # 返回
    /// - `true`: 数据缓冲区为空
    /// - `false`: 数据缓冲区非空
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// 获取报文数据总长度
    ///
    /// # 返回
    /// 数据缓冲区的字节长度
    pub fn len(&self) -> usize {
        self.data.len()
    }

    /// 重置读取偏移量到0
    ///
    /// 将偏移量重置到数据起始位置，可以重新读取数据。
    pub fn reset(&mut self) {
        self.offset = 0;
    }
}

// ========== 实现Default trait ==========

impl Default for Packet {
    fn default() -> Self {
        Self::new()
    }
}
