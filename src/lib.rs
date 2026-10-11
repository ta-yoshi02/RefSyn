pub mod alignment;
pub mod demonstration;
pub mod error;
pub mod escher_bridge;
pub mod models;
pub mod operation_graph;
pub mod pbe_problem;
mod pipeline;
#[cfg(all(feature = "server", not(target_arch = "wasm32")))]
pub mod server;
pub mod template;
mod validation;
pub use validation::{ValidationResult, ValidationStatus};

#[cfg(all(feature = "server", not(target_arch = "wasm32")))]
use crate::escher_bridge::{run_escher_js, write_spec_to_file, EscherJsInternalOutcome};
use crate::escher_bridge::{tasks_to_json, EscherJsOutcome};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
#[cfg(all(feature = "server", not(target_arch = "wasm32")))]
use std::sync::atomic::AtomicU64;
use std::sync::atomic::{AtomicBool, Ordering};
#[cfg(all(feature = "server", not(target_arch = "wasm32")))]
use warp::http::StatusCode;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct MethodCallOperation {
    #[serde(rename = "callLabel")]
    pub call_label: String,
    #[serde(rename = "contextSensitiveID")]
    pub context_sensitive_id: String,
    #[serde(rename = "receiverObject")]
    pub receiver_object: String,
    #[serde(rename = "receiverClassName")]
    pub receiver_class_name: Option<String>,
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

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct SynthesisRequest {
    pub method_calls: Vec<MethodCallOperation>,
    pub vis_graph: VisGraph,
    /// Kanon's typed pre-call states and class instances; see `demonstration`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub validation: Option<serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FieldTables {
    pub value: Vec<String>,
    pub pointer: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SynthesisResponse {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub validation: Option<ValidationResult>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hole_checks: Option<Vec<pbe_problem::HoleCheck>>,
    pub common_pattern: Option<String>,
    pub hole_information: Option<HashMap<String, Vec<String>>>,
    pub code: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub composed_method_code: Option<String>,
    pub individual_codes: Vec<String>,
    /// Display-only summary of the run, including why synthesis stopped.
    pub list_environment_info: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub escher_results: Option<Vec<EscherJsOutcome>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SynthesisCoreOptions {
    #[serde(default)]
    pub trace: bool,
}

#[derive(Debug, Serialize)]
pub struct SynthesisArtifacts {
    pub response: SynthesisResponse,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub task_json: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub warnings: Vec<String>,
    #[serde(skip_serializing)]
    spec_base_name: Option<String>,
    #[cfg(all(feature = "server", not(target_arch = "wasm32")))]
    #[serde(skip_serializing)]
    http_status: u16,
}

impl SynthesisArtifacts {
    fn from_response(response: SynthesisResponse) -> Self {
        Self::from_response_with_status(response, 200)
    }

    fn from_response_with_status(response: SynthesisResponse, _http_status: u16) -> Self {
        Self {
            response,
            task_json: None,
            warnings: Vec::new(),
            spec_base_name: None,
            #[cfg(all(feature = "server", not(target_arch = "wasm32")))]
            http_status: _http_status,
        }
    }
}

static TRACE_ENABLED: AtomicBool = AtomicBool::new(false);
#[cfg(all(feature = "server", not(target_arch = "wasm32")))]
static TASK_JSON_ARTIFACT_SEQUENCE: AtomicU64 = AtomicU64::new(0);

fn set_trace_enabled(enabled: bool) {
    TRACE_ENABLED.store(enabled, Ordering::Relaxed);
}

fn trace_enabled() -> bool {
    TRACE_ENABLED.load(Ordering::Relaxed)
}

#[cfg(all(feature = "server", not(target_arch = "wasm32")))]
fn sanitize_artifact_stem(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    for ch in raw.chars() {
        if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_') {
            out.push(ch);
        } else {
            out.push('_');
        }
    }
    if out.is_empty() {
        "refsyn-synthesis".to_string()
    } else {
        out
    }
}

#[cfg(all(feature = "server", not(target_arch = "wasm32")))]
fn next_task_json_output_path(base_name: &str) -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    let seq = TASK_JSON_ARTIFACT_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    format!(
        "target/escher/ts/{}_{}_{:09}_p{}_{}.json",
        sanitize_artifact_stem(base_name),
        now.as_secs(),
        now.subsec_nanos(),
        std::process::id(),
        seq
    )
}

// Types needed by the server module will be imported from main directly

fn blank_synthesis_response(list_env_info: Option<String>) -> SynthesisResponse {
    SynthesisResponse {
        validation: None,
        hole_checks: None,
        common_pattern: None,
        hole_information: None,
        code: vec![],
        composed_method_code: None,
        individual_codes: vec![],
        list_environment_info: list_env_info,
        escher_results: None,
    }
}

#[cfg(all(feature = "server", not(target_arch = "wasm32")))]
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
            let response = blank_synthesis_response(None);
            return Ok(warp::reply::with_status(
                warp::reply::json(&response),
                StatusCode::BAD_REQUEST,
            ));
        }
    };

    let trace = match std::env::var("REFSYN_TRACE") {
        Ok(v) => matches!(v.as_str(), "1" | "true" | "yes" | "on"),
        Err(_) => false,
    };
    let mut artifacts = match synthesize_core(req, SynthesisCoreOptions { trace }) {
        Ok(artifacts) => artifacts,
        Err(err) => {
            eprintln!("Synthesis failed: {}", err);
            let response =
                blank_synthesis_response(Some(format!("internal synthesis error: {}", err)));
            return Ok(warp::reply::with_status(
                warp::reply::json(&response),
                StatusCode::INTERNAL_SERVER_ERROR,
            ));
        }
    };
    finalize_native_synthesis(&mut artifacts);
    validation::validate_for_adoption(&body, &mut artifacts);
    log_synthesis_response(&artifacts.response);

    Ok(warp::reply::with_status(
        warp::reply::json(&artifacts.response),
        StatusCode::from_u16(artifacts.http_status).unwrap_or(StatusCode::OK),
    ))
}

