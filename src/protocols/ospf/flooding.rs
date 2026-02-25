// src/protocols/ospf/flooding.rs
//
// OSPF LSA 洪泛机制
// RFC 2328 Section 13: 洪泛链路状态更新

use crate::common::Ipv4Addr;
use crate::protocols::ospf2::{
    lsa::{Lsa, LsaHeader},
    neighbor::NeighborState,
    interface::OspfInterface,
};
use std::collections::HashMap;

/// 洪泛结果
#[derive(Debug, Clone)]
pub enum FloodResult {
    /// LSA 无效，应丢弃
    Invalid,
    /// LSA 已存在且无需更新
    AlreadyPresent,
    /// LSA 已安装但不需要洪泛
    InstalledWithoutFlooding,
    /// LSA 已安装并需要洪泛
    InstalledAndFlood {
        /// 目标邻居列表
        targets: Vec<Ipv4Addr>,
        /// 是否需要触发 SPF 计算
        trigger_spf: bool,
    },
}

/// LSA 洪泛器
pub struct LsaFlooder {
    // 洪泛状态可以在这里维护
}

impl LsaFlooder {
    /// 创建新的 LSA 洪泛器
    pub fn new() -> Self {
        Self {}
    }

    /// 验证 LSA
    fn validate_lsa(&self, lsa: &Lsa) -> bool {
        let header = lsa.header();

        // 检查 LSA 年龄
        if header.age > LsaHeader::MAX_AGE {
            return false;
        }

        // 检查 LSA 校验和
        // 简化实现：假设校验和已在外层验证
        // 实际实现应该重新计算校验和并验证

        // 检查序列号
        if header.sequence_number == 0 {
            return false;
        }

        true
    }

    /// 判断是否需要安装 LSA
    fn should_install_lsa(
        &self,
        lsdb: &crate::protocols::ospf2::lsdb::LinkStateDatabase,
        lsa: &Lsa,
    ) -> bool {
        let header = lsa.header();
        let key = (header.lsa_type, header.link_state_id, header.advertising_router);

        if let Some(entry) = lsdb.lsas.get(&key) {
            // 比较现有 LSA 和新 LSA
            // 如果新 LSA 更新，则需要安装
            if header.sequence_number > entry.header.sequence_number {
                return true;
            }
            // 序列号相同但校验和不同
            if header.sequence_number == entry.header.sequence_number
                && header.checksum != entry.header.checksum {
                return true;
            }
            false
        } else {
            // 数据库中没有此 LSA，需要安装
            true
        }
    }

    /// 处理接收到的 LSA（洪泛的主入口）
    ///
    /// # 参数
    /// - `lsa`: 接收到的 LSA
    /// - `receiving_interface`: 接收接口索引
    /// - `source_router_id`: 发送此 LSA 的路由器 ID
    /// - `lsdb`: 链路状态数据库
    /// - `interfaces`: 所有 OSPF 接口
    /// - `neighbors`: 所有邻居映射
    pub fn process_received_lsa(
        &mut self,
        lsa: Lsa,
        receiving_interface: u32,
        source_router_id: Ipv4Addr,
        lsdb: &mut crate::protocols::ospf2::lsdb::LinkStateDatabase,
        interfaces: &[OspfInterface],
        neighbors: &HashMap<Ipv4Addr, std::sync::Arc<std::sync::Mutex<crate::protocols::ospf2::OspfNeighbor>>>,
    ) -> FloodResult {
        // 1. 验证 LSA
        if !self.validate_lsa(&lsa) {
            return FloodResult::Invalid;
        }

        let header = lsa.header();

        // 2. 判断是否需要安装
        if !self.should_install_lsa(lsdb, &lsa) {
            return FloodResult::AlreadyPresent;
        }

        // 3. 安装 LSA 到 LSDB
        let key = (header.lsa_type, header.link_state_id, header.advertising_router);
        let _is_new = !lsdb.lsas.contains_key(&key);

        // 如果是新的 LSA 或更新的 LSA，需要安装
        if lsdb.install(lsa.clone()).is_err() {
            return FloodResult::Invalid;
        }

        // 4. 确定洪泛目标
        let mut flood_targets = Vec::new();

        for iface in interfaces {
            // 跳过接收接口
            if iface.ifindex == receiving_interface {
                continue;
            }

            // 对于广播网络，只洪泛到 DR/BDR
            // 对于点对点网络，洪泛到邻居
            // 简化实现：洪泛到所有 Full 状态的邻居
            for (neighbor_id, neighbor_arc) in neighbors {
                // 跳过发送此 LSA 的源
                if *neighbor_id == source_router_id {
                    continue;
                }

                // 只洪泛到已建立邻接关系的邻居
                if let Ok(neighbor) = neighbor_arc.lock() {
                    // 检查邻居是否在此接口上
                    // 简化实现：假设所有邻居都在同一接口上
                    if neighbor.state == NeighborState::Full {
                        // 检查是否需要洪泛到此邻居
                        if self.should_flood_to_neighbor(&lsa, iface, &neighbor) {
                            flood_targets.push(*neighbor_id);
                        }
                    }
                }
            }
        }

        // 5. 判断是否需要触发 SPF 计算
        // 如果是区域内 LSA（Router LSA 或 Network LSA），需要触发 SPF
        let trigger_spf = matches!(header.lsa_type, 1 | 2);

        // 6. 如果有需要洪泛的目标，返回洪泛结果
        if !flood_targets.is_empty() {
            FloodResult::InstalledAndFlood {
                targets: flood_targets,
                trigger_spf,
            }
        } else {
            FloodResult::InstalledWithoutFlooding
        }
    }

