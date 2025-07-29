use std::collections::{HashMap, BTreeSet};
use serde::{Deserialize, Serialize};

/// 操作ID（各操作を一意に識別）
pub type OpNum = String;

/// ノードID（グラフ内のオブジェクト/値を識別）
pub type NodeId = String;


#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub enum NodeExpr {
    AddNode {is_literal: bool, label: String},
    ExistNode {is_literal: bool, label: String},
    NullNode
}

/// 操作の種類
#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub enum GraphOp {
    // ノード操作
    Node(NodeExpr),
    DeleteNode(NodeExpr),

    // エッジ操作
    AddEdge { from: NodeExpr, to: NodeExpr, label: String },
    EditEdgeReference { from: NodeExpr, old_to: NodeExpr, new_to: NodeExpr, label: String },
    DeleteEdge { from: NodeExpr, to: NodeExpr, label: String },

    // 変数操作
    AddVariable { to: NodeExpr, label: String },
    EditVariableReference { old_to: NodeExpr, new_to: NodeExpr, label: String },
    DeleteVariable { to: NodeExpr, label: String }
}

/// 単一の操作
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct Op {
    pub id: OpNum,
    pub kind: GraphOp,
}

/// Canonicalize operations by renaming IDs.
pub fn canonicalize_ops(ops: &[Op]) -> Vec<GraphOp> {
    let mut map: HashMap<String, String> = HashMap::new();
    let mut new_cnt = 0;
    let mut lit_cnt = 0;
    let mut ex_cnt = 0;
    let mut result = Vec::new();

    let mut canonize_node_expr = |node_expr: &NodeExpr, map: &mut HashMap<String, String>| -> NodeExpr {
        match node_expr {
            NodeExpr::AddNode { is_literal, label } => {
                if !map.contains_key(label) {
                    let canon_label = if *is_literal {
                        lit_cnt += 1;
                        format!("L{}", lit_cnt)
                    } else {
                        new_cnt += 1;
                        format!("N{}", new_cnt)
                    };
                    map.insert(label.clone(), canon_label);
                }
                let new_label = map.get(label).unwrap().clone();
                NodeExpr::AddNode { is_literal: *is_literal, label: new_label }
            },
            NodeExpr::ExistNode { is_literal, label } => {
                if !map.contains_key(label) {
                    let canon_label = if *is_literal {
                        lit_cnt += 1;
                        format!("L{}", lit_cnt)
                    } else {
                        ex_cnt += 1;
                        format!("E{}", ex_cnt)
                    };
                    map.insert(label.clone(), canon_label);
                }
                let new_label = map.get(label).unwrap().clone();
                NodeExpr::ExistNode { is_literal: *is_literal, label: new_label }
            },
            NodeExpr::NullNode => NodeExpr::NullNode,
        }
    };

    for op in ops {
        let new_op = match &op.kind {
            GraphOp::Node(node_expr) => GraphOp::Node(canonize_node_expr(node_expr, &mut map)),
            GraphOp::DeleteNode(node_expr) => GraphOp::DeleteNode(canonize_node_expr(node_expr, &mut map)),
            GraphOp::AddEdge { from, to, label } => GraphOp::AddEdge {
                from: canonize_node_expr(from, &mut map),
                to: canonize_node_expr(to, &mut map),
                label: label.clone(),
            },
            GraphOp::EditEdgeReference { from, old_to, new_to, label } => GraphOp::EditEdgeReference {
                from: canonize_node_expr(from, &mut map),
                old_to: canonize_node_expr(old_to, &mut map),
                new_to: canonize_node_expr(new_to, &mut map),
                label: label.clone(),
            },
            GraphOp::DeleteEdge { from, to, label } => GraphOp::DeleteEdge {
                from: canonize_node_expr(from, &mut map),
                to: canonize_node_expr(to, &mut map),
                label: label.clone(),
            },
            GraphOp::AddVariable { to, label } => GraphOp::AddVariable {
                to: canonize_node_expr(to, &mut map),
                label: label.clone(),
            },
            GraphOp::EditVariableReference { old_to, new_to, label } => GraphOp::EditVariableReference {
                old_to: canonize_node_expr(old_to, &mut map),
                new_to: canonize_node_expr(new_to, &mut map),
                label: label.clone(),
            },
            GraphOp::DeleteVariable { to, label } => GraphOp::DeleteVariable {
                to: canonize_node_expr(to, &mut map),
                label: label.clone(),
            },
        };
        result.push(new_op);
    }

    result.sort_by_key(|k| format!("{:?}", k));
    result
}

/// Compute the maximum common set of operations between two sequences.
pub fn unify_operation_graphs(a: &[Op], b: &[Op]) -> Vec<GraphOp> {
    let ca = canonicalize_ops(a);
    let cb = canonicalize_ops(b);

    let mut freq_a: HashMap<GraphOp, usize> = HashMap::new();
    for op in ca {
        *freq_a.entry(op).or_insert(0) += 1;
    }

    let mut freq_b: HashMap<GraphOp, usize> = HashMap::new();
    for op in cb {
        *freq_b.entry(op).or_insert(0) += 1;
    }

    let mut result = Vec::new();
    for (op, count_a) in freq_a.into_iter() {
        if let Some(count_b) = freq_b.get(&op) {
            let n = std::cmp::min(count_a, *count_b);
            for _ in 0..n {
                result.push(op.clone());
            }
        }
    }

    result.sort_by_key(|k| format!("{:?}", k));
    result
}