pub fn synthesize_core(
    req: SynthesisRequest,
    options: SynthesisCoreOptions,
) -> anyhow::Result<SynthesisArtifacts> {
    set_trace_enabled(options.trace);

    if req.method_calls.is_empty() {
        println!("No method calls provided in the request.");
        return Ok(SynthesisArtifacts::from_response(blank_synthesis_response(
            None,
        )));
    }

    let plan = match pipeline::plan(&req) {
        Ok(plan) => plan,
        Err(err) => {
            let message = format!("{err:#}");
            eprintln!("{}", message);
            return Ok(SynthesisArtifacts::from_response_with_status(
                blank_synthesis_response(Some(message)),
                400,
            ));
        }
    };
    let task_json = if plan.tasks.is_empty() {
        None
    } else {
        let json_text = tasks_to_json(&plan.tasks)?;
        if trace_enabled() {
            println!("TRACE: Escher TS task JSON:\n{}", json_text);
        }
        Some(json_text)
    };
    let mut response = blank_synthesis_response(None);
    response.common_pattern = Some(plan.template_text);
    response.hole_information = Some(plan.hole_information);
    response.composed_method_code = Some(plan.composed_method_code);
    response.hole_checks = Some(plan.checks);
    let mut artifacts = SynthesisArtifacts::from_response(response);
    artifacts.task_json = task_json;
    artifacts.spec_base_name = Some(plan.method);
    Ok(artifacts)
}

#[cfg(all(feature = "server", not(target_arch = "wasm32")))]
fn append_list_env_info(list_env_info: &mut Option<String>, message: impl AsRef<str>) {
    let message = message.as_ref();
    match list_env_info {
        Some(existing) if !existing.is_empty() => {
            existing.push('\n');
            existing.push_str(message);
        }
        Some(existing) => existing.push_str(message),
        None => *list_env_info = Some(message.to_string()),
    }
}

#[cfg(all(feature = "server", not(target_arch = "wasm32")))]
fn log_synthesis_response(response: &SynthesisResponse) {
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
}

#[cfg(all(feature = "server", not(target_arch = "wasm32")))]
fn apply_escher_results(artifacts: &mut SynthesisArtifacts, results: &[EscherJsInternalOutcome]) {
    for out in results {
        if let Some(js) = out
            .compiled_js
            .as_ref()
            .filter(|js| out.success && !js.is_empty())
        {
            artifacts.response.code.push(js.clone());
            artifacts
                .response
                .individual_codes
                .push(format!("{}: {}", out.name, js));
        } else if let Some(err) = &out.error {
            artifacts
                .response
                .individual_codes
                .push(format!("{}: ERROR {}", out.name, err));
        } else if let Some(rendered) = &out.rendered {
            artifacts.response.individual_codes.push(format!(
                "{}: ERROR compiled_js missing for rendered term {}",
                out.name, rendered
            ));
        } else {
            artifacts
                .response
                .individual_codes
                .push(format!("{}: no output", out.name));
        }
    }

    let existing = artifacts.response.escher_results.take().unwrap_or_default();
    let mut merged = existing;
    merged.extend(results.iter().map(EscherJsOutcome::from));
    artifacts.response.escher_results = Some(merged);
}

#[cfg(all(feature = "server", not(target_arch = "wasm32")))]
fn finalize_native_synthesis(artifacts: &mut SynthesisArtifacts) {
    for warning in artifacts.warnings.clone() {
        append_list_env_info(&mut artifacts.response.list_environment_info, warning);
    }

    let Some(task_json) = artifacts.task_json.as_ref() else {
        return;
    };

    let base_name = artifacts
        .spec_base_name
        .as_deref()
        .unwrap_or("refsyn-synthesis");
    let out_path = next_task_json_output_path(base_name);
    match write_spec_to_file(&out_path, task_json) {
        Ok(()) => append_list_env_info(
            &mut artifacts.response.list_environment_info,
            format!("escher-ts task JSON saved: {}", out_path),
        ),
        Err(err) => eprintln!("Failed to write escher-ts task JSON: {}", err),
    }

    match run_escher_js(task_json) {
        Ok(results) => {
            let success_count = results.iter().filter(|result| result.success).count();
            let failure_count = results.len().saturating_sub(success_count);
            println!(
                "escher-ts synthesis completed ({} success / {} failure)",
                success_count, failure_count
            );
            apply_escher_results(artifacts, &results);
        }
        Err(err) => eprintln!("Escher JS synthesis failed: {}", err),
    }
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn synthesize_browser(
    request_json: &str,
    options_json: Option<String>,
) -> Result<String, JsValue> {
    let request: SynthesisRequest = serde_json::from_str(request_json)
        .map_err(|err| JsValue::from_str(&format!("invalid request json: {}", err)))?;
    let options = match options_json.as_deref() {
        Some(raw) => serde_json::from_str(raw)
            .map_err(|err| JsValue::from_str(&format!("invalid options json: {}", err)))?,
        None => SynthesisCoreOptions::default(),
    };
    let artifacts = synthesize_core(request, options)
        .map_err(|err| JsValue::from_str(&format!("browser synthesis failed: {}", err)))?;
    serde_json::to_string(&artifacts)
        .map_err(|err| JsValue::from_str(&format!("failed to serialize artifacts: {}", err)))
}
