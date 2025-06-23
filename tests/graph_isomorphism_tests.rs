use refsyn::ir::{Op, OpKind, match_graphs_with_isomorphism};

#[test]
fn test_match_graphs_with_isomorphism_append_example() {
    let ops_a = vec![
        Op { id: "op_0".to_string(), kind: OpKind::AddNode { id: "__temp1".to_string(), is_literal: false, label: "Node".to_string() } },
        Op { id: "op_1".to_string(), kind: OpKind::AddNode { id: "__temp2".to_string(), is_literal: true, label: "0".to_string() } },
        Op { id: "op_2".to_string(), kind: OpKind::AddEdge { from: "__temp1".to_string(), to: "__temp2".to_string(), label: "val".to_string() } },
        Op { id: "op_3".to_string(), kind: OpKind::AddEdge { from: "main-new1".to_string(), to: "__temp1".to_string(), label: "next".to_string() } },
    ];

    let ops_b = vec![
        Op { id: "op_0".to_string(), kind: OpKind::AddNode { id: "__temp3".to_string(), is_literal: true, label: "3".to_string() } },
        Op { id: "op_1".to_string(), kind: OpKind::AddNode { id: "__temp4".to_string(), is_literal: false, label: "Node".to_string() } },
        Op { id: "op_2".to_string(), kind: OpKind::AddEdge { from: "__temp1".to_string(), to: "__temp4".to_string(), label: "next".to_string() } },
        Op { id: "op_3".to_string(), kind: OpKind::AddEdge { from: "__temp4".to_string(), to: "__temp3".to_string(), label: "val".to_string() } },
    ];

    assert!(match_graphs_with_isomorphism(&ops_a, &ops_b));
}

fn append_ops_original() -> Vec<Op> {
    vec![
        Op { id: "op_0".to_string(), kind: OpKind::AddNode { id: "__temp1".to_string(), is_literal: false, label: "Node".to_string() } },
        Op { id: "op_1".to_string(), kind: OpKind::AddNode { id: "__temp2".to_string(), is_literal: true, label: "0".to_string() } },
        Op { id: "op_2".to_string(), kind: OpKind::AddEdge { from: "__temp1".to_string(), to: "__temp2".to_string(), label: "val".to_string() } },
        Op { id: "op_3".to_string(), kind: OpKind::AddEdge { from: "main-new1".to_string(), to: "__temp1".to_string(), label: "next".to_string() } },
    ]
}

fn append_ops_permuted() -> Vec<Op> {
    vec![
        Op { id: "op_0".to_string(), kind: OpKind::AddNode { id: "__temp2".to_string(), is_literal: true, label: "0".to_string() } },
        Op { id: "op_1".to_string(), kind: OpKind::AddNode { id: "__temp1".to_string(), is_literal: false, label: "Node".to_string() } },
        Op { id: "op_2".to_string(), kind: OpKind::AddEdge { from: "main-new1".to_string(), to: "__temp1".to_string(), label: "next".to_string() } },
        Op { id: "op_3".to_string(), kind: OpKind::AddEdge { from: "__temp1".to_string(), to: "__temp2".to_string(), label: "val".to_string() } },
    ]
}

