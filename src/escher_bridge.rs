//! Escher bridge: build intermediate specs and native escher-ts tasks from Kanon environments
//!
//! This module converts ListEnvironment snapshots (built from Kanon VisGraph + operations)
//! into either:
//! - intermediate grouped specs
//! - native escher-ts task JSON
//!
//! It supports:
//! - Dynamic field detection (value vs pointer) without hardcoding names
//! - Local indexing per test case via BFS from the detected root variable
//! - nullPtr is encoded as JSON null; missing Int values remain -1
//! - Deterministic ordering of inputs (args, then value lists, then pointer lists)

use crate::list_env::{FieldKind, ListEnvironment, PtrValue};
use crate::models::VisGraph;
use crate::FieldTables;
use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};
#[cfg(not(target_arch = "wasm32"))]
use std::io::Write;
#[cfg(not(target_arch = "wasm32"))]
use std::path::PathBuf;
#[cfg(not(target_arch = "wasm32"))]
use std::process::{Command, Stdio};

/// One test case: environment + args + expected output
#[derive(Debug, Clone)]
pub struct EscherCase {
    pub env: ListEnvironment,
    pub vis_graph: VisGraph,
    pub arguments: Vec<Value>,
    pub arg_names: Vec<String>,
    pub arg_types: Option<Vec<String>>,
    pub receiver_arg_index: Option<usize>,
    pub output: Value,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ExampleJson {
    pub input: Vec<Value>,
    pub output: Value,
}

#[derive(Serialize, Debug, Clone)]
pub struct EscherSpec {
    pub name: String,
    #[serde(rename = "inputTypes")]
    pub input_types: Vec<String>,
    #[serde(rename = "returnType")]
    pub return_type: String,
    pub examples: Vec<ExampleJson>,
}

#[derive(Debug, Clone)]
pub struct EscherSpecMeta {
    pub arg_count: usize,
    pub arg_names: Vec<String>,
    pub value_fields: Vec<String>,
    pub pointer_fields: Vec<String>,
    pub receiver_arg_index: Option<usize>,
}

#[derive(Serialize, Debug, Clone)]
pub struct EscherTaskSpec {
    pub name: String,
    pub category: String,
    pub classes: Vec<EscherTaskClassSpec>,
    #[serde(rename = "exposeClassComponents")]
    pub expose_class_components: bool,
    #[serde(rename = "autoClassFieldComponents")]
    pub auto_class_field_components: bool,
    pub signature: EscherTaskSignature,
    pub components: Vec<EscherTaskComponentSpec>,
    pub examples: Vec<(Vec<Value>, Value)>,
    #[serde(rename = "refsynMeta")]
    pub refsyn_meta: EscherTaskMeta,
}

#[derive(Serialize, Debug, Clone)]
pub struct EscherTaskClassSpec {
    pub name: String,
    pub fields: BTreeMap<String, String>,
}

#[derive(Serialize, Debug, Clone)]
pub struct EscherTaskSignature {
    #[serde(rename = "returnType")]
    pub return_type: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub args: Vec<EscherTaskSignatureArg>,
    #[serde(rename = "autoExpandClassSignature")]
    pub auto_expand_class_signature: EscherTaskAutoExpandSignature,
}

#[derive(Serialize, Debug, Clone)]
pub struct EscherTaskSignatureArg {
    pub name: String,
    #[serde(rename = "type")]
    pub arg_type: String,
}

#[derive(Serialize, Debug, Clone)]
pub struct EscherTaskAutoExpandSignature {
    #[serde(rename = "className")]
    pub class_name: String,
    #[serde(rename = "thisRefName")]
    pub this_ref_name: String,
    #[serde(rename = "classHeapName")]
    pub class_heap_name: String,
    #[serde(rename = "fieldHeapNames")]
    pub field_heap_names: BTreeMap<String, String>,
}

#[derive(Serialize, Debug, Clone)]
pub struct EscherTaskComponentSpec {
    pub name: String,
    pub kind: String,
    #[serde(skip_serializing_if = "Option::is_none", rename = "ref")]
    pub ref_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "inputTypes")]
    pub input_types: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "returnType")]
    pub return_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub args: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "bodyJs")]
    pub body_js: Option<String>,
}

#[derive(Serialize, Debug, Clone)]
pub struct EscherTaskMeta {
    #[serde(rename = "jsMethodName")]
    pub js_method_name: String,
    #[serde(rename = "className")]
    pub class_name: String,
    #[serde(rename = "thisRefName")]
    pub this_ref_name: String,
    #[serde(rename = "classHeapName")]
    pub class_heap_name: String,
    #[serde(rename = "valueFields")]
    pub value_fields: Vec<String>,
    #[serde(rename = "pointerFields")]
    pub pointer_fields: Vec<String>,
    #[serde(rename = "fieldHeapNames")]
    pub field_heap_names: BTreeMap<String, String>,
    #[serde(rename = "explicitArgs")]
    pub explicit_args: Vec<EscherTaskMetaArg>,
}

#[derive(Serialize, Debug, Clone)]
pub struct EscherTaskMetaArg {
    pub name: String,
    #[serde(rename = "legacyType")]
    pub legacy_type: String,
    #[serde(rename = "taskType")]
    pub task_type: String,
}

