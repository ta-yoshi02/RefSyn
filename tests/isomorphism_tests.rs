use refsyn::isomorphism::unify_isomorphic_graphs;
use refsyn::unify_ops::{Op, GraphOp, NodeExpr, EdgeExpr, UnificationResult};

fn append_ops_a() -> Vec<Op> {
    vec![
        Op { id: "a0".to_string(), kind: GraphOp::Node(NodeExpr::AddNode { is_literal: false, label: "Node".to_string(), id: "__temp1".to_string() }) },
        Op { id: "a1".to_string(), kind: GraphOp::Node(NodeExpr::AddNode { is_literal: true, label: "0".to_string(), id: "__temp2".to_string() }) },
        Op { id: "a2".to_string(), kind: GraphOp::Node(NodeExpr::ExistNode { is_literal: false, label: "Node".to_string(), id: "main-new1".to_string() }) },
        Op { 
            id: "a3".to_string(), kind: GraphOp::Edge(EdgeExpr::AddEdge { 
            from: "a0".to_string(), 
            to: "a1".to_string(), 
            label: "val".to_string() 
        }) },
        Op { 
            id: "a4".to_string(), kind: GraphOp::Edge(EdgeExpr::AddEdge { 
            from: "a2".to_string(),
            to: "a0".to_string(), 
            label: "next".to_string() 
        }) },
    ]
}

fn append_ops_b() -> Vec<Op> {
    vec![
        Op { id: "b0".to_string(), kind: GraphOp::Node(NodeExpr::AddNode { is_literal: false, label: "Node".to_string(), id: "__temp3".to_string() }) },
        Op { id: "b1".to_string(), kind: GraphOp::Node(NodeExpr::ExistNode { is_literal: false, label: "Node".to_string(), id: "__temp1".to_string() })},
        Op { 
            id: "b2".to_string(), kind: GraphOp::Edge(EdgeExpr::AddEdge { 
            from: "b1".to_string(), 
            to: "b0".to_string(), 
            label: "next".to_string() 
        }) },
        Op { id: "b3".to_string(), kind: GraphOp::Node(NodeExpr::AddNode { is_literal: true, label: "3".to_string(), id : "__temp4".to_string() }) },
        Op { 
            id: "b4".to_string(), kind: GraphOp::Edge(EdgeExpr::AddEdge { 
            from: "b0".to_string(), 
            to: "b3".to_string(), 
            label: "val".to_string() 
        }) },
    ]
}

#[test]
fn test_unify_append_iso() {
    let a = append_ops_a();
    let b = append_ops_b();
    let result = unify_isomorphic_graphs(&a, &b);

    let mut expected_common_a = vec![
        Op { id: "a0".to_string(), kind: GraphOp::Node(NodeExpr::AddNode { is_literal: false, label: "Node".to_string(), id: "__temp1".to_string() }) },
        Op { id: "a3".to_string(), kind: GraphOp::Edge(EdgeExpr::AddEdge { 
            from: "a0".to_string(), 
            to: "a1".to_string(), 
            label: "val".to_string() 
        }) },
        Op { id: "a4".to_string(), kind: GraphOp::Edge(EdgeExpr::AddEdge { 
            from: "a2".to_string(),
            to: "a0".to_string(), 
            label: "next".to_string() 
        }) },
    ];
    let mut expected_common_b = vec![
        Op { id: "b0".to_string(), kind: GraphOp::Node(NodeExpr::AddNode { is_literal: false, label: "Node".to_string(), id: "__temp3".to_string() }) },
        Op { 
            id: "b2".to_string(), kind: GraphOp::Edge(EdgeExpr::AddEdge { 
            from: "b1".to_string(), 
            to: "b0".to_string(), 
            label: "next".to_string() 
        }) },
        Op { 
            id: "b4".to_string(), kind: GraphOp::Edge(EdgeExpr::AddEdge { 
            from: "b0".to_string(), 
            to: "b3".to_string(), 
            label: "val".to_string() 
        }) },
    ];
    let mut expected_diff_a = vec![
        Op { id: "a1".to_string(), kind: GraphOp::Node(NodeExpr::AddNode { is_literal: true, label: "0".to_string(), id: "__temp2".to_string() }) },
        Op { id: "a2".to_string(), kind: GraphOp::Node(NodeExpr::ExistNode { is_literal: false, label: "Node".to_string(), id: "main-new1".to_string() }) },
    ];
    let mut expected_diff_b = vec![
        Op { id: "b1".to_string(), kind: GraphOp::Node(NodeExpr::ExistNode { is_literal: false, label: "Node".to_string(), id: "__temp1".to_string() }) },
        Op { id: "b3".to_string(), kind: GraphOp::Node(NodeExpr::AddNode { is_literal: true, label: "3".to_string(), id: "__temp4".to_string() }) },
    ];
    expected_common_a.sort_by_key(|k| format!("{:?}", k));
    expected_common_b.sort_by_key(|k| format!("{:?}", k));
    expected_diff_a.sort_by_key(|k| format!("{:?}", k));
    expected_diff_b.sort_by_key(|k| format!("{:?}", k));

    assert_eq!(result.common_a, expected_common_a);
    assert_eq!(result.common_b, expected_common_b);
    assert_eq!(result.diff_a, expected_diff_a);
    assert_eq!(result.diff_b, expected_diff_b);
}