#[test]
fn test_match_graphs_with_isomorphism_append_permuted() {
    let ops_a = append_ops_original();
    let ops_b = append_ops_permuted();
    assert!(match_graphs_with_isomorphism(&ops_a, &ops_b));
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

fn prepend_ops_permuted() -> Vec<Op> {
    vec![
        Op { id: "op_0".to_string(), kind: OpKind::AddNode { id: "__temp2".to_string(), is_literal: true, label: "0".to_string() } },
        Op { id: "op_1".to_string(), kind: OpKind::AddNode { id: "__temp1".to_string(), is_literal: false, label: "Node".to_string() } },
        Op { id: "op_2".to_string(), kind: OpKind::AddVariable { to: "__temp1".to_string(), label: "return".to_string() } },
        Op { id: "op_3".to_string(), kind: OpKind::AddEdge { from: "__temp1".to_string(), to: "__temp2".to_string(), label: "val".to_string() } },
        Op { id: "op_4".to_string(), kind: OpKind::AddEdge { from: "__temp1".to_string(), to: "main-new1".to_string(), label: "next".to_string() } },
    ]
}

#[test]
fn test_match_graphs_with_isomorphism_prepend_permuted() {
    let ops_a = prepend_ops_original();
    let ops_b = prepend_ops_permuted();
    assert!(match_graphs_with_isomorphism(&ops_a, &ops_b));
}

fn remove_last_ops_a() -> Vec<Op> {
    vec![
        Op { id: "op_0".to_string(), kind: OpKind::DeleteNode { id: "main-new3".to_string() } },
    ]
}

fn remove_last_ops_b() -> Vec<Op> {
    vec![
        Op { id: "op_0".to_string(), kind: OpKind::DeleteNode { id: "main-new2".to_string() } },
    ]
}

#[test]
fn test_match_graphs_with_isomorphism_remove_last() {
    let ops_a = remove_last_ops_a();
    let ops_b = remove_last_ops_b();
    assert!(match_graphs_with_isomorphism(&ops_a, &ops_b));
}

fn remove_first_ops_a() -> Vec<Op> {
    vec![Op { id: "op_0".to_string(), kind: OpKind::AddVariable { to: "main-new2".to_string(), label: "return".to_string() } }]
}

fn remove_first_ops_b() -> Vec<Op> {
    vec![Op { id: "op_0".to_string(), kind: OpKind::AddVariable { to: "main-new3".to_string(), label: "return".to_string() } }]
}

#[test]
fn test_match_graphs_with_isomorphism_remove_first() {
    let ops_a = remove_first_ops_a();
    let ops_b = remove_first_ops_b();
    assert!(match_graphs_with_isomorphism(&ops_a, &ops_b));
}

fn remove_at_ops_original() -> Vec<Op> {
    vec![
        Op { id: "op_0".to_string(), kind: OpKind::EditEdgeReference { from: "main-new2".to_string(), old_to: "main-new3".to_string(), new_to: "main-new4".to_string(), label: "next".to_string() } },
        Op { id: "op_1".to_string(), kind: OpKind::AddVariable { to: "main-new1".to_string(), label: "return".to_string() } },
    ]
}

fn remove_at_ops_permuted() -> Vec<Op> {
    vec![
        Op { id: "op_0".to_string(), kind: OpKind::AddVariable { to: "main-new1".to_string(), label: "return".to_string() } },
        Op { id: "op_1".to_string(), kind: OpKind::EditEdgeReference { from: "main-new1".to_string(), old_to: "main-new2".to_string(), new_to: "main-new4".to_string(), label: "next".to_string() } },
    ]
}

#[test]
fn test_match_graphs_with_isomorphism_remove_at_permuted() {
    let ops_a = remove_at_ops_original();
    let ops_b = remove_at_ops_permuted();
    assert!(match_graphs_with_isomorphism(&ops_a, &ops_b));
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

#[test]
fn test_match_graphs_with_isomorphism_insert_after_permuted() {
    let ops_a = insert_after_ops_original();
    let ops_b = insert_after_ops_permuted();
    assert!(match_graphs_with_isomorphism(&ops_a, &ops_b));
}

fn concat_ops_original() -> Vec<Op> {
    vec![Op { id: "op_0".to_string(), kind: OpKind::AddEdge { from: "main-new3".to_string(), to: "main-new4".to_string(), label: "next".to_string() } }]
}

fn concat_ops_permuted() -> Vec<Op> {
    vec![Op { id: "op_0".to_string(), kind: OpKind::AddEdge { from: "main-new5".to_string(), to: "main-new6".to_string(), label: "next".to_string() } }]
}

#[test]
fn test_match_graphs_with_isomorphism_concat_permuted() {
    let ops_a = concat_ops_original();
    let ops_b = concat_ops_permuted();
    assert!(match_graphs_with_isomorphism(&ops_a, &ops_b));
}

fn set_ops_original() -> Vec<Op> {
    vec![
        Op { id: "op_0".to_string(), kind: OpKind::AddNode { id: "__temp1".to_string(), is_literal: true, label: "5".to_string() } },
        Op { id: "op_1".to_string(), kind: OpKind::EditEdgeReference { from: "main-new3".to_string(), old_to: "main-new3-val".to_string(), new_to: "__temp1".to_string(), label: "val".to_string() } },
    ]
}

fn set_ops_permuted() -> Vec<Op> {
    vec![
        Op { id: "op_0".to_string(), kind: OpKind::AddNode { id: "__temp2".to_string(), is_literal: true, label: "4".to_string() } },
        Op { id: "op_1".to_string(), kind: OpKind::EditEdgeReference { from: "main-new2".to_string(), old_to: "main-new2-val".to_string(), new_to: "__temp2".to_string(), label: "val".to_string() } },
    ]
}

#[test]
fn test_match_graphs_with_isomorphism_set_permuted() {
    let ops_a = set_ops_original();
    let ops_b = set_ops_permuted();
    assert!(match_graphs_with_isomorphism(&ops_a, &ops_b));
}
