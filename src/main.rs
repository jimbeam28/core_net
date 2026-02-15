use core_net::{boot_default, shutdown, Packet, PacketProcessor, Scheduler};

fn main() {
    // (1) 启动上电模块申请资源
    let mut context = boot_default();
    println!("系统上电启动完成");
    println!("接口数量: {}", context.interface_count());

    // 显示所有接口信息
    for i in 0..context.interface_count() {
        if let Some(iface) = context.get_interface_by_index(i as u32) {
            println!("  接口: {}, 接收队列容量: {}, 发送队列容量: {}",
                     iface.name, iface.rxq.capacity(), iface.txq.capacity());
        }
    }

    // (2) 新建一个Packet类型的报文，内容为 "hello, packet!"
    let packet_data = b"hello, packet!".to_vec();
    let packet = Packet::from_bytes(packet_data);
    println!("创建报文，内容: hello, packet!");

    // (3) 将该报文放到第一个接口的收包队列中
    if context.interface_count() > 0 {
        if let Some(iface) = context.get_interface_by_index_mut(0) {
            match iface.rxq.enqueue(packet) {
                Ok(_) => println!("报文已放入接口 {} 的接收队列", iface.name),
                Err(_) => {
                    println!("错误：接收队列已满");
                    return;
                }
            }
        }
    } else {
        println!("错误：没有可用的接口");
        return;
    }

    // (4) 使用调度模块，处理所有接口的报文
    let scheduler = Scheduler::new("MainScheduler".to_string())
        .with_processor(PacketProcessor::new().with_verbose(true))
        .with_verbose(true);

    match scheduler.run_all_interfaces(&mut context.interfaces) {
        Ok(count) => println!("调度完成，处理了 {} 个报文", count),
        Err(e) => println!("调度错误: {}", e),
    }

    // (5) 处理完成，下电，释放资源
    shutdown(&mut context);
    println!("系统下电，资源已释放");
}
