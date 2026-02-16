//! 网络接口模块
//!
//! 负责管理网络接口的配置和状态。

mod types;
mod iface;
mod manager;
mod config;
mod global;

pub use types::{MacAddr, Ipv4Addr, InterfaceState, InterfaceType, InterfaceError};
pub use iface::{NetworkInterface, InterfaceConfig};
pub use manager::InterfaceManager;
pub use config::{load_default_config, save_config, InterfaceModuleConfig, DEFAULT_CONFIG_PATH};
pub use global::{
    init_global_manager,
    init_default,
    global_manager,
    update_interface,
    set_interface_ip,
    set_interface_mac,
    set_interface_netmask,
    set_interface_gateway,
    set_interface_mtu,
    interface_up,
    interface_down,
};