fn add2_ops_a() -> Vec<Op> {
    vec![
        Op { id: "a0".to_string(), kind: GraphOp::Node(NodeExpr::AddNode { is_literal: false, label: "Node".to_string(), id: "__temp1".to_string() }) },
        Op { id: "a1".to_string(), kind: GraphOp::Node(NodeExpr::AddNode { is_literal: true, label: "0".to_string(), id: "__temp2".to_string() }) },
        Op { id: "a2".to_string(), kind: GraphOp::Edge(EdgeExpr::AddEdge { 
            from: "a0".to_string(), 
            to: "a1".to_string(), 
            label: "val".to_string() 
        }) },
        Op { id: "a3".to_string(), kind: GraphOp::Node(NodeExpr::AddNode { is_literal: false, label: "Node".to_string(), id: "__temp3".to_string() }) },
        Op { id: "a4".to_string(), kind: GraphOp::Node(NodeExpr::AddNode { is_literal: true, label: "0".to_string(), id: "__temp4".to_string() }) },
        Op { id: "a5".to_string(), kind: GraphOp::Edge(EdgeExpr::AddEdge { 
            from: "a3".to_string(), 
            to: "a4".to_string(), 
            label: "val".to_string() 
        }) },
        Op { id: "a6".to_string(), kind: GraphOp::Node(NodeExpr::ExistNode { is_literal: false, label: "Node".to_string(), id: "main-new1".to_string() }) },
        Op { id: "a7".to_string(), kind: GraphOp::Edge(EdgeExpr::AddEdge { 
            from: "a6".to_string(), 
            to: "a0".to_string(), 
            label: "next".to_string() 
        }) },
        Op { id: "a8".to_string(), kind: GraphOp::Edge(EdgeExpr::AddEdge { 
            from: "a0".to_string(), 
            to: "a3".to_string(), 
            label: "next".to_string() 
        }) },
    ]
}

fn add2_ops_b() -> Vec<Op> {
    vec![
        Op { id: "b0".to_string(), kind: GraphOp::Node(NodeExpr::AddNode { is_literal: false, label: "Node".to_string(), id: "__temp5".to_string() }) },
        Op { id: "b1".to_string(), kind: GraphOp::Node(NodeExpr::AddNode { is_literal: true, label: "3".to_string(), id: "__temp6".to_string() }) },
        Op { id: "b2".to_string(), kind: GraphOp::Node(NodeExpr::AddNode { is_literal: false, label: "Node".to_string(), id: "__temp7".to_string() }) },
        Op { id: "b3".to_string(), kind: GraphOp::Edge(EdgeExpr::AddEdge { 
            from: "b0".to_string(), 
            to: "b2".to_string(), 
            label: "next".to_string() 
        }) },
        Op { id: "b4".to_string(), kind: GraphOp::Node(NodeExpr::ExistNode { is_literal: false, label: "Node".to_string(), id: "__temp3".to_string() }) },
        Op { id: "b5".to_string(), kind: GraphOp::Edge(EdgeExpr::AddEdge { 
            from: "b4".to_string(), 
            to: "b0".to_string(), 
            label: "next".to_string() 
        }) },
        Op { id: "b6".to_string(), kind: GraphOp::Node(NodeExpr::AddNode { is_literal: true, label: "3".to_string(), id: "__temp8".to_string() }) },
        Op { id: "b7".to_string(), kind: GraphOp::Edge(EdgeExpr::AddEdge { 
            from: "b0".to_string(), 
            to: "b1".to_string(), 
            label: "val".to_string() 
        }) },
        Op { id: "b8".to_string(), kind: GraphOp::Edge(EdgeExpr::AddEdge { 
            from: "b2".to_string(), 
            to: "b6".to_string(), 
            label: "val".to_string() 
        }) },
    ]
}

