use refsyn::unify_ops::{unify_operation_graphs, EdgeExpr, GraphOp, NodeExpr, Op, VarOp};

fn append_ops_a() -> Vec<Op> {
    vec![
        Op {
            id: "a0".to_string(),
            kind: GraphOp::Node(NodeExpr::AddNode {
                is_literal: false,
                label: "Node".to_string(),
                id: "__temp1".to_string(),
            }),
        },
        Op {
            id: "a1".to_string(),
            kind: GraphOp::Node(NodeExpr::AddNode {
                is_literal: true,
                label: "0".to_string(),
                id: "__temp2".to_string(),
            }),
        },
        Op {
            id: "a2".to_string(),
            kind: GraphOp::Node(NodeExpr::ExistNode {
                is_literal: false,
                label: "Node".to_string(),
                id: "main-new1".to_string(),
            }),
        },
        Op {
            id: "a3".to_string(),
            kind: GraphOp::Edge(EdgeExpr::AddEdge {
                from: "a0".to_string(),
                to: "a1".to_string(),
                label: "val".to_string(),
            }),
        },
        Op {
            id: "a4".to_string(),
            kind: GraphOp::Edge(EdgeExpr::AddEdge {
                from: "a2".to_string(),
                to: "a0".to_string(),
                label: "next".to_string(),
            }),
        },
    ]
}

fn append_ops_b() -> Vec<Op> {
    vec![
        Op {
            id: "b0".to_string(),
            kind: GraphOp::Node(NodeExpr::AddNode {
                is_literal: false,
                label: "Node".to_string(),
                id: "__temp3".to_string(),
            }),
        },
        Op {
            id: "b1".to_string(),
            kind: GraphOp::Node(NodeExpr::ExistNode {
                is_literal: false,
                label: "Node".to_string(),
                id: "__temp1".to_string(),
            }),
        },
        Op {
            id: "b2".to_string(),
            kind: GraphOp::Edge(EdgeExpr::AddEdge {
                from: "b1".to_string(),
                to: "b0".to_string(),
                label: "next".to_string(),
            }),
        },
        Op {
            id: "b3".to_string(),
            kind: GraphOp::Node(NodeExpr::AddNode {
                is_literal: true,
                label: "3".to_string(),
                id: "__temp4".to_string(),
            }),
        },
        Op {
            id: "b4".to_string(),
            kind: GraphOp::Edge(EdgeExpr::AddEdge {
                from: "b0".to_string(),
                to: "b3".to_string(),
                label: "val".to_string(),
            }),
        },
    ]
}

#[test]
fn test_unify_append() {
    let a = append_ops_a();
    let b = append_ops_b();
    let result = unify_operation_graphs(&a, &b);

    let mut expected_common_a = vec![
        Op {
            id: "a0".to_string(),
            kind: GraphOp::Node(NodeExpr::AddNode {
                is_literal: false,
                label: "Node".to_string(),
                id: "__temp1".to_string(),
            }),
        },
        Op {
            id: "a3".to_string(),
            kind: GraphOp::Edge(EdgeExpr::AddEdge {
                from: "a0".to_string(),
                to: "a1".to_string(),
                label: "val".to_string(),
            }),
        },
        Op {
            id: "a4".to_string(),
            kind: GraphOp::Edge(EdgeExpr::AddEdge {
                from: "a2".to_string(),
                to: "a0".to_string(),
                label: "next".to_string(),
            }),
        },
    ];
    let mut expected_common_b = vec![
        Op {
            id: "b0".to_string(),
            kind: GraphOp::Node(NodeExpr::AddNode {
                is_literal: false,
                label: "Node".to_string(),
                id: "__temp3".to_string(),
            }),
        },
        Op {
            id: "b2".to_string(),
            kind: GraphOp::Edge(EdgeExpr::AddEdge {
                from: "b1".to_string(),
                to: "b0".to_string(),
                label: "next".to_string(),
            }),
        },
        Op {
            id: "b4".to_string(),
            kind: GraphOp::Edge(EdgeExpr::AddEdge {
                from: "b0".to_string(),
                to: "b3".to_string(),
                label: "val".to_string(),
            }),
        },
    ];
    let mut expected_diff_a = vec![
        Op {
            id: "a1".to_string(),
            kind: GraphOp::Node(NodeExpr::AddNode {
                is_literal: true,
                label: "0".to_string(),
                id: "__temp2".to_string(),
            }),
        },
        Op {
            id: "a2".to_string(),
            kind: GraphOp::Node(NodeExpr::ExistNode {
                is_literal: false,
                label: "Node".to_string(),
                id: "main-new1".to_string(),
            }),
        },
    ];
    let mut expected_diff_b = vec![
        Op {
            id: "b1".to_string(),
            kind: GraphOp::Node(NodeExpr::ExistNode {
                is_literal: false,
                label: "Node".to_string(),
                id: "__temp1".to_string(),
            }),
        },
        Op {
            id: "b3".to_string(),
            kind: GraphOp::Node(NodeExpr::AddNode {
                is_literal: true,
                label: "3".to_string(),
                id: "__temp4".to_string(),
            }),
        },
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
        Op {
            id: "a0".to_string(),
            kind: GraphOp::Node(NodeExpr::AddNode {
                is_literal: false,
                label: "Node".to_string(),
                id: "__temp1".to_string(),
            }),
        },
        Op {
            id: "a1".to_string(),
            kind: GraphOp::Node(NodeExpr::AddNode {
                is_literal: true,
                label: "0".to_string(),
                id: "__temp2".to_string(),
            }),
        },
        Op {
            id: "a2".to_string(),
            kind: GraphOp::Edge(EdgeExpr::AddEdge {
                from: "a0".to_string(),
                to: "a1".to_string(),
                label: "val".to_string(),
            }),
        },
        Op {
            id: "a3".to_string(),
            kind: GraphOp::Node(NodeExpr::AddNode {
                is_literal: false,
                label: "Node".to_string(),
                id: "__temp3".to_string(),
            }),
        },
        Op {
            id: "a4".to_string(),
            kind: GraphOp::Node(NodeExpr::AddNode {
                is_literal: true,
                label: "0".to_string(),
                id: "__temp4".to_string(),
            }),
        },
        Op {
            id: "a5".to_string(),
            kind: GraphOp::Edge(EdgeExpr::AddEdge {
                from: "a3".to_string(),
                to: "a4".to_string(),
                label: "val".to_string(),
            }),
        },
        Op {
            id: "a6".to_string(),
            kind: GraphOp::Node(NodeExpr::ExistNode {
                is_literal: false,
                label: "Node".to_string(),
                id: "main-new1".to_string(),
            }),
        },
        Op {
            id: "a7".to_string(),
            kind: GraphOp::Edge(EdgeExpr::AddEdge {
                from: "a6".to_string(),
                to: "a0".to_string(),
                label: "next".to_string(),
            }),
        },
        Op {
            id: "a8".to_string(),
            kind: GraphOp::Edge(EdgeExpr::AddEdge {
                from: "a0".to_string(),
                to: "a3".to_string(),
                label: "next".to_string(),
            }),
        },
    ]
}

