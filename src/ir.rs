use petgraph::graph::{DiGraph, NodeIndex};
use petgraph::Direction;
use petgraph::visit::EdgeRef;
use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use crate::ast;
use crate::env::MemoEnv;
use crate::convert_operations_to_ir;

/// 操作ID
pub type OpId = String;

/// 操作の種類
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub enum OpKind {
    // ノード操作
    AddNode { id: OpId, is_literal: bool, label: String },
    EditNode { id: OpId, is_literal: bool, label: String },
    DeleteNode { id: OpId },
    
    // エッジ操作
    AddEdge { from: OpId, to: OpId, label: String },
    EditEdgeReference { from: OpId, old_to: OpId, new_to: OpId, label: String },
    EditEdgeLabel { from: OpId, to: OpId, old_label: String, new_label: String },
    DeleteEdge { from: OpId, to: OpId, label: String },
    
    // 変数操作
    AddVariable { to: OpId, label: String },
    EditVariableReference { old_to: Option<OpId>, new_to: OpId, label: String },
    EditVariableLabel { to: OpId, old_label: String, new_label: String },
    DeleteVariable { to: OpId, label: String }
}

/// 単一の操作
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct Op {
    pub id: OpId,
    pub kind: OpKind,
}

/// 依存辺のタグ
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EdgeTag {
    /// 生成-使用の依存関係
    GenUse,
    /// 上書きの依存関係
    Overwrite,
    /// 戻り値の依存関係
    ReturnDep,
}

/// 操作列から依存グラフを構築
pub fn build_graph(ops: &[Op]) -> (DiGraph<OpId, EdgeTag>, HashMap<OpId, NodeIndex>) {
    let mut graph = DiGraph::new();
    let mut node_indices = HashMap::new();
    
    // すべてのノードをグラフに追加
    for op in ops {
        let node_idx = graph.add_node(op.id.clone());
        node_indices.insert(op.id.clone(), node_idx);
    }
    
    // 操作間の依存関係を分析して辺を追加
    for (i, u) in ops.iter().enumerate() {
        for v in &ops[i+1..] {
            match (&u.kind, &v.kind) {
                // AddNode → 任意の操作 (v が id₁ を参照する場合)
                (OpKind::AddNode { id: id1, .. }, _) => {
                    if references_id(v, id1) {
                        if let (Some(&u_idx), Some(&v_idx)) = (node_indices.get(&u.id), node_indices.get(&v.id)) {
                            graph.add_edge(u_idx, v_idx, EdgeTag::GenUse);
                        }
                    }
                },
                
                // AddEdge → AddEdge (fromが一致かつlabelが一致なら順序あり)
                (OpKind::AddEdge { from: from1, label: label1, .. }, 
                 OpKind::AddEdge { from: from2, label: label2, .. }) => {
                    if from1 == from2 && label1 == label2 {
                        if let (Some(&u_idx), Some(&v_idx)) = (node_indices.get(&u.id), node_indices.get(&v.id)) {
                            graph.add_edge(u_idx, v_idx, EdgeTag::Overwrite);
                        }
                    }
                },
                
                // AddEdge → EditEdge (fromが一致かつlabelが一致なら順序あり)
                (OpKind::AddEdge { from: from1, label: label1, .. },
                 OpKind::EditEdgeReference { from: from2, label: label2, .. }) => {
                    if from1 == from2 && label1 == label2 {
                        if let (Some(&u_idx), Some(&v_idx)) = (node_indices.get(&u.id), node_indices.get(&v.id)) {
                            graph.add_edge(u_idx, v_idx, EdgeTag::Overwrite);
                        }
                    }
                },
                
                // EditEdge → EditEdge (fromが一致かつlabelが一致なら順序あり)
                (OpKind::EditEdgeReference { from: from1, label: label1, .. }, 
                 OpKind::EditEdgeReference { from: from2, label: label2, .. }) => {
                    if from1 == from2 && label1 == label2 {
                        if let (Some(&u_idx), Some(&v_idx)) = (node_indices.get(&u.id), node_indices.get(&v.id)) {
                            graph.add_edge(u_idx, v_idx, EdgeTag::Overwrite);
                        }
                    }
                },
                
                // AddVariable → AddVariable (labelが一致するなら順序あり)
                (OpKind::AddVariable { label: label1, .. }, 
                 OpKind::AddVariable { label: label2, .. }) => {
                    if label1 == label2 {
                        if let (Some(&u_idx), Some(&v_idx)) = (node_indices.get(&u.id), node_indices.get(&v.id)) {
                            graph.add_edge(u_idx, v_idx, EdgeTag::Overwrite);
                        }
                    }
                },
                
                // AddVariable → EditVariable (labelが一致するなら順序あり)
                (OpKind::AddVariable { label: label1, .. },
                 OpKind::EditVariableReference { label: label2, .. }) => {
                    if label1 == label2 {
                        if let (Some(&u_idx), Some(&v_idx)) = (node_indices.get(&u.id), node_indices.get(&v.id)) {
                            graph.add_edge(u_idx, v_idx, EdgeTag::Overwrite);
                        }
                    }
                },
                
                // EditVariable → EditVariable (labelが一致するなら順序あり)
                (OpKind::EditVariableReference { label: label1, .. }, 
                 OpKind::EditVariableReference { label: label2, .. }) => {
                    if label1 == label2 {
                        if let (Some(&u_idx), Some(&v_idx)) = (node_indices.get(&u.id), node_indices.get(&v.id)) {
                            graph.add_edge(u_idx, v_idx, EdgeTag::Overwrite);
                        }
                    }
                },
                
                // その他の依存関係は順序なし（独立）
                _ => {}
            }
        }
    }
    
    (graph, node_indices)
}

