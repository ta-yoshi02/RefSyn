use crate::ir::{Op, OpId, OpKind};
use petgraph::graph::{DiGraph, NodeIndex};
use std::collections::HashMap;

/// 操作の詳細な構造を表現するためのノード種別
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum DetailedNode {
    /// 操作自体
    Operation(OpId),
    /// ノードID
    NodeId(String),
    /// リテラル値
    Literal(String),
    /// ラベル
    Label(String),
    /// ノードタイプ（is_literalの情報）
    NodeType(bool), // true=literal, false=object
}

/// 操作の詳細な構造を表現するためのエッジ種別
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum DetailedEdge {
    /// 操作が特定のノードIDを参照
    ReferencesNodeId,
    /// 操作が特定のラベルを使用
    UsesLabel,
    /// 操作が特定のリテラル値を使用
    UsesLiteral,
    /// 操作が特定のノードタイプを指定
    SpecifiesNodeType,
    /// エッジの方向性（from/to）
    EdgeDirection(String), // "from", "to", "old_to", "new_to"
}

/// 操作列から詳細な構造グラフを構築
pub fn build_detailed_structure_graph(
    ops: &[Op],
) -> (
    DiGraph<DetailedNode, DetailedEdge>,
    HashMap<DetailedNode, NodeIndex>,
) {
    let mut graph = DiGraph::new();
    let mut node_indices = HashMap::new();

    // すべての操作ノードを追加
    for op in ops {
        let op_node = DetailedNode::Operation(op.id.clone());
        let op_idx = graph.add_node(op_node.clone());
        node_indices.insert(op_node, op_idx);
    }

    // 操作の詳細をノードとして追加し、エッジで接続
    for op in ops {
        let op_node = DetailedNode::Operation(op.id.clone());
        let op_idx = *node_indices.get(&op_node).unwrap();

        match &op.kind {
            OpKind::AddNode {
                id,
                is_literal,
                label,
            } => {
                // ノードIDをグラフに追加
                let node_id_node = DetailedNode::NodeId(id.clone());
                let node_id_idx = *node_indices
                    .entry(node_id_node.clone())
                    .or_insert_with(|| graph.add_node(node_id_node.clone()));
                graph.add_edge(op_idx, node_id_idx, DetailedEdge::ReferencesNodeId);

                // ノードタイプをグラフに追加
                let node_type_node = DetailedNode::NodeType(*is_literal);
                let node_type_idx = *node_indices
                    .entry(node_type_node.clone())
                    .or_insert_with(|| graph.add_node(node_type_node.clone()));
                graph.add_edge(op_idx, node_type_idx, DetailedEdge::SpecifiesNodeType);

                // ラベルをグラフに追加
                if *is_literal {
                    let literal_node = DetailedNode::Literal(label.clone());
                    let literal_idx = *node_indices
                        .entry(literal_node.clone())
                        .or_insert_with(|| graph.add_node(literal_node.clone()));
                    graph.add_edge(op_idx, literal_idx, DetailedEdge::UsesLiteral);
                } else {
                    let label_node = DetailedNode::Label(label.clone());
                    let label_idx = *node_indices
                        .entry(label_node.clone())
                        .or_insert_with(|| graph.add_node(label_node.clone()));
                    graph.add_edge(op_idx, label_idx, DetailedEdge::UsesLabel);
                }
            }

            OpKind::EditNode {
                id,
                is_literal,
                label,
            } => {
                // AddNodeと同様の処理
                let node_id_node = DetailedNode::NodeId(id.clone());
                let node_id_idx = *node_indices
                    .entry(node_id_node.clone())
                    .or_insert_with(|| graph.add_node(node_id_node.clone()));
                graph.add_edge(op_idx, node_id_idx, DetailedEdge::ReferencesNodeId);

                let node_type_node = DetailedNode::NodeType(*is_literal);
                let node_type_idx = *node_indices
                    .entry(node_type_node.clone())
                    .or_insert_with(|| graph.add_node(node_type_node.clone()));
                graph.add_edge(op_idx, node_type_idx, DetailedEdge::SpecifiesNodeType);

                if *is_literal {
                    let literal_node = DetailedNode::Literal(label.clone());
                    let literal_idx = *node_indices
                        .entry(literal_node.clone())
                        .or_insert_with(|| graph.add_node(literal_node.clone()));
                    graph.add_edge(op_idx, literal_idx, DetailedEdge::UsesLiteral);
                } else {
                    let label_node = DetailedNode::Label(label.clone());
                    let label_idx = *node_indices
                        .entry(label_node.clone())
                        .or_insert_with(|| graph.add_node(label_node.clone()));
                    graph.add_edge(op_idx, label_idx, DetailedEdge::UsesLabel);
                }
            }

            OpKind::AddEdge { from, to, label } => {
                // fromノードIDを追加
                let from_node = DetailedNode::NodeId(from.clone());
                let from_idx = *node_indices
                    .entry(from_node.clone())
                    .or_insert_with(|| graph.add_node(from_node.clone()));
                graph.add_edge(
                    op_idx,
                    from_idx,
                    DetailedEdge::EdgeDirection("from".to_string()),
                );

                // toノードIDを追加
                let to_node = DetailedNode::NodeId(to.clone());
                let to_idx = *node_indices
                    .entry(to_node.clone())
                    .or_insert_with(|| graph.add_node(to_node.clone()));
                graph.add_edge(
                    op_idx,
                    to_idx,
                    DetailedEdge::EdgeDirection("to".to_string()),
                );

                // ラベルを追加
                let label_node = DetailedNode::Label(label.clone());
                let label_idx = *node_indices
                    .entry(label_node.clone())
                    .or_insert_with(|| graph.add_node(label_node.clone()));
                graph.add_edge(op_idx, label_idx, DetailedEdge::UsesLabel);
            }

            OpKind::EditEdgeReference {
                from,
                old_to,
                new_to,
                label,
            } => {
                // fromノードID
                let from_node = DetailedNode::NodeId(from.clone());
                let from_idx = *node_indices
                    .entry(from_node.clone())
                    .or_insert_with(|| graph.add_node(from_node.clone()));
                graph.add_edge(
                    op_idx,
                    from_idx,
                    DetailedEdge::EdgeDirection("from".to_string()),
                );

                // old_toノードID
                let old_to_node = DetailedNode::NodeId(old_to.clone());
                let old_to_idx = *node_indices
                    .entry(old_to_node.clone())
                    .or_insert_with(|| graph.add_node(old_to_node.clone()));
                graph.add_edge(
                    op_idx,
                    old_to_idx,
                    DetailedEdge::EdgeDirection("old_to".to_string()),
                );

                // new_toノードID
                let new_to_node = DetailedNode::NodeId(new_to.clone());
                let new_to_idx = *node_indices
                    .entry(new_to_node.clone())
                    .or_insert_with(|| graph.add_node(new_to_node.clone()));
                graph.add_edge(
                    op_idx,
                    new_to_idx,
                    DetailedEdge::EdgeDirection("new_to".to_string()),
                );

                // ラベル
                let label_node = DetailedNode::Label(label.clone());
                let label_idx = *node_indices
                    .entry(label_node.clone())
                    .or_insert_with(|| graph.add_node(label_node.clone()));
                graph.add_edge(op_idx, label_idx, DetailedEdge::UsesLabel);
            }

            OpKind::EditEdgeLabel {
                from,
                to,
                old_label,
                new_label,
            } => {
                // fromノードID
                let from_node = DetailedNode::NodeId(from.clone());
                let from_idx = *node_indices
                    .entry(from_node.clone())
                    .or_insert_with(|| graph.add_node(from_node.clone()));
                graph.add_edge(
                    op_idx,
                    from_idx,
                    DetailedEdge::EdgeDirection("from".to_string()),
                );

                // toノードID
                let to_node = DetailedNode::NodeId(to.clone());
                let to_idx = *node_indices
                    .entry(to_node.clone())
                    .or_insert_with(|| graph.add_node(to_node.clone()));
                graph.add_edge(
                    op_idx,
                    to_idx,
                    DetailedEdge::EdgeDirection("to".to_string()),
                );

                // 古いラベル
                let old_label_node = DetailedNode::Label(old_label.clone());
                let old_label_idx = *node_indices
                    .entry(old_label_node.clone())
                    .or_insert_with(|| graph.add_node(old_label_node.clone()));
                graph.add_edge(op_idx, old_label_idx, DetailedEdge::UsesLabel);

                // 新しいラベル
                let new_label_node = DetailedNode::Label(new_label.clone());
                let new_label_idx = *node_indices
                    .entry(new_label_node.clone())
                    .or_insert_with(|| graph.add_node(new_label_node.clone()));
                graph.add_edge(op_idx, new_label_idx, DetailedEdge::UsesLabel);
            }

            OpKind::DeleteEdge { from, to, label } => {
                // fromノードID
                let from_node = DetailedNode::NodeId(from.clone());
                let from_idx = *node_indices
                    .entry(from_node.clone())
                    .or_insert_with(|| graph.add_node(from_node.clone()));
                graph.add_edge(
                    op_idx,
                    from_idx,
                    DetailedEdge::EdgeDirection("from".to_string()),
                );

                // toノードID
                let to_node = DetailedNode::NodeId(to.clone());
                let to_idx = *node_indices
                    .entry(to_node.clone())
                    .or_insert_with(|| graph.add_node(to_node.clone()));
                graph.add_edge(
                    op_idx,
                    to_idx,
                    DetailedEdge::EdgeDirection("to".to_string()),
                );

                // ラベル
                let label_node = DetailedNode::Label(label.clone());
                let label_idx = *node_indices
                    .entry(label_node.clone())
                    .or_insert_with(|| graph.add_node(label_node.clone()));
                graph.add_edge(op_idx, label_idx, DetailedEdge::UsesLabel);
            }

            OpKind::AddVariable { to, label } => {
                let to_node = DetailedNode::NodeId(to.clone());
                let to_idx = *node_indices
                    .entry(to_node.clone())
                    .or_insert_with(|| graph.add_node(to_node.clone()));
                graph.add_edge(
                    op_idx,
                    to_idx,
                    DetailedEdge::EdgeDirection("to".to_string()),
                );

                let label_node = DetailedNode::Label(label.clone());
                let label_idx = *node_indices
                    .entry(label_node.clone())
                    .or_insert_with(|| graph.add_node(label_node.clone()));
                graph.add_edge(op_idx, label_idx, DetailedEdge::UsesLabel);
            }

            OpKind::EditVariableReference {
                old_to,
                new_to,
                label,
            } => {
                let old_to_node = DetailedNode::NodeId(old_to.clone());
                let old_to_idx = *node_indices
                    .entry(old_to_node.clone())
                    .or_insert_with(|| graph.add_node(old_to_node.clone()));
                graph.add_edge(
                    op_idx,
                    old_to_idx,
                    DetailedEdge::EdgeDirection("old_to".to_string()),
                );

                let new_to_node = DetailedNode::NodeId(new_to.clone());
                let new_to_idx = *node_indices
                    .entry(new_to_node.clone())
                    .or_insert_with(|| graph.add_node(new_to_node.clone()));
                graph.add_edge(
                    op_idx,
                    new_to_idx,
                    DetailedEdge::EdgeDirection("new_to".to_string()),
                );

                let label_node = DetailedNode::Label(label.clone());
                let label_idx = *node_indices
                    .entry(label_node.clone())
                    .or_insert_with(|| graph.add_node(label_node.clone()));
                graph.add_edge(op_idx, label_idx, DetailedEdge::UsesLabel);
            }

            OpKind::EditVariableLabel {
                to,
                old_label,
                new_label,
            } => {
                let to_node = DetailedNode::NodeId(to.clone());
                let to_idx = *node_indices
                    .entry(to_node.clone())
                    .or_insert_with(|| graph.add_node(to_node.clone()));
                graph.add_edge(
                    op_idx,
                    to_idx,
                    DetailedEdge::EdgeDirection("to".to_string()),
                );

                let old_label_node = DetailedNode::Label(old_label.clone());
                let old_label_idx = *node_indices
                    .entry(old_label_node.clone())
                    .or_insert_with(|| graph.add_node(old_label_node.clone()));
                graph.add_edge(op_idx, old_label_idx, DetailedEdge::UsesLabel);

                let new_label_node = DetailedNode::Label(new_label.clone());
                let new_label_idx = *node_indices
                    .entry(new_label_node.clone())
                    .or_insert_with(|| graph.add_node(new_label_node.clone()));
                graph.add_edge(op_idx, new_label_idx, DetailedEdge::UsesLabel);
            }

            OpKind::DeleteVariable { to, label } => {
                let to_node = DetailedNode::NodeId(to.clone());
                let to_idx = *node_indices
                    .entry(to_node.clone())
                    .or_insert_with(|| graph.add_node(to_node.clone()));
                graph.add_edge(
                    op_idx,
                    to_idx,
                    DetailedEdge::EdgeDirection("to".to_string()),
                );

                let label_node = DetailedNode::Label(label.clone());
                let label_idx = *node_indices
                    .entry(label_node.clone())
                    .or_insert_with(|| graph.add_node(label_node.clone()));
                graph.add_edge(op_idx, label_idx, DetailedEdge::UsesLabel);
            }

            OpKind::DeleteNode { id } => {
                let node_id_node = DetailedNode::NodeId(id.clone());
                let node_id_idx = *node_indices
                    .entry(node_id_node.clone())
                    .or_insert_with(|| graph.add_node(node_id_node.clone()));
                graph.add_edge(op_idx, node_id_idx, DetailedEdge::ReferencesNodeId);
            }
        }
    }

    (graph, node_indices)
}

