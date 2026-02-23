// IPv6 分片与重组集成测试
//
// 测试 IPv6 分片和重组功能，包括分片创建、重组、边界情况和安全测试

use core_net::protocols::Ipv6Addr;
use core_net::protocols::ipv6::{
    FragmentHeader, create_fragments,
    ReassemblyKey, FragmentInfo, FragmentCache,
    DEFAULT_MAX_REASSEMBLY_ENTRIES, DEFAULT_MAX_FRAGMENTS_PER_PACKET,
};

use serial_test::serial;

mod common;
use common::create_test_context;

// 测试环境配置：本机接口 eth0: ifindex=0, MAC=00:11:22:33:44:55

// ========== 全局测试生命周期 ==========

// 1. 分片创建测试组

#[test]
#[serial]
fn test_fragment_header_constants() {
    assert_eq!(FragmentHeader::HEADER_SIZE, 8);
}

#[test]
#[serial]
fn test_fragment_header_creation() {
    let _ctx = create_test_context();

    let frag = FragmentHeader::new(58, 0, true, 0xABCDEF01);

    assert_eq!(frag.next_header, 58); // ICMPv6
    assert_eq!(frag.fragment_offset(), 0);
    assert!(frag.more_fragments());
    assert!(!frag.is_atomic_fragment());
}

#[test]
#[serial]
fn test_fragment_header_encode_decode() {
    let _ctx = create_test_context();

    let original = FragmentHeader::new(17, 123, true, 0x12345678);
    let bytes = original.to_bytes();

    let decoded = FragmentHeader::from_bytes(&bytes).unwrap();

    assert_eq!(decoded.next_header, 17);
    assert_eq!(decoded.fragment_offset(), 123);
    assert!(decoded.more_fragments());
    // 使用 to_bytes() 后读取 identification 来避免 packed struct 引用问题
    assert_eq!(u32::from_be_bytes([
        bytes[4], bytes[5], bytes[6], bytes[7]
    ]), 0x12345678);
}

#[test]
#[serial]
fn test_atomic_fragment_detection() {
    let _ctx = create_test_context();

    // 原子分片：offset=0, M=0
    let atomic = FragmentHeader::new(58, 0, false, 12345);
    assert!(atomic.is_atomic_fragment());

    // 非原子分片
    let frag1 = FragmentHeader::new(58, 0, true, 12345);
    assert!(!frag1.is_atomic_fragment());

    let frag2 = FragmentHeader::new(58, 8, false, 12345);
    assert!(!frag2.is_atomic_fragment());
}

// 2. 分片创建测试组

#[test]
#[serial]
fn test_create_fragments_no_fragmentation_needed() {
    let _ctx = create_test_context();

    let unfragmentable = vec![0xFFu8; 40]; // IPv6 基本头部
    let fragmentable = vec![1u8; 20]; // 可分片数据
    let mtu = 128; // MTU 128 字节

    let fragments = create_fragments(&unfragmentable, &fragmentable, mtu, 12345, 58);

    assert_eq!(fragments.len(), 1);
    assert_eq!(fragments[0].fragment_offset, 0);
    assert!(!fragments[0].more_fragments);
    assert_eq!(fragments[0].data.len(), 60); // 40 + 20
}

#[test]
#[serial]
fn test_create_fragments_with_fragmentation() {
    let _ctx = create_test_context();

    let unfragmentable = vec![0xFFu8; 40]; // IPv6 基本头部
    let fragmentable = vec![1u8; 200]; // 可分片数据
    let mtu = 100; // MTU 100 字节

    let fragments = create_fragments(&unfragmentable, &fragmentable, mtu, 12345, 58);

    // 每个分片: 40 (不可分片) + 8 (分片头) + 分片数据
    // 分片数据: (100 - 48) / 8 * 8 = 48 字节 (52向下对齐到48)
    // 200 字节需要 5 个分片: 48 + 48 + 48 + 48 + 8
    assert_eq!(fragments.len(), 5);

    // 检查第一个分片
    assert_eq!(fragments[0].fragment_offset, 0);
    assert!(fragments[0].more_fragments);
    assert_eq!(fragments[0].identification, 12345);
    assert_eq!(fragments[0].data.len(), 96); // 40 + 8 + 48

    // 检查最后一个分片
    assert!(!fragments[4].more_fragments);
}