/// 操作が特定のIDを参照しているかを判定
fn references_id(op: &Op, id: &OpId) -> bool {
    match &op.kind {
        OpKind::AddNode { .. } => false,
        OpKind::EditNode { id: node_id, .. } => node_id == id,
        OpKind::DeleteNode { id: node_id } => node_id == id,
        OpKind::AddEdge { from, to, .. } => from == id || to == id,
        OpKind::EditEdgeReference { from, old_to, new_to, .. } => 
            from == id || old_to == id || new_to == id,
        OpKind::EditEdgeLabel { from, to, .. } => from == id || to == id,
        OpKind::DeleteEdge { from, to, .. } => from == id || to == id,
        OpKind::AddVariable { to, .. } => to == id,
        OpKind::EditVariableReference { old_to, new_to, .. } => 
            old_to.as_ref().map_or(false, |ot| ot == id) || new_to == id,
        OpKind::EditVariableLabel { to, .. } => to == id,
        OpKind::DeleteVariable { to, .. } => to == id,
    }
}

/// 正規トポロジカル順序を計算
pub fn canonical_order(ops: &[Op]) -> Vec<OpId> {
    let (graph, node_indices) = build_graph(ops);
    let mut result = Vec::new();
    let mut in_degree = HashMap::new();
    
    // 入次数を計算
    for node_idx in graph.node_indices() {
        let node_id = graph.node_weight(node_idx).unwrap();
        in_degree.insert(node_id.clone(), 0);
    }
    
    for edge in graph.edge_indices() {
        let (_, target) = graph.edge_endpoints(edge).unwrap();
        let target_id = graph.node_weight(target).unwrap();
        *in_degree.entry(target_id.clone()).or_insert(0) += 1;
    }
    
    // 入次数0のノードを優先度付きキューに入れる
    let mut queue = std::collections::BinaryHeap::new();
    for (node_id, &degree) in &in_degree {
        if degree == 0 {
            queue.push(std::cmp::Reverse(node_id.clone()));
        }
    }
    
    // Kahn のアルゴリズムでトポロジカルソート
    while let Some(std::cmp::Reverse(node_id)) = queue.pop() {
        result.push(node_id.clone());
        
        if let Some(&node_idx) = node_indices.get(&node_id) {
            let mut outgoing_edges = Vec::new();
            for edge in graph.edges_directed(node_idx, Direction::Outgoing) {
                let target = edge.target();
                let target_id = graph.node_weight(target).unwrap().clone();
                outgoing_edges.push(target_id);
            }
            
            for target_id in outgoing_edges {
                let count = in_degree.get_mut(&target_id).unwrap();
                *count -= 1;
                if *count == 0 {
                    queue.push(std::cmp::Reverse(target_id.clone()));
                }
            }
        }
    }
    
    result
}