/// 柔軟な構造マッチング（リテラル値とノードIDの差異を許容）
/// ノードIDやリテラル値は具体的な値が異なっていても、同じ構造的位置で使われていればマッチする
pub fn match_graphs_with_flexible_structure(a: &[Op], b: &[Op]) -> bool {
    use petgraph::algo::is_isomorphic_matching;

    let (graph_a, _) = build_detailed_structure_graph(a);
    let (graph_b, _) = build_detailed_structure_graph(b);

    is_isomorphic_matching(
        &graph_a,
        &graph_b,
        |node_a, node_b| {
            match (node_a, node_b) {
                // 操作ノード同士は常にマッチ（種類は接続されたノードで判定）
                (DetailedNode::Operation(_), DetailedNode::Operation(_)) => true,
                // ノードIDは異なっていても良い（ホールとして扱う）
                (DetailedNode::NodeId(_), DetailedNode::NodeId(_)) => true,
                // リテラル値は異なっていても良い（ホールとして扱う）
                (DetailedNode::Literal(_), DetailedNode::Literal(_)) => true,
                // ラベルとノードタイプは一致する必要がある（構造の重要な部分）
                (DetailedNode::Label(a_label), DetailedNode::Label(b_label)) => a_label == b_label,
                (DetailedNode::NodeType(a_type), DetailedNode::NodeType(b_type)) => {
                    a_type == b_type
                }
                // 異なる種類のノードはマッチしない
                _ => false,
            }
        },
        // エッジの種類は一致する必要がある（構造の関係性を保つため）
        |edge_a, edge_b| edge_a == edge_b,
    )
}