fn add2_ops_b() -> Vec<Op> {
    vec![
        Op {
            id: "b0".to_string(),
            kind: GraphOp::Node(NodeExpr::AddNode {
                is_literal: false,
                label: "Node".to_string(),
                id: "__temp5".to_string(),
            }),
        },
        Op {
            id: "b1".to_string(),
            kind: GraphOp::Node(NodeExpr::AddNode {
                is_literal: true,
                label: "3".to_string(),
                id: "__temp6".to_string(),
            }),
        },
        Op {
            id: "b2".to_string(),
            kind: GraphOp::Node(NodeExpr::AddNode {
                is_literal: false,
                label: "Node".to_string(),
                id: "__temp7".to_string(),
            }),
        },
        Op {
            id: "b3".to_string(),
            kind: GraphOp::Edge(EdgeExpr::AddEdge {
                from: "b0".to_string(),
                to: "b2".to_string(),
                label: "next".to_string(),
            }),
        },
        Op {
            id: "b4".to_string(),
            kind: GraphOp::Node(NodeExpr::ExistNode {
                is_literal: false,
                label: "Node".to_string(),
                id: "__temp3".to_string(),
            }),
        },
        Op {
            id: "b5".to_string(),
            kind: GraphOp::Edge(EdgeExpr::AddEdge {
                from: "b4".to_string(),
                to: "b0".to_string(),
                label: "next".to_string(),
            }),
        },
        Op {
            id: "b6".to_string(),
            kind: GraphOp::Node(NodeExpr::AddNode {
                is_literal: true,
                label: "3".to_string(),
                id: "__temp8".to_string(),
            }),
        },
        Op {
            id: "b7".to_string(),
            kind: GraphOp::Edge(EdgeExpr::AddEdge {
                from: "b0".to_string(),
                to: "b1".to_string(),
                label: "val".to_string(),
            }),
        },
        Op {
            id: "b8".to_string(),
            kind: GraphOp::Edge(EdgeExpr::AddEdge {
                from: "b2".to_string(),
                to: "b6".to_string(),
                label: "val".to_string(),
            }),
        },
    ]
}

