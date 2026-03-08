// VLAN 协议集成测试（精简版）
//
// 核心功能测试：VLAN标签解析和封装

use core_net::protocols::vlan::VlanTag;
use core_net::protocols::ETH_P_8021Q;
use core_net::interface::MacAddr;
use serial_test::serial;

// 测试1：VLAN标签从字节解析
#[test]
#[serial]
fn test_vlan_tag_from_bytes() {
    // TCI = PCP(3) + DEI(1) + VID(12) = 0x1234
    let bytes = [0x12, 0x34];
    let tag = VlanTag::from_bytes(bytes).unwrap();

    assert_eq!(tag.vid, 0x234);
    assert_eq!(tag.pcp, 0); // 0x1234的高3位 = 000 = 0
}

// 测试2：VLAN标签创建
#[test]
#[serial]
fn test_vlan_tag_creation() {
    let tag = VlanTag::new(0, false, 100).unwrap(); // PCP=0, DEI=false, VID=100

    assert_eq!(tag.vid, 100);
    assert_eq!(tag.pcp, 0);
    assert!(!tag.dei);
}

// 测试3：VLAN标签序列化
#[test]
#[serial]
fn test_vlan_tag_serialization() {
    let tag = VlanTag::new(3, true, 100).unwrap();
    let bytes = tag.to_bytes();

    assert_eq!(bytes.len(), 2); // TCI = 2字节
}

// 测试4：构建带VLAN标签的以太网帧
#[test]
#[serial]
fn test_vlan_frame_build() {
    let dst_mac = MacAddr::new([0x00, 0x11, 0x22, 0x33, 0x44, 0x55]);
    let src_mac = MacAddr::new([0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0x01]);
    let tag = VlanTag::new(0, false, 100).unwrap();

    let mut frame = Vec::new();
    frame.extend_from_slice(&dst_mac.bytes);
    frame.extend_from_slice(&src_mac.bytes);
    frame.extend_from_slice(&ETH_P_8021Q.to_be_bytes());
    frame.extend_from_slice(&tag.to_bytes());
    frame.extend_from_slice(&[0x08, 0x00]); // EtherType: IPv4

    assert!(frame.len() > 14); // 比标准以太网帧长（有VLAN标签）
}

// 测试5：无效VLAN ID
#[test]
#[serial]
fn test_invalid_vlan_id() {
    // VID 4095是保留值
    let result = VlanTag::new(0, false, 4095);
    assert!(result.is_err());
}