/// Build an intermediate grouped spec from cases.
/// - `name`: synthesized function name
/// - `return_type`: Escher type string (e.g., "Int", "List[Int]")
pub fn build_escher_spec(
    name: &str,
    return_type: &str,
    cases: &[EscherCase],
    field_tables: Option<&FieldTables>,
) -> Result<EscherSpec> {
    if cases.is_empty() {
        return Err(anyhow!("no cases provided"));
    }
    let (sorted_value_fields, sorted_pointer_fields) = resolve_field_order(cases, field_tables)?;

    // inputTypes: args first (from arg_types), then one List[Int] per value field, then per pointer field
    let mut input_types: Vec<String> = Vec::new();
    let arg_count = cases[0].arguments.len();
    let arg_types = resolve_arg_types(cases, arg_count)?;
    input_types.extend(arg_types);
    for _ in &sorted_value_fields {
        input_types.push("List[Int]".to_string());
    }
    for _ in &sorted_pointer_fields {
        // Pointer fields are represented as List[Ptr] in the intermediate spec.
        input_types.push("List[Ptr]".to_string());
    }

    // Examples
    let mut examples: Vec<ExampleJson> = Vec::new();
    for case in cases {
        // Build BFS-local index mapping using the union of pointer fields
        let bfs = build_bfs_order(&case.env, &case.vis_graph, &sorted_pointer_fields)?;

        // Compose input: args + value lists + pointer index lists
        let mut input: Vec<Value> = case.arguments.clone();

        // Value lists
        for vf in &sorted_value_fields {
            let list = build_value_list(&case.env, &bfs, vf);
            input.push(json!(list));
        }

        // Pointer lists
        for pf in &sorted_pointer_fields {
            let list = build_pointer_index_list(&case.env, &bfs, pf);
            input.push(json!(list));
        }

        examples.push(ExampleJson {
            input,
            output: case.output.clone(),
        });
    }

    Ok(EscherSpec {
        name: name.to_string(),
        input_types,
        return_type: return_type.to_string(),
        examples,
    })
}

pub fn derive_spec_meta(cases: &[EscherCase]) -> Result<EscherSpecMeta> {
    derive_spec_meta_with_fields(cases, None)
}

pub fn derive_spec_meta_with_fields(
    cases: &[EscherCase],
    field_tables: Option<&FieldTables>,
) -> Result<EscherSpecMeta> {
    if cases.is_empty() {
        return Err(anyhow!("no cases provided"));
    }
    let arg_count = cases[0].arguments.len();
    resolve_arg_types(cases, arg_count)?;
    let arg_names = resolve_arg_names(cases, arg_count)?;
    let (value_fields, pointer_fields) = resolve_field_order(cases, field_tables)?;
    let receiver_arg_index = resolve_receiver_arg_index(cases, arg_count)?;
    Ok(EscherSpecMeta {
        arg_count,
        arg_names,
        value_fields,
        pointer_fields,
        receiver_arg_index,
    })
}

