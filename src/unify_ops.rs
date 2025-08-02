use std::collections::{HashMap, BTreeSet};
use serde::{Deserialize, Serialize};

/// 操作ID（各操作を一意に識別）
pub type OpNum = String;

// Node ID
pub type NodeId = String;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub enum NodeExpr {
    AddNode {is_literal: bool, label: String, id: NodeId},
    ExistNode {is_literal: bool, label: String, id: NodeId},
    NullNode
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub enum EdgeExpr {
    AddEdge { from: OpNum, to: OpNum, label: String },
    EditEdgeReference { from: OpNum, old_to: OpNum, new_to: OpNum, label: String },
    DeleteEdge { from: OpNum, to: OpNum, label: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub enum VarOp {
    AddVariable { to: OpNum, label: String },
    EditVariableReference { old_to: OpNum, new_to: OpNum, label: String },
    DeleteVariable { to: OpNum, label: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub enum GraphOp {
    Node(NodeExpr),
    Edge(EdgeExpr),
    Variable(VarOp),
}

/// 単一の操作
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct Op {
    pub id: OpNum,
    pub kind: GraphOp,
}

#[derive(Debug, PartialEq, Eq)]
pub struct UnificationResult {
    pub common_a: Vec<Op>,
    pub common_b: Vec<Op>,
    pub diff_a: Vec<Op>,
    pub diff_b: Vec<Op>,
}

pub fn unify_operation_graphs(a: &[Op], b: &[Op]) -> UnificationResult {
    let ops_a_map: HashMap<_, _> = a.iter().map(|op| (op.id.clone(), op)).collect();
    let ops_b_map: HashMap<_, _> = b.iter().map(|op| (op.id.clone(), op)).collect();

    // 1. Group all nodes from B for structural matching.
    // Key: (is_add_node, is_literal)
    type NodeKey = (bool, bool);
    let mut nodes_b_groups: HashMap<NodeKey, Vec<OpNum>> = HashMap::new();
    for op in b {
        match &op.kind {
            GraphOp::Node(NodeExpr::AddNode { is_literal, .. }) => {
                nodes_b_groups.entry((true, *is_literal)).or_default().push(op.id.clone());
            }
            GraphOp::Node(NodeExpr::ExistNode { is_literal, .. }) => {
                nodes_b_groups.entry((false, *is_literal)).or_default().push(op.id.clone());
            }
            _ => {}
        }
    }

    let nodes_a: Vec<OpNum> = a.iter()
        .filter(|op| matches!(&op.kind, GraphOp::Node(_)))
        .map(|op| op.id.clone())
        .collect();

    // 2. Find the best structural mapping.
    let mut best_mapping = (HashMap::new(), 0);
    find_best_mapping_recursive(
        &nodes_a,
        &nodes_b_groups,
        &mut HashMap::new(),
        &mut best_mapping,
        &ops_a_map,
        &ops_b_map,
    );
    let final_mapping = &best_mapping.0;

    // 3. Classify operations based on the best structural mapping.
    let mut common_a_ids = BTreeSet::new();
    let mut common_b_ids = BTreeSet::new();
    let mut diff_a_ids = BTreeSet::new();
    let mut diff_b_ids = BTreeSet::new();

    // Initially, all ops are considered diffs.
    for op in a { diff_a_ids.insert(op.id.clone()); }
    for op in b { diff_b_ids.insert(op.id.clone()); }

    // Re-classify mapped nodes based on attribute equality.
    for (id_a, id_b) in final_mapping.iter() {
        let op_a = &ops_a_map[id_a];
        let op_b = &ops_b_map[id_b];
        let are_attrs_equal = match (&op_a.kind, &op_b.kind) {
            (GraphOp::Node(NodeExpr::AddNode { label: label_a, .. }), GraphOp::Node(NodeExpr::AddNode { label: label_b, .. })) => label_a == label_b,
            (GraphOp::Node(NodeExpr::ExistNode { id: id_a_node, .. }), GraphOp::Node(NodeExpr::ExistNode { id: id_b_node, .. })) => id_a_node == id_b_node,
            _ => false, // Should not happen with the current grouping logic
        };

        if are_attrs_equal {
            common_a_ids.insert(id_a.clone());
            common_b_ids.insert(id_b.clone());
            diff_a_ids.remove(id_a);
            diff_b_ids.remove(id_b);
        }
    }

    // Classify edges based on structural mapping.
    let edges_a: Vec<_> = a.iter().filter(|op| matches!(&op.kind, GraphOp::Edge(_))).collect();
    for edge_a in &edges_a {
        if let GraphOp::Edge(EdgeExpr::AddEdge { from: from_a, to: to_a, label: label_a }) = &edge_a.kind {
            if let (Some(from_b), Some(to_b)) = (final_mapping.get(from_a), final_mapping.get(to_a)) {
                // Search for a corresponding edge in b.
                let found_edge_b = b.iter().any(|op_b| {
                    if let GraphOp::Edge(EdgeExpr::AddEdge { from, to, label }) = &op_b.kind {
                        label == label_a && from == from_b && to == to_b
                    } else { false }
                });

                if found_edge_b {
                    common_a_ids.insert(edge_a.id.clone());
                    diff_a_ids.remove(&edge_a.id);
                    // Also find and move the corresponding b_edge to common.
                    b.iter().for_each(|op_b| {
                        if let GraphOp::Edge(EdgeExpr::AddEdge { from, to, label }) = &op_b.kind {
                            if label == label_a && from == from_b && to == to_b {
                                common_b_ids.insert(op_b.id.clone());
                                diff_b_ids.remove(&op_b.id);
                            }
                        }
                    });
                }
            }
        }
    }

    let mut result = UnificationResult {
        common_a: a.iter().filter(|op| common_a_ids.contains(&op.id)).cloned().collect(),
        common_b: b.iter().filter(|op| common_b_ids.contains(&op.id)).cloned().collect(),
        diff_a: a.iter().filter(|op| diff_a_ids.contains(&op.id)).cloned().collect(),
        diff_b: b.iter().filter(|op| diff_b_ids.contains(&op.id)).cloned().collect(),
    };

    result.common_a.sort_by_key(|k| format!("{:?}", k));
    result.common_b.sort_by_key(|k| format!("{:?}", k));
    result.diff_a.sort_by_key(|k| format!("{:?}", k));
    result.diff_b.sort_by_key(|k| format!("{:?}", k));

    result
}

fn calculate_structural_score(
    mapping: &HashMap<OpNum, OpNum>,
    ops_a_map: &HashMap<OpNum, &Op>,
    ops_b_map: &HashMap<OpNum, &Op>,
) -> usize {
    let mut score = 0;
    for edge_a in ops_a_map.values().filter(|op| matches!(&op.kind, GraphOp::Edge(_))) {
        if let GraphOp::Edge(EdgeExpr::AddEdge { from: from_a, to: to_a, label: label_a }) = &edge_a.kind {
            if let (Some(from_b), Some(to_b)) = (mapping.get(from_a), mapping.get(to_a)) {
                for edge_b in ops_b_map.values().filter(|op| matches!(&op.kind, GraphOp::Edge(_))) {
                    if let GraphOp::Edge(EdgeExpr::AddEdge { from, to, label }) = &edge_b.kind {
                        if label == label_a && from == from_b && to == to_b {
                            score += 1;
                        }
                    }
                }
            }
        }
    }
    score
}

fn find_best_mapping_recursive<'a>(
    nodes_to_map_a: &[OpNum],
    mappable_nodes_b: &HashMap<(bool, bool), Vec<OpNum>>,
    current_mapping: &mut HashMap<OpNum, OpNum>,
    best_mapping: &mut (HashMap<OpNum, OpNum>, usize),
    ops_a_map: &HashMap<OpNum, &'a Op>,
    ops_b_map: &HashMap<OpNum, &'a Op>,
) {
    if nodes_to_map_a.is_empty() {
        let score = calculate_structural_score(current_mapping, ops_a_map, ops_b_map);
        if score > best_mapping.1 {
            best_mapping.0 = current_mapping.clone();
            best_mapping.1 = score;
        }
        return;
    }

    let op_a_id = &nodes_to_map_a[0];
    let op_a = ops_a_map[op_a_id];
    let remaining_nodes_a = &nodes_to_map_a[1..];

    let key = match &op_a.kind {
        GraphOp::Node(NodeExpr::AddNode { is_literal, .. }) => (true, *is_literal),
        GraphOp::Node(NodeExpr::ExistNode { is_literal, .. }) => (false, *is_literal),
        _ => return, // Should not happen
    };

    // Path 1: Explore mapping the current node
    if let Some(candidates) = mappable_nodes_b.get(&key) {
        for op_b_id in candidates {
            if current_mapping.values().any(|id| id == op_b_id) {
                continue;
            }
            current_mapping.insert(op_a_id.clone(), op_b_id.clone());
            find_best_mapping_recursive(remaining_nodes_a, mappable_nodes_b, current_mapping, best_mapping, ops_a_map, ops_b_map);
            current_mapping.remove(op_a_id);
        }
    }

    // Path 2: Explore NOT mapping the current node
    find_best_mapping_recursive(remaining_nodes_a, mappable_nodes_b, current_mapping, best_mapping, ops_a_map, ops_b_map);
}
