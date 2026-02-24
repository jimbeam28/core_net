// src/protocols/vlan/filter.rs
//
// VLAN 过滤功能
// 实现 802.1Q VLAN 帧过滤逻辑

use super::tag::VlanTag;
use super::error::VlanError;
use std::collections::HashSet;

/// VLAN 过滤器
///
/// 管理允许通过的 VLAN ID 列表，用于实现基于 VLAN 的访问控制。
#[derive(Debug, Clone)]
pub struct VlanFilter {
    /// 允许的 VLAN ID 集合
    allowed_vlans: HashSet<u16>,

    /// 拒绝的 VLAN ID 集合（优先级高于允许列表）
    denied_vlans: HashSet<u16>,

    /// 是否启用过滤（false = 接受所有 VLAN）
    enabled: bool,
}

impl VlanFilter {
    /// 创建新的 VLAN 过滤器
    ///
    /// # 参数
    /// - enabled: 是否启用过滤
    ///
    /// # 返回
    /// - 新的 VLAN 过滤器实例
    pub fn new(enabled: bool) -> Self {
        Self {
            allowed_vlans: HashSet::new(),
            denied_vlans: HashSet::new(),
            enabled,
        }
    }

    /// 创建禁用的过滤器（接受所有 VLAN）
    pub fn disabled() -> Self {
        Self::new(false)
    }

    /// 创建启用的过滤器（需要配置允许列表）
    pub fn enabled() -> Self {
        Self::new(true)
    }

    /// 启用过滤
    pub fn enable(&mut self) {
        self.enabled = true;
    }

    /// 禁用过滤（接受所有 VLAN）
    pub fn disable(&mut self) {
        self.enabled = false;
    }

    /// 检查过滤是否启用
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// 添加允许的 VLAN ID
    ///
    /// # 参数
    /// - vid: VLAN ID
    ///
    /// # 返回
    /// - Ok(()): 添加成功
    /// - Err(VlanError): VLAN ID 无效
    pub fn allow_vlan(&mut self, vid: u16) -> Result<(), VlanError> {
        if !VlanTag::is_valid_vid(vid) {
            return Err(VlanError::InvalidVlanId { vid });
        }
        self.allowed_vlans.insert(vid);
        // 从拒绝列表中移除（如果存在）
        self.denied_vlans.remove(&vid);
        Ok(())
    }

    /// 移除允许的 VLAN ID
    ///
    /// # 参数
    /// - vid: VLAN ID
    ///
    /// # 返回
    /// - true: VLAN ID 曾在允许列表中
    /// - false: VLAN ID 不在允许列表中
    pub fn remove_allowed_vlan(&mut self, vid: u16) -> bool {
        self.allowed_vlans.remove(&vid)
    }

    /// 添加拒绝的 VLAN ID
    ///
    /// # 参数
    /// - vid: VLAN ID
    ///
    /// # 返回
    /// - Ok(()): 添加成功
    /// - Err(VlanError): VLAN ID 无效
    pub fn deny_vlan(&mut self, vid: u16) -> Result<(), VlanError> {
        if !VlanTag::is_valid_vid(vid) {
            return Err(VlanError::InvalidVlanId { vid });
        }
        self.denied_vlans.insert(vid);
        // 从允许列表中移除（如果存在）
        self.allowed_vlans.remove(&vid);
        Ok(())
    }

    /// 移除拒绝的 VLAN ID
    ///
    /// # 参数
    /// - vid: VLAN ID
    ///
    /// # 返回
    /// - true: VLAN ID 曾在拒绝列表中
    /// - false: VLAN ID 不在拒绝列表中
    pub fn remove_denied_vlan(&mut self, vid: u16) -> bool {
        self.denied_vlans.remove(&vid)
    }

    /// 清空所有规则
    pub fn clear(&mut self) {
        self.allowed_vlans.clear();
        self.denied_vlans.clear();
    }

    /// 检查 VLAN 帧是否应该被接受
    ///
    /// # 参数
    /// - vlan_tag: VLAN 标签
    ///
    /// # 返回
    /// - true: 接受此帧
    /// - false: 拒绝此帧
    ///
    /// # 过滤逻辑
    /// 1. 如果过滤未启用，接受所有帧
    /// 2. 如果 VLAN ID 在拒绝列表中，拒绝
    /// 3. 如果允许列表为空，接受所有未拒绝的帧
    /// 4. 如果 VLAN ID 在允许列表中，接受
    /// 5. 否则拒绝
    pub fn should_accept(&self, vlan_tag: &VlanTag) -> bool {
        // 如果过滤未启用，接受所有帧
        if !self.enabled {
            return true;
        }

        let vid = vlan_tag.vid;

        // 检查拒绝列表（优先级最高）
        if self.denied_vlans.contains(&vid) {
            return false;
        }

        // 如果允许列表为空，接受所有未拒绝的帧
        if self.allowed_vlans.is_empty() {
            return true;
        }

        // 检查允许列表
        self.allowed_vlans.contains(&vid)
    }