/// ホール情報
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Hole {
    /// 定数の差異
    Const { placeholder: String, values: Vec<String> },
    /// 参照の差異
    Ref { placeholder: String, refs: Vec<String> },
}

/// マッチング結果
#[derive(Debug, Clone)]
pub struct MatchResult {
    pub common_ops: Vec<(OpId, OpId)>,
    pub holes: Vec<Hole>,
}

/// 2つの操作グラフをマッチング
pub fn match_graphs(a: &[Op], b: &[Op]) -> MatchResult {
    // まずは操作を正規順序に並べ替え
    let a_order = canonical_order(a);
    let b_order = canonical_order(b);
    
    let mut common_ops = Vec::new();
    let mut holes = Vec::new();
    let mut next_hole_id = 1;
    
    // 簡易マッチング：同じ種類の操作が同じ順序で現れるかを確認
    let mut i = 0;
    let mut j = 0;
    
    while i < a_order.len() && j < b_order.len() {
        let a_op = a.iter().find(|op| op.id == a_order[i]).unwrap();
        let b_op = b.iter().find(|op| op.id == b_order[j]).unwrap();
        
        // 操作の種類が一致するか確認
        if std::mem::discriminant(&a_op.kind) == std::mem::discriminant(&b_op.kind) {
            match (&a_op.kind, &b_op.kind) {
                // AddNode の場合、is_literal と label を比較
                (OpKind::AddNode { is_literal: a_is_lit, label: a_label, .. },
                 OpKind::AddNode { is_literal: b_is_lit, label: b_label, .. }) => {
                    if a_is_lit == b_is_lit {
                        if a_label == b_label {
                            common_ops.push((a_op.id.clone(), b_op.id.clone()));
                        } else {
                            // ラベルが異なる場合、ホールを作成
                            let placeholder = format!("Hole{}", next_hole_id);
                            next_hole_id += 1;
                            holes.push(Hole::Const { 
                                placeholder, 
                                values: vec![a_label.clone(), b_label.clone()] 
                            });
                            common_ops.push((a_op.id.clone(), b_op.id.clone()));
                        }
                    }
                },
                
                // AddEdge の場合、from と label が一致するか確認
                (OpKind::AddEdge { from: a_from, label: a_label, to: a_to, .. },
                 OpKind::AddEdge { from: b_from, label: b_label, to: b_to, .. }) => {
                    if a_from == b_from && a_label == b_label {
                        common_ops.push((a_op.id.clone(), b_op.id.clone()));
                        
                        // to が異なる場合、参照ホールを作成
                        if a_to != b_to {
                            let placeholder = format!("Hole{}", next_hole_id);
                            next_hole_id += 1;
                            holes.push(Hole::Ref { 
                                placeholder, 
                                refs: vec![a_to.clone(), b_to.clone()] 
                            });
                        }
                    }
                },
                
                // EditEdgeReference の場合
                (OpKind::EditEdgeReference { from: a_from, old_to: a_old_to, new_to: a_new_to, label: a_label },
                 OpKind::EditEdgeReference { from: b_from, old_to: b_old_to, new_to: b_new_to, label: b_label }) => {
                    if a_from == b_from && a_label == b_label && a_old_to == b_old_to {
                        common_ops.push((a_op.id.clone(), b_op.id.clone()));
                        
                        // new_to が異なる場合、参照ホールを作成
                        if a_new_to != b_new_to {
                            let placeholder = format!("Hole{}", next_hole_id);
                            next_hole_id += 1;
                            holes.push(Hole::Ref { 
                                placeholder, 
                                refs: vec![a_new_to.clone(), b_new_to.clone()] 
                            });
                        }
                    }
                },
                
                // AddVariable の場合
                (OpKind::AddVariable { to: a_to, label: a_label },
                 OpKind::AddVariable { to: b_to, label: b_label }) => {
                    if a_label == b_label {
                        common_ops.push((a_op.id.clone(), b_op.id.clone()));
                        
                        // to が異なる場合、参照ホールを作成
                        if a_to != b_to {
                            let placeholder = format!("Hole{}", next_hole_id);
                            next_hole_id += 1;
                            holes.push(Hole::Ref { 
                                placeholder, 
                                refs: vec![a_to.clone(), b_to.clone()] 
                            });
                        }
                    }
                },
                
                // EditVariableReference の場合
                (OpKind::EditVariableReference { old_to: a_old_to, new_to: a_new_to, label: a_label },
                 OpKind::EditVariableReference { old_to: b_old_to, new_to: b_new_to, label: b_label }) => {
                    if a_label == b_label && a_old_to == b_old_to {
                        common_ops.push((a_op.id.clone(), b_op.id.clone()));
                        
                        // new_to が異なる場合、参照ホールを作成
                        if a_new_to != b_new_to {
                            let placeholder = format!("Hole{}", next_hole_id);
                            next_hole_id += 1;
                            holes.push(Hole::Ref { 
                                placeholder, 
                                refs: vec![a_new_to.clone(), b_new_to.clone()] 
                            });
                        }
                    }
                },
                
                // その他の操作タイプは必要に応じて追加
                _ => {}
            }
        }
        
        i += 1;
        j += 1;
    }
    
    MatchResult { common_ops, holes }
}

