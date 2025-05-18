//! Kanon operations JSON → AST(Program)

use std::collections::HashMap;

use serde::Deserialize;
use serde_json::Value;

use crate::ast::{Expr, Lhs, Program, Stmt};
use crate::env::MemoEnv; // MemoEnv をインポート

/// ============ Kanon Operation ============
/// 受信 JSON をそのまま Deserialize して使う
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Operation {
    pub edit_type: String,

    // どの editType でも出現し得るキーを全部 optional で持つ
    pub id: Option<String>,
    pub label: Option<String>,
    pub from: Option<String>,
    pub to: Option<String>,
    pub new_to: Option<String>, // Added
    pub value: Option<Value>,   // Added for addNode literal, addVariable value
    pub is_literal: Option<bool>, // 既存だが、addNodeでのリテラル判定に引き続き使用
}

// Value から Operation に変換する関数
fn get_operation(val: &Value) -> Option<Operation> {
    serde_json::from_value(val.clone()).ok()
}

/// Kanon の operations 配列を AST へ
pub fn parse_operations(
    ops: &[Value],
    current_receiver_id_as_this: Option<&String>, // 引数名を変更
    memo_env: &mut MemoEnv, // MemoEnv を可変参照で受け取る
) -> anyhow::Result<Program> { // 戻り値を Program のみに変更
    let mut stmts = Vec::<Stmt>::new();
    let mut literal_nodes: HashMap<String, Expr> = HashMap::new(); // 値をExprとして保持

    // receiver_object_id が None の場合、main-new* で始まるIDを自動的にthisとして設定
    if current_receiver_id_as_this.is_none() {
        for op_val in ops {
            if let Some(op) = get_operation(op_val) {
                if let Some(from_id) = &op.from {
                    if from_id.starts_with("main-new") {
                        memo_env.add_special_mapping(from_id.clone(), "this".to_string());
                        println!("Auto-detected receiver_object_id: {}", from_id);
                        break;
                    }
                }
            }
        }
    } else if let Some(receiver_id) = current_receiver_id_as_this {
        memo_env.add_special_mapping(receiver_id.clone(), "this".to_string());
    }

    for op_val in ops {
        let op: Operation = match get_operation(op_val) {
            Some(o) => o,
            None => {
                eprintln!("Failed to parse operation: {:?}", op_val);
                continue;
            }
        };

        match op.edit_type.as_str() {
            "addNode" => {
                let id = op.id.as_ref().ok_or_else(|| anyhow::anyhow!("addNode operation missing id"))?;
                let label = op.label.as_deref().unwrap_or_default();

                if Some(id.as_str()) == current_receiver_id_as_this.map(|s| s.as_str()) {
                    // 'this' node, no VarDecl needed, already mapped by add_special_mapping
                    // Still, register it as part of the current scope if it's explicitly added.
                    memo_env.register_var_in_current_scope(id);
                } else if let Some(json_val) = &op.value {
                    let expr = parse_json_value_to_expr(json_val)?;
                    literal_nodes.insert(id.clone(), expr.clone());
                } else if op.is_literal.unwrap_or(false) {
                    let expr = parse_literal(label);
                    literal_nodes.insert(id.clone(), expr.clone());
                } else {
                    let var_name = memo_env.resolve_or_create_var_name_for_id(id, false);
                    stmts.push(Stmt::VarDecl {
                        name: var_name,
                        expr: Expr::New(label.to_string()),
                    });
                    memo_env.register_var_in_current_scope(id); // Register var in scope
                }
            }
            "addEdge" => {
                let from_id = op.from.as_ref().ok_or_else(|| anyhow::anyhow!("addEdge operation missing from"))?;
                let to_id = op.to.as_ref().ok_or_else(|| anyhow::anyhow!("addEdge operation missing to"))?;
                let edge_label = op.label.as_deref().unwrap_or_default();

                let is_receiver_from = current_receiver_id_as_this
                    .map(|s| s == from_id)
                    .unwrap_or(false);
                let owner_name = memo_env.resolve_or_create_var_name_for_id(from_id, is_receiver_from);

                let lhs_base = if owner_name == "this" { Lhs::This } else { Lhs::Var(owner_name.clone()) };
                let lhs = Lhs::ObjAccess(Box::new(lhs_base), edge_label.to_string());

                let expr = resolve_id_to_expr(to_id, memo_env, &literal_nodes, current_receiver_id_as_this)?;
                stmts.push(Stmt::Assign { lhs, expr });

                if !literal_nodes.contains_key(to_id) {
                    memo_env.register_property_assignment(from_id.clone(), edge_label.to_string(), to_id.clone());
                    
                    // Check for external reference registration
                    if let Some(owner_access_path) = memo_env.get_access_path(from_id) {
                        if owner_access_path == "this" || owner_access_path.starts_with("this.") {
                            let external_path_for_value = format!("{}.{}", owner_access_path, edge_label);
                            memo_env.register_external_reference(to_id, &external_path_for_value);
                        }
                    }
                }
            }
            "editEdge" => { // Refer (プロパティ参照の変更)
                let from_id = op.from.as_ref().ok_or_else(|| anyhow::anyhow!("editEdge operation missing from"))?;
                let new_to_id = op.new_to.as_ref().ok_or_else(|| anyhow::anyhow!("editEdge operation missing newTo"))?;
                let edge_label = op.label.as_deref().unwrap_or_default(); // プロパティ名

                let is_receiver_from = current_receiver_id_as_this
                    .map(|s| s == from_id)
                    .unwrap_or(false);
                let owner_name_for_lhs = memo_env.resolve_or_create_var_name_for_id(from_id, is_receiver_from);
                let lhs_base_obj_expr = if owner_name_for_lhs == "this" {
                    Expr::Var("this".to_string())
                } else {
                    Expr::Var(owner_name_for_lhs)
                };

                let lhs_base = expr_to_lhs(lhs_base_obj_expr);
                let lhs = Lhs::ObjAccess(Box::new(lhs_base), edge_label.to_string());

                let expr = resolve_id_to_expr(new_to_id, memo_env, &literal_nodes, current_receiver_id_as_this)?;
                stmts.push(Stmt::Assign { lhs, expr });

                // プロパティ割り当てを登録
                if !literal_nodes.contains_key(new_to_id) {
                    memo_env.register_property_assignment(from_id.clone(), edge_label.to_string(), new_to_id.clone());

                    // Check for external reference registration
                    if let Some(owner_access_path) = memo_env.get_access_path(from_id) {
                        if owner_access_path == "this" || owner_access_path.starts_with("this.") {
                            let external_path_for_value = format!("{}.{}", owner_access_path, edge_label);
                            memo_env.register_external_reference(new_to_id, &external_path_for_value);
                        }
                    }
                }
            }
            "editVariable" => { // Refer (変数の参照先の変更)
                let var_id = op.id.as_ref().ok_or_else(|| anyhow::anyhow!("editVariable operation missing id (variable id)"))?;
                let new_to_id = op.new_to.as_ref().ok_or_else(|| anyhow::anyhow!("editVariable operation missing newTo"))?;

                let lhs_expr = resolve_id_to_expr(
                    var_id,
                    memo_env,
                    &literal_nodes,
                    current_receiver_id_as_this,
                )?;
                let lhs = expr_to_lhs(lhs_expr);

                if matches!(lhs, Lhs::This) {
                    eprintln!(
                        "Warning: Attempting to reassign 'this' via editVariable for id {}",
                        var_id
                    );
                    continue;
                }

                let expr = resolve_id_to_expr(new_to_id, memo_env, &literal_nodes, current_receiver_id_as_this)?;
                stmts.push(Stmt::Assign { lhs, expr });
            }
            "addVariable" => {
                let var_id = op.id.as_ref().ok_or_else(|| anyhow::anyhow!("addVariable operation missing id"))?;
                let var_name_hint = op.label.as_deref();

                // For addVariable, is_receiver should be false.
                // If var_id happens to be the actual receiver_id, resolve_or_create_var_name_for_id
                // should pick it up from special_mappings or main-new* heuristic.
                let var_name = memo_env.resolve_or_create_var_name_for_id(var_id, false);

                if var_name == "this" && Some(var_id.as_str()) != current_receiver_id_as_this.map(|s| s.as_str()) {
                     // Avoid declaring 'let this = ...' unless var_id is the actual receiver.
                     // This case should be rare if current_receiver_id_as_this is correctly set.
                     eprintln!("Warning: addVariable for id {} resolved to 'this' but is not the designated receiver. Hint: {:?}", var_id, var_name_hint);
                     // Potentially generate a different name or handle as an error.
                     // For now, proceed but this might lead to incorrect code.
                }


                if let Some(val_json) = &op.value {
                    let expr = parse_json_value_to_expr(val_json)?;
                    stmts.push(Stmt::VarDecl { name: var_name, expr });
                } else {
                    stmts.push(Stmt::VarDecl {
                        name: var_name,
                        expr: Expr::Literal("null".to_string()), // 仮の初期値
                    });
                    eprintln!("Warning: addVariable for id {} without value, initialized to null. Label hint: {:?}", var_id, var_name_hint);
                }
                memo_env.register_var_in_current_scope(var_id); // Register var in scope
            }
            "methodCall" => {
                // main.rs で処理
            }
            "editNode" => {
                let id = op.id.as_ref().ok_or_else(|| anyhow::anyhow!("editNode operation missing id"))?;
                let new_label = op.label.as_deref().unwrap_or_default();

                if Some(id.as_str()) == current_receiver_id_as_this.map(|s| s.as_str()) {
                    // 'this' node label cannot be changed
                } else if literal_nodes.contains_key(id) {
                    let new_expr = parse_literal(new_label);
                    literal_nodes.insert(id.clone(), new_expr);
                } else if memo_env.get_name_by_id(id).is_some() {
                    eprintln!("Warning: 'editNode' on existing variable {} to new label {} is not generating an AST statement.", id, new_label);
                }
            }
            _ => {
                // 他の操作はそのまま
            }
        }
    }
    Ok(Program { stmts })
}

