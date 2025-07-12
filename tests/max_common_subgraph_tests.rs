use refsyn::ir::{Op, OpKind, maximum_common_subgraph};

fn append_ops_original() -> Vec<Op> {
    vec![
        Op { id: "op_0".to_string(), kind: OpKind::AddNode { id: "__temp1".to_string(), is_literal: false, label: "Node".to_string() } },
        Op { id: "op_1".to_string(), kind: OpKind::AddNode { id: "__temp2".to_string(), is_literal: true, label: "0".to_string() } },
        Op { id: "op_2".to_string(), kind: OpKind::AddEdge { from: "__temp1".to_string(), to: "__temp2".to_string(), label: "val".to_string() } },
        Op { id: "op_3".to_string(), kind: OpKind::AddEdge { from: "main-new1".to_string(), to: "__temp1".to_string(), label: "next".to_string() } },
    ]
}

fn append_ops_b() -> Vec<Op> {
    vec![
        Op { id: "op_0".to_string(), kind: OpKind::AddNode { id: "__temp3".to_string(), is_literal: true, label: "3".to_string() } },
        Op { id: "op_1".to_string(), kind: OpKind::AddNode { id: "__temp4".to_string(), is_literal: false, label: "Node".to_string() } },
        Op { id: "op_2".to_string(), kind: OpKind::AddEdge { from: "__temp1".to_string(), to: "__temp4".to_string(), label: "next".to_string() } },
        Op { id: "op_3".to_string(), kind: OpKind::AddEdge { from: "__temp4".to_string(), to: "__temp3".to_string(), label: "val".to_string() } },
    ]
}

#[test]
fn test_max_common_subgraph_mapping_and_diff() {
    let ops_a = append_ops_original();
    let ops_b = append_ops_b();
    let res = maximum_common_subgraph(&ops_a, &ops_b);
    let mut mapping = res.mapping.clone();
    mapping.sort();
    let mut expected_mapping = vec![("op_0".to_string(), "op_1".to_string()), ("op_2".to_string(), "op_3".to_string())];
    expected_mapping.sort();
    assert_eq!(mapping, expected_mapping);
    assert_eq!(res.diff_a, vec!["op_1".to_string(), "op_3".to_string()]);
    assert_eq!(res.diff_b, vec!["op_0".to_string(), "op_2".to_string()]);
}

fn prepend_ops_original() -> Vec<Op> {
    vec![
        Op { id: "op_0".to_string(), kind: OpKind::AddNode { id: "__temp1".to_string(), is_literal: false, label: "Node".to_string() } },
        Op { id: "op_1".to_string(), kind: OpKind::AddNode { id: "__temp2".to_string(), is_literal: true, label: "0".to_string() } },
        Op { id: "op_2".to_string(), kind: OpKind::AddEdge { from: "__temp1".to_string(), to: "__temp2".to_string(), label: "val".to_string() } },
        Op { id: "op_3".to_string(), kind: OpKind::AddEdge { from: "__temp1".to_string(), to: "main-new1".to_string(), label: "next".to_string() } },
        Op { id: "op_4".to_string(), kind: OpKind::AddVariable { to: "__temp1".to_string(), label: "return".to_string() } },
    ]
}

fn insert_after_ops_original() -> Vec<Op> {
    vec![
        Op { id: "op_0".to_string(), kind: OpKind::AddNode { id: "__temp1".to_string(), is_literal: false, label: "Node".to_string() } },
        Op { id: "op_1".to_string(), kind: OpKind::AddNode { id: "__temp2".to_string(), is_literal: true, label: "3".to_string() } },
        Op { id: "op_2".to_string(), kind: OpKind::AddEdge { from: "__temp1".to_string(), to: "__temp2".to_string(), label: "val".to_string() } },
        Op { id: "op_3".to_string(), kind: OpKind::AddEdge { from: "__temp1".to_string(), to: "main-new2".to_string(), label: "next".to_string() } },
        Op { id: "op_4".to_string(), kind: OpKind::EditEdgeReference { from: "main-new1".to_string(), old_to: "main-new2".to_string(), new_to: "__temp1".to_string(), label: "next".to_string() } },
    ]
}

#[test]
fn test_max_common_subgraph_difference() {
    let ops_a = prepend_ops_original();
    let ops_b = insert_after_ops_original();
    let res = maximum_common_subgraph(&ops_a, &ops_b);
    let mut mapping = res.mapping.clone();
    mapping.sort();
    let mut expected_mapping = vec![
        ("op_0".to_string(), "op_0".to_string()),
        ("op_2".to_string(), "op_2".to_string()),
        ("op_3".to_string(), "op_3".to_string()),
    ];
    expected_mapping.sort();
    assert_eq!(mapping, expected_mapping);
    assert_eq!(res.diff_a, vec!["op_1".to_string(), "op_4".to_string()]);
    assert_eq!(res.diff_b, vec!["op_1".to_string(), "op_4".to_string()]);
}

