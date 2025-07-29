use refsyn::ir::{Op, OpKind, match_graphs_with_isomorphism};
use refsyn::build_graph_detailed::match_graphs_with_flexible_structure;


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
fn test_match_graphs_with_isomorphism_append_original() {
    let ops_a = append_ops_original();
    let ops_b = append_ops_b();
    assert!(match_graphs_with_isomorphism(&ops_a, &ops_b));
}

fn append_ops_permuted() -> Vec<Op> {
    vec![
        Op { id: "op_0".to_string(), kind: OpKind::AddNode { id: "__temp1".to_string(), is_literal: true, label: "0".to_string() } },
        Op { id: "op_1".to_string(), kind: OpKind::AddNode { id: "__temp2".to_string(), is_literal: false, label: "Node".to_string() } },
        Op { id: "op_2".to_string(), kind: OpKind::AddEdge { from: "main-new1".to_string(), to: "__temp2".to_string(), label: "next".to_string() } },
        Op { id: "op_3".to_string(), kind: OpKind::AddEdge { from: "__temp2".to_string(), to: "__temp1".to_string(), label: "val".to_string() } },
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

// 注意: これらの操作は構造的には違うパターンだが、操作の依存関係のグラフ構造しか見ていないのでダメ。
// #[test]
// fn test_match_graphs_with_isomorphism_remove_at_permuted() {
//     let ops_a = remove_at_ops_original();
//     let ops_b = remove_at_ops_permuted();
//     assert!(match_graphs_with_isomorphism(&ops_a, &ops_b));
// }

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

#[test]
fn test_match_graphs_with_isomorphism_different() {
    let ops_a = prepend_ops_original();
    let ops_b = insert_after_ops_original();
    assert!(!match_graphs_with_isomorphism(&ops_a, &ops_b));
}

// === 詳細構造グラフを使用したテスト ===

#[test]
fn test_flexible_structure_append_original() {
    let ops_a = append_ops_original();
    let ops_b = append_ops_b();
    assert!(match_graphs_with_flexible_structure(&ops_a, &ops_b));
}

#[test]
fn test_flexible_structure_append_permuted() {
    let ops_a = append_ops_original();
    let ops_b = append_ops_permuted();
    assert!(match_graphs_with_flexible_structure(&ops_a, &ops_b));
}

#[test]
fn test_flexible_structure_prepend_permuted() {
    let ops_a = prepend_ops_original();
    let ops_b = prepend_ops_permuted();
    assert!(match_graphs_with_flexible_structure(&ops_a, &ops_b));
}

#[test]
fn test_flexible_structure_remove_last() {
    let ops_a = remove_last_ops_a();
    let ops_b = remove_last_ops_b();
    assert!(match_graphs_with_flexible_structure(&ops_a, &ops_b));
}

#[test]
fn test_flexible_structure_remove_first() {
    let ops_a = remove_first_ops_a();
    let ops_b = remove_first_ops_b();
    assert!(match_graphs_with_flexible_structure(&ops_a, &ops_b));
}

#[test]
fn test_flexible_structure_remove_at_permuted() {
    let ops_a = remove_at_ops_original();
    let ops_b = remove_at_ops_permuted();
    assert!(!match_graphs_with_flexible_structure(&ops_a, &ops_b));
}

#[test]
fn test_flexible_structure_insert_after_permuted() {
    let ops_a = insert_after_ops_original();
    let ops_b = insert_after_ops_permuted();
    assert!(match_graphs_with_flexible_structure(&ops_a, &ops_b));
}

#[test]
fn test_flexible_structure_concat_permuted() {
    let ops_a = concat_ops_original();
    let ops_b = concat_ops_permuted();
    assert!(match_graphs_with_flexible_structure(&ops_a, &ops_b));
}

#[test]
fn test_flexible_structure_set_permuted() {
    let ops_a = set_ops_original();
    let ops_b = set_ops_permuted();
    assert!(match_graphs_with_flexible_structure(&ops_a, &ops_b));
}

#[test]
fn test_flexible_structure_different() {
    let ops_a = prepend_ops_original();
    let ops_b = insert_after_ops_original();
    assert!(!match_graphs_with_flexible_structure(&ops_a, &ops_b));
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
fn test_match_graphs_with_isomorphism_add2_nodes() {
    let ops_a = add2_nodes_ops_a();
    let ops_b = add2_nodes_ops_b();
    assert!(match_graphs_with_flexible_structure(&ops_a, &ops_b));
}