pub(crate) fn resolve_field_order(
    cases: &[EscherCase],
    field_tables: Option<&FieldTables>,
) -> Result<(Vec<String>, Vec<String>)> {
    if let Some(tables) = field_tables {
        let value_fields = dedupe_preserve_order(&tables.value);
        let pointer_fields = dedupe_preserve_order(&tables.pointer);
        return Ok((value_fields, pointer_fields));
    }
    classify_fields(cases)
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

fn resolve_arg_types(cases: &[EscherCase], arg_count: usize) -> Result<Vec<String>> {
    let mut resolved: Option<Vec<String>> = None;
    for case in cases {
        if case.arguments.len() != arg_count {
            return Err(anyhow!("inconsistent argument counts across cases"));
        }
        if let Some(types) = &case.arg_types {
            if types.len() != arg_count {
                return Err(anyhow!(
                    "argument type count {} does not match argument count {}",
                    types.len(),
                    arg_count
                ));
            }
            if let Some(existing) = &resolved {
                if existing != types {
                    return Err(anyhow!("inconsistent argument types across cases"));
                }
            } else {
                resolved = Some(types.clone());
            }
        }
    }
    Ok(resolved.unwrap_or_else(|| vec!["Int".to_string(); arg_count]))
}

fn resolve_arg_names(cases: &[EscherCase], arg_count: usize) -> Result<Vec<String>> {
    let mut resolved: Option<Vec<String>> = None;
    for case in cases {
        if !case.arg_names.is_empty() {
            if case.arg_names.len() != arg_count {
                return Err(anyhow!(
                    "argument name count {} does not match argument count {}",
                    case.arg_names.len(),
                    arg_count
                ));
            }
            if let Some(existing) = &resolved {
                if existing != &case.arg_names {
                    return Err(anyhow!(
                        "inconsistent argument names across cases: {:?} vs {:?}",
                        existing,
                        case.arg_names
                    ));
                }
            } else {
                resolved = Some(case.arg_names.clone());
            }
        }
    }
    Ok(resolved.unwrap_or_else(|| (0..arg_count).map(|i| format!("arg{}", i)).collect()))
}

fn resolve_receiver_arg_index(cases: &[EscherCase], arg_count: usize) -> Result<Option<usize>> {
    let mut resolved: Option<usize> = None;
    for case in cases {
        if let Some(idx) = case.receiver_arg_index {
            if idx >= arg_count {
                return Err(anyhow!(
                    "receiver arg index {} out of bounds for {} args",
                    idx,
                    arg_count
                ));
            }
            if let Some(existing) = resolved {
                if existing != idx {
                    return Err(anyhow!(
                        "inconsistent receiver arg index across cases: {} vs {}",
                        existing,
                        idx
                    ));
                }
            } else {
                resolved = Some(idx);
            }
        }
    }
    Ok(resolved)
}

fn classify_fields(cases: &[EscherCase]) -> Result<(Vec<String>, Vec<String>)> {
    if cases.is_empty() {
        return Err(anyhow!("no cases provided"));
    }
    let mut value_field_set: HashSet<String> = HashSet::new();
    let mut pointer_field_set: HashSet<String> = HashSet::new();

    for case in cases {
        if !case.env.field_kinds.is_empty() {
            let mut variable_fields: HashSet<String> = HashSet::new();
            let var_prefix = "__Variable-";
            for node in &case.vis_graph.nodes {
                if !node.is_literal && node.id.starts_with(var_prefix) {
                    variable_fields.insert(node.id.trim_start_matches(var_prefix).to_string());
                }
            }
            for (field, kind) in &case.env.field_kinds {
                if variable_fields.contains(field) {
                    continue;
                }
                match kind {
                    FieldKind::Pointer => {
                        pointer_field_set.insert(field.clone());
                    }
                    FieldKind::Value => {
                        value_field_set.insert(field.clone());
                    }
                }
            }
            continue;
        }

        let (graph_value_fields, graph_pointer_fields) =
            classify_fields_from_graph(&case.vis_graph);
        for (field, values) in &case.env.field_lists {
            let mut has_pointer = false;
            let mut has_value = false;
            for v in values {
                if let Some(ptr) = PtrValue::from_value(v) {
                    match ptr {
                        PtrValue::Null => {}
                        PtrValue::Index(idx) => {
                            if case.env.index_to_obj_id.contains_key(&idx) {
                                has_pointer = true;
                            } else {
                                has_value = true;
                            }
                        }
                    }
                } else if !v.is_null() {
                    has_value = true;
                }
            }
            if !has_pointer && !has_value {
                if graph_pointer_fields.contains(field) {
                    has_pointer = true;
                } else if graph_value_fields.contains(field) {
                    has_value = true;
                }
            }
            if has_pointer {
                pointer_field_set.insert(field.clone());
            }
            if has_value {
                value_field_set.insert(field.clone());
            }
        }
    }

    let mut sorted_value_fields: Vec<String> = value_field_set.into_iter().collect();
    sorted_value_fields.sort();
    let mut sorted_pointer_fields: Vec<String> = pointer_field_set.into_iter().collect();
    sorted_pointer_fields.sort();
    Ok((sorted_value_fields, sorted_pointer_fields))
}

fn classify_fields_from_graph(vis_graph: &VisGraph) -> (HashSet<String>, HashSet<String>) {
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
    (value_fields, pointer_fields)
}

// ---------- internals ----------

#[derive(Debug, Clone)]
struct BfsOrder {
    // BFS order of object indices (original env indices)
    order_indices: Vec<usize>,
    // Map from original env index -> bfs index
    idx_to_bfs: HashMap<usize, usize>,
}

/// Build BFS order from the detected root variable across pointer fields only.
fn build_bfs_order(
    env: &ListEnvironment,
    vis_graph: &VisGraph,
    pointer_fields: &[String],
) -> Result<BfsOrder> {
    // 1) detect root from variable nodes
    let (root_env_idx, _) = detect_root(env, vis_graph)?;

    // 2) build adjacency across pointer fields using env.field_lists
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

    // 3) BFS
    let mut visited: HashSet<usize> = HashSet::new();
    let mut q: VecDeque<usize> = VecDeque::new();
    let mut order: Vec<usize> = Vec::new();
    q.push_back(root_env_idx);
    while let Some(u) = q.pop_front() {
        if !visited.insert(u) {
            continue;
        }
        order.push(u);
        if let Some(neis) = adj.get(&u) {
            for &v in neis {
                if !visited.contains(&v) {
                    q.push_back(v);
                }
            }
        }
    }
    // map indices for visited nodes only
    let mut idx_to_bfs = HashMap::new();
    for (bi, &orig) in order.iter().enumerate() {
        idx_to_bfs.insert(orig, bi);
    }
    Ok(BfsOrder {
        order_indices: order,
        idx_to_bfs,
    })
}

/// Detect root object index from variable bindings in env/graph.
/// Heuristic:
/// - prefer "__Variable-this" if present
/// - otherwise find nodes with id prefix "__Variable-<name>"
/// - for the same label <name> as a field, if field_lists[<name>][var_idx] points to an object index >= 0, use it as root
fn detect_root(env: &ListEnvironment, vis_graph: &VisGraph) -> Result<(usize, String)> {
    let var_prefix = "__Variable-";
    // reverse map from env index to obj id
    let mut idx_to_id: HashMap<usize, &str> = HashMap::new();
    for (id, idx) in &env.obj_id_to_index {
        idx_to_id.insert(*idx, id.as_str());
    }

    // collect variable nodes present in env
    let mut var_ids: Vec<&str> = vis_graph
        .nodes
        .iter()
        .filter(|n| !n.is_literal)
        .map(|n| n.id.as_str())
        .filter(|id| id.starts_with(var_prefix))
        .collect();
    var_ids.sort();

    let mut ordered_var_ids: Vec<&str> = Vec::new();
    if var_ids.contains(&"__Variable-this") {
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
                        return Ok((root_idx, var_name));
                    }
                }
            }
        }
    }
    Err(anyhow!("failed to detect root from variable nodes"))
}

fn build_value_list(env: &ListEnvironment, bfs: &BfsOrder, field: &str) -> Vec<i32> {
    let mut result = Vec::new();
    let default = json!(-1);
    let vec_ref = env.field_lists.get(field);
    for &orig_idx in &bfs.order_indices {
        let v = match vec_ref {
            Some(vs) if orig_idx < vs.len() => &vs[orig_idx],
            _ => &default,
        };
        let as_int = match v {
            Value::Number(n) => n.as_i64().unwrap_or(-1) as i32,
            Value::String(s) => s.parse::<i64>().unwrap_or(-1) as i32,
            _ => -1,
        };
        result.push(as_int);
    }
    result
}

fn build_pointer_index_list(env: &ListEnvironment, bfs: &BfsOrder, field: &str) -> Vec<Value> {
    let mut result = Vec::new();
    let default = Value::Null;
    let vec_ref = env.field_lists.get(field);
    for &orig_idx in &bfs.order_indices {
        let v = match vec_ref {
            Some(vs) if orig_idx < vs.len() => &vs[orig_idx],
            _ => &default,
        };
        // env stores pointer as original index (0..N-1) or null
        let ptr = PtrValue::from_value(v).unwrap_or(PtrValue::Null);
        match ptr {
            PtrValue::Null => result.push(Value::Null),
            PtrValue::Index(to_orig) => {
                let mapped = bfs.idx_to_bfs.get(&to_orig).copied();
                match mapped {
                    Some(i) => result.push(json!(i as i32)),
                    None => result.push(Value::Null),
                }
            }
        }
    }
    result
}

