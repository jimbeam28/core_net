// RingQueue 集成测试（精简版）
//
// 核心功能测试：队列基本操作

use core_net::common::queue::RingQueue;
use core_net::common::packet::Packet;

// 测试1：基本入队出队
#[test]
fn test_packet_queue_basic() {
    let mut q: RingQueue<Packet> = RingQueue::new(10);

    let packet = Packet::from_bytes(vec![0x01, 0x02, 0x03]);
    assert!(q.enqueue(packet).is_ok());
    assert_eq!(q.len(), 1);

    let out = q.dequeue();
    assert!(out.is_some());
    assert_eq!(q.len(), 0);
}

// 测试2：队列容量限制
#[test]
fn test_queue_capacity() {
    let mut q: RingQueue<Packet> = RingQueue::new(3);

    for i in 0..3 {
        let packet = Packet::from_bytes(vec![i as u8]);
        assert!(q.enqueue(packet).is_ok());
    }

    // 队列已满，再次入队应失败
    let packet = Packet::from_bytes(vec![0xFF]);
    assert!(q.enqueue(packet).is_err());
}

// 测试3：先进先出顺序
#[test]
fn test_fifo_order() {
    let mut q: RingQueue<Packet> = RingQueue::new(10);

    for i in 0..5 {
        let packet = Packet::from_bytes(vec![i as u8]);
        q.enqueue(packet).unwrap();
    }

    for i in 0..5 {
        let out = q.dequeue().unwrap();
        assert_eq!(out.as_slice(), &[i as u8]);
    }
}

// 测试4：空队列出队
#[test]
fn test_empty_queue() {
    let mut q: RingQueue<Packet> = RingQueue::new(10);
    assert!(q.dequeue().is_none());
}
