use std::sync::{Mutex, OnceLock};

use crate::interface::InterfaceManager;
use crate::interface::types::InterfaceError;

/// 全局接口管理器
///
/// 使用 OnceLock + Mutex 实现线程安全的单例模式，支持运行时修改接口配置
static GLOBAL_MANAGER: OnceLock<Mutex<InterfaceManager>> = OnceLock::new();

/// 初始化全局接口管理器
///
/// # 参数
/// - `manager`: 要设置为全局的接口管理器
///
/// # 返回
/// - `Ok(())`: 初始化成功
/// - `Err(InterfaceError::InvalidFormat)`: 已经初始化过
pub fn init_global_manager(manager: InterfaceManager) -> Result<(), InterfaceError> {
    GLOBAL_MANAGER
        .set(Mutex::new(manager))
        .map_err(|_| InterfaceError::InvalidFormat("全局接口管理器已经初始化".to_string()))
}

/// 获取全局接口管理器的引用（只读）
///
/// # 返回
/// - `Some(&Mutex<InterfaceManager>)`: 如果已初始化
/// - `None`: 如果未初始化
pub fn global_manager() -> Option<&'static Mutex<InterfaceManager>> {
    GLOBAL_MANAGER.get()
}

/// 修改指定接口的配置
///
/// # 参数
/// - `name`: 接口名称
/// - `f`: 修改闭包，接收 `&mut NetworkInterface`
///
/// # 返回
/// - `Ok(())`: 修改成功
/// - `Err(InterfaceError)`: 修改失败
///
/// # 示例
/// ```rust
/// use crate::interface::{Ipv4Addr, MacAddr};
///
/// // 修改 IP 地址
/// update_interface("eth0", |iface| {
///     iface.set_ip_addr(Ipv4Addr::new(192, 168, 2, 100));
/// })?;
///
/// // 修改 MAC 地址
/// update_interface("eth0", |iface| {
///     iface.set_mac_addr(MacAddr::new([0x00, 0x11, 0x22, 0x33, 0x44, 0x56]));
/// })?;
/// ```
pub fn update_interface<F>(name: &str, f: F) -> Result<(), InterfaceError>
where
    F: FnOnce(&mut crate::interface::iface::NetworkInterface),
{
    let manager = GLOBAL_MANAGER
        .get()
        .ok_or_else(|| InterfaceError::InvalidFormat("全局接口管理器未初始化".to_string()))?;

    let mut guard = manager.lock().map_err(|e| {
        InterfaceError::MutexLockFailed(format!("锁定互斥锁失败: {}", e.to_string()))
    })?;

    let iface = guard.get_by_name_mut(name)?;
    f(iface);
    Ok(())
}

/// 设置接口的 IP 地址
///
/// # 参数
/// - `name`: 接口名称
/// - `addr`: 新的 IP 地址
pub fn set_interface_ip(name: &str, addr: crate::interface::types::Ipv4Addr) -> Result<(), InterfaceError> {
    update_interface(name, |iface| iface.set_ip_addr(addr))
}

/// 设置接口的 MAC 地址
///
/// # 参数
/// - `name`: 接口名称
/// - `addr`: 新的 MAC 地址
pub fn set_interface_mac(name: &str, addr: crate::interface::types::MacAddr) -> Result<(), InterfaceError> {
    update_interface(name, |iface| iface.set_mac_addr(addr))
}

/// 设置接口的子网掩码
///
/// # 参数
/// - `name`: 接口名称
/// - `mask`: 新的子网掩码
pub fn set_interface_netmask(name: &str, mask: crate::interface::types::Ipv4Addr) -> Result<(), InterfaceError> {
    update_interface(name, |iface| iface.set_netmask(mask))
}

/// 设置接口的网关
///
/// # 参数
/// - `name`: 接口名称
/// - `addr`: 新的网关地址
pub fn set_interface_gateway(name: &str, addr: Option<crate::interface::types::Ipv4Addr>) -> Result<(), InterfaceError> {
    update_interface(name, |iface| iface.set_gateway(addr))
}

/// 设置接口的 MTU
///
/// # 参数
/// - `name`: 接口名称
/// - `mtu`: 新的 MTU 值
pub fn set_interface_mtu(name: &str, mtu: u16) -> Result<(), InterfaceError> {
    update_interface(name, |iface| iface.set_mtu(mtu))
}

/// 启用接口
///
/// # 参数
/// - `name`: 接口名称
pub fn interface_up(name: &str) -> Result<(), InterfaceError> {
    update_interface(name, |iface| iface.up())
}

/// 禁用接口
///
/// # 参数
/// - `name`: 接口名称
pub fn interface_down(name: &str) -> Result<(), InterfaceError> {
    update_interface(name, |iface| iface.down())
}

/// 使用默认配置路径初始化全局接口管理器
///
/// # 参数
/// - `config_path`: 配置文件路径
/// - `rxq_capacity`: 每个接口的接收队列容量
/// - `txq_capacity`: 每个接口的发送队列容量
///
/// # 返回
/// - `Ok(())`: 初始化成功
/// - `Err(InterfaceError)`: 初始化失败
pub fn init_from_config(config_path: &str, rxq_capacity: usize, txq_capacity: usize) -> Result<(), InterfaceError> {
    let manager = crate::interface::load_config(config_path, rxq_capacity, txq_capacity)?;
    init_global_manager(manager)
}

/// 便捷宏：获取全局接口管理器，如果未初始化则 panic
#[macro_export]
macro_rules! get_interfaces {
    () => {
        $crate::interface::global::global_manager()
            .expect("全局接口管理器未初始化，请先调用 init_global_manager() 或 init_from_config()")
    };
}
