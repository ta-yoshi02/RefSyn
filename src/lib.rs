pub mod env;
pub mod error;
pub mod escher_bridge;
pub mod escher_js;
pub mod isomorphism;
pub mod list_env;
pub mod models;
pub mod operation_analyzer;
pub mod server;
pub mod unify_ops;

use crate::escher_bridge::{
    build_escher_spec, derive_spec_meta_with_fields, resolve_field_order, run_escher_js,
    specs_to_json, write_spec_to_file, EscherCase, EscherJsOutcome, EscherSpec, EscherSpecMeta,
    ExampleJson,
};
use crate::escher_js::{build_context_from_spec, translate_rendered_method};
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
    #[serde(default)]
    pub arguments: Vec<serde_json::Value>,
    #[serde(rename = "argumentTypes")]
    pub argument_types: Option<Vec<String>>,
    #[serde(rename = "argumentNames")]
    pub argument_names: Option<Vec<String>>,
    #[serde(rename = "methodParamNames")]
    pub method_param_names: Option<Vec<String>>,
    pub operations: Vec<serde_json::Value>,
    #[serde(rename = "precondGraph")]
    pub precond_graph: Option<VisGraph>,
    #[serde(rename = "actualGraph")]
    pub actual_graph: Option<VisGraph>,
    #[serde(rename = "idMapping")]
    pub id_mapping: Option<HashMap<String, String>>,
    #[serde(rename = "fieldTables")]
    pub field_tables: Option<FieldTables>,
}

use crate::models::VisGraph;