pub fn tasks_to_json(tasks: &[EscherTaskSpec]) -> Result<String> {
    Ok(serde_json::to_string_pretty(tasks)?)
}

/// Convenience: write the produced JSON string to a file path
/// (for example `target/escher/ts/tests.json`).
#[cfg(not(target_arch = "wasm32"))]
pub fn write_spec_to_file(path: &str, content: &str) -> Result<()> {
    if let Some(parent) = std::path::Path::new(path).parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)?;
        }
    }
    std::fs::write(path, content)?;
    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EscherJsOutcome {
    pub name: String,
    pub success: bool,
    pub rendered: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct EscherJsInternalOutcome {
    pub name: String,
    pub success: bool,
    pub rendered: Option<String>,
    pub error: Option<String>,
    #[serde(default)]
    pub compiled_js: Option<String>,
}

#[cfg(not(target_arch = "wasm32"))]
fn parse_escher_js_max_old_space_mb(raw: Option<&str>) -> Result<usize> {
    const DEFAULT_MB: usize = 8192;
    const MIN_MB: usize = 256;

    let Some(raw) = raw else {
        return Ok(DEFAULT_MB);
    };

    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Ok(DEFAULT_MB);
    }

    let parsed = trimmed.parse::<usize>().map_err(|_| {
        anyhow!(
            "Invalid ESCHER_JS_MAX_OLD_SPACE_MB='{}'. Expected a positive integer (MB).",
            trimmed
        )
    })?;
    if parsed < MIN_MB {
        return Err(anyhow!(
            "ESCHER_JS_MAX_OLD_SPACE_MB must be >= {} (got {}).",
            MIN_MB,
            parsed
        ));
    }
    Ok(parsed)
}

#[cfg(not(target_arch = "wasm32"))]
fn resolve_escher_js_max_old_space_mb() -> Result<usize> {
    parse_escher_js_max_old_space_mb(std::env::var("ESCHER_JS_MAX_OLD_SPACE_MB").ok().as_deref())
}

/// Invoke the configured escher-ts runner via Node and parse normalized results.
#[cfg(not(target_arch = "wasm32"))]
pub fn run_escher_js(task_json: &str) -> Result<Vec<EscherJsInternalOutcome>> {
    let default_runner = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("scripts")
        .join("run_escher.js");
    let runner = std::env::var("ESCHER_JS_RUNNER")
        .map(PathBuf::from)
        .unwrap_or(default_runner);

    if !runner.exists() {
        return Err(anyhow!(
            "Escher JS runner not found at {}",
            runner.display()
        ));
    }

    let max_old_space_mb = resolve_escher_js_max_old_space_mb()?;
    let mut command = Command::new("node");
    command
        .arg(format!("--max-old-space-size={}", max_old_space_mb))
        .arg(&runner)
        .arg("--quiet")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit());

    let mut child = command
        .spawn()
        .map_err(|e| anyhow!("Failed to spawn Node runner: {}", e))?;

    {
        let stdin = child
            .stdin
            .as_mut()
            .ok_or_else(|| anyhow!("Failed to open stdin for Node runner"))?;
        stdin.write_all(task_json.as_bytes())?;
    }

    let output = child.wait_with_output()?;
    if !output.status.success() {
        return Err(anyhow!(
            "Escher JS runner exited with status {} (node max-old-space-size={} MB; override via ESCHER_JS_MAX_OLD_SPACE_MB)",
            output.status,
            max_old_space_mb
        ));
    }

    let stdout = String::from_utf8(output.stdout)?;
    let trimmed = stdout.trim();
    if trimmed.is_empty() {
        return Err(anyhow!("Escher JS runner returned empty output"));
    }

    let outcomes: Vec<EscherJsInternalOutcome> = serde_json::from_str(trimmed)?;
    Ok(outcomes)
}

#[cfg(not(target_arch = "wasm32"))]
pub fn run_escher_js_from_tasks(tasks: &[EscherTaskSpec]) -> Result<Vec<EscherJsInternalOutcome>> {
    let json_text = tasks_to_json(tasks)?;
    run_escher_js(&json_text)
}

impl From<&EscherJsInternalOutcome> for EscherJsOutcome {
    fn from(value: &EscherJsInternalOutcome) -> Self {
        Self {
            name: value.name.clone(),
            success: value.success,
            rendered: value.rendered.clone(),
            error: value.error.clone(),
        }
    }
}

