use serde::{Deserialize, Serialize};
use std::collections::{BTreeSet, HashMap};

/// 操作ID（各操作を一意に識別）
pub type OpNum = String;

// Node ID
pub type NodeId = String;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub enum NodeExpr {
    AddNode {
        is_literal: bool,
        label: String,
        id: NodeId,
    },
    ExistNode {
        is_literal: bool,
        label: String,
        id: NodeId,
    },
    NullNode,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub enum EdgeExpr {
    AddEdge {
        from: OpNum,
        to: OpNum,
        label: String,
    },
    EditEdgeReference {
        from: OpNum,
        old_to: Option<OpNum>,
        new_to: OpNum,
        label: String,
    },
    DeleteEdge {
        from: OpNum,
        to: OpNum,
        label: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub enum VarOp {
    AddVariable {
        to: OpNum,
        label: String,
    },
    EditVariableReference {
        old_to: OpNum,
        new_to: OpNum,
        label: String,
    },
    DeleteVariable {
        to: OpNum,
        label: String,
    },
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

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct UnificationResult {
    pub common_a: Vec<Op>,
    pub common_b: Vec<Op>,
    pub diff_a: Vec<Op>,
    pub diff_b: Vec<Op>,
    /// Mapping from operation IDs in sequence A to their counterparts in sequence B
    pub final_mapping: HashMap<OpNum, OpNum>,
}

fn relation_endpoint_attributes_match(
    op_a_id: &OpNum,
    op_b_id: &OpNum,
    ops_a_map: &HashMap<OpNum, &Op>,
    ops_b_map: &HashMap<OpNum, &Op>,
) -> bool {
    match (&ops_a_map[op_a_id].kind, &ops_b_map[op_b_id].kind) {
        (
            GraphOp::Node(NodeExpr::ExistNode { id: id_a, .. }),
            GraphOp::Node(NodeExpr::ExistNode { id: id_b, .. }),
        ) => id_a == id_b,
        (GraphOp::Node(_), GraphOp::Node(_)) => true,
        _ => false,
    }
}

fn mapped_targets_are_literal(
    op_a_id: &OpNum,
    op_b_id: &OpNum,
    ops_a_map: &HashMap<OpNum, &Op>,
    ops_b_map: &HashMap<OpNum, &Op>,
) -> bool {
    let is_literal = |op: &&Op| {
        matches!(
            &op.kind,
            GraphOp::Node(NodeExpr::AddNode {
                is_literal: true,
                ..
            }) | GraphOp::Node(NodeExpr::ExistNode {
                is_literal: true,
                ..
            })
        )
    };
    ops_a_map.get(op_a_id).is_some_and(is_literal) && ops_b_map.get(op_b_id).is_some_and(is_literal)
}

pub fn unify_operation_graphs(a: &[Op], b: &[Op]) -> UnificationResult {
    let ops_a_map: HashMap<_, _> = a.iter().map(|op| (op.id.clone(), op)).collect();
    let ops_b_map: HashMap<_, _> = b.iter().map(|op| (op.id.clone(), op)).collect();

    // 1. Group all nodes from B for structural matching.
    type NodeKey = (bool, bool, bool); // (is_add, is_literal, is_null)
    let mut nodes_b_groups: HashMap<NodeKey, Vec<OpNum>> = HashMap::new();
    for op in b {
        match &op.kind {
            GraphOp::Node(NodeExpr::AddNode { is_literal, .. }) => {
                nodes_b_groups
                    .entry((true, *is_literal, false))
                    .or_default()
                    .push(op.id.clone());
            }
            GraphOp::Node(NodeExpr::ExistNode { is_literal, .. }) => {
                nodes_b_groups
                    .entry((false, *is_literal, false))
                    .or_default()
                    .push(op.id.clone());
            }
            GraphOp::Node(NodeExpr::NullNode) => {
                nodes_b_groups
                    .entry((false, false, true))
                    .or_default()
                    .push(op.id.clone());
            }
            _ => {}
        }
    }

    let nodes_a: Vec<OpNum> = a
        .iter()
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
    let final_mapping = best_mapping.0.clone();

    // 3. Classify operations based on the best structural mapping.
    let mut common_a_ids = BTreeSet::new();
    let mut common_b_ids = BTreeSet::new();
    let mut diff_a_ids = BTreeSet::new();
    let mut diff_b_ids = BTreeSet::new();

    // Initially, all ops are considered diffs.
    for op in a {
        diff_a_ids.insert(op.id.clone());
    }
    for op in b {
        diff_b_ids.insert(op.id.clone());
    }

    // Re-classify mapped nodes based on attribute equality.
    for (id_a, id_b) in final_mapping.iter() {
        let op_a = &ops_a_map[id_a];
        let op_b = &ops_b_map[id_b];
        let are_attrs_equal = match (&op_a.kind, &op_b.kind) {
            (
                GraphOp::Node(NodeExpr::AddNode { label: label_a, .. }),
                GraphOp::Node(NodeExpr::AddNode { label: label_b, .. }),
            ) => label_a == label_b,
            (
                GraphOp::Node(NodeExpr::ExistNode { id: id_a_node, .. }),
                GraphOp::Node(NodeExpr::ExistNode { id: id_b_node, .. }),
            ) => id_a_node == id_b_node,
            (GraphOp::Node(NodeExpr::NullNode), GraphOp::Node(NodeExpr::NullNode)) => true,
            _ => false,
        };

        if are_attrs_equal {
            common_a_ids.insert(id_a.clone());
            common_b_ids.insert(id_b.clone());
            diff_a_ids.remove(id_a);
            diff_b_ids.remove(id_b);
        }
    }

    // Classify edges based on structural mapping.
    for edge_a in a.iter().filter(|op| matches!(&op.kind, GraphOp::Edge(_))) {
        if let GraphOp::Edge(edge_expr_a) = &edge_a.kind {
            let b_edge_id = match edge_expr_a {
                EdgeExpr::AddEdge {
                    from: from_a,
                    to: to_a,
                    label: label_a,
                } => {
                    if let (Some(from_b), Some(to_b)) =
                        (final_mapping.get(from_a), final_mapping.get(to_a))
                    {
                        b.iter().find_map(|op_b| {
                            if let GraphOp::Edge(EdgeExpr::AddEdge { from, to, label }) = &op_b.kind
                            {
                                if label == label_a && from == from_b && to == to_b {
                                    return Some(op_b.id.clone());
                                }
                            }
                            None
                        })
                    } else {
                        None
                    }
                }
                EdgeExpr::EditEdgeReference {
                    from: from_a,
                    old_to: old_to_a,
                    new_to: new_to_a,
                    label: label_a,
                } => {
                    if let (Some(from_b), Some(new_to_b)) =
                        (final_mapping.get(from_a), final_mapping.get(new_to_a))
                    {
                        let value_update =
                            mapped_targets_are_literal(new_to_a, new_to_b, &ops_a_map, &ops_b_map);
                        // Literal assignments share one update operation even when the receiver,
                        // previous value, and new value are different mapped nodes. Pointer
                        // rewires retain the stricter rule because their operand relationships
                        // are handled by the existing rewire-hole path.
                        let old_to_attributes_match = match old_to_a {
                            Some(old_to_a) => final_mapping.get(old_to_a).is_some_and(|old_to_b| {
                                relation_endpoint_attributes_match(
                                    old_to_a, old_to_b, &ops_a_map, &ops_b_map,
                                )
                            }),
                            None => true,
                        };
                        let operands_may_unify = value_update
                            || (relation_endpoint_attributes_match(
                                from_a, from_b, &ops_a_map, &ops_b_map,
                            ) && relation_endpoint_attributes_match(
                                new_to_a, new_to_b, &ops_a_map, &ops_b_map,
                            ) && old_to_attributes_match);
                        operands_may_unify.then_some(()).and_then(|_| {
                            b.iter().find_map(|op_b| {
                                if let GraphOp::Edge(EdgeExpr::EditEdgeReference {
                                    from,
                                    old_to,
                                    new_to,
                                    label,
                                }) = &op_b.kind
                                {
                                    let old_to_matches = match (old_to_a, old_to.as_ref()) {
                                        (Some(old_to_a), Some(old_to_b)) => {
                                            final_mapping.get(old_to_a) == Some(old_to_b)
                                        }
                                        (None, None) => true,
                                        _ => false,
                                    };
                                    if label == label_a
                                        && from == from_b
                                        && new_to == new_to_b
                                        && old_to_matches
                                    {
                                        return Some(op_b.id.clone());
                                    }
                                }
                                None
                            })
                        })
                    } else {
                        None
                    }
                }
                EdgeExpr::DeleteEdge {
                    from: from_a,
                    to: to_a,
                    label: label_a,
                } => {
                    if let (Some(from_b), Some(to_b)) =
                        (final_mapping.get(from_a), final_mapping.get(to_a))
                    {
                        b.iter().find_map(|op_b| {
                            if let GraphOp::Edge(EdgeExpr::DeleteEdge { from, to, label }) =
                                &op_b.kind
                            {
                                if label == label_a && from == from_b && to == to_b {
                                    return Some(op_b.id.clone());
                                }
                            }
                            None
                        })
                    } else {
                        None
                    }
                }
            };

            if let Some(id_b) = b_edge_id {
                common_a_ids.insert(edge_a.id.clone());
                common_b_ids.insert(id_b.clone());
                diff_a_ids.remove(&edge_a.id);
                diff_b_ids.remove(&id_b);
            }
        }
    }

    // Classify variables based on structural mapping.
    for var_a in a
        .iter()
        .filter(|op| matches!(&op.kind, GraphOp::Variable(_)))
    {
        if let GraphOp::Variable(var_expr_a) = &var_a.kind {
            let b_var_id = match var_expr_a {
                VarOp::AddVariable {
                    to: to_a,
                    label: label_a,
                } => {
                    if let Some(to_b) = final_mapping.get(to_a) {
                        b.iter().find_map(|op_b| {
                            if let GraphOp::Variable(VarOp::AddVariable { to, label }) = &op_b.kind
                            {
                                if label == label_a && to == to_b {
                                    return Some(op_b.id.clone());
                                }
                            }
                            None
                        })
                    } else {
                        None
                    }
                }
                VarOp::EditVariableReference {
                    old_to: old_to_a,
                    new_to: new_to_a,
                    label: label_a,
                } => {
                    if let (Some(old_to_b), Some(new_to_b)) =
                        (final_mapping.get(old_to_a), final_mapping.get(new_to_a))
                    {
                        b.iter().find_map(|op_b| {
                            if let GraphOp::Variable(VarOp::EditVariableReference {
                                old_to,
                                new_to,
                                label,
                            }) = &op_b.kind
                            {
                                if label == label_a && old_to == old_to_b && new_to == new_to_b {
                                    return Some(op_b.id.clone());
                                }
                            }
                            None
                        })
                    } else {
                        None
                    }
                }
                VarOp::DeleteVariable {
                    to: to_a,
                    label: label_a,
                } => {
                    if let Some(to_b) = final_mapping.get(to_a) {
                        b.iter().find_map(|op_b| {
                            if let GraphOp::Variable(VarOp::DeleteVariable { to, label }) =
                                &op_b.kind
                            {
                                if label == label_a && to == to_b {
                                    return Some(op_b.id.clone());
                                }
                            }
                            None
                        })
                    } else {
                        None
                    }
                }
            };

            if let Some(id_b) = b_var_id {
                common_a_ids.insert(var_a.id.clone());
                common_b_ids.insert(id_b.clone());
                diff_a_ids.remove(&var_a.id);
                diff_b_ids.remove(&id_b);
            }
        }
    }

    let mut result = UnificationResult {
        common_a: a
            .iter()
            .filter(|op| common_a_ids.contains(&op.id))
            .cloned()
            .collect(),
        common_b: b
            .iter()
            .filter(|op| common_b_ids.contains(&op.id))
            .cloned()
            .collect(),
        diff_a: a
            .iter()
            .filter(|op| diff_a_ids.contains(&op.id))
            .cloned()
            .collect(),
        diff_b: b
            .iter()
            .filter(|op| diff_b_ids.contains(&op.id))
            .cloned()
            .collect(),
        final_mapping,
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

    // Score for edges
    for edge_a in ops_a_map
        .values()
        .filter(|op| matches!(&op.kind, GraphOp::Edge(_)))
    {
        if let GraphOp::Edge(edge_expr_a) = &edge_a.kind {
            let found_matching_edge = match edge_expr_a {
                EdgeExpr::AddEdge {
                    from: from_a,
                    to: to_a,
                    label: label_a,
                } => {
                    if let (Some(from_b), Some(to_b)) = (mapping.get(from_a), mapping.get(to_a)) {
                        ops_b_map.values().any(|edge_b| {
                            matches!(&edge_b.kind, GraphOp::Edge(EdgeExpr::AddEdge { from, to, label }) if label == label_a && from == from_b && to == to_b)
                        })
                    } else {
                        false
                    }
                }
                EdgeExpr::EditEdgeReference {
                    from: from_a,
                    old_to: old_to_a,
                    new_to: new_to_a,
                    label: label_a,
                } => {
                    if let (Some(from_b), Some(new_to_b)) =
                        (mapping.get(from_a), mapping.get(new_to_a))
                    {
                        let value_update =
                            mapped_targets_are_literal(new_to_a, new_to_b, ops_a_map, ops_b_map);
                        let old_to_attributes_match = match old_to_a {
                            Some(old_to_a) => mapping.get(old_to_a).is_some_and(|old_to_b| {
                                relation_endpoint_attributes_match(
                                    old_to_a, old_to_b, ops_a_map, ops_b_map,
                                )
                            }),
                            None => true,
                        };
                        let operands_may_unify = value_update
                            || (relation_endpoint_attributes_match(
                                from_a, from_b, ops_a_map, ops_b_map,
                            ) && relation_endpoint_attributes_match(
                                new_to_a, new_to_b, ops_a_map, ops_b_map,
                            ) && old_to_attributes_match);
                        operands_may_unify
                            && ops_b_map.values().any(|edge_b| {
                                let GraphOp::Edge(EdgeExpr::EditEdgeReference {
                                    from,
                                    old_to,
                                    new_to,
                                    label,
                                }) = &edge_b.kind
                                else {
                                    return false;
                                };
                                let old_to_matches = match (old_to_a, old_to.as_ref()) {
                                    (Some(old_to_a), Some(old_to_b)) => {
                                        mapping.get(old_to_a) == Some(old_to_b)
                                    }
                                    (None, None) => true,
                                    _ => false,
                                };
                                label == label_a
                                    && from == from_b
                                    && new_to == new_to_b
                                    && old_to_matches
                            })
                    } else {
                        false
                    }
                }
                EdgeExpr::DeleteEdge {
                    from: from_a,
                    to: to_a,
                    label: label_a,
                } => {
                    if let (Some(from_b), Some(to_b)) = (mapping.get(from_a), mapping.get(to_a)) {
                        ops_b_map.values().any(|edge_b| {
                            matches!(&edge_b.kind, GraphOp::Edge(EdgeExpr::DeleteEdge { from, to, label }) if label == label_a && from == from_b && to == to_b)
                        })
                    } else {
                        false
                    }
                }
            };
            if found_matching_edge {
                score += 1;
            }
        }
    }

    // Score for variables
    for var_a in ops_a_map
        .values()
        .filter(|op| matches!(&op.kind, GraphOp::Variable(_)))
    {
        if let GraphOp::Variable(var_expr_a) = &var_a.kind {
            let found_matching_var = match var_expr_a {
                VarOp::AddVariable {
                    to: to_a,
                    label: label_a,
                } => {
                    if let Some(to_b) = mapping.get(to_a) {
                        ops_b_map.values().any(|var_b| {
                            matches!(&var_b.kind, GraphOp::Variable(VarOp::AddVariable { to, label }) if label == label_a && to == to_b)
                        })
                    } else {
                        false
                    }
                }
                VarOp::EditVariableReference {
                    old_to: old_to_a,
                    new_to: new_to_a,
                    label: label_a,
                } => {
                    if let (Some(old_to_b), Some(new_to_b)) =
                        (mapping.get(old_to_a), mapping.get(new_to_a))
                    {
                        ops_b_map.values().any(|var_b| {
                            matches!(&var_b.kind, GraphOp::Variable(VarOp::EditVariableReference { old_to, new_to, label }) if label == label_a && old_to == old_to_b && new_to == new_to_b)
                        })
                    } else {
                        false
                    }
                }
                VarOp::DeleteVariable {
                    to: to_a,
                    label: label_a,
                } => {
                    if let Some(to_b) = mapping.get(to_a) {
                        ops_b_map.values().any(|var_b| {
                            matches!(&var_b.kind, GraphOp::Variable(VarOp::DeleteVariable { to, label }) if label == label_a && to == to_b)
                        })
                    } else {
                        false
                    }
                }
            };
            if found_matching_var {
                score += 1;
            }
        }
    }

    score
}

fn find_best_mapping_recursive<'a>(
    nodes_to_map_a: &[OpNum],
    mappable_nodes_b: &HashMap<(bool, bool, bool), Vec<OpNum>>,
    current_mapping: &mut HashMap<OpNum, OpNum>,
    best_mapping: &mut (HashMap<OpNum, OpNum>, usize),
    ops_a_map: &HashMap<OpNum, &'a Op>,
    ops_b_map: &HashMap<OpNum, &'a Op>,
) {
    if nodes_to_map_a.is_empty() {
        let score = calculate_structural_score(current_mapping, ops_a_map, ops_b_map);
        if score > best_mapping.1
            || (score == best_mapping.1 && current_mapping.len() > best_mapping.0.len())
        {
            best_mapping.0 = current_mapping.clone();
            best_mapping.1 = score;
        }
        return;
    }

    let op_a_id = &nodes_to_map_a[0];
    let op_a = ops_a_map[op_a_id];
    let remaining_nodes_a = &nodes_to_map_a[1..];

    if let GraphOp::Node(node_expr_a) = &op_a.kind {
        let key = match node_expr_a {
            NodeExpr::AddNode { is_literal, .. } => (true, *is_literal, false),
            NodeExpr::ExistNode { is_literal, .. } => (false, *is_literal, false),
            NodeExpr::NullNode => (false, false, true),
        };

        // Path 1: Explore mapping the current node
        if let Some(candidates) = mappable_nodes_b.get(&key) {
            for op_b_id in candidates {
                if current_mapping.values().any(|id| id == op_b_id) {
                    continue; // Already mapped
                }
                current_mapping.insert(op_a_id.clone(), op_b_id.clone());
                find_best_mapping_recursive(
                    remaining_nodes_a,
                    mappable_nodes_b,
                    current_mapping,
                    best_mapping,
                    ops_a_map,
                    ops_b_map,
                );
                current_mapping.remove(op_a_id);
            }
        }
    }

    // Path 2: Explore NOT mapping the current node
    find_best_mapping_recursive(
        remaining_nodes_a,
        mappable_nodes_b,
        current_mapping,
        best_mapping,
        ops_a_map,
        ops_b_map,
    );
}
