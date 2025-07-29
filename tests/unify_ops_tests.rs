use refsyn::unify_ops::{Op, NodeExpr, GraphOp, unify_operation_graphs};

fn append_ops_a() -> Vec<Op> {
    vec![
        Op { id: "a0".to_string(), kind: GraphOp::Node(NodeExpr::AddNode { is_literal: false, label: "Node".to_string() }) },
        Op { id: "a1".to_string(), kind: GraphOp::Node(NodeExpr::AddNode { is_literal: true, label: "0".to_string() }) },
        Op { id: "a2".to_string(), kind: GraphOp::Node(NodeExpr::ExistNode { is_literal: false, label: "Node".to_string() }) },
        Op { id: "a3".to_string(), kind: GraphOp::AddEdge { 
            from: NodeExpr::AddNode { is_literal: false, label: "Node".to_string() }, 
            to: NodeExpr::AddNode { is_literal: true, label: "0".to_string() }, 
            label: "val".to_string() 
        } },
        Op { id: "a4".to_string(), kind: GraphOp::AddEdge { 
            from: NodeExpr::ExistNode { is_literal: false, label: "Node".to_string() }, 
            to: NodeExpr::AddNode { is_literal: false, label: "Node".to_string() }, 
            label: "next".to_string() 
        } },
    ]
}

fn append_ops_b() -> Vec<Op> {
    vec![
        Op { id: "b0".to_string(), kind: GraphOp::Node(NodeExpr::AddNode { is_literal: false, label: "Node".to_string() }) },
        Op { id: "b1".to_string(), kind: GraphOp::Node(NodeExpr::ExistNode { is_literal: false, label: "Node".to_string() })},
        Op { id: "b2".to_string(), kind: GraphOp::AddEdge { 
            from: NodeExpr::ExistNode { is_literal: false, label: "Node".to_string() }, 
            to: NodeExpr::AddNode { is_literal: false, label: "Node".to_string() }, 
            label: "next".to_string() 
        } },
        Op { id: "b3".to_string(), kind: GraphOp::Node(NodeExpr::AddNode { is_literal: true, label: "3".to_string() }) },
        Op { id: "b4".to_string(), kind: GraphOp::AddEdge { 
            from: NodeExpr::AddNode { is_literal: false, label: "Node".to_string() }, 
            to: NodeExpr::AddNode { is_literal: true, label: "3".to_string() }, 
            label: "val".to_string() 
        } },
    ]
}

#[test]
fn test_unify_append() {
    let a = append_ops_a();
    let b = append_ops_b();
    let common = unify_operation_graphs(&a, &b).common;
    let diff = unify_operation_graphs(&a, &b).diff;
    let mut expected_common_a = vec![
        GraphOp::Node(NodeExpr::AddNode { is_literal: true, label: "Node".to_string() }),
        GraphOp::Node(NodeExpr::ExistNode { is_literal: false, label: "Node".to_string() }),
        GraphOp::AddEdge { 
            from: NodeExpr::ExistNode { is_literal: false, label: "Node".to_string() }, 
            to: NodeExpr::AddNode { is_literal: false, label: "Node".to_string() }, 
            label: "next".to_string() 
        },
        GraphOp::AddEdge { 
            from: NodeExpr::AddNode { is_literal: false, label: "Node".to_string() }, 
            to: NodeExpr::AddNode { is_literal: true, label: "0".to_string() }, 
            label: "val".to_string() 
        },
    ];
    let mut expected_common_b = vec![
        GraphOp::Node(NodeExpr::AddNode { is_literal: true, label: "Node".to_string() }),
        GraphOp::Node(NodeExpr::ExistNode { is_literal: false, label: "Node".to_string() }),
        GraphOp::AddEdge { 
            from: NodeExpr::ExistNode { is_literal: false, label: "Node".to_string() }, 
            to: NodeExpr::AddNode { is_literal: false, label: "Node".to_string() }, 
            label: "next".to_string() 
        },
        GraphOp::AddEdge { 
            from: NodeExpr::AddNode { is_literal: false, label: "Node".to_string() }, 
            to: NodeExpr::AddNode { is_literal: true, label: "3".to_string() }, 
            label: "val".to_string() 
        },
    ];
    let mut expected_diff_a = vec![
        GraphOp::Node(NodeExpr::ExistNode { is_literal: false, label: "Node".to_string() }),
        GraphOp::Node(NodeExpr::AddNode { is_literal: true, label: "0".to_string() }),
    ];
    let mut expected_diff_b = vec![
        GraphOp::Node(NodeExpr::ExistNode { is_literal: false, label: "Node".to_string() }),
        GraphOp::Node(NodeExpr::AddNode { is_literal: true, label: "3".to_string() }),
    ];
    expected_common_a.sort_by_key(|k| format!("{:?}", k));
    expected_common_b.sort_by_key(|k| format!("{:?}", k));
    assert_eq!(common, {expected_common_a, expected_common_b});
    assert_eq!(diff, {expected_diff_a, expected_diff_b});
}


