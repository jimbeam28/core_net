// IPsec 协议集成测试（精简版）
//
// 核心功能测试：ESP头部解析

use core_net::protocols::ipsec::esp::{EspHeader, EspPacket, EspTrailer};
use serial_test::serial;

// 测试1：ESP头部解析
#[test]
#[serial]
fn test_esp_header_parse() {
    let bytes = [
        0x00, 0x00, 0x00, 0x01, // SPI = 1
        0x00, 0x00, 0x00, 0x0A, // Sequence = 10
    ];

    let header = EspHeader::parse(&bytes).unwrap();
    assert_eq!(header.spi, 1);
    assert_eq!(header.sequence_number, 10);
}

// 测试2：ESP数据包创建
#[test]
#[serial]
fn test_esp_packet_creation() {
    let encrypted_data = vec![0xAB, 0xCD, 0xEF];
    let trailer = EspTrailer::new(0, 4, vec![]); // pad_len=0, next_header=4 (IPv4), no padding
    let packet = EspPacket::new(0x12345678, 42, encrypted_data.clone(), trailer, None);

    assert_eq!(packet.header.spi, 0x12345678);
    assert_eq!(packet.header.sequence_number, 42);
    assert_eq!(packet.encrypted_data, encrypted_data);
}

// 测试3：ESP尾部解析
#[test]
#[serial]
fn test_esp_trailer_parse() {
    // ESP尾部最后两个字节：Pad Length + Next Header
    let data = vec![
        0xAA, 0xBB, 0xCC, 0xDD, // 一些填充数据
        0x02, // Pad Length = 2
        0x06, // Next Header = TCP
    ];

    let (trailer, trailer_start) = EspTrailer::parse(&data).unwrap();
    // trailer_start是填充数据开始的位置（不是填充长度）
    assert!(trailer_start > 0);
}

// 测试4：ESP序列化
#[test]
#[serial]
fn test_esp_serialization() {
    let trailer = EspTrailer::new(0, 4, vec![]);
    let packet = EspPacket::new(1, 1, vec![0xAA, 0xBB], trailer, None);
    let bytes = packet.to_bytes();

    assert!(bytes.len() >= 8); // 至少8字节头部
}
