// src/protocols/tcp/mod.rs
//
// TCP 协议模块
// 实现 RFC 793 / RFC 9293 Transmission Control Protocol

mod constant;
mod config;
mod error;
mod header;
mod segment;
mod tcb;
mod connection;
mod process;
mod socket;
mod socket_manager;

pub use constant::*;
pub use config::{TcpConfig, TCP_CONFIG_DEFAULT};
pub use error::TcpError;
pub use header::TcpHeader;
pub use segment::TcpSegment;
pub use tcb::{Tcb, TcpConnectionId, TcpState, SentSegment};
pub use connection::{TcpConnectionManager, TcpOption};
pub use process::{
    TcpProcessResult,
    process_tcp_packet,
    encapsulate_tcp_segment,
    create_syn,
    create_ack,
    create_fin,
    create_rst,
};
pub use socket::{TcpSocket, TcpEvent, TcpCallback};
pub use socket_manager::TcpSocketManager;
