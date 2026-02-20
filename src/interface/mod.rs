//! 网络接口模块，负责管理网络接口的配置和状态

mod types;
mod iface;
mod manager;
mod config;

pub use types::{MacAddr, Ipv4Addr, InterfaceState, InterfaceType, InterfaceError};
pub use iface::{NetworkInterface, InterfaceConfig};
pub use manager::InterfaceManager;
pub use config::{load_default_config, save_config, InterfaceModuleConfig, DEFAULT_CONFIG_PATH};
