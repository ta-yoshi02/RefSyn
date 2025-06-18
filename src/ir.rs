use petgraph::graph::{DiGraph, NodeIndex};
use petgraph::Direction;
use petgraph::visit::EdgeRef;
use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use crate::ast;
use crate::env::MemoEnv;
use crate::convert_operations_to_ir;

/// 操作ID（各操作を一意に識別）
pub type OpId = String;

/// ノードID（グラフ内のオブジェクト/値を識別）
pub type NodeId = String;

/// 操作の種類
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub enum OpKind {
    // ノード操作
    AddNode { id: NodeId, is_literal: bool, label: String },
    EditNode { id: NodeId, is_literal: bool, label: String },
    DeleteNode { id: NodeId },
    
    // エッジ操作
    AddEdge { from: NodeId, to: NodeId, label: String },
    EditEdgeReference { from: NodeId, old_to: NodeId, new_to: NodeId, label: String },
    EditEdgeLabel { from: NodeId, to: NodeId, old_label: String, new_label: String },
    DeleteEdge { from: NodeId, to: NodeId, label: String },
    
    // 変数操作
    AddVariable { to: NodeId, label: String },
    EditVariableReference { old_to: NodeId, new_to: NodeId, label: String },
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
                
                // 他の一致ケースは上でカバー済み
                
                // その他の依存関係は順序なし（独立）
                _ => {}
            }
        }
    }
    
    (graph, node_indices)
}

/// 操作が特定のNodeIDを参照しているかを判定
fn references_id(op: &Op, node_id: &NodeId) -> bool {
    match &op.kind {
        OpKind::AddNode { .. } => false,
        OpKind::EditNode { id, .. } => id == node_id,
        OpKind::DeleteNode { id } => id == node_id,
        OpKind::AddEdge { from, to, .. } => from == node_id || to == node_id,
        OpKind::EditEdgeReference { from, old_to, new_to, .. } => 
            from == node_id || old_to == node_id || new_to == node_id,
        OpKind::EditEdgeLabel { from, to, .. } => from == node_id || to == node_id,
        OpKind::DeleteEdge { from, to, .. } => from == node_id || to == node_id,
        OpKind::AddVariable { to, .. } => to == node_id,
        OpKind::EditVariableReference { old_to, new_to, .. } => 
            old_to == node_id || new_to == node_id,
        OpKind::EditVariableLabel { to, .. } => to == node_id,
        OpKind::DeleteVariable { to, .. } => to == node_id,
    }
}

// 操作の種類からラベル優先度を取得する関数
fn get_label_priority(op: &Op) -> (i32, &str) {
    match &op.kind {
        OpKind::AddNode { .. } => (0, ""),
        OpKind::AddEdge { .. } => (1, ""),
        OpKind::EditEdgeReference { .. } => (1, ""),
        OpKind::AddVariable { .. } => (2, ""),
        OpKind::EditVariableReference { .. } => (2, ""),
        _ => (3, ""),
    }
}

