use refsyn::ir::{Op, OpKind, semantic_common_subgraph, NodeIdDiff};

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

fn insert_after_ops_original() -> Vec<Op> {
    vec![
        Op { id: "op_0".to_string(), kind: OpKind::AddNode { id: "__temp1".to_string(), is_literal: false, label: "Node".to_string() } },
        Op { id: "op_1".to_string(), kind: OpKind::AddNode { id: "__temp2".to_string(), is_literal: true, label: "3".to_string() } },
        Op { id: "op_2".to_string(), kind: OpKind::AddEdge { from: "__temp1".to_string(), to: "__temp2".to_string(), label: "val".to_string() } },
        Op { id: "op_3".to_string(), kind: OpKind::AddEdge { from: "__temp1".to_string(), to: "main-new2".to_string(), label: "next".to_string() } },
        Op { id: "op_4".to_string(), kind: OpKind::EditEdgeReference { from: "main-new1".to_string(), old_to: "main-new2".to_string(), new_to: "__temp1".to_string(), label: "next".to_string() } },
    ]
}

fn insert_after_ops_permuted() -> Vec<Op> {
    vec![
        Op { id: "op_0".to_string(), kind: OpKind::AddNode { id: "__temp3".to_string(), is_literal: false, label: "Node".to_string() } },
        Op { id: "op_1".to_string(), kind: OpKind::AddNode { id: "__temp4".to_string(), is_literal: true, label: "4".to_string() } },
        Op { id: "op_2".to_string(), kind: OpKind::AddEdge { from: "__temp3".to_string(), to: "__temp4".to_string(), label: "val".to_string() } },
        Op { id: "op_3".to_string(), kind: OpKind::AddEdge { from: "__temp3".to_string(), to: "main-new2".to_string(), label: "next".to_string() } },
        Op { id: "op_4".to_string(), kind: OpKind::EditEdgeReference { from: "main-new1".to_string(), old_to: "main-new2".to_string(), new_to: "__temp3".to_string(), label: "next".to_string() } },
    ]
}

fn concat_ops_original() -> Vec<Op> {
    vec![Op { id: "op_0".to_string(), kind: OpKind::AddEdge { from: "main-new3".to_string(), to: "main-new4".to_string(), label: "next".to_string() } }]
}

fn concat_ops_permuted() -> Vec<Op> {
    vec![Op { id: "op_0".to_string(), kind: OpKind::AddEdge { from: "main-new5".to_string(), to: "main-new6".to_string(), label: "next".to_string() } }]
}

fn add2_nodes_ops_a() -> Vec<Op> {
    vec![
        Op { id: "op_0".to_string(), kind: OpKind::AddNode { id: "__temp1".to_string(), is_literal: false, label: "Node".to_string() } },
        Op { id: "op_1".to_string(), kind: OpKind::AddNode { id: "__temp2".to_string(), is_literal: true, label: "0".to_string() } },
        Op { id: "op_2".to_string(), kind: OpKind::AddEdge { from: "__temp1".to_string(), to: "__temp2".to_string(), label: "val".to_string() } },
        Op { id: "op_3".to_string(), kind: OpKind::AddNode { id: "__temp3".to_string(), is_literal: false, label: "Node".to_string() } },
        Op { id: "op_4".to_string(), kind: OpKind::AddNode { id: "__temp4".to_string(), is_literal: true, label: "3".to_string() } },
        Op { id: "op_5".to_string(), kind: OpKind::AddEdge { from: "__temp3".to_string(), to: "__temp4".to_string(), label: "val".to_string() } },
        Op { id: "op_6".to_string(), kind: OpKind::AddEdge { from: "main-new1".to_string(), to: "__temp1".to_string(), label: "next".to_string() } },
        Op { id: "op_7".to_string(), kind: OpKind::AddEdge { from: "__temp1".to_string(), to: "__temp3".to_string(), label: "next".to_string() } },
    ]
}

fn add2_nodes_ops_b() -> Vec<Op> {
    vec![
        Op { id: "op_0".to_string(), kind: OpKind::AddNode { id: "__temp5".to_string(), is_literal: false, label: "Node".to_string() } },
        Op { id: "op_1".to_string(), kind: OpKind::AddNode { id: "__temp6".to_string(), is_literal: true, label: "1".to_string() } },
        Op { id: "op_2".to_string(), kind: OpKind::AddNode { id: "__temp7".to_string(), is_literal: false, label: "Node".to_string() } },
        Op { id: "op_3".to_string(), kind: OpKind::AddEdge { from: "__temp7".to_string(), to: "__temp5".to_string(), label: "next".to_string() } },
        Op { id: "op_4".to_string(), kind: OpKind::AddEdge { from: "__temp3".to_string(), to: "__temp7".to_string(), label: "next".to_string() } },
        Op { id: "op_5".to_string(), kind: OpKind::AddNode { id: "__temp8".to_string(), is_literal: true, label: "4".to_string() } },
        Op { id: "op_6".to_string(), kind: OpKind::AddEdge { from: "__temp7".to_string(), to: "__temp6".to_string(), label: "val".to_string() } },
        Op { id: "op_7".to_string(), kind: OpKind::AddEdge { from: "__temp5".to_string(), to: "__temp8".to_string(), label: "val".to_string() } },
    ]
}

