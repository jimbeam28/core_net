# 测试工具设计

## 1. 概述

CoreNet采用纯模拟模式，通过报文注入器向接收队列注入测试报文，处理结果通过发送队列收集，由结果输出模块打印或保存。

## 2. 模块结构

```
src/test/
├── mod.rs
├── injector.rs      # 报文注入器
└── output.rs        # 结果输出
```

## 3. 报文注入器

### 3.1 功能

从不同来源创建Packet并注入到接收队列

### 3.2 API设计

```rust
pub struct PacketInjector {
    rx_queue: Arc<SafeQueue<Packet>>,
}

impl PacketInjector {
    pub fn new(rx_queue: Arc<SafeQueue<Packet>>) -> Self;

    /// 从hex字符串注入
    /// 格式: "ff ff ff ff ff ff ff aa bb cc dd ee ff 08 00 ..."
    pub fn inject_from_hex(&self, hex: &str) -> Result<()>;

    /// 从字节数组注入
    pub fn inject_from_bytes(&self, data: Vec<u8>) -> Result<()>;

    /// 从文件注入（pcap格式）
    pub fn inject_from_file(&self, path: &str) -> Result<()>;

    /// 注入以太网帧
    pub fn inject_ethernet_frame(
        &self,
        src_mac: MacAddr,
        dst_mac: MacAddr,
        ether_type: EtherType,
        payload: Vec<u8>,
    ) -> Result<()>;

    /// 注入IPv4报文（自动封装以太网头）
    pub fn inject_ipv4_packet(
        &self,
        src_ip: Ipv4Addr,
        dst_ip: Ipv4Addr,
        protocol: IpProtocol,
        payload: Vec<u8>,
    ) -> Result<()>;

    /// 注入ICMP echo请求（ping）
    pub fn inject_ping_request(
        &self,
        dst_ip: Ipv4Addr,
        identifier: u16,
        sequence: u16,
    ) -> Result<()>;
}
```

### 3.3 使用示例

```rust
// 创建注入器
let injector = PacketInjector::new(rx_queue);

// 从hex字符串注入以太网帧
let hex = "ff ff ff ff ff ff ff aa bb cc dd ee ff 08 00 45 00...";
injector.inject_from_hex(hex)?;

// 注入ping请求
injector.inject_ping_request("192.168.1.1".parse()?, 1234, 1)?;

// 从文件注入pcap
injector.inject_from_file("test_packets.pcap")?;
```

## 4. 结果输出

### 4.1 功能

从发送队列读取处理结果并输出

### 4.2 API设计

```rust
pub struct OutputCollector {
    tx_queue: Arc<SafeQueue<Packet>>,
    mode: OutputMode,
}

pub enum OutputMode {
    Print,           // 打印到终端
    Save(String),    // 保存到文件
    None,           // 仅计数
}

impl OutputCollector {
    pub fn new(tx_queue: Arc<SafeQueue<Packet>>, mode: OutputMode) -> Self;

    /// 收集并输出一个报文
    pub fn collect_one(&self) -> Result<bool>;  // 返回是否还有更多

    /// 收集所有报文（阻塞直到队列关闭）
    pub fn collect_all(&self) -> Result<()>;

    /// 获取统计信息
    pub fn stats(&self) -> OutputStats;
}

pub struct OutputStats {
    pub received: usize,      // 接收数量
    pub sent: usize,         // 发送数量
    pub bytes_received: usize,
    pub bytes_sent: usize,
}
```

### 4.3 输出格式

```
=== 发送报文 #1 ===
时间: 2024-01-01 12:00:00.123
长度: 98 字节

以太网头:
  源MAC: AA:BB:CC:DD:EE:FF
  目的MAC: FF:FF:FF:FF:FF:FF
  类型: 0x0800 (IPv4)

IP头:
  源IP: 192.168.1.10
  目的IP: 192.168.1.1
  协议: ICMP (1)

ICMP:
  类型: Echo Reply (0)
  代码: 0
  标识符: 1234
  序列号: 1

Payload:
  00 01 02 03 04 05 06 07 08 09 0a 0b 0c 0d 0e 0f
  10 11 12 13 14 15 16 17 18 19 1a 1b 1c 1d 1e 1f
```

## 5. 交互式测试

### 5.1 REPL模式

```rust
pub struct TestRepl {
    injector: PacketInjector,
    output: OutputCollector,
    engine: ProtocolEngine,
}

impl TestRepl {
    pub fn new(/* ... */) -> Self;

    /// 启动交互式测试
    pub fn run(&mut self) -> Result<()> {
        println!("CoreNet 协议栈测试 REPL");
        println!("输入 'help' 查看命令");

        loop {
            print!("> ");
            io::stdout().flush()?;

            let mut input = String::new();
            io::stdin().read_line(&mut input)?;

            match self.parse_and_execute(&input)? {
                CommandResult::Continue => {},
                CommandResult::Quit => break,
            }
        }
        Ok(())
    }

    fn parse_and_execute(&mut self, input: &str) -> Result<CommandResult>;
}

pub enum CommandResult {
    Continue,
    Quit,
}
```

### 5.2 REPL命令

```
> help
  可用命令:
    inject <hex>       - 从hex字符串注入报文
    ping <ip>         - 发送ping请求
    send <file>       - 从文件发送pcap报文
    stats             - 显示统计信息
    output <mode>     - 设置输出模式 (print/none)
    quit              - 退出

> inject ff ff ff ff ff ff ff aa bb cc dd ee ff 08 00 45 00...
  [✓] 已注入: 42 字节

> ping 192.168.1.1
  [✓] 已注入 ICMP Echo Request
  [→] 收到 ICMP Echo Reply from 192.168.1.1
      RTT: 2.345ms

> stats
  接收: 5
  发送: 5
  接收字节: 490
  发送字节: 490

> quit
```

## 6. 单元测试

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ethernet_parse() {
        // 创建测试报文
        let mut packet = create_ethernet_packet();
        // 解析
        let frame = EthernetFrame::parse(&mut packet).unwrap();
        // 验证
        assert_eq!(frame.src, MacAddr::new(0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF));
        assert_eq!(frame.ether_type, EtherType::IPv4);
    }

    #[test]
    fn test_ipv4_checksum() {
        let packet = create_ipv4_packet();
        assert!(verify_ipv4_checksum(&packet));
    }

    #[test]
    fn test_icmp_ping() {
        let injector = setup_test_injector();
        let output = setup_test_output();

        // 发送ping
        injector.inject_ping_request("192.168.1.1".parse().unwrap(), 1, 1).unwrap();

        // 运行处理引擎
        engine.process_one();

        // 验证收到reply
        let reply = output.collect_one().unwrap();
        assert!(is_icmp_echo_reply(&reply));
    }
}
```

## 7. 测试用例

### 7.1 基础解析测试

- [ ] 解析以太网广播帧
- [ ] 解析ARP请求/响应
- [ ] 解析IPv4单播/组播
- [ ] 解析ICMP Echo Request
- [ ] 解析TCP SYN包

### 7.2 协议交互测试

- [ ] ARP请求 → 收到ARP响应
- [ ] ICMP Echo Request → 收到Echo Reply
- [ ] UDP数据包收发
- [ ] TCP三次握手
- [ ] TCP数据传输

### 7.3 边界测试

- [ ] 最小以太网帧（64字节）
- [ ] 最大以太网帧（1500字节）
- [ ] IP分片报文
- [ ] 错误校验和报文
- [ ] 不支持的协议类型
