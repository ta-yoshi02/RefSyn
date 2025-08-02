use petgraph::graph::{Graph, NodeIndex};
use petgraph::visit::EdgeRef;
use std::collections::{HashMap, HashSet};

use crate::unify_ops::{Op, GraphOp, EdgeExpr, UnificationResult, NodeExpr};

// Operation Graph representation
struct OpGraph<'a> {
    graph: Graph<&'a Op, &'static str>,
    op_map: HashMap<&'a str, NodeIndex>,
}

// Build the operation graph from a sequence of operations
fn build_op_graph<'a>(ops: &'a [Op]) -> OpGraph<'a> {
    let mut graph = Graph::new();
    let mut op_map = HashMap::new();

    for op in ops {
        let node_index = graph.add_node(op);
        op_map.insert(op.id.as_str(), node_index);
    }

    for op in ops {
        if let GraphOp::Edge(edge_expr) = &op.kind {
            if let EdgeExpr::AddEdge { from, to, .. } = edge_expr {
                if let (Some(&from_node), Some(&to_node), Some(&op_node)) = (op_map.get(from.as_str()), op_map.get(to.as_str()), op_map.get(op.id.as_str())) {
                    graph.add_edge(op_node, from_node, "from");
                    graph.add_edge(op_node, to_node, "to");
                }
            }
        }
    }

    OpGraph { graph, op_map }
}

fn count_preserved_edges<'a>(
    g1: &Graph<&'a Op, &'static str>,
    g2: &Graph<&'a Op, &'static str>,
    mapping: &HashMap<NodeIndex, NodeIndex>,
) -> usize {
    let mut preserved_edges = 0;
    for edge in g1.edge_references() {
        if let (Some(&mapped_source), Some(&mapped_target)) =
            (mapping.get(&edge.source()), mapping.get(&edge.target()))
        {
            if g2.edges_connecting(mapped_source, mapped_target)
                .any(|e| e.weight() == edge.weight())
            {
                preserved_edges += 1;
            }
        }
    }
    preserved_edges
}


// Find the maximum common subgraph isomorphism
fn find_mcs<'a>(g1: &OpGraph<'a>, g2: &OpGraph<'a>) -> HashMap<NodeIndex, NodeIndex> {
    let mut best_mapping = HashMap::new();
    let mut best_score = 0;
    let mut current_mapping = HashMap::new();
    let mut g2_used_nodes = HashSet::new();

    let g1_nodes: Vec<NodeIndex> = g1.graph.node_indices().collect();

    backtrack_mcs(
        &g1.graph,
        &g2.graph,
        &g1_nodes,
        0,
        &mut current_mapping,
        &mut g2_used_nodes,
        &mut best_score,
        &mut best_mapping,
    );

    best_mapping
}

fn backtrack_mcs<'a>(
    g1: &Graph<&'a Op, &'static str>,
    g2: &Graph<&'a Op, &'static str>,
    g1_nodes: &[NodeIndex],
    g1_idx: usize,
    current_mapping: &mut HashMap<NodeIndex, NodeIndex>,
    g2_used_nodes: &mut HashSet<NodeIndex>,
    best_score: &mut usize,
    best_mapping: &mut HashMap<NodeIndex, NodeIndex>,
) {
    if g1_idx == g1_nodes.len() {
        let score = count_preserved_edges(g1, g2, current_mapping);
        if score > *best_score {
            *best_score = score;
            *best_mapping = current_mapping.clone();
        } else if score == *best_score && current_mapping.len() > best_mapping.len() {
            *best_mapping = current_mapping.clone();
        }
        return;
    }

    let g1_node = g1_nodes[g1_idx];

    // Option 1: Try to map g1_node to each compatible, unused node in g2
    for g2_node in g2.node_indices() {
        if !g2_used_nodes.contains(&g2_node) && node_labels_match(g1[g1_node], g2[g2_node]) {
            current_mapping.insert(g1_node, g2_node);
            g2_used_nodes.insert(g2_node);

            backtrack_mcs(g1, g2, g1_nodes, g1_idx + 1, current_mapping, g2_used_nodes, best_score, best_mapping);

            current_mapping.remove(&g1_node);
            g2_used_nodes.remove(&g2_node);
        }
    }

    // Option 2: Don't map g1_node and move to the next one
    backtrack_mcs(g1, g2, g1_nodes, g1_idx + 1, current_mapping, g2_used_nodes, best_score, best_mapping);
}


fn node_labels_match(op1: &Op, op2: &Op) -> bool {
    match (&op1.kind, &op2.kind) {
        (GraphOp::Node(n1), GraphOp::Node(n2)) => std::mem::discriminant(n1) == std::mem::discriminant(n2),
        (GraphOp::Edge(e1), GraphOp::Edge(e2)) => std::mem::discriminant(e1) == std::mem::discriminant(e2),
        _ => false,
    }
}

fn attributes_match(op1: &Op, op2: &Op) -> bool {
    match (&op1.kind, &op2.kind) {
        (GraphOp::Node(NodeExpr::AddNode { is_literal: l1, label: lab1, .. }), GraphOp::Node(NodeExpr::AddNode { is_literal: l2, label: lab2, .. })) => {
            if !l1 && !l2 {
                true
            } else {
                lab1 == lab2
            }
        },
        (GraphOp::Node(NodeExpr::ExistNode { id: id1, .. }), GraphOp::Node(NodeExpr::ExistNode { id: id2, .. })) => id1 == id2,
        (GraphOp::Edge(EdgeExpr::AddEdge { label: l1, .. }), GraphOp::Edge(EdgeExpr::AddEdge { label: l2, .. })) => l1 == l2,
        _ => false,
    }
}

pub fn unify_isomorphic_graphs(a: &[Op], b: &[Op]) -> UnificationResult {
    let op_graph_a = build_op_graph(a);
    let op_graph_b = build_op_graph(b);

    if op_graph_a.graph.node_count() != op_graph_b.graph.node_count() {
        panic!("Graphs are not isomorphic: different number of nodes.");
    }

    let mcs_mapping = find_mcs(&op_graph_a, &op_graph_b);

    if mcs_mapping.len() != op_graph_a.graph.node_count() {
        panic!("Graphs are not isomorphic: could not find a full mapping.");
    }

    let mut common_a = Vec::new();
    let mut common_b = Vec::new();
    let mut diff_a = Vec::new();
    let mut diff_b = Vec::new();

    for (node_a_idx, node_b_idx) in &mcs_mapping {
        let op_a = op_graph_a.graph[*node_a_idx];
        let op_b = op_graph_b.graph[*node_b_idx];

        if attributes_match(op_a, op_b) {
            common_a.push(op_a.clone());
            common_b.push(op_b.clone());
        } else {
            diff_a.push(op_a.clone());
            diff_b.push(op_b.clone());
        }
    }
    
    common_a.sort_by_key(|k| format!("{:?}", k));
    common_b.sort_by_key(|k| format!("{:?}", k));
    diff_a.sort_by_key(|k| format!("{:?}", k));
    diff_b.sort_by_key(|k| format!("{:?}", k));

    UnificationResult {
        common_a,
        common_b,
        diff_a,
        diff_b,
    }
}
