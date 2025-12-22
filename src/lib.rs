pub mod env;
pub mod error;
pub mod escher_bridge;
pub mod isomorphism;
pub mod list_env;
pub mod models;
pub mod operation_analyzer;
pub mod server;
pub mod unify_ops;

use crate::escher_bridge::{
    build_escher_spec, run_escher_js, specs_to_json, write_spec_to_file, EscherCase,
    EscherJsOutcome, EscherSpec,
};
use anyhow;
use serde::{Deserialize, Serialize};
use serde_json;
use std::collections::{BTreeMap, HashMap, HashSet};
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub escher_results: Option<Vec<EscherJsOutcome>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OperationAnalysisData {
    pub common_operations_count: usize,
    pub total_operations_counts: Vec<usize>,
    pub difference_summary: String,
    pub differences_found: usize,
    pub synthesis_matches: Option<usize>, // 合成で見つかった共通パターン数
}

fn trace_enabled() -> bool {
    match std::env::var("REFSYN_TRACE") {
        Ok(v) => matches!(v.as_str(), "1" | "true" | "yes" | "on"),
        Err(_) => false,
    }
}

fn trace_json<T: serde::Serialize>(label: &str, value: &T) {
    if !trace_enabled() {
        return;
    }
    match serde_json::to_string_pretty(value) {
        Ok(json) => println!("TRACE: {}:\n{}", label, json),
        Err(err) => println!("TRACE: {}: <failed to serialize: {}>", label, err),
    }
}

// Types needed by the server module will be imported from main directly

