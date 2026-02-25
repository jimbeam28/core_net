// src/protocols/ospf/spf.rs
//
// OSPF SPF (Shortest Path First) 计算实现
// 使用 Dijkstra 算法计算最短路径树

use crate::common::Ipv4Addr;
use std::collections::{HashMap, HashSet, BinaryHeap};
use std::cmp::Ordering;

/// SPF 节点类型（路由器或网络）
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SpfNode {
    /// 路由器节点（Router ID）
    Router(Ipv4Addr),
    /// 网络节点（通常是 DR 的接口 IP）
    Network(Ipv4Addr),
}

impl SpfNode {
    /// 获取节点 ID
    pub fn id(&self) -> Ipv4Addr {
        match self {
            SpfNode::Router(id) => *id,
            SpfNode::Network(id) => *id,
        }
    }

    /// 是否是路由器节点
    pub fn is_router(&self) -> bool {
        matches!(self, SpfNode::Router(_))
    }

    /// 是否是网络节点
    pub fn is_network(&self) -> bool {
        matches!(self, SpfNode::Network(_))
    }
}

impl std::fmt::Display for SpfNode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SpfNode::Router(id) => write!(f, "Router({})", id),
            SpfNode::Network(id) => write!(f, "Network({})", id),
        }
    }
}

/// SPF 顶点（用于 Dijkstra 算法）
#[derive(Debug, Clone)]
pub struct SpfVertex {
    /// 节点类型
    pub node: SpfNode,
    /// 从根到该节点的距离（Cost）
    pub distance: u32,
    /// 父节点（用于回溯路径）
    pub parent: Option<Box<SpfVertex>>,
    /// 关联的 LSA
    pub lsa_id: Option<Ipv4Addr>,
}

impl SpfVertex {
    /// 创建新的顶点
    pub fn new(node: SpfNode, distance: u32) -> Self {
        Self {
            node,
            distance,
            parent: None,
            lsa_id: None,
        }
    }

    /// 设置父节点
    pub fn with_parent(mut self, parent: SpfVertex) -> Self {
        self.parent = Some(Box::new(parent));
        self
    }

    /// 设置 LSA ID
    pub fn with_lsa_id(mut self, lsa_id: Ipv4Addr) -> Self {
        self.lsa_id = Some(lsa_id);
        self
    }
}

/// 为 BinaryHeap 实现排序（最小堆）
impl PartialEq for SpfVertex {
    fn eq(&self, other: &Self) -> bool {
        self.distance == other.distance && self.node == other.node
    }
}

impl Eq for SpfVertex {}

impl Ord for SpfVertex {
    fn cmp(&self, other: &Self) -> Ordering {
        // 先按距离排序，距离相同时按节点 ID 排序
        match self.distance.cmp(&other.distance) {
            Ordering::Equal => self.node.id().cmp(&other.node.id()),
            other => other,
        }
    }
}

impl PartialOrd for SpfVertex {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
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

impl RouteType {
    pub fn name(&self) -> &'static str {
        match self {
            RouteType::IntraArea => "Intra-Area",
            RouteType::InterArea => "Inter-Area",
            RouteType::External1 => "External1",
            RouteType::External2 => "External2",
        }
    }
}

impl std::fmt::Display for RouteType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name())
    }
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

impl RouteEntry {
    /// 创建新的路由条目
    pub fn new(
        destination: Ipv4Addr,
        mask: Ipv4Addr,
        next_hop: Option<Ipv4Addr>,
        outgoing_interface: String,
        route_type: RouteType,
        cost: u32,
        area_id: Ipv4Addr,
    ) -> Self {
        Self {
            destination,
            mask,
            next_hop,
            outgoing_interface,
            route_type,
            cost,
            area_id,
        }
    }

    /// 获取前缀长度（简化实现）
    pub fn prefix_length(&self) -> u8 {
        // 简化实现：计算掩码中 1 的个数
        let mask_val = self.mask.to_u32();
        if mask_val == 0 {
            0
        } else {
            32 - mask_val.leading_zeros() as u8
        }
    }

    /// 是否是默认路由
    pub fn is_default_route(&self) -> bool {
        self.destination.is_zero() && self.mask.is_zero()
    }
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

impl LsaDescriptor {
    /// 创建 Router LSA 描述符
    pub fn router_lsa(
        link_state_id: Ipv4Addr,
        advertising_router: Ipv4Addr,
        sequence_number: u32,
        links: Vec<LsaLink>,
    ) -> Self {
        Self {
            lsa_type: 1,  // Router LSA
            link_state_id,
            advertising_router,
            sequence_number,
            links,
        }
    }