pub fn build_escher_task_spec(spec: &EscherSpec, meta: &EscherSpecMeta) -> Result<EscherTaskSpec> {
    let class_name = "Node".to_string();
    let this_ref_name = "thisRef".to_string();
    let class_heap_name = "nodeHeap".to_string();

    let mut field_heap_names = BTreeMap::new();
    for field in meta.value_fields.iter().chain(meta.pointer_fields.iter()) {
        field_heap_names.insert(field.clone(), field_to_heap_name(field));
    }

    let mut fields = BTreeMap::new();
    for field in &meta.value_fields {
        fields.insert(field.clone(), "Ref[Int]".to_string());
    }
    for field in &meta.pointer_fields {
        fields.insert(field.clone(), format!("Ref[{}]", class_name));
    }

    let explicit_args = build_task_explicit_args(spec, meta, &class_name)?;
    let signature = EscherTaskSignature {
        return_type: legacy_type_to_task_type(&spec.return_type, &class_name)?,
        args: explicit_args
            .iter()
            .map(|arg| EscherTaskSignatureArg {
                name: arg.name.clone(),
                arg_type: arg.task_type.clone(),
            })
            .collect(),
        auto_expand_class_signature: EscherTaskAutoExpandSignature {
            class_name: class_name.clone(),
            this_ref_name: this_ref_name.clone(),
            class_heap_name: class_heap_name.clone(),
            field_heap_names: field_heap_names.clone(),
        },
    };

    let examples = spec
        .examples
        .iter()
        .map(|example| build_task_example(spec, meta, example, &class_name))
        .collect::<Result<Vec<_>>>()?;

    Ok(EscherTaskSpec {
        name: spec.name.clone(),
        category: "refsyn".to_string(),
        classes: vec![EscherTaskClassSpec {
            name: class_name.clone(),
            fields,
        }],
        expose_class_components: false,
        auto_class_field_components: true,
        signature,
        components: default_task_components(spec, meta),
        examples,
        refsyn_meta: EscherTaskMeta {
            js_method_name: sanitize_js_identifier(&spec.name),
            class_name,
            this_ref_name,
            class_heap_name,
            value_fields: meta.value_fields.clone(),
            pointer_fields: meta.pointer_fields.clone(),
            field_heap_names,
            explicit_args,
        },
    })
}

fn build_task_explicit_args(
    spec: &EscherSpec,
    meta: &EscherSpecMeta,
    class_name: &str,
) -> Result<Vec<EscherTaskMetaArg>> {
    let mut explicit_args = Vec::new();
    for idx in 0..meta.arg_count {
        if meta.receiver_arg_index == Some(idx) {
            continue;
        }
        let legacy_type = spec
            .input_types
            .get(idx)
            .cloned()
            .unwrap_or_else(|| "Int".to_string());
        let task_type = legacy_type_to_task_type(&legacy_type, class_name)?;
        let raw_name = meta
            .arg_names
            .get(idx)
            .cloned()
            .unwrap_or_else(|| format!("arg{}", idx));
        explicit_args.push(EscherTaskMetaArg {
            name: raw_name,
            legacy_type,
            task_type,
        });
    }
    Ok(explicit_args)
}

fn build_task_example(
    spec: &EscherSpec,
    meta: &EscherSpecMeta,
    example: &ExampleJson,
    class_name: &str,
) -> Result<(Vec<Value>, Value)> {
    let mut next_pos = meta.arg_count;
    let mut value_lists: Vec<(String, Vec<Value>)> = Vec::new();
    for field in &meta.value_fields {
        let list = example
            .input
            .get(next_pos)
            .and_then(Value::as_array)
            .cloned()
            .ok_or_else(|| anyhow!("missing value list '{}' for {}", field, spec.name))?;
        value_lists.push((field.clone(), list));
        next_pos += 1;
    }
    let mut pointer_lists: Vec<(String, Vec<Value>)> = Vec::new();
    for field in &meta.pointer_fields {
        let list = example
            .input
            .get(next_pos)
            .and_then(Value::as_array)
            .cloned()
            .ok_or_else(|| anyhow!("missing pointer list '{}' for {}", field, spec.name))?;
        pointer_lists.push((field.clone(), list));
        next_pos += 1;
    }

    let object_count = value_lists
        .iter()
        .map(|(_, list)| list.len())
        .chain(pointer_lists.iter().map(|(_, list)| list.len()))
        .max()
        .unwrap_or(0);

    let object_heap = build_object_heap(object_count, &value_lists, &pointer_lists, class_name)?;

    let mut input = Vec::new();
    let receiver_ref = match meta.receiver_arg_index {
        Some(idx) => map_legacy_value_to_task_literal(
            example
                .input
                .get(idx)
                .ok_or_else(|| anyhow!("missing receiver arg {}", idx))?,
            spec.input_types
                .get(idx)
                .map(String::as_str)
                .unwrap_or("Ptr"),
            class_name,
        )?,
        None => json!({ "ref": if object_count == 0 { -1 } else { 0 } }),
    };
    input.push(receiver_ref);
    input.push(Value::Array(object_heap.clone()));

    for (_, list) in &value_lists {
        input.push(Value::Array(
            list.iter()
                .map(|value| json!(legacy_int_value(value)))
                .collect(),
        ));
    }
    for (_, list) in &pointer_lists {
        let heap = list
            .iter()
            .map(|value| {
                Ok(json!({
                    "ref": legacy_ptr_value_to_index(value)?
                        .map(|idx| idx as i32)
                        .unwrap_or(-1)
                }))
            })
            .collect::<Result<Vec<_>>>()?;
        input.push(Value::Array(heap));
    }

    for idx in 0..meta.arg_count {
        if meta.receiver_arg_index == Some(idx) {
            continue;
        }
        let legacy_type = spec
            .input_types
            .get(idx)
            .map(String::as_str)
            .unwrap_or("Int");
        let literal = map_legacy_value_to_task_literal(
            example
                .input
                .get(idx)
                .ok_or_else(|| anyhow!("missing arg {} for {}", idx, spec.name))?,
            legacy_type,
            class_name,
        )?;
        input.push(literal);
    }

    Ok((
        input,
        map_legacy_value_to_task_literal(&example.output, &spec.return_type, class_name)?,
    ))
}

fn build_object_heap(
    object_count: usize,
    value_lists: &[(String, Vec<Value>)],
    pointer_lists: &[(String, Vec<Value>)],
    class_name: &str,
) -> Result<Vec<Value>> {
    let mut heap = Vec::with_capacity(object_count);
    for idx in 0..object_count {
        let mut fields = Map::new();
        for (field, list) in value_lists {
            let value = list.get(idx).map(legacy_int_value).unwrap_or(-1);
            let ref_index = if value < 0 { -1 } else { idx as i32 };
            fields.insert(field.clone(), json!({ "ref": ref_index }));
        }
        for (field, list) in pointer_lists {
            let ref_index = match list.get(idx) {
                Some(value) => legacy_ptr_value_to_index(value)?
                    .map(|v| v as i32)
                    .unwrap_or(-1),
                None => -1,
            };
            fields.insert(field.clone(), json!({ "ref": ref_index }));
        }
        heap.push(json!({
            "object": {
                "className": class_name,
                "fields": fields,
            }
        }));
    }
    Ok(heap)
}