pub async fn handle_synthesis(body: bytes::Bytes) -> Result<impl warp::Reply, warp::Rejection> {
    if let Ok(body_str) = std::str::from_utf8(&body) {
        println!("--- RAW REQUEST PAYLOAD ---");
        println!("{}", body_str);
        println!("---------------------------");
    }

    let req: SynthesisRequest = match serde_json::from_slice(&body) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("Deserialization error: {}", e);
            let response = SynthesisResponse {
                common_pattern: None,
                hole_information: None,
                code: vec![],
                individual_codes: vec![],
                list_environment_info: None,
                operation_analysis: None,
                escher_results: None,
            };
            return Ok(warp::reply::with_status(
                warp::reply::json(&response),
                StatusCode::BAD_REQUEST,
            ));
        }
    };

    if req.method_calls.is_empty() {
        println!("No method calls provided in the request.");
        let response = SynthesisResponse {
            common_pattern: None,
            hole_information: None,
            code: vec![],
            individual_codes: vec![],
            list_environment_info: None,
            operation_analysis: None,
            escher_results: None,
        };
        return Ok(warp::reply::with_status(
            warp::reply::json(&response),
            StatusCode::OK,
        ));
    }

    // Gather raw operation traces for later analysis/spec generation.
    let operations_list: Vec<Vec<serde_json::Value>> = req
        .method_calls
        .iter()
        .map(|call| call.operations.clone())
        .collect();

    // Provide a summary of the current list environment derived from the base VisGraph.
    use crate::list_env::ListEnvironment;
    let list_env = ListEnvironment::from_vis_graph(&req.vis_graph);
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

    // Run structural analysis across the first two traces (if present).
    let mut unification_analysis: Option<UnificationAnalysisResult> = None;
    let operation_analysis_data = if operations_list.len() > 1 {
        println!(
            "Starting operation analysis with {} operation sequences",
            operations_list.len()
        );
        match analyze_operations_with_unification(
            &req.vis_graph,
            &operations_list[0],
            &operations_list[1],
        ) {
            Ok(analysis_result) => {
                unification_analysis = Some(analysis_result.clone());
                let difference_summary = if analysis_result.common_operations_count == 0 {
                    "統合ベース分析: 共通する操作パターンが見つかりませんでした".to_string()
                } else {
                    format!(
                        "統合ベース分析: {}個の共通操作パターンを特定。差異部分では{}個と{}個の異なる操作。",
                        analysis_result.common_operations_count,
                        analysis_result.differences_found,
                        analysis_result
                            .total_operations_counts
                            .get(1)
                            .unwrap_or(&0)
                            - analysis_result.common_operations_count
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
                match crate::operation_analyzer::analyze_operations_with_environments(
                    &req.vis_graph,
                    &operations_list[0],
                    &operations_list[1],
                ) {
                    Ok(analysis_result) => {
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

    // Aggregate Escher specs when we have at least two traces.
    let mut escher_written_paths: Vec<String> = Vec::new();
    let spec_base_name = derive_spec_base_name(&req.method_calls);
    let mut aggregated_specs: Vec<EscherSpec> = Vec::new();
    let mut escher_json: Option<String> = None;
    let mut escher_outcomes: Option<Vec<EscherJsOutcome>> = None;
    let mut synthesized_codes: Vec<String> = Vec::new();
    let mut individual_codes: Vec<String> = Vec::new();

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
        let base_env_a = ListEnvironment::from_vis_graph(vis_graph_a);
        let base_env_b = ListEnvironment::from_vis_graph(vis_graph_b);

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
                escher_json = Some(json_text.clone());
                if trace_enabled() {
                    println!("TRACE: Escher JSON:\n{}", json_text);
                }
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

    if let Some(json_text) = escher_json.as_ref() {
        match run_escher_js(json_text) {
            Ok(results) => {
                let success_count = results.iter().filter(|r| r.success).count();
                let failure_count = results.len().saturating_sub(success_count);
                println!(
                    "Escher JS synthesis completed ({} success / {} failure)",
                    success_count, failure_count
                );

                for out in &results {
                    if let Some(rendered) = &out.rendered {
                        synthesized_codes.push(rendered.clone());
                        individual_codes.push(format!("{}: {}", out.name, rendered));
                    } else if let Some(err) = &out.error {
                        individual_codes.push(format!("{}: ERROR {}", out.name, err));
                    } else {
                        individual_codes.push(format!("{}: no output", out.name));
                    }
                }

                escher_outcomes = Some(results);
            }
            Err(e) => eprintln!("Escher JS synthesis failed: {}", e),
        }
    }

    if !escher_written_paths.is_empty() {
        list_env_info = format!(
            "{}\nEscher JSON saved: {}",
            list_env_info,
            escher_written_paths.join(", ")
        );
    }

    let response = SynthesisResponse {
        common_pattern: None,
        hole_information: None,
        code: synthesized_codes,
        individual_codes,
        list_environment_info: Some(list_env_info),
        operation_analysis: operation_analysis_data,
        escher_results: escher_outcomes,
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

    if response.code.is_empty() {
        println!("Synthesized code: none");
    } else {
        println!("Synthesized code:");
        for (i, c) in response.code.iter().enumerate() {
            println!("  [{}] {}", i, c);
        }
    }

    if let Some(results) = &response.escher_results {
        println!("Escher results:");
        for r in results {
            let status = if r.success { "OK" } else { "FAIL" };
            let rendered = r.rendered.as_deref().unwrap_or("<none>");
            let err = r.error.as_deref().unwrap_or("<no error>");
            if r.success {
                println!("  [{}] {} -> {}", status, r.name, rendered);
            } else {
                println!("  [{}] {} -> {}", status, r.name, err);
            }
        }
    } else {
        println!("Escher results: none");
    }

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
    trace_json("Unify ops A", &ops_a);
    trace_json("Unify ops B", &ops_b);

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
    if trace_enabled() {
        let mut mapping_pairs: Vec<(String, String)> = unification_result
            .final_mapping
            .iter()
            .map(|(a, b)| (a.clone(), b.clone()))
            .collect();
        mapping_pairs.sort();
        trace_json("Unification final_mapping", &mapping_pairs);
        trace_json("Unification common_a", &unification_result.common_a);
        trace_json("Unification common_b", &unification_result.common_b);
        trace_json("Unification diff_a", &unification_result.diff_a);
        trace_json("Unification diff_b", &unification_result.diff_b);
    }

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

    #[derive(Clone, serde::Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct OperationRecord {
        edit_type: String,
        id: Option<String>,
        label: Option<String>,
        is_literal: Option<bool>,
        from: Option<String>,
        to: Option<String>,
    }

    // 1) デコードして保持
    let mut decoded: Vec<(usize, OperationRecord)> = Vec::new();
    for (i, op_val) in operations.iter().enumerate() {
        if let Ok(op) = serde_json::from_value::<OperationRecord>(op_val.clone()) {
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

fn build_env_from_common_ops(
    base_env: &crate::list_env::ListEnvironment,
    graph_ops: &[crate::list_env::GraphOperation],
    common_ops: &[crate::unify_ops::Op],
) -> anyhow::Result<crate::list_env::ListEnvironment> {
    let mut env = base_env.clone();
    let mut indices: Vec<usize> = common_ops
        .iter()
        .filter_map(|op| parse_op_index(&op.id))
        .collect();
    indices.sort_unstable();
    indices.dedup();
    for idx in indices {
        if let Some(graph_op) = graph_ops.get(idx) {
            env.apply_operation(graph_op)
                .map_err(|e| anyhow::anyhow!(e))?;
        }
    }
    Ok(env)
}

#[derive(Clone)]
enum DiffOp {
    Json {
        graph_op: crate::list_env::GraphOperation,
        index: usize,
    },
    ExistNode {
        id: String,
        is_literal: bool,
        label: Option<String>,
    },
}

#[derive(Clone)]
struct DiffPair {
    op_a: Option<DiffOp>,
    op_b: Option<DiffOp>,
}

#[derive(Clone)]
struct ExistCandidate {
    id: String,
    is_literal: bool,
    label: Option<String>,
}

fn format_graph_op(graph_op: &crate::list_env::GraphOperation) -> String {
    let label = graph_op_label_string(graph_op).unwrap_or_else(|| "<none>".to_string());
    let id = graph_op.id.as_deref().unwrap_or("-");
    let from = graph_op.from.as_deref().unwrap_or("-");
    let to = graph_op.to.as_deref().unwrap_or("-");
    let is_literal = graph_op.is_literal.unwrap_or(false);
    format!(
        "{} id={} from={} to={} label={} is_literal={}",
        graph_op.edit_type, id, from, to, label, is_literal
    )
}

fn format_diff_op(op: &DiffOp) -> String {
    match op {
        DiffOp::Json { graph_op, .. } => format_graph_op(graph_op),
        DiffOp::ExistNode {
            id,
            is_literal,
            label,
        } => {
            let label_str = label.as_deref().unwrap_or("<none>");
            format!(
                "ExistNode id={} label={} is_literal={}",
                id, label_str, is_literal
            )
        }
    }
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

fn diff_op_index(op: &DiffOp) -> Option<usize> {
    match op {
        DiffOp::Json { index, .. } => Some(*index),
        DiffOp::ExistNode { .. } => None,
    }
}

fn diff_op_from_unify_node(
    op: &crate::unify_ops::Op,
    graph_ops: &[crate::list_env::GraphOperation],
) -> Option<DiffOp> {
    use crate::unify_ops::{GraphOp, NodeExpr};

    match &op.kind {
        GraphOp::Node(NodeExpr::AddNode { .. }) => {
            let idx = parse_op_index(&op.id)?;
            let graph_op = graph_ops.get(idx)?;
            Some(DiffOp::Json {
                graph_op: graph_op.clone(),
                index: idx,
            })
        }
        GraphOp::Node(NodeExpr::ExistNode {
            id,
            is_literal,
            label,
        }) => {
            let label = if label.is_empty() {
                None
            } else {
                Some(label.clone())
            };
            Some(DiffOp::ExistNode {
                id: id.clone(),
                is_literal: *is_literal,
                label,
            })
        }
        _ => None,
    }
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

fn canonicalize_object_id(id: &str, mapping: &HashMap<String, String>) -> String {
    mapping.get(id).cloned().unwrap_or_else(|| id.to_string())
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
    trace_json("GraphOperation A", &graph_ops_a);
    trace_json("GraphOperation B", &graph_ops_b);

    let env_boundary_a = build_env_from_common_ops(
        base_env_a,
        &graph_ops_a,
        &analysis.unification_result.common_a,
    )?;
    let env_boundary_b = build_env_from_common_ops(
        base_env_b,
        &graph_ops_b,
        &analysis.unification_result.common_b,
    )?;
    if trace_enabled() {
        println!(
            "TRACE: env_boundary_a:\n{}",
            env_boundary_a.to_debug_string()
        );
        println!(
            "TRACE: env_boundary_b:\n{}",
            env_boundary_b.to_debug_string()
        );
    }

    let mut diff_pairs = Vec::new();
    let mut used_indices_a: HashSet<usize> = HashSet::new();
    let mut used_indices_b: HashSet<usize> = HashSet::new();
    let mut used_diff_ids_a: HashSet<String> = HashSet::new();
    let mut used_diff_ids_b: HashSet<String> = HashSet::new();

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

    let mut op_a_by_id: HashMap<String, &crate::unify_ops::Op> = HashMap::new();
    for op in analysis
        .unification_result
        .common_a
        .iter()
        .chain(analysis.unification_result.diff_a.iter())
    {
        op_a_by_id.insert(op.id.clone(), op);
    }
    let mut op_b_by_id: HashMap<String, &crate::unify_ops::Op> = HashMap::new();
    for op in analysis
        .unification_result
        .common_b
        .iter()
        .chain(analysis.unification_result.diff_b.iter())
    {
        op_b_by_id.insert(op.id.clone(), op);
    }
    let diff_a_ids: HashSet<String> = analysis
        .unification_result
        .diff_a
        .iter()
        .map(|op| op.id.clone())
        .collect();
    let diff_b_ids: HashSet<String> = analysis
        .unification_result
        .diff_b
        .iter()
        .map(|op| op.id.clone())
        .collect();

    // Pair diff nodes using structural mapping so connected diffs stay aligned.
    for (op_a_id, op_b_id) in &analysis.unification_result.final_mapping {
        if used_diff_ids_a.contains(op_a_id) || used_diff_ids_b.contains(op_b_id) {
            continue;
        }
        if !diff_a_ids.contains(op_a_id) || !diff_b_ids.contains(op_b_id) {
            continue;
        }
        let op_a = match op_a_by_id.get(op_a_id) {
            Some(op) => *op,
            None => continue,
        };
        let op_b = match op_b_by_id.get(op_b_id) {
            Some(op) => *op,
            None => continue,
        };
        if !matches!(&op_a.kind, crate::unify_ops::GraphOp::Node(_))
            || !matches!(&op_b.kind, crate::unify_ops::GraphOp::Node(_))
        {
            continue;
        }
        let diff_a = match diff_op_from_unify_node(op_a, &graph_ops_a) {
            Some(op) => op,
            None => continue,
        };
        let diff_b = match diff_op_from_unify_node(op_b, &graph_ops_b) {
            Some(op) => op,
            None => continue,
        };
        if let Some(idx) = diff_op_index(&diff_a) {
            used_indices_a.insert(idx);
        }
        if let Some(idx) = diff_op_index(&diff_b) {
            used_indices_b.insert(idx);
        }
        used_diff_ids_a.insert(op_a_id.clone());
        used_diff_ids_b.insert(op_b_id.clone());
        diff_pairs.push(DiffPair {
            op_a: Some(diff_a),
            op_b: Some(diff_b),
        });
    }

    let mut diff_map_a: HashMap<DiffKey, Vec<DiffCandidate>> = HashMap::new();
    for op in &analysis.unification_result.diff_a {
        if used_diff_ids_a.contains(&op.id) {
            continue;
        }
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
        if used_diff_ids_b.contains(&op.id) {
            continue;
        }
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
                    op_a: Some(DiffOp::Json {
                        graph_op: cand_a.graph_op.clone(),
                        index: cand_a.index,
                    }),
                    op_b: Some(DiffOp::Json {
                        graph_op: cand_b.graph_op.clone(),
                        index: cand_b.index,
                    }),
                });
                used_indices_a.insert(cand_a.index);
                used_indices_b.insert(cand_b.index);
            }
        }
    }

    let mut addnode_a_by_id: BTreeMap<String, Vec<DiffOp>> = BTreeMap::new();
    let mut addnode_b_by_id: BTreeMap<String, Vec<DiffOp>> = BTreeMap::new();
    let mut exist_a_by_id: BTreeMap<String, Vec<ExistCandidate>> = BTreeMap::new();
    let mut exist_b_by_id: BTreeMap<String, Vec<ExistCandidate>> = BTreeMap::new();

    {
        use crate::unify_ops::{GraphOp, NodeExpr};

        for op in &analysis.unification_result.diff_a {
            if used_diff_ids_a.contains(&op.id) {
                continue;
            }
            if let Some(idx) = parse_op_index(&op.id) {
                if used_indices_a.contains(&idx) {
                    continue;
                }
                if let Some(graph_op) = graph_ops_a.get(idx) {
                    if graph_op.edit_type == "addNode" {
                        if let Some(id) = graph_op.id.as_deref() {
                            addnode_a_by_id
                                .entry(id.to_string())
                                .or_default()
                                .push(DiffOp::Json {
                                    graph_op: graph_op.clone(),
                                    index: idx,
                                });
                        }
                    }
                }
                continue;
            }

            if let GraphOp::Node(NodeExpr::ExistNode {
                id,
                is_literal,
                label,
            }) = &op.kind
            {
                let label = if label.is_empty() {
                    None
                } else {
                    Some(label.clone())
                };
                exist_a_by_id
                    .entry(id.clone())
                    .or_default()
                    .push(ExistCandidate {
                        id: id.clone(),
                        is_literal: *is_literal,
                        label,
                    });
            }
        }

        for op in &analysis.unification_result.diff_b {
            if used_diff_ids_b.contains(&op.id) {
                continue;
            }
            if let Some(idx) = parse_op_index(&op.id) {
                if used_indices_b.contains(&idx) {
                    continue;
                }
                if let Some(graph_op) = graph_ops_b.get(idx) {
                    if graph_op.edit_type == "addNode" {
                        if let Some(id) = graph_op.id.as_deref() {
                            let key = canonicalize_object_id(id, &object_id_mapping_inv);
                            addnode_b_by_id.entry(key).or_default().push(DiffOp::Json {
                                graph_op: graph_op.clone(),
                                index: idx,
                            });
                        }
                    }
                }
                continue;
            }

            if let GraphOp::Node(NodeExpr::ExistNode {
                id,
                is_literal,
                label,
            }) = &op.kind
            {
                let key = canonicalize_object_id(id, &object_id_mapping_inv);
                let label = if label.is_empty() {
                    None
                } else {
                    Some(label.clone())
                };
                exist_b_by_id.entry(key).or_default().push(ExistCandidate {
                    id: id.clone(),
                    is_literal: *is_literal,
                    label,
                });
            }
        }
    }

    for candidates in addnode_a_by_id.values_mut() {
        candidates.sort_by_key(|op| diff_op_index(op).unwrap_or(usize::MAX));
    }
    for candidates in addnode_b_by_id.values_mut() {
        candidates.sort_by_key(|op| diff_op_index(op).unwrap_or(usize::MAX));
    }

    let addnode_keys_a: Vec<String> = addnode_a_by_id.keys().cloned().collect();
    for key in addnode_keys_a {
        let mut remove_add = false;
        let mut remove_exist = false;
        {
            if let (Some(adds), Some(exists)) =
                (addnode_a_by_id.get_mut(&key), exist_b_by_id.get_mut(&key))
            {
                while !adds.is_empty() && !exists.is_empty() {
                    let add_op = adds.remove(0);
                    let exist = exists.remove(0);
                    if let Some(idx) = diff_op_index(&add_op) {
                        used_indices_a.insert(idx);
                    }
                    diff_pairs.push(DiffPair {
                        op_a: Some(add_op),
                        op_b: Some(DiffOp::ExistNode {
                            id: exist.id,
                            is_literal: exist.is_literal,
                            label: exist.label,
                        }),
                    });
                }
                if adds.is_empty() {
                    remove_add = true;
                }
                if exists.is_empty() {
                    remove_exist = true;
                }
            }
        }
        if remove_add {
            addnode_a_by_id.remove(&key);
        }
        if remove_exist {
            exist_b_by_id.remove(&key);
        }
    }

    let addnode_keys_b: Vec<String> = addnode_b_by_id.keys().cloned().collect();
    for key in addnode_keys_b {
        let mut remove_add = false;
        let mut remove_exist = false;
        {
            if let (Some(adds), Some(exists)) =
                (addnode_b_by_id.get_mut(&key), exist_a_by_id.get_mut(&key))
            {
                while !adds.is_empty() && !exists.is_empty() {
                    let add_op = adds.remove(0);
                    let exist = exists.remove(0);
                    if let Some(idx) = diff_op_index(&add_op) {
                        used_indices_b.insert(idx);
                    }
                    diff_pairs.push(DiffPair {
                        op_a: Some(DiffOp::ExistNode {
                            id: exist.id,
                            is_literal: exist.is_literal,
                            label: exist.label,
                        }),
                        op_b: Some(add_op),
                    });
                }
                if adds.is_empty() {
                    remove_add = true;
                }
                if exists.is_empty() {
                    remove_exist = true;
                }
            }
        }
        if remove_add {
            addnode_b_by_id.remove(&key);
        }
        if remove_exist {
            exist_a_by_id.remove(&key);
        }
    }

    let exist_keys: Vec<String> = exist_a_by_id
        .keys()
        .filter(|key| exist_b_by_id.contains_key(*key))
        .cloned()
        .collect();
    for key in exist_keys {
        let mut remove_a = false;
        let mut remove_b = false;
        {
            if let (Some(exists_a), Some(exists_b)) =
                (exist_a_by_id.get_mut(&key), exist_b_by_id.get_mut(&key))
            {
                while !exists_a.is_empty() && !exists_b.is_empty() {
                    let exist_a = exists_a.remove(0);
                    let exist_b = exists_b.remove(0);
                    diff_pairs.push(DiffPair {
                        op_a: Some(DiffOp::ExistNode {
                            id: exist_a.id,
                            is_literal: exist_a.is_literal,
                            label: exist_a.label,
                        }),
                        op_b: Some(DiffOp::ExistNode {
                            id: exist_b.id,
                            is_literal: exist_b.is_literal,
                            label: exist_b.label,
                        }),
                    });
                }
                if exists_a.is_empty() {
                    remove_a = true;
                }
                if exists_b.is_empty() {
                    remove_b = true;
                }
            }
        }
        if remove_a {
            exist_a_by_id.remove(&key);
        }
        if remove_b {
            exist_b_by_id.remove(&key);
        }
    }

    let mut remaining_a: Vec<DiffOp> = Vec::new();
    for op in &analysis.unification_result.diff_a {
        if used_diff_ids_a.contains(&op.id) {
            continue;
        }
        if let Some(idx) = parse_op_index(&op.id) {
            if used_indices_a.contains(&idx) {
                continue;
            }
            if let Some(graph_op) = graph_ops_a.get(idx) {
                if graph_op.edit_type == "addNode" {
                    continue;
                }
                remaining_a.push(DiffOp::Json {
                    graph_op: graph_op.clone(),
                    index: idx,
                });
                used_indices_a.insert(idx);
            }
        }
    }
    remaining_a.sort_by_key(|op| diff_op_index(op).unwrap_or(usize::MAX));
    for op in remaining_a {
        diff_pairs.push(DiffPair {
            op_a: Some(op),
            op_b: None,
        });
    }

    let mut remaining_b: Vec<DiffOp> = Vec::new();
    for op in &analysis.unification_result.diff_b {
        if used_diff_ids_b.contains(&op.id) {
            continue;
        }
        if let Some(idx) = parse_op_index(&op.id) {
            if used_indices_b.contains(&idx) {
                continue;
            }
            if let Some(graph_op) = graph_ops_b.get(idx) {
                if graph_op.edit_type == "addNode" {
                    continue;
                }
                remaining_b.push(DiffOp::Json {
                    graph_op: graph_op.clone(),
                    index: idx,
                });
                used_indices_b.insert(idx);
            }
        }
    }
    remaining_b.sort_by_key(|op| diff_op_index(op).unwrap_or(usize::MAX));
    for op in remaining_b {
        diff_pairs.push(DiffPair {
            op_a: None,
            op_b: Some(op),
        });
    }

    for (_key, mut ops) in addnode_a_by_id {
        ops.sort_by_key(|op| diff_op_index(op).unwrap_or(usize::MAX));
        for op in ops {
            diff_pairs.push(DiffPair {
                op_a: Some(op),
                op_b: None,
            });
        }
    }
    for (_key, mut ops) in addnode_b_by_id {
        ops.sort_by_key(|op| diff_op_index(op).unwrap_or(usize::MAX));
        for op in ops {
            diff_pairs.push(DiffPair {
                op_a: None,
                op_b: Some(op),
            });
        }
    }
    for (_key, exists) in exist_a_by_id {
        for exist in exists {
            diff_pairs.push(DiffPair {
                op_a: Some(DiffOp::ExistNode {
                    id: exist.id,
                    is_literal: exist.is_literal,
                    label: exist.label,
                }),
                op_b: None,
            });
        }
    }
    for (_key, exists) in exist_b_by_id {
        for exist in exists {
            diff_pairs.push(DiffPair {
                op_a: None,
                op_b: Some(DiffOp::ExistNode {
                    id: exist.id,
                    is_literal: exist.is_literal,
                    label: exist.label,
                }),
            });
        }
    }

    if trace_enabled() {
        println!("TRACE: diff_pairs ({}):", diff_pairs.len());
        for (i, pair) in diff_pairs.iter().enumerate() {
            let a_desc = pair
                .op_a
                .as_ref()
                .map(format_diff_op)
                .unwrap_or_else(|| "<none>".to_string());
            let b_desc = pair
                .op_b
                .as_ref()
                .map(format_diff_op)
                .unwrap_or_else(|| "<none>".to_string());
            println!("  [{}] A: {} | B: {}", i, a_desc, b_desc);
        }
    }

    let mut specs = Vec::new();
    let mut suffix_index = 0usize;

    for (pair_index, pair) in diff_pairs.into_iter().enumerate() {
        let env_a = match &pair.op_a {
            Some(DiffOp::Json { index, .. }) => {
                build_environment_prefix(base_env_a, operations_a, *index, true)?
            }
            _ => env_boundary_a.clone(),
        };
        let env_b = match &pair.op_b {
            Some(DiffOp::Json { index, .. }) => {
                build_environment_prefix(base_env_b, operations_b, *index, true)?
            }
            _ => env_boundary_b.clone(),
        };
        if trace_enabled() {
            println!(
                "TRACE: diff_pair[{}] env_a:\n{}",
                pair_index,
                env_a.to_debug_string()
            );
            println!(
                "TRACE: diff_pair[{}] env_b:\n{}",
                pair_index,
                env_b.to_debug_string()
            );
        }

        let (mut out_a, out_ty_a) = match &pair.op_a {
            Some(op) => {
                let (out, ty) = output_for_diff_op(op, &env_a, vis_graph_a);
                (out, Some(ty))
            }
            None => (serde_json::json!(-1), None),
        };
        let (mut out_b, out_ty_b) = match &pair.op_b {
            Some(op) => {
                let (out, ty) = output_for_diff_op(op, &env_b, vis_graph_b);
                (out, Some(ty))
            }
            None => (serde_json::json!(-1), None),
        };

        let return_type = match (out_ty_a, out_ty_b) {
            (Some(OutputType::Ptr), Some(OutputType::Ptr)) => OutputType::Ptr,
            (Some(OutputType::Int), Some(OutputType::Int)) => OutputType::Int,
            (Some(OutputType::Ptr), Some(OutputType::Int))
            | (Some(OutputType::Int), Some(OutputType::Ptr)) => OutputType::Int,
            (Some(ty), None) | (None, Some(ty)) => ty,
            (None, None) => OutputType::Int,
        };

        if pair.op_a.is_none() {
            out_a = missing_output_for_type(return_type);
        }
        if pair.op_b.is_none() {
            out_b = missing_output_for_type(return_type);
        }
        if trace_enabled() {
            let out_a_str =
                serde_json::to_string(&out_a).unwrap_or_else(|_| format!("{:?}", out_a));
            let out_b_str =
                serde_json::to_string(&out_b).unwrap_or_else(|_| format!("{:?}", out_b));
            println!(
                "TRACE: diff_pair[{}] outputs A={} ({:?}) B={} ({:?}) return={:?}",
                pair_index, out_a_str, out_ty_a, out_b_str, out_ty_b, return_type
            );
        }

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

        let spec = build_escher_spec(&spec_name, return_type.as_escher_type(), &cases)?;
        trace_json(&format!("EscherSpec {}", spec.name), &spec);
        specs.push(spec);
    }

    Ok(specs)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OutputType {
    Int,
    Ptr,
}

impl OutputType {
    fn as_escher_type(self) -> &'static str {
        match self {
            OutputType::Int => "Int",
            OutputType::Ptr => "Ptr",
        }
    }
}

fn determine_output_value(
    operation: &crate::list_env::GraphOperation,
    env: &crate::list_env::ListEnvironment,
    vis_graph: &models::VisGraph,
) -> (serde_json::Value, OutputType) {
    match operation.edit_type.as_str() {
        "addEdge" | "removeEdge" => {
            if let Some(to_id) = operation.to.as_deref() {
                if env.obj_id_to_index.contains_key(to_id) {
                    let value = match bfs_local_index_for_object(env, vis_graph, to_id) {
                        Ok(idx) => serde_json::json!(idx),
                        Err(_) => serde_json::json!(-1),
                    };
                    (value, OutputType::Ptr)
                } else if let Some(val) = env.literal_id_to_value.get(to_id) {
                    let value = value_to_i32(val)
                        .map(|v| serde_json::json!(v))
                        .unwrap_or_else(|| serde_json::json!(-1));
                    (value, OutputType::Int)
                } else {
                    (serde_json::json!(-1), OutputType::Int)
                }
            } else {
                (serde_json::json!(-1), OutputType::Int)
            }
        }
        "addNode" | "removeNode" => {
            if operation.is_literal.unwrap_or(false) {
                if let Some(label) = operation.label.as_ref() {
                    let value = value_to_i32(label)
                        .map(|v| serde_json::json!(v))
                        .unwrap_or_else(|| serde_json::json!(-1));
                    (value, OutputType::Int)
                } else {
                    (serde_json::json!(-1), OutputType::Int)
                }
            } else if let Some(id) = operation.id.as_deref() {
                let value = match bfs_local_index_for_object(env, vis_graph, id) {
                    Ok(idx) => serde_json::json!(idx),
                    Err(_) => serde_json::json!(-1),
                };
                (value, OutputType::Ptr)
            } else {
                (serde_json::json!(-1), OutputType::Int)
            }
        }
        _ => (serde_json::json!(-1), OutputType::Int),
    }
}

fn output_for_diff_op(
    op: &DiffOp,
    env: &crate::list_env::ListEnvironment,
    vis_graph: &models::VisGraph,
) -> (serde_json::Value, OutputType) {
    match op {
        DiffOp::Json { graph_op, .. } => determine_output_value(graph_op, env, vis_graph),
        DiffOp::ExistNode {
            id,
            is_literal,
            label,
        } => {
            determine_output_value_for_exist_node(id, *is_literal, label.as_deref(), env, vis_graph)
        }
    }
}

fn determine_output_value_for_exist_node(
    id: &str,
    is_literal: bool,
    label: Option<&str>,
    env: &crate::list_env::ListEnvironment,
    vis_graph: &models::VisGraph,
) -> (serde_json::Value, OutputType) {
    if is_literal {
        let value = label
            .map(|s| serde_json::Value::String(s.to_string()))
            .and_then(|v| value_to_i32(&v))
            .map(|v| serde_json::json!(v))
            .unwrap_or_else(|| serde_json::json!(-1));
        (value, OutputType::Int)
    } else {
        let value = match bfs_local_index_for_object(env, vis_graph, id) {
            Ok(idx) => serde_json::json!(idx),
            Err(_) => serde_json::json!(-1),
        };
        (value, OutputType::Ptr)
    }
}

fn missing_output_for_type(output_type: OutputType) -> serde_json::Value {
    match output_type {
        OutputType::Ptr => serde_json::Value::Null,
        OutputType::Int => serde_json::json!(-1),
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn simple_vis_graph_with_root() -> VisGraph {
        VisGraph {
            nodes: vec![
                crate::models::Node {
                    id: "__Variable-lst".to_string(),
                    is_literal: false,
                    label: json!("lst"),
                },
                crate::models::Node {
                    id: "obj1".to_string(),
                    is_literal: false,
                    label: json!("Node"),
                },
            ],
            edges: vec![crate::models::Edge {
                from: "__Variable-lst".to_string(),
                to: "obj1".to_string(),
                label: "lst".to_string(),
            }],
        }
    }

    #[test]
    fn exist_node_ptr_output_uses_bfs_index() {
        let vis_graph = simple_vis_graph_with_root();
        let env = list_env::ListEnvironment::from_vis_graph(&vis_graph);
        let (value, ty) =
            determine_output_value_for_exist_node("obj1", false, None, &env, &vis_graph);
        assert_eq!(ty, OutputType::Ptr);
        assert_eq!(value, json!(0));
    }

    #[test]
    fn exist_node_literal_output_is_int() {
        let vis_graph = simple_vis_graph_with_root();
        let env = list_env::ListEnvironment::from_vis_graph(&vis_graph);
        let (value, ty) =
            determine_output_value_for_exist_node("lit1", true, Some("3"), &env, &vis_graph);
        assert_eq!(ty, OutputType::Int);
        assert_eq!(value, json!(3));
    }

    #[test]
    fn missing_output_sentinels() {
        assert_eq!(
            missing_output_for_type(OutputType::Ptr),
            serde_json::Value::Null
        );
        assert_eq!(missing_output_for_type(OutputType::Int), json!(-1));
    }
}
