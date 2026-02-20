// RingQueue 集成测试

use core_net::common::queue::{RingQueue, MIN_QUEUE_CAPACITY};
use core_net::common::packet::Packet;
use core_net::common::CoreError;

// 与Packet模块集成

#[test]
fn test_packet_queue_flow() {
    let mut rx_q: RingQueue<Packet> = RingQueue::new(10);

    // 创建测试报文
    let packet1 = Packet::from_bytes(vec![0x01, 0x02, 0x03, 0x04]);
    let packet2 = Packet::from_bytes(vec![0x05, 0x06, 0x07, 0x08]);

    // 入队
    assert!(rx_q.enqueue(packet1).is_ok());
    assert!(rx_q.enqueue(packet2).is_ok());
    assert_eq!(rx_q.len(), 2);

    // 出队
    let out1 = rx_q.dequeue();
    assert!(out1.is_some());
    assert_eq!(out1.unwrap().as_slice(), &[0x01, 0x02, 0x03, 0x04]);

    let out2 = rx_q.dequeue();
    assert!(out2.is_some());
    assert_eq!(out2.unwrap().as_slice(), &[0x05, 0x06, 0x07, 0x08]);
}

#[test]
fn test_multiple_packets() {
    let mut q: RingQueue<Packet> = RingQueue::new(10);

    // 多个报文入队出队
    for i in 0..5 {
        let data = vec![i as u8; 10];
        let packet = Packet::from_bytes(data);
        assert!(q.enqueue(packet).is_ok());
    }

    assert_eq!(q.len(), 5);

    // 验证每个报文独立处理
    for i in 0..5 {
        let received = q.dequeue();
        assert!(received.is_some());
        let expected_data = vec![i as u8; 10];
        assert_eq!(received.unwrap().as_slice(), expected_data.as_slice());
    }
}

#[test]
fn test_large_packet() {
    let mut q: RingQueue<Packet> = RingQueue::new(5);

    // 创建大报文（1500字节）
    let large_data = vec![0u8; 1500];
    let large_packet = Packet::from_bytes(large_data);

    assert!(q.enqueue(large_packet).is_ok());

    let received = q.dequeue().unwrap();
    assert_eq!(received.len(), 1500);
    assert_eq!(received.as_slice(), &[0u8; 1500][..]);
}

// 并发场景模拟（单线程）

#[test]
fn test_producer_consumer_pattern() {
    let mut q: RingQueue<u32> = RingQueue::new(100);

    for i in 0..50 {
        assert!(q.enqueue(i).is_ok());
    }

    let mut consumed = 0;
    while let Some(value) = q.dequeue() {
        assert_eq!(value, consumed);
        consumed += 1;
    }
    assert_eq!(consumed, 50);
}

#[test]
fn test_burst_enqueue() {
    let mut q: RingQueue<u32> = RingQueue::new(100);

    for i in 0..100 {
        assert!(q.enqueue(i).is_ok());
    }

    assert!(q.is_full());
    assert_eq!(q.len(), 100);
}

#[test]
fn test_burst_dequeue() {
    let mut q: RingQueue<u32> = RingQueue::new(100);

    for i in 0..100 {
        q.enqueue(i).unwrap();
    }

    for i in 0..100 {
        assert_eq!(q.dequeue(), Some(i));
    }

    assert!(q.is_empty());
}

#[test]
fn test_alternating_ops() {
    let mut q: RingQueue<u32> = RingQueue::new(10);

    for i in 0..20 {
        if i % 3 == 0 {
            q.enqueue(i).unwrap();
        } else if !q.is_empty() {
            q.dequeue();
        }
    }

    assert!(!q.is_full());
    assert!(q.len() <= 10);
}

// 边界压力测试

#[test]
fn test_stress_fill_drain() {
    let mut q: RingQueue<u8> = RingQueue::new(10);

    // 多次填充清空循环
    for _ in 0..100 {
        // 填充
        for i in 0..10 {
            assert!(q.enqueue(i).is_ok());
        }
        assert!(q.is_full());

        // 清空
        for _ in 0..10 {
            assert!(q.dequeue().is_some());
        }
        assert!(q.is_empty());
    }
}

#[test]
fn test_random_ops() {
    use std::collections::VecDeque;

    let mut ring_q: RingQueue<u8> = RingQueue::new(20);
    let mut reference: VecDeque<u8> = VecDeque::new();

    // 随机操作序列（使用确定性的伪随机序列）
    let ops: Vec<(bool, Option<u8>)> = vec![
        (true, Some(1)), (true, Some(2)), (true, Some(3)),  // 入队3个
        (false, None), (false, None),                       // 出队2个
        (true, Some(4)), (true, Some(5)),                   // 入队2个
        (false, None),                                      // 出队1个
        (true, Some(6)), (true, Some(7)), (true, Some(8)),  // 入队3个
        (false, None), (false, None), (false, None),       // 出队3个
    ];

    for (is_enqueue, value) in ops {
        if is_enqueue {
            if let Some(v) = value {
                ring_q.enqueue(v).unwrap();
                reference.push_back(v);
            }
        } else {
            let ring_val = ring_q.dequeue();
            let ref_val = reference.pop_front();
            assert_eq!(ring_val, ref_val);
        }
    }

    // 最终状态应该一致
    assert_eq!(ring_q.len(), reference.len());
}

#[test]
fn test_zero_capacity_handling() {
    let q: RingQueue<u8> = RingQueue::new(0);
    assert_eq!(q.capacity(), MIN_QUEUE_CAPACITY);
}

// 与CoreError集成

#[test]
fn test_queue_error_conversion() {
    let mut q: RingQueue<u8> = RingQueue::new(2);
    q.enqueue(1).unwrap();
    q.enqueue(2).unwrap();

    let result = q.enqueue(3);
    assert!(result.is_err());

    let core_error: CoreError = result.unwrap_err().into();
    matches!(core_error, CoreError::QueueFull);
}

#[test]
fn test_error_propagation() {
    #[derive(Debug)]
    struct TestQueue {
        inner: RingQueue<u8>,
    }

    impl TestQueue {
        fn new(capacity: usize) -> Self {
            Self { inner: RingQueue::new(capacity) }
        }

        fn push(&mut self, value: u8) -> Result<(), String> {
            self.inner.enqueue(value).map_err(|_| "队列已满".to_string())
        }
    }

    let mut q = TestQueue::new(2);
    assert!(q.push(1).is_ok());
    assert!(q.push(2).is_ok());
    assert!(q.push(3).is_err());
}