/// 正規トポロジカル順序を計算（ラベルの優先度を考慮）
pub fn canonical_order(ops: &[Op]) -> Vec<OpId> {
    let (graph, node_indices) = build_graph(ops);
    let mut result = Vec::new();
    let mut in_degree = HashMap::new();
    
    // 各操作のIDとインデックスのマッピングを作成
    let op_map: HashMap<&str, &Op> = ops.iter().map(|op| (op.id.as_str(), op)).collect();
    
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
    
    // 入次数0のノードを優先度付きキューに入れる（ラベルによってソート）
    let mut queue = std::collections::BinaryHeap::new();
    for (node_id, &degree) in &in_degree {
        if degree == 0 {
            if let Some(op) = op_map.get(node_id.as_str()) {
                let (priority, label) = get_label_priority(op);
                queue.push(std::cmp::Reverse((priority, label, node_id.clone())));
            } else {
                queue.push(std::cmp::Reverse((999, "", node_id.clone())));
            }
        }
    }
    
    // Kahn のアルゴリズムでトポロジカルソート（ラベル優先度を考慮）
    while let Some(std::cmp::Reverse((_, _, node_id))) = queue.pop() {
        result.push(node_id.clone());
        
        if let Some(&node_idx) = node_indices.get(&node_id) {
            let mut new_candidates = Vec::new();
            
            for edge in graph.edges_directed(node_idx, Direction::Outgoing) {
                let target = edge.target();
                let target_id = graph.node_weight(target).unwrap().clone();
                
                let count = in_degree.get_mut(&target_id).unwrap();
                *count -= 1;
                if *count == 0 {
                    new_candidates.push(target_id);
                }
            }
            
            // 優先度付きキューに新しい候補を追加
            for candidate_id in new_candidates {
                if let Some(op) = op_map.get(candidate_id.as_str()) {
                    let (priority, label) = get_label_priority(op);
                    queue.push(std::cmp::Reverse((priority, label, candidate_id)));
                } else {
                    queue.push(std::cmp::Reverse((999, "", candidate_id)));
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

/// 複数の操作列をマッチングしてパターンを抽出するための結果構造体
#[derive(Debug, Clone)]
pub struct MultiMatchResult {
    pub common_patterns: Vec<Op>,
    pub holes: Vec<Hole>,
}

/// 操作を依存関係に基づいて正規化
fn normalize_operations_by_dependencies(ops: &[Op]) -> Vec<Op> {
    let canonical_ids = canonical_order(ops);
    canonical_ids.iter()
        .filter_map(|op_id| ops.iter().find(|op| op.id == *op_id))
        .cloned()
        .collect()
}

/// 複数の操作列をマッチングしてパターンを抽出
fn match_multiple_operation_sequences(ops_list: &[Vec<Op>]) -> MultiMatchResult {
    if ops_list.is_empty() {
        return MultiMatchResult {
            common_patterns: Vec::new(),
            holes: Vec::new(),
        };
    }
    
    eprintln!("=== Starting operation-level matching ===");
    eprintln!("Input operation lists count: {}", ops_list.len());
    
    // 各操作列を依存関係に基づいて正規化
    let mut normalized_ops_list = Vec::new();
    for (list_idx, ops) in ops_list.iter().enumerate() {
        eprintln!("Normalizing operation list {}: {} operations", list_idx, ops.len());
        let normalized = normalize_operations_by_dependencies(ops);
        eprintln!("After normalization: {} operations", normalized.len());
        normalized_ops_list.push(normalized);
    }
    
    // 2つの操作列をペアワイズマッチング（最初の2つのリストを使用）
    if normalized_ops_list.len() >= 2 {
        let match_result = match_graphs(&normalized_ops_list[0], &normalized_ops_list[1]);
        
        // マッチング結果から共通パターンを抽出
        let mut common_patterns = Vec::new();
        for (a_id, _b_id) in &match_result.common_ops {
            if let Some(op) = normalized_ops_list[0].iter().find(|o| o.id == *a_id) {
                common_patterns.push(op.clone());
            }
        }
        
        MultiMatchResult {
            common_patterns,
            holes: match_result.holes,
        }
    } else {
        // 1つのリストしかない場合はそのまま返す
        MultiMatchResult {
            common_patterns: normalized_ops_list.into_iter().next().unwrap_or_default(),
            holes: Vec::new(),
        }
    }
}

/// マッチング結果からホール情報を抽出し、重複を統合、番号を正規化
fn extract_holes_from_match_result(match_result: &MultiMatchResult) -> HashMap<String, Vec<String>> {
    let mut hole_map = HashMap::new();
    for hole in &match_result.holes {
        match hole {
            Hole::Const { placeholder, values } => {
                hole_map.insert(format!("{} (const)", placeholder), values.clone());
            },
            Hole::Ref { placeholder, refs } => {
                hole_map.insert(format!("{} (ref)", placeholder), refs.clone());
            }
        }
    }
    hole_map
}

/// マッチング結果からテンプレートASTを生成
fn generate_template_ast(match_result: &MultiMatchResult, _env: &MemoEnv) -> ast::Program {
    let mut stmts = Vec::new();
    let mut declared_vars = std::collections::HashSet::new();
    let mut var_counter = 0;
    
    // ID -> 正規化された変数名のマッピングを作成
    let mut id_to_var = HashMap::new();
    
    // エッジの関係性を追跡するマップ (from_id -> (property_name -> to_id))
    let mut edge_map: HashMap<String, HashMap<String, String>> = HashMap::new();
    
    // ID -> ラベル値のマッピングを作成（ホール検出用）
    let mut id_to_label = HashMap::new();
    for op in &match_result.common_patterns {
        if let OpKind::AddNode { id, label, .. } = &op.kind {
            id_to_label.insert(id.clone(), label.clone());
        }
        // エッジ関係を記録
        if let OpKind::AddEdge { from, to, label } = &op.kind {
            edge_map.entry(from.clone())
                .or_insert_with(HashMap::new)
                .insert(label.clone(), to.clone());
        }
    }
    
    // まずホールマッピングを作成し、ホール内のIDを統一的に扱う
    let mut hole_to_ids: HashMap<String, Vec<String>> = HashMap::new();
    let mut id_to_hole: HashMap<String, String> = HashMap::new();
    
    // ホール情報を解析
    for hole in &match_result.holes {
        match hole {
            Hole::Const { placeholder, values } => {
                // 定数ホールの場合、値に対応するIDを見つける
                for (id, label) in &id_to_label {
                    if values.contains(label) {
                        id_to_hole.insert(id.clone(), placeholder.clone());
                    }
                }
            },
            Hole::Ref { placeholder, refs } => {
                hole_to_ids.insert(placeholder.clone(), refs.clone());
                for ref_id in refs {
                    id_to_hole.insert(ref_id.clone(), placeholder.clone());
                }
            }
        }
    }
    
    // 既知のIDを収集して正規化（リテラルIDは除外）
    let mut all_ids = std::collections::HashSet::new();
    let mut literal_ids = std::collections::HashSet::new();
    
    for op in &match_result.common_patterns {
        match &op.kind {
            OpKind::AddNode { id, is_literal, .. } => {
                if *is_literal {
                    literal_ids.insert(id.clone());
                } else {
                    all_ids.insert(id.clone());
                }
            },
            OpKind::AddEdge { from, to, .. } => {
                all_ids.insert(from.clone());
                // toがリテラルでない場合のみ追加
                if !literal_ids.contains(to) {
                    all_ids.insert(to.clone());
                }
            },
            OpKind::EditEdgeReference { from, new_to, .. } => {
                all_ids.insert(from.clone());
                if !literal_ids.contains(new_to) {
                    all_ids.insert(new_to.clone());
                }
            },
            OpKind::AddVariable { to, .. } => {
                if !literal_ids.contains(to) {
                    all_ids.insert(to.clone());
                }
            },
            OpKind::EditVariableReference { new_to, .. } => {
                if !literal_ids.contains(new_to) {
                    all_ids.insert(new_to.clone());
                }
            },
            _ => {}
        }
    }
    
    // IDを決定論的な順序でソートし、正規化された変数名にマッピング
    let mut sorted_ids: Vec<String> = all_ids.into_iter().collect();
    sorted_ids.sort();
    
    // 特別なIDを最初に処理し、その他のIDを順番に変数番号を割り当て
    for id in &sorted_ids {
        if id == "this" || id.contains("main-new") {
            id_to_var.insert(id.clone(), "this".to_string());
        } else if !id_to_var.contains_key(id) {
            // ホールに属しているかチェック - ただし、変数名のマッピングではホールは使わない
            // ホールは式の値の部分のみで使用される
            let var_name = format!("obj_{}", var_counter);
            id_to_var.insert(id.clone(), var_name.clone());
            var_counter += 1;
        }
    }
    
    // ホール番号を正規化するマッピングを作成
    let mut old_to_new_hole = HashMap::new();
    let mut hole_counter = 1;
    for hole in &match_result.holes {
        let old_placeholder = match hole {
            Hole::Const { placeholder, .. } => placeholder,
            Hole::Ref { placeholder, .. } => placeholder,
        };
        
        if !old_to_new_hole.contains_key(old_placeholder) {
            let new_placeholder = format!("Hole{}", hole_counter);
            old_to_new_hole.insert(old_placeholder.clone(), new_placeholder);
            hole_counter += 1;
        }
    }
    
    for op in &match_result.common_patterns {
        match &op.kind {
            OpKind::AddNode { id, is_literal, label, .. } => {
                if !is_literal {
                    let var_name = id_to_var.get(id)
                        .cloned()
                        .unwrap_or_else(|| format!("v{}", var_counter));
                    
                    if !declared_vars.contains(&var_name) && var_name != "this" && !var_name.starts_with("Hole") {
                        declared_vars.insert(var_name.clone());
                        stmts.push(ast::Stmt::VarDecl {
                            name: var_name,
                            expr: ast::Expr::New(label.clone()),
                        });
                    }
                }
            },
            
            OpKind::AddEdge { from, to, label } => {
                let from_lhs = get_normalized_lhs_for_edge_with_holes(
                    from, 
                    &id_to_var, 
                    &match_result.holes, 
                    &old_to_new_hole,
                    &edge_map
                );
                let to_expr = get_normalized_expr_for_id_with_mapping_and_holes(to, &id_to_var, &match_result.holes, &old_to_new_hole, &id_to_hole);
                
                let lhs = ast::Lhs::ObjAccess(
                    Box::new(from_lhs),
                    label.clone()
                );
                
                stmts.push(ast::Stmt::Assign {
                    lhs,
                    expr: to_expr,
                });
            },
            
            OpKind::AddVariable { to, label } => {
                let var_name = label.clone();
                let to_expr = get_normalized_expr_for_id_with_mapping_and_holes(to, &id_to_var, &match_result.holes, &old_to_new_hole, &id_to_hole);
                
                if !declared_vars.contains(&var_name) {
                    declared_vars.insert(var_name.clone());
                    stmts.push(ast::Stmt::VarDecl {
                        name: var_name,
                        expr: to_expr,
                    });
                }
            },
            
            OpKind::EditEdgeReference { from, new_to, label, .. } => {
                let from_lhs = get_normalized_lhs_for_edge_with_holes(
                    from, 
                    &id_to_var, 
                    &match_result.holes, 
                    &old_to_new_hole,
                    &edge_map
                );
                let to_expr = get_normalized_expr_for_id_with_mapping_and_holes(new_to, &id_to_var, &match_result.holes, &old_to_new_hole, &id_to_hole);
                
                let lhs = ast::Lhs::ObjAccess(
                    Box::new(from_lhs),
                    label.clone()
                );
                
                stmts.push(ast::Stmt::Assign {
                    lhs,
                    expr: to_expr,
                });
            },
            
            // 他の操作タイプも同様に処理
            _ => {}
        }
    }
    
    ast::Program { stmts }
}

/// IDマッピングとホール情報を考慮した正規化された式を取得（ホール番号正規化対応）
fn get_normalized_expr_for_id_with_mapping_and_holes(
    id: &str, 
    id_mapping: &HashMap<String, String>,
    holes: &[Hole],
    hole_mapping: &HashMap<String, String>,
    id_to_hole: &HashMap<String, String>
) -> ast::Expr {
    // まずid_to_holeをチェック
    if let Some(hole_name) = id_to_hole.get(id) {
        if let Some(new_placeholder) = hole_mapping.get(hole_name) {
            return ast::Expr::Hole(new_placeholder.clone());
        } else {
            return ast::Expr::Hole(hole_name.clone());
        }
    }
    
    // 直接ホールに含まれるかチェック（定数値のホールか参照のホールか）
    for hole in holes {
        match hole {
            Hole::Const { placeholder, values } => {
                if values.contains(&id.to_string()) {
                    // 正規化されたホール名を使用
                    if let Some(new_placeholder) = hole_mapping.get(placeholder) {
                        return ast::Expr::Hole(new_placeholder.clone());
                    } else {
                        return ast::Expr::Hole(placeholder.clone());
                    }
                }
            },
            Hole::Ref { placeholder: _, refs } => {
                if refs.contains(&id.to_string()) {
                    // 参照型のホールの場合：
                    // エッジのtoの場合は、通常の変数マッピングを使用する
                    // （なぜなら、this.next = obj_0 という形になるべきだから）
                    break;
                }
            }
        }
    }
    
    // ホールに含まれない場合、または参照型ホールの場合は通常の処理
    if let Some(normalized_name) = id_mapping.get(id) {
        if normalized_name == "this" {
            ast::Expr::This
        } else {
            ast::Expr::Var(normalized_name.clone())
        }
    } else {
        // id_mappingにない場合、ホールを再チェック
        for hole in holes {
            match hole {
                Hole::Const { placeholder, values } => {
                    if values.contains(&id.to_string()) {
                        if let Some(new_placeholder) = hole_mapping.get(placeholder) {
                            return ast::Expr::Hole(new_placeholder.clone());
                        } else {
                            return ast::Expr::Hole(placeholder.clone());
                        }
                    }
                },
                _ => continue,
            }
        }
        
        if id == "this" || id.contains("main-new") {
            ast::Expr::This
        } else {
            ast::Expr::Var(format!("v_{}", id))
        }
    }
}

/// IDマッピングとホール情報を考慮した正規化された左辺式を取得（ホール番号正規化対応）
/// 2つのIDが構造的に同等かを判定する関数
fn is_structurally_equivalent(
    id_a: &str, 
    id_b: &str, 
    a_ops: &[Op], 
    b_ops: &[Op], 
    id_mapping: &HashMap<String, String>
) -> bool {
    // 同じIDの場合は同等
    if id_a == id_b {
        return true;
    }
    
    // 特別なケース：main-new（既存オブジェクト）と__temp（新規作成オブジェクト）は
    // 構造的に同じでも異なるものとして扱う
    let a_is_existing = id_a.contains("main-new");
    let b_is_existing = id_b.contains("main-new");
    let a_is_temp = id_a.contains("__temp");
    let b_is_temp = id_b.contains("__temp");
    
    if (a_is_existing && b_is_temp) || (a_is_temp && b_is_existing) {
        return false;
    }
    
    // IDマッピングで対応している場合は同等とみなす
    if let Some(mapped_id) = id_mapping.get(id_a) {
        if mapped_id == id_b {
            return true;
        }
    }
    
    // 逆方向の確認
    if let Some(mapped_id) = id_mapping.get(id_b) {
        if mapped_id == id_a {
            return true;
        }
    }
    
    // 両方のIDに対応するAddNode操作を見つける
    let a_node = a_ops.iter().find(|op| {
        if let OpKind::AddNode { id, .. } = &op.kind {
            id == id_a
        } else {
            false
        }
    });
    
    let b_node = b_ops.iter().find(|op| {
        if let OpKind::AddNode { id, .. } = &op.kind {
            id == id_b
        } else {
            false
        }
    });
    
    match (a_node, b_node) {
        (Some(a_op), Some(b_op)) => {
            if let (OpKind::AddNode { is_literal: a_is_lit, label: a_label, .. },
                    OpKind::AddNode { is_literal: b_is_lit, label: b_label, .. }) = (&a_op.kind, &b_op.kind) {
                // 構造的に同じ：is_literalとlabelが一致
                a_is_lit == b_is_lit && a_label == b_label
            } else {
                false
            }
        },
        _ => false
    }
}

pub fn match_graphs(a: &[Op], b: &[Op]) -> MatchResult {
    eprintln!("=== match_graphs called ===");
    eprintln!("Operations list A has {} operations:", a.len());
    for (i, op) in a.iter().enumerate() {
        eprintln!("  A[{}]: {:?}", i, op);
    }
    eprintln!("Operations list B has {} operations:", b.len());
    for (i, op) in b.iter().enumerate() {
        eprintln!("  B[{}]: {:?}", i, op);
    }
    
    // まずは操作を正規順序に並べ替え
    let a_order = canonical_order(a);
    let b_order = canonical_order(b);
    
    eprintln!("Canonical order A: {:?}", a_order);
    eprintln!("Canonical order B: {:?}", b_order);
    
    let mut common_ops = Vec::new();
    let mut holes = Vec::new();
    let mut next_hole_id = 1;
    
    // ID同等性のマッピングを追跡するため
    let mut id_equivalence = HashMap::new();
    // 既に作成されたホールのマッピング（同じペアに対して重複を避ける）
    let mut created_holes: HashMap<(String, String), String> = HashMap::new();
    
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
                (OpKind::AddNode { id: a_id, is_literal: a_is_lit, label: a_label },
                 OpKind::AddNode { id: b_id, is_literal: b_is_lit, label: b_label }) => {
                    if a_is_lit == b_is_lit {
                        if a_label == b_label {
                            common_ops.push((a_op.id.clone(), b_op.id.clone()));
                            // 構造的に同等なIDをマッピングに記録
                            id_equivalence.insert(a_id.clone(), b_id.clone());
                        } else {
                            // ラベルが異なる場合、ホールを作成
                            let placeholder = format!("Hole{}", next_hole_id);
                            next_hole_id += 1;
                            holes.push(Hole::Const { 
                                placeholder: placeholder.clone(), 
                                values: vec![a_label.clone(), b_label.clone()] 
                            });
                            common_ops.push((a_op.id.clone(), b_op.id.clone()));
                            // 構造的に同等なIDをマッピングに記録
                            id_equivalence.insert(a_id.clone(), b_id.clone());
                        }
                    }
                },
                
                // AddEdge の場合、from と label が一致するか確認
                (OpKind::AddEdge { from: a_from, label: a_label, to: a_to },
                 OpKind::AddEdge { from: b_from, label: b_label, to: b_to }) => {
                    if a_label == b_label {
                        common_ops.push((a_op.id.clone(), b_op.id.clone()));
                        
                        // from が構造的に異なる場合、参照ホールを作成
                        let should_create_from_hole = !is_structurally_equivalent(a_from, b_from, a, b, &id_equivalence);
                        
                        if should_create_from_hole {
                            let hole_key = (a_from.clone(), b_from.clone());
                            if !created_holes.contains_key(&hole_key) {
                                let placeholder = format!("Hole{}", next_hole_id);
                                next_hole_id += 1;
                                holes.push(Hole::Ref { 
                                    placeholder: placeholder.clone(), 
                                    refs: vec![a_from.clone(), b_from.clone()] 
                                });
                                created_holes.insert(hole_key, placeholder.clone());
                            }
                        }
                        
                        // to が構造的に異なる場合のみ、参照ホールを作成
                        if !is_structurally_equivalent(a_to, b_to, a, b, &id_equivalence) {
                            let hole_key = (a_to.clone(), b_to.clone());
                            if !created_holes.contains_key(&hole_key) {
                                let placeholder = format!("Hole{}", next_hole_id);
                                next_hole_id += 1;
                                holes.push(Hole::Ref { 
                                    placeholder: placeholder.clone(), 
                                    refs: vec![a_to.clone(), b_to.clone()] 
                                });
                                created_holes.insert(hole_key, placeholder);
                            }
                        }
                    }
                },
                
                // EditEdgeReference の場合
                (OpKind::EditEdgeReference { from: a_from, old_to: a_old_to, new_to: a_new_to, label: a_label },
                 OpKind::EditEdgeReference { from: b_from, old_to: b_old_to, new_to: b_new_to, label: b_label }) => {
                    // labelが同じであれば共通パターンとして認識
                    if a_label == b_label {
                        common_ops.push((a_op.id.clone(), b_op.id.clone()));
                        
                        // from が構造的に異なる場合、参照ホールを作成
                        if !is_structurally_equivalent(a_from, b_from, a, b, &id_equivalence) {
                            let hole_key = (a_from.clone(), b_from.clone());
                            if !created_holes.contains_key(&hole_key) {
                                let placeholder = format!("Hole{}", next_hole_id);
                                next_hole_id += 1;
                                holes.push(Hole::Ref { 
                                    placeholder: placeholder.clone(), 
                                    refs: vec![a_from.clone(), b_from.clone()] 
                                });
                                created_holes.insert(hole_key, placeholder);
                            }
                        }
                        
                        // old_to が構造的に異なる場合、参照ホールを作成
                        if !is_structurally_equivalent(a_old_to, b_old_to, a, b, &id_equivalence) {
                            let hole_key = (a_old_to.clone(), b_old_to.clone());
                            if !created_holes.contains_key(&hole_key) {
                                let placeholder = format!("Hole{}", next_hole_id);
                                next_hole_id += 1;
                                holes.push(Hole::Ref { 
                                    placeholder: placeholder.clone(), 
                                    refs: vec![a_old_to.clone(), b_old_to.clone()] 
                                });
                                created_holes.insert(hole_key, placeholder);
                            }
                        }
                        
                        // new_to が構造的に異なる場合、参照ホールを作成
                        if !is_structurally_equivalent(a_new_to, b_new_to, a, b, &id_equivalence) {
                            let hole_key = (a_new_to.clone(), b_new_to.clone());
                            if !created_holes.contains_key(&hole_key) {
                                let placeholder = format!("Hole{}", next_hole_id);
                                next_hole_id += 1;
                                holes.push(Hole::Ref { 
                                    placeholder: placeholder.clone(), 
                                    refs: vec![a_new_to.clone(), b_new_to.clone()] 
                                });
                                created_holes.insert(hole_key, placeholder);
                            }
                        }
                    }
                },
                
                // AddVariable の場合
                (OpKind::AddVariable { to: a_to, label: a_label },
                 OpKind::AddVariable { to: b_to, label: b_label }) => {
                    if a_label == b_label {
                        common_ops.push((a_op.id.clone(), b_op.id.clone()));
                        
                        // to が構造的に異なる場合のみ、参照ホールを作成
                        if !is_structurally_equivalent(a_to, b_to, a, b, &id_equivalence) {
                            let hole_key = (a_to.clone(), b_to.clone());
                            if !created_holes.contains_key(&hole_key) {
                                let placeholder = format!("Hole{}", next_hole_id);
                                next_hole_id += 1;
                                holes.push(Hole::Ref { 
                                    placeholder: placeholder.clone(), 
                                    refs: vec![a_to.clone(), b_to.clone()] 
                                });
                                created_holes.insert(hole_key, placeholder);
                            }
                        }
                    }
                },
                
                // EditVariableReference の場合
                (OpKind::EditVariableReference { old_to: a_old_to, new_to: a_new_to, label: a_label },
                 OpKind::EditVariableReference { old_to: b_old_to, new_to: b_new_to, label: b_label }) => {
                    if a_label == b_label && a_old_to == b_old_to {
                        common_ops.push((a_op.id.clone(), b_op.id.clone()));
                        
                        // new_to が構造的に異なる場合のみ、参照ホールを作成
                        if !is_structurally_equivalent(a_new_to, b_new_to, a, b, &id_equivalence) {
                            let hole_key = (a_new_to.clone(), b_new_to.clone());
                            if !created_holes.contains_key(&hole_key) {
                                let placeholder = format!("Hole{}", next_hole_id);
                                next_hole_id += 1;
                                holes.push(Hole::Ref { 
                                    placeholder: placeholder.clone(), 
                                    refs: vec![a_new_to.clone(), b_new_to.clone()] 
                                });
                                created_holes.insert(hole_key, placeholder);
                            }
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
    
    eprintln!("Debug: Final common_ops: {:?}", common_ops);
    eprintln!("Debug: Final holes: {:?}", holes);
    MatchResult { common_ops, holes }
}

/// 操作列からASTを生成する関数
pub fn generate_ast_from_sorted_ops(ops: &[Op], env: &MemoEnv) -> ast::Program {
    eprintln!("=== generate_ast_from_sorted_ops called with {} operations ===", ops.len());
    let mut stmts = Vec::new();
    
    // 操作から使用されるIDを収集し、適切な変数名マッピングを作成
    let mut local_env = env.clone();
    let mut id_counter = 1;
    
    // main-new1 を this にマッピング
    local_env.add_special_mapping("main-new1".to_string(), "this".to_string());
    
    for op in ops {
        match &op.kind {
            OpKind::AddNode { id, .. } => {
                if id.starts_with("main-new") && id != "main-new1" {
                    let var_name = format!("node{}", id_counter);
                    local_env.add_special_mapping(id.clone(), var_name);
                    id_counter += 1;
                } else if id.starts_with("__temp") {
                    let var_name = format!("temp{}", id_counter);
                    local_env.add_special_mapping(id.clone(), var_name);
                    id_counter += 1;
                }
            },
            OpKind::EditEdgeReference { from, new_to, .. } => {
                // fromとnew_toが未マッピングの場合にマッピングを追加
                if local_env.get_name_by_id(from).is_none() && from != "main-new1" {
                    let var_name = format!("node{}", id_counter);
                    local_env.add_special_mapping(from.clone(), var_name);
                    id_counter += 1;
                }
                if local_env.get_name_by_id(new_to).is_none() && new_to != "main-new1" && new_to != "undefined" {
                    let var_name = format!("node{}", id_counter);
                    local_env.add_special_mapping(new_to.clone(), var_name);
                    id_counter += 1;
                }
            },
            _ => {}
        }
    }
    let mut declared_vars = std::collections::HashSet::new();
    
    for (i, op) in ops.iter().enumerate() {
        eprintln!("Processing operation {}: {:?}", i, op);
        match &op.kind {
            // ノード追加操作の場合（クラスインスタンス作成に対応）
            OpKind::AddNode { id, is_literal, label } => {
                if !is_literal {
                    // 変数名を取得、存在しない場合はIDをそのまま使用
                    let var_name = local_env.get_name_by_id(id).cloned().unwrap_or_else(|| format!("obj_{}", id));
                    
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
                let from_lhs = get_lhs_for_id(from, &local_env);
                let to_expr = get_expr_for_id(to, &local_env);
                
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
            OpKind::EditEdgeReference { from, new_to, label, .. } => {
                let from_lhs = get_lhs_for_id(from, &local_env);
                let new_to_expr = if new_to == "undefined" {
                    ast::Expr::Literal("null".to_string())
                } else {
                    get_expr_for_id(new_to, &local_env)
                };
                
                let lhs = ast::Lhs::ObjAccess(
                    Box::new(from_lhs),
                    label.clone()
                );
                
                stmts.push(ast::Stmt::Assign {
                    lhs,
                    expr: new_to_expr,
                });
                eprintln!("Generated AST assignment statement for EditEdgeReference: {}.{} = {:?}", from, label, new_to);
            },
            
            // 変数追加操作の場合（変数宣言に対応）
            OpKind::AddVariable { to, label } => {
                let var_name = label.clone();
                let to_expr = get_expr_for_id(to, &local_env);
                
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
                let new_to_expr = get_expr_for_id(new_to, &local_env);
                
                stmts.push(ast::Stmt::Assign {
                    lhs: ast::Lhs::Var(var_name),
                    expr: new_to_expr,
                });
            },
            
            // ノード削除操作（変数削除またはプロパティ削除として処理）
            OpKind::DeleteNode { id } => {
                // ノード削除は明示的に表現しない（削除されたノードは参照されなくなるだけ）
                eprintln!("Warning: DeleteNode operation for {} not explicitly handled in AST", id);
            },
            
            // エッジ削除操作（プロパティをnullまたは未定義にセット）
            OpKind::DeleteEdge { from, to: _, label } => {
                let from_lhs = get_lhs_for_id(from, &local_env);
                
                let lhs = ast::Lhs::ObjAccess(
                    Box::new(from_lhs),
                    label.clone()
                );
                
                // プロパティをnullにセット
                stmts.push(ast::Stmt::Assign {
                    lhs,
                    expr: ast::Expr::Literal("null".to_string()),
                });
                eprintln!("Generated AST assignment to null for DeleteEdge");
            },
            
            // ノード編集操作（変数の値の変更）
            OpKind::EditNode { id, is_literal, label } => {
                if *is_literal {
                    // リテラル値の変更は通常は直接表現されない
                    eprintln!("Warning: EditNode literal operation for {} not handled in AST", id);
                } else {
                    // 非リテラル値の変更は再代入として処理
                    let var_name = local_env.get_name_by_id(id).cloned().unwrap_or_else(|| format!("obj_{}", id));
                    stmts.push(ast::Stmt::Assign {
                        lhs: ast::Lhs::Var(var_name),
                        expr: ast::Expr::New(label.clone()),
                    });
                }
            },
            
            // その他の操作タイプ
            _ => {}
        }
    }
    
    eprintln!("Final AST program has {} statements", stmts.len());
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
    eprintln!("=== find_common_pattern_from_operations called with {} operation lists ===", operations_list.len());
    
    if operations_list.len() < 2 {
        if operations_list.len() == 1 && memo_envs.len() >= 1 {
            // 1つだけの場合は、そのまま処理
            let ops = &operations_list[0];
            eprintln!("Processing single operation list with {} operations", ops.len());
            for (i, op) in ops.iter().enumerate() {
                eprintln!("Operation {}: {:?}", i, op);
            }
            
            match convert_operations_to_ir(ops) {
                Ok(ir_ops) => {
                    eprintln!("Converted to {} IR operations:", ir_ops.len());
                    for (i, ir_op) in ir_ops.iter().enumerate() {
                        eprintln!("IR Operation {}: {:?}", i, ir_op);
                    }
                    
                    // 操作の依存関係に基づいてソート
                    let canonical_ids = canonical_order(&ir_ops);
                    eprintln!("Canonical order: {:?}", canonical_ids);
                    
                    let mut sorted_ops = Vec::new();
                    
                    for id in canonical_ids {
                        if let Some(op) = ir_ops.iter().find(|o| o.id == id) {
                            sorted_ops.push(op.clone());
                        }
                    }
                    
                    eprintln!("Sorted {} operations", sorted_ops.len());
                    
                    // ソートされた操作列からASTを生成
                    let program = generate_ast_from_sorted_ops(&sorted_ops, &memo_envs[0]);
                    eprintln!("Generated AST with {} statements", program.stmts.len());
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
    for (i, ops) in operations_list.iter().enumerate() {
        eprintln!("Converting operation list {} with {} operations", i, ops.len());
        for (j, op) in ops.iter().enumerate() {
            eprintln!("Operation {}.{}: {:?}", i, j, op);
        }
        
        match convert_operations_to_ir(ops) {
            Ok(ir_ops) => {
                eprintln!("Converted to {} IR operations:", ir_ops.len());
                for (j, ir_op) in ir_ops.iter().enumerate() {
                    eprintln!("IR Operation {}.{}: {:?}", i, j, ir_op);
                }
                ir_ops_list.push(ir_ops);
            },
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
    
    // 複数の操作列をマッチングしてホールを抽出
    if sorted_ops_list.len() >= 2 {
        eprintln!("Starting pattern matching for {} operation lists", sorted_ops_list.len());
        let match_result = match_multiple_operation_sequences(&sorted_ops_list);
        eprintln!("Match result: {} common patterns, {} holes", match_result.common_patterns.len(), match_result.holes.len());
        
        let template_program = generate_template_ast(&match_result, &memo_envs[0]);
        eprintln!("Template program generated with {} statements", template_program.stmts.len());
        
        let holes = extract_holes_from_match_result(&match_result);
        return (Some(template_program), holes);
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

/// エッジのチェーンを考慮して正規化されたLHSを取得する関数
fn get_normalized_lhs_for_edge_with_holes(
    id: &str,
    id_mapping: &HashMap<String, String>,
    holes: &[Hole],
    hole_mapping: &HashMap<String, String>,
    _edge_map: &HashMap<String, HashMap<String, String>>
) -> ast::Lhs {
    // まずホールをチェック
    for hole in holes {
        match hole {
            Hole::Ref { placeholder, refs } => {
                if refs.contains(&id.to_string()) {
                    // 参照型のホールの場合はホールを使用
                    if let Some(new_placeholder) = hole_mapping.get(placeholder) {
                        return ast::Lhs::Hole(new_placeholder.clone());
                    } else {
                        return ast::Lhs::Hole(placeholder.clone());
                    }
                }
            },
            _ => continue,
        }
    }
    
    // ホールに含まれない場合は通常の変数マッピングを使用
    if let Some(normalized_name) = id_mapping.get(id) {
        if normalized_name == "this" {
            ast::Lhs::This
        } else {
            ast::Lhs::Var(normalized_name.clone())
        }
    } else {
        if id == "this" || id.contains("main-new") {
            ast::Lhs::This
        } else {
            ast::Lhs::Var(format!("v_{}", id))
        }
    }
}
