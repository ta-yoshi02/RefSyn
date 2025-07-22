use refsyn::ir::{Op, OpKind};
use refsyn::unify_ops::{unify_operation_graphs, canonicalize_ops};

fn ops_a() -> Vec<Op> {
    vec![
        Op { id: "a0".to_string(), kind: OpKind::AddNode { id: "__temp1".to_string(), is_literal: false, label: "Node".to_string() } },
        Op { id: "a1".to_string(), kind: OpKind::AddNode { id: "__temp2".to_string(), is_literal: true, label: "0".to_string() } },
        Op { id: "a2".to_string(), kind: OpKind::AddEdge { from: "__temp1".to_string(), to: "__temp2".to_string(), label: "val".to_string() } },
        Op { id: "a3".to_string(), kind: OpKind::AddEdge { from: "main-new1".to_string(), to: "__temp1".to_string(), label: "next".to_string() } },
    ]
}

fn ops_b() -> Vec<Op> {
    vec![
        Op { id: "b0".to_string(), kind: OpKind::AddEdge { from: "main-new2".to_string(), to: "__tempA".to_string(), label: "next".to_string() } },
        Op { id: "b1".to_string(), kind: OpKind::AddNode { id: "__tempA".to_string(), is_literal: false, label: "Node".to_string() } },
        Op { id: "b2".to_string(), kind: OpKind::AddNode { id: "__tempB".to_string(), is_literal: true, label: "42".to_string() } },
        Op { id: "b3".to_string(), kind: OpKind::AddEdge { from: "__tempA".to_string(), to: "__tempB".to_string(), label: "val".to_string() } },
    ]
}

#[test]
fn test_unify_simple() {
    let a = ops_a();
    let b = ops_b();
    let common = unify_operation_graphs(&a, &b);
    let expected = vec![
        OpKind::AddEdge { from: "E1".to_string(), to: "N1".to_string(), label: "next".to_string() },
        OpKind::AddEdge { from: "N1".to_string(), to: "L1".to_string(), label: "val".to_string() },
        OpKind::AddNode { id: "L1".to_string(), is_literal: true, label: "_".to_string() },
        OpKind::AddNode { id: "N1".to_string(), is_literal: false, label: "Node".to_string() },
    ];
    assert_eq!(common, expected);
}

fn ops_var_del_a() -> Vec<Op> {
    vec![
        Op { id: "a0".to_string(), kind: OpKind::AddVariable { to: "main-new2".to_string(), label: "return".to_string() } },
        Op { id: "a1".to_string(), kind: OpKind::DeleteNode { id: "main-new3".to_string() } },
    ]
}

fn ops_var_del_b() -> Vec<Op> {
    vec![
        Op { id: "b0".to_string(), kind: OpKind::DeleteNode { id: "main-new6".to_string() } },
        Op { id: "b1".to_string(), kind: OpKind::AddVariable { to: "main-new5".to_string(), label: "return".to_string() } },
    ]
}

#[test]
fn test_unify_var_delete() {
    let a = ops_var_del_a();
    let b = ops_var_del_b();
    let common = unify_operation_graphs(&a, &b);
    let expected = vec![
        OpKind::AddVariable { to: "E1".to_string(), label: "return".to_string() },
        OpKind::DeleteNode { id: "E2".to_string() },
    ];
    assert_eq!(common, expected);
}

// --- Additional operations from graph_isomorphism_tests.rs ---

fn ops_permuted() -> Vec<Op> {
    vec![
        Op { id: "c0".to_string(), kind: OpKind::AddNode { id: "__temp1".to_string(), is_literal: true, label: "0".to_string() } },
        Op { id: "c1".to_string(), kind: OpKind::AddNode { id: "__temp2".to_string(), is_literal: false, label: "Node".to_string() } },
        Op { id: "c2".to_string(), kind: OpKind::AddEdge { from: "main-new1".to_string(), to: "__temp2".to_string(), label: "next".to_string() } },
        Op { id: "c3".to_string(), kind: OpKind::AddEdge { from: "__temp2".to_string(), to: "__temp1".to_string(), label: "val".to_string() } },
    ]
}