fn map_legacy_value_to_task_literal(
    value: &Value,
    legacy_type: &str,
    class_name: &str,
) -> Result<Value> {
    match legacy_type {
        "Ptr" => Ok(json!({
            "ref": legacy_ptr_value_to_index(value)?
                .map(|idx| idx as i32)
                .unwrap_or(-1)
        })),
        "Int" => Ok(json!(legacy_int_value(value))),
        "Bool" => Ok(json!(value.as_bool().unwrap_or(false))),
        "List[Int]" => Ok(Value::Array(
            value
                .as_array()
                .cloned()
                .unwrap_or_default()
                .into_iter()
                .map(|entry| json!(legacy_int_value(&entry)))
                .collect(),
        )),
        "List[Ptr]" => Ok(Value::Array(
            value
                .as_array()
                .cloned()
                .unwrap_or_default()
                .into_iter()
                .map(|entry| {
                    json!({
                        "ref": legacy_ptr_value_to_index(&entry)
                            .ok()
                            .flatten()
                            .map(|idx| idx as i32)
                            .unwrap_or(-1)
                    })
                })
                .collect(),
        )),
        other if other.starts_with("Ref[") && other.ends_with(']') => Ok(json!({
            "ref": legacy_ptr_value_to_index(value)?
                .map(|idx| idx as i32)
                .unwrap_or(-1)
        })),
        other => Err(anyhow!(
            "Unsupported legacy value type '{}' for task conversion (class={})",
            other,
            class_name
        )),
    }
}

fn legacy_type_to_task_type(legacy_type: &str, class_name: &str) -> Result<String> {
    match legacy_type {
        "Int" | "Bool" => Ok(legacy_type.to_string()),
        "Ptr" => Ok(format!("Ref[{}]", class_name)),
        "List[Int]" => Ok("List[Int]".to_string()),
        "List[Ptr]" => Ok(format!("List[Ref[{}]]", class_name)),
        other => Err(anyhow!(
            "Unsupported legacy type '{}' for escher-ts phase1 task generation",
            other
        )),
    }
}

fn default_task_components(
    spec: &EscherSpec,
    meta: &EscherSpecMeta,
) -> Vec<EscherTaskComponentSpec> {
    let mut components = Vec::new();
    let mut seen = HashSet::new();
    push_library_component(&mut components, &mut seen, "isNull");
    push_library_component(&mut components, &mut seen, "equal");
    push_library_component(&mut components, &mut seen, "and");
    push_library_component(&mut components, &mut seen, "or");
    push_library_component(&mut components, &mut seen, "not");

    let needs_int_components = spec.return_type == "Int"
        || spec
            .input_types
            .iter()
            .take(meta.arg_count)
            .any(|ty| ty == "Int")
        || !meta.value_fields.is_empty();
    if needs_int_components {
        push_library_component(&mut components, &mut seen, "isZero");
        push_library_component(&mut components, &mut seen, "isNonNeg");
        push_library_component(&mut components, &mut seen, "zero");
        push_library_component(&mut components, &mut seen, "inc");
        push_library_component(&mut components, &mut seen, "dec");
    }

    if spec.return_type == "Ptr" && meta.pointer_fields.len() == 1 {
        let last_ptr = last_ptr_component_spec("Node");
        if seen.insert(last_ptr.name.clone()) {
            components.push(last_ptr);
        }
        let nth_next_ref = nth_next_ref_component_spec("Node");
        if seen.insert(nth_next_ref.name.clone()) {
            components.push(nth_next_ref);
        }
    }

    if spec.return_type == "Ptr" && meta.pointer_fields.len() == 1 && meta.value_fields.len() == 1 {
        let find_by_value_ref = find_by_value_ref_component_spec("Node");
        if seen.insert(find_by_value_ref.name.clone()) {
            components.push(find_by_value_ref);
        }
    }

    components
}

fn push_library_component(
    components: &mut Vec<EscherTaskComponentSpec>,
    seen: &mut HashSet<String>,
    name: &str,
) {
    if seen.insert(name.to_string()) {
        components.push(EscherTaskComponentSpec {
            name: name.to_string(),
            kind: "libraryRef".to_string(),
            ref_name: Some(name.to_string()),
            input_types: None,
            return_type: None,
            args: None,
            body_js: None,
        });
    }
}

fn last_ptr_component_spec(class_name: &str) -> EscherTaskComponentSpec {
    let object_type = format!("Object[{}]", class_name);
    let ref_type = format!("Ref[{}]", object_type);
    EscherTaskComponentSpec {
        name: "last_ptr".to_string(),
        kind: "js".to_string(),
        ref_name: None,
        input_types: Some(vec![ref_type.clone(), format!("List[{}]", object_type)]),
        return_type: Some(ref_type),
        args: Some(vec!["start".to_string(), "nextHeap".to_string()]),
        body_js: Some(
            [
                "if (!start || typeof start !== 'object' || typeof start.ref !== 'number' || !Array.isArray(nextHeap)) return 'error';",
                "let current = start.ref;",
                "if (current === -1) return { ref: -1 };",
                "const seen = new Set();",
                "while (current !== -1) {",
                "  if (!Number.isInteger(current) || current < 0 || current >= nextHeap.length) return 'error';",
                "  if (seen.has(current)) return 'error';",
                "  seen.add(current);",
                "  const next = nextHeap[current];",
                "  if (!next || typeof next !== 'object' || typeof next.ref !== 'number') return 'error';",
                "  if (next.ref === -1) return { ref: current };",
                "  current = next.ref;",
                "}",
                "return { ref: -1 };",
            ]
            .join(" "),
        ),
    }
}