#[test]
#[serial]
fn test_create_fragments_alignment() {
    let _ctx = create_test_context();

    let unfragmentable = vec![0xFFu8; 40];
    let fragmentable = vec![1u8; 100];
    let mtu = 96; // 需要对齐的分片

    let fragments = create_fragments(&unfragmentable, &fragmentable, mtu, 12345, 58);

    // 验证所有分片（除最后一片）的数据长度是 8 字节的倍数
    for (i, frag) in fragments.iter().enumerate() {
        if frag.more_fragments {
            // 分片数据长度 = 总长度 - 不可分片 - 分片头
            let frag_data_len = frag.data.len() - 40 - 8;
            assert_eq!(frag_data_len % 8, 0, "分片 {} 的数据长度不是 8 字节倍数", i);
        }
    }
}

#[test]
#[serial]
fn test_create_fragments_small_mtu() {
    let _ctx = create_test_context();

    let unfragmentable = vec![0xFFu8; 40];
    let fragmentable = vec![1u8; 50];
    let mtu = 60; // 小 MTU

    let fragments = create_fragments(&unfragmentable, &fragmentable, mtu, 12345, 58);

    // 每个分片: 40 + 8 + 分片数据
    // 分片数据: (60 - 48) / 8 * 8 = 8 字节
    // 50 字节需要 7 个分片
    assert_eq!(fragments.len(), 7);
}

// 3. 分片信息测试组

#[test]
#[serial]
fn test_fragment_info_creation() {
    let _ctx = create_test_context();

    let frag = FragmentInfo::new(0, true, vec![1u8; 16]);

    assert_eq!(frag.offset, 0);
    assert!(frag.more_fragments);
    assert_eq!(frag.data.len(), 16);
    assert_eq!(frag.end_offset(), 16);
}

#[test]
#[serial]
fn test_fragment_info_overlap() {
    let _ctx = create_test_context();

    let frag1 = FragmentInfo::new(0, true, vec![1u8; 16]); // 0-16 bytes
    let frag2 = FragmentInfo::new(2, true, vec![2u8; 16]); // 16-32 bytes

    // 不重叠
    assert!(!frag1.overlaps_with(&frag2));

    let frag3 = FragmentInfo::new(1, true, vec![3u8; 16]); // 8-24 bytes

    // 重叠
    assert!(frag1.overlaps_with(&frag3));
}

#[test]
#[serial]
fn test_fragment_info_end_offset() {
    let _ctx = create_test_context();

    // offset=1 表示 8 字节
    let frag = FragmentInfo::new(1, false, vec![1u8; 20]);
    assert_eq!(frag.end_offset(), 28); // 8 + 20
}

// 4. 重组键测试组

#[test]
#[serial]
fn test_reassembly_key() {
    let _ctx = create_test_context();

    let src = Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 1);
    let dst = Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 2);

    let key = ReassemblyKey::new(src, dst, 12345);

    assert_eq!(key.source_addr, src);
    assert_eq!(key.dest_addr, dst);
    assert_eq!(key.identification, 12345);
}

#[test]
#[serial]
fn test_reassembly_key_hash() {
    let _ctx = create_test_context();

    let src = Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 1);
    let dst = Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 2);

    let key1 = ReassemblyKey::new(src, dst, 12345);
    let key2 = ReassemblyKey::new(src, dst, 12345);

    assert_eq!(key1, key2);

    use std::collections::HashMap;
    let mut map = HashMap::new();
    map.insert(key1, 42);
    assert_eq!(map.get(&key2), Some(&42));
}

// 5. 分片缓存测试组

#[test]
#[serial]
fn test_fragment_cache_basic() {
    let _ctx = create_test_context();

    let mut cache = FragmentCache::new(10);

    let key = ReassemblyKey::new(
        Ipv6Addr::UNSPECIFIED,
        Ipv6Addr::UNSPECIFIED,
        12345,
    );

    // 添加第一个分片
    let frag1 = FragmentInfo::new(0, true, vec![1u8; 16]);
    let result = cache.add_fragment(key.clone(), frag1).unwrap();
    assert!(result.is_none());
    assert_eq!(cache.len(), 1);

    // 添加第二个分片（完成）
    let frag2 = FragmentInfo::new(2, false, vec![2u8; 8]);
    let result = cache.add_fragment(key.clone(), frag2).unwrap();
    assert!(result.is_some());
    assert_eq!(cache.len(), 0); // 应该被移除
}

