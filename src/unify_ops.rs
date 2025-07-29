use std::collections::{HashMap, BTreeSet};
use serde::{Deserialize, Serialize};

/// 操作ID（各操作を一意に識別）
pub type OpNum = String;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub enum NodeExpr {
    AddNode {is_literal: bool, label: String},
    ExistNode {is_literal: bool, label: String},
    DeleteNode { id: OpNum },
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
    pub common_a: Vec<GraphOp>,
    pub common_b: Vec<GraphOp>,
    pub diff_a: Vec<GraphOp>,
    pub diff_b: Vec<GraphOp>,
}

/// Canonicalize operations by renaming IDs.
/// (This implementation is a placeholder and will be properly implemented in the next step)
pub fn canonicalize_ops(ops: &[Op]) -> Vec<GraphOp> {
    // TODO: Implement the logic to rename node IDs to a canonical form.
    // For now, we just extract the GraphOp kinds, which is not correct
    // but allows the tests for the data structures to be written.
    ops.iter().map(|op| op.kind.clone()).collect()
}

/// Compute the maximum common set of operations between two sequences.
/// (This implementation is a placeholder and will be properly implemented in the next step)
pub fn unify_operation_graphs(a: &[Op], b: &[Op]) -> UnificationResult {
    // TODO: Implement the unification logic based on canonicalized operations.
    UnificationResult {
        common_a: vec![],
        common_b: vec![],
        diff_a: vec![],
        diff_b: vec![],
    }
}
