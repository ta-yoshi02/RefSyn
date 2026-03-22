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
    let mut expected_common_a = vec![];
    let mut expected_common_b = vec![];
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
        Op {
            id: "a_2".to_string(),
            kind: GraphOp::Edge(EdgeExpr::EditEdgeReference {
                from: "a_1".to_string(),
                old_to: None,
                new_to: "a_0".to_string(),
                label: "val".to_string(),
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
        Op {
            id: "b_2".to_string(),
            kind: GraphOp::Edge(EdgeExpr::EditEdgeReference {
                from: "b_0".to_string(),
                old_to: None,
                new_to: "b_1".to_string(),
                label: "val".to_string(),
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