fn nth_next_ref_component_spec(class_name: &str) -> EscherTaskComponentSpec {
    let object_type = format!("Object[{}]", class_name);
    let ref_type = format!("Ref[{}]", object_type);
    EscherTaskComponentSpec {
        name: "nthNextRef".to_string(),
        kind: "js".to_string(),
        ref_name: None,
        input_types: Some(vec![
            ref_type.clone(),
            format!("List[{}]", object_type),
            format!("List[{}]", ref_type),
            "Int".to_string(),
        ]),
        return_type: Some(ref_type),
        args: Some(vec![
            "start".to_string(),
            "nodeHeap".to_string(),
            "nextHeap".to_string(),
            "steps".to_string(),
        ]),
        body_js: Some(
            [
                "if (!start || typeof start !== 'object' || typeof start.ref !== 'number') return 'error';",
                "if (!Array.isArray(nodeHeap) || !Array.isArray(nextHeap) || !Number.isInteger(steps) || steps < 0) return 'error';",
                "if (start.ref === -1) return { ref: -1 };",
                "if (start.ref < 0 || start.ref >= nodeHeap.length) return 'error';",
                "let current = start.ref;",
                "let remaining = steps;",
                "while (remaining > 0) {",
                "  if (current === -1) return { ref: -1 };",
                "  if (!Number.isInteger(current) || current < 0 || current >= nextHeap.length) return 'error';",
                "  const next = nextHeap[current];",
                "  if (!next || typeof next !== 'object' || typeof next.ref !== 'number') return 'error';",
                "  current = next.ref;",
                "  remaining -= 1;",
                "}",
                "return { ref: current };",
            ]
            .join(" "),
        ),
    }
}

fn find_by_value_ref_component_spec(class_name: &str) -> EscherTaskComponentSpec {
    let object_type = format!("Object[{}]", class_name);
    let ref_type = format!("Ref[{}]", object_type);
    EscherTaskComponentSpec {
        name: "findByValueRef".to_string(),
        kind: "js".to_string(),
        ref_name: None,
        input_types: Some(vec![
            ref_type.clone(),
            format!("List[{}]", object_type),
            format!("List[{}]", ref_type),
            "List[Int]".to_string(),
            "Int".to_string(),
        ]),
        return_type: Some(ref_type),
        args: Some(vec![
            "start".to_string(),
            "nodeHeap".to_string(),
            "nextHeap".to_string(),
            "valueHeap".to_string(),
            "target".to_string(),
        ]),
        body_js: Some(
            [
                "if (!start || typeof start !== 'object' || typeof start.ref !== 'number') return 'error';",
                "if (!Array.isArray(nodeHeap) || !Array.isArray(nextHeap) || !Array.isArray(valueHeap) || !Number.isInteger(target)) return 'error';",
                "if (start.ref === -1) return { ref: -1 };",
                "const seen = new Set();",
                "let current = start.ref;",
                "while (current !== -1) {",
                "  if (!Number.isInteger(current) || current < 0) return 'error';",
                "  if (seen.has(current)) return { ref: -1 };",
                "  seen.add(current);",
                "  if (current >= nodeHeap.length || current >= nextHeap.length || current >= valueHeap.length) return 'error';",
                "  const currentValue = valueHeap[current];",
                "  if (!Number.isInteger(currentValue)) return 'error';",
                "  if (currentValue === target) return { ref: current };",
                "  const next = nextHeap[current];",
                "  if (!next || typeof next !== 'object' || typeof next.ref !== 'number') return 'error';",
                "  current = next.ref;",
                "}",
                "return { ref: -1 };",
            ]
            .join(" "),
        ),
    }
}

fn field_to_heap_name(field: &str) -> String {
    let mut out = String::with_capacity(field.len() + 4);
    for (idx, ch) in field.chars().enumerate() {
        let valid = if idx == 0 {
            ch.is_ascii_alphabetic() || ch == '_' || ch == '$'
        } else {
            ch.is_ascii_alphanumeric() || ch == '_' || ch == '$'
        };
        if valid {
            out.push(ch);
        } else if idx == 0 && ch.is_ascii_digit() {
            out.push('_');
            out.push(ch);
        } else {
            out.push('_');
        }
    }
    if out.is_empty() {
        out.push('_');
    }
    out.push_str("Heap");
    out
}