// リテラル文字列を適切な式に変換するヘルパー関数
fn parse_literal(literal_str: &str) -> Expr {
    if literal_str == "true" {
        return Expr::Literal("true".to_string());
    }
    if literal_str == "false" {
        return Expr::Literal("false".to_string());
    }
    if literal_str == "null" {
        return Expr::Literal("null".to_string());
    }
    if literal_str == "undefined" {
        return Expr::Literal("undefined".to_string());
    }
    if let Ok(num) = literal_str.parse::<i64>() {
        return Expr::Num(num);
    }
    if let Ok(num_f) = literal_str.parse::<f64>() {
        return Expr::Num(num_f as i64);
    }
    Expr::Str(literal_str.to_string())
}

// JSON Value を AST Expr に変換するヘルパー関数
fn parse_json_value_to_expr(
    val: &Value,
) -> anyhow::Result<Expr> {
    match val {
        Value::Null => Ok(Expr::Literal("null".to_string())),
        Value::Bool(b) => Ok(Expr::Literal(b.to_string())),
        Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Ok(Expr::Num(i))
            } else if let Some(f) = n.as_f64() {
                Ok(Expr::Num(f as i64))
            } else {
                Err(anyhow::anyhow!("Unsupported number format in JSON value"))
            }
        }
        Value::String(s) => Ok(Expr::Str(s.clone())),
        Value::Array(_arr) => {
            eprintln!("Warning: Array literal in JSON value is not fully supported, stringifying.");
            Ok(Expr::Str(val.to_string()))
        }
        Value::Object(_map) => {
            eprintln!("Warning: Object literal in JSON value is not fully supported, stringifying.");
            Ok(Expr::Str(val.to_string()))
        }
    }
}

