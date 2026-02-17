use core_net::{boot_default, shutdown, Packet, PacketProcessor, Scheduler};
use core_net::protocols::{MacAddr, Ipv4Addr};

fn main() {
    // (1) 启动上电模块申请资源
    let mut context = boot_default();
    println!("系统上电启动完成");
    println!("接口数量: {}", context.interface_count());

    // 显示所有接口信息
    for i in 0..context.interface_count() {
        if let Some(iface) = context.get_interface_by_index(i as u32) {
            println!("  接口: {}, MAC: {}, IP: {}, 接收队列容量: {}, 发送队列容量: {}",
                     iface.name, iface.mac_addr, iface.ip_addr,
                     iface.rxq.capacity(), iface.txq.capacity());
        }
    }

    // (2) 构造 ARP 请求报文（以太网帧封装）
    // 获取接口的 IP 地址，作为 ARP 请求的目标 IP
    let target_ip = if let Some(iface) = context.get_interface_by_index(0) {
        iface.ip_addr
    } else {
        println!("错误：没有可用的接口");
        return;
    };

    // 模拟外部请求者的信息
    let src_mac = MacAddr::new([0x00, 0xaa, 0xbb, 0xcc, 0xdd, 0xee]);
    let src_ip = Ipv4Addr::new(192, 168, 1, 200);

    // 构造 ARP 请求报文（以太网帧）
    let mut arp_request = Vec::new();

    // === 以太网头部 (14 字节) ===
    arp_request.extend_from_slice(&MacAddr::broadcast().bytes);  // 目标 MAC (广播)
    arp_request.extend_from_slice(&src_mac.bytes);  // 源 MAC
    arp_request.extend_from_slice(&0x0806u16.to_be_bytes());  // 以太网类型: ARP

    // === ARP 报文 (28 字节) ===
    // 硬件类型 (1=以太网)
    arp_request.extend_from_slice(&1u16.to_be_bytes());
    // 协议类型 (0x0800=IPv4)
    arp_request.extend_from_slice(&0x0800u16.to_be_bytes());
    // 硬件地址长度 (6)
    arp_request.push(6);
    // 协议地址长度 (4)
    arp_request.push(4);
    // 操作码 (1=请求)
    arp_request.extend_from_slice(&1u16.to_be_bytes());
    // 发送方硬件地址
    arp_request.extend_from_slice(&src_mac.bytes);
    // 发送方协议地址
    arp_request.extend_from_slice(&src_ip.bytes);
    // 目标硬件地址 (请求时为 0)
    arp_request.extend_from_slice(&[0u8; 6]);
    // 目标协议地址 (查询的 IP)
    arp_request.extend_from_slice(&target_ip.bytes);

    println!("创建 ARP 请求报文:");
    println!("  源 MAC: {}, 源 IP: {}", src_mac, src_ip);
    println!("  目标 IP: {}", target_ip);

    // (3) 将该报文放到第一个接口的收包队列中
    let packet = Packet::from_bytes(arp_request);
    if let Some(iface) = context.get_interface_by_index_mut(0) {
        match iface.rxq.enqueue(packet) {
            Ok(_) => println!("报文已放入接口 {} 的接收队列", iface.name),
            Err(_) => {
                println!("错误：接收队列已满");
                return;
            }
        }
    }

    // (4) 使用调度模块，处理所有接口的报文
    let scheduler = Scheduler::new("MainScheduler".to_string())
        .with_processor(PacketProcessor::new().with_verbose(true))
        .with_verbose(true);

    match scheduler.run_all_interfaces(&mut context.interfaces) {
        Ok(count) => println!("调度完成，处理了 {} 个报文", count),
        Err(e) => println!("调度错误: {}", e),
    }

    // (5) 从发包队列取出响应报文并打印
    if let Some(iface) = context.get_interface_by_index_mut(0) {
        println!("\n检查接口 {} 的发送队列:", iface.name);
        if let Some(response_packet) = iface.txq.dequeue() {
            println!("收到响应报文，长度: {} 字节", response_packet.len());
            println!("报文内容 (hex):");
            let data = response_packet.as_slice();
            // 以16字节为一行打印
            for (i, chunk) in data.chunks(16).enumerate() {
                print!("  {:04x}: ", i * 16);
                for byte in chunk {
                    print!("{:02x} ", byte);
                }
                println!();
            }

            // 解析响应报文
            if data.len() >= 14 {
                let dst_mac = MacAddr::new([
                    data[0], data[1], data[2], data[3], data[4], data[5]
                ]);
                let src_mac = MacAddr::new([
                    data[6], data[7], data[8], data[9], data[10], data[11]
                ]);
                let ether_type = u16::from_be_bytes([data[12], data[13]]);

                println!("\n解析响应报文:");
                println!("  以太网头部:");
                println!("    目标 MAC: {}", dst_mac);
                println!("    源 MAC: {}", src_mac);
                println!("    类型: 0x{:04x}", ether_type);

                // 如果是 ARP 响应
                if ether_type == 0x0806 && data.len() >= 42 {
                    let arp_op = u16::from_be_bytes([data[20], data[21]]);
                    if arp_op == 2 {  // ARP Reply
                        let arp_src_mac = MacAddr::new([
                            data[22], data[23], data[24], data[25], data[26], data[27]
                        ]);
                        let arp_src_ip = Ipv4Addr::new(data[28], data[29], data[30], data[31]);
                        let arp_dst_mac = MacAddr::new([
                            data[32], data[33], data[34], data[35], data[36], data[37]
                        ]);
                        let arp_dst_ip = Ipv4Addr::new(data[38], data[39], data[40], data[41]);

                        println!("  ARP 响应:");
                        println!("    发送方 MAC: {}", arp_src_mac);
                        println!("    发送方 IP: {}", arp_src_ip);
                        println!("    目标 MAC: {}", arp_dst_mac);
                        println!("    目标 IP: {}", arp_dst_ip);
                    }
                }
            }
        } else {
            println!("发送队列为空，没有响应报文");
        }
    }

    // (6) 处理完成，下电，释放资源
    shutdown(&mut context);
    println!("系统下电，资源已释放");
}
