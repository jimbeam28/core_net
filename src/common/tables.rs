// src/common/tables.rs
//
// 通用表管理模块
// 提供各种网络表的统一接口和实现

// 通用表接口 trait
// 所有表类型都应实现此 trait 以提供统一的操作接口
pub trait Table<K, V> {
    /// 查找表项
    fn lookup(&self, key: &K) -> Option<&V>;

    /// 查找并返回可变引用
    fn lookup_mut(&mut self, key: &K) -> Option<&mut V>;

    /// 插入或更新表项
    fn insert(&mut self, key: K, value: V) -> Option<V>;

    /// 删除表项
    fn remove(&mut self, key: &K) -> Option<V>;

    /// 清空所有表项
    fn clear(&mut self);

    /// 获取表项数量
    fn len(&self) -> usize;

    /// 检查是否为空
    fn is_empty(&self) -> bool;

    /// 清理过期表项
    fn cleanup(&mut self);
}
