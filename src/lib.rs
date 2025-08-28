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
pub mod isomorphism;
pub mod list_env;
pub mod operation_analyzer;

use serde::{Deserialize, Serialize};
use serde_json;
use anyhow;
use std::collections::HashMap;
use warp::http::StatusCode;
use crate::ast::{Placeholder, Program};
use crate::parser::parse_operations;
use crate::env::MemoEnv;

#[derive(Deserialize, Serialize, Debug)]
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

use crate::models::VisGraph;

#[derive(Deserialize, Serialize, Debug)]
pub struct SynthesisRequest {
    pub method_calls: Vec<MethodCallOperation>,
    pub vis_graph: VisGraph,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SynthesisResponse {
    pub common_pattern: Option<String>,
    pub hole_information: Option<HashMap<String, Vec<String>>>,
    pub code: Vec<String>,
    pub individual_codes: Vec<String>,
    pub list_environment_info: Option<String>, // ListEnvironmentの情報を追加
    // 操作分析結果のフィールド（複数の操作列が提供された場合のみ設定）
    pub operation_analysis: Option<OperationAnalysisData>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OperationAnalysisData {
    pub common_operations_count: usize,
    pub total_operations_counts: Vec<usize>,
    pub difference_summary: String,
    pub differences_found: usize,
    pub synthesis_matches: Option<usize>, // 合成で見つかった共通パターン数
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

pub async fn handle_synthesis(body: bytes::Bytes) -> Result<impl warp::Reply, warp::Rejection> {
    // 受け取ったリクエストボディをターミナルに出力
    if let Ok(body_str) = std::str::from_utf8(&body) {
        println!("--- RAW REQUEST PAYLOAD ---");
        println!("{}", body_str);
        println!("---------------------------");
    }

    // 手動でデシリアライズ
    let req: SynthesisRequest = match serde_json::from_slice(&body) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("Deserialization error: {}", e);
            let response = SynthesisResponse {
                common_pattern: Some(format!("Invalid request body: {}", e)),
                hole_information: None,
                code: vec![],
                individual_codes: vec![],
                list_environment_info: None,
                operation_analysis: None,
            };
            return Ok(warp::reply::with_status(
                warp::reply::json(&response),
                StatusCode::BAD_REQUEST,
            ));
        }
    };

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
            list_environment_info: None,
            operation_analysis: None,
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
                    list_environment_info: None,
                    operation_analysis: None,
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
            list_environment_info: None,
            operation_analysis: None,
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

    // ListEnvironmentを作成し、より構造化された情報を提供
    use crate::list_env::ListEnvironment;
    let list_env = ListEnvironment::from_vis_graph(&req.vis_graph);
    
    // List環境の要約情報を作成
    let field_count = list_env.field_lists.len();
    let object_count = list_env.obj_id_to_index.len();
    let literal_count = list_env.literal_id_to_value.len();
    
    let field_names: Vec<String> = list_env.field_lists.keys().cloned().collect();
    
    let list_env_info = format!(
        "List Environment Summary:\n- {} objects tracked\n- {} literals\n- {} field types: {}\n\nCurrent state:\n{}",
        object_count,
        literal_count,
        field_count,
        field_names.join(", "),
        list_env.to_debug_string()
    );
    
    println!("List Environment Summary: {}", list_env_info);

    // 複数の操作列がある場合は操作分析も実行
    let operation_analysis_data = if operations_list.len() > 1 {
        println!("Starting operation analysis with {} operation sequences", operations_list.len());
        
        // unify_isomorphic_graphsを使って同型グラフ統合を試行
        match analyze_operations_with_unification(&req.vis_graph, &operations_list[0], &operations_list[1]) {
            Ok(analysis_result) => {
                println!("Unification-based analysis succeeded!");
                let difference_summary = if analysis_result.common_operations_count == 0 {
                    "統合ベース分析: 共通する操作パターンが見つかりませんでした".to_string()
                } else {
                    format!(
                        "統合ベース分析: {}個の共通操作パターンを特定。差異部分では{}個と{}個の異なる操作。",
                        analysis_result.common_operations_count,
                        analysis_result.differences_found,
                        analysis_result.total_operations_counts.get(1).unwrap_or(&0) - analysis_result.common_operations_count
                    )
                };

                Some(OperationAnalysisData {
                    common_operations_count: analysis_result.common_operations_count,
                    total_operations_counts: analysis_result.total_operations_counts.clone(),
                    difference_summary,
                    differences_found: analysis_result.differences_found,
                    synthesis_matches: Some(analysis_result.common_operations_count),
                })
            }
            Err(e) => {
                eprintln!("Error during unification-based operation analysis: {}", e);
                // フォールバックとして従来の操作分析を使用
                use crate::operation_analyzer::analyze_operations_with_environments;
                match analyze_operations_with_environments(&req.vis_graph, &operations_list[0], &operations_list[1]) {
                    Ok(analysis_result) => {
                        println!("Fallback to position-based analysis succeeded");
                        let difference_summary = format!(
                            "位置ベース分析（フォールバック）: {}個の共通操作、{}個の差異点",
                            analysis_result.common_operations_count,
                            analysis_result.difference_points.len()
                        );
                        
                        Some(OperationAnalysisData {
                            common_operations_count: analysis_result.common_operations_count,
                            total_operations_counts: vec![
                                analysis_result.total_operations_a,
                                analysis_result.total_operations_b,
                            ],
                            difference_summary,
                            differences_found: analysis_result.difference_points.len(),
                            synthesis_matches: None,
                        })
                    }
                    Err(e2) => {
                        eprintln!("Error during fallback operation analysis: {}", e2);
                        None
                    }
                }
            }
        }
    } else {
        None
    };

    let response = SynthesisResponse {
        common_pattern: Some(common_pattern_str),
        hole_information: formatted_hole_info,
        code: individual_codes_str.clone(),
        individual_codes: individual_codes_str,
        list_environment_info: Some(list_env_info),
        operation_analysis: operation_analysis_data,
    };
    
    // 改善されたレスポンス表示
    println!("=== IMPROVED SYNTHESIS RESPONSE ===");
    println!("Common Pattern: {}", response.common_pattern.as_ref().unwrap_or(&"None".to_string()));
    println!("Holes: {}", response.hole_information.as_ref().map(|h| format!("{:?}", h)).unwrap_or("None".to_string()));
    
    if let Some(analysis) = &response.operation_analysis {
        println!("Operation Analysis Summary: {}", analysis.difference_summary);
        println!("  - Raw operation differences: {}", analysis.differences_found);
        println!("  - Synthesis common patterns: {}", analysis.synthesis_matches.unwrap_or(0));
    } else {
        println!("Operation Analysis: Not performed (single operation sequence)");
    }
    
    println!("List Environment Summary: {}", response.list_environment_info.as_ref().unwrap_or(&"None".to_string()));
    println!("=====================================");

    Ok(warp::reply::with_status(warp::reply::json(&response), StatusCode::OK))
}

// unify_isomorphic_graphsを使用した統合ベースの操作分析
pub fn analyze_operations_with_unification(
    vis_graph: &models::VisGraph,
    operations_a: &[serde_json::Value],
    operations_b: &[serde_json::Value],
) -> anyhow::Result<UnificationAnalysisResult> {
    use crate::isomorphism::unify_isomorphic_graphs;
    
    println!("=== UNIFICATION-BASED OPERATION ANALYSIS ===");
    
    // JSON操作をunify_ops::Op形式に変換
    let ops_a = convert_json_to_unify_ops(operations_a)?;
    let ops_b = convert_json_to_unify_ops(operations_b)?;
    
    println!("Converted {} operations from sequence A", ops_a.len());
    println!("Converted {} operations from sequence B", ops_b.len());
    
    // unify_isomorphic_graphsを呼び出し
    let unification_result = unify_isomorphic_graphs(&ops_a, &ops_b);
    
    // 統合結果を分析
    let common_count = unification_result.common_a.len();
    let diff_a_count = unification_result.diff_a.len();
    let diff_b_count = unification_result.diff_b.len();
    
    println!("Unification results:");
    println!("  - Common operations: {}", common_count);
    println!("  - Differences in A: {}", diff_a_count);
    println!("  - Differences in B: {}", diff_b_count);
    
    // List環境を共通操作のポイントまで構築
    if common_count > 0 {
        let list_env = create_environment_at_unification_boundary(vis_graph, &unification_result.common_a)?;
        println!("Created List environment at unification boundary with {} common operations", common_count);
        println!("Environment state: {}", list_env.to_debug_string());
    }
    
    Ok(UnificationAnalysisResult {
        common_operations_count: common_count,
        total_operations_counts: vec![ops_a.len(), ops_b.len()],
        differences_found: std::cmp::max(diff_a_count, diff_b_count),
        unification_result,
    })
}

// JSON操作をunify_ops::Op形式に変換
fn convert_json_to_unify_ops(operations: &[serde_json::Value]) -> anyhow::Result<Vec<crate::unify_ops::Op>> {
    use crate::unify_ops::{Op, GraphOp, NodeExpr, EdgeExpr};
    
    let mut result = Vec::new();
    
    for (i, op_val) in operations.iter().enumerate() {
        if let Ok(op) = serde_json::from_value::<parser::Operation>(op_val.clone()) {
            let graph_op = match op.edit_type.as_str() {
                "addNode" => {
                    if let (Some(id), Some(is_literal)) = (op.id.clone(), op.is_literal) {
                        let label = op.label.clone().unwrap_or_default();
                        GraphOp::Node(NodeExpr::AddNode {
                            id,
                            label,
                            is_literal,
                        })
                    } else {
                        continue; // スキップ
                    }
                },
                "addEdge" => {
                    if let (Some(from), Some(to), Some(label)) = (op.from.clone(), op.to.clone(), op.label.clone()) {
                        GraphOp::Edge(EdgeExpr::AddEdge {
                            from,
                            to,
                            label,
                        })
                    } else {
                        continue; // スキップ
                    }
                },
                "deleteNode" => {
                    if let Some(id) = op.id.clone() {
                        // DeleteNodeは現在NodeExprに定義されていないため、ExistNodeとして扱う
                        GraphOp::Node(NodeExpr::ExistNode {
                            id,
                            label: op.label.clone().unwrap_or_default(),
                            is_literal: op.is_literal.unwrap_or(false),
                        })
                    } else {
                        continue; // スキップ
                    }
                },
                "deleteEdge" => {
                    if let (Some(from), Some(to)) = (op.from.clone(), op.to.clone()) {
                        let label = op.label.clone().unwrap_or_default();
                        GraphOp::Edge(EdgeExpr::DeleteEdge {
                            from,
                            to,
                            label,
                        })
                    } else {
                        continue; // スキップ
                    }
                },
                _ => continue, // 他の操作タイプはスキップ
            };
            
            result.push(Op {
                id: format!("op_{}", i),
                kind: graph_op,
            });
        }
    }
    
    Ok(result)
}

// 統合境界でのList環境作成
fn create_environment_at_unification_boundary(
    vis_graph: &models::VisGraph,
    common_operations: &[crate::unify_ops::Op],
) -> anyhow::Result<crate::list_env::ListEnvironment> {
    use crate::list_env::ListEnvironment;
    
    // 初期状態のList環境を作成
    let mut list_env = ListEnvironment::from_vis_graph(vis_graph);
    
    // 共通操作を順次適用してホール境界までの状態を構築
    for op in common_operations {
        apply_operation_to_list_env(&mut list_env, op)?;
    }
    
    Ok(list_env)
}

// List環境に操作を適用
fn apply_operation_to_list_env(
    list_env: &mut crate::list_env::ListEnvironment,
    op: &crate::unify_ops::Op,
) -> anyhow::Result<()> {
    use crate::unify_ops::{GraphOp, NodeExpr, EdgeExpr};
    
    match &op.kind {
        GraphOp::Node(NodeExpr::AddNode { id, label, is_literal }) => {
            // __RectForVariable__と__Variable-lstは無視
            if id == "__RectForVariable__" || id == "__Variable-lst" {
                return Ok(());
            }
            
            // ノード追加をList環境に反映
            if *is_literal {
                list_env.literal_id_to_value.insert(id.clone(), serde_json::Value::String(label.clone()));
            } else {
                // 新しいオブジェクトインデックスを追加
                let new_index = list_env.obj_id_to_index.len();
                list_env.obj_id_to_index.insert(id.clone(), new_index);
                list_env.index_to_obj_id.insert(new_index, id.clone());
            }
        },
        GraphOp::Edge(EdgeExpr::AddEdge { from, to, label }) => {
            // __RectForVariable__や__Variable-lstに関連するエッジは無視
            if from == "__RectForVariable__" || from == "__Variable-lst" || 
               to == "__RectForVariable__" || to == "__Variable-lst" {
                return Ok(());
            }
            
            // エッジ追加をList環境に反映
            if let Some(&from_index) = list_env.obj_id_to_index.get(from) {
                // フィールドリストの更新
                let field_list = list_env.field_lists.entry(label.clone()).or_insert_with(Vec::new);
                
                // インデックスが範囲外の場合はリストを拡張
                while field_list.len() <= from_index {
                    field_list.push(serde_json::Value::Null);
                }
                
                // toが既存オブジェクトか新しいリテラルかを判定
                if let Some(&to_index) = list_env.obj_id_to_index.get(to) {
                    field_list[from_index] = serde_json::Value::String(format!("obj_{}", to_index));
                } else if let Some(literal_value) = list_env.literal_id_to_value.get(to) {
                    field_list[from_index] = literal_value.clone();
                } else {
                    field_list[from_index] = serde_json::Value::String(to.clone());
                }
            }
        },
        GraphOp::Node(NodeExpr::ExistNode { .. }) => {
            // 既存ノード参照は環境変更なし
        },
        GraphOp::Node(NodeExpr::NullNode) => {
            // NullNode操作は環境変更なし
        },
        GraphOp::Edge(EdgeExpr::DeleteEdge { .. }) => {
            // エッジ削除は複雑なため、現在は未実装
        },
        GraphOp::Edge(EdgeExpr::EditEdgeReference { .. }) => {
            // エッジ編集は複雑なため、現在は未実装
        },
        GraphOp::Variable(_) => {
            // 変数操作は現在未実装
        },
    }
    
    Ok(())
}

// 統合分析結果の構造体
#[derive(Debug)]
pub struct UnificationAnalysisResult {
    pub common_operations_count: usize,
    pub total_operations_counts: Vec<usize>,
    pub differences_found: usize,
    pub unification_result: crate::unify_ops::UnificationResult,
}