#[test]
#[serial]
fn test_fragment_cache_overlap_rejection() {
    let _ctx = create_test_context();

    let mut cache = FragmentCache::new(10);

    let key = ReassemblyKey::new(
        Ipv6Addr::UNSPECIFIED,
        Ipv6Addr::UNSPECIFIED,
        12345,
    );

    // 添加第一个分片
    let frag1 = FragmentInfo::new(0, true, vec![1u8; 16]);
    cache.add_fragment(key.clone(), frag1).unwrap();

    // 尝试添加重叠的分片
    let frag2 = FragmentInfo::new(1, true, vec![2u8; 16]); // 重叠
    let result = cache.add_fragment(key.clone(), frag2);

    assert!(result.is_err());
}

#[test]
#[serial]
fn test_fragment_cache_cleanup_expired() {
    let _ctx = create_test_context();

    let mut cache = FragmentCache::new(10);

    let key = ReassemblyKey::new(
        Ipv6Addr::UNSPECIFIED,
        Ipv6Addr::UNSPECIFIED,
        12345,
    );

    // 添加不完整的分片
    let frag1 = FragmentInfo::new(0, true, vec![1u8; 16]);
    cache.add_fragment(key.clone(), frag1).unwrap();

    // 新创建的条目不应该超时
    cache.cleanup_expired();
    assert_eq!(cache.len(), 1);
}

#[test]
#[serial]
fn test_fragment_cache_max_entries() {
    let _ctx = create_test_context();

    let mut cache = FragmentCache::new(2);

    // 添加三个不同的分片组
    for i in 0..3 {
        let key = ReassemblyKey::new(
            Ipv6Addr::UNSPECIFIED,
            Ipv6Addr::UNSPECIFIED,
            i,
        );
        let frag = FragmentInfo::new(0, true, vec![1u8; 8]);
        cache.add_fragment(key, frag).unwrap();
    }

    // 应该只保留最新的 2 个条目
    assert!(cache.len() <= 2);
}

// 6. 完整重组测试组

#[test]
#[serial]
fn test_reassembly_complete() {
    let _ctx = create_test_context();

    let mut cache = FragmentCache::new(10);

    let key = ReassemblyKey::new(
        Ipv6Addr::UNSPECIFIED,
        Ipv6Addr::UNSPECIFIED,
        12345,
    );

    // 添加完整的分片序列
    let frag1 = FragmentInfo::new(0, true, vec![1u8; 16]); // 0-15
    let frag2 = FragmentInfo::new(2, false, vec![2u8; 8]);  // 16-23

    cache.add_fragment(key.clone(), frag1).unwrap();
    let result = cache.add_fragment(key.clone(), frag2).unwrap();

    assert!(result.is_some());
    let reassembled = result.unwrap();
    assert_eq!(reassembled.len(), 24);
    assert_eq!(&reassembled[0..16], &[1u8; 16]);
    assert_eq!(&reassembled[16..24], &[2u8; 8]);
}

#[test]
#[serial]
fn test_reassembly_out_of_order() {
    let _ctx = create_test_context();

    let mut cache = FragmentCache::new(10);

    let key = ReassemblyKey::new(
        Ipv6Addr::UNSPECIFIED,
        Ipv6Addr::UNSPECIFIED,
        12345,
    );

    // 乱序添加分片
    let frag2 = FragmentInfo::new(2, false, vec![2u8; 8]);
    let frag1 = FragmentInfo::new(0, true, vec![1u8; 16]);

    cache.add_fragment(key.clone(), frag2).unwrap();
    let result = cache.add_fragment(key.clone(), frag1).unwrap();

    assert!(result.is_some());
}

#[test]
#[serial]
fn test_reassembly_incomplete() {
    let _ctx = create_test_context();

    let mut cache = FragmentCache::new(10);

    let key = ReassemblyKey::new(
        Ipv6Addr::UNSPECIFIED,
        Ipv6Addr::UNSPECIFIED,
        12345,
    );

    // 只添加第一个分片
    let frag1 = FragmentInfo::new(0, true, vec![1u8; 16]);
    let result = cache.add_fragment(key.clone(), frag1).unwrap();

    assert!(result.is_none());
    assert_eq!(cache.len(), 1);
}

