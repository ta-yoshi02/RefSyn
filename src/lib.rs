pub mod ast;
pub mod env;
pub mod error;
pub mod ir;
pub mod models;
pub mod parser;
pub mod server;
pub mod is_structurally_equivalent_option;
pub mod program_analyzer;
pub mod build_graph_detailed;
pub mod unify_ops;

use serde::{Deserialize, Serialize};
use serde_json;
use anyhow;
use std::collections::HashMap;
use warp::http::StatusCode;
use crate::ast::{Placeholder, Program};
use crate::parser::parse_operations;
use crate::env::MemoEnv;

#[derive(Deserialize, Debug)]
pub struct MethodCallOperation {
    #[serde(rename = "callLabel")]
    pub call_label: String,
    #[serde(rename = "contextSensitiveID")]
    pub context_sensitive_id: String,
    #[serde(rename = "receiverObject")]
    pub receiver_object: String,
    #[serde(rename = "methodName")]
    pub method_name: String,
    pub operations: Vec<serde_json::Value>,
}

#[derive(Deserialize, Debug)]
pub struct SynthesisRequest {
    pub method_calls: Vec<MethodCallOperation>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SynthesisResponse {
    pub common_pattern: Option<String>,
    pub hole_information: Option<HashMap<String, Vec<String>>>,
    pub code: Vec<String>,
    pub individual_codes: Vec<String>,
}

fn add_to_hole(
    hole_map: &mut HashMap<String, Vec<String>>,
    next_hole_id: &mut usize,
    category: &str,
    value1_str: String,
    value2_str: String,
) -> Placeholder {
    let hole_key = format!("Hole{} ({})", *next_hole_id, category);
    let placeholder_name = format!("Hole{}", *next_hole_id);
    *next_hole_id += 1;
    hole_map.insert(hole_key, vec![value1_str, value2_str]);
    placeholder_name
}

fn update_hole_value(
    hole_map: &mut HashMap<String, Vec<String>>,
    placeholder_key: &str,
    new_value_str: String,
) {
    if let Some(values) = hole_map.get_mut(placeholder_key) {
        values.push(new_value_str);
    } else {
        eprintln!("Warning: Attempted to update non-existent hole with key: {}", placeholder_key);
    }
}

fn find_common_pattern_and_holes(
    programs: &[ast::Program],
    memo_envs: &[MemoEnv],
    operations_list: Option<&[Vec<serde_json::Value>]>,
) -> (Option<ast::Program>, HashMap<String, Vec<String>>) {
    if programs.is_empty() {
        return (None, HashMap::new());
    }
    if programs.len() == 1 {
        return (Some(programs[0].clone()), HashMap::new());
    }

    // 操作ログを使用した改良されたIRベースのアプローチを試す
    if let Some(ops_list) = operations_list {
        if ops_list.len() >= 2 {
            // 新しい依存関係に基づく正規化アプローチを使用
            let result = ir::find_common_pattern_from_operations(ops_list, memo_envs);
            if result.0.is_some() {
                return result;
            }
            
            // バックアップとして従来のペアワイズマッチングも試みる
            let mut ir_ops_list = Vec::new();
            
            // 各操作ログをIRに変換
            for ops in ops_list {
                match convert_operations_to_ir(ops) {
                    Ok(ir_ops) => ir_ops_list.push(ir_ops),
                    Err(e) => {
                        // IR変換に失敗した場合は従来のアプローチにフォールバック
                        eprintln!("Failed to convert operations to IR: {}, falling back to traditional approach", e);
                        return find_common_pattern_and_holes_traditional(programs);
                    }
                }
            }
            
            // IRを使って2つの操作グラフをマッチング
            if ir_ops_list.len() >= 2 {
                let match_result = ir::match_graphs(&ir_ops_list[0], &ir_ops_list[1]);
                
                // マッチング結果をホールマップに変換
                let mut hole_map = HashMap::new();
                for hole in match_result.holes {
                    match hole {
                        ir::Hole::Const { placeholder, values } => {
                            hole_map.insert(format!("{} (const)", placeholder), values);
                        },
                        ir::Hole::Ref { placeholder, refs } => {
                            hole_map.insert(format!("{} (ref)", placeholder), refs);
                        }
                    }
                }
                
                // 共通パターンに基づいたプログラムを構築
                let (traditional_ast, traditional_holes) = find_common_pattern_and_holes_traditional(programs);
                
                // IRベースのホール情報と従来のホール情報を統合
                if let Some(ast) = traditional_ast {
                    // 従来のホール情報を追加（競合する場合はIRベースを優先）
                    for (key, values) in traditional_holes {
                        if !hole_map.contains_key(&key) {
                            hole_map.insert(key, values);
                        }
                    }
                    return (Some(ast), hole_map);
                }
            }
        }
    }
    
    // IRベースのアプローチが利用できない場合は従来のアプローチを使用
    find_common_pattern_and_holes_traditional(programs)
}

pub fn find_common_pattern_and_holes_traditional(
    programs: &[ast::Program],
) -> (Option<ast::Program>, HashMap<String, Vec<String>>) {
    use crate::ast::synchronize_and_hole_stmt;
    
    if programs.is_empty() {
        return (None, HashMap::new());
    }
    if programs.len() == 1 {
        return (Some(programs[0].clone()), HashMap::new());
    }

    let mut hole_map: HashMap<String, Vec<String>> = HashMap::new();
    let mut next_hole_id = 1;

    let overall_min_stmts = programs.iter().map(|p| p.stmts.len()).min().unwrap_or(0);

    let mut template_program = programs[0].clone();
    template_program.stmts.truncate(overall_min_stmts);

    for i in 1..programs.len() {
        let target_program = &programs[i];
        for stmt_idx in 0..overall_min_stmts {
            if stmt_idx < template_program.stmts.len() && stmt_idx < target_program.stmts.len() {
                synchronize_and_hole_stmt(
                    &mut template_program.stmts[stmt_idx],
                    &target_program.stmts[stmt_idx],
                    &mut next_hole_id,
                    &mut hole_map,
                );
            } else {
                break; 
            }
        }
    }

    (Some(template_program), hole_map)
}

// Convert Kanon operations to IR
pub fn convert_operations_to_ir(operations: &[serde_json::Value]) -> anyhow::Result<Vec<ir::Op>> {
    let mut result = Vec::new();
    
    for (i, op_val) in operations.iter().enumerate() {
        let op_id = format!("op_{}", i); // 一意のID生成
        
        if let Ok(op) = serde_json::from_value::<parser::Operation>(op_val.clone()) {
            match op.edit_type.as_str() {
                "addNode" => {
                    if let (Some(id), Some(is_literal)) = (op.id.clone(), op.is_literal) {
                        let label = op.label.clone().unwrap_or_default();
                        result.push(ir::Op {
                            id: op_id,
                            kind: ir::OpKind::AddNode {
                                id,
                                is_literal,
                                label,
                            }
                        });
                    }
                },
                "addEdge" => {
                    if let (Some(from), Some(to), Some(label)) = (op.from.clone(), op.to.clone(), op.label.clone()) {
                        result.push(ir::Op {
                            id: op_id,
                            kind: ir::OpKind::AddEdge {
                                from,
                                to,
                                label,
                            }
                        });
                    }
                },
                "editEdgeReference" => {
                    if let (Some(from), Some(old_to), Some(new_to), Some(label)) = 
                        (op.from.clone(), op.old_to.clone(), op.new_to.clone(), op.label.clone()) {
                        result.push(ir::Op {
                            id: op_id,
                            kind: ir::OpKind::EditEdgeReference {
                                from,
                                old_to,
                                new_to,
                                label,
                            }
                        });
                    }
                },
                "deleteNode" => {
                    if let Some(id) = op.id.clone() {
                        result.push(ir::Op {
                            id: op_id,
                            kind: ir::OpKind::DeleteNode {
                                id,
                            }
                        });
                    }
                },
                "deleteEdge" => {
                    if let (Some(from), Some(to)) = (op.from.clone(), op.to.clone()) {
                        let label = op.label.clone().unwrap_or_default();
                        result.push(ir::Op {
                            id: op_id,
                            kind: ir::OpKind::DeleteEdge {
                                from,
                                to,
                                label,
                            }
                        });
                    }
                },
                "addVariable" => {
                    // toフィールドを優先し、なければidフィールドを使用
                    let target_id = op.to.clone().or_else(|| op.id.clone());
                    if let Some(to) = target_id {
                        let label = op.label.clone().unwrap_or_default();
                        result.push(ir::Op {
                            id: op_id,
                            kind: ir::OpKind::AddVariable {
                                to,
                                label,
                            }
                        });
                    } else {
                        return Err(anyhow::anyhow!("addVariable operation missing id"));
                    }
                },
                "editVariable" => {
                    if let (Some(old_to), Some(new_to)) = (op.old_to.clone(), op.new_to.clone()) {
                        let label = op.label.clone().unwrap_or_default();
                        result.push(ir::Op {
                            id: op_id,
                            kind: ir::OpKind::EditVariableReference {
                                old_to,
                                new_to,
                                label,
                            }
                        });
                    }
                },
                "editNode" => {
                    if let (Some(id), Some(is_literal)) = (op.id.clone(), op.is_literal) {
                        let label = op.label.clone().unwrap_or_default();
                        result.push(ir::Op {
                            id: op_id,
                            kind: ir::OpKind::EditNode {
                                id,
                                is_literal,
                                label,
                            }
                        });
                    }
                },
                // その他のeditType対応を追加
                _ => {} // 未対応の操作は無視
            }
        }
    }
    
    Ok(result)
}

// Types needed by the server module will be imported from main directly

pub async fn handle_synthesis(req: SynthesisRequest) -> Result<impl warp::Reply, warp::Rejection> {
    println!("Received synthesis request: {:?}", req);

    let mut parsed_programs = Vec::new();
    let mut all_memo_envs_for_pattern_finding = Vec::new();
    let mut operations_list = Vec::new();

    if req.method_calls.is_empty() {
        println!("No method calls provided in the request.");
        let response = SynthesisResponse {
            common_pattern: Some("No method calls provided.".to_string()),
            hole_information: None,
            code: vec![],
            individual_codes: vec![],
        };
        return Ok(warp::reply::with_status(warp::reply::json(&response), StatusCode::OK));
    }

    let mut global_memo_env = MemoEnv::new();
    
    for (index, method_call_op) in req.method_calls.iter().enumerate() {
        println!(
            "Processing method call: {}, Receiver: {}, Method: {}",
            method_call_op.call_label,
            method_call_op.receiver_object,
            method_call_op.method_name
        );

        let mut local_memo_env = MemoEnv::new();
        
        // メソッド呼び出しのスコープを開始し、レシーバーを設定
        let scope_id = format!("method_call_{}_{}", index, method_call_op.call_label);
        local_memo_env.start_method_call_scope_with_receiver(&scope_id, &method_call_op.receiver_object);
        
        if index > 0 {
            local_memo_env.setup_cross_scope_references(&global_memo_env);
        }

        match parse_operations(
            &method_call_op.operations,
            Some(&method_call_op.receiver_object),
            &mut local_memo_env,
        ) {
            Ok(program) => {
                println!(
                    "Parsed AST for {}: \n{}",
                    method_call_op.call_label,
                    program
                );
                parsed_programs.push(program);
                
                global_memo_env.merge_from(&local_memo_env);
                
                all_memo_envs_for_pattern_finding.push(local_memo_env.clone());
                operations_list.push(method_call_op.operations.clone());
            }
            Err(e) => {
                eprintln!(
                    "Error parsing operations for {}: {}",
                    method_call_op.call_label,
                    e
                );
                let response = SynthesisResponse {
                    common_pattern: None,
                    hole_information: None,
                    code: vec![format!(
                        "Error parsing operations for {}: {}",
                        method_call_op.call_label,
                        e
                    )],
                    individual_codes: vec![],
                };
                return Ok(warp::reply::with_status(
                    warp::reply::json(&response),
                    StatusCode::BAD_REQUEST,
                ));
            }
        }
    }

    if parsed_programs.is_empty() {
        println!("No ASTs were parsed successfully from any method call.");
        let response = SynthesisResponse {
            common_pattern: Some("No operations to synthesize or error during parsing.".to_string()),
            hole_information: None,
            code: vec![],
            individual_codes: vec![],
        };
        return Ok(warp::reply::with_status(warp::reply::json(&response), StatusCode::OK));
    }

    let (common_pattern_ast_option, hole_map) = find_common_pattern_and_holes(&parsed_programs, &all_memo_envs_for_pattern_finding, Some(&operations_list));
    
    let common_pattern_ast = match common_pattern_ast_option {
        Some(ast) => {
            println!("Common Pattern AST: \n{}", ast);
            ast
        }
        None => {
            println!("No common pattern could be determined.");
            Program { stmts: vec![] }
        }
    };
    
    println!("Initial Hole Map: {:?}", hole_map);

    let common_pattern_str = format!("{}", common_pattern_ast);
    let individual_codes_str: Vec<String> = parsed_programs
        .iter()
        .map(|p| format!("{}", p))
        .collect();

    let formatted_hole_info = if !hole_map.is_empty() {
        let mut info = HashMap::new();
        for (hole_id_with_cat, values) in &hole_map { 
            info.insert(hole_id_with_cat.clone(), values.clone());
        }
        Some(info)
    } else {
        None
    };
    println!("Formatted Hole Info: {:?}", formatted_hole_info);

    let response = SynthesisResponse {
        common_pattern: Some(common_pattern_str),
        hole_information: formatted_hole_info,
        code: individual_codes_str.clone(),
        individual_codes: individual_codes_str,
    };
    println!("Final Response: {:?}", response);

    Ok(warp::reply::with_status(warp::reply::json(&response), StatusCode::OK))
}