#[test]
fn test_unify_add2_iso() {
    let a = add2_ops_a();
    let b = add2_ops_b();
    let result = unify_isomorphic_graphs(&a, &b);

    let mut expected_common_a = vec![
        Op { id: "a0".to_string(), kind: GraphOp::Node(NodeExpr::AddNode { is_literal: false, label: "Node".to_string(), id: "__temp1".to_string() }) },
        Op { id: "a2".to_string(), kind: GraphOp::Edge(EdgeExpr::AddEdge { 
            from: "a0".to_string(), 
            to: "a1".to_string(), 
            label: "val".to_string() 
        }) },
        Op { id: "a3".to_string(), kind: GraphOp::Node(NodeExpr::AddNode { is_literal: false, label: "Node".to_string(), id: "__temp3".to_string() }) },
        Op { id: "a5".to_string(), kind: GraphOp::Edge(EdgeExpr::AddEdge { 
            from: "a3".to_string(), 
            to: "a4".to_string(), 
            label: "val".to_string() 
        }) },
        Op { id: "a7".to_string(), kind: GraphOp::Edge(EdgeExpr::AddEdge { 
            from: "a6".to_string(), 
            to: "a0".to_string(), 
            label: "next".to_string() 
        }) },
        Op { id: "a8".to_string(), kind: GraphOp::Edge(EdgeExpr::AddEdge { 
            from: "a0".to_string(), 
            to: "a3".to_string(), 
            label: "next".to_string() 
        }) },
    ];
    let mut expected_common_b = vec![
        Op { id: "b0".to_string(), kind: GraphOp::Node(NodeExpr::AddNode { is_literal: false, label: "Node".to_string(), id: "__temp5".to_string() }) },
        Op { id: "b2".to_string(), kind: GraphOp::Node(NodeExpr::AddNode { is_literal: false, label: "Node".to_string(), id: "__temp7".to_string() }) },
        Op { id: "b3".to_string(), kind: GraphOp::Edge(EdgeExpr::AddEdge { 
            from: "b0".to_string(), 
            to: "b2".to_string(), 
            label: "next".to_string() 
        }) },
        Op { id: "b5".to_string(), kind: GraphOp::Edge(EdgeExpr::AddEdge { 
            from: "b4".to_string(), 
            to: "b0".to_string(), 
            label: "next".to_string() 
        }) },
        Op { id: "b7".to_string(), kind: GraphOp::Edge(EdgeExpr::AddEdge { 
            from: "b0".to_string(), 
            to: "b1".to_string(), 
            label: "val".to_string() 
        }) },
        Op { id: "b8".to_string(), kind: GraphOp::Edge(EdgeExpr::AddEdge { 
            from: "b2".to_string(), 
            to: "b6".to_string(), 
            label: "val".to_string() 
        }) },
    ];
    let mut expected_diff_a = vec![
        Op { id: "a1".to_string(), kind: GraphOp::Node(NodeExpr::AddNode { is_literal: true, label: "0".to_string(), id: "__temp2".to_string() }) },
        Op { id: "a4".to_string(), kind: GraphOp::Node(NodeExpr::AddNode { is_literal: true, label: "0".to_string(), id: "__temp4".to_string() }) },
        Op { id: "a6".to_string(), kind: GraphOp::Node(NodeExpr::ExistNode { is_literal: false, label: "Node".to_string(), id: "main-new1".to_string() }) },
    ];
    let mut expected_diff_b = vec![
        Op { id: "b1".to_string(), kind: GraphOp::Node(NodeExpr::AddNode { is_literal: true, label: "3".to_string(), id: "__temp6".to_string() }) },
        Op { id: "b4".to_string(), kind: GraphOp::Node(NodeExpr::ExistNode { is_literal: false, label: "Node".to_string(), id: "__temp3".to_string() }) },
        Op { id: "b6".to_string(), kind: GraphOp::Node(NodeExpr::AddNode { is_literal: true, label: "3".to_string(), id: "__temp8".to_string() }) },
    ];

    expected_common_a.sort_by_key(|k| format!("{:?}", k));
    expected_common_b.sort_by_key(|k| format!("{:?}", k));
    expected_diff_a.sort_by_key(|k| format!("{:?}", k));
    expected_diff_b.sort_by_key(|k| format!("{:?}", k));

    assert_eq!(result.common_a, expected_common_a);
    assert_eq!(result.common_b, expected_common_b);
    assert_eq!(result.diff_a, expected_diff_a);
    assert_eq!(result.diff_b, expected_diff_b);
}