    /// 检查未标记帧是否应该被接受
    ///
    /// 对于没有 VLAN 标签的帧，通常接受（因为它们属于默认 VLAN）
    ///
    /// # 返回
    /// - true: 接受此帧
    /// - false: 拒绝此帧
    pub fn should_accept_untagged(&self) -> bool {
        // 未标记帧通常属于默认 VLAN，始终接受
        // 实际实现中可能需要更复杂的策略
        true
    }

    /// 获取允许的 VLAN ID 列表
    pub fn allowed_vlans(&self) -> Vec<u16> {
        let mut vlans: Vec<_> = self.allowed_vlans.iter().copied().collect();
        vlans.sort();
        vlans
    }

    /// 获取拒绝的 VLAN ID 列表
    pub fn denied_vlans(&self) -> Vec<u16> {
        let mut vlans: Vec<_> = self.denied_vlans.iter().copied().collect();
        vlans.sort();
        vlans
    }

    /// 获取允许的 VLAN ID 数量
    pub fn allowed_count(&self) -> usize {
        self.allowed_vlans.len()
    }

    /// 获取拒绝的 VLAN ID 数量
    pub fn denied_count(&self) -> usize {
        self.denied_vlans.len()
    }
}

impl Default for VlanFilter {
    fn default() -> Self {
        Self::disabled()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vlan_filter_new() {
        let filter = VlanFilter::new(false);
        assert!(!filter.is_enabled());
        assert_eq!(filter.allowed_count(), 0);
        assert_eq!(filter.denied_count(), 0);
    }

    #[test]
    fn test_vlan_filter_default() {
        let filter = VlanFilter::default();
        assert!(!filter.is_enabled());
    }

    #[test]
    fn test_vlan_filter_disabled() {
        let filter = VlanFilter::disabled();
        assert!(!filter.is_enabled());
    }

    #[test]
    fn test_vlan_filter_enabled() {
        let filter = VlanFilter::enabled();
        assert!(filter.is_enabled());
    }

    #[test]
    fn test_enable_disable() {
        let mut filter = VlanFilter::disabled();
        assert!(!filter.is_enabled());

        filter.enable();
        assert!(filter.is_enabled());

        filter.disable();
        assert!(!filter.is_enabled());
    }

    #[test]
    fn test_allow_vlan() {
        let mut filter = VlanFilter::enabled();
        assert!(filter.allow_vlan(100).is_ok());
        assert_eq!(filter.allowed_count(), 1);

        // 不允许无效的 VLAN ID
        assert!(filter.allow_vlan(0).is_err());
        assert!(filter.allow_vlan(4095).is_err());
    }

    #[test]
    fn test_deny_vlan() {
        let mut filter = VlanFilter::enabled();
        assert!(filter.deny_vlan(200).is_ok());
        assert_eq!(filter.denied_count(), 1);

        // 不允许无效的 VLAN ID
        assert!(filter.deny_vlan(0).is_err());
        assert!(filter.deny_vlan(4095).is_err());
    }

    #[test]
    fn test_allow_then_deny() {
        let mut filter = VlanFilter::enabled();

        // 先允许
        assert!(filter.allow_vlan(100).is_ok());
        assert_eq!(filter.allowed_count(), 1);

        // 后拒绝（允许列表中会被移除）
        assert!(filter.deny_vlan(100).is_ok());
        assert_eq!(filter.allowed_count(), 0);
        assert_eq!(filter.denied_count(), 1);
    }

    #[test]
    fn test_deny_then_allow() {
        let mut filter = VlanFilter::enabled();

        // 先拒绝
        assert!(filter.deny_vlan(100).is_ok());
        assert_eq!(filter.denied_count(), 1);

        // 后允许（拒绝列表中会被移除）
        assert!(filter.allow_vlan(100).is_ok());
        assert_eq!(filter.denied_count(), 0);
        assert_eq!(filter.allowed_count(), 1);
    }

    #[test]
    fn test_should_accept_disabled() {
        let filter = VlanFilter::disabled();
        let tag = VlanTag::new(0, false, 100).unwrap();

        // 禁用状态下，接受所有 VLAN
        assert!(filter.should_accept(&tag));
    }

    #[test]
    fn test_should_accept_denied() {
        let mut filter = VlanFilter::enabled();
        filter.deny_vlan(100).unwrap();

        let tag = VlanTag::new(0, false, 100).unwrap();

        // 拒绝列表中的 VLAN 被拒绝
        assert!(!filter.should_accept(&tag));
    }

    #[test]
    fn test_should_accept_allowed() {
        let mut filter = VlanFilter::enabled();
        filter.allow_vlan(100).unwrap();

        let tag = VlanTag::new(0, false, 100).unwrap();

        // 允许列表中的 VLAN 被接受
        assert!(filter.should_accept(&tag));
    }

    #[test]
    fn test_should_accept_empty_allowed_list() {
        let filter = VlanFilter::enabled();
        let tag = VlanTag::new(0, false, 100).unwrap();

        // 允许列表为空时，接受所有未拒绝的帧
        assert!(filter.should_accept(&tag));
    }

    #[test]
    fn test_should_accept_not_in_allowed_list() {
        let mut filter = VlanFilter::enabled();
        filter.allow_vlan(100).unwrap();
        filter.allow_vlan(200).unwrap();

        let tag = VlanTag::new(0, false, 300).unwrap();

        // 不在允许列表中的 VLAN 被拒绝
        assert!(!filter.should_accept(&tag));
    }

    #[test]
    fn test_should_accept_untagged() {
        let filter = VlanFilter::enabled();
        assert!(filter.should_accept_untagged());
    }

    #[test]
    fn test_remove_allowed_vlan() {
        let mut filter = VlanFilter::enabled();
        filter.allow_vlan(100).unwrap();
        assert_eq!(filter.allowed_count(), 1);

        assert!(filter.remove_allowed_vlan(100));
        assert_eq!(filter.allowed_count(), 0);

        // 再次移除返回 false
        assert!(!filter.remove_allowed_vlan(100));
    }

    #[test]
    fn test_remove_denied_vlan() {
        let mut filter = VlanFilter::enabled();
        filter.deny_vlan(100).unwrap();
        assert_eq!(filter.denied_count(), 1);

        assert!(filter.remove_denied_vlan(100));
        assert_eq!(filter.denied_count(), 0);

        // 再次移除返回 false
        assert!(!filter.remove_denied_vlan(100));
    }

    #[test]
    fn test_clear() {
        let mut filter = VlanFilter::enabled();
        filter.allow_vlan(100).unwrap();
        filter.allow_vlan(200).unwrap();
        filter.deny_vlan(300).unwrap();

        assert_eq!(filter.allowed_count(), 2);
        assert_eq!(filter.denied_count(), 1);

        filter.clear();

        assert_eq!(filter.allowed_count(), 0);
        assert_eq!(filter.denied_count(), 0);
    }

    #[test]
    fn test_allowed_vlans_sorted() {
        let mut filter = VlanFilter::enabled();
        filter.allow_vlan(300).unwrap();
        filter.allow_vlan(100).unwrap();
        filter.allow_vlan(200).unwrap();

        let vlans = filter.allowed_vlans();
        assert_eq!(vlans, vec![100, 200, 300]);
    }

    #[test]
    fn test_denied_vlans_sorted() {
        let mut filter = VlanFilter::enabled();
        filter.deny_vlan(300).unwrap();
        filter.deny_vlan(100).unwrap();
        filter.deny_vlan(200).unwrap();

        let vlans = filter.denied_vlans();
        assert_eq!(vlans, vec![100, 200, 300]);
    }

    #[test]
    fn test_vlan_id_0_reserved() {
        let mut filter = VlanFilter::enabled();
        // VLAN ID 0 是保留的，应该被拒绝
        assert!(filter.allow_vlan(0).is_err());
    }

    #[test]
    fn test_vlan_id_4095_reserved() {
        let mut filter = VlanFilter::enabled();
        // VLAN ID 4095 是保留的，应该被拒绝
        assert!(filter.allow_vlan(4095).is_err());
    }

    #[test]
    fn test_valid_vlan_ranges() {
        let mut filter = VlanFilter::enabled();
        // 测试边界值
        assert!(filter.allow_vlan(1).is_ok());
        assert!(filter.allow_vlan(4094).is_ok());
    }
}