#[test]
fn test_unify_append_permuted() {
    let a = ops_a();
    let b = ops_permuted();
    let common = unify_operation_graphs(&a, &b);
    let expected = canonicalize_ops(&a);
    assert_eq!(common, expected);
}

fn prepend_ops_original() -> Vec<Op> {
    vec![
        Op { id: "p0".to_string(), kind: OpKind::AddNode { id: "__temp1".to_string(), is_literal: false, label: "Node".to_string() } },
        Op { id: "p1".to_string(), kind: OpKind::AddNode { id: "__temp2".to_string(), is_literal: true, label: "0".to_string() } },
        Op { id: "p2".to_string(), kind: OpKind::AddEdge { from: "__temp1".to_string(), to: "__temp2".to_string(), label: "val".to_string() } },
        Op { id: "p3".to_string(), kind: OpKind::AddEdge { from: "__temp1".to_string(), to: "main-new1".to_string(), label: "next".to_string() } },
        Op { id: "p4".to_string(), kind: OpKind::AddVariable { to: "__temp1".to_string(), label: "return".to_string() } },
    ]
}

fn prepend_ops_permuted() -> Vec<Op> {
    vec![
        Op { id: "p0".to_string(), kind: OpKind::AddNode { id: "__temp2".to_string(), is_literal: true, label: "0".to_string() } },
        Op { id: "p1".to_string(), kind: OpKind::AddNode { id: "__temp1".to_string(), is_literal: false, label: "Node".to_string() } },
        Op { id: "p2".to_string(), kind: OpKind::AddVariable { to: "__temp1".to_string(), label: "return".to_string() } },
        Op { id: "p3".to_string(), kind: OpKind::AddEdge { from: "__temp1".to_string(), to: "__temp2".to_string(), label: "val".to_string() } },
        Op { id: "p4".to_string(), kind: OpKind::AddEdge { from: "__temp1".to_string(), to: "main-new1".to_string(), label: "next".to_string() } },
    ]
}

#[test]
fn test_unify_prepend_permuted() {
    let a = prepend_ops_original();
    let b = prepend_ops_permuted();
    let common = unify_operation_graphs(&a, &b);
    let expected = canonicalize_ops(&a);
    assert_eq!(common, expected);
}

fn remove_last_ops_a() -> Vec<Op> {
    vec![Op { id: "r0".to_string(), kind: OpKind::DeleteNode { id: "main-new3".to_string() } }]
}

fn remove_last_ops_b() -> Vec<Op> {
    vec![Op { id: "r0".to_string(), kind: OpKind::DeleteNode { id: "main-new2".to_string() } }]
}

#[test]
fn test_unify_remove_last() {
    let a = remove_last_ops_a();
    let b = remove_last_ops_b();
    let common = unify_operation_graphs(&a, &b);
    let expected = canonicalize_ops(&a);
    assert_eq!(common, expected);
}

fn remove_first_ops_a() -> Vec<Op> {
    vec![Op { id: "rf0".to_string(), kind: OpKind::AddVariable { to: "main-new2".to_string(), label: "return".to_string() } }]
}

fn remove_first_ops_b() -> Vec<Op> {
    vec![Op { id: "rf0".to_string(), kind: OpKind::AddVariable { to: "main-new3".to_string(), label: "return".to_string() } }]
}

#[test]
fn test_unify_remove_first() {
    let a = remove_first_ops_a();
    let b = remove_first_ops_b();
    let common = unify_operation_graphs(&a, &b);
    let expected = canonicalize_ops(&a);
    assert_eq!(common, expected);
}