// ID を解決して適切な Expr を返すヘルパー関数
fn resolve_id_to_expr(
    id: &str,
    memo_env: &mut MemoEnv,
    literal_nodes: &HashMap<String, Expr>,
    current_receiver_id_as_this: Option<&String>,
) -> anyhow::Result<Expr> {
    if current_receiver_id_as_this.map_or(false, |s| s == id) {
        if memo_env.get_name_by_id(id).map_or(false, |name| name == "this") {
            return Ok(Expr::This);
        }
    }

    if let Some(access_path) = memo_env.get_access_path(id) {
        if access_path == "this" {
            return Ok(Expr::This);
        }
        
        if access_path.starts_with("this.") {
            let parts: Vec<&str> = access_path.split('.').collect();
            let mut lhs = Lhs::This;
            
            for prop in parts.into_iter().skip(1) {
                lhs = Lhs::ObjAccess(Box::new(lhs), prop.to_string());
            }
            
            return Ok(Expr::Lhs(Box::new(lhs)));
        }
    }

    let property_access_info = memo_env.get_property_access_for_id(id).cloned();

    if let Some((owner_id, prop_name)) = property_access_info {
        let owner_expr = resolve_id_to_expr(&owner_id, memo_env, literal_nodes, current_receiver_id_as_this)?;
        let lhs_base = match owner_expr {
            Expr::Var(name) => Lhs::Var(name),
            Expr::This => Lhs::This,
            _ => {
                eprintln!(
                    "Warning: Property access base for id '{}' is not a simple var or this. Falling back.",
                    id
                );
                let name = memo_env.resolve_or_create_var_name_for_id(id, false);
                if name == "this" {
                    return Ok(Expr::This);
                } else {
                    return Ok(Expr::Var(name));
                }
            }
        };
        return Ok(Expr::Lhs(Box::new(Lhs::ObjAccess(Box::new(lhs_base), prop_name))));
    }

    if let Some(expr) = literal_nodes.get(id) {
        return Ok(expr.clone());
    }

    let name = memo_env.resolve_or_create_var_name_for_id(id, false);
    if name == "this" {
        Ok(Expr::This)
    } else {
        Ok(Expr::Var(name))
    }
}

fn expr_to_lhs(expr: Expr) -> Lhs {
    match expr {
        Expr::This => Lhs::This,
        Expr::Var(name) => Lhs::Var(name),
        Expr::Lhs(lhs) => *lhs,
        other => {
            eprintln!("Unexpected expr for LHS: {other:?}. Fallback to Hole");
            Lhs::Hole("__UnexpectedExpr__".into())
        }
    }
}