/// 操作列からASTを生成する関数
pub fn generate_ast_from_sorted_ops(ops: &[Op], env: &MemoEnv) -> ast::Program {
    let mut stmts = Vec::new();
    let mut declared_vars = std::collections::HashSet::new();
    
    for op in ops {
        match &op.kind {
            // ノード追加操作の場合（クラスインスタンス作成に対応）
            OpKind::AddNode { id, is_literal, label } => {
                if !is_literal {
                    // 変数名を取得、存在しない場合はIDをそのまま使用
                    let var_name = env.get_name_by_id(id).cloned().unwrap_or_else(|| format!("obj_{}", id));
                    
                    // 既に宣言済みの変数は再宣言しない
                    if !declared_vars.contains(&var_name) {
                        declared_vars.insert(var_name.clone());
                        stmts.push(ast::Stmt::VarDecl {
                            name: var_name,
                            expr: ast::Expr::New(label.clone()),
                        });
                    }
                }
            },
            
            // エッジ追加操作の場合（プロパティ代入に対応）
            OpKind::AddEdge { from, to, label } => {
                let from_lhs = get_lhs_for_id(from, env);
                let to_expr = get_expr_for_id(to, env);
                
                let lhs = ast::Lhs::ObjAccess(
                    Box::new(from_lhs),
                    label.clone()
                );
                
                stmts.push(ast::Stmt::Assign {
                    lhs,
                    expr: to_expr,
                });
            },
            
            // エッジ参照先変更操作の場合
            OpKind::EditEdgeReference { from, old_to: _, new_to, label } => {
                let from_lhs = get_lhs_for_id(from, env);
                let new_to_expr = get_expr_for_id(new_to, env);
                
                let lhs = ast::Lhs::ObjAccess(
                    Box::new(from_lhs),
                    label.clone()
                );
                
                stmts.push(ast::Stmt::Assign {
                    lhs,
                    expr: new_to_expr,
                });
            },
            
            // 変数追加操作の場合（変数宣言に対応）
            OpKind::AddVariable { to, label } => {
                let var_name = label.clone();
                let to_expr = get_expr_for_id(to, env);
                
                // 変数がまだ宣言されていない場合は宣言する
                if !declared_vars.contains(&var_name) {
                    declared_vars.insert(var_name.clone());
                    stmts.push(ast::Stmt::VarDecl {
                        name: var_name,
                        expr: to_expr,
                    });
                } else {
                    // 再代入の場合
                    stmts.push(ast::Stmt::Assign {
                        lhs: ast::Lhs::Var(var_name),
                        expr: to_expr,
                    });
                }
            },
            
            // 変数参照先変更操作の場合
            OpKind::EditVariableReference { old_to: _, new_to, label } => {
                let var_name = label.clone();
                let new_to_expr = get_expr_for_id(new_to, env);
                
                stmts.push(ast::Stmt::Assign {
                    lhs: ast::Lhs::Var(var_name),
                    expr: new_to_expr,
                });
            },
            
            // その他の操作タイプ
            _ => {}
        }
    }
    
    ast::Program { stmts }
}

