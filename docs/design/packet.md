# 报文描述符设计

## 1. 概述

Packet是CoreNet中核心的数据结构，用于在协议栈各层之间传递报文数据。它封装了原始buffer和基本的读写状态。

## 2. 核心结构

```rust
/// 报文描述符
pub struct Packet {
    /// 数据缓冲区
    pub data: Vec<u8>,
    /// 当前读取偏移量
    pub offset: usize,
}
```

## 3. 主要方法

```rust
impl Packet {
    // === 创建相关 ===

    /// 创建新的空Packet
    pub fn new() -> Self;

    /// 从已有数据创建Packet
    pub fn from_bytes(data: Vec<u8>) -> Self;

    // === 偏移相关 ===

    /// 获取剩余可读取总长度
    pub fn remaining(&self) -> usize;

    /// 检查是否有足够的数据可读
    pub fn has_remaining(&self, len: usize) -> bool;

    /// 读取指定字节数，不移动offset
    pub fn peek(&self, len: usize) -> Option<&[u8]>;

    /// 读取指定字节数，移动offset
    pub fn read(&mut self, len: usize) -> Option<&[u8]>;

    /// 跳过指定字节数
    pub fn skip(&mut self, len: usize) -> bool;

    /// 重置offset到指定位置
    pub fn seek(&mut self, offset: usize) -> bool;

    // === 清空相关 ===

    /// 清空所有数据
    pub fn clear(&mut self);

    // === 复制相关 ===

    /// 复制Packet（深拷贝数据）
    pub fn clone(&self) -> Self;

    // === 数据操作 ===

    /// 追加数据到buffer末尾
    pub fn extend(&mut self, data: &[u8]);

    /// 获取所有数据
    pub fn as_slice(&self) -> &[u8];

    /// 获取剩余可读数据的切片
    pub fn as_remaining_slice(&self) -> &[u8];

    // === 查询相关 ===

    /// 判断是否为空
    pub fn is_empty(&self) -> bool;

    /// 获取总长度
    pub fn len(&self) -> usize;

    /// 重置offset到0
    pub fn reset(&mut self);

    /// 获取当前offset位置
    pub fn get_offset(&self) -> usize;
}
```

## 4. 内存布局

### 4.1 Packet布局

```
┌─────────────────────────────────────────────────────────┐
│                    Packet                               │
├─────────────────────────────────────────────────────────┤
│  已读数据  │           可读数据                          │
│           │                                             │
├───────────┼─────────────────────────────────────────────┤
0           offset                                         len
```

- **data**: Vec<u8> 存储所有报文数据
- **offset**: 当前读取位置，从0开始，最大为data.len()

## 5. 注意事项

1. **所有权转移**: Packet在协议层之间传递时转移所有权，避免不必要的拷贝
2. **切片引用**: 读操作返回切片引用 (&[u8])，实现零拷贝
3. **深拷贝**: clone() 方法会复制底层数据，用于需要保留副本的场景
4. **偏移管理**: 读取操作会自动移动offset，peek操作不影响offset