// 7. 分片包结构测试组

#[test]
#[serial]
fn test_fragment_packet_structure() {
    let _ctx = create_test_context();

    let unfragmentable = vec![0xFFu8; 40];
    let fragmentable = vec![1u8; 100];
    let mtu = 96;

    let fragments = create_fragments(&unfragmentable, &fragmentable, mtu, 12345, 58);

    // 检查 FragmentPacket 结构
    for frag in &fragments {
        // 验证数据以不可分片部分开头
        assert_eq!(&frag.data[0..40], &unfragmentable[..]);

        // 验证分片头在正确位置
        let frag_hdr = FragmentHeader::from_bytes(&frag.data[40..48]).unwrap();
        assert_eq!(frag_hdr.fragment_offset(), frag.fragment_offset);
        assert_eq!(frag_hdr.more_fragments(), frag.more_fragments);
        // 验证 identification（使用 to_bytes 避免 packed struct 引用问题）
        let hdr_bytes = frag_hdr.to_bytes();
        let id = u32::from_be_bytes([hdr_bytes[4], hdr_bytes[5], hdr_bytes[6], hdr_bytes[7]]);
        assert_eq!(id, frag.identification);
    }
}

// 8. 边界情况测试组

#[test]
#[serial]
fn test_fragment_minimum_size() {
    let _ctx = create_test_context();

    let unfragmentable = vec![0xFFu8; 40];
    let fragmentable = vec![1u8; 1]; // 1 字节数据
    let mtu = 1500;

    let fragments = create_fragments(&unfragmentable, &fragmentable, mtu, 12345, 58);

    // 1 字节不需要分片
    assert_eq!(fragments.len(), 1);
    assert!(!fragments[0].more_fragments);
}

#[test]
#[serial]
fn test_fragment_exactly_mtu() {
    let _ctx = create_test_context();

    let unfragmentable = vec![0xFFu8; 40];
    let fragmentable = vec![1u8; 49]; // 正好需要分片
    let mtu = 88; // 40 + 8 + 40

    let fragments = create_fragments(&unfragmentable, &fragmentable, mtu, 12345, 58);

    // 总大小 = 40 + 49 = 89 > MTU (88)，需要分片
    // 分片数据 = (88 - 48) / 8 * 8 = 40 字节
    // 49 字节需要 2 个分片: 40 + 9
    assert_eq!(fragments.len(), 2);
    assert!(fragments[0].more_fragments);
    assert!(!fragments[1].more_fragments);
}

// 9. 配置常量测试组

#[test]
#[serial]
fn test_fragment_config_constants() {
    let _ctx = create_test_context();

    assert!(DEFAULT_MAX_REASSEMBLY_ENTRIES > 0);
    assert!(DEFAULT_MAX_FRAGMENTS_PER_PACKET > 0);

    // 验证配置合理性
    assert!(DEFAULT_MAX_REASSEMBLY_ENTRIES <= 4096); // 不应过大
    assert!(DEFAULT_MAX_FRAGMENTS_PER_PACKET <= 1024);
}

// 10. 错误处理测试组

#[test]
#[serial]
fn test_fragment_header_decode_invalid_length() {
    let _ctx = create_test_context();

    let data = [0u8; 4]; // 不足 8 字节
    let result = FragmentHeader::from_bytes(&data);

    assert!(result.is_err());
}

#[test]
#[serial]
fn test_fragment_cache_too_many_fragments() {
    let _ctx = create_test_context();

    let mut cache = FragmentCache::new(10);

    let key = ReassemblyKey::new(
        Ipv6Addr::UNSPECIFIED,
        Ipv6Addr::UNSPECIFIED,
        12345,
    );

    // 添加大量分片（超过限制）
    for i in 0..(DEFAULT_MAX_FRAGMENTS_PER_PACKET + 1) {
        let frag = FragmentInfo::new(i as u16, i < DEFAULT_MAX_FRAGMENTS_PER_PACKET, vec![1u8; 8]);
        if i < DEFAULT_MAX_FRAGMENTS_PER_PACKET {
            cache.add_fragment(key.clone(), frag).unwrap();
        } else {
            let result = cache.add_fragment(key.clone(), frag);
            assert!(result.is_err());
        }
    }
}
