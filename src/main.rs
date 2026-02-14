use core_net::{boot_default, shutdown, Packet, PacketProcessor, Scheduler};

fn main() {
    // (1) 启动上电模块申请资源
    let mut context = boot_default();
    println!("系统上电启动完成");
    println!("接收队列容量: {}, 发送队列容量: {}",
             context.rxq.capacity(), context.txq.capacity());

    // (2) 新建一个Packet类型的报文，内容为 "hello, packet!"
    let packet_data = b"hello, packet!".to_vec();
    let packet = Packet::from_bytes(packet_data);
    println!("创建报文，内容: hello, packet!");

    // (3) 将该报文放到收包队列中
    match context.rxq.enqueue(packet) {
        Ok(_) => println!("报文已放入接收队列"),
        Err(_) => {
            println!("错误：接收队列已满");
            return;
        }
    }

    // (4) 使用调度模块，处理该报文
    let scheduler = Scheduler::new("MainScheduler".to_string())
        .with_processor(PacketProcessor::new().with_verbose(true))
        .with_verbose(true);
    match scheduler.run(&mut context.rxq) {
        Ok(count) => println!("调度完成，处理了 {} 个报文", count),
        Err(e) => println!("调度错误: {}", e),
    }

    // (5) 处理完成，下电，释放资源
    shutdown(&mut context);
    println!("系统下电，资源已释放");
}
