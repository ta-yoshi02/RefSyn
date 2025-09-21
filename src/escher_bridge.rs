//! Escher-Scala bridge: build tests.json-like specs from Kanon environments
//!
//! This module converts ListEnvironment snapshots (built from Kanon VisGraph + operations)
//! into Escher-Scala-compatible JSON examples. It supports:
//! - Dynamic field detection (value vs pointer) without hardcoding names
//! - Local indexing per test case via BFS from the detected root variable
//! - Sentinel -1 for null/undefined and pointer terminals
//! - Deterministic ordering of inputs (args, then value lists, then pointer lists)

use crate::list_env::ListEnvironment;
use crate::models::VisGraph;
use anyhow::{anyhow, Result};
use serde::Serialize;
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet, VecDeque};

/// One test case: environment + args + expected output
#[derive(Debug, Clone)]
pub struct EscherCase {
    pub env: ListEnvironment,
    pub vis_graph: VisGraph,
    pub arguments: Vec<Value>,
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

/// Build Escher-Scala tests.json content from cases.
/// - `name`: synthesized function name
/// - `return_type`: Escher type string (e.g., "Int", "List[Int]")
pub fn build_escher_spec(
    name: &str,
    return_type: &str,
    cases: &[EscherCase],
) -> Result<EscherSpec> {
    if cases.is_empty() {
        return Err(anyhow!("no cases provided"));
    }

    // Analyze fields from first case's graph to determine input arity/types
    let (value_fields, pointer_fields) = analyze_fields(&cases[0].vis_graph)?;
    let mut sorted_value_fields = value_fields.clone();
    sorted_value_fields.sort();
    let mut sorted_pointer_fields = pointer_fields.clone();
    sorted_pointer_fields.sort();

    // inputTypes: args first (Int per arg), then one List[Int] per value field, then per pointer field
    let mut input_types: Vec<String> = Vec::new();
    let arg_count = cases[0].arguments.len();
    for _ in 0..arg_count {
        input_types.push("Int".to_string());
    }
    for _ in &sorted_value_fields {
        input_types.push("List[Int]".to_string());
    }
    for _ in &sorted_pointer_fields {
        input_types.push("List[Int]".to_string());
    }

    // Examples
    let mut examples: Vec<ExampleJson> = Vec::new();
    for case in cases {
        // Field classification for this case (in case labels differ, but we keep names consistent)
        let (value_fields_c, pointer_fields_c) = analyze_fields(&case.vis_graph)?;

        // Enforce compatibility with first case
        if set_of(&value_fields_c) != set_of(&value_fields)
            || set_of(&pointer_fields_c) != set_of(&pointer_fields)
        {
            return Err(anyhow!(
                "field sets differ across cases: values={:?}/{:?}, pointers={:?}/{:?}",
                value_fields_c,
                value_fields,
                pointer_fields_c,
                pointer_fields
            ));
        }

        // Build BFS-local index mapping
        let bfs = build_bfs_order(&case.env, &case.vis_graph, &pointer_fields_c)?;

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

// ---------- internals ----------

fn set_of(v: &[String]) -> HashSet<String> {
    v.iter().cloned().collect()
}

/// Analyze field labels dynamically:
/// - value fields: have at least one edge to a literal node
/// - pointer fields: have at least one edge to a non-literal object node
/// Ignores edges whose `from` is a special variable node (id starts with "__Variable-") or
/// special rect ("__RectForVariable__").
fn analyze_fields(vis_graph: &VisGraph) -> Result<(Vec<String>, Vec<String>)> {
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
            // variable binding edge; ignore for field classification
            continue;
        }
        if literal_ids.contains(e.to.as_str()) {
            value_fields.insert(e.label.clone());
        } else if object_ids.contains(e.to.as_str()) {
            pointer_fields.insert(e.label.clone());
        }
    }

    Ok((
        value_fields.into_iter().collect(),
        pointer_fields.into_iter().collect(),
    ))
}

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
                if let Some(to) = v.as_i64() {
                    let to_i = to as i64;
                    if to_i >= 0 {
                        let to_usize = to_i as usize;
                        adj.entry(from_idx).or_default().push(to_usize);
                    }
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

    // map indices
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
/// - find nodes with id prefix "__Variable-<name>"
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

    for var_id in var_ids {
        if let Some(&var_idx) = env.obj_id_to_index.get(var_id) {
            let var_name = var_id.trim_start_matches(var_prefix).to_string();
            if let Some(vec) = env.field_lists.get(&var_name) {
                if var_idx < vec.len() {
                    if let Some(root_i64) = vec[var_idx].as_i64() {
                        if root_i64 >= 0 {
                            return Ok((root_i64 as usize, var_name));
                        }
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

fn build_pointer_index_list(env: &ListEnvironment, bfs: &BfsOrder, field: &str) -> Vec<i32> {
    let mut result = Vec::new();
    let default = json!(-1);
    let vec_ref = env.field_lists.get(field);
    for &orig_idx in &bfs.order_indices {
        let v = match vec_ref {
            Some(vs) if orig_idx < vs.len() => &vs[orig_idx],
            _ => &default,
        };
        // env stores pointer as original index (0..N-1) or -1
        let target_orig = v.as_i64().unwrap_or(-1);
        if target_orig < 0 {
            result.push(-1);
        } else {
            let to_orig = target_orig as usize;
            let mapped = bfs
                .idx_to_bfs
                .get(&to_orig)
                .copied()
                .map(|i| i as i32)
                .unwrap_or(-1);
            result.push(mapped);
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
            output: json!(2), // e.g., last node index in BFS order
        };
        let spec = build_escher_spec("append-g", "Int", &[case]).expect("spec");
        assert_eq!(spec.name, "append-g");
        assert_eq!(
            spec.input_types,
            vec![
                "Int".to_string(),
                "List[Int]".to_string(),
                "List[Int]".to_string()
            ]
        );
        assert_eq!(spec.return_type, "Int".to_string());
        assert_eq!(spec.examples.len(), 1);
    }
}
