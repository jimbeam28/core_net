// src/protocols/ospf/spf.rs
//
// OSPF SPF (Shortest Path First) 计算接口定义（简化版）

use crate::common::Ipv4Addr;

/// SPF 节点类型（路由器或网络）
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SpfNode {
    /// 路由器节点（Router ID）
    Router(Ipv4Addr),
    /// 网络节点（通常是 DR 的接口 IP）
    Network(Ipv4Addr),
}

/// 路由类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RouteType {
    /// 区域内路由
    IntraArea,
    /// 区域间路由
    InterArea,
    /// 类型 1 外部路由（E1）
    External1,
    /// 类型 2 外部路由（E2）
    External2,
}

/// 路由表条目
#[derive(Debug, Clone)]
pub struct RouteEntry {
    /// 目标网络
    pub destination: Ipv4Addr,
    /// 子网掩码
    pub mask: Ipv4Addr,
    /// 下一跳
    pub next_hop: Option<Ipv4Addr>,
    /// 出接口名称
    pub outgoing_interface: String,
    /// 路由类型
    pub route_type: RouteType,
    /// 路由 Cost
    pub cost: u32,
    /// 区域 ID
    pub area_id: Ipv4Addr,
}

/// LSA 链路描述（用于 SPF 计算）
#[derive(Debug, Clone)]
pub struct LsaLink {
    /// 链路 ID（邻居 Router ID 或网络地址）
    pub link_id: Ipv4Addr,
    /// 链路数据（接口 IP 地址）
    pub link_data: Ipv4Addr,
    /// 链路类型
    pub link_type: u8,
    /// 链路度量值（Cost）
    pub metric: u32,
}

/// LSA 描述符（用于 SPF 计算）
#[derive(Debug, Clone)]
pub struct LsaDescriptor {
    /// LSA 类型
    pub lsa_type: u8,
    /// 链路状态 ID
    pub link_state_id: Ipv4Addr,
    /// 通告路由器
    pub advertising_router: Ipv4Addr,
    /// LSA 序列号
    pub sequence_number: u32,
    /// LSA 链路列表
    pub links: Vec<LsaLink>,
}

/// SPF 计算结果
#[derive(Debug, Clone)]
pub struct SpfResult {
    /// 计算生成的路由表
    pub routes: Vec<RouteEntry>,
    /// 计算是否成功
    pub success: bool,
}

impl SpfResult {
    /// 创建空结果
    pub fn empty() -> Self {
        Self {
            routes: Vec::new(),
            success: false,
        }
    }
}

/// 运行 SPF 计算（简化版接口）
///
/// # 参数
/// - `root`: 根节点（通常是本路由器的 Router ID）
/// - `lsas`: LSA 数据库
///
/// # 返回
/// SPF 计算结果
pub fn run_spf_calculation(
    _root: Ipv4Addr,
    _lsas: &std::collections::HashMap<(u8, Ipv4Addr, Ipv4Addr), LsaDescriptor>,
) -> SpfResult {
    // 简化实现：返回空结果
    SpfResult::empty()
}

/// 检查 LSA 是否需要更新
pub fn should_update_lsa(existing_seq: u32, new_seq: u32) -> bool {
    let existing = existing_seq as i32;
    let new = new_seq as i32;
    let diff = new.wrapping_sub(existing);
    diff > 0
}

/// 验证 LSA 校验和（简化实现）
pub fn verify_lsa_checksum(_lsa_data: &[u8]) -> bool {
    true
}

/// 将 SPF 计算结果同步到路由表（简化版）
pub fn sync_spf_routes_to_route_table(
    _spf_result: &SpfResult,
    _route_table: &mut crate::route::RouteTable,
    _area_id: Ipv4Addr,
) -> Result<(), String> {
    // 简化实现：直接返回成功
    Ok(())
}