#[test]
fn test_unify_add2() {
    let a = add2_ops_a();
    let b = add2_ops_b();
    let result = unify_operation_graphs(&a, &b);

    let mut expected_common_a = vec![
        Op {
            id: "a0".to_string(),
            kind: GraphOp::Node(NodeExpr::AddNode {
                is_literal: false,
                label: "Node".to_string(),
                id: "__temp1".to_string(),
            }),
        },
        Op {
            id: "a2".to_string(),
            kind: GraphOp::Edge(EdgeExpr::AddEdge {
                from: "a0".to_string(),
                to: "a1".to_string(),
                label: "val".to_string(),
            }),
        },
        Op {
            id: "a3".to_string(),
            kind: GraphOp::Node(NodeExpr::AddNode {
                is_literal: false,
                label: "Node".to_string(),
                id: "__temp3".to_string(),
            }),
        },
        Op {
            id: "a5".to_string(),
            kind: GraphOp::Edge(EdgeExpr::AddEdge {
                from: "a3".to_string(),
                to: "a4".to_string(),
                label: "val".to_string(),
            }),
        },
        Op {
            id: "a7".to_string(),
            kind: GraphOp::Edge(EdgeExpr::AddEdge {
                from: "a6".to_string(),
                to: "a0".to_string(),
                label: "next".to_string(),
            }),
        },
        Op {
            id: "a8".to_string(),
            kind: GraphOp::Edge(EdgeExpr::AddEdge {
                from: "a0".to_string(),
                to: "a3".to_string(),
                label: "next".to_string(),
            }),
        },
    ];
    let mut expected_common_b = vec![
        Op {
            id: "b0".to_string(),
            kind: GraphOp::Node(NodeExpr::AddNode {
                is_literal: false,
                label: "Node".to_string(),
                id: "__temp5".to_string(),
            }),
        },
        Op {
            id: "b2".to_string(),
            kind: GraphOp::Node(NodeExpr::AddNode {
                is_literal: false,
                label: "Node".to_string(),
                id: "__temp7".to_string(),
            }),
        },
        Op {
            id: "b3".to_string(),
            kind: GraphOp::Edge(EdgeExpr::AddEdge {
                from: "b0".to_string(),
                to: "b2".to_string(),
                label: "next".to_string(),
            }),
        },
        Op {
            id: "b5".to_string(),
            kind: GraphOp::Edge(EdgeExpr::AddEdge {
                from: "b4".to_string(),
                to: "b0".to_string(),
                label: "next".to_string(),
            }),
        },
        Op {
            id: "b7".to_string(),
            kind: GraphOp::Edge(EdgeExpr::AddEdge {
                from: "b0".to_string(),
                to: "b1".to_string(),
                label: "val".to_string(),
            }),
        },
        Op {
            id: "b8".to_string(),
            kind: GraphOp::Edge(EdgeExpr::AddEdge {
                from: "b2".to_string(),
                to: "b6".to_string(),
                label: "val".to_string(),
            }),
        },
    ];
    let mut expected_diff_a = vec![
        Op {
            id: "a1".to_string(),
            kind: GraphOp::Node(NodeExpr::AddNode {
                is_literal: true,
                label: "0".to_string(),
                id: "__temp2".to_string(),
            }),
        },
        Op {
            id: "a4".to_string(),
            kind: GraphOp::Node(NodeExpr::AddNode {
                is_literal: true,
                label: "0".to_string(),
                id: "__temp4".to_string(),
            }),
        },
        Op {
            id: "a6".to_string(),
            kind: GraphOp::Node(NodeExpr::ExistNode {
                is_literal: false,
                label: "Node".to_string(),
                id: "main-new1".to_string(),
            }),
        },
    ];
    let mut expected_diff_b = vec![
        Op {
            id: "b1".to_string(),
            kind: GraphOp::Node(NodeExpr::AddNode {
                is_literal: true,
                label: "3".to_string(),
                id: "__temp6".to_string(),
            }),
        },
        Op {
            id: "b4".to_string(),
            kind: GraphOp::Node(NodeExpr::ExistNode {
                is_literal: false,
                label: "Node".to_string(),
                id: "__temp3".to_string(),
            }),
        },
        Op {
            id: "b6".to_string(),
            kind: GraphOp::Node(NodeExpr::AddNode {
                is_literal: true,
                label: "3".to_string(),
                id: "__temp8".to_string(),
            }),
        },
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

fn prepend_ops_a() -> Vec<Op> {
    vec![
        Op {
            id: "a0".to_string(),
            kind: GraphOp::Node(NodeExpr::AddNode {
                is_literal: false,
                label: "Node".to_string(),
                id: "__temp1".to_string(),
            }),
        },
        Op {
            id: "a1".to_string(),
            kind: GraphOp::Node(NodeExpr::AddNode {
                is_literal: true,
                label: "0".to_string(),
                id: "__temp2".to_string(),
            }),
        },
        Op {
            id: "a2".to_string(),
            kind: GraphOp::Edge(EdgeExpr::AddEdge {
                from: "a0".to_string(),
                to: "a1".to_string(),
                label: "val".to_string(),
            }),
        },
        Op {
            id: "a3".to_string(),
            kind: GraphOp::Node(NodeExpr::ExistNode {
                is_literal: false,
                label: "Node".to_string(),
                id: "main-new1".to_string(),
            }),
        },
        Op {
            id: "a4".to_string(),
            kind: GraphOp::Edge(EdgeExpr::AddEdge {
                from: "a0".to_string(),
                to: "a3".to_string(),
                label: "next".to_string(),
            }),
        },
        Op {
            id: "a5".to_string(),
            kind: GraphOp::Variable(VarOp::AddVariable {
                to: "a0".to_string(),
                label: "return".to_string(),
            }),
        },
    ]
}

fn prepend_ops_b() -> Vec<Op> {
    vec![
        Op {
            id: "b0".to_string(),
            kind: GraphOp::Node(NodeExpr::AddNode {
                is_literal: true,
                label: "3".to_string(),
                id: "__temp3".to_string(),
            }),
        },
        Op {
            id: "b1".to_string(),
            kind: GraphOp::Node(NodeExpr::AddNode {
                is_literal: false,
                label: "Node".to_string(),
                id: "__temp4".to_string(),
            }),
        },
        Op {
            id: "b2".to_string(),
            kind: GraphOp::Variable(VarOp::AddVariable {
                to: "b1".to_string(),
                label: "return".to_string(),
            }),
        },
        Op {
            id: "b3".to_string(),
            kind: GraphOp::Node(NodeExpr::ExistNode {
                is_literal: false,
                label: "Node".to_string(),
                id: "__temp1".to_string(),
            }),
        },
        Op {
            id: "b4".to_string(),
            kind: GraphOp::Edge(EdgeExpr::AddEdge {
                from: "b1".to_string(),
                to: "b3".to_string(),
                label: "next".to_string(),
            }),
        },
        Op {
            id: "b5".to_string(),
            kind: GraphOp::Edge(EdgeExpr::AddEdge {
                from: "b1".to_string(),
                to: "b0".to_string(),
                label: "val".to_string(),
            }),
        },
    ]
}

#[test]
fn test_unify_prepend() {
    let a = prepend_ops_a();
    let b = prepend_ops_b();
    let result = unify_operation_graphs(&a, &b);
    let mut expected_common_a = vec![
        Op {
            id: "a0".to_string(),
            kind: GraphOp::Node(NodeExpr::AddNode {
                is_literal: false,
                label: "Node".to_string(),
                id: "__temp1".to_string(),
            }),
        },
        Op {
            id: "a2".to_string(),
            kind: GraphOp::Edge(EdgeExpr::AddEdge {
                from: "a0".to_string(),
                to: "a1".to_string(),
                label: "val".to_string(),
            }),
        },
        Op {
            id: "a4".to_string(),
            kind: GraphOp::Edge(EdgeExpr::AddEdge {
                from: "a0".to_string(),
                to: "a3".to_string(),
                label: "next".to_string(),
            }),
        },
        Op {
            id: "a5".to_string(),
            kind: GraphOp::Variable(VarOp::AddVariable {
                to: "a0".to_string(),
                label: "return".to_string(),
            }),
        },
    ];
    let mut expected_common_b = vec![
        Op {
            id: "b1".to_string(),
            kind: GraphOp::Node(NodeExpr::AddNode {
                is_literal: false,
                label: "Node".to_string(),
                id: "__temp4".to_string(),
            }),
        },
        Op {
            id: "b2".to_string(),
            kind: GraphOp::Variable(VarOp::AddVariable {
                to: "b1".to_string(),
                label: "return".to_string(),
            }),
        },
        Op {
            id: "b4".to_string(),
            kind: GraphOp::Edge(EdgeExpr::AddEdge {
                from: "b1".to_string(),
                to: "b3".to_string(),
                label: "next".to_string(),
            }),
        },
        Op {
            id: "b5".to_string(),
            kind: GraphOp::Edge(EdgeExpr::AddEdge {
                from: "b1".to_string(),
                to: "b0".to_string(),
                label: "val".to_string(),
            }),
        },
    ];
    let mut expected_diff_a = vec![
        Op {
            id: "a1".to_string(),
            kind: GraphOp::Node(NodeExpr::AddNode {
                is_literal: true,
                label: "0".to_string(),
                id: "__temp2".to_string(),
            }),
        },
        Op {
            id: "a3".to_string(),
            kind: GraphOp::Node(NodeExpr::ExistNode {
                is_literal: false,
                label: "Node".to_string(),
                id: "main-new1".to_string(),
            }),
        },
    ];
    let mut expected_diff_b = vec![
        Op {
            id: "b0".to_string(),
            kind: GraphOp::Node(NodeExpr::AddNode {
                is_literal: true,
                label: "3".to_string(),
                id: "__temp3".to_string(),
            }),
        },
        Op {
            id: "b3".to_string(),
            kind: GraphOp::Node(NodeExpr::ExistNode {
                is_literal: false,
                label: "Node".to_string(),
                id: "__temp1".to_string(),
            }),
        },
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

fn remove_last_ops_a() -> Vec<Op> {
    vec![
        Op {
            id: "a_0".to_string(),
            kind: GraphOp::Node(NodeExpr::ExistNode {
                is_literal: false,
                label: "Node".to_string(),
                id: "main-new2".to_string(),
            }),
        },
        Op {
            id: "a_1".to_string(),
            kind: GraphOp::Node(NodeExpr::NullNode {}),
        },
        Op {
            id: "a_2".to_string(),
            kind: GraphOp::Edge(EdgeExpr::EditEdgeReference {
                from: "a_0".to_string(),
                old_to: None,
                new_to: "a_1".to_string(),
                label: "next".to_string(),
            }),
        },
    ]
}

fn remove_last_ops_b() -> Vec<Op> {
    vec![
        Op {
            id: "b_0".to_string(),
            kind: GraphOp::Node(NodeExpr::NullNode {}),
        },
        Op {
            id: "b_1".to_string(),
            kind: GraphOp::Node(NodeExpr::ExistNode {
                is_literal: false,
                label: "Node".to_string(),
                id: "main-new1".to_string(),
            }),
        },
        Op {
            id: "b_2".to_string(),
            kind: GraphOp::Edge(EdgeExpr::EditEdgeReference {
                from: "b_1".to_string(),
                old_to: None,
                new_to: "b_0".to_string(),
                label: "next".to_string(),
            }),
        },
    ]
}

#[test]
fn test_unify_remove_last() {
    let a = remove_last_ops_a();
    let b = remove_last_ops_b();
    let result = unify_operation_graphs(&a, &b);
    let mut expected_common_a = vec![Op {
        id: "a_1".to_string(),
        kind: GraphOp::Node(NodeExpr::NullNode {}),
    }];
    let mut expected_common_b = vec![Op {
        id: "b_0".to_string(),
        kind: GraphOp::Node(NodeExpr::NullNode {}),
    }];
    let mut expected_diff_a = vec![
        Op {
            id: "a_0".to_string(),
            kind: GraphOp::Node(NodeExpr::ExistNode {
                is_literal: false,
                label: "Node".to_string(),
                id: "main-new2".to_string(),
            }),
        },
        Op {
            id: "a_2".to_string(),
            kind: GraphOp::Edge(EdgeExpr::EditEdgeReference {
                from: "a_0".to_string(),
                old_to: None,
                new_to: "a_1".to_string(),
                label: "next".to_string(),
            }),
        },
    ];
    let mut expected_diff_b = vec![
        Op {
            id: "b_1".to_string(),
            kind: GraphOp::Node(NodeExpr::ExistNode {
                is_literal: false,
                label: "Node".to_string(),
                id: "main-new1".to_string(),
            }),
        },
        Op {
            id: "b_2".to_string(),
            kind: GraphOp::Edge(EdgeExpr::EditEdgeReference {
                from: "b_1".to_string(),
                old_to: None,
                new_to: "b_0".to_string(),
                label: "next".to_string(),
            }),
        },
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

fn remove_first_ops_a() -> Vec<Op> {
    vec![
        Op {
            id: "a_0".to_string(),
            kind: GraphOp::Node(NodeExpr::ExistNode {
                is_literal: false,
                label: "Node".to_string(),
                id: "main-new2".to_string(),
            }),
        },
        Op {
            id: "a_1".to_string(),
            kind: GraphOp::Variable(VarOp::AddVariable {
                to: "a_0".to_string(),
                label: "return".to_string(),
            }),
        },
    ]
}

fn remove_first_ops_b() -> Vec<Op> {
    vec![
        Op {
            id: "b_0".to_string(),
            kind: GraphOp::Node(NodeExpr::ExistNode {
                is_literal: false,
                label: "Node".to_string(),
                id: "main-new3".to_string(),
            }),
        },
        Op {
            id: "b_1".to_string(),
            kind: GraphOp::Variable(VarOp::AddVariable {
                to: "b_0".to_string(),
                label: "return".to_string(),
            }),
        },
    ]
}

#[test]
fn test_unify_remove_first() {
    let a = remove_first_ops_a();
    let b = remove_first_ops_b();
    let result = unify_operation_graphs(&a, &b);
    let mut expected_common_a = vec![Op {
        id: "a_1".to_string(),
        kind: GraphOp::Variable(VarOp::AddVariable {
            to: "a_0".to_string(),
            label: "return".to_string(),
        }),
    }];
    let mut expected_common_b = vec![Op {
        id: "b_1".to_string(),
        kind: GraphOp::Variable(VarOp::AddVariable {
            to: "b_0".to_string(),
            label: "return".to_string(),
        }),
    }];
    let mut expected_diff_a = vec![Op {
        id: "a_0".to_string(),
        kind: GraphOp::Node(NodeExpr::ExistNode {
            is_literal: false,
            label: "Node".to_string(),
            id: "main-new2".to_string(),
        }),
    }];
    let mut expected_diff_b = vec![Op {
        id: "b_0".to_string(),
        kind: GraphOp::Node(NodeExpr::ExistNode {
            is_literal: false,
            label: "Node".to_string(),
            id: "main-new3".to_string(),
        }),
    }];
    expected_common_a.sort_by_key(|k| format!("{:?}", k));
    expected_common_b.sort_by_key(|k| format!("{:?}", k));
    expected_diff_a.sort_by_key(|k| format!("{:?}", k));
    expected_diff_b.sort_by_key(|k| format!("{:?}", k));
    assert_eq!(result.common_a, expected_common_a);
    assert_eq!(result.common_b, expected_common_b);
    assert_eq!(result.diff_a, expected_diff_a);
    assert_eq!(result.diff_b, expected_diff_b);
}

fn insert_after_ops_a() -> Vec<Op> {
    vec![
        Op {
            id: "a_0".to_string(),
            kind: GraphOp::Node(NodeExpr::AddNode {
                is_literal: false,
                label: "Node".to_string(),
                id: "__temp1".to_string(),
            }),
        },
        Op {
            id: "a_1".to_string(),
            kind: GraphOp::Node(NodeExpr::AddNode {
                is_literal: true,
                label: "3".to_string(),
                id: "__temp2".to_string(),
            }),
        },
        Op {
            id: "a_2".to_string(),
            kind: GraphOp::Edge(EdgeExpr::AddEdge {
                from: "a_0".to_string(),
                to: "a_1".to_string(),
                label: "val".to_string(),
            }),
        },
        Op {
            id: "a_3".to_string(),
            kind: GraphOp::Node(NodeExpr::ExistNode {
                is_literal: false,
                label: "Node".to_string(),
                id: "main-new1".to_string(),
            }),
        },
        Op {
            id: "a_4".to_string(),
            kind: GraphOp::Edge(EdgeExpr::AddEdge {
                from: "a_3".to_string(),
                to: "a_0".to_string(),
                label: "next".to_string(),
            }),
        },
        Op {
            id: "a_5".to_string(),
            kind: GraphOp::Node(NodeExpr::ExistNode {
                is_literal: false,
                label: "Node".to_string(),
                id: "main-new2".to_string(),
            }),
        },
        Op {
            id: "a_6".to_string(),
            kind: GraphOp::Edge(EdgeExpr::AddEdge {
                from: "a_0".to_string(),
                to: "a_5".to_string(),
                label: "next".to_string(),
            }),
        },
    ]
}

fn insert_after_ops_b() -> Vec<Op> {
    vec![
        Op {
            id: "b_0".to_string(),
            kind: GraphOp::Node(NodeExpr::ExistNode {
                is_literal: false,
                label: "Node".to_string(),
                id: "__temp1".to_string(),
            }),
        },
        Op {
            id: "b_1".to_string(),
            kind: GraphOp::Node(NodeExpr::AddNode {
                is_literal: false,
                label: "Node".to_string(),
                id: "__temp3".to_string(),
            }),
        },
        Op {
            id: "b_2".to_string(),
            kind: GraphOp::Edge(EdgeExpr::AddEdge {
                from: "b_0".to_string(),
                to: "b_1".to_string(),
                label: "next".to_string(),
            }),
        },
        Op {
            id: "b_3".to_string(),
            kind: GraphOp::Node(NodeExpr::ExistNode {
                is_literal: false,
                label: "Node".to_string(),
                id: "main-new2".to_string(),
            }),
        },
        Op {
            id: "b_4".to_string(),
            kind: GraphOp::Edge(EdgeExpr::AddEdge {
                from: "b_1".to_string(),
                to: "b_3".to_string(),
                label: "next".to_string(),
            }),
        },
        Op {
            id: "b_5".to_string(),
            kind: GraphOp::Node(NodeExpr::AddNode {
                is_literal: true,
                label: "4".to_string(),
                id: "__temp4".to_string(),
            }),
        },
        Op {
            id: "b_6".to_string(),
            kind: GraphOp::Edge(EdgeExpr::AddEdge {
                from: "b_1".to_string(),
                to: "b_5".to_string(),
                label: "val".to_string(),
            }),
        },
    ]
}

#[test]
fn test_unify_insert_after() {
    let a = insert_after_ops_a();
    let b = insert_after_ops_b();
    let result = unify_operation_graphs(&a, &b);
    let mut expected_common_a = vec![
        Op {
            id: "a_0".to_string(),
            kind: GraphOp::Node(NodeExpr::AddNode {
                is_literal: false,
                label: "Node".to_string(),
                id: "__temp1".to_string(),
            }),
        },
        Op {
            id: "a_2".to_string(),
            kind: GraphOp::Edge(EdgeExpr::AddEdge {
                from: "a_0".to_string(),
                to: "a_1".to_string(),
                label: "val".to_string(),
            }),
        },
        Op {
            id: "a_4".to_string(),
            kind: GraphOp::Edge(EdgeExpr::AddEdge {
                from: "a_3".to_string(),
                to: "a_0".to_string(),
                label: "next".to_string(),
            }),
        },
        Op {
            id: "a_5".to_string(),
            kind: GraphOp::Node(NodeExpr::ExistNode {
                is_literal: false,
                label: "Node".to_string(),
                id: "main-new2".to_string(),
            }),
        },
        Op {
            id: "a_6".to_string(),
            kind: GraphOp::Edge(EdgeExpr::AddEdge {
                from: "a_0".to_string(),
                to: "a_5".to_string(),
                label: "next".to_string(),
            }),
        },
    ];
    let mut expected_common_b = vec![
        Op {
            id: "b_1".to_string(),
            kind: GraphOp::Node(NodeExpr::AddNode {
                is_literal: false,
                label: "Node".to_string(),
                id: "__temp3".to_string(),
            }),
        },
        Op {
            id: "b_2".to_string(),
            kind: GraphOp::Edge(EdgeExpr::AddEdge {
                from: "b_0".to_string(),
                to: "b_1".to_string(),
                label: "next".to_string(),
            }),
        },
        Op {
            id: "b_3".to_string(),
            kind: GraphOp::Node(NodeExpr::ExistNode {
                is_literal: false,
                label: "Node".to_string(),
                id: "main-new2".to_string(),
            }),
        },
        Op {
            id: "b_4".to_string(),
            kind: GraphOp::Edge(EdgeExpr::AddEdge {
                from: "b_1".to_string(),
                to: "b_3".to_string(),
                label: "next".to_string(),
            }),
        },
        Op {
            id: "b_6".to_string(),
            kind: GraphOp::Edge(EdgeExpr::AddEdge {
                from: "b_1".to_string(),
                to: "b_5".to_string(),
                label: "val".to_string(),
            }),
        },
    ];
    let mut expected_diff_a = vec![
        Op {
            id: "a_1".to_string(),
            kind: GraphOp::Node(NodeExpr::AddNode {
                is_literal: true,
                label: "3".to_string(),
                id: "__temp2".to_string(),
            }),
        },
        Op {
            id: "a_3".to_string(),
            kind: GraphOp::Node(NodeExpr::ExistNode {
                is_literal: false,
                label: "Node".to_string(),
                id: "main-new1".to_string(),
            }),
        },
    ];
    let mut expected_diff_b = vec![
        Op {
            id: "b_0".to_string(),
            kind: GraphOp::Node(NodeExpr::ExistNode {
                is_literal: false,
                label: "Node".to_string(),
                id: "__temp1".to_string(),
            }),
        },
        Op {
            id: "b_5".to_string(),
            kind: GraphOp::Node(NodeExpr::AddNode {
                is_literal: true,
                label: "4".to_string(),
                id: "__temp4".to_string(),
            }),
        },
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

fn concat_ops_a() -> Vec<Op> {
    vec![
        Op {
            id: "a_0".to_string(),
            kind: GraphOp::Node(NodeExpr::ExistNode {
                is_literal: false,
                label: "Node".to_string(),
                id: "main-new3".to_string(),
            }),
        },
        Op {
            id: "a_1".to_string(),
            kind: GraphOp::Node(NodeExpr::ExistNode {
                is_literal: false,
                label: "Node".to_string(),
                id: "main-new4".to_string(),
            }),
        },
        Op {
            id: "a_2".to_string(),
            kind: GraphOp::Edge(EdgeExpr::AddEdge {
                from: "a_0".to_string(),
                to: "a_1".to_string(),
                label: "next".to_string(),
            }),
        },
    ]
}

fn concat_ops_b() -> Vec<Op> {
    vec![
        Op {
            id: "b_0".to_string(),
            kind: GraphOp::Node(NodeExpr::ExistNode {
                is_literal: false,
                label: "Node".to_string(),
                id: "main-new6".to_string(),
            }),
        },
        Op {
            id: "b_1".to_string(),
            kind: GraphOp::Node(NodeExpr::ExistNode {
                is_literal: false,
                label: "Node".to_string(),
                id: "main-new5".to_string(),
            }),
        },
        Op {
            id: "b_2".to_string(),
            kind: GraphOp::Edge(EdgeExpr::AddEdge {
                from: "b_1".to_string(),
                to: "b_0".to_string(),
                label: "next".to_string(),
            }),
        },
    ]
}

#[test]
fn test_unify_concat() {
    let a = concat_ops_a();
    let b = concat_ops_b();
    let result = unify_operation_graphs(&a, &b);
    let mut expected_common_a = vec![Op {
        id: "a_2".to_string(),
        kind: GraphOp::Edge(EdgeExpr::AddEdge {
            from: "a_0".to_string(),
            to: "a_1".to_string(),
            label: "next".to_string(),
        }),
    }];
    let mut expected_common_b = vec![Op {
        id: "b_2".to_string(),
        kind: GraphOp::Edge(EdgeExpr::AddEdge {
            from: "b_1".to_string(),
            to: "b_0".to_string(),
            label: "next".to_string(),
        }),
    }];
    let mut expected_diff_a = vec![
        Op {
            id: "a_0".to_string(),
            kind: GraphOp::Node(NodeExpr::ExistNode {
                is_literal: false,
                label: "Node".to_string(),
                id: "main-new3".to_string(),
            }),
        },
        Op {
            id: "a_1".to_string(),
            kind: GraphOp::Node(NodeExpr::ExistNode {
                is_literal: false,
                label: "Node".to_string(),
                id: "main-new4".to_string(),
            }),
        },
    ];
    let mut expected_diff_b = vec![
        Op {
            id: "b_0".to_string(),
            kind: GraphOp::Node(NodeExpr::ExistNode {
                is_literal: false,
                label: "Node".to_string(),
                id: "main-new6".to_string(),
            }),
        },
        Op {
            id: "b_1".to_string(),
            kind: GraphOp::Node(NodeExpr::ExistNode {
                is_literal: false,
                label: "Node".to_string(),
                id: "main-new5".to_string(),
            }),
        },
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

fn set_ops_a() -> Vec<Op> {
    vec![
        Op {
            id: "a_0".to_string(),
            kind: GraphOp::Node(NodeExpr::AddNode {
                is_literal: true,
                label: "5".to_string(),
                id: "__temp1".to_string(),
            }),
        },
        Op {
            id: "a_1".to_string(),
            kind: GraphOp::Node(NodeExpr::ExistNode {
                is_literal: false,
                label: "Node".to_string(),
                id: "main-new3".to_string(),
            }),
        },
        Op {
            id: "a_2".to_string(),
            kind: GraphOp::Edge(EdgeExpr::EditEdgeReference {
                from: "a_1".to_string(),
                old_to: None,
                new_to: "a_0".to_string(),
                label: "val".to_string(),
            }),
        },
    ]
}

fn set_ops_b() -> Vec<Op> {
    vec![
        Op {
            id: "b_0".to_string(),
            kind: GraphOp::Node(NodeExpr::ExistNode {
                is_literal: false,
                label: "Node".to_string(),
                id: "main-new2".to_string(),
            }),
        },
        Op {
            id: "b_1".to_string(),
            kind: GraphOp::Node(NodeExpr::AddNode {
                is_literal: true,
                label: "4".to_string(),
                id: "__temp2".to_string(),
            }),
        },
        Op {
            id: "b_2".to_string(),
            kind: GraphOp::Edge(EdgeExpr::EditEdgeReference {
                from: "b_0".to_string(),
                old_to: None,
                new_to: "b_1".to_string(),
                label: "val".to_string(),
            }),
        },
    ]
}

#[test]
fn test_unify_set() {
    let a = set_ops_a();
    let b = set_ops_b();
    let result = unify_operation_graphs(&a, &b);
    let mut expected_common_a = vec![Op {
        id: "a_2".to_string(),
        kind: GraphOp::Edge(EdgeExpr::EditEdgeReference {
            from: "a_1".to_string(),
            old_to: None,
            new_to: "a_0".to_string(),
            label: "val".to_string(),
        }),
    }];
    let mut expected_common_b = vec![Op {
        id: "b_2".to_string(),
        kind: GraphOp::Edge(EdgeExpr::EditEdgeReference {
            from: "b_0".to_string(),
            old_to: None,
            new_to: "b_1".to_string(),
            label: "val".to_string(),
        }),
    }];
    let mut expected_diff_a = vec![
        Op {
            id: "a_0".to_string(),
            kind: GraphOp::Node(NodeExpr::AddNode {
                is_literal: true,
                label: "5".to_string(),
                id: "__temp1".to_string(),
            }),
        },
        Op {
            id: "a_1".to_string(),
            kind: GraphOp::Node(NodeExpr::ExistNode {
                is_literal: false,
                label: "Node".to_string(),
                id: "main-new3".to_string(),
            }),
        },
    ];
    let mut expected_diff_b = vec![
        Op {
            id: "b_0".to_string(),
            kind: GraphOp::Node(NodeExpr::ExistNode {
                is_literal: false,
                label: "Node".to_string(),
                id: "main-new2".to_string(),
            }),
        },
        Op {
            id: "b_1".to_string(),
            kind: GraphOp::Node(NodeExpr::AddNode {
                is_literal: true,
                label: "4".to_string(),
                id: "__temp2".to_string(),
            }),
        },
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

#[test]
fn test_unify_edit_reference_uses_mapped_old_target_instead_of_raw_ids() {
    let a = vec![
        Op {
            id: "a_from".to_string(),
            kind: GraphOp::Node(NodeExpr::ExistNode {
                is_literal: false,
                label: "Node".to_string(),
                id: "node-a".to_string(),
            }),
        },
        Op {
            id: "a_old".to_string(),
            kind: GraphOp::Node(NodeExpr::ExistNode {
                is_literal: true,
                label: "1".to_string(),
                id: "old-a".to_string(),
            }),
        },
        Op {
            id: "a_new".to_string(),
            kind: GraphOp::Node(NodeExpr::AddNode {
                is_literal: true,
                label: "10".to_string(),
                id: "temp-a".to_string(),
            }),
        },
        Op {
            id: "a_edit".to_string(),
            kind: GraphOp::Edge(EdgeExpr::EditEdgeReference {
                from: "a_from".to_string(),
                old_to: Some("a_old".to_string()),
                new_to: "a_new".to_string(),
                label: "val".to_string(),
            }),
        },
    ];
    let b = vec![
        Op {
            id: "b_from".to_string(),
            kind: GraphOp::Node(NodeExpr::ExistNode {
                is_literal: false,
                label: "Node".to_string(),
                id: "node-b".to_string(),
            }),
        },
        Op {
            id: "b_old".to_string(),
            kind: GraphOp::Node(NodeExpr::ExistNode {
                is_literal: true,
                label: "2".to_string(),
                id: "old-b".to_string(),
            }),
        },
        Op {
            id: "b_new".to_string(),
            kind: GraphOp::Node(NodeExpr::AddNode {
                is_literal: true,
                label: "20".to_string(),
                id: "temp-b".to_string(),
            }),
        },
        Op {
            id: "b_edit".to_string(),
            kind: GraphOp::Edge(EdgeExpr::EditEdgeReference {
                from: "b_from".to_string(),
                old_to: Some("b_old".to_string()),
                new_to: "b_new".to_string(),
                label: "val".to_string(),
            }),
        },
    ];

    let result = unify_operation_graphs(&a, &b);

    assert!(result.common_a.iter().any(|op| op.id == "a_edit"));
    assert!(result.common_b.iter().any(|op| op.id == "b_edit"));
    assert_eq!(
        result.final_mapping.get("a_from"),
        Some(&"b_from".to_string())
    );
    assert_eq!(
        result.final_mapping.get("a_old"),
        Some(&"b_old".to_string())
    );
    assert_eq!(
        result.final_mapping.get("a_new"),
        Some(&"b_new".to_string())
    );
}

fn edit_reference_fixture(
    prefix: &str,
    source_runtime_id: &str,
    old_literal: Option<&str>,
    new_is_literal: bool,
    field: &str,
) -> Vec<Op> {
    let source_id = format!("{prefix}_source");
    let old_id = format!("{prefix}_old");
    let new_id = format!("{prefix}_new");
    let edit_id = format!("{prefix}_edit");
    let mut operations = vec![Op {
        id: source_id.clone(),
        kind: GraphOp::Node(NodeExpr::ExistNode {
            is_literal: false,
            label: "Node".to_string(),
            id: source_runtime_id.to_string(),
        }),
    }];
    if let Some(value) = old_literal {
        operations.push(Op {
            id: old_id.clone(),
            kind: GraphOp::Node(NodeExpr::ExistNode {
                is_literal: true,
                label: value.to_string(),
                id: format!("{prefix}-old-value"),
            }),
        });
    }
    operations.push(Op {
        id: new_id.clone(),
        kind: GraphOp::Node(NodeExpr::AddNode {
            is_literal: new_is_literal,
            label: if new_is_literal { "99" } else { "Node" }.to_string(),
            id: format!("{prefix}-new-value"),
        }),
    });
    operations.push(Op {
        id: edit_id,
        kind: GraphOp::Edge(EdgeExpr::EditEdgeReference {
            from: source_id,
            old_to: old_literal.map(|_| old_id),
            new_to: new_id,
            label: field.to_string(),
        }),
    });
    operations
}

fn assert_edit_is_not_common(a: &[Op], b: &[Op]) {
    let result = unify_operation_graphs(a, b);
    assert!(
        result
            .common_a
            .iter()
            .all(|op| !matches!(op.kind, GraphOp::Edge(EdgeExpr::EditEdgeReference { .. }))),
        "editEdgeReference must remain a difference outside the literal-value relaxation"
    );
}

#[test]
fn edit_reference_does_not_relax_nonliteral_targets() {
    let a = edit_reference_fixture("a", "receiver-a", None, false, "next");
    let b = edit_reference_fixture("b", "receiver-b", None, false, "next");
    assert_edit_is_not_common(&a, &b);
}

#[test]
fn edit_reference_does_not_relax_mixed_literal_targets() {
    let a = edit_reference_fixture("a", "receiver", None, true, "val");
    let b = edit_reference_fixture("b", "receiver", None, false, "val");
    assert_edit_is_not_common(&a, &b);
}

#[test]
fn edit_reference_requires_matching_old_target_presence() {
    let a = edit_reference_fixture("a", "receiver", Some("1"), true, "val");
    let b = edit_reference_fixture("b", "receiver", None, true, "val");
    assert_edit_is_not_common(&a, &b);
}

#[test]
fn edit_reference_requires_matching_field_name() {
    let a = edit_reference_fixture("a", "receiver", None, true, "val");
    let b = edit_reference_fixture("b", "receiver", None, true, "other");
    assert_edit_is_not_common(&a, &b);
}