#[test]
fn test_semantic_common_subgraph_append() {
    let ops_a = append_ops_original();
    let ops_b = append_ops_b();
    let res = semantic_common_subgraph(&ops_a, &ops_b);
    let mut mapping = res.mapping.clone();
    mapping.sort();
    let mut expected_mapping = vec![
        ("op_0".to_string(), "op_1".to_string()),
        ("op_1".to_string(), "op_0".to_string()),
        ("op_2".to_string(), "op_3".to_string()),
        ("op_3".to_string(), "op_2".to_string()),
    ];
    expected_mapping.sort();
    assert_eq!(mapping, expected_mapping);
    assert!(res.diff_a.is_empty());
    assert!(res.diff_b.is_empty());
    let mut diffs = res.node_id_diffs.clone();
    diffs.sort_by(|a,b| (a.op_a.clone(), a.op_b.clone(), a.field.clone()).cmp(&(b.op_a.clone(), b.op_b.clone(), b.field.clone())));
    let expected = vec![
        NodeIdDiff { op_a: "op_2".to_string(), op_b: "op_3".to_string(), field: "from".to_string(), id_a: "__temp1".to_string(), id_b: "__temp4".to_string() },
        NodeIdDiff { op_a: "op_2".to_string(), op_b: "op_3".to_string(), field: "to".to_string(), id_a: "__temp2".to_string(), id_b: "__temp3".to_string() },
        NodeIdDiff { op_a: "op_3".to_string(), op_b: "op_2".to_string(), field: "from".to_string(), id_a: "main-new1".to_string(), id_b: "__temp1".to_string() },
        NodeIdDiff { op_a: "op_3".to_string(), op_b: "op_2".to_string(), field: "to".to_string(), id_a: "__temp1".to_string(), id_b: "__temp4".to_string() },
    ];
    let mut expected_sorted = expected.clone();
    expected_sorted.sort_by(|a,b| (a.op_a.clone(), a.op_b.clone(), a.field.clone()).cmp(&(b.op_a.clone(), b.op_b.clone(), b.field.clone())));
    assert_eq!(diffs, expected_sorted);
}

#[test]
fn test_semantic_common_subgraph_concat() {
    let ops_a = concat_ops_original();
    let ops_b = concat_ops_permuted();
    let res = semantic_common_subgraph(&ops_a, &ops_b);
    assert_eq!(res.mapping, vec![("op_0".to_string(), "op_0".to_string())]);
    assert!(res.diff_a.is_empty());
    assert!(res.diff_b.is_empty());
    let mut diffs = res.node_id_diffs.clone();
    diffs.sort_by(|a, b| a.field.cmp(&b.field));
    assert_eq!(diffs.len(), 2);
    assert_eq!(
        diffs[0],
        NodeIdDiff {
            op_a: "op_0".to_string(),
            op_b: "op_0".to_string(),
            field: "from".to_string(),
            id_a: "main-new3".to_string(),
            id_b: "main-new5".to_string(),
        }
    );
    assert_eq!(
        diffs[1],
        NodeIdDiff {
            op_a: "op_0".to_string(),
            op_b: "op_0".to_string(),
            field: "to".to_string(),
            id_a: "main-new4".to_string(),
            id_b: "main-new6".to_string(),
        }
    );
}

#[test]
fn test_semantic_common_subgraph_remove() {
    use refsyn::ir::OpKind::DeleteNode;
    let ops_a = vec![Op { id: "op_0".to_string(), kind: DeleteNode { id: "main-new3".to_string() } }];
    let ops_b = vec![Op { id: "op_0".to_string(), kind: DeleteNode { id: "main-new2".to_string() } }];
    let res = semantic_common_subgraph(&ops_a, &ops_b);
    assert_eq!(res.mapping, vec![("op_0".to_string(), "op_0".to_string())]);
    assert!(res.node_id_diffs.is_empty());
}

#[test]
fn test_semantic_common_subgraph_insert_after() {
    let ops_a = insert_after_ops_original();
    let ops_b = insert_after_ops_permuted();
    let res = semantic_common_subgraph(&ops_a, &ops_b);
    assert_eq!(res.mapping.len(), 5);
    assert_eq!(res.node_id_diffs.len(), 3);
}

#[test]
fn test_semantic_common_subgraph_add2_nodes() {
    let ops_a = add2_nodes_ops_a();
    let ops_b = add2_nodes_ops_b();
    let res = semantic_common_subgraph(&ops_a, &ops_b);
    assert_eq!(res.mapping.len(), 8);
    assert!(!res.node_id_diffs.is_empty());
}