fn add2_ops_a() -> Vec<Op> {
    vec![
        Op { id: "a0".to_string(), kind: GraphOp::Node(NodeExpr::AddNode { is_literal: false, label: "Node".to_string() }) },
        Op { id: "a1".to_string(), kind: GraphOp::Node(NodeExpr::AddNode { is_literal: true, label: "0".to_string() }) },
        Op { id: "a2".to_string(), kind: GraphOp::AddEdge { 
            from: NodeExpr::AddNode { is_literal: false, label: "Node".to_string() }, 
            to: NodeExpr::AddNode { is_literal: true, label: "0".to_string() }, 
            label: "val".to_string() 
        } },
        Op { id: "a3".to_string(), kind: GraphOp::Node(NodeExpr::AddNode { is_literal: false, label: "Node".to_string() }) },
        Op { id: "a4".to_string(), kind: GraphOp::Node(NodeExpr::AddNode { is_literal: true, label: "0".to_string() }) },
        Op { id: "a5".to_string(), kind: GraphOp::AddEdge { 
            from: NodeExpr::AddNode { is_literal: false, label: "Node".to_string() }, 
            to: NodeExpr::AddNode { is_literal: true, label: "0".to_string() }, 
            label: "val".to_string() 
        } },
        Op { id: "a6".to_string(), kind: GraphOp::Node(NodeExpr::ExistNode { is_literal: false, label: "Node".to_string() }) },
        Op { id: "a7".to_string(), kind: GraphOp::AddEdge { 
            from: NodeExpr::ExistNode { is_literal: false, label: "Node".to_string() }, 
            to: NodeExpr::AddNode { is_literal: false, label: "Node".to_string() }, 
            label: "next".to_string() 
        } },
        Op { id: "a8".to_string(), kind: GraphOp::AddEdge { 
            from: NodeExpr::AddNode { is_literal: false, label: "Node".to_string() }, 
            to: NodeExpr::AddNode { is_literal: false, label: "Node".to_string() }, 
            label: "next".to_string() 
        } },
    ]
}