#[derive(Deserialize, Serialize, Debug)]
pub struct SynthesisRequest {
    pub method_calls: Vec<MethodCallOperation>,
    pub vis_graph: VisGraph,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FieldTables {
    pub value: Vec<String>,
    pub pointer: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SynthesisResponse {
    pub common_pattern: Option<String>,
    pub hole_information: Option<HashMap<String, Vec<String>>>,
    pub code: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub composed_method_code: Option<String>,
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

#[derive(Debug, Clone)]
struct CommonPlanArtifact {
    pattern_text: String,
    hole_information: HashMap<String, Vec<String>>,
    composed_method_code: Option<String>,
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
                composed_method_code: None,
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
            composed_method_code: None,
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

    let unique_method_names = collect_unique_method_names(&req.method_calls);
    if unique_method_names.len() > 1 {
        let message = format!(
            "Mismatched method names in method_calls: {}",
            unique_method_names.join(", ")
        );
        eprintln!("{}", message);
        let response = SynthesisResponse {
            common_pattern: None,
            hole_information: None,
            code: vec![],
            composed_method_code: None,
            individual_codes: vec![],
            list_environment_info: Some(message),
            operation_analysis: None,
            escher_results: None,
        };
        return Ok(warp::reply::with_status(
            warp::reply::json(&response),
            StatusCode::BAD_REQUEST,
        ));
    }

    // Normalize runtime-scoped IDs in operations by using all known temp/runtime mappings.
    let runtime_to_temp_map = build_runtime_to_temp_map(&req.method_calls);
    let operations_list: Vec<Vec<serde_json::Value>> = req
        .method_calls
        .iter()
        .map(|call| normalize_operations_with_runtime_map(&call.operations, &runtime_to_temp_map))
        .collect();
    if let Some(message) = detect_unresolved_runtime_scoped_ids(&operations_list) {
        eprintln!("{}", message);
        let response = SynthesisResponse {
            common_pattern: None,
            hole_information: None,
            code: vec![],
            composed_method_code: None,
            individual_codes: vec![],
            list_environment_info: Some(message),
            operation_analysis: None,
            escher_results: None,
        };
        return Ok(warp::reply::with_status(
            warp::reply::json(&response),
            StatusCode::BAD_REQUEST,
        ));
    }

    if let Some(message) = detect_unsupported_remove_operation(&operations_list) {
        eprintln!("{}", message);
        let response = SynthesisResponse {
            common_pattern: None,
            hole_information: None,
            code: vec![],
            composed_method_code: None,
            individual_codes: vec![],
            list_environment_info: Some(message),
            operation_analysis: None,
            escher_results: None,
        };
        return Ok(warp::reply::with_status(
            warp::reply::json(&response),
            StatusCode::BAD_REQUEST,
        ));
    }

    let resolved_call_graphs: Vec<VisGraph> = req
        .method_calls
        .iter()
        .map(|call| resolve_method_call_vis_graph(call, &req.vis_graph, &runtime_to_temp_map))
        .collect();
    let primary_vis_graph = resolved_call_graphs.first().unwrap_or(&req.vis_graph);

    // Provide a summary of the current list environment derived from the base VisGraph.
    use crate::list_env::ListEnvironment;
    let list_env = ListEnvironment::from_vis_graph(primary_vis_graph);
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

    // Run structural analysis.
    let mut unification_analysis: Option<UnificationAnalysisResult> = None;
    let mut incremental_unification: Option<IncrementalUnificationResult> = None;
    let operation_analysis_data = if operations_list.len() > 1 {
        println!(
            "Starting operation analysis with {} operation sequences",
            operations_list.len()
        );
        if operations_list.len() == 2 {
            match analyze_operations_with_unification(
                primary_vis_graph,
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
                        primary_vis_graph,
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
            match analyze_operations_incrementally(&operations_list) {
                Ok(result) => {
                    incremental_unification = Some(result.clone());
                    let difference_summary = if result.common_operations_count == 0 {
                        format!(
                            "統合ベース分析（多仕様）: {}件の仕様に共通する操作が見つかりませんでした",
                            operations_list.len()
                        )
                    } else {
                        format!(
                            "統合ベース分析（多仕様）: {}件の仕様に共通する操作を{}個特定。各仕様との差分は最大{}個。",
                            operations_list.len(),
                            result.common_operations_count,
                            result.differences_found
                        )
                    };

                    Some(OperationAnalysisData {
                        common_operations_count: result.common_operations_count,
                        total_operations_counts: result.total_operations_counts.clone(),
                        difference_summary,
                        differences_found: result.differences_found,
                        synthesis_matches: Some(result.common_operations_count),
                    })
                }
                Err(e) => {
                    eprintln!("Error during incremental unification analysis: {}", e);
                    None
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
    let mut spec_meta_by_name: HashMap<String, EscherSpecMeta> = HashMap::new();
    let mut common_plan_artifact: Option<CommonPlanArtifact> = None;
    let mut escher_json: Option<String> = None;
    let mut escher_outcomes: Option<Vec<EscherJsOutcome>> = None;
    let mut synthesized_codes: Vec<String> = Vec::new();
    let mut individual_codes: Vec<String> = Vec::new();

    if operations_list.len() >= 2 {
        if operations_list.len() == 2 {
            let vis_graph_a = resolved_call_graphs.get(0).unwrap_or(primary_vis_graph);
            let vis_graph_b = resolved_call_graphs.get(1).unwrap_or(primary_vis_graph);
            let base_env_a = ListEnvironment::from_vis_graph(vis_graph_a);
            let base_env_b = ListEnvironment::from_vis_graph(vis_graph_b);
            let declared_receiver_a = req.method_calls.get(0).map(|m| m.receiver_object.as_str());
            let declared_receiver_b = req.method_calls.get(1).map(|m| m.receiver_object.as_str());
            let receiver_object_a =
                resolve_effective_receiver_object(declared_receiver_a, &base_env_a, vis_graph_a);
            let receiver_object_b =
                resolve_effective_receiver_object(declared_receiver_b, &base_env_b, vis_graph_b);

            if let Some(uni) = &unification_analysis {
                let merged_field_tables = merge_field_tables(
                    req.method_calls
                        .get(0)
                        .and_then(|m| m.field_tables.as_ref()),
                    req.method_calls
                        .get(1)
                        .and_then(|m| m.field_tables.as_ref()),
                );
                match generate_specs_from_unification(
                    vis_graph_a,
                    vis_graph_b,
                    &base_env_a,
                    &base_env_b,
                    receiver_object_a.as_deref(),
                    receiver_object_b.as_deref(),
                    req.method_calls
                        .get(0)
                        .map(|m| m.arguments.as_slice())
                        .unwrap_or(&[]),
                    req.method_calls
                        .get(1)
                        .map(|m| m.arguments.as_slice())
                        .unwrap_or(&[]),
                    req.method_calls
                        .get(0)
                        .and_then(|m| m.argument_types.as_deref()),
                    req.method_calls
                        .get(1)
                        .and_then(|m| m.argument_types.as_deref()),
                    req.method_calls
                        .get(0)
                        .and_then(|m| m.argument_names.as_deref()),
                    req.method_calls
                        .get(1)
                        .and_then(|m| m.argument_names.as_deref()),
                    req.method_calls
                        .get(0)
                        .and_then(|m| m.method_param_names.as_deref()),
                    req.method_calls
                        .get(1)
                        .and_then(|m| m.method_param_names.as_deref()),
                    &operations_list[0],
                    &operations_list[1],
                    uni,
                    &spec_base_name,
                    req.method_calls
                        .get(0)
                        .map(|m| m.method_name.as_str())
                        .unwrap_or("method"),
                    merged_field_tables.as_ref(),
                    &mut spec_meta_by_name,
                ) {
                    Ok(spec_result) => {
                        let GeneratedSpecsResult { specs, common_plan } = spec_result;
                        if specs.is_empty() {
                            println!(
                                "Unification diff groups were empty; no Escher specs generated."
                            );
                        } else {
                            aggregated_specs.extend(specs);
                        }
                        if common_plan_artifact.is_none() {
                            common_plan_artifact = common_plan;
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
        } else if let Some(incremental) = &incremental_unification {
            let reference_call = match req.method_calls.first() {
                Some(call) => call,
                None => unreachable!("method_calls is non-empty when operations_list.len() >= 2"),
            };
            let global_field_tables = infer_field_tables_for_method_calls(
                &req.method_calls,
                primary_vis_graph,
                &runtime_to_temp_map,
            );
            let vis_graph_consensus = resolved_call_graphs.get(0).unwrap_or(primary_vis_graph);
            let base_env_consensus = ListEnvironment::from_vis_graph(vis_graph_consensus);
            let receiver_object_consensus = resolve_effective_receiver_object(
                Some(reference_call.receiver_object.as_str()),
                &base_env_consensus,
                vis_graph_consensus,
            );
            let mut multi_candidates: Vec<MultiTraceSpecCandidate> = Vec::new();
            let mut multi_common_plans: Vec<CommonPlanArtifact> = Vec::new();

            for (call_index, call) in req.method_calls.iter().enumerate() {
                let vis_graph_b = resolved_call_graphs
                    .get(call_index)
                    .unwrap_or(vis_graph_consensus);
                let base_env_b = ListEnvironment::from_vis_graph(vis_graph_b);
                let receiver_object_b = resolve_effective_receiver_object(
                    Some(call.receiver_object.as_str()),
                    &base_env_b,
                    vis_graph_b,
                );

                let analysis = match analyze_operations_pair_unification(
                    &incremental.consensus_operations,
                    &operations_list[call_index],
                ) {
                    Ok(analysis) => analysis,
                    Err(e) => {
                        eprintln!(
                            "Failed to compare consensus against call {}: {}",
                            call_index, e
                        );
                        continue;
                    }
                };

                let pair_base_name = format!("{}-trace{}", spec_base_name, call_index);

                match generate_specs_from_unification(
                    vis_graph_consensus,
                    vis_graph_b,
                    &base_env_consensus,
                    &base_env_b,
                    receiver_object_consensus.as_deref(),
                    receiver_object_b.as_deref(),
                    reference_call.arguments.as_slice(),
                    call.arguments.as_slice(),
                    reference_call.argument_types.as_deref(),
                    call.argument_types.as_deref(),
                    reference_call.argument_names.as_deref(),
                    call.argument_names.as_deref(),
                    reference_call.method_param_names.as_deref(),
                    call.method_param_names.as_deref(),
                    &incremental.consensus_operations,
                    &operations_list[call_index],
                    &analysis,
                    &pair_base_name,
                    reference_call.method_name.as_str(),
                    global_field_tables.as_ref(),
                    &mut spec_meta_by_name,
                ) {
                    Ok(spec_result) => {
                        let GeneratedSpecsResult { specs, common_plan } = spec_result;
                        if let Some(plan) = common_plan.clone() {
                            multi_common_plans.push(plan);
                        }
                        if specs.is_empty() {
                            println!(
                                "No Escher specs generated for consensus vs call {}",
                                call_index
                            );
                        } else if let Some(plan) = common_plan.as_ref() {
                            let descriptors = parse_hole_descriptors_by_spec(plan);
                            for spec in specs {
                                let Some(meta) = spec_meta_by_name.get(&spec.name).cloned() else {
                                    continue;
                                };
                                let Some(desc) = descriptors.get(&spec.name) else {
                                    continue;
                                };
                                let Some(signature) = hole_signature(desc) else {
                                    continue;
                                };
                                let Some(trace_example) = spec.examples.get(1).cloned() else {
                                    continue;
                                };
                                let reference_example = spec.examples.get(0).cloned();
                                let mut trace_spec = spec.clone();
                                trace_spec.examples = vec![trace_example];
                                multi_candidates.push(MultiTraceSpecCandidate {
                                    signature: signature.clone(),
                                    trace_index: call_index,
                                    meta: meta.clone(),
                                    original_spec_name: spec.name.clone(),
                                    spec: trace_spec,
                                });

                                // For call>0, example[0] corresponds to the consensus/reference trace.
                                // Keep it when it is a meaningful output so pointer holes can cover all traces.
                                if call_index > 0 {
                                    if let Some(reference_example) = reference_example {
                                        if !is_missing_example_output(
                                            &reference_example,
                                            &desc.return_type,
                                        ) {
                                            let mut ref_spec = spec.clone();
                                            ref_spec.examples = vec![reference_example];
                                            multi_candidates.push(MultiTraceSpecCandidate {
                                                signature,
                                                trace_index: 0,
                                                meta,
                                                original_spec_name: spec.name.clone(),
                                                spec: ref_spec,
                                            });
                                        }
                                    }
                                }
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!(
                            "Failed to generate Escher specs for consensus vs call {}: {}",
                            call_index, e
                        );
                    }
                }
            }

            let (grouped_specs, grouped_meta, spec_renames) = aggregate_multi_trace_specs(
                &multi_candidates,
                req.method_calls.len(),
                &spec_base_name,
            );
            if grouped_specs.is_empty() {
                println!("No multi-trace grouped Escher specs were produced.");
            } else {
                let (dedup_specs, dedup_meta, dedup_renames) =
                    dedupe_equivalent_specs(grouped_specs, grouped_meta);
                let mut combined_spec_renames = spec_renames;
                for grouped_name in combined_spec_renames.values_mut() {
                    if let Some(canonical_name) = dedup_renames.get(grouped_name) {
                        *grouped_name = canonical_name.clone();
                    }
                }
                println!(
                    "Grouped multi-trace Escher specs: {} candidates -> {} grouped specs (dedup: {})",
                    multi_candidates.len(),
                    dedup_specs.len() + dedup_renames.len(),
                    dedup_specs.len()
                );
                aggregated_specs.extend(dedup_specs);
                spec_meta_by_name = dedup_meta;
                if let Some(best_plan) =
                    choose_best_common_plan_artifact(&multi_common_plans, &combined_spec_renames)
                {
                    common_plan_artifact = Some(remap_common_plan_artifact(
                        best_plan,
                        &combined_spec_renames,
                    ));
                }
            }
        } else {
            println!("Incremental unification unavailable; skipping Escher spec generation.");
        }
    }

    if let Some(artifact) = common_plan_artifact.as_mut() {
        prune_unused_holes_and_specs(artifact, &mut aggregated_specs, &mut spec_meta_by_name);
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

                let spec_by_name: HashMap<String, EscherSpec> = aggregated_specs
                    .iter()
                    .map(|spec| (spec.name.clone(), spec.clone()))
                    .collect();

                for out in &results {
                    if let Some(rendered) = &out.rendered {
                        match (
                            spec_by_name.get(&out.name),
                            spec_meta_by_name.get(&out.name),
                        ) {
                            (Some(spec), Some(meta)) => {
                                match build_context_from_spec(&out.name, spec, meta) {
                                    Ok((ctx, params_js)) => {
                                        match translate_rendered_method(rendered, &params_js, &ctx)
                                        {
                                            Ok(js) => {
                                                synthesized_codes.push(js.clone());
                                                individual_codes
                                                    .push(format!("{}: {}", out.name, js));
                                            }
                                            Err(e) => {
                                                individual_codes
                                                    .push(format!("{}: ERROR {}", out.name, e));
                                            }
                                        }
                                    }
                                    Err(e) => {
                                        individual_codes.push(format!("{}: ERROR {}", out.name, e));
                                    }
                                }
                            }
                            _ => {
                                individual_codes
                                    .push(format!("{}: ERROR missing spec metadata", out.name));
                            }
                        }
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
        common_pattern: common_plan_artifact
            .as_ref()
            .map(|artifact| artifact.pattern_text.clone()),
        hole_information: common_plan_artifact
            .as_ref()
            .map(|artifact| artifact.hole_information.clone()),
        code: synthesized_codes,
        composed_method_code: common_plan_artifact
            .as_ref()
            .and_then(|artifact| artifact.composed_method_code.clone()),
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
    if let Some(composed) = &response.composed_method_code {
        println!("Composed method code:\n{}", composed);
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

fn analyze_operations_pair_unification(
    operations_a: &[serde_json::Value],
    operations_b: &[serde_json::Value],
) -> anyhow::Result<UnificationAnalysisResult> {
    use crate::unify_ops::unify_operation_graphs;

    let ops_a = convert_json_to_unify_ops(operations_a)?;
    let ops_b = convert_json_to_unify_ops(operations_b)?;
    let unification_result = unify_operation_graphs(&ops_a, &ops_b);

    let common_count = unification_result.common_a.len();
    let diff_a_count = unification_result.diff_a.len();
    let diff_b_count = unification_result.diff_b.len();

    Ok(UnificationAnalysisResult {
        common_operations_count: common_count,
        total_operations_counts: vec![ops_a.len(), ops_b.len()],
        differences_found: std::cmp::max(diff_a_count, diff_b_count),
        unification_result,
    })
}

// unify_isomorphic_graphsを使用した統合ベースの操作分析
pub fn analyze_operations_with_unification(
    vis_graph: &models::VisGraph,
    operations_a: &[serde_json::Value],
    operations_b: &[serde_json::Value],
) -> anyhow::Result<UnificationAnalysisResult> {
    println!("=== UNIFICATION-BASED OPERATION ANALYSIS ===");

    let analysis = analyze_operations_pair_unification(operations_a, operations_b)?;
    let ops_a = convert_json_to_unify_ops(operations_a)?;
    let ops_b = convert_json_to_unify_ops(operations_b)?;

    println!("Converted {} operations from sequence A", ops_a.len());
    println!("Converted {} operations from sequence B", ops_b.len());
    trace_json("Unify ops A", &ops_a);
    trace_json("Unify ops B", &ops_b);

    println!("Unification results:");
    println!(
        "  - Common operations: {}",
        analysis.common_operations_count
    );
    println!(
        "  - Differences in A: {}",
        analysis.unification_result.diff_a.len()
    );
    println!(
        "  - Differences in B: {}",
        analysis.unification_result.diff_b.len()
    );
    if trace_enabled() {
        let mut mapping_pairs: Vec<(String, String)> = analysis
            .unification_result
            .final_mapping
            .iter()
            .map(|(a, b)| (a.clone(), b.clone()))
            .collect();
        mapping_pairs.sort();
        trace_json("Unification final_mapping", &mapping_pairs);
        trace_json(
            "Unification common_a",
            &analysis.unification_result.common_a,
        );
        trace_json(
            "Unification common_b",
            &analysis.unification_result.common_b,
        );
        trace_json("Unification diff_a", &analysis.unification_result.diff_a);
        trace_json("Unification diff_b", &analysis.unification_result.diff_b);
    }

    if analysis.common_operations_count > 0 {
        let list_env = create_environment_at_unification_boundary(
            vis_graph,
            &analysis.unification_result.common_a,
        )?;
        println!(
            "Created List environment at unification boundary with {} common operations",
            analysis.common_operations_count
        );
        println!("Environment state: {}", list_env.to_debug_string());
    }

    Ok(analysis)
}

// JSON操作をunify_ops::Op形式に変換
fn convert_json_to_unify_ops(
    operations: &[serde_json::Value],
) -> anyhow::Result<Vec<crate::unify_ops::Op>> {
    use crate::unify_ops::{EdgeExpr, GraphOp, NodeExpr, Op, VarOp};

    #[derive(Clone, serde::Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct OperationRecord {
        edit_type: String,
        id: Option<String>,
        label: Option<String>,
        is_literal: Option<bool>,
        from: Option<String>,
        to: Option<String>,
        old_to: Option<String>,
        new_to: Option<String>,
    }

    fn ensure_reference_node(
        node_id: &str,
        id_to_opnum: &mut std::collections::HashMap<String, String>,
        result: &mut Vec<Op>,
        exist_idx: &mut usize,
    ) -> String {
        if let Some(existing) = id_to_opnum.get(node_id) {
            return existing.clone();
        }

        if node_id == "null" {
            let op_id = "op_null".to_string();
            result.push(Op {
                id: op_id.clone(),
                kind: GraphOp::Node(NodeExpr::NullNode),
            });
            id_to_opnum.insert(node_id.to_string(), op_id.clone());
            return op_id;
        }

        let op_id = format!("op_exist_{}", *exist_idx);
        *exist_idx += 1;
        result.push(Op {
            id: op_id.clone(),
            kind: GraphOp::Node(NodeExpr::ExistNode {
                id: node_id.to_string(),
                label: String::new(),
                is_literal: false,
            }),
        });
        id_to_opnum.insert(node_id.to_string(), op_id.clone());
        op_id
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

    // 3) 参照されるIDのうち、addNodeで定義されていないものに対して ExistNode/NullNode を作る
    let mut referenced_ids: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    for (_i, op) in &decoded {
        match op.edit_type.as_str() {
            "addEdge" | "deleteEdge" | "editEdgeReference" => {
                if let Some(f) = &op.from {
                    referenced_ids.insert(f.clone());
                }
                if let Some(t) = &op.to {
                    referenced_ids.insert(t.clone());
                }
                if let Some(old_t) = &op.old_to {
                    referenced_ids.insert(old_t.clone());
                }
                if let Some(new_t) = &op.new_to {
                    referenced_ids.insert(new_t.clone());
                }
            }
            "addVariable" | "editVariableReference" => {
                if let Some(t) = &op.to {
                    referenced_ids.insert(t.clone());
                }
                if let Some(old_t) = &op.old_to {
                    referenced_ids.insert(old_t.clone());
                }
                if let Some(new_t) = &op.new_to {
                    referenced_ids.insert(new_t.clone());
                }
            }
            _ => {}
        }
    }
    let mut exist_idx: usize = 0;
    for obj_id in referenced_ids {
        ensure_reference_node(&obj_id, &mut id_to_opnum, &mut result, &mut exist_idx);
    }

    // 4) エッジ系/変数系を opId に解決して追加（set-referenceへ正規化）
    for (i, op) in &decoded {
        match op.edit_type.as_str() {
            "addEdge" | "editEdgeReference" => {
                let target = op.new_to.clone().or(op.to.clone());
                if let (Some(from), Some(to), Some(label)) =
                    (op.from.clone(), target, op.label.clone())
                {
                    let from_op =
                        ensure_reference_node(&from, &mut id_to_opnum, &mut result, &mut exist_idx);
                    let to_op =
                        ensure_reference_node(&to, &mut id_to_opnum, &mut result, &mut exist_idx);
                    result.push(Op {
                        id: format!("op_{}", i),
                        kind: GraphOp::Edge(EdgeExpr::AddEdge {
                            from: from_op,
                            to: to_op,
                            label,
                        }),
                    });
                }
            }
            "deleteEdge" => {
                if let (Some(from), Some(to)) = (op.from.clone(), op.to.clone()) {
                    let label = op.label.clone().unwrap_or_default();
                    let from_op =
                        ensure_reference_node(&from, &mut id_to_opnum, &mut result, &mut exist_idx);
                    let to_op =
                        ensure_reference_node(&to, &mut id_to_opnum, &mut result, &mut exist_idx);
                    result.push(Op {
                        id: format!("op_{}", i),
                        kind: GraphOp::Edge(EdgeExpr::DeleteEdge {
                            from: from_op,
                            to: to_op,
                            label,
                        }),
                    });
                }
            }
            "addVariable" | "editVariableReference" => {
                let target = op.new_to.clone().or(op.to.clone());
                if let (Some(to), Some(label)) = (target, op.label.clone()) {
                    let to_op =
                        ensure_reference_node(&to, &mut id_to_opnum, &mut result, &mut exist_idx);
                    result.push(Op {
                        id: format!("op_{}", i),
                        kind: GraphOp::Variable(VarOp::AddVariable { to: to_op, label }),
                    });
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
    let mut op_to_object_id: HashMap<String, String> = HashMap::new();

    // 共通操作を順次適用してホール境界までの状態を構築
    for op in common_operations {
        apply_operation_to_list_env(&mut list_env, op, &mut op_to_object_id)?;
    }

    Ok(list_env)
}

// List環境に操作を適用
fn apply_operation_to_list_env(
    list_env: &mut crate::list_env::ListEnvironment,
    op: &crate::unify_ops::Op,
    op_to_object_id: &mut HashMap<String, String>,
) -> anyhow::Result<()> {
    use crate::list_env::GraphOperation;
    use crate::unify_ops::{EdgeExpr, GraphOp, NodeExpr, VarOp};

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
                let new_index = list_env.next_index;
                list_env.obj_id_to_index.insert(id.clone(), new_index);
                list_env.index_to_obj_id.insert(new_index, id.clone());
                list_env.next_index += 1;
                for (field_name, field_vec) in list_env.field_lists.iter_mut() {
                    let default_value = list_env
                        .field_kinds
                        .get(field_name)
                        .copied()
                        .unwrap_or(crate::list_env::FieldKind::Value)
                        .default_value();
                    field_vec.push(default_value);
                }
            }
            op_to_object_id.insert(op.id.clone(), id.clone());
        }
        GraphOp::Edge(EdgeExpr::AddEdge { from, to, label }) => {
            let from_id = op_to_object_id
                .get(from)
                .cloned()
                .unwrap_or_else(|| from.clone());
            let to_id = op_to_object_id
                .get(to)
                .cloned()
                .unwrap_or_else(|| to.clone());
            let graph_op = GraphOperation {
                edit_type: "addEdge".to_string(),
                from: Some(from_id),
                to: Some(to_id),
                label: Some(serde_json::json!(label)),
                ..GraphOperation::default()
            };
            list_env
                .apply_operation(&graph_op)
                .map_err(|e| anyhow::anyhow!(e))?;
        }
        GraphOp::Node(NodeExpr::ExistNode { .. }) => {
            if let GraphOp::Node(NodeExpr::ExistNode { id, .. }) = &op.kind {
                op_to_object_id.insert(op.id.clone(), id.clone());
            }
        }
        GraphOp::Node(NodeExpr::NullNode) => {
            op_to_object_id.insert(op.id.clone(), "null".to_string());
        }
        GraphOp::Edge(EdgeExpr::DeleteEdge { .. }) => {
            // エッジ削除は複雑なため、現在は未実装
        }
        GraphOp::Edge(EdgeExpr::EditEdgeReference {
            from,
            new_to,
            label,
        }) => {
            let from_id = op_to_object_id
                .get(from)
                .cloned()
                .unwrap_or_else(|| from.clone());
            let new_to_id = op_to_object_id
                .get(new_to)
                .cloned()
                .unwrap_or_else(|| new_to.clone());
            let graph_op = GraphOperation {
                edit_type: "editEdgeReference".to_string(),
                from: Some(from_id),
                new_to: Some(new_to_id),
                label: Some(serde_json::json!(label)),
                ..GraphOperation::default()
            };
            list_env
                .apply_operation(&graph_op)
                .map_err(|e| anyhow::anyhow!(e))?;
        }
        GraphOp::Variable(VarOp::AddVariable { to, label }) => {
            let to_id = op_to_object_id
                .get(to)
                .cloned()
                .unwrap_or_else(|| to.clone());
            let graph_op = GraphOperation {
                edit_type: "addVariable".to_string(),
                to: Some(to_id),
                label: Some(serde_json::json!(label)),
                ..GraphOperation::default()
            };
            list_env
                .apply_operation(&graph_op)
                .map_err(|e| anyhow::anyhow!(e))?;
        }
        GraphOp::Variable(VarOp::EditVariableReference {
            old_to,
            new_to,
            label,
        }) => {
            let old_to_id = op_to_object_id
                .get(old_to)
                .cloned()
                .unwrap_or_else(|| old_to.clone());
            let new_to_id = op_to_object_id
                .get(new_to)
                .cloned()
                .unwrap_or_else(|| new_to.clone());
            let graph_op = GraphOperation {
                edit_type: "editVariableReference".to_string(),
                old_to: Some(old_to_id),
                new_to: Some(new_to_id),
                label: Some(serde_json::json!(label)),
                ..GraphOperation::default()
            };
            list_env
                .apply_operation(&graph_op)
                .map_err(|e| anyhow::anyhow!(e))?;
        }
        GraphOp::Variable(VarOp::DeleteVariable { .. }) => {
            // 変数削除は現段階で未対応
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

#[derive(Debug, Clone)]
struct IncrementalUnificationResult {
    consensus_operations: Vec<serde_json::Value>,
    common_operations_count: usize,
    total_operations_counts: Vec<usize>,
    differences_found: usize,
}

#[derive(Debug, Clone, Default)]
struct HoleDescriptor {
    spec_name: String,
    return_type: String,
    side_b: String,
    anchor_b: Option<usize>,
}

#[derive(Debug, Clone)]
struct MultiTraceSpecCandidate {
    signature: String,
    trace_index: usize,
    spec: EscherSpec,
    meta: EscherSpecMeta,
    original_spec_name: String,
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
    use crate::list_env::PtrValue;
    let var_prefix = "__Variable-";
    let mut var_ids: Vec<&str> = vis_graph
        .nodes
        .iter()
        .filter(|n| !n.is_literal)
        .map(|n| n.id.as_str())
        .filter(|id| id.starts_with(var_prefix))
        .collect();
    var_ids.sort();
    let mut ordered_var_ids: Vec<&str> = Vec::new();
    if var_ids.iter().any(|id| *id == "__Variable-this") {
        ordered_var_ids.push("__Variable-this");
    }
    for var_id in var_ids {
        if var_id != "__Variable-this" {
            ordered_var_ids.push(var_id);
        }
    }
    for var_id in ordered_var_ids {
        if let Some(&var_idx) = env.obj_id_to_index.get(var_id) {
            let var_name = var_id.trim_start_matches(var_prefix).to_string();
            if let Some(vec) = env.field_lists.get(&var_name) {
                if var_idx < vec.len() {
                    if let Some(PtrValue::Index(root_idx)) = PtrValue::from_value(&vec[var_idx]) {
                        return Some(root_idx);
                    }
                }
            }
        }
    }
    None
}

fn detect_runtime_receiver_object_id(
    env: &crate::list_env::ListEnvironment,
    vis_graph: &models::VisGraph,
) -> Option<String> {
    let object_ids: HashSet<&str> = vis_graph
        .nodes
        .iter()
        .filter(|n| !n.is_literal)
        .map(|n| n.id.as_str())
        .filter(|id| *id != "__RectForVariable__")
        .filter(|id| !id.starts_with("__Variable-"))
        .collect();

    let pick_variable_target = |name: &str| -> Option<String> {
        let var_id = format!("__Variable-{}", name);
        vis_graph
            .edges
            .iter()
            .find(|e| e.from == var_id && e.label == name && object_ids.contains(e.to.as_str()))
            .map(|e| e.to.clone())
    };

    // Prefer explicit `this` binding if present.
    if let Some(target) = pick_variable_target("this") {
        return Some(target);
    }
    // Fallback to `lst` for list-style payloads.
    if let Some(target) = pick_variable_target("lst") {
        return Some(target);
    }
    // Generic fallback: any __Variable-<name> --(<name>)--> object
    for edge in &vis_graph.edges {
        let Some(var_name) = edge.from.strip_prefix("__Variable-") else {
            continue;
        };
        if edge.label == var_name && object_ids.contains(edge.to.as_str()) {
            return Some(edge.to.clone());
        }
    }
    // Last fallback: root used by BFS encoding.
    if let Some(root_idx) = detect_root_index(env, vis_graph) {
        if let Some(id) = env.index_to_obj_id.get(&root_idx) {
            return Some(id.clone());
        }
    }
    None
}

fn resolve_effective_receiver_object(
    declared_receiver: Option<&str>,
    env: &crate::list_env::ListEnvironment,
    vis_graph: &models::VisGraph,
) -> Option<String> {
    if let Some(id) = declared_receiver {
        if env.obj_id_to_index.contains_key(id) {
            return Some(id.to_string());
        }
    }
    if let Some(runtime_receiver) = detect_runtime_receiver_object_id(env, vis_graph) {
        return Some(runtime_receiver);
    }
    declared_receiver.map(|id| id.to_string())
}

fn js_field_access_expr(base: &str, field: &str) -> String {
    if is_valid_js_identifier(field) {
        format!("{}.{}", base, field)
    } else {
        let quoted = serde_json::to_string(field).unwrap_or_else(|_| "\"\"".to_string());
        format!("{}[{}]", base, quoted)
    }
}

fn build_runtime_object_expression_map(
    vis_graph: &models::VisGraph,
    receiver_object_id: Option<&str>,
) -> HashMap<String, String> {
    use std::collections::VecDeque;

    let Some(receiver_id) = receiver_object_id else {
        return HashMap::new();
    };

    let object_ids: HashSet<&str> = vis_graph
        .nodes
        .iter()
        .filter(|n| !n.is_literal)
        .map(|n| n.id.as_str())
        .filter(|id| *id != "__RectForVariable__")
        .filter(|id| !id.starts_with("__Variable-"))
        .collect();
    let literal_ids: HashSet<&str> = vis_graph
        .nodes
        .iter()
        .filter(|n| n.is_literal)
        .map(|n| n.id.as_str())
        .collect();

    if !object_ids.contains(receiver_id) {
        return HashMap::new();
    }

    let mut adjacency: HashMap<String, Vec<(String, String)>> = HashMap::new();
    let mut field_labels: HashSet<String> = HashSet::new();
    for edge in &vis_graph.edges {
        field_labels.insert(edge.label.clone());
        if object_ids.contains(edge.from.as_str()) && object_ids.contains(edge.to.as_str()) {
            adjacency
                .entry(edge.from.clone())
                .or_default()
                .push((edge.label.clone(), edge.to.clone()));
        }
    }
    for edges in adjacency.values_mut() {
        edges.sort();
    }

    let mut expr_by_id: HashMap<String, String> = HashMap::new();
    let mut queue: VecDeque<String> = VecDeque::new();
    expr_by_id.insert(receiver_id.to_string(), "this".to_string());
    queue.push_back(receiver_id.to_string());

    while let Some(from_id) = queue.pop_front() {
        let Some(from_expr) = expr_by_id.get(&from_id).cloned() else {
            continue;
        };
        if let Some(edges) = adjacency.get(&from_id) {
            for (label, to_id) in edges {
                if expr_by_id.contains_key(to_id) {
                    continue;
                }
                let to_expr = js_field_access_expr(&from_expr, label);
                expr_by_id.insert(to_id.clone(), to_expr);
                queue.push_back(to_id.clone());
            }
        }
    }

    // Map literal node ids reachable as object.<field> to support Kanon-style IDs like "...-val".
    for edge in &vis_graph.edges {
        if !literal_ids.contains(edge.to.as_str()) {
            continue;
        }
        if let Some(from_expr) = expr_by_id.get(&edge.from).cloned() {
            let to_expr = js_field_access_expr(&from_expr, &edge.label);
            expr_by_id.entry(edge.to.clone()).or_insert(to_expr);
        }
    }

    // Naming-based fallback for Kanon field nodes: <object-id>-<field>.
    // Example: "main-call2-FunctionExpression2-new1-val" -> "this.next.val"
    let mut extra_bindings: Vec<(String, String)> = Vec::new();
    for node in &vis_graph.nodes {
        if let Some((parent, field)) = node.id.rsplit_once('-') {
            if !field_labels.contains(field) || !object_ids.contains(parent) {
                continue;
            }
            if let Some(parent_expr) = expr_by_id.get(parent) {
                extra_bindings.push((node.id.clone(), js_field_access_expr(parent_expr, field)));
            }
        }
    }
    for (id, expr) in extra_bindings {
        expr_by_id.entry(id).or_insert(expr);
    }

    expr_by_id
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ArgKind {
    Ptr,
    Int,
}

struct ArgBundle {
    names: Vec<String>,
    values: Vec<serde_json::Value>,
    types: Vec<String>,
    receiver_arg_index: Option<usize>,
}

fn build_case_arguments_from_variables(
    env: &crate::list_env::ListEnvironment,
    vis_graph: &models::VisGraph,
    pointer_fields: &[String],
    receiver_object: Option<&str>,
) -> anyhow::Result<ArgBundle> {
    use crate::list_env::{FieldKind, PtrValue};
    use serde_json::Value;
    use std::collections::{HashMap, HashSet};

    let var_prefix = "__Variable-";
    let mut var_names: Vec<String> = Vec::new();
    let mut var_id_by_name: HashMap<String, String> = HashMap::new();
    for node in &vis_graph.nodes {
        if node.is_literal {
            continue;
        }
        if node.id.starts_with(var_prefix) {
            let name = node.id.trim_start_matches(var_prefix).to_string();
            var_id_by_name.insert(name.clone(), node.id.clone());
            var_names.push(name);
        }
    }
    var_names.sort();
    var_names.dedup();
    if let Some(pos) = var_names.iter().position(|n| n == "this") {
        let this_name = var_names.remove(pos);
        var_names.insert(0, this_name);
    }

    if var_names.is_empty() {
        return Ok(ArgBundle {
            names: Vec::new(),
            values: Vec::new(),
            types: Vec::new(),
            receiver_arg_index: None,
        });
    }

    let mut literal_ids: HashSet<&str> = HashSet::new();
    let mut object_ids: HashSet<&str> = HashSet::new();
    for node in &vis_graph.nodes {
        if node.id == "__RectForVariable__" {
            continue;
        }
        if node.is_literal {
            literal_ids.insert(node.id.as_str());
        } else if !node.id.starts_with(var_prefix) {
            object_ids.insert(node.id.as_str());
        }
    }

    let mut kind_by_name: HashMap<String, ArgKind> = HashMap::new();
    for edge in &vis_graph.edges {
        if edge.from == "__RectForVariable__" || edge.to == "__RectForVariable__" {
            continue;
        }
        if let Some(var_name) = edge.from.strip_prefix(var_prefix) {
            if edge.label != var_name {
                continue;
            }
            if literal_ids.contains(edge.to.as_str()) {
                kind_by_name
                    .entry(var_name.to_string())
                    .or_insert(ArgKind::Int);
            } else if object_ids.contains(edge.to.as_str()) {
                kind_by_name.insert(var_name.to_string(), ArgKind::Ptr);
            }
        }
    }

    for name in &var_names {
        if !kind_by_name.contains_key(name) {
            if let Some(kind) = env.field_kinds.get(name) {
                let arg_kind = match kind {
                    FieldKind::Pointer => ArgKind::Ptr,
                    FieldKind::Value => ArgKind::Int,
                };
                kind_by_name.insert(name.clone(), arg_kind);
            }
        }
    }
    for name in &var_names {
        kind_by_name.entry(name.clone()).or_insert(ArgKind::Ptr);
    }

    let needs_bfs = !pointer_fields.is_empty()
        || var_names
            .iter()
            .any(|name| kind_by_name.get(name) == Some(&ArgKind::Ptr));
    let idx_to_bfs = if needs_bfs {
        build_bfs_index_map(env, vis_graph, pointer_fields)?
    } else {
        HashMap::new()
    };

    let mut values: Vec<Value> = Vec::with_capacity(var_names.len());
    let mut types: Vec<String> = Vec::with_capacity(var_names.len());
    let mut receiver_matches: Vec<usize> = Vec::new();

    for (idx, name) in var_names.iter().enumerate() {
        let var_id = var_id_by_name.get(name);
        let var_idx = var_id.and_then(|id| env.obj_id_to_index.get(id)).copied();
        let arg_kind = kind_by_name.get(name).copied().unwrap_or(ArgKind::Ptr);
        match arg_kind {
            ArgKind::Ptr => {
                types.push("Ptr".to_string());
                let mut value = Value::Null;
                if let Some(var_idx) = var_idx {
                    if let Some(list) = env.field_lists.get(name) {
                        if var_idx < list.len() {
                            if let Some(PtrValue::Index(to_idx)) =
                                PtrValue::from_value(&list[var_idx])
                            {
                                if let Some(mapped) = idx_to_bfs.get(&to_idx) {
                                    value = serde_json::json!(*mapped as i32);
                                }
                                if let Some(receiver_object) = receiver_object {
                                    if let Some(obj_id) = env.index_to_obj_id.get(&to_idx) {
                                        if obj_id == receiver_object {
                                            receiver_matches.push(idx);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                values.push(value);
            }
            ArgKind::Int => {
                types.push("Int".to_string());
                let mut value = serde_json::json!(-1);
                if let Some(var_idx) = var_idx {
                    if let Some(list) = env.field_lists.get(name) {
                        if var_idx < list.len() {
                            value = value_to_i32(&list[var_idx])
                                .map(|v| serde_json::json!(v))
                                .unwrap_or_else(|| serde_json::json!(-1));
                        }
                    }
                }
                values.push(value);
            }
        }
    }

    let mut receiver_arg_index = None;
    if let Some(pos) = var_names.iter().position(|n| n == "this") {
        if types.get(pos).map(|t| t.as_str()) != Some("Ptr") {
            return Err(anyhow::anyhow!(
                "variable 'this' must be Ptr to serve as receiver"
            ));
        }
        receiver_arg_index = Some(pos);
    } else if receiver_object.is_some() {
        if receiver_matches.len() > 1 {
            return Err(anyhow::anyhow!(
                "receiver object is referenced by multiple variables: {:?}",
                receiver_matches
            ));
        }
        if receiver_matches.len() == 1 {
            receiver_arg_index = Some(receiver_matches[0]);
        }
    }

    if receiver_arg_index.is_none() {
        let ptr_indices: Vec<usize> = types
            .iter()
            .enumerate()
            .filter_map(|(idx, ty)| if ty == "Ptr" { Some(idx) } else { None })
            .collect();
        if ptr_indices.len() == 1 {
            receiver_arg_index = Some(ptr_indices[0]);
        }
    }

    Ok(ArgBundle {
        names: var_names,
        values,
        types,
        receiver_arg_index,
    })
}

fn append_method_call_arguments(
    bundle: &mut ArgBundle,
    call_arguments: &[serde_json::Value],
    call_argument_types: Option<&[String]>,
    call_argument_names: Option<&[String]>,
    env: &crate::list_env::ListEnvironment,
    vis_graph: &models::VisGraph,
    pointer_fields: &[String],
) -> anyhow::Result<()> {
    use std::collections::HashMap;

    if call_arguments.is_empty() {
        return Ok(());
    }

    if let Some(types) = call_argument_types {
        if types.len() != call_arguments.len() {
            return Err(anyhow::anyhow!(
                "argument type count {} does not match argument count {}",
                types.len(),
                call_arguments.len()
            ));
        }
    }
    if let Some(names) = call_argument_names {
        if names.len() != call_arguments.len() {
            return Err(anyhow::anyhow!(
                "argument name count {} does not match argument count {}",
                names.len(),
                call_arguments.len()
            ));
        }
    }

    let mut arg_kinds: Vec<ArgKind> = Vec::with_capacity(call_arguments.len());
    for (idx, value) in call_arguments.iter().enumerate() {
        let declared = call_argument_types
            .and_then(|types| types.get(idx))
            .map(String::as_str);
        arg_kinds.push(resolve_call_argument_kind(value, declared, env)?);
    }

    let needs_bfs = arg_kinds.iter().any(|k| *k == ArgKind::Ptr);
    let idx_to_bfs: HashMap<usize, usize> = if needs_bfs {
        build_bfs_index_map(env, vis_graph, pointer_fields)?
    } else {
        HashMap::new()
    };

    for (idx, value) in call_arguments.iter().enumerate() {
        let base_name = call_argument_names
            .and_then(|names| names.get(idx))
            .filter(|name| !name.trim().is_empty())
            .cloned()
            .unwrap_or_else(|| format!("arg{}", idx));
        let mut name = base_name.clone();
        let mut suffix = 1usize;
        while bundle.names.contains(&name) {
            name = format!("{}_{}", base_name, suffix);
            suffix += 1;
        }
        bundle.names.push(name);

        match arg_kinds[idx] {
            ArgKind::Int => {
                let int_value = value_to_i32(value).ok_or_else(|| {
                    anyhow::anyhow!("failed to encode argument {} as Int: {}", idx, value)
                })?;
                bundle.types.push("Int".to_string());
                bundle.values.push(serde_json::json!(int_value));
            }
            ArgKind::Ptr => {
                let ptr_value = encode_call_argument_ptr(value, env, &idx_to_bfs, idx)?;
                bundle.types.push("Ptr".to_string());
                bundle.values.push(ptr_value);
            }
        }
    }

    Ok(())
}

fn resolve_call_argument_kind(
    value: &serde_json::Value,
    declared: Option<&str>,
    env: &crate::list_env::ListEnvironment,
) -> anyhow::Result<ArgKind> {
    if let Some(ty) = declared {
        return match ty {
            "Int" => Ok(ArgKind::Int),
            "Ptr" => Ok(ArgKind::Ptr),
            other => Err(anyhow::anyhow!("unsupported argument type '{}'", other)),
        };
    }

    if value_to_i32(value).is_some() {
        return Ok(ArgKind::Int);
    }

    if value.is_null() {
        return Ok(ArgKind::Ptr);
    }

    if let Some(id) = value.as_str() {
        if env.obj_id_to_index.contains_key(id) {
            return Ok(ArgKind::Ptr);
        }
    }
    if let Some(id) = value.get("id").and_then(|v| v.as_str()) {
        if env.obj_id_to_index.contains_key(id) {
            return Ok(ArgKind::Ptr);
        }
    }

    Err(anyhow::anyhow!(
        "cannot infer argument type from value {}; supply argumentTypes",
        value
    ))
}

fn encode_call_argument_ptr(
    value: &serde_json::Value,
    env: &crate::list_env::ListEnvironment,
    idx_to_bfs: &HashMap<usize, usize>,
    arg_index: usize,
) -> anyhow::Result<serde_json::Value> {
    if value.is_null() {
        return Ok(serde_json::Value::Null);
    }

    if let Some(num) = value.as_i64() {
        if let Ok(v) = i32::try_from(num) {
            if v >= 0 {
                return Ok(serde_json::json!(v));
            }
        }
        return Err(anyhow::anyhow!(
            "invalid Ptr argument {} value {}",
            arg_index,
            value
        ));
    }

    if let Some(id) = value.as_str() {
        return encode_call_argument_ptr_from_id(id, env, idx_to_bfs, arg_index);
    }
    if let Some(id) = value.get("id").and_then(|v| v.as_str()) {
        return encode_call_argument_ptr_from_id(id, env, idx_to_bfs, arg_index);
    }

    Err(anyhow::anyhow!(
        "unsupported Ptr argument {} value {}",
        arg_index,
        value
    ))
}

fn encode_call_argument_ptr_from_id(
    raw: &str,
    env: &crate::list_env::ListEnvironment,
    idx_to_bfs: &HashMap<usize, usize>,
    arg_index: usize,
) -> anyhow::Result<serde_json::Value> {
    if let Ok(num) = raw.parse::<i64>() {
        if let Ok(v) = i32::try_from(num) {
            if v >= 0 {
                return Ok(serde_json::json!(v));
            }
        }
    }

    let orig_idx =
        env.obj_id_to_index.get(raw).copied().ok_or_else(|| {
            anyhow::anyhow!("unknown Ptr argument {} object id '{}'", arg_index, raw)
        })?;
    let mapped_idx = idx_to_bfs.get(&orig_idx).copied().ok_or_else(|| {
        anyhow::anyhow!(
            "Ptr argument {} object '{}' is not reachable from receiver root",
            arg_index,
            raw
        )
    })?;
    Ok(serde_json::json!(mapped_idx as i32))
}

fn build_bfs_index_map(
    env: &crate::list_env::ListEnvironment,
    vis_graph: &models::VisGraph,
    pointer_fields: &[String],
) -> anyhow::Result<HashMap<usize, usize>> {
    use crate::list_env::PtrValue;
    use std::collections::{HashMap, HashSet, VecDeque};

    let root =
        detect_root_index(env, vis_graph).ok_or_else(|| anyhow::anyhow!("root not found"))?;
    let mut adj: HashMap<usize, Vec<usize>> = HashMap::new();
    for pf in pointer_fields {
        if let Some(vec) = env.field_lists.get(pf) {
            for (from_idx, v) in vec.iter().enumerate() {
                if let Some(PtrValue::Index(to)) = PtrValue::from_value(v) {
                    adj.entry(from_idx).or_default().push(to);
                }
            }
        }
    }
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
    Ok(idx_to_bfs)
}

fn bfs_local_index_for_object(
    env: &crate::list_env::ListEnvironment,
    vis_graph: &models::VisGraph,
    object_id: &str,
) -> anyhow::Result<Option<i32>> {
    use crate::list_env::PtrValue;
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
                if let Some(PtrValue::Index(to)) = PtrValue::from_value(v) {
                    adj.entry(from_idx).or_default().push(to);
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
        Ok(idx_to_bfs.get(&orig_idx).copied().map(|x| x as i32))
    } else {
        Ok(None)
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
    let to = graph_op
        .new_to
        .as_deref()
        .or(graph_op.to.as_deref())
        .unwrap_or("-");
    let old_to = graph_op.old_to.as_deref().unwrap_or("-");
    let is_literal = graph_op.is_literal.unwrap_or(false);
    format!(
        "{} id={} from={} to={} old_to={} label={} is_literal={}",
        graph_op.edit_type, id, from, to, old_to, label, is_literal
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

#[derive(Clone)]
struct HoleBinding {
    hole_key: String,
    spec_name: String,
    return_type: String,
    js_method_name: String,
    js_call_template: String,
    side_a: String,
    side_b: String,
    anchor_a: Option<usize>,
    anchor_b: Option<usize>,
}

#[derive(Clone)]
struct GeneratedSpecsResult {
    specs: Vec<EscherSpec>,
    common_plan: Option<CommonPlanArtifact>,
}

fn diff_op_index(op: &DiffOp) -> Option<usize> {
    match op {
        DiffOp::Json { index, .. } => Some(*index),
        DiffOp::ExistNode { .. } => None,
    }
}

fn first_reference_index_for_object(
    object_id: &str,
    graph_ops: &[crate::list_env::GraphOperation],
) -> Option<usize> {
    graph_ops.iter().position(|op| {
        op.id.as_deref() == Some(object_id)
            || op.from.as_deref() == Some(object_id)
            || op.to.as_deref() == Some(object_id)
            || op.old_to.as_deref() == Some(object_id)
            || op.new_to.as_deref() == Some(object_id)
    })
}

fn diff_op_anchor_index(
    op: &DiffOp,
    graph_ops: &[crate::list_env::GraphOperation],
) -> Option<usize> {
    match op {
        DiffOp::Json { index, .. } => Some(*index),
        DiffOp::ExistNode { id, .. } => first_reference_index_for_object(id, graph_ops),
    }
}

fn anchor_index_for_side(
    own_index: Option<usize>,
    other_index: Option<usize>,
    own_ops_len: usize,
) -> Option<usize> {
    if let Some(idx) = own_index {
        return Some(idx.min(own_ops_len));
    }
    other_index.filter(|idx| *idx < own_ops_len)
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

fn project_common_ops_to_source_operations(
    source_operations: &[serde_json::Value],
    common_ops: &[crate::unify_ops::Op],
) -> Vec<serde_json::Value> {
    let mut indices: Vec<usize> = common_ops
        .iter()
        .filter_map(|op| parse_op_index(&op.id))
        .collect();
    indices.sort_unstable();
    indices.dedup();

    indices
        .into_iter()
        .filter_map(|idx| source_operations.get(idx).cloned())
        .collect()
}

fn analyze_operations_incrementally(
    operations_list: &[Vec<serde_json::Value>],
) -> anyhow::Result<IncrementalUnificationResult> {
    if operations_list.len() < 2 {
        return Err(anyhow::anyhow!(
            "incremental unification requires at least two operation sequences"
        ));
    }

    let mut consensus_operations = operations_list[0].clone();
    let mut common_operations_count = 0usize;

    for operations in operations_list.iter().skip(1) {
        let analysis = analyze_operations_pair_unification(&consensus_operations, operations)?;
        common_operations_count = analysis.common_operations_count;
        consensus_operations = project_common_ops_to_source_operations(
            &consensus_operations,
            &analysis.unification_result.common_a,
        );
    }

    let mut total_operations_counts = Vec::with_capacity(operations_list.len());
    let mut differences_found = 0usize;
    for operations in operations_list {
        let analysis = analyze_operations_pair_unification(&consensus_operations, operations)?;
        if let Some(count) = analysis.total_operations_counts.get(1) {
            total_operations_counts.push(*count);
        } else {
            total_operations_counts.push(operations.len());
        }
        differences_found = differences_found.max(analysis.differences_found);
    }

    Ok(IncrementalUnificationResult {
        consensus_operations,
        common_operations_count,
        total_operations_counts,
        differences_found,
    })
}

fn parse_hole_descriptors_by_spec(
    common_plan: &CommonPlanArtifact,
) -> HashMap<String, HoleDescriptor> {
    let mut result = HashMap::new();
    for values in common_plan.hole_information.values() {
        let mut desc = HoleDescriptor::default();
        for item in values {
            if let Some((key, value)) = item.split_once('=') {
                match key {
                    "spec" => desc.spec_name = value.to_string(),
                    "return" => desc.return_type = value.to_string(),
                    "sideB" => desc.side_b = value.to_string(),
                    "anchorB" => desc.anchor_b = value.parse::<usize>().ok(),
                    _ => {}
                }
            }
        }
        if !desc.spec_name.is_empty() {
            result.insert(desc.spec_name.clone(), desc);
        }
    }
    result
}

fn token_value(desc: &str, key: &str) -> Option<String> {
    let prefix = format!("{}=", key);
    for token in desc.split_whitespace() {
        if let Some(rest) = token.strip_prefix(&prefix) {
            return Some(rest.to_string());
        }
    }
    None
}

fn normalize_side_descriptor(desc: &str) -> String {
    let op = desc.split_whitespace().next().unwrap_or("<none>");
    let is_literal = token_value(desc, "is_literal").unwrap_or_else(|| "<none>".to_string());
    let mut label = token_value(desc, "label").unwrap_or_else(|| "<none>".to_string());
    // Literal payload values differ per trace (e.g. 25/93/48) and should not split hole groups.
    if is_literal == "true" {
        label = "<literal>".to_string();
    }
    format!("op={}|label={}|is_literal={}", op, label, is_literal)
}

fn hole_signature(desc: &HoleDescriptor) -> Option<String> {
    if desc.side_b.is_empty() || desc.side_b == "<none>" {
        return None;
    }
    let normalized_side = normalize_side_descriptor(&desc.side_b);
    Some(format!("ret={}|{}", desc.return_type, normalized_side))
}

fn example_to_input_output_key(example: &ExampleJson) -> Option<(String, String)> {
    let value = serde_json::to_value(example).ok()?;
    let input = value.get("input")?;
    let output = value.get("output")?;
    let input_key = serde_json::to_string(input).ok()?;
    let output_key = serde_json::to_string(output).ok()?;
    Some((input_key, output_key))
}

fn has_conflicting_examples(examples: &[ExampleJson]) -> bool {
    let mut seen: HashMap<String, String> = HashMap::new();
    for ex in examples {
        let (input_key, output_key) = match example_to_input_output_key(ex) {
            Some(v) => v,
            None => continue,
        };
        if let Some(existing) = seen.get(&input_key) {
            if existing != &output_key {
                return true;
            }
        } else {
            seen.insert(input_key, output_key);
        }
    }
    false
}

fn spec_equivalence_key(spec: &EscherSpec) -> Option<String> {
    let input_types = serde_json::to_string(&spec.input_types).ok()?;
    let examples = serde_json::to_string(&spec.examples).ok()?;
    Some(format!(
        "ret={}|in={}|ex={}",
        spec.return_type, input_types, examples
    ))
}

fn dedupe_equivalent_specs(
    specs: Vec<EscherSpec>,
    metas: HashMap<String, EscherSpecMeta>,
) -> (
    Vec<EscherSpec>,
    HashMap<String, EscherSpecMeta>,
    HashMap<String, String>,
) {
    let mut key_to_name: HashMap<String, String> = HashMap::new();
    let mut dedup_specs: Vec<EscherSpec> = Vec::new();
    let mut dedup_meta: HashMap<String, EscherSpecMeta> = HashMap::new();
    let mut renames: HashMap<String, String> = HashMap::new();

    for spec in specs {
        let key = spec_equivalence_key(&spec).unwrap_or_else(|| format!("name={}", spec.name));
        if let Some(existing_name) = key_to_name.get(&key) {
            renames.insert(spec.name.clone(), existing_name.clone());
            continue;
        }
        key_to_name.insert(key, spec.name.clone());
        if let Some(meta) = metas.get(&spec.name) {
            dedup_meta.insert(spec.name.clone(), meta.clone());
        }
        dedup_specs.push(spec);
    }

    (dedup_specs, dedup_meta, renames)
}

fn is_missing_example_output(example: &ExampleJson, return_type: &str) -> bool {
    if return_type == "Ptr" {
        example.output.is_null()
    } else {
        value_to_i32(&example.output) == Some(-1)
    }
}

fn aggregate_multi_trace_specs(
    candidates: &[MultiTraceSpecCandidate],
    trace_count: usize,
    base_name: &str,
) -> (
    Vec<EscherSpec>,
    HashMap<String, EscherSpecMeta>,
    HashMap<String, String>,
) {
    let mut grouped: BTreeMap<String, Vec<&MultiTraceSpecCandidate>> = BTreeMap::new();
    for candidate in candidates {
        grouped
            .entry(candidate.signature.clone())
            .or_default()
            .push(candidate);
    }

    let mut specs: Vec<EscherSpec> = Vec::new();
    let mut metas: HashMap<String, EscherSpecMeta> = HashMap::new();
    let mut renames: HashMap<String, String> = HashMap::new();
    let mut suffix_index = 0usize;

    for members in grouped.values() {
        let representative = match members.first() {
            Some(rep) => *rep,
            None => continue,
        };

        let mut by_trace: HashMap<usize, &MultiTraceSpecCandidate> = HashMap::new();
        for candidate in members {
            if candidate.spec.input_types != representative.spec.input_types
                || candidate.spec.return_type != representative.spec.return_type
            {
                continue;
            }
            by_trace.entry(candidate.trace_index).or_insert(*candidate);
        }

        if by_trace.len() != trace_count {
            continue;
        }

        let mut ordered: Vec<(usize, &MultiTraceSpecCandidate)> = by_trace.into_iter().collect();
        ordered.sort_by_key(|(idx, _)| *idx);

        let examples: Vec<ExampleJson> = ordered
            .iter()
            .filter_map(|(_, candidate)| candidate.spec.examples.first().cloned())
            .collect();
        if examples.len() != trace_count {
            continue;
        }
        if has_conflicting_examples(&examples) {
            continue;
        }

        let spec_name = build_spec_name(base_name, suffix_index);
        suffix_index += 1;

        specs.push(EscherSpec {
            name: spec_name.clone(),
            input_types: representative.spec.input_types.clone(),
            return_type: representative.spec.return_type.clone(),
            examples,
        });
        metas.insert(spec_name.clone(), representative.meta.clone());
        for (_, candidate) in &ordered {
            renames.insert(candidate.original_spec_name.clone(), spec_name.clone());
        }
    }

    (specs, metas, renames)
}

fn choose_best_common_plan_artifact(
    artifacts: &[CommonPlanArtifact],
    spec_renames: &HashMap<String, String>,
) -> Option<CommonPlanArtifact> {
    let mut best: Option<(usize, usize, CommonPlanArtifact)> = None;
    for artifact in artifacts {
        let descriptors = parse_hole_descriptors_by_spec(artifact);
        let covered = descriptors
            .keys()
            .filter(|spec_name| spec_renames.contains_key(*spec_name))
            .count();
        let total = descriptors.len();
        match &best {
            None => best = Some((covered, total, artifact.clone())),
            Some((best_covered, best_total, _)) => {
                if covered > *best_covered || (covered == *best_covered && total > *best_total) {
                    best = Some((covered, total, artifact.clone()));
                }
            }
        }
    }
    best.map(|(_, _, artifact)| artifact)
}

fn is_js_identifier_char(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || ch == '_' || ch == '$'
}

fn extract_called_js_methods_from_code(code: &str) -> HashSet<String> {
    let mut methods: HashSet<String> = HashSet::new();
    let mut cursor = 0usize;

    while cursor < code.len() {
        let Some(pos) = code[cursor..].find("this.") else {
            break;
        };
        let start = cursor + pos + "this.".len();
        let mut end = start;
        while end < code.len() {
            let ch = code[end..].chars().next().unwrap_or('\0');
            if !is_js_identifier_char(ch) {
                break;
            }
            end += ch.len_utf8();
        }
        if end > start {
            let method = &code[start..end];
            if code[end..].starts_with('(') {
                methods.insert(method.to_string());
            }
        }
        cursor = end.saturating_add(1);
    }

    methods
}

fn filter_common_plan_hole_bindings(pattern_text: &str, keep_specs: &HashSet<String>) -> String {
    let mut lines_out: Vec<String> = Vec::new();
    let mut in_hole_bindings = false;

    for line in pattern_text.lines() {
        let trimmed = line.trim();
        if trimmed == "HOLE_BINDINGS" {
            in_hole_bindings = true;
            lines_out.push(line.to_string());
            continue;
        }

        if in_hole_bindings {
            if trimmed.is_empty() {
                lines_out.push(line.to_string());
                continue;
            }
            if let Some((_, right)) = line.split_once("->") {
                let spec = right.trim().split_whitespace().next().unwrap_or_default();
                if keep_specs.contains(spec) {
                    lines_out.push(line.to_string());
                }
                continue;
            }
            in_hole_bindings = false;
        }

        lines_out.push(line.to_string());
    }

    lines_out.join("\n")
}

fn prune_unused_holes_and_specs(
    artifact: &mut CommonPlanArtifact,
    specs: &mut Vec<EscherSpec>,
    spec_meta_by_name: &mut HashMap<String, EscherSpecMeta>,
) {
    let Some(code) = artifact.composed_method_code.as_ref() else {
        return;
    };
    let used_js_methods = extract_called_js_methods_from_code(code);
    if used_js_methods.is_empty() {
        return;
    }

    let mut keep_specs: HashSet<String> = HashSet::new();
    artifact.hole_information.retain(|_, values| {
        let mut spec_name: Option<String> = None;
        let mut js_method: Option<String> = None;
        for value in values {
            if let Some(spec) = value.strip_prefix("spec=") {
                spec_name = Some(spec.to_string());
            } else if let Some(method) = value.strip_prefix("jsMethod=") {
                js_method = Some(method.to_string());
            }
        }
        let keep = js_method
            .as_ref()
            .map(|method| used_js_methods.contains(method))
            .unwrap_or(false);
        if keep {
            if let Some(spec) = spec_name {
                keep_specs.insert(spec);
            }
        }
        keep
    });

    if keep_specs.is_empty() {
        return;
    }

    specs.retain(|spec| keep_specs.contains(&spec.name));
    spec_meta_by_name.retain(|name, _| keep_specs.contains(name));
    artifact.pattern_text = filter_common_plan_hole_bindings(&artifact.pattern_text, &keep_specs);
}

fn remap_common_plan_artifact(
    artifact: CommonPlanArtifact,
    spec_renames: &HashMap<String, String>,
) -> CommonPlanArtifact {
    if spec_renames.is_empty() {
        return artifact;
    }

    let mut pattern_text = artifact.pattern_text;
    for (old, new) in spec_renames {
        pattern_text = pattern_text.replace(old, new);
    }

    let mut hole_information: HashMap<String, Vec<String>> = HashMap::new();
    for (hole, values) in artifact.hole_information {
        let mut spec_name: Option<String> = None;
        for item in &values {
            if let Some((key, value)) = item.split_once('=') {
                if key == "spec" {
                    spec_name = Some(value.to_string());
                    break;
                }
            }
        }
        let Some(old_spec_name) = spec_name else {
            continue;
        };
        let Some(new_spec_name) = spec_renames.get(&old_spec_name) else {
            continue;
        };
        let old_js = spec_name_to_js_method_name(&old_spec_name);
        let new_js = spec_name_to_js_method_name(new_spec_name);
        let mut remapped_values = Vec::new();
        for item in values {
            if item.starts_with("spec=") {
                remapped_values.push(format!("spec={}", new_spec_name));
            } else if item.starts_with("jsMethod=") {
                remapped_values.push(format!("jsMethod={}", new_js));
            } else if item.starts_with("jsCall=") {
                remapped_values
                    .push(item.replace(&format!("this.{}(", old_js), &format!("this.{}(", new_js)));
            } else {
                remapped_values.push(item);
            }
        }
        hole_information.insert(hole, remapped_values);
    }

    let mut composed_method_code = artifact.composed_method_code;
    if let Some(code) = composed_method_code.as_mut() {
        for (old, new) in spec_renames {
            let old_js = spec_name_to_js_method_name(old);
            let new_js = spec_name_to_js_method_name(new);
            *code = code.replace(&format!("this.{}(", old_js), &format!("this.{}(", new_js));
        }
    }

    CommonPlanArtifact {
        pattern_text,
        hole_information,
        composed_method_code,
    }
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
        "addEdge" | "editEdgeReference" | "removeEdge" => op.from.clone(),
        "addVariable" | "editVariableReference" => {
            graph_op_label_string(op).map(|label| format!("__Variable-{}", label))
        }
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

fn graph_op_target_id(op: &crate::list_env::GraphOperation) -> Option<String> {
    match op.edit_type.as_str() {
        "editEdgeReference" | "editVariableReference" => op.new_to.clone().or(op.to.clone()),
        "addEdge" | "removeEdge" | "addVariable" => op.to.clone(),
        _ => op.to.clone(),
    }
}

fn canonicalize_object_id(id: &str, mapping: &HashMap<String, String>) -> String {
    mapping.get(id).cloned().unwrap_or_else(|| id.to_string())
}

fn sanitize_js_identifier(name: &str) -> String {
    let mut out = String::with_capacity(name.len());
    for ch in name.chars() {
        let valid = ch.is_ascii_alphanumeric() || ch == '_' || ch == '$';
        out.push(if valid { ch } else { '_' });
    }
    if out.is_empty() {
        "_".to_string()
    } else {
        if out
            .chars()
            .next()
            .map(|c| c.is_ascii_digit())
            .unwrap_or(false)
        {
            format!("_{}", out)
        } else {
            out
        }
    }
}

fn spec_name_to_js_method_name(spec_name: &str) -> String {
    sanitize_js_identifier(spec_name)
}

fn default_method_param_names(arg_count: usize) -> Vec<String> {
    match arg_count {
        0 => vec![],
        1 => vec!["arg".to_string()],
        n => (0..n).map(|i| format!("arg{}", i)).collect(),
    }
}

fn build_hole_call_template(js_method_name: &str, method_param_names: &[String]) -> String {
    if method_param_names.is_empty() {
        format!("this.{}()", js_method_name)
    } else {
        format!("this.{}({})", js_method_name, method_param_names.join(", "))
    }
}

fn is_valid_js_identifier(name: &str) -> bool {
    let mut chars = name.chars();
    let first = match chars.next() {
        Some(c) => c,
        None => return false,
    };
    if !(first.is_ascii_alphabetic() || first == '_' || first == '$') {
        return false;
    }
    chars.all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '$')
}

fn contains_js_identifier_reference(text: &str, ident: &str) -> bool {
    if ident.is_empty() {
        return false;
    }
    let mut cursor = 0usize;
    while let Some(pos) = text[cursor..].find(ident) {
        let start = cursor + pos;
        let end = start + ident.len();
        let prev = text[..start].chars().next_back();
        let next = text[end..].chars().next();
        let prev_ok = prev.map(|ch| !is_js_identifier_char(ch)).unwrap_or(true);
        let next_ok = next.map(|ch| !is_js_identifier_char(ch)).unwrap_or(true);
        if prev_ok && next_ok {
            return true;
        }
        cursor = end;
    }
    false
}

fn resolve_method_param_names(
    method_param_names_a: Option<&[String]>,
    method_param_names_b: Option<&[String]>,
    fallback_arg_count: usize,
) -> Vec<String> {
    let normalize = |names: &[String]| -> Vec<String> {
        names
            .iter()
            .enumerate()
            .map(|(i, name)| {
                let trimmed = name.trim();
                if trimmed.is_empty() {
                    format!("arg{}", i)
                } else {
                    sanitize_js_identifier(trimmed)
                }
            })
            .collect()
    };

    let candidate_a = method_param_names_a
        .filter(|names| !names.is_empty())
        .map(normalize);
    let candidate_b = method_param_names_b
        .filter(|names| !names.is_empty())
        .map(normalize);

    match (candidate_a, candidate_b) {
        (Some(a), Some(b)) if a == b => a,
        (Some(a), None) => a,
        (None, Some(b)) => b,
        (Some(a), Some(_b)) => a,
        (None, None) => default_method_param_names(fallback_arg_count),
    }
}

fn js_literal_expr_from_graph_label(label: Option<&serde_json::Value>) -> String {
    match label {
        Some(serde_json::Value::Number(num)) => num.to_string(),
        Some(serde_json::Value::Bool(b)) => b.to_string(),
        Some(serde_json::Value::Null) => "null".to_string(),
        Some(serde_json::Value::String(s)) => {
            if let Ok(int_value) = s.parse::<i64>() {
                int_value.to_string()
            } else if let Ok(float_value) = s.parse::<f64>() {
                if float_value.is_finite() {
                    float_value.to_string()
                } else {
                    serde_json::to_string(s).unwrap_or_else(|_| "\"\"".to_string())
                }
            } else {
                serde_json::to_string(s).unwrap_or_else(|_| "\"\"".to_string())
            }
        }
        Some(other) => serde_json::to_string(other).unwrap_or_else(|_| "null".to_string()),
        None => "undefined".to_string(),
    }
}

fn js_field_assignment(lhs: &str, field: &str, rhs: &str) -> String {
    if is_valid_js_identifier(field) {
        format!("{}.{} = {};", lhs, field, rhs)
    } else {
        let quoted = serde_json::to_string(field).unwrap_or_else(|_| "\"\"".to_string());
        format!("{}[{}] = {};", lhs, quoted, rhs)
    }
}

fn resolve_object_expression(
    object_id: &str,
    receiver_object_id: Option<&str>,
    object_expr_by_id: &HashMap<String, String>,
    hole_by_object_id: &HashMap<String, String>,
    hole_expr_by_key: &HashMap<String, String>,
) -> Option<String> {
    if object_id == "null" {
        return Some("null".to_string());
    }
    if let Some(hole_key) = hole_by_object_id.get(object_id) {
        if let Some(expr) = hole_expr_by_key.get(hole_key) {
            return Some(expr.clone());
        }
    }
    if let Some(expr) = object_expr_by_id.get(object_id) {
        return Some(expr.clone());
    }
    if receiver_object_id == Some(object_id) {
        return Some("this".to_string());
    }
    None
}

fn build_composed_method_code(
    method_name: &str,
    method_param_names: &[String],
    ordered_common_ops: &[(usize, crate::list_env::GraphOperation)],
    hole_bindings: &[HoleBinding],
    hole_by_object_id: &HashMap<String, String>,
    receiver_object_id: Option<&str>,
    runtime_object_expr_by_id: &HashMap<String, String>,
) -> anyhow::Result<String> {
    let method_name = sanitize_js_identifier(method_name);
    let mut body_lines: Vec<String> = Vec::new();
    let mut hole_declarations: Vec<(String, String)> = Vec::new();
    let mut hole_expr_by_key: HashMap<String, String> = HashMap::new();
    let mut object_expr_by_id: HashMap<String, String> = runtime_object_expr_by_id.clone();
    if let Some(receiver_id) = receiver_object_id {
        object_expr_by_id.insert(receiver_id.to_string(), "this".to_string());
    }

    let mut ptr_idx = 0usize;
    let mut int_idx = 0usize;
    for binding in hole_bindings {
        let var_name = if binding.return_type == "Ptr" {
            let name = format!("h_ptr_{}", ptr_idx);
            ptr_idx += 1;
            name
        } else {
            let name = format!("h_int_{}", int_idx);
            int_idx += 1;
            name
        };
        hole_declarations.push((
            var_name.clone(),
            format!("const {} = {};", var_name, binding.js_call_template),
        ));
        hole_expr_by_key.insert(binding.hole_key.clone(), var_name);
    }

    let mut tmp_index = 0usize;
    let mut last_created_object_expr: Option<String> = None;
    let mut return_expr: Option<String> = None;
    for (_op_index, op) in ordered_common_ops {
        match op.edit_type.as_str() {
            "addNode" => {
                let node_id = op
                    .id
                    .as_deref()
                    .ok_or_else(|| anyhow::anyhow!("addNode op missing id"))?;
                if op.is_literal.unwrap_or(false) {
                    object_expr_by_id.insert(
                        node_id.to_string(),
                        js_literal_expr_from_graph_label(op.label.as_ref()),
                    );
                } else {
                    let var_name = format!("tmp{}", tmp_index);
                    tmp_index += 1;
                    let ctor = graph_op_label_string(op).unwrap_or_else(|| "Object".to_string());
                    if is_valid_js_identifier(&ctor) {
                        body_lines.push(format!("const {} = new {}();", var_name, ctor));
                    } else {
                        body_lines.push(format!("const {} = {{}};", var_name));
                    }
                    last_created_object_expr = Some(var_name.clone());
                    object_expr_by_id.insert(node_id.to_string(), var_name);
                }
            }
            "addEdge" | "editEdgeReference" => {
                let from_id = op
                    .from
                    .as_deref()
                    .ok_or_else(|| anyhow::anyhow!("edge op missing from"))?;
                let to_id = op
                    .new_to
                    .as_deref()
                    .or(op.to.as_deref())
                    .ok_or_else(|| anyhow::anyhow!("edge op missing target"))?;
                let field = graph_op_label_string(op).unwrap_or_default();

                let from_expr = resolve_object_expression(
                    from_id,
                    receiver_object_id,
                    &object_expr_by_id,
                    hole_by_object_id,
                    &hole_expr_by_key,
                )
                .ok_or_else(|| {
                    anyhow::anyhow!("failed to resolve from expression for {}", from_id)
                })?;
                let to_expr = resolve_object_expression(
                    to_id,
                    receiver_object_id,
                    &object_expr_by_id,
                    hole_by_object_id,
                    &hole_expr_by_key,
                )
                .ok_or_else(|| anyhow::anyhow!("failed to resolve to expression for {}", to_id))?;

                body_lines.push(js_field_assignment(&from_expr, &field, &to_expr));
            }
            "addVariable" | "editVariableReference" => {
                let label = graph_op_label_string(op).unwrap_or_default();
                let to_id = op
                    .new_to
                    .as_deref()
                    .or(op.to.as_deref())
                    .ok_or_else(|| anyhow::anyhow!("variable op missing target"))?;
                let to_expr = resolve_object_expression(
                    to_id,
                    receiver_object_id,
                    &object_expr_by_id,
                    hole_by_object_id,
                    &hole_expr_by_key,
                )
                .ok_or_else(|| anyhow::anyhow!("failed to resolve to expression for {}", to_id))?;

                if label == "return" {
                    return_expr = Some(to_expr);
                } else {
                    body_lines.push(format!(
                        "// NOTE: variable '{}' update is omitted in composed code",
                        label
                    ));
                }
            }
            "removeNode" | "removeEdge" => {
                return Err(anyhow::anyhow!(
                    "remove operations are not supported yet in composed method generation"
                ));
            }
            other => {
                return Err(anyhow::anyhow!(
                    "unsupported operation in common plan for composed generation: {}",
                    other
                ));
            }
        }
    }

    // If value-setting edges stayed in diff (not in common ops), recover a minimal assignment
    // from hole metadata so composed code can still connect synthesized value holes.
    let mut recovered_assignments: HashSet<String> = HashSet::new();
    for binding in hole_bindings {
        if binding.return_type != "Int" || !binding.side_b.starts_with("addEdge ") {
            continue;
        }
        let Some(from_id) = token_value(&binding.side_b, "from") else {
            continue;
        };
        if from_id == "-" {
            continue;
        }
        let Some(field) = token_value(&binding.side_b, "label") else {
            continue;
        };
        if field.is_empty() || field == "<none>" || field == "-" {
            continue;
        }
        let from_expr = resolve_object_expression(
            &from_id,
            receiver_object_id,
            &object_expr_by_id,
            hole_by_object_id,
            &hole_expr_by_key,
        )
        .or_else(|| last_created_object_expr.clone());
        let Some(from_expr) = from_expr else {
            continue;
        };
        let Some(hole_expr) = hole_expr_by_key.get(&binding.hole_key) else {
            continue;
        };
        let line = js_field_assignment(&from_expr, &field, hole_expr);
        if recovered_assignments.insert(line.clone()) {
            body_lines.push(line);
        }
    }

    if let Some(expr) = return_expr {
        body_lines.push(format!("return {};", expr));
    }

    let mut lines: Vec<String> = Vec::new();
    for (var_name, decl) in hole_declarations {
        let used = body_lines
            .iter()
            .any(|line| contains_js_identifier_reference(line, &var_name));
        if used {
            lines.push(decl);
        }
    }
    lines.extend(body_lines);

    let mut code_lines = Vec::new();
    code_lines.push(format!(
        "{}({}) {{",
        method_name,
        method_param_names.join(", ")
    ));
    for line in lines {
        code_lines.push(format!("    {}", line));
    }
    code_lines.push("}".to_string());
    Ok(code_lines.join("\n"))
}

fn common_ops_in_source_order(
    common_ops: &[crate::unify_ops::Op],
    graph_ops: &[crate::list_env::GraphOperation],
) -> Vec<(usize, crate::list_env::GraphOperation)> {
    let mut pairs: Vec<(usize, crate::list_env::GraphOperation)> = common_ops
        .iter()
        .filter_map(|op| {
            parse_op_index(&op.id)
                .and_then(|idx| graph_ops.get(idx).cloned().map(|graph_op| (idx, graph_op)))
        })
        .collect();
    pairs.sort_by_key(|(idx, _)| *idx);
    pairs.dedup_by_key(|(idx, _)| *idx);
    pairs
}

fn diff_op_primary_object_id(op: &DiffOp) -> Option<String> {
    match op {
        DiffOp::Json { graph_op, .. } => match graph_op.edit_type.as_str() {
            "addNode" | "removeNode" => graph_op.id.clone(),
            "addEdge"
            | "editEdgeReference"
            | "removeEdge"
            | "addVariable"
            | "editVariableReference" => graph_op_target_id(graph_op),
            _ => None,
        },
        DiffOp::ExistNode { id, .. } => Some(id.clone()),
    }
}

fn render_common_graph_op(
    index: usize,
    op: &crate::list_env::GraphOperation,
    hole_by_object_id: &HashMap<String, String>,
) -> String {
    let resolve_id = |id: &str| {
        hole_by_object_id
            .get(id)
            .cloned()
            .unwrap_or_else(|| id.to_string())
    };

    match op.edit_type.as_str() {
        "addNode" | "removeNode" => {
            let id = op
                .id
                .as_deref()
                .map(resolve_id)
                .unwrap_or_else(|| "-".to_string());
            let label = graph_op_label_string(op).unwrap_or_else(|| "<none>".to_string());
            let is_literal = op.is_literal.unwrap_or(false);
            format!(
                "[op_{}] {}(id={}, label={}, isLiteral={})",
                index, op.edit_type, id, label, is_literal
            )
        }
        "addEdge" | "editEdgeReference" | "removeEdge" => {
            let from = op
                .from
                .as_deref()
                .map(resolve_id)
                .unwrap_or_else(|| "-".to_string());
            let to = op
                .new_to
                .as_deref()
                .or(op.to.as_deref())
                .map(resolve_id)
                .unwrap_or_else(|| "-".to_string());
            let label = graph_op_label_string(op).unwrap_or_else(|| "<none>".to_string());
            format!(
                "[op_{}] {}(from={}, to={}, label={})",
                index, op.edit_type, from, to, label
            )
        }
        "addVariable" | "editVariableReference" => {
            let to = op
                .new_to
                .as_deref()
                .or(op.to.as_deref())
                .map(resolve_id)
                .unwrap_or_else(|| "-".to_string());
            let label = graph_op_label_string(op).unwrap_or_else(|| "<none>".to_string());
            format!(
                "[op_{}] {}(label={}, to={})",
                index, op.edit_type, label, to
            )
        }
        _ => format!("[op_{}] {}", index, format_graph_op(op)),
    }
}

fn build_common_plan_artifact(
    ordered_common_ops: &[(usize, crate::list_env::GraphOperation)],
    hole_bindings: &[HoleBinding],
    hole_by_object_id: &HashMap<String, String>,
    composed_method_code: Option<String>,
) -> Option<CommonPlanArtifact> {
    if ordered_common_ops.is_empty() && hole_bindings.is_empty() {
        return None;
    }

    let mut lines: Vec<String> = Vec::new();
    lines.push("COMMON_PLAN (operation-level, ordered)".to_string());
    for (index, op) in ordered_common_ops {
        lines.push(render_common_graph_op(*index, op, hole_by_object_id));
    }
    if !hole_bindings.is_empty() {
        lines.push(String::new());
        lines.push("HOLE_BINDINGS".to_string());
        for binding in hole_bindings {
            lines.push(format!(
                "{} -> {} ({})",
                binding.hole_key, binding.spec_name, binding.return_type
            ));
        }
    }

    let mut hole_information: HashMap<String, Vec<String>> = HashMap::new();
    for binding in hole_bindings {
        let mut values = vec![
            format!("spec={}", binding.spec_name),
            format!("return={}", binding.return_type),
            format!("jsMethod={}", binding.js_method_name),
            format!("jsCall={}", binding.js_call_template),
            format!("sideA={}", binding.side_a),
            format!("sideB={}", binding.side_b),
        ];
        if let Some(anchor) = binding.anchor_a {
            values.push(format!("anchorA={}", anchor));
        }
        if let Some(anchor) = binding.anchor_b {
            values.push(format!("anchorB={}", anchor));
        }
        hole_information.insert(binding.hole_key.clone(), values);
    }

    Some(CommonPlanArtifact {
        pattern_text: lines.join("\n"),
        hole_information,
        composed_method_code,
    })
}

fn generate_specs_from_unification(
    vis_graph_a: &models::VisGraph,
    vis_graph_b: &models::VisGraph,
    base_env_a: &crate::list_env::ListEnvironment,
    base_env_b: &crate::list_env::ListEnvironment,
    receiver_object_a: Option<&str>,
    receiver_object_b: Option<&str>,
    call_arguments_a: &[serde_json::Value],
    call_arguments_b: &[serde_json::Value],
    call_argument_types_a: Option<&[String]>,
    call_argument_types_b: Option<&[String]>,
    call_argument_names_a: Option<&[String]>,
    call_argument_names_b: Option<&[String]>,
    method_param_names_a: Option<&[String]>,
    method_param_names_b: Option<&[String]>,
    operations_a: &[serde_json::Value],
    operations_b: &[serde_json::Value],
    analysis: &UnificationAnalysisResult,
    base_name: &str,
    method_name: &str,
    field_tables: Option<&FieldTables>,
    spec_meta_by_name: &mut HashMap<String, EscherSpecMeta>,
) -> anyhow::Result<GeneratedSpecsResult> {
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
    let ordered_common_ops =
        common_ops_in_source_order(&analysis.unification_result.common_a, &graph_ops_a);
    let method_param_names = resolve_method_param_names(
        method_param_names_a,
        method_param_names_b,
        call_arguments_a.len().max(call_arguments_b.len()),
    );

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
    let mut hole_bindings: Vec<HoleBinding> = Vec::new();
    let mut hole_by_object_id: HashMap<String, String> = HashMap::new();
    let mut suffix_index = 0usize;

    for (pair_index, pair) in diff_pairs.into_iter().enumerate() {
        let op_index_a = pair.op_a.as_ref().and_then(diff_op_index);
        let op_index_b = pair.op_b.as_ref().and_then(diff_op_index);
        let own_anchor_a = pair
            .op_a
            .as_ref()
            .and_then(|op| diff_op_anchor_index(op, &graph_ops_a));
        let own_anchor_b = pair
            .op_b
            .as_ref()
            .and_then(|op| diff_op_anchor_index(op, &graph_ops_b));
        let anchor_a = anchor_index_for_side(own_anchor_a, own_anchor_b, operations_a.len());
        let anchor_b = anchor_index_for_side(own_anchor_b, own_anchor_a, operations_b.len());

        // Input environment should be the state immediately before the target diff op.
        // For ExistNode (no direct op index), we fall back to the paired side index.
        let env_input_a = match anchor_a {
            Some(index) => build_environment_prefix(base_env_a, operations_a, index, false)?,
            None => env_boundary_a.clone(),
        };
        let env_input_b = match anchor_b {
            Some(index) => build_environment_prefix(base_env_b, operations_b, index, false)?,
            None => env_boundary_b.clone(),
        };

        // Output extraction for Json ops must observe the post-state of that op.
        let env_output_a = match op_index_a {
            Some(index) => build_environment_prefix(base_env_a, operations_a, index, true)?,
            None => env_input_a.clone(),
        };
        let env_output_b = match op_index_b {
            Some(index) => build_environment_prefix(base_env_b, operations_b, index, true)?,
            None => env_input_b.clone(),
        };
        if trace_enabled() {
            println!(
                "TRACE: diff_pair[{}] env_input_a:\n{}",
                pair_index,
                env_input_a.to_debug_string()
            );
            println!(
                "TRACE: diff_pair[{}] env_input_b:\n{}",
                pair_index,
                env_input_b.to_debug_string()
            );
            println!(
                "TRACE: diff_pair[{}] env_output_a:\n{}",
                pair_index,
                env_output_a.to_debug_string()
            );
            println!(
                "TRACE: diff_pair[{}] env_output_b:\n{}",
                pair_index,
                env_output_b.to_debug_string()
            );
        }

        let (mut out_a, out_ty_a) = match &pair.op_a {
            Some(op) => {
                let (out, ty) = output_for_diff_op(op, &env_output_a, vis_graph_a);
                (out, Some(ty))
            }
            None => (serde_json::json!(-1), None),
        };
        let (mut out_b, out_ty_b) = match &pair.op_b {
            Some(op) => {
                let (out, ty) = output_for_diff_op(op, &env_output_b, vis_graph_b);
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

        let field_stub_cases = vec![
            EscherCase {
                env: env_input_a.clone(),
                vis_graph: vis_graph_a.clone(),
                arguments: Vec::new(),
                arg_names: Vec::new(),
                arg_types: None,
                receiver_arg_index: None,
                output: serde_json::Value::Null,
            },
            EscherCase {
                env: env_input_b.clone(),
                vis_graph: vis_graph_b.clone(),
                arguments: Vec::new(),
                arg_names: Vec::new(),
                arg_types: None,
                receiver_arg_index: None,
                output: serde_json::Value::Null,
            },
        ];
        let (_value_fields, pointer_fields) = resolve_field_order(&field_stub_cases, field_tables)?;

        let mut arg_bundle_a = build_case_arguments_from_variables(
            &env_input_a,
            vis_graph_a,
            &pointer_fields,
            receiver_object_a,
        )?;
        let mut arg_bundle_b = build_case_arguments_from_variables(
            &env_input_b,
            vis_graph_b,
            &pointer_fields,
            receiver_object_b,
        )?;

        append_method_call_arguments(
            &mut arg_bundle_a,
            call_arguments_a,
            call_argument_types_a,
            call_argument_names_a,
            &env_input_a,
            vis_graph_a,
            &pointer_fields,
        )?;
        append_method_call_arguments(
            &mut arg_bundle_b,
            call_arguments_b,
            call_argument_types_b,
            call_argument_names_b,
            &env_input_b,
            vis_graph_b,
            &pointer_fields,
        )?;

        if arg_bundle_a.names != arg_bundle_b.names {
            return Err(anyhow::anyhow!(
                "argument variable names differ across cases: {:?} vs {:?}",
                arg_bundle_a.names,
                arg_bundle_b.names
            ));
        }
        if arg_bundle_a.types != arg_bundle_b.types {
            return Err(anyhow::anyhow!(
                "argument types differ across cases: {:?} vs {:?}",
                arg_bundle_a.types,
                arg_bundle_b.types
            ));
        }
        if arg_bundle_a.receiver_arg_index != arg_bundle_b.receiver_arg_index {
            return Err(anyhow::anyhow!(
                "receiver arg differs across cases: {:?} vs {:?}",
                arg_bundle_a.receiver_arg_index,
                arg_bundle_b.receiver_arg_index
            ));
        }
        if arg_bundle_a.receiver_arg_index.is_none() {
            let ptr_count = arg_bundle_a
                .types
                .iter()
                .filter(|t| t.as_str() == "Ptr")
                .count();
            if ptr_count > 1 {
                return Err(anyhow::anyhow!(
                    "receiver arg unresolved with multiple Ptr inputs"
                ));
            }
        }

        let arg_names_a = arg_bundle_a.names.clone();
        let arg_names_b = arg_bundle_b.names.clone();
        let cases = vec![
            EscherCase {
                env: env_input_a,
                vis_graph: vis_graph_a.clone(),
                arguments: arg_bundle_a.values,
                arg_names: arg_names_a,
                arg_types: Some(arg_bundle_a.types.clone()),
                receiver_arg_index: arg_bundle_a.receiver_arg_index,
                output: out_a,
            },
            EscherCase {
                env: env_input_b,
                vis_graph: vis_graph_b.clone(),
                arguments: arg_bundle_b.values,
                arg_names: arg_names_b,
                arg_types: Some(arg_bundle_b.types),
                receiver_arg_index: arg_bundle_b.receiver_arg_index,
                output: out_b,
            },
        ];

        let spec_name = build_spec_name(base_name, suffix_index);
        suffix_index += 1;
        let hole_key = format!("__hole_{}", pair_index);
        let js_method_name = spec_name_to_js_method_name(&spec_name);
        let js_call_template = build_hole_call_template(&js_method_name, &method_param_names);

        if let Some(id_a) = pair.op_a.as_ref().and_then(diff_op_primary_object_id) {
            hole_by_object_id
                .entry(id_a)
                .or_insert_with(|| hole_key.clone());
        }
        if let Some(id_b) = pair.op_b.as_ref().and_then(diff_op_primary_object_id) {
            let canonical_b = canonicalize_object_id(&id_b, &object_id_mapping_inv);
            hole_by_object_id
                .entry(canonical_b)
                .or_insert_with(|| hole_key.clone());
        }

        let side_a = pair
            .op_a
            .as_ref()
            .map(format_diff_op)
            .unwrap_or_else(|| "<none>".to_string());
        let side_b = pair
            .op_b
            .as_ref()
            .map(format_diff_op)
            .unwrap_or_else(|| "<none>".to_string());
        hole_bindings.push(HoleBinding {
            hole_key,
            spec_name: spec_name.clone(),
            return_type: return_type.as_escher_type().to_string(),
            js_method_name,
            js_call_template,
            side_a,
            side_b,
            anchor_a,
            anchor_b,
        });

        let meta = derive_spec_meta_with_fields(&cases, field_tables)?;
        spec_meta_by_name.insert(spec_name.clone(), meta);

        let spec = build_escher_spec(
            &spec_name,
            return_type.as_escher_type(),
            &cases,
            field_tables,
        )?;
        trace_json(&format!("EscherSpec {}", spec.name), &spec);
        specs.push(spec);
    }

    let runtime_object_expr_by_id =
        build_runtime_object_expression_map(vis_graph_a, receiver_object_a);
    let composed_method_code = match build_composed_method_code(
        method_name,
        &method_param_names,
        &ordered_common_ops,
        &hole_bindings,
        &hole_by_object_id,
        receiver_object_a,
        &runtime_object_expr_by_id,
    ) {
        Ok(code) => Some(code),
        Err(err) => {
            eprintln!("Failed to compose primary method code: {}", err);
            None
        }
    };

    let common_plan = build_common_plan_artifact(
        &ordered_common_ops,
        &hole_bindings,
        &hole_by_object_id,
        composed_method_code,
    );

    Ok(GeneratedSpecsResult { specs, common_plan })
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
    let resolve_target_output = |target_id: Option<&str>| -> (serde_json::Value, OutputType) {
        if let Some(to_id) = target_id {
            if env.obj_id_to_index.contains_key(to_id) {
                let value = match bfs_local_index_for_object(env, vis_graph, to_id) {
                    Ok(Some(idx)) => serde_json::json!(idx),
                    Ok(None) | Err(_) => serde_json::Value::Null,
                };
                (value, OutputType::Ptr)
            } else if let Some(val) = env.literal_id_to_value.get(to_id) {
                let value = value_to_i32(val)
                    .map(|v| serde_json::json!(v))
                    .unwrap_or_else(|| serde_json::json!(-1));
                (value, OutputType::Int)
            } else if to_id == "null" {
                (serde_json::Value::Null, OutputType::Ptr)
            } else {
                (serde_json::json!(-1), OutputType::Int)
            }
        } else {
            (serde_json::json!(-1), OutputType::Int)
        }
    };

    match operation.edit_type.as_str() {
        "addEdge" | "removeEdge" | "addVariable" => resolve_target_output(operation.to.as_deref()),
        "editEdgeReference" | "editVariableReference" => {
            resolve_target_output(operation.new_to.as_deref().or(operation.to.as_deref()))
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
                    Ok(Some(idx)) => serde_json::json!(idx),
                    Ok(None) | Err(_) => serde_json::Value::Null,
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
            Ok(Some(idx)) => serde_json::json!(idx),
            Ok(None) | Err(_) => serde_json::Value::Null,
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

fn collect_unique_method_names(method_calls: &[MethodCallOperation]) -> Vec<String> {
    let mut names: Vec<String> = method_calls
        .iter()
        .map(|m| m.method_name.trim().to_string())
        .filter(|name| !name.is_empty())
        .collect();
    names.sort();
    names.dedup();
    names
}

fn detect_unsupported_remove_operation(
    operations_list: &[Vec<serde_json::Value>],
) -> Option<String> {
    for (call_index, operations) in operations_list.iter().enumerate() {
        for (op_index, op) in operations.iter().enumerate() {
            let edit_type = op
                .get("editType")
                .or_else(|| op.get("edit_type"))
                .and_then(|v| v.as_str());
            let Some(edit_type) = edit_type else {
                continue;
            };
            if matches!(
                edit_type,
                "removeNode" | "removeEdge" | "deleteNode" | "deleteEdge" | "deleteVariable"
            ) {
                return Some(format!(
                    "Unsupported remove operation detected at call {} op {}: {}",
                    call_index, op_index, edit_type
                ));
            }
        }
    }
    None
}

fn is_runtime_scoped_id(id: &str) -> bool {
    id.contains("-call")
}

fn canonicalize_runtime_scope_segment(input: &str, marker: &str) -> String {
    if !input.contains(marker) {
        return input.to_string();
    }

    let mut out = String::with_capacity(input.len());
    let bytes = input.as_bytes();
    let mut i: usize = 0;
    while i < bytes.len() {
        if input[i..].starts_with(marker) {
            out.push_str(marker);
            i += marker.len();
            let digit_start = i;
            while i < bytes.len() && bytes[i].is_ascii_digit() {
                i += 1;
            }
            if i > digit_start {
                out.push('*');
            }
        } else {
            out.push(bytes[i] as char);
            i += 1;
        }
    }
    out
}

fn canonicalize_runtime_scoped_id(id: &str) -> String {
    let mut canonical = id.to_string();
    for marker in [
        "FunctionExpression",
        "FunctionDeclaration",
        "ArrowFunction",
        "MethodDefinition",
    ] {
        canonical = canonicalize_runtime_scope_segment(&canonical, marker);
    }
    canonical
}

fn build_runtime_to_temp_map(method_calls: &[MethodCallOperation]) -> HashMap<String, String> {
    let mut runtime_to_temp: HashMap<String, String> = HashMap::new();
    let mut canonical_to_temp: HashMap<String, String> = HashMap::new();
    let mut conflicting_canonical: HashSet<String> = HashSet::new();
    for call in method_calls {
        let Some(id_mapping) = call.id_mapping.as_ref() else {
            continue;
        };
        for (temp_id, runtime_id) in id_mapping {
            if temp_id.is_empty() || runtime_id.is_empty() {
                continue;
            }
            runtime_to_temp
                .entry(runtime_id.clone())
                .or_insert_with(|| temp_id.clone());
            if is_runtime_scoped_id(runtime_id) {
                let canonical_runtime = canonicalize_runtime_scoped_id(runtime_id);
                if canonical_runtime != *runtime_id {
                    match canonical_to_temp.get(&canonical_runtime) {
                        Some(existing) if existing != temp_id => {
                            conflicting_canonical.insert(canonical_runtime.clone());
                        }
                        Some(_) => {}
                        None => {
                            canonical_to_temp.insert(canonical_runtime.clone(), temp_id.clone());
                        }
                    }
                }
            }
        }
    }
    for canonical in conflicting_canonical {
        canonical_to_temp.remove(&canonical);
    }
    for (canonical_runtime, temp_id) in canonical_to_temp {
        runtime_to_temp.entry(canonical_runtime).or_insert(temp_id);
    }
    runtime_to_temp
}

fn remap_id_with_runtime_map(id: &str, runtime_to_temp: &HashMap<String, String>) -> String {
    if let Some(mapped) = runtime_to_temp.get(id) {
        return mapped.clone();
    }
    if is_runtime_scoped_id(id) {
        let canonical_id = canonicalize_runtime_scoped_id(id);
        if canonical_id != id {
            if let Some(mapped) = runtime_to_temp.get(&canonical_id) {
                return mapped.clone();
            }
            let mut matched: Option<&String> = None;
            let mut conflict = false;
            for (runtime_id, temp_id) in runtime_to_temp.iter() {
                if !is_runtime_scoped_id(runtime_id) {
                    continue;
                }
                if canonicalize_runtime_scoped_id(runtime_id) == canonical_id {
                    match matched {
                        Some(existing) if existing != temp_id => {
                            conflict = true;
                            break;
                        }
                        Some(_) => {}
                        None => matched = Some(temp_id),
                    }
                }
            }
            if !conflict {
                if let Some(mapped) = matched {
                    return mapped.clone();
                }
            }
        }
    }
    id.to_string()
}

fn normalize_operations_with_runtime_map(
    operations: &[serde_json::Value],
    runtime_to_temp: &HashMap<String, String>,
) -> Vec<serde_json::Value> {
    let mut normalized: Vec<serde_json::Value> = Vec::with_capacity(operations.len());
    for op in operations {
        let mut next = op.clone();
        if let Some(obj) = next.as_object_mut() {
            for key in ["id", "from", "to", "oldTo", "newTo", "old_to", "new_to"] {
                if let Some(value) = obj.get_mut(key) {
                    if let Some(id) = value.as_str() {
                        let mapped = remap_id_with_runtime_map(id, runtime_to_temp);
                        if mapped != id {
                            *value = serde_json::Value::String(mapped);
                        }
                    }
                }
            }
        }
        normalized.push(next);
    }
    normalized
}

fn detect_unresolved_runtime_scoped_ids(
    operations_list: &[Vec<serde_json::Value>],
) -> Option<String> {
    for (call_index, operations) in operations_list.iter().enumerate() {
        for (op_index, op) in operations.iter().enumerate() {
            let Some(obj) = op.as_object() else {
                continue;
            };
            for key in ["id", "from", "to", "oldTo", "newTo", "old_to", "new_to"] {
                let Some(value) = obj.get(key) else {
                    continue;
                };
                let Some(id) = value.as_str() else {
                    continue;
                };
                if is_runtime_scoped_id(id) {
                    return Some(format!(
                        "Unresolved runtime-scoped id detected at call {} op {} key {}: {}. Normalize operations to temp IDs before synthesis.",
                        call_index, op_index, key, id
                    ));
                }
            }
        }
    }
    None
}

fn sanitize_base_name(name: &str) -> String {
    if name.starts_with("call") && name[4..].chars().all(|c| c.is_ascii_digit()) {
        "aux".to_string()
    } else {
        name.to_string()
    }
}

fn normalize_actual_graph_with_id_mapping(
    actual_graph: &VisGraph,
    id_mapping: &HashMap<String, String>,
) -> Option<VisGraph> {
    if id_mapping.is_empty() {
        return Some(actual_graph.clone());
    }

    let mut actual_to_temp: HashMap<&str, &str> = HashMap::new();
    for (temp_id, actual_id) in id_mapping {
        if temp_id.is_empty() || actual_id.is_empty() {
            continue;
        }
        if let Some(existing) = actual_to_temp.insert(actual_id.as_str(), temp_id.as_str()) {
            if existing != temp_id.as_str() {
                eprintln!(
                    "Conflicting id mapping for runtime id '{}': '{}' vs '{}'",
                    actual_id, existing, temp_id
                );
                return None;
            }
        }
    }

    if actual_to_temp.is_empty() {
        return None;
    }

    let remap_id = |id: &str| {
        actual_to_temp
            .get(id)
            .map(|mapped| (*mapped).to_string())
            .unwrap_or_else(|| id.to_string())
    };

    let nodes: Vec<crate::models::Node> = actual_graph
        .nodes
        .iter()
        .map(|node| crate::models::Node {
            id: remap_id(&node.id),
            is_literal: node.is_literal,
            label: node.label.clone(),
        })
        .collect();

    let mut seen_node_ids: HashSet<&str> = HashSet::new();
    for node in &nodes {
        if !seen_node_ids.insert(node.id.as_str()) {
            eprintln!(
                "Failed to normalize actualGraph: duplicate node id '{}' after remapping",
                node.id
            );
            return None;
        }
    }

    let edges: Vec<crate::models::Edge> = actual_graph
        .edges
        .iter()
        .map(|edge| crate::models::Edge {
            from: remap_id(&edge.from),
            to: remap_id(&edge.to),
            label: edge.label.clone(),
        })
        .collect();

    Some(VisGraph { nodes, edges })
}

fn normalize_vis_graph_with_runtime_map(
    vis_graph: &VisGraph,
    runtime_to_temp: &HashMap<String, String>,
) -> VisGraph {
    if runtime_to_temp.is_empty() {
        return vis_graph.clone();
    }

    let remap_id = |id: &str| remap_id_with_runtime_map(id, runtime_to_temp);
    let nodes = vis_graph
        .nodes
        .iter()
        .map(|node| crate::models::Node {
            id: remap_id(&node.id),
            is_literal: node.is_literal,
            label: node.label.clone(),
        })
        .collect();
    let edges = vis_graph
        .edges
        .iter()
        .map(|edge| crate::models::Edge {
            from: remap_id(&edge.from),
            to: remap_id(&edge.to),
            label: edge.label.clone(),
        })
        .collect();
    VisGraph { nodes, edges }
}

fn resolve_method_call_vis_graph(
    call: &MethodCallOperation,
    fallback: &VisGraph,
    runtime_to_temp: &HashMap<String, String>,
) -> VisGraph {
    let base_graph = if let Some(precond_graph) = call.precond_graph.as_ref() {
        if let Some(id_mapping) = call.id_mapping.as_ref() {
            if let Some(remapped) =
                normalize_actual_graph_with_id_mapping(precond_graph, id_mapping)
            {
                remapped
            } else {
                eprintln!(
                    "Failed to normalize precondGraph for {} ({}) - using raw precondGraph",
                    call.call_label, call.context_sensitive_id
                );
                precond_graph.clone()
            }
        } else {
            precond_graph.clone()
        }
    } else if let Some(actual_graph) = call.actual_graph.as_ref() {
        if let Some(id_mapping) = call.id_mapping.as_ref() {
            if let Some(remapped) = normalize_actual_graph_with_id_mapping(actual_graph, id_mapping)
            {
                remapped
            } else {
                eprintln!(
                    "Failed to normalize actualGraph for {} ({}) - using raw actualGraph",
                    call.call_label, call.context_sensitive_id
                );
                actual_graph.clone()
            }
        } else {
            actual_graph.clone()
        }
    } else {
        fallback.clone()
    };
    normalize_vis_graph_with_runtime_map(&base_graph, runtime_to_temp)
}

fn infer_field_tables_for_method_calls(
    method_calls: &[MethodCallOperation],
    fallback_vis_graph: &VisGraph,
    runtime_to_temp: &HashMap<String, String>,
) -> Option<FieldTables> {
    let mut value_fields: Vec<String> = Vec::new();
    let mut pointer_fields: Vec<String> = Vec::new();
    let mut seen_values: HashSet<String> = HashSet::new();
    let mut seen_pointers: HashSet<String> = HashSet::new();

    for call in method_calls {
        if let Some(tables) = call.field_tables.as_ref() {
            for field in &tables.value {
                if seen_pointers.contains(field) {
                    continue;
                }
                if seen_values.insert(field.clone()) {
                    value_fields.push(field.clone());
                }
            }
            for field in &tables.pointer {
                if seen_values.contains(field) {
                    continue;
                }
                if seen_pointers.insert(field.clone()) {
                    pointer_fields.push(field.clone());
                }
            }
        }

        let vis_graph = resolve_method_call_vis_graph(call, fallback_vis_graph, runtime_to_temp);
        let (values, pointers) = analyze_fields_for_graph(&vis_graph);
        for field in values {
            if seen_pointers.contains(&field) {
                continue;
            }
            if seen_values.insert(field.clone()) {
                value_fields.push(field);
            }
        }
        for field in pointers {
            if seen_values.contains(&field) {
                continue;
            }
            if seen_pointers.insert(field.clone()) {
                pointer_fields.push(field);
            }
        }
    }

    if value_fields.is_empty() && pointer_fields.is_empty() {
        None
    } else {
        Some(FieldTables {
            value: value_fields,
            pointer: pointer_fields,
        })
    }
}

fn merge_field_tables(a: Option<&FieldTables>, b: Option<&FieldTables>) -> Option<FieldTables> {
    match (a, b) {
        (None, None) => None,
        (Some(one), None) | (None, Some(one)) => Some(FieldTables {
            value: dedupe_preserve_order(&one.value),
            pointer: dedupe_preserve_order(&one.pointer),
        }),
        (Some(left), Some(right)) => {
            if field_table_conflict(left, right) {
                return None;
            }
            Some(FieldTables {
                value: merge_field_list(&left.value, &right.value),
                pointer: merge_field_list(&left.pointer, &right.pointer),
            })
        }
    }
}

fn field_table_conflict(a: &FieldTables, b: &FieldTables) -> bool {
    let a_value: HashSet<&str> = a.value.iter().map(|s| s.as_str()).collect();
    let a_pointer: HashSet<&str> = a.pointer.iter().map(|s| s.as_str()).collect();
    let b_value: HashSet<&str> = b.value.iter().map(|s| s.as_str()).collect();
    let b_pointer: HashSet<&str> = b.pointer.iter().map(|s| s.as_str()).collect();
    a_value.iter().any(|f| b_pointer.contains(*f)) || a_pointer.iter().any(|f| b_value.contains(*f))
}

fn merge_field_list(base: &[String], extra: &[String]) -> Vec<String> {
    let mut out = dedupe_preserve_order(base);
    let mut seen: HashSet<String> = out.iter().cloned().collect();
    for item in extra {
        if seen.insert(item.clone()) {
            out.push(item.clone());
        }
    }
    out
}

fn dedupe_preserve_order(values: &[String]) -> Vec<String> {
    let mut seen: HashSet<String> = HashSet::new();
    let mut out = Vec::new();
    for v in values {
        if seen.insert(v.clone()) {
            out.push(v.clone());
        }
    }
    out
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
    fn infer_field_tables_prefers_actual_graph_without_mapping() {
        let fallback_vis_graph = VisGraph {
            nodes: vec![
                crate::models::Node {
                    id: "obj1".to_string(),
                    is_literal: false,
                    label: json!("Node"),
                },
                crate::models::Node {
                    id: "obj2".to_string(),
                    is_literal: false,
                    label: json!("Node"),
                },
                crate::models::Node {
                    id: "lit1".to_string(),
                    is_literal: true,
                    label: json!("42"),
                },
            ],
            edges: vec![
                crate::models::Edge {
                    from: "obj1".to_string(),
                    to: "obj2".to_string(),
                    label: "next".to_string(),
                },
                crate::models::Edge {
                    from: "obj1".to_string(),
                    to: "lit1".to_string(),
                    label: "val".to_string(),
                },
            ],
        };
        let actual_graph = VisGraph {
            nodes: vec![
                crate::models::Node {
                    id: "objX".to_string(),
                    is_literal: false,
                    label: json!("Node"),
                },
                crate::models::Node {
                    id: "objY".to_string(),
                    is_literal: false,
                    label: json!("Node"),
                },
            ],
            edges: vec![crate::models::Edge {
                from: "objX".to_string(),
                to: "objY".to_string(),
                label: "evil".to_string(),
            }],
        };
        let method_calls = vec![MethodCallOperation {
            call_label: "call1".to_string(),
            context_sensitive_id: "main".to_string(),
            receiver_object: "obj1".to_string(),
            method_name: "append".to_string(),
            arguments: vec![],
            argument_types: None,
            argument_names: None,
            method_param_names: None,
            operations: vec![],
            precond_graph: None,
            actual_graph: Some(actual_graph),
            id_mapping: None,
            field_tables: None,
        }];

        let runtime_to_temp: HashMap<String, String> = HashMap::new();
        let tables = infer_field_tables_for_method_calls(
            &method_calls,
            &fallback_vis_graph,
            &runtime_to_temp,
        )
        .unwrap();
        assert!(tables.pointer.contains(&"evil".to_string()));
        assert!(!tables.value.contains(&"val".to_string()));
        assert!(!tables.pointer.contains(&"next".to_string()));
    }

    #[test]
    fn normalize_operations_with_runtime_map_remaps_runtime_ids() {
        let operations = vec![json!({
            "editType": "addEdge",
            "from": "main-call2-FunctionExpression4-new2",
            "to": "__temp5",
            "label": "next"
        })];
        let runtime_to_temp = HashMap::from([(
            "main-call2-FunctionExpression4-new2".to_string(),
            "__temp4".to_string(),
        )]);
        let normalized = normalize_operations_with_runtime_map(&operations, &runtime_to_temp);
        assert_eq!(
            normalized[0]
                .get("from")
                .and_then(|v| v.as_str())
                .unwrap_or_default(),
            "__temp4"
        );
        assert!(detect_unresolved_runtime_scoped_ids(&[normalized]).is_none());
    }

    #[test]
    fn normalize_operations_with_runtime_map_accepts_generation_shifted_runtime_ids() {
        let operations = vec![json!({
            "editType": "addEdge",
            "from": "main-call2-FunctionExpression4-new2",
            "to": "__temp5",
            "label": "next"
        })];
        let runtime_to_temp = HashMap::from([(
            "main-call2-FunctionExpression5-new2".to_string(),
            "__temp4".to_string(),
        )]);
        let normalized = normalize_operations_with_runtime_map(&operations, &runtime_to_temp);
        assert_eq!(
            normalized[0]
                .get("from")
                .and_then(|v| v.as_str())
                .unwrap_or_default(),
            "__temp4"
        );
        assert!(detect_unresolved_runtime_scoped_ids(&[normalized]).is_none());
    }

    #[test]
    fn normalize_operations_with_runtime_map_supports_snake_case_edge_keys() {
        let operations = vec![json!({
            "editType": "editEdgeReference",
            "old_to": "main-call2-FunctionExpression4-new2",
            "new_to": "main-call2-FunctionExpression4-new3",
            "label": "next"
        })];
        let runtime_to_temp = HashMap::from([
            (
                "main-call2-FunctionExpression5-new2".to_string(),
                "__temp4".to_string(),
            ),
            (
                "main-call2-FunctionExpression5-new3".to_string(),
                "__temp5".to_string(),
            ),
        ]);
        let normalized = normalize_operations_with_runtime_map(&operations, &runtime_to_temp);
        assert_eq!(
            normalized[0]
                .get("old_to")
                .and_then(|v| v.as_str())
                .unwrap_or_default(),
            "__temp4"
        );
        assert_eq!(
            normalized[0]
                .get("new_to")
                .and_then(|v| v.as_str())
                .unwrap_or_default(),
            "__temp5"
        );
        assert!(detect_unresolved_runtime_scoped_ids(&[normalized]).is_none());
    }

    #[test]
    fn detect_unresolved_runtime_scoped_ids_reports_unmapped_ids() {
        let operations = vec![vec![json!({
            "editType": "addEdge",
            "from": "main-call9-FunctionExpression1-new1",
            "to": "__temp1",
            "label": "next"
        })]];
        let message = detect_unresolved_runtime_scoped_ids(&operations)
            .expect("unresolved runtime id should be reported");
        assert!(message.contains("Unresolved runtime-scoped id detected"));
    }

    #[test]
    fn infer_field_tables_uses_actual_graph_when_id_mapping_is_present() {
        let fallback_vis_graph = VisGraph {
            nodes: vec![crate::models::Node {
                id: "obj1".to_string(),
                is_literal: false,
                label: json!("Node"),
            }],
            edges: vec![],
        };
        let actual_graph = VisGraph {
            nodes: vec![
                crate::models::Node {
                    id: "runtime-root".to_string(),
                    is_literal: false,
                    label: json!("Node"),
                },
                crate::models::Node {
                    id: "runtime-lit".to_string(),
                    is_literal: true,
                    label: json!("99"),
                },
            ],
            edges: vec![crate::models::Edge {
                from: "runtime-root".to_string(),
                to: "runtime-lit".to_string(),
                label: "payload".to_string(),
            }],
        };
        let method_calls = vec![MethodCallOperation {
            call_label: "call1".to_string(),
            context_sensitive_id: "main".to_string(),
            receiver_object: "__temp1".to_string(),
            method_name: "append".to_string(),
            arguments: vec![],
            argument_types: None,
            argument_names: None,
            method_param_names: None,
            operations: vec![],
            precond_graph: None,
            actual_graph: Some(actual_graph),
            id_mapping: Some(HashMap::from([
                ("__temp1".to_string(), "runtime-root".to_string()),
                ("__temp2".to_string(), "runtime-lit".to_string()),
            ])),
            field_tables: None,
        }];

        let runtime_to_temp: HashMap<String, String> = HashMap::new();
        let tables = infer_field_tables_for_method_calls(
            &method_calls,
            &fallback_vis_graph,
            &runtime_to_temp,
        )
        .unwrap();
        assert!(tables.value.contains(&"payload".to_string()));
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

    #[test]
    fn append_method_call_arguments_appends_int_values() {
        let vis_graph = simple_vis_graph_with_root();
        let env = list_env::ListEnvironment::from_vis_graph(&vis_graph);
        let mut bundle =
            build_case_arguments_from_variables(&env, &vis_graph, &[], Some("obj1")).unwrap();
        let call_arguments = vec![json!(26)];
        let call_argument_types = vec!["Int".to_string()];

        append_method_call_arguments(
            &mut bundle,
            &call_arguments,
            Some(&call_argument_types),
            None,
            &env,
            &vis_graph,
            &[],
        )
        .unwrap();

        assert_eq!(bundle.types, vec!["Ptr".to_string(), "Int".to_string()]);
        assert_eq!(bundle.values[1], json!(26));
    }

    #[test]
    fn append_method_call_arguments_maps_ptr_object_id() {
        let vis_graph = VisGraph {
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
                crate::models::Node {
                    id: "obj2".to_string(),
                    is_literal: false,
                    label: json!("Node"),
                },
            ],
            edges: vec![
                crate::models::Edge {
                    from: "__Variable-lst".to_string(),
                    to: "obj1".to_string(),
                    label: "lst".to_string(),
                },
                crate::models::Edge {
                    from: "obj1".to_string(),
                    to: "obj2".to_string(),
                    label: "next".to_string(),
                },
            ],
        };
        let env = list_env::ListEnvironment::from_vis_graph(&vis_graph);
        let pointer_fields = vec!["next".to_string()];
        let mut bundle =
            build_case_arguments_from_variables(&env, &vis_graph, &pointer_fields, Some("obj1"))
                .unwrap();
        let call_arguments = vec![json!("obj2")];
        let call_argument_types = vec!["Ptr".to_string()];

        append_method_call_arguments(
            &mut bundle,
            &call_arguments,
            Some(&call_argument_types),
            None,
            &env,
            &vis_graph,
            &pointer_fields,
        )
        .unwrap();

        assert_eq!(bundle.types, vec!["Ptr".to_string(), "Ptr".to_string()]);
        assert_eq!(bundle.values[1], json!(1));
    }

    #[test]
    fn anchor_index_prefers_own_index() {
        let anchor = anchor_index_for_side(Some(3), Some(1), 10);
        assert_eq!(anchor, Some(3));
    }

    #[test]
    fn anchor_index_uses_other_index_when_own_missing() {
        let anchor = anchor_index_for_side(None, Some(2), 5);
        assert_eq!(anchor, Some(2));
    }

    #[test]
    fn anchor_index_ignores_out_of_range_other_index() {
        let anchor = anchor_index_for_side(None, Some(7), 3);
        assert_eq!(anchor, None);
    }

    #[test]
    fn first_reference_index_for_object_finds_edge_reference() {
        let ops = vec![
            crate::list_env::GraphOperation {
                edit_type: "addNode".to_string(),
                id: Some("__temp1".to_string()),
                label: Some(json!("Node")),
                is_literal: Some(false),
                node_type: None,
                from: None,
                to: None,
                old_to: None,
                new_to: None,
                old_label: None,
                new_label: None,
            },
            crate::list_env::GraphOperation {
                edit_type: "addEdge".to_string(),
                id: None,
                label: Some(json!("next")),
                is_literal: None,
                node_type: None,
                from: Some("main-new1".to_string()),
                to: Some("__temp1".to_string()),
                old_to: None,
                new_to: None,
                old_label: None,
                new_label: None,
            },
        ];
        assert_eq!(first_reference_index_for_object("main-new1", &ops), Some(1));
        assert_eq!(first_reference_index_for_object("__temp1", &ops), Some(0));
    }

    #[test]
    fn first_reference_index_for_object_finds_old_to_and_new_to() {
        let ops = vec![
            crate::list_env::GraphOperation {
                edit_type: "editEdgeReference".to_string(),
                id: None,
                label: Some(json!("next")),
                is_literal: None,
                node_type: None,
                from: Some("main-new1".to_string()),
                to: None,
                old_to: Some("old-target".to_string()),
                new_to: Some("new-target".to_string()),
                old_label: None,
                new_label: None,
            },
            crate::list_env::GraphOperation {
                edit_type: "addEdge".to_string(),
                id: None,
                label: Some(json!("next")),
                is_literal: None,
                node_type: None,
                from: Some("__temp1".to_string()),
                to: Some("old-target".to_string()),
                old_to: None,
                new_to: None,
                old_label: None,
                new_label: None,
            },
        ];

        // Should prefer the earliest mention, including oldTo/newTo.
        assert_eq!(
            first_reference_index_for_object("old-target", &ops),
            Some(0)
        );
        assert_eq!(
            first_reference_index_for_object("new-target", &ops),
            Some(0)
        );
    }

    #[test]
    fn diff_op_anchor_index_for_exist_node_uses_first_reference() {
        let op = DiffOp::ExistNode {
            id: "main-new1".to_string(),
            is_literal: false,
            label: None,
        };
        let ops = vec![crate::list_env::GraphOperation {
            edit_type: "addEdge".to_string(),
            id: None,
            label: Some(json!("next")),
            is_literal: None,
            node_type: None,
            from: Some("main-new1".to_string()),
            to: Some("__temp1".to_string()),
            old_to: None,
            new_to: None,
            old_label: None,
            new_label: None,
        }];
        assert_eq!(diff_op_anchor_index(&op, &ops), Some(0));
    }

    #[test]
    fn common_ops_in_source_order_uses_op_index_order() {
        use crate::unify_ops::{GraphOp, NodeExpr, Op};

        let graph_ops = vec![
            crate::list_env::GraphOperation {
                edit_type: "addNode".to_string(),
                id: Some("n0".to_string()),
                label: Some(json!("Node")),
                is_literal: Some(false),
                node_type: None,
                from: None,
                to: None,
                old_to: None,
                new_to: None,
                old_label: None,
                new_label: None,
            },
            crate::list_env::GraphOperation {
                edit_type: "addNode".to_string(),
                id: Some("n1".to_string()),
                label: Some(json!("Node")),
                is_literal: Some(false),
                node_type: None,
                from: None,
                to: None,
                old_to: None,
                new_to: None,
                old_label: None,
                new_label: None,
            },
            crate::list_env::GraphOperation {
                edit_type: "addEdge".to_string(),
                id: None,
                label: Some(json!("next")),
                is_literal: None,
                node_type: None,
                from: Some("n0".to_string()),
                to: Some("n1".to_string()),
                old_to: None,
                new_to: None,
                old_label: None,
                new_label: None,
            },
        ];

        let common_ops = vec![
            Op {
                id: "op_2".to_string(),
                kind: GraphOp::Node(NodeExpr::NullNode),
            },
            Op {
                id: "op_0".to_string(),
                kind: GraphOp::Node(NodeExpr::NullNode),
            },
        ];

        let ordered = common_ops_in_source_order(&common_ops, &graph_ops);
        let indices: Vec<usize> = ordered.into_iter().map(|(idx, _)| idx).collect();
        assert_eq!(indices, vec![0, 2]);
    }

    #[test]
    fn render_common_graph_op_replaces_hole_references() {
        let op = crate::list_env::GraphOperation {
            edit_type: "addEdge".to_string(),
            id: None,
            label: Some(json!("val")),
            is_literal: None,
            node_type: None,
            from: Some("__temp1".to_string()),
            to: Some("__temp2".to_string()),
            old_to: None,
            new_to: None,
            old_label: None,
            new_label: None,
        };
        let mut holes = HashMap::new();
        holes.insert("__temp2".to_string(), "__hole_0".to_string());

        let line = render_common_graph_op(3, &op, &holes);
        assert_eq!(line, "[op_3] addEdge(from=__temp1, to=__hole_0, label=val)");
    }

    #[test]
    fn spec_name_to_js_method_name_replaces_hyphen() {
        assert_eq!(spec_name_to_js_method_name("aux-f"), "aux_f");
        assert_eq!(spec_name_to_js_method_name("1bad-name"), "_1bad_name");
    }

    #[test]
    fn default_method_param_names_prefers_arg_for_single_param() {
        assert_eq!(default_method_param_names(0), Vec::<String>::new());
        assert_eq!(default_method_param_names(1), vec!["arg".to_string()]);
        assert_eq!(
            default_method_param_names(2),
            vec!["arg0".to_string(), "arg1".to_string()]
        );
    }

    #[test]
    fn build_hole_call_template_uses_this_receiver() {
        let args = vec!["arg".to_string()];
        assert_eq!(
            build_hole_call_template("aux_f", &args),
            "this.aux_f(arg)".to_string()
        );
    }

    #[test]
    fn resolve_method_param_names_prefers_explicit_names() {
        let a = vec!["id".to_string(), "value".to_string()];
        let b = vec!["id".to_string(), "value".to_string()];
        assert_eq!(
            resolve_method_param_names(Some(&a), Some(&b), 2),
            vec!["id".to_string(), "value".to_string()]
        );
    }

    #[test]
    fn detect_unsupported_remove_operation_reports_error() {
        let operations = vec![vec![json!({
            "editType": "removeEdge",
            "from": "n1",
            "to": "n2",
            "label": "next"
        })]];
        assert_eq!(
            detect_unsupported_remove_operation(&operations),
            Some("Unsupported remove operation detected at call 0 op 0: removeEdge".to_string())
        );
    }

    #[test]
    fn build_composed_method_code_from_common_ops() {
        let ordered_common_ops = vec![
            (
                0usize,
                crate::list_env::GraphOperation {
                    edit_type: "addNode".to_string(),
                    id: Some("__temp1".to_string()),
                    label: Some(json!("Node")),
                    is_literal: Some(false),
                    node_type: None,
                    from: None,
                    to: None,
                    old_to: None,
                    new_to: None,
                    old_label: None,
                    new_label: None,
                },
            ),
            (
                1usize,
                crate::list_env::GraphOperation {
                    edit_type: "addEdge".to_string(),
                    id: None,
                    label: Some(json!("next")),
                    is_literal: None,
                    node_type: None,
                    from: Some("main-new1".to_string()),
                    to: Some("__temp1".to_string()),
                    old_to: None,
                    new_to: None,
                    old_label: None,
                    new_label: None,
                },
            ),
        ];
        let code = build_composed_method_code(
            "append",
            &vec!["arg".to_string()],
            &ordered_common_ops,
            &[],
            &HashMap::new(),
            Some("main-new1"),
            &HashMap::new(),
        )
        .unwrap();

        let expected = [
            "append(arg) {",
            "    const tmp0 = new Node();",
            "    this.next = tmp0;",
            "}",
        ]
        .join("\n");
        assert_eq!(code, expected);
    }

    #[test]
    fn build_composed_method_code_with_holes_exact() {
        let ordered_common_ops = vec![
            (
                0usize,
                crate::list_env::GraphOperation {
                    edit_type: "addNode".to_string(),
                    id: Some("__temp1".to_string()),
                    label: Some(json!("Node")),
                    is_literal: Some(false),
                    node_type: None,
                    from: None,
                    to: None,
                    old_to: None,
                    new_to: None,
                    old_label: None,
                    new_label: None,
                },
            ),
            (
                1usize,
                crate::list_env::GraphOperation {
                    edit_type: "addEdge".to_string(),
                    id: None,
                    label: Some(json!("val")),
                    is_literal: None,
                    node_type: None,
                    from: Some("__temp1".to_string()),
                    to: Some("__temp2".to_string()),
                    old_to: None,
                    new_to: None,
                    old_label: None,
                    new_label: None,
                },
            ),
            (
                2usize,
                crate::list_env::GraphOperation {
                    edit_type: "addEdge".to_string(),
                    id: None,
                    label: Some(json!("next")),
                    is_literal: None,
                    node_type: None,
                    from: Some("main-new1".to_string()),
                    to: Some("__temp1".to_string()),
                    old_to: None,
                    new_to: None,
                    old_label: None,
                    new_label: None,
                },
            ),
        ];

        let hole_bindings = vec![HoleBinding {
            hole_key: "__hole_0".to_string(),
            spec_name: "append-g".to_string(),
            return_type: "Int".to_string(),
            js_method_name: "append_g".to_string(),
            js_call_template: "this.append_g(first, second)".to_string(),
            side_a: "sideA".to_string(),
            side_b: "sideB".to_string(),
            anchor_a: Some(1),
            anchor_b: Some(2),
        }];

        let mut hole_by_object_id = HashMap::new();
        hole_by_object_id.insert("__temp2".to_string(), "__hole_0".to_string());

        let code = build_composed_method_code(
            "append",
            &vec!["first".to_string(), "second".to_string()],
            &ordered_common_ops,
            &hole_bindings,
            &hole_by_object_id,
            Some("main-new1"),
            &HashMap::new(),
        )
        .unwrap();

        let expected = [
            "append(first, second) {",
            "    const h_int_0 = this.append_g(first, second);",
            "    const tmp0 = new Node();",
            "    tmp0.val = h_int_0;",
            "    this.next = tmp0;",
            "}",
        ]
        .join("\n");
        assert_eq!(code, expected);
    }

    #[test]
    fn build_composed_method_code_drops_unused_hole_calls() {
        let ordered_common_ops = vec![(
            0usize,
            crate::list_env::GraphOperation {
                edit_type: "addNode".to_string(),
                id: Some("__temp1".to_string()),
                label: Some(json!("Node")),
                is_literal: Some(false),
                node_type: None,
                from: None,
                to: None,
                old_to: None,
                new_to: None,
                old_label: None,
                new_label: None,
            },
        )];

        let hole_bindings = vec![HoleBinding {
            hole_key: "__hole_0".to_string(),
            spec_name: "append-g".to_string(),
            return_type: "Int".to_string(),
            js_method_name: "append_g".to_string(),
            js_call_template: "this.append_g(arg)".to_string(),
            side_a: "sideA".to_string(),
            side_b: "sideB".to_string(),
            anchor_a: Some(0),
            anchor_b: Some(0),
        }];

        let code = build_composed_method_code(
            "append",
            &vec!["arg".to_string()],
            &ordered_common_ops,
            &hole_bindings,
            &HashMap::new(),
            Some("main-new1"),
            &HashMap::new(),
        )
        .unwrap();

        assert!(!code.contains("const h_int_0 = this.append_g(arg);"));
        assert!(code.contains("const tmp0 = new Node();"));
    }

    #[test]
    fn convert_json_to_unify_ops_normalizes_set_reference_ops() {
        use crate::unify_ops::{EdgeExpr, GraphOp, NodeExpr, VarOp};

        let operations = vec![
            json!({
                "editType": "addNode",
                "id": "__temp1",
                "label": "Node",
                "isLiteral": false
            }),
            json!({
                "editType": "editEdgeReference",
                "from": "__temp1",
                "oldTo": "null",
                "newTo": "null",
                "label": "next"
            }),
            json!({
                "editType": "addVariable",
                "to": "__temp1",
                "label": "return"
            }),
            json!({
                "editType": "editVariableReference",
                "oldTo": "__temp1",
                "newTo": "null",
                "label": "return"
            }),
        ];

        let ops = convert_json_to_unify_ops(&operations).expect("conversion should succeed");

        assert!(
            ops.iter()
                .any(|op| matches!(op.kind, GraphOp::Node(NodeExpr::NullNode))),
            "null should be normalized into NullNode"
        );

        let edge_op = ops
            .iter()
            .find(|op| op.id == "op_1")
            .expect("editEdgeReference op should exist");
        assert!(
            matches!(edge_op.kind, GraphOp::Edge(EdgeExpr::AddEdge { .. })),
            "editEdgeReference should normalize to AddEdge"
        );

        let add_var_op = ops
            .iter()
            .find(|op| op.id == "op_2")
            .expect("addVariable op should exist");
        assert!(
            matches!(
                add_var_op.kind,
                GraphOp::Variable(VarOp::AddVariable { .. })
            ),
            "addVariable should be represented as VarOp::AddVariable"
        );

        let edit_var_op = ops
            .iter()
            .find(|op| op.id == "op_3")
            .expect("editVariableReference op should exist");
        assert!(
            matches!(
                edit_var_op.kind,
                GraphOp::Variable(VarOp::AddVariable { .. })
            ),
            "editVariableReference should normalize to VarOp::AddVariable"
        );
    }

    #[test]
    fn build_composed_method_code_supports_edit_edge_and_return_variable() {
        let ordered_common_ops = vec![
            (
                0usize,
                crate::list_env::GraphOperation {
                    edit_type: "addNode".to_string(),
                    id: Some("__temp1".to_string()),
                    label: Some(json!("Node")),
                    is_literal: Some(false),
                    node_type: None,
                    from: None,
                    to: None,
                    old_to: None,
                    new_to: None,
                    old_label: None,
                    new_label: None,
                },
            ),
            (
                1usize,
                crate::list_env::GraphOperation {
                    edit_type: "addNode".to_string(),
                    id: Some("__temp2".to_string()),
                    label: Some(json!("3")),
                    is_literal: Some(true),
                    node_type: Some("string".to_string()),
                    from: None,
                    to: None,
                    old_to: None,
                    new_to: None,
                    old_label: None,
                    new_label: None,
                },
            ),
            (
                2usize,
                crate::list_env::GraphOperation {
                    edit_type: "editEdgeReference".to_string(),
                    id: None,
                    label: Some(json!("val")),
                    is_literal: None,
                    node_type: None,
                    from: Some("__temp1".to_string()),
                    to: None,
                    old_to: Some("null".to_string()),
                    new_to: Some("__temp2".to_string()),
                    old_label: None,
                    new_label: None,
                },
            ),
            (
                3usize,
                crate::list_env::GraphOperation {
                    edit_type: "addVariable".to_string(),
                    id: None,
                    label: Some(json!("return")),
                    is_literal: None,
                    node_type: None,
                    from: None,
                    to: Some("__temp1".to_string()),
                    old_to: None,
                    new_to: None,
                    old_label: None,
                    new_label: None,
                },
            ),
        ];

        let code = build_composed_method_code(
            "setVal",
            &vec!["arg".to_string()],
            &ordered_common_ops,
            &[],
            &HashMap::new(),
            Some("main-new1"),
            &HashMap::new(),
        )
        .unwrap();

        let expected = [
            "setVal(arg) {",
            "    const tmp0 = new Node();",
            "    tmp0.val = 3;",
            "    return tmp0;",
            "}",
        ]
        .join("\n");
        assert_eq!(code, expected);
    }

    #[test]
    fn resolve_effective_receiver_object_prefers_runtime_binding() {
        let vis_graph = VisGraph {
            nodes: vec![
                crate::models::Node {
                    id: "main-new2".to_string(),
                    is_literal: false,
                    label: json!("Node"),
                },
                crate::models::Node {
                    id: "__Variable-lst".to_string(),
                    is_literal: false,
                    label: json!("lst"),
                },
            ],
            edges: vec![crate::models::Edge {
                from: "__Variable-lst".to_string(),
                to: "main-new2".to_string(),
                label: "lst".to_string(),
            }],
        };
        let env = list_env::ListEnvironment::from_vis_graph(&vis_graph);
        let resolved = resolve_effective_receiver_object(Some("main-new1"), &env, &vis_graph);
        assert_eq!(resolved.as_deref(), Some("main-new2"));
    }

    #[test]
    fn build_composed_method_code_resolves_existing_kanon_ids_via_runtime_graph() {
        let target_id = "main-call2-FunctionExpression2-new1";
        let target_val_id = "main-call2-FunctionExpression2-new1-val";
        let vis_graph = VisGraph {
            nodes: vec![
                crate::models::Node {
                    id: "main-new2".to_string(),
                    is_literal: false,
                    label: json!("Node"),
                },
                crate::models::Node {
                    id: target_id.to_string(),
                    is_literal: false,
                    label: json!("Node"),
                },
                crate::models::Node {
                    id: target_val_id.to_string(),
                    is_literal: true,
                    label: json!("25"),
                },
                crate::models::Node {
                    id: "__Variable-lst".to_string(),
                    is_literal: false,
                    label: json!("lst"),
                },
            ],
            edges: vec![
                crate::models::Edge {
                    from: "main-new2".to_string(),
                    to: target_id.to_string(),
                    label: "next".to_string(),
                },
                crate::models::Edge {
                    from: target_id.to_string(),
                    to: target_val_id.to_string(),
                    label: "val".to_string(),
                },
                crate::models::Edge {
                    from: "__Variable-lst".to_string(),
                    to: "main-new2".to_string(),
                    label: "lst".to_string(),
                },
            ],
        };

        let runtime_map = build_runtime_object_expression_map(&vis_graph, Some("main-new2"));
        assert_eq!(
            runtime_map.get(target_id).map(String::as_str),
            Some("this.next")
        );
        assert_eq!(
            runtime_map.get(target_val_id).map(String::as_str),
            Some("this.next.val")
        );

        let ordered_common_ops = vec![
            (
                0usize,
                crate::list_env::GraphOperation {
                    edit_type: "addNode".to_string(),
                    id: Some("__temp1".to_string()),
                    label: Some(json!("10")),
                    is_literal: Some(true),
                    node_type: Some("string".to_string()),
                    from: None,
                    to: None,
                    old_to: None,
                    new_to: None,
                    old_label: None,
                    new_label: None,
                },
            ),
            (
                1usize,
                crate::list_env::GraphOperation {
                    edit_type: "editEdgeReference".to_string(),
                    id: None,
                    label: Some(json!("val")),
                    is_literal: None,
                    node_type: None,
                    from: Some(target_id.to_string()),
                    to: None,
                    old_to: Some(target_val_id.to_string()),
                    new_to: Some("__temp1".to_string()),
                    old_label: None,
                    new_label: None,
                },
            ),
        ];

        let code = build_composed_method_code(
            "set",
            &vec!["arg".to_string()],
            &ordered_common_ops,
            &[],
            &HashMap::new(),
            Some("main-new2"),
            &runtime_map,
        )
        .unwrap();

        let expected = ["set(arg) {", "    this.next.val = 10;", "}"].join("\n");
        assert_eq!(code, expected);
    }

    fn dummy_method_call(name: &str) -> MethodCallOperation {
        MethodCallOperation {
            call_label: "call1".to_string(),
            context_sensitive_id: "main".to_string(),
            receiver_object: "main-new1".to_string(),
            method_name: name.to_string(),
            arguments: vec![],
            argument_types: None,
            argument_names: None,
            method_param_names: None,
            operations: vec![],
            precond_graph: None,
            actual_graph: None,
            id_mapping: None,
            field_tables: None,
        }
    }

    #[test]
    fn collect_unique_method_names_single() {
        let calls = vec![dummy_method_call("append"), dummy_method_call("append")];
        assert_eq!(
            collect_unique_method_names(&calls),
            vec!["append".to_string()]
        );
    }

    #[test]
    fn collect_unique_method_names_multiple() {
        let calls = vec![
            dummy_method_call(" append "),
            dummy_method_call("push"),
            dummy_method_call("append"),
        ];
        assert_eq!(
            collect_unique_method_names(&calls),
            vec!["append".to_string(), "push".to_string()]
        );
    }

    fn example_json(input: serde_json::Value, output: serde_json::Value) -> ExampleJson {
        serde_json::from_value(json!({
            "input": input,
            "output": output
        }))
        .expect("example json should deserialize")
    }

    fn dummy_meta() -> EscherSpecMeta {
        EscherSpecMeta {
            arg_count: 2,
            arg_names: vec!["this".to_string(), "arg".to_string()],
            value_fields: vec!["val".to_string()],
            pointer_fields: vec!["next".to_string()],
            receiver_arg_index: Some(0),
        }
    }

    #[test]
    fn aggregate_multi_trace_specs_groups_full_coverage() {
        let meta = dummy_meta();
        let mut candidates = Vec::new();
        for trace_index in 0..3 {
            candidates.push(MultiTraceSpecCandidate {
                signature: "ret=Int|anchor=1|op=addNode|label=<none>|is_literal=true".to_string(),
                trace_index,
                original_spec_name: format!("append-trace{}-f", trace_index),
                meta: meta.clone(),
                spec: EscherSpec {
                    name: format!("append-trace{}-f", trace_index),
                    input_types: vec![
                        "Ptr".to_string(),
                        "Int".to_string(),
                        "List[Int]".to_string(),
                        "List[Ptr]".to_string(),
                    ],
                    return_type: "Int".to_string(),
                    examples: vec![example_json(
                        json!([0, trace_index as i32, [2], [null]]),
                        json!(trace_index as i32),
                    )],
                },
            });
        }

        let (specs, metas, renames) = aggregate_multi_trace_specs(&candidates, 3, "append");
        assert_eq!(specs.len(), 1);
        assert_eq!(specs[0].examples.len(), 3);
        assert_eq!(metas.len(), 1);
        assert_eq!(renames.len(), 3);
    }

    #[test]
    fn dedupe_equivalent_specs_merges_identical_specs() {
        let meta = dummy_meta();
        let spec_a = EscherSpec {
            name: "append-f".to_string(),
            input_types: vec![
                "Ptr".to_string(),
                "Int".to_string(),
                "List[Int]".to_string(),
                "List[Ptr]".to_string(),
            ],
            return_type: "Int".to_string(),
            examples: vec![example_json(json!([0, 25, [2], [null]]), json!(25))],
        };
        let spec_b = EscherSpec {
            name: "append-g".to_string(),
            input_types: spec_a.input_types.clone(),
            return_type: "Int".to_string(),
            examples: spec_a.examples.clone(),
        };
        let mut metas = HashMap::new();
        metas.insert(spec_a.name.clone(), meta.clone());
        metas.insert(spec_b.name.clone(), meta);

        let (specs, dedup_metas, renames) = dedupe_equivalent_specs(vec![spec_a, spec_b], metas);
        assert_eq!(specs.len(), 1);
        assert_eq!(dedup_metas.len(), 1);
        assert_eq!(renames.get("append-g"), Some(&"append-f".to_string()));
    }

    #[test]
    fn aggregate_multi_trace_specs_skips_conflicting_examples() {
        let meta = dummy_meta();
        let candidates = vec![
            MultiTraceSpecCandidate {
                signature: "ret=Int|anchor=1|op=addNode|label=<none>|is_literal=true".to_string(),
                trace_index: 0,
                original_spec_name: "append-trace0-f".to_string(),
                meta: meta.clone(),
                spec: EscherSpec {
                    name: "append-trace0-f".to_string(),
                    input_types: vec![
                        "Ptr".to_string(),
                        "Int".to_string(),
                        "List[Int]".to_string(),
                        "List[Ptr]".to_string(),
                    ],
                    return_type: "Int".to_string(),
                    examples: vec![example_json(json!([0, 1, [2], [null]]), json!(10))],
                },
            },
            MultiTraceSpecCandidate {
                signature: "ret=Int|anchor=1|op=addNode|label=<none>|is_literal=true".to_string(),
                trace_index: 1,
                original_spec_name: "append-trace1-f".to_string(),
                meta,
                spec: EscherSpec {
                    name: "append-trace1-f".to_string(),
                    input_types: vec![
                        "Ptr".to_string(),
                        "Int".to_string(),
                        "List[Int]".to_string(),
                        "List[Ptr]".to_string(),
                    ],
                    return_type: "Int".to_string(),
                    examples: vec![example_json(json!([0, 1, [2], [null]]), json!(20))],
                },
            },
        ];

        let (specs, metas, renames) = aggregate_multi_trace_specs(&candidates, 2, "append");
        assert!(specs.is_empty());
        assert!(metas.is_empty());
        assert!(renames.is_empty());
    }
}