    /// 判断是否应该洪泛到指定邻居
    fn should_flood_to_neighbor(
        &self,
        lsa: &Lsa,
        _interface: &OspfInterface,
        neighbor: &crate::protocols::ospf2::OspfNeighbor,
    ) -> bool {
        // 只洪泛到 Full 状态的邻居
        if neighbor.state != NeighborState::Full {
            return false;
        }

        let header = lsa.header();

        // 对于 AS External LSA (Type 5)，需要特殊处理
        if header.lsa_type == 5 {
            // 如果是 Stub 区域，不洪泛 AS External LSA
            // 简化实现：假设所有区域都不是 Stub 区域
            return true;
        }

        // 对于区域内 LSA，洪泛到所有邻居
        true
    }

    /// 构建 LSU 报文用于洪泛
    pub fn build_lsu_for_flooding(
        &self,
        lsas: &[Lsa],
        router_id: Ipv4Addr,
        area_id: Ipv4Addr,
    ) -> Result<Vec<u8>, String> {
        use crate::protocols::ospf2::packet::{OspfHeader, OspfLinkStateUpdate, OspfType};

        // 构建 LSU 报文
        let mut lsu = OspfLinkStateUpdate::new();
        for lsa in lsas {
            lsu.add_lsa(lsa.clone());
        }

        // 构建 OSPF 头部
        let mut header = OspfHeader::new(OspfType::LinkStateUpdate, router_id, area_id);
        let lsu_bytes = lsu.to_bytes();
        header.length = (OspfHeader::LENGTH + lsu_bytes.len()) as u16;

        // 计算校验和
        header.calculate_checksum(&lsu_bytes);

        // 组装完整报文
        let mut packet = header.to_bytes();
        packet.extend_from_slice(&lsu_bytes);

        Ok(packet)
    }

    /// 构建 LSAck 报文
    pub fn build_lsack(
        &self,
        lsa_headers: &[LsaHeader],
        router_id: Ipv4Addr,
        area_id: Ipv4Addr,
    ) -> Result<Vec<u8>, String> {
        use crate::protocols::ospf2::packet::{OspfHeader, OspfLinkStateAck, OspfType};

        // 构建 LSAck 报文
        let mut lsack = OspfLinkStateAck::new();
        for lsa_hdr in lsa_headers {
            lsack.add_lsa_header(lsa_hdr.clone());
        }

        // 构建 OSPF 头部
        let mut header = OspfHeader::new(OspfType::LinkStateAck, router_id, area_id);
        let lsack_bytes = lsack.to_bytes();
        header.length = (OspfHeader::LENGTH + lsack_bytes.len()) as u16;

        // 计算校验和
        header.calculate_checksum(&lsack_bytes);

        // 组装完整报文
        let mut packet = header.to_bytes();
        packet.extend_from_slice(&lsack_bytes);

        Ok(packet)
    }
}

impl Default for LsaFlooder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_flooder_new() {
        let _flooder = LsaFlooder::new();
        // 测试创建成功
    }

    #[test]
    fn test_flooder_default() {
        let _flooder = LsaFlooder::default();
        // 测试默认创建成功
    }
}