fn add2_ops_b() -> Vec<Op> {
    vec![
        Op { id: "b0".to_string(), kind: GraphOp::Node(NodeExpr::AddNode { is_literal: false, label: "Node".to_string() }) },
        Op { id: "b1".to_string(), kind: GraphOp::Node(NodeExpr::AddNode { is_literal: true, label: "3".to_string() }) },
        Op { id: "b2".to_string(), kind: GraphOp::Node(NodeExpr::AddNode { is_literal: false, label: "Node".to_string() }) },
        Op { id: "b3".to_string(), kind: GraphOp::AddEdge { 
            from: NodeExpr::AddNode { is_literal: false, label: "Node".to_string() }, 
            to: NodeExpr::AddNode { is_literal: false, label: "Node".to_string() }, 
            label: "next".to_string() 
        } },
        Op { id: "b4".to_string(), kind: GraphOp::Node(NodeExpr::ExistNode { is_literal: false, label: "Node".to_string() }) },
        Op { id: "b5".to_string(), kind: GraphOp::AddEdge { 
            from: NodeExpr::ExistNode { is_literal: false, label: "Node".to_string() }, 
            to: NodeExpr::AddNode { is_literal: false, label: "Node".to_string() }, 
            label: "next".to_string() 
        } },
        Op { id: "b6".to_string(), kind: GraphOp::Node(NodeExpr::AddNode { is_literal: true, label: "3".to_string() }) },
        Op { id: "b7".to_string(), kind: GraphOp::AddEdge { 
            from: NodeExpr::AddNode { is_literal: false, label: "Node".to_string() }, 
            to: NodeExpr::AddNode { is_literal: true, label: "0".to_string() }, 
            label: "val".to_string() 
        } },
        Op { id: "b8".to_string(), kind: GraphOp::AddEdge { 
            from: NodeExpr::AddNode { is_literal: false, label: "Node".to_string() }, 
            to: NodeExpr::AddNode { is_literal: true, label: "0".to_string() }, 
            label: "val".to_string() 
        } },
    ]
}

#[test]
fn test_unify_add2() {
    let a = add2_ops_a();
    let b = add2_ops_b();
    let common = unify_operation_graphs(&a, &b).common;
    let diff = unify_operation_graphs(&a, &b).diff;
    let mut expected_common_a = vec![
        GraphOp::Node(NodeExpr::AddNode { is_literal: true, label: "Node".to_string() }),
        GraphOp::Node(NodeExpr::ExistNode { is_literal: false, label: "Node".to_string() }),
        GraphOp::AddEdge { 
            from: NodeExpr::ExistNode { is_literal: false, label: "Node".to_string() }, 
            to: NodeExpr::AddNode { is_literal: false, label: "Node".to_string() }, 
            label: "next".to_string() 
        },
        GraphOp::AddEdge { 
            from: NodeExpr::AddNode { is_literal: false, label: "Node".to_string() }, 
            to: NodeExpr::AddNode { is_literal: true, label: "0".to_string() }, 
            label: "val".to_string() 
        },
    ];
    let mut expected_common_b = vec![
        GraphOp::Node(NodeExpr::AddNode { is_literal: true, label: "Node".to_string() }),
        GraphOp::Node(NodeExpr::ExistNode { is_literal: false, label: "Node".to_string() }),
        GraphOp::AddEdge { 
            from: NodeExpr::ExistNode { is_literal: false, label: "Node".to_string() }, 
            to: NodeExpr::AddNode { is_literal: false, label: "Node".to_string() }, 
            label: "next".to_string() 
        },
        GraphOp::AddEdge { 
            from: NodeExpr::AddNode { is_literal: false, label: "Node".to_string() }, 
            to: NodeExpr::AddNode { is_literal: true, label: "3".to_string() }, 
            label: "val".to_string() 
        },
    ];
    let mut expected_diff_a = vec![
        GraphOp::Node(NodeExpr::ExistNode { is_literal: false, label: "Node".to_string() }),
        GraphOp::Node(NodeExpr::AddNode { is_literal: true, label: "0".to_string() }),
    ];
    let mut expected_diff_b = vec![
        GraphOp::Node(NodeExpr::ExistNode { is_literal: false, label: "Node".to_string() }),
        GraphOp::Node(NodeExpr::AddNode { is_literal: true, label: "3".to_string() }),
    ];
    expected_common_a.sort_by_key(|k| format!("{:?}", k));
    expected_common_b.sort_by_key(|k| format!("{:?}", k));
    assert_eq!(common, {expected_common_a, expected_common_b});
    assert_eq!(diff, {expected_diff_a, expected_diff_b});
}