    /// 创建 Network LSA 描述符
    pub fn network_lsa(
        link_state_id: Ipv4Addr,
        advertising_router: Ipv4Addr,
        sequence_number: u32,
        attached_routers: Vec<Ipv4Addr>,
    ) -> Self {
        // Network LSA 将连接的路由器转换为链路描述
        let links = attached_routers.into_iter()
            .map(|router_id| LsaLink {
                link_id: router_id,
                link_data: Ipv4Addr::unspecified(),
                link_type: 2,  // Transit network
                metric: 0,
            })
            .collect();

        Self {
            lsa_type: 2,  // Network LSA
            link_state_id,
            advertising_router,
            sequence_number,
            links,
        }
    }
}

/// SPF 计算结果
#[derive(Debug, Clone)]
pub struct SpfResult {
    /// 计算生成的路由表
    pub routes: Vec<RouteEntry>,
    /// 最短路径树
    pub shortest_path_tree: Vec<SpfVertex>,
    /// 计算是否成功
    pub success: bool,
}

impl SpfResult {
    /// 创建空结果
    pub fn empty() -> Self {
        Self {
            routes: Vec::new(),
            shortest_path_tree: Vec::new(),
            success: false,
        }
    }

    /// 创建成功结果
    pub fn success(routes: Vec<RouteEntry>, spt: Vec<SpfVertex>) -> Self {
        Self {
            routes,
            shortest_path_tree: spt,
            success: true,
        }
    }
}

/// 运行 SPF 计算（Dijkstra 算法）
///
/// # 参数
/// - `root`: 根节点（通常是本路由器的 Router ID）
/// - `lsas`: LSA 数据库（键为 (LSA Type, Link State ID, Advertising Router)）
///
/// # 返回
/// SPF 计算结果，包含最短路径树和路由表
pub fn run_spf_calculation(
    root: Ipv4Addr,
    lsas: &HashMap<(u8, Ipv4Addr, Ipv4Addr), LsaDescriptor>,
) -> SpfResult {
    // 初始化
    let mut candidates = BinaryHeap::new();
    let mut shortest_path_tree: Vec<SpfVertex> = Vec::new();
    let mut calculated = HashSet::new();
    let mut routes: Vec<RouteEntry> = Vec::new();

    // 将根节点加入候选列表
    let root_vertex = SpfVertex::new(SpfNode::Router(root), 0);
    candidates.push(root_vertex);

    // Dijkstra 主循环
    while let Some(current) = candidates.pop() {
        let current_id = current.node.id();

        // 如果已计算过，跳过
        if calculated.contains(&current_id) {
            continue;
        }

        // 标记为已计算
        calculated.insert(current_id);
        shortest_path_tree.push(current.clone());

        // 查找当前节点的 LSA
        let lsa_key = if current.node.is_router() {
            (1, current_id, current_id)  // Router LSA
        } else {
            (2, current_id, current_id)  // Network LSA
        };

        if let Some(lsa) = lsas.get(&lsa_key) {
            // 处理 LSA 的每条链路
            for link in &lsa.links {
                let neighbor_id = link.link_id;
                let new_distance = current.distance + link.metric;

                // 跳过已计算的节点
                if calculated.contains(&neighbor_id) {
                    continue;
                }

                // 创建邻居顶点
                let neighbor_node = if link.link_type == 2 {
                    // Transit network - 使用网络节点
                    SpfNode::Network(neighbor_id)
                } else {
                    // Point-to-point link - 使用路由器节点
                    SpfNode::Router(neighbor_id)
                };

                let neighbor_vertex = SpfVertex::new(neighbor_node, new_distance)
                    .with_parent(current.clone())
                    .with_lsa_id(lsa.link_state_id);

                candidates.push(neighbor_vertex);

                // 生成路由条目（简化实现）
                if link.link_type == 3 || link.link_type == 1 {
                    // Point-to-point 或 Stub 网络
                    let route = RouteEntry::new(
                        link.link_id,
                        Ipv4Addr::new(255, 255, 255, 255),  // 简化：假设 /32
                        Some(current_id),
                        "unknown".to_string(),
                        RouteType::IntraArea,
                        new_distance,
                        Ipv4Addr::new(0, 0, 0, 0),
                    );
                    routes.push(route);
                }
            }
        }
    }

    SpfResult::success(routes, shortest_path_tree)
}

/// 检查 LSA 是否需要更新
///
/// 比较两个 LSA 的序列号，确定是否需要更新数据库
pub fn should_update_lsa(
    existing_seq: u32,
    new_seq: u32,
) -> bool {
    // RFC 2328: LSA 序列号比较逻辑
    // 使用有符号比较处理回卷
    let existing = existing_seq as i32;
    let new = new_seq as i32;
    let diff = new.wrapping_sub(existing);

    // 如果差值在有效范围内且新序列号更大，则更新
    diff > 0 && diff <= 0x7FFFFFFF
}

/// 验证 LSA 校验和（简化实现）
///
/// 实际实现需要计算 RFC 2328 定义的 Fletcher 校验和
pub fn verify_lsa_checksum(_lsa_data: &[u8]) -> bool {
    // 简化实现：总是返回 true
    // 实际实现需要计算校验和
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spf_node() {
        let router = SpfNode::Router(Ipv4Addr::new(1, 1, 1, 1));
        assert!(router.is_router());
        assert_eq!(router.id(), Ipv4Addr::new(1, 1, 1, 1));

        let network = SpfNode::Network(Ipv4Addr::new(192, 168, 1, 1));
        assert!(network.is_network());
        assert_eq!(network.id(), Ipv4Addr::new(192, 168, 1, 1));
    }

    #[test]
    fn test_spf_vertex() {
        let vertex = SpfVertex::new(
            SpfNode::Router(Ipv4Addr::new(1, 1, 1, 1)),
            10,
        );

        assert_eq!(vertex.distance, 10);
        assert!(vertex.parent.is_none());

        let parent = SpfVertex::new(
            SpfNode::Router(Ipv4Addr::new(1, 1, 1, 2)),
            5,
        );
        let child = vertex.clone().with_parent(parent);

        assert!(child.parent.is_some());
    }

    #[test]
    fn test_route_entry() {
        let route = RouteEntry::new(
            Ipv4Addr::new(192, 168, 1, 0),
            Ipv4Addr::new(255, 255, 255, 0),
            Some(Ipv4Addr::new(10, 0, 0, 1)),
            "eth0".to_string(),
            RouteType::IntraArea,
            10,
            Ipv4Addr::new(0, 0, 0, 0),
        );

        assert_eq!(route.destination, Ipv4Addr::new(192, 168, 1, 0));
        assert_eq!(route.next_hop, Some(Ipv4Addr::new(10, 0, 0, 1)));
        assert_eq!(route.route_type, RouteType::IntraArea);
        assert!(!route.is_default_route());
    }

    #[test]
    fn test_lsa_descriptor() {
        let links = vec![
            LsaLink {
                link_id: Ipv4Addr::new(192, 168, 1, 1),
                link_data: Ipv4Addr::new(10, 0, 0, 1),
                link_type: 1,
                metric: 10,
            },
        ];

        let lsa = LsaDescriptor::router_lsa(
            Ipv4Addr::new(1, 1, 1, 1),
            Ipv4Addr::new(1, 1, 1, 1),
            0x80000001,
            links,
        );

        assert_eq!(lsa.lsa_type, 1);
        assert_eq!(lsa.links.len(), 1);
    }

    #[test]
    fn test_should_update_lsa() {
        // 新序列号更大
        assert!(should_update_lsa(0x80000001, 0x80000002));

        // 新序列号更小
        assert!(!should_update_lsa(0x80000002, 0x80000001));

        // 序列号回卷
        assert!(should_update_lsa(0x7FFFFFFF, 0x80000001));
    }

    #[test]
    fn test_run_spf_calculation_simple() {
        let mut lsas = HashMap::new();

        // 创建简单的拓扑：
        // Router A (root) --10--> Router B --5--> Router C
        //                  |
        //                  1
        //                  |
        //               Network 192.168.1.0/24

        let root = Ipv4Addr::new(1, 1, 1, 1);

        // Router A LSA
        let router_a_links = vec![
            LsaLink {
                link_id: Ipv4Addr::new(1, 1, 1, 2),  // Router B
                link_data: Ipv4Addr::new(10, 0, 0, 1),
                link_type: 1,
                metric: 10,
            },
            LsaLink {
                link_id: Ipv4Addr::new(192, 168, 1, 0),  // Network
                link_data: Ipv4Addr::new(10, 0, 0, 1),
                link_type: 3,
                metric: 1,
            },
        ];

        lsas.insert(
            (1, root, root),
            LsaDescriptor::router_lsa(root, root, 0x80000001, router_a_links),
        );

        // Router B LSA
        let router_b = Ipv4Addr::new(1, 1, 1, 2);
        let router_b_links = vec![
            LsaLink {
                link_id: root,  // Router A
                link_data: Ipv4Addr::new(10, 0, 0, 2),
                link_type: 1,
                metric: 10,
            },
            LsaLink {
                link_id: Ipv4Addr::new(1, 1, 1, 3),  // Router C
                link_data: Ipv4Addr::new(10, 0, 0, 2),
                link_type: 1,
                metric: 5,
            },
        ];

        lsas.insert(
            (1, router_b, router_b),
            LsaDescriptor::router_lsa(router_b, router_b, 0x80000001, router_b_links),
        );

        // 运行 SPF 计算
        let result = run_spf_calculation(root, &lsas);

        assert!(result.success);
        assert!(!result.shortest_path_tree.is_empty());
    }

    #[test]
    fn test_route_type_display() {
        assert_eq!(RouteType::IntraArea.to_string(), "Intra-Area");
        assert_eq!(RouteType::InterArea.to_string(), "Inter-Area");
        assert_eq!(RouteType::External1.to_string(), "External1");
        assert_eq!(RouteType::External2.to_string(), "External2");
    }
}
