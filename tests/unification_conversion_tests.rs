use refsyn::analyze_operations_with_unification;
use refsyn::models::VisGraph;
use serde_json::json;

fn make_min_vis_graph() -> VisGraph {
    // 空でも良いが、将来の拡張で安全のため最小構成を返す
    VisGraph {
        nodes: vec![],
        edges: vec![],
    }
}

#[test]
fn test_convert_and_unify_with_exist_nodes_and_edges() {
    // A: __nA(Node) --val--> "0" ; main --next--> __nA
    let ops_a = vec![
        json!({"editType": "addNode", "id": "__nA", "label": "Node", "isLiteral": false}),
        json!({"editType": "addNode", "id": "__litA0", "label": "0", "isLiteral": true}),
        json!({"editType": "addEdge", "from": "__nA", "to": "__litA0", "label": "val"}),
        json!({"editType": "addEdge", "from": "main", "to": "__nA", "label": "next"}),
    ];

    // B: __nB(Node) --val--> "0" ; root --next--> __nB
    let ops_b = vec![
        json!({"editType": "addNode", "id": "__nB", "label": "Node", "isLiteral": false}),
        json!({"editType": "addNode", "id": "__litB0", "label": "0", "isLiteral": true}),
        json!({"editType": "addEdge", "from": "__nB", "to": "__litB0", "label": "val"}),
        json!({"editType": "addEdge", "from": "root", "to": "__nB", "label": "next"}),
    ];

    let vis = make_min_vis_graph();
    let result = analyze_operations_with_unification(&vis, &ops_a, &ops_b).expect("analysis");
    let uni = result.unification_result;

    // 期待: 共通に含まれるもの（厳密）
    // - AddNode(Node, is_literal=false)
    // - AddNode("0", is_literal=true)
    // - AddEdge(val)
    // - AddEdge(next)  ※ from/to は ExistNode/Node のマッピングで一致

    // helper: カウント関数
    let mut add_node_nonlit = 0;
    let mut add_node_lit0 = 0;
    let mut add_edge_val = 0;
    let mut add_edge_next = 0;

    for op in &uni.common_a {
        match &op.kind {
            refsyn::unify_ops::GraphOp::Node(refsyn::unify_ops::NodeExpr::AddNode {
                is_literal,
                label,
                ..
            }) => {
                if !*is_literal && label == "Node" {
                    add_node_nonlit += 1;
                }
                if *is_literal && label == "0" {
                    add_node_lit0 += 1;
                }
            }
            refsyn::unify_ops::GraphOp::Edge(refsyn::unify_ops::EdgeExpr::AddEdge {
                label,
                ..
            }) => {
                if label == "val" {
                    add_edge_val += 1;
                }
                if label == "next" {
                    add_edge_next += 1;
                }
            }
            _ => {}
        }
    }

    assert_eq!(
        add_node_nonlit, 1,
        "non-literal Node add should be common once"
    );
    assert_eq!(add_node_lit0, 1, "literal '0' add should be common once");
    assert_eq!(add_edge_val, 1, "val edge should be common once");
    assert_eq!(add_edge_next, 1, "next edge should be common once");

    // 総数も厳密に 4 に収まっていること
    assert_eq!(uni.common_a.len(), 4, "expected exactly 4 common ops in A");
    assert_eq!(uni.common_b.len(), 4, "expected exactly 4 common ops in B");
}
