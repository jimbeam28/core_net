// src/engine/processor.rs
//
// 报文处理器
// 提供报文处理接口，负责逐层解析/封装报文

use crate::common::Packet;

/// 报文处理结果
pub type ProcessResult = Result<(), ProcessError>;

/// 报文处理错误
#[derive(Debug)]
pub enum ProcessError {
    /// 报文解析错误
    ParseError(String),

    /// 报文封装错误
    EncapError(String),

    /// 不支持的协议
    UnsupportedProtocol(String),

    /// 报文格式错误
    InvalidPacket(String),
}

impl std::fmt::Display for ProcessError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProcessError::ParseError(msg) => write!(f, "解析错误: {}", msg),
            ProcessError::EncapError(msg) => write!(f, "封装错误: {}", msg),
            ProcessError::UnsupportedProtocol(proto) => write!(f, "不支持的协议: {}", proto),
            ProcessError::InvalidPacket(msg) => write!(f, "报文格式错误: {}", msg),
        }
    }
}

impl std::error::Error for ProcessError {}

// ========== 错误转换 ==========

/// 从 CoreError 转换
impl From<crate::common::CoreError> for ProcessError {
    fn from(err: crate::common::CoreError) -> Self {
        match err {
            crate::common::CoreError::ParseError(msg) => {
                ProcessError::ParseError(msg)
            }
            crate::common::CoreError::InvalidPacket(msg) => {
                ProcessError::InvalidPacket(msg)
            }
            crate::common::CoreError::UnsupportedProtocol(proto) => {
                ProcessError::UnsupportedProtocol(proto)
            }
            _ => ProcessError::EncapError(format!("{:?}", err)),
        }
    }
}

/// 报文处理器
///
/// 负责对报文进行协议解析（上行）或封装（下行）处理。
/// 目前为简化实现，仅打印报文内容，后续会逐步完善。
pub struct PacketProcessor {
    /// 处理器名称
    name: String,

    /// 是否启用详细输出
    verbose: bool,
}

impl PacketProcessor {
    /// 创建新的报文处理器
    pub fn new() -> Self {
        Self {
            name: "DefaultProcessor".to_string(),
            verbose: false,
        }
    }

    /// 创建命名处理器
    pub fn with_name(name: String) -> Self {
        Self {
            name,
            verbose: false,
        }
    }

    /// 启用详细输出
    pub fn with_verbose(mut self, verbose: bool) -> Self {
        self.verbose = verbose;
        self
    }

    /// 获取处理器名称
    pub fn name(&self) -> &str {
        &self.name
    }

    /// 处理报文（上行解析）
    ///
    /// # 参数
    /// - packet: 要处理的报文（按值传递，取得所有权）
    ///
    /// # 返回
    /// - Ok(()): 处理成功
    /// - Err(ProcessError): 处理失败
    pub fn process(&self, packet: Packet) -> ProcessResult {
        self.print_packet(&packet);
        Ok(())
    }

    /// 打印报文信息（简化实现）
    fn print_packet(&self, packet: &Packet) {
        if self.verbose {
            println!("=== 报文处理 [{}] ===", self.name);
            println!("报文长度: {} 字节", packet.len());
            println!("当前偏移: {} 字节", packet.get_offset());
            println!("剩余数据: {} 字节", packet.remaining());
            println!("报文内容:");
            self.print_hexdump(packet.as_slice());
            println!("====================");
        } else {
            println!("报文处理 [{}]: 长度={} 字节", self.name, packet.len());
        }
    }

    /// 打印十六进制格式
    fn print_hexdump(&self, data: &[u8]) {
        let mut i = 0;
        while i < data.len() {
            // 打印偏移量
            print!("{:04x}: ", i);

            // 打印十六进制（每行16字节）
            for j in 0..16 {
                if i + j < data.len() {
                    print!("{:02x} ", data[i + j]);
                } else {
                    print!("   ");
                }
                if j == 7 {
                    print!(" ");
                }
            }

            print!(" |");

            // 打印ASCII
            for j in 0..16 {
                if i + j < data.len() {
                    let b = data[i + j];
                    if b.is_ascii_graphic() || b == b' ' {
                        print!("{}", b as char);
                    } else {
                        print!(".");
                    }
                }
            }

            println!("|");
            i += 16;
        }
    }
}

impl Default for PacketProcessor {
    fn default() -> Self {
        Self::new()
    }
}

/// 便捷函数：处理报文
///
/// 使用默认处理器处理报文。
///
/// # 参数
/// - packet: 要处理的报文
///
/// # 返回
/// - Ok(()): 处理成功
/// - Err(ProcessError): 处理失败
pub fn process_packet(packet: Packet) -> ProcessResult {
    PacketProcessor::new().process(packet)
}

/// 便捷函数：详细模式处理报文
///
/// 使用详细输出模式处理报文。
///
/// # 参数
/// - packet: 要处理的报文
///
/// # 返回
/// - Ok(()): 处理成功
/// - Err(ProcessError): 处理失败
pub fn process_packet_verbose(packet: Packet) -> ProcessResult {
    PacketProcessor::new().with_verbose(true).process(packet)
}
