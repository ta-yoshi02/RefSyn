//! Escher-Scala bridge: build tests.json-like specs from Kanon environments
//!
//! This module converts ListEnvironment snapshots (built from Kanon VisGraph + operations)
//! into Escher-Scala-compatible JSON examples. It supports:
//! - Dynamic field detection (value vs pointer) without hardcoding names
//! - Local indexing per test case via BFS from the detected root variable
//! - nullPtr is encoded as JSON null; missing Int values remain -1
//! - Deterministic ordering of inputs (args, then value lists, then pointer lists)

use crate::list_env::{FieldKind, ListEnvironment, PtrValue};
use crate::models::VisGraph;
use crate::FieldTables;
use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet, VecDeque};
use std::io::Write;
use std::path::PathBuf;
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

#[derive(Serialize, Debug, Clone)]
pub struct ExampleJson {
    input: Vec<Value>,
    output: Value,
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

/// Build Escher-Scala tests.json content from cases.
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
        // Pointer fields are typed as List[Ptr] on Escher-Scala side
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

pub fn specs_to_json(specs: &[EscherSpec]) -> Result<String> {
    Ok(serde_json::to_string_pretty(specs)?)
}

/// Convenience: write the produced spec JSON string to a file path (e.g.,
/// `Escher-Scala/src/main/resources/escher/tests.json`).
pub fn write_spec_to_file(path: &str, content: &str) -> Result<()> {
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

/// Invoke Scala.js build via Node and parse normalized results.
pub fn run_escher_js(spec_json: &str) -> Result<Vec<EscherJsOutcome>> {
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

    let mut child = Command::new("node")
        .arg(&runner)
        .arg("--quiet")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .map_err(|e| anyhow!("Failed to spawn Node runner: {}", e))?;

    {
        let stdin = child
            .stdin
            .as_mut()
            .ok_or_else(|| anyhow!("Failed to open stdin for Node runner"))?;
        stdin.write_all(spec_json.as_bytes())?;
    }

    let output = child.wait_with_output()?;
    if !output.status.success() {
        return Err(anyhow!(
            "Escher JS runner exited with status {}",
            output.status
        ));
    }

    let stdout = String::from_utf8(output.stdout)?;
    let trimmed = stdout.trim();
    if trimmed.is_empty() {
        return Err(anyhow!("Escher JS runner returned empty output"));
    }

    let outcomes: Vec<EscherJsOutcome> = serde_json::from_str(trimmed)?;
    Ok(outcomes)
}

pub fn run_escher_js_from_specs(specs: &[EscherSpec]) -> Result<Vec<EscherJsOutcome>> {
    let json_text = specs_to_json(specs)?;
    run_escher_js(&json_text)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Edge, Node};
    use serde_json::json;

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
}