fn sanitize_js_identifier(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    for ch in raw.chars() {
        let valid = ch.is_ascii_alphanumeric() || ch == '_' || ch == '$';
        out.push(if valid { ch } else { '_' });
    }
    if out.is_empty() {
        "_".to_string()
    } else if out
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

fn legacy_int_value(value: &Value) -> i32 {
    match value {
        Value::Number(num) => num.as_i64().unwrap_or(-1) as i32,
        Value::String(text) => text.parse::<i64>().unwrap_or(-1) as i32,
        Value::Bool(flag) => {
            if *flag {
                1
            } else {
                0
            }
        }
        _ => -1,
    }
}

fn legacy_ptr_value_to_index(value: &Value) -> Result<Option<usize>> {
    match value {
        Value::Null => Ok(None),
        Value::Number(num) => {
            let raw = num
                .as_i64()
                .ok_or_else(|| anyhow!("invalid pointer numeric value: {}", value))?;
            if raw < 0 {
                Ok(None)
            } else {
                Ok(Some(raw as usize))
            }
        }
        Value::Object(map) => match map.get("ref").and_then(Value::as_i64) {
            Some(raw) if raw >= 0 => Ok(Some(raw as usize)),
            Some(_) | None => Ok(None),
        },
        other => Err(anyhow!("invalid pointer literal: {}", other)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Edge, Node};
    use serde_json::json;

    #[test]
    fn parse_escher_js_max_old_space_mb_defaults_to_8192() {
        let mb = parse_escher_js_max_old_space_mb(None).expect("default");
        assert_eq!(mb, 8192);
    }

    #[test]
    fn parse_escher_js_max_old_space_mb_accepts_valid_integer() {
        let mb = parse_escher_js_max_old_space_mb(Some("12288")).expect("parsed");
        assert_eq!(mb, 12288);
    }

    #[test]
    fn parse_escher_js_max_old_space_mb_rejects_small_value() {
        let err = parse_escher_js_max_old_space_mb(Some("128")).expect_err("must fail");
        assert!(err.to_string().contains("must be >= 256"));
    }

    #[test]
    fn parse_escher_js_max_old_space_mb_rejects_non_numeric() {
        let err = parse_escher_js_max_old_space_mb(Some("abc")).expect_err("must fail");
        assert!(err
            .to_string()
            .contains("Invalid ESCHER_JS_MAX_OLD_SPACE_MB"));
    }

    fn graph_for_linear_list() -> (VisGraph, ListEnvironment) {
        // Objects: n1 -> n2 -> n3, values 10, 20, 30, and variable lst bound to n1
        let vis_graph = VisGraph {
            nodes: vec![
                Node {
                    id: "n1".to_string(),
                    is_literal: false,
                    label: json!("Node"),
                },
                Node {
                    id: "n2".to_string(),
                    is_literal: false,
                    label: json!("Node"),
                },
                Node {
                    id: "n3".to_string(),
                    is_literal: false,
                    label: json!("Node"),
                },
                Node {
                    id: "__Variable-lst".to_string(),
                    is_literal: false,
                    label: json!("Var"),
                },
                Node {
                    id: "v10".to_string(),
                    is_literal: true,
                    label: json!(10),
                },
                Node {
                    id: "v20".to_string(),
                    is_literal: true,
                    label: json!(20),
                },
                Node {
                    id: "v30".to_string(),
                    is_literal: true,
                    label: json!(30),
                },
            ],
            edges: vec![
                Edge {
                    from: "n1".to_string(),
                    to: "n2".to_string(),
                    label: "next".to_string(),
                },
                Edge {
                    from: "n2".to_string(),
                    to: "n3".to_string(),
                    label: "next".to_string(),
                },
                Edge {
                    from: "n1".to_string(),
                    to: "v10".to_string(),
                    label: "val".to_string(),
                },
                Edge {
                    from: "n2".to_string(),
                    to: "v20".to_string(),
                    label: "val".to_string(),
                },
                Edge {
                    from: "n3".to_string(),
                    to: "v30".to_string(),
                    label: "val".to_string(),
                },
                Edge {
                    from: "__Variable-lst".to_string(),
                    to: "n1".to_string(),
                    label: "lst".to_string(),
                },
            ],
        };
        let env = ListEnvironment::from_vis_graph(&vis_graph);
        (vis_graph, env)
    }

    #[test]
    fn test_build_spec_simple_list() {
        let (vis_graph, env) = graph_for_linear_list();
        let case = EscherCase {
            env,
            vis_graph,
            arguments: vec![json!(99)],
            arg_names: vec!["this".to_string()],
            arg_types: Some(vec!["Ptr".to_string()]),
            receiver_arg_index: Some(0),
            output: json!(2), // e.g., last node index in BFS order
        };
        let spec = build_escher_spec("append-g", "Int", &[case], None).expect("spec");
        assert_eq!(spec.name, "append-g");
        assert_eq!(
            spec.input_types,
            vec![
                "Ptr".to_string(),
                "List[Int]".to_string(),
                "List[Ptr]".to_string()
            ]
        );
        assert_eq!(spec.return_type, "Int".to_string());
        assert_eq!(spec.examples.len(), 1);
    }

    #[test]
    fn test_build_task_spec_from_legacy_spec() {
        let (vis_graph, env) = graph_for_linear_list();
        let case = EscherCase {
            env,
            vis_graph,
            arguments: vec![json!(0), json!(42)],
            arg_names: vec!["this".to_string(), "delta".to_string()],
            arg_types: Some(vec!["Ptr".to_string(), "Int".to_string()]),
            receiver_arg_index: Some(0),
            output: json!(1),
        };

        let meta = derive_spec_meta(std::slice::from_ref(&case)).expect("meta");
        let spec = build_escher_spec("advance-next", "Ptr", &[case], None).expect("spec");
        let task = build_escher_task_spec(&spec, &meta).expect("task");

        assert_eq!(task.name, "advance-next");
        assert!(task.auto_class_field_components);
        assert_eq!(task.signature.return_type, "Ref[Node]");
        assert!(task
            .components
            .iter()
            .any(|component| component.name == "last_ptr"));
        assert!(task
            .components
            .iter()
            .any(|component| component.name == "nthNextRef" && component.kind == "js"));
        assert!(task
            .components
            .iter()
            .any(|component| component.name == "findByValueRef" && component.kind == "js"));
        assert_eq!(task.signature.args.len(), 1);
        assert_eq!(task.signature.args[0].name, "delta");
        assert_eq!(task.signature.args[0].arg_type, "Int");
        assert_eq!(
            task.classes[0].fields.get("next").map(String::as_str),
            Some("Ref[Node]")
        );
        assert_eq!(
            task.classes[0].fields.get("val").map(String::as_str),
            Some("Ref[Int]")
        );

        assert_eq!(task.examples.len(), 1);
        let (input, output) = &task.examples[0];
        assert_eq!(input.len(), 5);
        assert_eq!(input[0], json!({ "ref": 0 }));
        assert_eq!(input[2], json!([10, 20, 30]));
        assert_eq!(input[3], json!([{ "ref": 1 }, { "ref": 2 }, { "ref": -1 }]));
        assert_eq!(input[4], json!(42));
        assert_eq!(*output, json!({ "ref": 1 }));
    }
}