// IDから左辺式を取得する関数
fn get_lhs_for_id(id: &str, env: &MemoEnv) -> ast::Lhs {
    let var_name = env.get_name_by_id(id).cloned().unwrap_or_else(|| format!("obj_{}", id));
    
    if var_name == "this" {
        ast::Lhs::This
    } else {
        ast::Lhs::Var(var_name)
    }
}

// IDから式を取得する関数
fn get_expr_for_id(id: &str, env: &MemoEnv) -> ast::Expr {
    let var_name = env.get_name_by_id(id).cloned().unwrap_or_else(|| format!("obj_{}", id));
    
    if var_name == "this" {
        ast::Expr::This
    } else {
        ast::Expr::Var(var_name)
    }
}

/// 操作列を依存関係に基づいて並び替え、ASTに変換する関数
pub fn find_common_pattern_from_operations(
    operations_list: &[Vec<serde_json::Value>],
    memo_envs: &[MemoEnv],
) -> (Option<ast::Program>, HashMap<String, Vec<String>>) {
    if operations_list.len() < 2 {
        if operations_list.len() == 1 && memo_envs.len() >= 1 {
            // 1つだけの場合は、そのまま処理
            let ops = &operations_list[0];
            match convert_operations_to_ir(ops) {
                Ok(ir_ops) => {
                    // 操作の依存関係に基づいてソート
                    let canonical_ids = canonical_order(&ir_ops);
                    let mut sorted_ops = Vec::new();
                    
                    for id in canonical_ids {
                        if let Some(op) = ir_ops.iter().find(|o| o.id == id) {
                            sorted_ops.push(op.clone());
                        }
                    }
                    
                    // ソートされた操作列からASTを生成
                    let program = generate_ast_from_sorted_ops(&sorted_ops, &memo_envs[0]);
                    return (Some(program), HashMap::new());
                },
                Err(e) => {
                    eprintln!("Failed to convert operations to IR: {}", e);
                    return (None, HashMap::new());
                }
            }
        }
        
        return (None, HashMap::new());
    }
    
    let mut ir_ops_list = Vec::new();
    
    // 各操作ログをIRに変換
    for ops in operations_list {
        match convert_operations_to_ir(ops) {
            Ok(ir_ops) => ir_ops_list.push(ir_ops),
            Err(e) => {
                eprintln!("Failed to convert operations to IR: {}", e);
                return (None, HashMap::new());
            }
        }
    }
    
    // 各操作列をトポロジカルソート
    let mut sorted_ops_list = Vec::new();
    for ir_ops in &ir_ops_list {
        let canonical_ids = canonical_order(ir_ops);
        let mut sorted_ops = Vec::new();
        
        for id in canonical_ids {
            if let Some(op) = ir_ops.iter().find(|o| o.id == id) {
                sorted_ops.push(op.clone());
            }
        }
        
        sorted_ops_list.push(sorted_ops);
    }
    
    // 各操作列からASTを生成
    let mut sorted_programs = Vec::new();
    for (i, sorted_ops) in sorted_ops_list.iter().enumerate() {
        if i < memo_envs.len() {
            let program = generate_ast_from_sorted_ops(sorted_ops, &memo_envs[i]);
            sorted_programs.push(program);
        }
    }
    
    // 最初のプログラムを返す（ホール抽出は行わない）
    if let Some(first_program) = sorted_programs.first() {
        return (Some(first_program.clone()), HashMap::new());
    }
    
    (None, HashMap::new())
}