fn insert_after_ops_original() -> Vec<Op> {
    vec![
        Op { id: "ia0".to_string(), kind: OpKind::AddNode { id: "__temp1".to_string(), is_literal: false, label: "Node".to_string() } },
        Op { id: "ia1".to_string(), kind: OpKind::AddNode { id: "__temp2".to_string(), is_literal: true, label: "3".to_string() } },
        Op { id: "ia2".to_string(), kind: OpKind::AddEdge { from: "__temp1".to_string(), to: "__temp2".to_string(), label: "val".to_string() } },
        Op { id: "ia3".to_string(), kind: OpKind::AddEdge { from: "__temp1".to_string(), to: "main-new2".to_string(), label: "next".to_string() } },
        Op { id: "ia4".to_string(), kind: OpKind::EditEdgeReference { from: "main-new1".to_string(), old_to: "main-new2".to_string(), new_to: "__temp1".to_string(), label: "next".to_string() } },
    ]
}

fn insert_after_ops_permuted() -> Vec<Op> {
    vec![
        Op { id: "ib0".to_string(), kind: OpKind::AddNode { id: "__temp3".to_string(), is_literal: false, label: "Node".to_string() } },
        Op { id: "ib1".to_string(), kind: OpKind::AddNode { id: "__temp4".to_string(), is_literal: true, label: "4".to_string() } },
        Op { id: "ib2".to_string(), kind: OpKind::AddEdge { from: "__temp3".to_string(), to: "__temp4".to_string(), label: "val".to_string() } },
        Op { id: "ib3".to_string(), kind: OpKind::AddEdge { from: "__temp3".to_string(), to: "main-new2".to_string(), label: "next".to_string() } },
        Op { id: "ib4".to_string(), kind: OpKind::EditEdgeReference { from: "main-new1".to_string(), old_to: "main-new2".to_string(), new_to: "__temp3".to_string(), label: "next".to_string() } },
    ]
}

#[test]
fn test_unify_insert_after() {
    let a = insert_after_ops_original();
    let b = insert_after_ops_permuted();
    let common = unify_operation_graphs(&a, &b);
    let expected = canonicalize_ops(&a);
    assert_eq!(common, expected);
}

fn concat_ops_original() -> Vec<Op> {
    vec![Op { id: "co0".to_string(), kind: OpKind::AddEdge { from: "main-new3".to_string(), to: "main-new4".to_string(), label: "next".to_string() } }]
}

fn concat_ops_permuted() -> Vec<Op> {
    vec![Op { id: "co0".to_string(), kind: OpKind::AddEdge { from: "main-new5".to_string(), to: "main-new6".to_string(), label: "next".to_string() } }]
}

#[test]
fn test_unify_concat() {
    let a = concat_ops_original();
    let b = concat_ops_permuted();
    let common = unify_operation_graphs(&a, &b);
    let expected = canonicalize_ops(&a);
    assert_eq!(common, expected);
}

fn set_ops_original() -> Vec<Op> {
    vec![
        Op { id: "s0".to_string(), kind: OpKind::AddNode { id: "__temp1".to_string(), is_literal: true, label: "5".to_string() } },
        Op { id: "s1".to_string(), kind: OpKind::EditEdgeReference { from: "main-new3".to_string(), old_to: "main-new3-val".to_string(), new_to: "__temp1".to_string(), label: "val".to_string() } },
    ]
}

fn set_ops_permuted() -> Vec<Op> {
    vec![
        Op { id: "s0".to_string(), kind: OpKind::AddNode { id: "__temp2".to_string(), is_literal: true, label: "4".to_string() } },
        Op { id: "s1".to_string(), kind: OpKind::EditEdgeReference { from: "main-new2".to_string(), old_to: "main-new2-val".to_string(), new_to: "__temp2".to_string(), label: "val".to_string() } },
    ]
}

#[test]
fn test_unify_set() {
    let a = set_ops_original();
    let b = set_ops_permuted();
    let common = unify_operation_graphs(&a, &b);
    let expected = canonicalize_ops(&a);
    assert_eq!(common, expected);
}

