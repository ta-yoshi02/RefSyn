use crate::ir::{Op, OpKind};
use std::collections::HashMap;

fn is_existing_object(id: &str) -> bool {
    id.starts_with("main-new") || id == "this" || id == "undefined"
}

/// Canonicalize operations by renaming IDs.
pub fn canonicalize_ops(ops: &[Op]) -> Vec<OpKind> {
    let order = crate::ir::canonical_order(ops);
    let mut map: HashMap<String, String> = HashMap::new();
    let mut new_cnt = 0;
    let mut lit_cnt = 0;
    let mut ex_cnt = 0;

    let mut result = Vec::new();

    for op_id in order {
        if let Some(op) = ops.iter().find(|o| o.id == op_id) {
            match &op.kind {
                OpKind::AddNode { id, is_literal, label } => {
                    let canon = if *is_literal {
                        lit_cnt += 1;
                        map.entry(id.clone()).or_insert_with(|| format!("L{}", lit_cnt)).clone()
                    } else {
                        new_cnt += 1;
                        map.entry(id.clone()).or_insert_with(|| format!("N{}", new_cnt)).clone()
                    };
                    let lbl = if *is_literal { "_".to_string() } else { label.clone() };
                    result.push(OpKind::AddNode { id: canon, is_literal: *is_literal, label: lbl });
                }
                OpKind::EditNode { id, is_literal, label } => {
                    let canon = map.entry(id.clone()).or_insert_with(|| {
                        if *is_literal {
                            lit_cnt += 1;
                            format!("L{}", lit_cnt)
                        } else if is_existing_object(id) {
                            ex_cnt += 1;
                            format!("E{}", ex_cnt)
                        } else {
                            new_cnt += 1;
                            format!("N{}", new_cnt)
                        }
                    }).clone();
                    let lbl = if *is_literal { "_".to_string() } else { label.clone() };
                    result.push(OpKind::EditNode { id: canon, is_literal: *is_literal, label: lbl });
                }
                OpKind::DeleteNode { id } => {
                    let canon = map.entry(id.clone()).or_insert_with(|| {
                        if is_existing_object(id) {
                            ex_cnt += 1;
                            format!("E{}", ex_cnt)
                        } else {
                            new_cnt += 1;
                            format!("N{}", new_cnt)
                        }
                    }).clone();
                    result.push(OpKind::DeleteNode { id: canon });
                }
                OpKind::AddEdge { from, to, label } => {
                    let f = if let Some(c) = map.get(from) { c.clone() } else if is_existing_object(from) {
                        ex_cnt += 1; let c = format!("E{}", ex_cnt); map.insert(from.clone(), c.clone()); c
                    } else { new_cnt += 1; let c = format!("N{}", new_cnt); map.insert(from.clone(), c.clone()); c };
                    let t = if let Some(c) = map.get(to) { c.clone() } else if is_existing_object(to) {
                        ex_cnt += 1; let c = format!("E{}", ex_cnt); map.insert(to.clone(), c.clone()); c
                    } else { new_cnt += 1; let c = format!("N{}", new_cnt); map.insert(to.clone(), c.clone()); c };
                    result.push(OpKind::AddEdge { from: f, to: t, label: label.clone() });
                }
                OpKind::EditEdgeReference { from, old_to, new_to, label } => {
                    let f = if let Some(c) = map.get(from) { c.clone() } else if is_existing_object(from) {
                        ex_cnt += 1; let c = format!("E{}", ex_cnt); map.insert(from.clone(), c.clone()); c
                    } else { new_cnt += 1; let c = format!("N{}", new_cnt); map.insert(from.clone(), c.clone()); c };
                    let o = if let Some(c) = map.get(old_to) { c.clone() } else if is_existing_object(old_to) {
                        ex_cnt += 1; let c = format!("E{}", ex_cnt); map.insert(old_to.clone(), c.clone()); c
                    } else { new_cnt += 1; let c = format!("N{}", new_cnt); map.insert(old_to.clone(), c.clone()); c };
                    let n = if let Some(c) = map.get(new_to) { c.clone() } else if is_existing_object(new_to) {
                        ex_cnt += 1; let c = format!("E{}", ex_cnt); map.insert(new_to.clone(), c.clone()); c
                    } else { new_cnt += 1; let c = format!("N{}", new_cnt); map.insert(new_to.clone(), c.clone()); c };
                    result.push(OpKind::EditEdgeReference { from: f, old_to: o, new_to: n, label: label.clone() });
                }
                OpKind::EditEdgeLabel { from, to, old_label, new_label } => {
                    let f = if let Some(c) = map.get(from) { c.clone() } else if is_existing_object(from) {
                        ex_cnt += 1; let c = format!("E{}", ex_cnt); map.insert(from.clone(), c.clone()); c
                    } else { new_cnt += 1; let c = format!("N{}", new_cnt); map.insert(from.clone(), c.clone()); c };
                    let t = if let Some(c) = map.get(to) { c.clone() } else if is_existing_object(to) {
                        ex_cnt += 1; let c = format!("E{}", ex_cnt); map.insert(to.clone(), c.clone()); c
                    } else { new_cnt += 1; let c = format!("N{}", new_cnt); map.insert(to.clone(), c.clone()); c };
                    result.push(OpKind::EditEdgeLabel { from: f, to: t, old_label: old_label.clone(), new_label: new_label.clone() });
                }
                OpKind::DeleteEdge { from, to, label } => {
                    let f = if let Some(c) = map.get(from) { c.clone() } else if is_existing_object(from) {
                        ex_cnt += 1; let c = format!("E{}", ex_cnt); map.insert(from.clone(), c.clone()); c
                    } else { new_cnt += 1; let c = format!("N{}", new_cnt); map.insert(from.clone(), c.clone()); c };
                    let t = if let Some(c) = map.get(to) { c.clone() } else if is_existing_object(to) {
                        ex_cnt += 1; let c = format!("E{}", ex_cnt); map.insert(to.clone(), c.clone()); c
                    } else { new_cnt += 1; let c = format!("N{}", new_cnt); map.insert(to.clone(), c.clone()); c };
                    result.push(OpKind::DeleteEdge { from: f, to: t, label: label.clone() });
                }
                OpKind::AddVariable { to, label } => {
                    let t = if let Some(c) = map.get(to) { c.clone() } else if is_existing_object(to) {
                        ex_cnt += 1; let c = format!("E{}", ex_cnt); map.insert(to.clone(), c.clone()); c
                    } else { new_cnt += 1; let c = format!("N{}", new_cnt); map.insert(to.clone(), c.clone()); c };
                    result.push(OpKind::AddVariable { to: t, label: label.clone() });
                }
                OpKind::EditVariableReference { old_to, new_to, label } => {
                    let o = if let Some(c) = map.get(old_to) { c.clone() } else if is_existing_object(old_to) {
                        ex_cnt += 1; let c = format!("E{}", ex_cnt); map.insert(old_to.clone(), c.clone()); c
                    } else { new_cnt += 1; let c = format!("N{}", new_cnt); map.insert(old_to.clone(), c.clone()); c };
                    let n = if let Some(c) = map.get(new_to) { c.clone() } else if is_existing_object(new_to) {
                        ex_cnt += 1; let c = format!("E{}", ex_cnt); map.insert(new_to.clone(), c.clone()); c
                    } else { new_cnt += 1; let c = format!("N{}", new_cnt); map.insert(new_to.clone(), c.clone()); c };
                    result.push(OpKind::EditVariableReference { old_to: o, new_to: n, label: label.clone() });
                }
                OpKind::EditVariableLabel { to, old_label, new_label } => {
                    // OpId to is not rewritten
                    result.push(OpKind::EditVariableLabel { to: to.clone(), old_label: old_label.clone(), new_label: new_label.clone() });
                }
                OpKind::DeleteVariable { to, label } => {
                    result.push(OpKind::DeleteVariable { to: to.clone(), label: label.clone() });
                }
            }
        }
    }
    result.sort_by_key(|k| format!("{:?}", k));
    result
}

/// Compute the maximum common set of operations between two sequences.
///
/// The returned operations are in canonical form and represent the largest
/// multiset of operations that appear in both sequences when existing object
/// references and literals are normalized.
pub fn unify_operation_graphs(a: &[Op], b: &[Op]) -> Vec<OpKind> {
    let ca = canonicalize_ops(a);
    let cb = canonicalize_ops(b);

    let mut freq_a: HashMap<String, (OpKind, usize)> = HashMap::new();
    for op in ca {
        let key = format!("{:?}", op);
        let entry = freq_a.entry(key).or_insert_with(|| (op.clone(), 0));
        entry.1 += 1;
    }

    let mut freq_b: HashMap<String, usize> = HashMap::new();
    for op in cb {
        let key = format!("{:?}", op);
        *freq_b.entry(key).or_insert(0) += 1;
    }

    let mut result = Vec::new();
    for (key, (op, count_a)) in freq_a.into_iter() {
        if let Some(count_b) = freq_b.get(&key) {
            let n = std::cmp::min(count_a, *count_b);
            for _ in 0..n {
                result.push(op.clone());
            }
        }
    }

    result.sort_by_key(|k| format!("{:?}", k));
    result
}

