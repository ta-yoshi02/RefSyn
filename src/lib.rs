pub mod ast;
pub mod build_graph_detailed;
pub mod env;
pub mod error;
pub mod escher_bridge;
pub mod ir;
pub mod is_structurally_equivalent_option;
pub mod isomorphism;
pub mod list_env;
pub mod models;
pub mod operation_analyzer;
pub mod parser;
pub mod program_analyzer;
pub mod server;
pub mod unify_ops;

use crate::ast::{Placeholder, Program};
use crate::env::MemoEnv;
use crate::escher_bridge::{
    build_escher_spec, specs_to_json, write_spec_to_file, EscherCase, EscherSpec,
};
use crate::parser::parse_operations;
use anyhow;
use serde::{Deserialize, Serialize};
use serde_json;
use std::collections::{HashMap, HashSet};
use std::convert::TryFrom;
use warp::http::StatusCode;

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
    #[serde(rename = "actualGraph")]
    pub actual_graph: Option<VisGraph>,
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
        eprintln!(
            "Warning: Attempted to update non-existent hole with key: {}",
            placeholder_key
        );
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
                        ir::Hole::Const {
                            placeholder,
                            values,
                        } => {
                            hole_map.insert(format!("{} (const)", placeholder), values);
                        }
                        ir::Hole::Ref { placeholder, refs } => {
                            hole_map.insert(format!("{} (ref)", placeholder), refs);
                        }
                    }
                }

                // 共通パターンに基づいたプログラムを構築
                let (traditional_ast, traditional_holes) =
                    find_common_pattern_and_holes_traditional(programs);

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
                            },
                        });
                    }
                }
                "addEdge" => {
                    if let (Some(from), Some(to), Some(label)) =
                        (op.from.clone(), op.to.clone(), op.label.clone())
                    {
                        result.push(ir::Op {
                            id: op_id,
                            kind: ir::OpKind::AddEdge { from, to, label },
                        });
                    }
                }
                "editEdgeReference" => {
                    if let (Some(from), Some(old_to), Some(new_to), Some(label)) = (
                        op.from.clone(),
                        op.old_to.clone(),
                        op.new_to.clone(),
                        op.label.clone(),
                    ) {
                        result.push(ir::Op {
                            id: op_id,
                            kind: ir::OpKind::EditEdgeReference {
                                from,
                                old_to,
                                new_to,
                                label,
                            },
                        });
                    }
                }
                "deleteNode" => {
                    if let Some(id) = op.id.clone() {
                        result.push(ir::Op {
                            id: op_id,
                            kind: ir::OpKind::DeleteNode { id },
                        });
                    }
                }
                "deleteEdge" => {
                    if let (Some(from), Some(to)) = (op.from.clone(), op.to.clone()) {
                        let label = op.label.clone().unwrap_or_default();
                        result.push(ir::Op {
                            id: op_id,
                            kind: ir::OpKind::DeleteEdge { from, to, label },
                        });
                    }
                }
                "addVariable" => {
                    // toフィールドを優先し、なければidフィールドを使用
                    let target_id = op.to.clone().or_else(|| op.id.clone());
                    if let Some(to) = target_id {
                        let label = op.label.clone().unwrap_or_default();
                        result.push(ir::Op {
                            id: op_id,
                            kind: ir::OpKind::AddVariable { to, label },
                        });
                    } else {
                        return Err(anyhow::anyhow!("addVariable operation missing id"));
                    }
                }
                "editVariable" => {
                    if let (Some(old_to), Some(new_to)) = (op.old_to.clone(), op.new_to.clone()) {
                        let label = op.label.clone().unwrap_or_default();
                        result.push(ir::Op {
                            id: op_id,
                            kind: ir::OpKind::EditVariableReference {
                                old_to,
                                new_to,
                                label,
                            },
                        });
                    }
                }
                "editNode" => {
                    if let (Some(id), Some(is_literal)) = (op.id.clone(), op.is_literal) {
                        let label = op.label.clone().unwrap_or_default();
                        result.push(ir::Op {
                            id: op_id,
                            kind: ir::OpKind::EditNode {
                                id,
                                is_literal,
                                label,
                            },
                        });
                    }
                }
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
        return Ok(warp::reply::with_status(
            warp::reply::json(&response),
            StatusCode::OK,
        ));
    }

    let mut global_memo_env = MemoEnv::new();

    for (index, method_call_op) in req.method_calls.iter().enumerate() {
        println!(
            "Processing method call: {}, Receiver: {}, Method: {}",
            method_call_op.call_label, method_call_op.receiver_object, method_call_op.method_name
        );

        let mut local_memo_env = MemoEnv::new();

        // メソッド呼び出しのスコープを開始し、レシーバーを設定
        let scope_id = format!("method_call_{}_{}", index, method_call_op.call_label);
        local_memo_env
            .start_method_call_scope_with_receiver(&scope_id, &method_call_op.receiver_object);

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
                    method_call_op.call_label, program
                );
                parsed_programs.push(program);

                global_memo_env.merge_from(&local_memo_env);

                all_memo_envs_for_pattern_finding.push(local_memo_env.clone());
                operations_list.push(method_call_op.operations.clone());
            }
            Err(e) => {
                eprintln!(
                    "Error parsing operations for {}: {}",
                    method_call_op.call_label, e
                );
                let response = SynthesisResponse {
                    common_pattern: None,
                    hole_information: None,
                    code: vec![format!(
                        "Error parsing operations for {}: {}",
                        method_call_op.call_label, e
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
            common_pattern: Some(
                "No operations to synthesize or error during parsing.".to_string(),
            ),
            hole_information: None,
            code: vec![],
            individual_codes: vec![],
            list_environment_info: None,
            operation_analysis: None,
        };
        return Ok(warp::reply::with_status(
            warp::reply::json(&response),
            StatusCode::OK,
        ));
    }

    let (common_pattern_ast_option, hole_map) = find_common_pattern_and_holes(
        &parsed_programs,
        &all_memo_envs_for_pattern_finding,
        Some(&operations_list),
    );

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
    let individual_codes_str: Vec<String> =
        parsed_programs.iter().map(|p| format!("{}", p)).collect();

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

    let mut list_env_info = format!(
        "List Environment Summary:\n- {} objects tracked\n- {} literals\n- {} field types: {}\n\nCurrent state:\n{}",
        object_count,
        literal_count,
        field_count,
        field_names.join(", "),
        list_env.to_debug_string()
    );

    println!("List Environment Summary: {}", list_env_info);

    // 複数の操作列がある場合は操作分析も実行
    let mut unification_analysis: Option<UnificationAnalysisResult> = None;

    let operation_analysis_data = if operations_list.len() > 1 {
        println!(
            "Starting operation analysis with {} operation sequences",
            operations_list.len()
        );

        // unify_isomorphic_graphsを使って同型グラフ統合を試行
        match analyze_operations_with_unification(
            &req.vis_graph,
            &operations_list[0],
            &operations_list[1],
        ) {
            Ok(analysis_result) => {
                println!("Unification-based analysis succeeded!");
                unification_analysis = Some(analysis_result.clone());
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
                match analyze_operations_with_environments(
                    &req.vis_graph,
                    &operations_list[0],
                    &operations_list[1],
                ) {
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

    // Escher-Scala 用の JSON を1ファイルへ集約して保存
    let mut escher_written_paths: Vec<String> = Vec::new();
    let spec_base_name = derive_spec_base_name(&req.method_calls);
    let mut aggregated_specs: Vec<EscherSpec> = Vec::new();

    if operations_list.len() >= 2 {
        let vis_graph_a = req
            .method_calls
            .get(0)
            .and_then(|m| m.actual_graph.as_ref())
            .unwrap_or(&req.vis_graph);
        let vis_graph_b = req
            .method_calls
            .get(1)
            .and_then(|m| m.actual_graph.as_ref())
            .unwrap_or(&req.vis_graph);
        let base_env_a = crate::list_env::ListEnvironment::from_vis_graph(vis_graph_a);
        let base_env_b = crate::list_env::ListEnvironment::from_vis_graph(vis_graph_b);

        if let Some(uni) = &unification_analysis {
            match generate_specs_from_unification(
                vis_graph_a,
                vis_graph_b,
                &base_env_a,
                &base_env_b,
                &operations_list[0],
                &operations_list[1],
                uni,
                &spec_base_name,
            ) {
                Ok(specs) => {
                    if specs.is_empty() {
                        println!("Unification diff groups were empty; no Escher specs generated.");
                    } else {
                        aggregated_specs.extend(specs);
                    }
                }
                Err(e) => {
                    eprintln!(
                        "Failed to generate Escher specs from unification diffs: {}",
                        e
                    );
                }
            }
        } else {
            println!("Unification analysis unavailable; skipping Escher spec generation.");
        }
    }

    if !aggregated_specs.is_empty() {
        match specs_to_json(&aggregated_specs) {
            Ok(json_text) => {
                let ts = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_secs())
                    .unwrap_or(0);
                let out_path = format!(
                    "Escher-Scala/src/main/resources/escher/{}_{}.json",
                    spec_base_name, ts
                );
                if let Err(e) = write_spec_to_file(&out_path, &json_text) {
                    eprintln!("Failed to write aggregated Escher spec: {}", e);
                } else {
                    println!(
                        "Wrote aggregated Escher spec to {} ({} functions)",
                        out_path,
                        aggregated_specs.len()
                    );
                    escher_written_paths.push(out_path);
                }
            }
            Err(e) => eprintln!("Failed to serialize aggregated Escher specs: {}", e),
        }
    }

    // 生成した Escher JSON の保存先をレスポンスの情報欄に追記（フロントは未改修のため文字列に含める）
    if !escher_written_paths.is_empty() {
        list_env_info = format!(
            "{}\nEscher JSON saved: {}",
            list_env_info,
            escher_written_paths.join(", ")
        );
    }

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
    println!(
        "Common Pattern: {}",
        response
            .common_pattern
            .as_ref()
            .unwrap_or(&"None".to_string())
    );
    println!(
        "Holes: {}",
        response
            .hole_information
            .as_ref()
            .map(|h| format!("{:?}", h))
            .unwrap_or("None".to_string())
    );

    if let Some(analysis) = &response.operation_analysis {
        println!(
            "Operation Analysis Summary: {}",
            analysis.difference_summary
        );
        println!(
            "  - Raw operation differences: {}",
            analysis.differences_found
        );
        println!(
            "  - Synthesis common patterns: {}",
            analysis.synthesis_matches.unwrap_or(0)
        );
    } else {
        println!("Operation Analysis: Not performed (single operation sequence)");
    }

    println!(
        "List Environment Summary: {}",
        response
            .list_environment_info
            .as_ref()
            .unwrap_or(&"None".to_string())
    );
    println!("=====================================");

    Ok(warp::reply::with_status(
        warp::reply::json(&response),
        StatusCode::OK,
    ))
}

// unify_isomorphic_graphsを使用した統合ベースの操作分析
pub fn analyze_operations_with_unification(
    vis_graph: &models::VisGraph,
    operations_a: &[serde_json::Value],
    operations_b: &[serde_json::Value],
) -> anyhow::Result<UnificationAnalysisResult> {
    use crate::unify_ops::unify_operation_graphs;

    println!("=== UNIFICATION-BASED OPERATION ANALYSIS ===");

    // JSON操作をunify_ops::Op形式に変換
    let ops_a = convert_json_to_unify_ops(operations_a)?;
    let ops_b = convert_json_to_unify_ops(operations_b)?;

    println!("Converted {} operations from sequence A", ops_a.len());
    println!("Converted {} operations from sequence B", ops_b.len());

    // unify_isomorphic_graphsを呼び出し
    let unification_result = unify_operation_graphs(&ops_a, &ops_b);

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
        let list_env =
            create_environment_at_unification_boundary(vis_graph, &unification_result.common_a)?;
        println!(
            "Created List environment at unification boundary with {} common operations",
            common_count
        );
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
fn convert_json_to_unify_ops(
    operations: &[serde_json::Value],
) -> anyhow::Result<Vec<crate::unify_ops::Op>> {
    use crate::unify_ops::{EdgeExpr, GraphOp, NodeExpr, Op};

    // 1) デコードして保持
    let mut decoded: Vec<(usize, parser::Operation)> = Vec::new();
    for (i, op_val) in operations.iter().enumerate() {
        if let Ok(op) = serde_json::from_value::<parser::Operation>(op_val.clone()) {
            decoded.push((i, op));
        }
    }

    // 2) addNode を先に追加し、objId -> opId をマップ
    let mut result: Vec<Op> = Vec::new();
    let mut id_to_opnum: std::collections::HashMap<String, String> =
        std::collections::HashMap::new();
    for (i, op) in &decoded {
        if op.edit_type == "addNode" {
            if let (Some(id), Some(is_literal)) = (op.id.clone(), op.is_literal) {
                let op_id = format!("op_{}", i);
                let label = op.label.clone().unwrap_or_default();
                result.push(Op {
                    id: op_id.clone(),
                    kind: GraphOp::Node(NodeExpr::AddNode {
                        id: id.clone(),
                        label,
                        is_literal,
                    }),
                });
                id_to_opnum.insert(id, op_id);
            }
        }
    }

    // 3) 参照されるIDのうち、addNodeで定義されていないものに対して ExistNode を作る
    let mut referenced_ids: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    for (_i, op) in &decoded {
        match op.edit_type.as_str() {
            "addEdge" | "deleteEdge" => {
                if let Some(f) = &op.from {
                    referenced_ids.insert(f.clone());
                }
                if let Some(t) = &op.to {
                    referenced_ids.insert(t.clone());
                }
            }
            _ => {}
        }
    }
    let mut exist_idx: usize = 0;
    for obj_id in referenced_ids {
        if !id_to_opnum.contains_key(&obj_id) {
            let op_id = format!("op_exist_{}", exist_idx);
            exist_idx += 1;
            // is_literal/label は厳密には不要（ExistNodeの等価判定はidのみ）
            result.push(Op {
                id: op_id.clone(),
                kind: GraphOp::Node(NodeExpr::ExistNode {
                    id: obj_id.clone(),
                    label: String::new(),
                    is_literal: false,
                }),
            });
            id_to_opnum.insert(obj_id, op_id);
        }
    }

    // 4) エッジ系を opId に解決して追加
    for (i, op) in &decoded {
        match op.edit_type.as_str() {
            "addEdge" => {
                if let (Some(from), Some(to), Some(label)) =
                    (op.from.clone(), op.to.clone(), op.label.clone())
                {
                    if let (Some(from_op), Some(to_op)) =
                        (id_to_opnum.get(&from), id_to_opnum.get(&to))
                    {
                        result.push(Op {
                            id: format!("op_{}", i),
                            kind: GraphOp::Edge(EdgeExpr::AddEdge {
                                from: from_op.clone(),
                                to: to_op.clone(),
                                label,
                            }),
                        });
                    }
                }
            }
            "deleteEdge" => {
                if let (Some(from), Some(to)) = (op.from.clone(), op.to.clone()) {
                    let label = op.label.clone().unwrap_or_default();
                    if let (Some(from_op), Some(to_op)) =
                        (id_to_opnum.get(&from), id_to_opnum.get(&to))
                    {
                        result.push(Op {
                            id: format!("op_{}", i),
                            kind: GraphOp::Edge(EdgeExpr::DeleteEdge {
                                from: from_op.clone(),
                                to: to_op.clone(),
                                label,
                            }),
                        });
                    }
                }
            }
            "deleteNode" => {
                if let Some(id) = op.id.clone() {
                    // DeleteNode は ExistNode として扱う（構造用の存在参照）
                    result.push(Op {
                        id: format!("op_{}", i),
                        kind: GraphOp::Node(NodeExpr::ExistNode {
                            id,
                            label: op.label.clone().unwrap_or_default(),
                            is_literal: op.is_literal.unwrap_or(false),
                        }),
                    });
                }
            }
            _ => {}
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
    use crate::unify_ops::{EdgeExpr, GraphOp, NodeExpr};

    match &op.kind {
        GraphOp::Node(NodeExpr::AddNode {
            id,
            label,
            is_literal,
        }) => {
            // __RectForVariable__と__Variable-lstは無視
            if id == "__RectForVariable__" || id == "__Variable-lst" {
                return Ok(());
            }

            // ノード追加をList環境に反映
            if *is_literal {
                list_env
                    .literal_id_to_value
                    .insert(id.clone(), serde_json::Value::String(label.clone()));
            } else {
                // 新しいオブジェクトインデックスを追加
                let new_index = list_env.obj_id_to_index.len();
                list_env.obj_id_to_index.insert(id.clone(), new_index);
                list_env.index_to_obj_id.insert(new_index, id.clone());
            }
        }
        GraphOp::Edge(EdgeExpr::AddEdge { from, to, label }) => {
            // __RectForVariable__や__Variable-lstに関連するエッジは無視
            if from == "__RectForVariable__"
                || from == "__Variable-lst"
                || to == "__RectForVariable__"
                || to == "__Variable-lst"
            {
                return Ok(());
            }

            // エッジ追加をList環境に反映
            if let Some(&from_index) = list_env.obj_id_to_index.get(from) {
                // フィールドリストの更新
                let field_list = list_env
                    .field_lists
                    .entry(label.clone())
                    .or_insert_with(Vec::new);

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
        }
        GraphOp::Node(NodeExpr::ExistNode { .. }) => {
            // 既存ノード参照は環境変更なし
        }
        GraphOp::Node(NodeExpr::NullNode) => {
            // NullNode操作は環境変更なし
        }
        GraphOp::Edge(EdgeExpr::DeleteEdge { .. }) => {
            // エッジ削除は複雑なため、現在は未実装
        }
        GraphOp::Edge(EdgeExpr::EditEdgeReference { .. }) => {
            // エッジ編集は複雑なため、現在は未実装
        }
        GraphOp::Variable(_) => {
            // 変数操作は現在未実装
        }
    }

    Ok(())
}

// 統合分析結果の構造体
#[derive(Debug, Clone)]
pub struct UnificationAnalysisResult {
    pub common_operations_count: usize,
    pub total_operations_counts: Vec<usize>,
    pub differences_found: usize,
    pub unification_result: crate::unify_ops::UnificationResult,
}

// ===== helper fns for diff-based Escher output =====

fn analyze_fields_for_graph(vis_graph: &models::VisGraph) -> (Vec<String>, Vec<String>) {
    use std::collections::HashSet;
    let var_prefix = "__Variable-";
    let mut object_ids: HashSet<&str> = HashSet::new();
    let mut literal_ids: HashSet<&str> = HashSet::new();
    let mut variable_ids: HashSet<&str> = HashSet::new();

    for n in &vis_graph.nodes {
        if n.id == "__RectForVariable__" {
            continue;
        }
        if n.is_literal {
            literal_ids.insert(n.id.as_str());
        } else if n.id.starts_with(var_prefix) {
            variable_ids.insert(n.id.as_str());
        } else {
            object_ids.insert(n.id.as_str());
        }
    }

    let mut value_fields: HashSet<String> = HashSet::new();
    let mut pointer_fields: HashSet<String> = HashSet::new();
    for e in &vis_graph.edges {
        if e.from == "__RectForVariable__" || e.to == "__RectForVariable__" {
            continue;
        }
        if variable_ids.contains(e.from.as_str()) {
            continue;
        }
        if literal_ids.contains(e.to.as_str()) {
            value_fields.insert(e.label.clone());
        } else if object_ids.contains(e.to.as_str()) {
            pointer_fields.insert(e.label.clone());
        }
    }
    (
        value_fields.into_iter().collect(),
        pointer_fields.into_iter().collect(),
    )
}

fn detect_root_index(
    env: &crate::list_env::ListEnvironment,
    vis_graph: &models::VisGraph,
) -> Option<usize> {
    let var_prefix = "__Variable-";
    let mut var_ids: Vec<&str> = vis_graph
        .nodes
        .iter()
        .filter(|n| !n.is_literal)
        .map(|n| n.id.as_str())
        .filter(|id| id.starts_with(var_prefix))
        .collect();
    var_ids.sort();
    for var_id in var_ids {
        if let Some(&var_idx) = env.obj_id_to_index.get(var_id) {
            let var_name = var_id.trim_start_matches(var_prefix).to_string();
            if let Some(vec) = env.field_lists.get(&var_name) {
                if var_idx < vec.len() {
                    if let Some(root_i64) = vec[var_idx].as_i64() {
                        if root_i64 >= 0 {
                            return Some(root_i64 as usize);
                        }
                    }
                }
            }
        }
    }
    None
}

fn bfs_local_index_for_object(
    env: &crate::list_env::ListEnvironment,
    vis_graph: &models::VisGraph,
    object_id: &str,
) -> anyhow::Result<i32> {
    use std::collections::{HashMap, HashSet, VecDeque};
    let (_value_fields, mut pointer_fields) = analyze_fields_for_graph(vis_graph);
    pointer_fields.sort();
    let root =
        detect_root_index(env, vis_graph).ok_or_else(|| anyhow::anyhow!("root not found"))?;
    // Build adjacency using pointer fields (original indices)
    let mut adj: HashMap<usize, Vec<usize>> = HashMap::new();
    for pf in &pointer_fields {
        if let Some(vec) = env.field_lists.get(pf) {
            for (from_idx, v) in vec.iter().enumerate() {
                if let Some(to) = v.as_i64() {
                    if to >= 0 {
                        adj.entry(from_idx).or_default().push(to as usize);
                    }
                }
            }
        }
    }
    // BFS order and map
    let mut visited: HashSet<usize> = HashSet::new();
    let mut q: VecDeque<usize> = VecDeque::new();
    let mut order: Vec<usize> = Vec::new();
    q.push_back(root);
    while let Some(u) = q.pop_front() {
        if !visited.insert(u) {
            continue;
        }
        order.push(u);
        if let Some(ns) = adj.get(&u) {
            for &v in ns {
                if !visited.contains(&v) {
                    q.push_back(v);
                }
            }
        }
    }
    let mut idx_to_bfs: HashMap<usize, usize> = HashMap::new();
    for (bi, &orig) in order.iter().enumerate() {
        idx_to_bfs.insert(orig, bi);
    }
    // map object id to original index then to bfs index
    if let Some(&orig_idx) = env.obj_id_to_index.get(object_id) {
        Ok(idx_to_bfs
            .get(&orig_idx)
            .copied()
            .map(|x| x as i32)
            .unwrap_or(-1))
    } else {
        Ok(-1)
    }
}

fn build_environment_prefix(
    base_env: &crate::list_env::ListEnvironment,
    operations: &[serde_json::Value],
    position: usize,
    inclusive: bool,
) -> anyhow::Result<crate::list_env::ListEnvironment> {
    use crate::list_env::GraphOperation;

    let mut env = base_env.clone();
    let mut limit = position.min(operations.len());
    if inclusive {
        limit = limit.saturating_add(1).min(operations.len());
    }
    for op_json in operations.iter().take(limit) {
        let graph_op: GraphOperation = serde_json::from_value(op_json.clone())?;
        env.apply_operation(&graph_op)
            .map_err(|e| anyhow::anyhow!(e))?;
    }
    Ok(env)
}

#[derive(Clone)]
struct DiffPair {
    op_a: crate::list_env::GraphOperation,
    index_a: usize,
    op_b: crate::list_env::GraphOperation,
    index_b: usize,
}

#[derive(Debug, Hash, Eq, PartialEq, Clone, Ord, PartialOrd)]
struct DiffKey {
    parent_id: String,
    label: Option<String>,
    edit_type: String,
}

#[derive(Clone)]
struct DiffCandidate {
    graph_op: crate::list_env::GraphOperation,
    index: usize,
}

fn operations_equivalent_graph(
    op_a: &crate::list_env::GraphOperation,
    op_b: &crate::list_env::GraphOperation,
) -> bool {
    op_a.edit_type == op_b.edit_type
        && op_a.id == op_b.id
        && op_a.label == op_b.label
        && op_a.is_literal == op_b.is_literal
        && op_a.from == op_b.from
        && op_a.to == op_b.to
}

fn parse_op_index(op_id: &str) -> Option<usize> {
    op_id
        .strip_prefix("op_")
        .and_then(|rest| rest.parse::<usize>().ok())
}

fn extract_node_object_id(op: &crate::unify_ops::Op) -> Option<String> {
    use crate::unify_ops::{GraphOp, NodeExpr};

    match &op.kind {
        GraphOp::Node(NodeExpr::AddNode { id, .. }) => Some(id.clone()),
        GraphOp::Node(NodeExpr::ExistNode { id, .. }) => Some(id.clone()),
        _ => None,
    }
}

fn graph_op_parent_id(op: &crate::list_env::GraphOperation) -> Option<String> {
    match op.edit_type.as_str() {
        "addEdge" | "removeEdge" => op.from.clone(),
        "addNode" | "removeNode" => op.id.clone(),
        _ => None,
    }
}

fn graph_op_label_string(op: &crate::list_env::GraphOperation) -> Option<String> {
    op.label.as_ref().and_then(|value| match value {
        serde_json::Value::String(s) => Some(s.clone()),
        serde_json::Value::Number(num) => Some(num.to_string()),
        serde_json::Value::Bool(b) => Some(b.to_string()),
        _ => None,
    })
}

fn generate_specs_from_unification(
    vis_graph_a: &models::VisGraph,
    vis_graph_b: &models::VisGraph,
    base_env_a: &crate::list_env::ListEnvironment,
    base_env_b: &crate::list_env::ListEnvironment,
    operations_a: &[serde_json::Value],
    operations_b: &[serde_json::Value],
    analysis: &UnificationAnalysisResult,
    base_name: &str,
) -> anyhow::Result<Vec<EscherSpec>> {
    let graph_ops_a: Vec<crate::list_env::GraphOperation> = operations_a
        .iter()
        .map(|op| serde_json::from_value(op.clone()))
        .collect::<Result<_, _>>()?;
    let graph_ops_b: Vec<crate::list_env::GraphOperation> = operations_b
        .iter()
        .map(|op| serde_json::from_value(op.clone()))
        .collect::<Result<_, _>>()?;

    let mut diff_pairs = Vec::new();
    let mut used_indices_a: HashSet<usize> = HashSet::new();
    let mut used_indices_b: HashSet<usize> = HashSet::new();

    // Build op-id -> object-id lookup for nodes in both sequences
    let mut op_id_to_obj_a: HashMap<String, String> = HashMap::new();
    for op in analysis
        .unification_result
        .common_a
        .iter()
        .chain(analysis.unification_result.diff_a.iter())
    {
        if let Some(node_id) = extract_node_object_id(op) {
            op_id_to_obj_a.insert(op.id.clone(), node_id);
        }
    }
    let mut op_id_to_obj_b: HashMap<String, String> = HashMap::new();
    for op in analysis
        .unification_result
        .common_b
        .iter()
        .chain(analysis.unification_result.diff_b.iter())
    {
        if let Some(node_id) = extract_node_object_id(op) {
            op_id_to_obj_b.insert(op.id.clone(), node_id);
        }
    }

    // Convert final mapping to object-id mapping for parent tracking
    let mut object_id_mapping: HashMap<String, String> = HashMap::new();
    for (op_a_id, op_b_id) in &analysis.unification_result.final_mapping {
        if let (Some(obj_a), Some(obj_b)) =
            (op_id_to_obj_a.get(op_a_id), op_id_to_obj_b.get(op_b_id))
        {
            object_id_mapping.insert(obj_a.clone(), obj_b.clone());
        }
    }
    let mut object_id_mapping_inv: HashMap<String, String> = HashMap::new();
    for (a, b) in &object_id_mapping {
        object_id_mapping_inv.insert(b.clone(), a.clone());
    }

    let mut diff_map_a: HashMap<DiffKey, Vec<DiffCandidate>> = HashMap::new();
    for op in &analysis.unification_result.diff_a {
        if let Some(idx) = parse_op_index(&op.id) {
            if let Some(graph_op) = graph_ops_a.get(idx) {
                if let Some(parent_id) = graph_op_parent_id(graph_op) {
                    let key = DiffKey {
                        parent_id,
                        label: graph_op_label_string(graph_op),
                        edit_type: graph_op.edit_type.clone(),
                    };
                    diff_map_a.entry(key).or_default().push(DiffCandidate {
                        graph_op: graph_op.clone(),
                        index: idx,
                    });
                }
            }
        }
    }

    let mut diff_map_b: HashMap<DiffKey, Vec<DiffCandidate>> = HashMap::new();
    for op in &analysis.unification_result.diff_b {
        if let Some(idx) = parse_op_index(&op.id) {
            if let Some(graph_op) = graph_ops_b.get(idx) {
                if let Some(parent_id_b) = graph_op_parent_id(graph_op) {
                    let canonical_parent = object_id_mapping_inv
                        .get(&parent_id_b)
                        .cloned()
                        .unwrap_or(parent_id_b.clone());
                    let key = DiffKey {
                        parent_id: canonical_parent,
                        label: graph_op_label_string(graph_op),
                        edit_type: graph_op.edit_type.clone(),
                    };
                    diff_map_b.entry(key).or_default().push(DiffCandidate {
                        graph_op: graph_op.clone(),
                        index: idx,
                    });
                }
            }
        }
    }

    for candidates in diff_map_a.values_mut() {
        candidates.sort_by_key(|c| c.index);
    }
    for candidates in diff_map_b.values_mut() {
        candidates.sort_by_key(|c| c.index);
    }

    let mut sorted_keys: Vec<DiffKey> = diff_map_a.keys().cloned().collect();
    sorted_keys.sort();

    for key in sorted_keys {
        if let (Some(candidates_a), Some(candidates_b)) =
            (diff_map_a.get(&key), diff_map_b.get(&key))
        {
            let count = candidates_a.len().min(candidates_b.len());
            for i in 0..count {
                let cand_a = &candidates_a[i];
                let cand_b = &candidates_b[i];
                diff_pairs.push(DiffPair {
                    op_a: cand_a.graph_op.clone(),
                    index_a: cand_a.index,
                    op_b: cand_b.graph_op.clone(),
                    index_b: cand_b.index,
                });
                used_indices_a.insert(cand_a.index);
                used_indices_b.insert(cand_b.index);
            }
        }
    }

    let target_pairs = std::cmp::min(
        analysis.unification_result.diff_a.len(),
        analysis.unification_result.diff_b.len(),
    );

    if target_pairs > diff_pairs.len() {
        let mut diff_candidates_a: Vec<(usize, crate::list_env::GraphOperation)> = Vec::new();
        for op in &analysis.unification_result.diff_a {
            if let Some(idx) = parse_op_index(&op.id) {
                if used_indices_a.contains(&idx) {
                    continue;
                }
                if let Some(graph_op) = graph_ops_a.get(idx) {
                    diff_candidates_a.push((idx, graph_op.clone()));
                }
            }
        }
        diff_candidates_a.sort_by_key(|(idx, _)| *idx);

        let mut diff_candidates_b: Vec<(usize, crate::list_env::GraphOperation)> = Vec::new();
        for op in &analysis.unification_result.diff_b {
            if let Some(idx) = parse_op_index(&op.id) {
                if used_indices_b.contains(&idx) {
                    continue;
                }
                if let Some(graph_op) = graph_ops_b.get(idx) {
                    diff_candidates_b.push((idx, graph_op.clone()));
                }
            }
        }
        diff_candidates_b.sort_by_key(|(idx, _)| *idx);

        let pair_count = diff_candidates_a
            .len()
            .min(diff_candidates_b.len())
            .min(target_pairs.saturating_sub(diff_pairs.len()));
        for i in 0..pair_count {
            let (idx_a, op_a) = &diff_candidates_a[i];
            let (idx_b, op_b) = &diff_candidates_b[i];
            diff_pairs.push(DiffPair {
                op_a: op_a.clone(),
                index_a: *idx_a,
                op_b: op_b.clone(),
                index_b: *idx_b,
            });
            used_indices_a.insert(*idx_a);
            used_indices_b.insert(*idx_b);
        }
    }

    if target_pairs > diff_pairs.len() {
        let max_len = graph_ops_a.len().max(graph_ops_b.len());
        for idx in 0..max_len {
            if diff_pairs.len() >= target_pairs {
                break;
            }
            if used_indices_a.contains(&idx) || used_indices_b.contains(&idx) {
                continue;
            }

            let op_a = graph_ops_a.get(idx);
            let op_b = graph_ops_b.get(idx);

            match (op_a, op_b) {
                (Some(a), Some(b)) if operations_equivalent_graph(a, b) => continue,
                (Some(a), Some(b)) => {
                    diff_pairs.push(DiffPair {
                        op_a: a.clone(),
                        index_a: idx,
                        op_b: b.clone(),
                        index_b: idx,
                    });
                    used_indices_a.insert(idx);
                    used_indices_b.insert(idx);
                }
                _ => {}
            }
        }
    }

    let mut specs = Vec::new();
    let mut suffix_index = 0usize;

    for pair in diff_pairs {
        let env_a = build_environment_prefix(base_env_a, operations_a, pair.index_a, true)?;
        let env_b = build_environment_prefix(base_env_b, operations_b, pair.index_b, true)?;

        let out_a = determine_output_value(&pair.op_a, &env_a, vis_graph_a);
        let out_b = determine_output_value(&pair.op_b, &env_b, vis_graph_b);

        let root_idx_a = if let Some(idx) = detect_root_index(&env_a, vis_graph_a) {
            if let Some(obj_id) = env_a.index_to_obj_id.get(&idx) {
                bfs_local_index_for_object(&env_a, vis_graph_a, obj_id).unwrap_or(-1)
            } else {
                -1
            }
        } else {
            -1
        };
        let root_idx_b = if let Some(idx) = detect_root_index(&env_b, vis_graph_b) {
            if let Some(obj_id) = env_b.index_to_obj_id.get(&idx) {
                bfs_local_index_for_object(&env_b, vis_graph_b, obj_id).unwrap_or(-1)
            } else {
                -1
            }
        } else {
            -1
        };

        let cases = vec![
            EscherCase {
                env: env_a,
                vis_graph: vis_graph_a.clone(),
                arguments: vec![serde_json::json!(root_idx_a)],
                output: out_a,
            },
            EscherCase {
                env: env_b,
                vis_graph: vis_graph_b.clone(),
                arguments: vec![serde_json::json!(root_idx_b)],
                output: out_b,
            },
        ];

        let spec_name = build_spec_name(base_name, suffix_index);
        suffix_index += 1;

        let spec = build_escher_spec(&spec_name, "Int", &cases)?;
        specs.push(spec);
    }

    Ok(specs)
}

fn determine_output_value(
    operation: &crate::list_env::GraphOperation,
    env: &crate::list_env::ListEnvironment,
    vis_graph: &models::VisGraph,
) -> serde_json::Value {
    match operation.edit_type.as_str() {
        "addEdge" | "removeEdge" => {
            if let Some(to_id) = operation.to.as_deref() {
                if env.obj_id_to_index.contains_key(to_id) {
                    match bfs_local_index_for_object(env, vis_graph, to_id) {
                        Ok(idx) => serde_json::json!(idx),
                        Err(_) => serde_json::json!(-1),
                    }
                } else if let Some(val) = env.literal_id_to_value.get(to_id) {
                    value_to_i32(val)
                        .map(|v| serde_json::json!(v))
                        .unwrap_or_else(|| serde_json::json!(-1))
                } else {
                    serde_json::json!(-1)
                }
            } else {
                serde_json::json!(-1)
            }
        }
        "addNode" | "removeNode" => {
            if operation.is_literal.unwrap_or(false) {
                if let Some(label) = operation.label.as_ref() {
                    value_to_i32(label)
                        .map(|v| serde_json::json!(v))
                        .unwrap_or_else(|| serde_json::json!(-1))
                } else {
                    serde_json::json!(-1)
                }
            } else if let Some(id) = operation.id.as_deref() {
                match bfs_local_index_for_object(env, vis_graph, id) {
                    Ok(idx) => serde_json::json!(idx),
                    Err(_) => serde_json::json!(-1),
                }
            } else {
                serde_json::json!(-1)
            }
        }
        _ => serde_json::json!(-1),
    }
}

fn value_to_i32(val: &serde_json::Value) -> Option<i32> {
    match val {
        serde_json::Value::Number(num) => num.as_i64().and_then(|v| i32::try_from(v).ok()),
        serde_json::Value::String(s) => s.parse::<i64>().ok().and_then(|v| i32::try_from(v).ok()),
        serde_json::Value::Bool(b) => Some(if *b { 1 } else { 0 }),
        _ => None,
    }
}

fn build_spec_name(base: &str, idx: usize) -> String {
    let start = b'f';
    let letter = start.saturating_add(idx as u8);
    if letter <= b'z' {
        format!("{}-{}", base, letter as char)
    } else {
        format!("{}-f{}", base, idx)
    }
}

fn derive_spec_base_name(method_calls: &[MethodCallOperation]) -> String {
    let raw = method_calls
        .get(0)
        .map(|m| m.method_name.as_str())
        .unwrap_or("auto");
    let sanitized = sanitize_base_name(raw);
    if sanitized.is_empty() {
        "aux".to_string()
    } else {
        sanitized
    }
}

fn sanitize_base_name(name: &str) -> String {
    if name.starts_with("call") && name[4..].chars().all(|c| c.is_ascii_digit()) {
        "aux".to_string()
    } else {
        name.to_string()
    }
}
