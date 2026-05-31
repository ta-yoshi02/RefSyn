use refsyn::models::{Edge, Node, VisGraph};
use refsyn::{handle_synthesis, MethodCallOperation, SynthesisRequest, SynthesisResponse};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use warp::http::StatusCode;
use warp::Reply;

fn base_vis_graph() -> VisGraph {
    VisGraph {
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
        ],
        edges: vec![Edge {
            from: "n1".to_string(),
            to: "n2".to_string(),
            label: "next".to_string(),
        }],
    }
}

fn make_call(
    label: &str,
    context_id: &str,
    receiver: &str,
    method_name: &str,
    operations: Vec<serde_json::Value>,
    actual_graph: &VisGraph,
) -> MethodCallOperation {
    MethodCallOperation {
        call_label: label.to_string(),
        context_sensitive_id: context_id.to_string(),
        receiver_object: receiver.to_string(),
        method_name: method_name.to_string(),
        arguments: vec![],
        argument_types: None,
        argument_names: None,
        method_param_names: None,
        operations,
        precond_graph: None,
        actual_graph: Some(actual_graph.clone()),
        id_mapping: None,
        field_tables: None,
    }
}

fn fixture_vis_graph_for_operations_json() -> VisGraph {
    VisGraph {
        nodes: vec![
            Node {
                id: "main-new1".to_string(),
                is_literal: false,
                label: json!("Node"),
            },
            Node {
                id: "main-new2".to_string(),
                is_literal: false,
                label: json!("Node"),
            },
            Node {
                id: "main-new3".to_string(),
                is_literal: false,
                label: json!("Node"),
            },
            Node {
                id: "main-new4".to_string(),
                is_literal: false,
                label: json!("Node"),
            },
            Node {
                id: "main-new1-val".to_string(),
                is_literal: true,
                label: json!("1"),
            },
            Node {
                id: "main-new2-val".to_string(),
                is_literal: true,
                label: json!("2"),
            },
            Node {
                id: "main-new3-val".to_string(),
                is_literal: true,
                label: json!("3"),
            },
            Node {
                id: "main-new4-val".to_string(),
                is_literal: true,
                label: json!("4"),
            },
        ],
        edges: vec![
            Edge {
                from: "main-new1".to_string(),
                to: "main-new1-val".to_string(),
                label: "val".to_string(),
            },
            Edge {
                from: "main-new2".to_string(),
                to: "main-new2-val".to_string(),
                label: "val".to_string(),
            },
            Edge {
                from: "main-new3".to_string(),
                to: "main-new3-val".to_string(),
                label: "val".to_string(),
            },
            Edge {
                from: "main-new4".to_string(),
                to: "main-new4-val".to_string(),
                label: "val".to_string(),
            },
            Edge {
                from: "main-new1".to_string(),
                to: "main-new2".to_string(),
                label: "next".to_string(),
            },
            Edge {
                from: "main-new2".to_string(),
                to: "main-new3".to_string(),
                label: "next".to_string(),
            },
            Edge {
                from: "main-new3".to_string(),
                to: "main-new4".to_string(),
                label: "next".to_string(),
            },
        ],
    }
}

fn load_fixture_method_calls(method_name: &str) -> Vec<MethodCallOperation> {
    let content = fs::read_to_string("tests/data/operations.json")
        .expect("tests/data/operations.json should be readable");
    let root: serde_json::Value =
        serde_json::from_str(&content).expect("operations.json should be valid json");
    let method_calls = root
        .get("methodCalls")
        .and_then(|v| v.as_array())
        .expect("operations.json should contain methodCalls array");

    method_calls
        .iter()
        .filter(|call| call.get("methodName").and_then(|v| v.as_str()) == Some(method_name))
        .map(|call| {
            serde_json::from_value::<MethodCallOperation>(call.clone())
                .expect("fixture method call should deserialize")
        })
        .collect()
}

fn duplicate_method_call(call: &MethodCallOperation, suffix: &str) -> MethodCallOperation {
    MethodCallOperation {
        call_label: format!("{}-{}", call.call_label, suffix),
        context_sensitive_id: format!("{}-{}", call.context_sensitive_id, suffix),
        receiver_object: call.receiver_object.clone(),
        method_name: call.method_name.clone(),
        arguments: call.arguments.clone(),
        argument_types: call.argument_types.clone(),
        argument_names: call.argument_names.clone(),
        method_param_names: call.method_param_names.clone(),
        operations: call.operations.clone(),
        precond_graph: call.precond_graph.clone(),
        actual_graph: call.actual_graph.clone(),
        id_mapping: call.id_mapping.clone(),
        field_tables: call.field_tables.clone(),
    }
}

async fn run_synthesis_and_decode(request: SynthesisRequest) -> (StatusCode, SynthesisResponse) {
    let request_body = serde_json::to_vec(&request).unwrap();
    let body_bytes = bytes::Bytes::from(request_body);

    let reply = handle_synthesis(body_bytes)
        .await
        .expect("handle_synthesis should not return a warp error");
    let response = reply.into_response();
    let status = response.status();
    let body = hyper::body::to_bytes(response.into_body())
        .await
        .expect("response body should be readable");
    let parsed: SynthesisResponse =
        serde_json::from_slice(&body).expect("response body should be valid synthesis json");
    (status, parsed)
}

fn extract_task_json_path(summary: &str) -> Option<String> {
    let marker = "escher-ts task JSON saved:";
    let start = summary.find(marker)?;
    let after = summary[start + marker.len()..].trim();
    let first = after.split(',').next()?.trim();
    if first.is_empty() {
        None
    } else {
        Some(first.to_string())
    }
}

fn escher_ts_dist_index_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("external")
        .join("escher-ts")
        .join("dist")
        .join("index.js")
}

fn require_escher_ts_backend() {
    let dist_index = escher_ts_dist_index_path();
    assert!(
        dist_index.exists(),
        "backend execution test requires {}. Run `git submodule update --init --recursive` and then `cd external/escher-ts && pnpm install --frozen-lockfile && pnpm build`.",
        dist_index.display()
    );
}

fn extract_saved_return_type(spec: &Value) -> Option<&str> {
    spec.get("returnType").and_then(Value::as_str).or_else(|| {
        spec.get("signature")
            .and_then(|signature| signature.get("returnType"))
            .and_then(Value::as_str)
    })
}

fn is_pointer_return_type(spec: &Value) -> bool {
    matches!(extract_saved_return_type(spec), Some("Ptr"))
        || extract_saved_return_type(spec)
            .map(|ty| ty.starts_with("Ref["))
            .unwrap_or(false)
}

fn extract_saved_example_io(example: &Value) -> Option<(&Value, &Value)> {
    match example {
        Value::Object(fields) => Some((fields.get("input")?, fields.get("output")?)),
        Value::Array(items) if items.len() == 2 => Some((&items[0], &items[1])),
        _ => None,
    }
}

fn extract_component_names(spec: &Value) -> Vec<&str> {
    spec.get("components")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|component| component.get("name").and_then(Value::as_str))
        .collect()
}

fn input_slot_is_ref_heap(input: &Value, slot: usize) -> bool {
    input
        .get(slot)
        .and_then(Value::as_array)
        .and_then(|entries| entries.first())
        .and_then(Value::as_object)
        .map(|entry| entry.contains_key("ref"))
        .unwrap_or(false)
}

/// 統合されたsynthesizeエンドポイントのテスト - 単一の操作列
#[tokio::test]
async fn test_integrated_synthesis_single_operation_list() {
    let vis_graph = base_vis_graph();
    let operations = vec![
        json!({
            "editType": "addNode",
            "id": "n3",
            "label": "Node",
            "isLiteral": false
        }),
        json!({
            "editType": "addEdge",
            "from": "n1",
            "to": "n3",
            "label": "next"
        }),
    ];
    let method_call = make_call("call1", "main", "n1", "append", operations, &vis_graph);

    let request = SynthesisRequest {
        method_calls: vec![method_call],
        vis_graph,
    };
    let (status, response) = run_synthesis_and_decode(request).await;

    assert_eq!(status, StatusCode::OK);
    assert!(response.code.is_empty());
    assert!(response.individual_codes.is_empty());
    let pattern = response
        .common_pattern
        .expect("single-trace synthesis should produce a common plan");
    assert!(pattern.contains("COMMON_PLAN"));
    assert!(pattern.contains("addNode"));
    assert!(pattern.contains("addEdge"));
    let holes = response
        .hole_information
        .expect("single-trace synthesis should include hole info (empty)");
    assert!(holes.is_empty());
    let composed = response
        .composed_method_code
        .expect("single-trace synthesis should produce composed method code");
    assert!(composed.contains("append("));
    assert!(composed.contains("const tmp0"));
    assert!(composed.contains("this.next = tmp0"));
    assert!(response.operation_analysis.is_none());
    let info = response
        .list_environment_info
        .expect("list environment summary should be present");
    assert!(info.contains("List Environment Summary:"));
    assert!(info.contains("- 2 objects tracked"));
    assert!(info.contains("Current state:"));
}

/// 統合されたsynthesizeエンドポイントのテスト - 複数の操作列（操作分析が含まれる）
#[tokio::test]
async fn test_integrated_synthesis_multiple_operation_lists() {
    let vis_graph = base_vis_graph();
    let operations_a = vec![
        json!({
            "editType": "addNode",
            "id": "n3",
            "label": "Node",
            "isLiteral": false
        }),
        json!({
            "editType": "addEdge",
            "from": "n1",
            "to": "n3",
            "label": "next"
        }),
    ];
    let operations_b = vec![
        json!({
            "editType": "addNode",
            "id": "n4",
            "label": "Node",
            "isLiteral": false
        }),
        json!({
            "editType": "addEdge",
            "from": "n2",
            "to": "n4",
            "label": "next"
        }),
    ];

    let method_call_a = make_call("call1", "main", "n1", "append", operations_a, &vis_graph);
    let method_call_b = make_call("call2", "main", "n2", "append", operations_b, &vis_graph);

    let request = SynthesisRequest {
        method_calls: vec![method_call_a, method_call_b],
        vis_graph,
    };
    let (status, response) = run_synthesis_and_decode(request).await;

    assert_eq!(status, StatusCode::OK);
    let analysis = response
        .operation_analysis
        .expect("operation analysis should be generated for two traces");
    assert_eq!(analysis.total_operations_counts, vec![3, 3]);
    assert!(analysis.common_operations_count <= 2);
    assert!(analysis.differences_found <= 2);
}

#[tokio::test]
async fn test_integrated_synthesis_three_operation_lists_recompute_consensus() {
    let vis_graph = base_vis_graph();
    let operations_a = vec![
        json!({
            "editType": "addNode",
            "id": "n3",
            "label": "Node",
            "isLiteral": false
        }),
        json!({
            "editType": "addEdge",
            "from": "n1",
            "to": "n3",
            "label": "next"
        }),
    ];
    let operations_b = vec![
        json!({
            "editType": "addNode",
            "id": "n4",
            "label": "Node",
            "isLiteral": false
        }),
        json!({
            "editType": "addEdge",
            "from": "n2",
            "to": "n4",
            "label": "next"
        }),
    ];
    let operations_c = vec![json!({
        "editType": "addNode",
        "id": "n5",
        "label": "Node",
        "isLiteral": false
    })];

    let method_call_a = make_call("call1", "main", "n1", "append", operations_a, &vis_graph);
    let method_call_b = make_call("call2", "main", "n2", "append", operations_b, &vis_graph);
    let method_call_c = make_call("call3", "main", "n1", "append", operations_c, &vis_graph);

    let request = SynthesisRequest {
        method_calls: vec![method_call_a, method_call_b, method_call_c],
        vis_graph,
    };
    let (status, response) = run_synthesis_and_decode(request).await;

    assert_eq!(status, StatusCode::OK);
    let analysis = response
        .operation_analysis
        .expect("operation analysis should be generated for three traces");
    assert_eq!(analysis.total_operations_counts.len(), 3);
    assert!(analysis.common_operations_count <= 1);
    assert!(analysis.differences_found >= 1);
}

/// 統合されたsynthesizeエンドポイントのテスト - 空のmethod_calls
#[tokio::test]
async fn test_integrated_synthesis_empty_method_calls() {
    let request = SynthesisRequest {
        method_calls: vec![],
        vis_graph: base_vis_graph(),
    };
    let (status, response) = run_synthesis_and_decode(request).await;

    assert_eq!(status, StatusCode::OK);
    assert!(response.code.is_empty());
    assert!(response.individual_codes.is_empty());
    assert_eq!(response.common_pattern, None);
    assert_eq!(response.hole_information, None);
    assert_eq!(response.composed_method_code, None);
    assert_eq!(response.list_environment_info, None);
    assert!(response.operation_analysis.is_none());
}

#[tokio::test]
async fn test_integrated_synthesis_rejects_mixed_method_names() {
    let vis_graph = base_vis_graph();
    let call_a = make_call("call1", "main", "n1", "append", vec![], &vis_graph);
    let call_b = make_call("call2", "main", "n1", "insert", vec![], &vis_graph);
    let request = SynthesisRequest {
        method_calls: vec![call_a, call_b],
        vis_graph,
    };
    let (status, response) = run_synthesis_and_decode(request).await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(response.common_pattern, None);
    assert_eq!(response.hole_information, None);
    assert_eq!(response.composed_method_code, None);
    assert_eq!(response.code, Vec::<String>::new());
    assert_eq!(response.individual_codes, Vec::<String>::new());
    assert_eq!(
        response.list_environment_info,
        Some("Mismatched method names in method_calls: append, insert".to_string())
    );
}

#[tokio::test]
async fn test_integrated_synthesis_rejects_remove_operations() {
    let vis_graph = base_vis_graph();
    let operations = vec![json!({
        "editType": "removeEdge",
        "from": "n1",
        "to": "n2",
        "label": "next"
    })];
    let call = make_call("call1", "main", "n1", "append", operations, &vis_graph);
    let request = SynthesisRequest {
        method_calls: vec![call],
        vis_graph,
    };
    let (status, response) = run_synthesis_and_decode(request).await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(response.common_pattern, None);
    assert_eq!(response.hole_information, None);
    assert_eq!(response.composed_method_code, None);
    assert_eq!(response.code, Vec::<String>::new());
    assert_eq!(response.individual_codes, Vec::<String>::new());
    assert_eq!(
        response.list_environment_info,
        Some("Unsupported remove operation detected at call 0 op 0: removeEdge".to_string())
    );
}

#[tokio::test]
async fn test_operations_json_removeval_accepts_delete_edge() {
    let method_calls = load_fixture_method_calls("removeVal");
    assert_eq!(method_calls.len(), 2, "removeVal should have two traces");
    let request = SynthesisRequest {
        method_calls,
        vis_graph: fixture_vis_graph_for_operations_json(),
    };
    let (status, response) = run_synthesis_and_decode(request).await;

    assert_eq!(status, StatusCode::OK);
    let info = response
        .list_environment_info
        .expect("environment summary should be present");
    assert!(info.contains("List[obj_next]"));
    assert!(info.contains("Null"));
}

#[tokio::test]
async fn test_popback_delete_edge_uses_edge_source_hole() {
    let graph_a = fixture_vis_graph_for_operations_json();
    let mut graph_a = graph_a;
    graph_a.nodes.push(Node {
        id: "__Variable-lst".to_string(),
        is_literal: false,
        label: json!("lst"),
    });
    graph_a.edges.push(Edge {
        from: "__Variable-lst".to_string(),
        to: "main-new1".to_string(),
        label: "lst".to_string(),
    });
    let mut graph_b = graph_a.clone();
    graph_b.edges.retain(|edge| {
        !(edge.from == "main-new3" && edge.to == "main-new4" && edge.label == "next")
    });

    let call_a = MethodCallOperation {
        call_label: "call1".to_string(),
        context_sensitive_id: "main".to_string(),
        receiver_object: "main-new1".to_string(),
        method_name: "popBack".to_string(),
        arguments: vec![],
        argument_types: None,
        argument_names: None,
        method_param_names: None,
        operations: vec![json!({
            "editType": "deleteEdge",
            "from": "main-new3",
            "to": "main-new4",
            "label": "next"
        })],
        precond_graph: Some(graph_a.clone()),
        actual_graph: None,
        id_mapping: None,
        field_tables: None,
    };
    let call_b = MethodCallOperation {
        call_label: "call2".to_string(),
        context_sensitive_id: "main".to_string(),
        receiver_object: "main-new1".to_string(),
        method_name: "popBack".to_string(),
        arguments: vec![],
        argument_types: None,
        argument_names: None,
        method_param_names: None,
        operations: vec![json!({
            "editType": "deleteEdge",
            "from": "main-new2",
            "to": "main-new3",
            "label": "next"
        })],
        precond_graph: Some(graph_b),
        actual_graph: None,
        id_mapping: None,
        field_tables: None,
    };
    let request = SynthesisRequest {
        method_calls: vec![call_a, call_b],
        vis_graph: graph_a,
    };
    let (status, response) = run_synthesis_and_decode(request).await;

    assert_eq!(status, StatusCode::OK);
    let code = response
        .composed_method_code
        .expect("popBack should compose from deleteEdge common plan");
    assert!(
        code.contains("this.popBack_"),
        "popBack should call synthesized predecessor helper, got:\n{}",
        code
    );
    assert!(
        code.contains(".next = null;"),
        "deleteEdge should clear the synthesized edge source, got:\n{}",
        code
    );
    assert!(
        !code.contains("this.next.next.next = null;"),
        "popBack must not replay the first fixed-depth trace"
    );
}

#[tokio::test]
async fn test_operations_json_set_supports_edit_edge_reference_analysis() {
    let mut method_calls = load_fixture_method_calls("set");
    assert_eq!(
        method_calls.len(),
        1,
        "set should have one trace in fixture"
    );
    let base_call = method_calls.pop().expect("set call should exist");
    let duplicate = duplicate_method_call(&base_call, "dup");

    let request = SynthesisRequest {
        method_calls: vec![base_call, duplicate],
        vis_graph: fixture_vis_graph_for_operations_json(),
    };
    let (status, response) = run_synthesis_and_decode(request).await;

    assert_eq!(status, StatusCode::OK);
    let analysis = response
        .operation_analysis
        .expect("operation analysis should exist for two traces");
    assert_eq!(analysis.differences_found, 0);
    let common_pattern = response
        .common_pattern
        .expect("common pattern should be generated");
    assert!(common_pattern.contains("editEdgeReference"));
}

#[tokio::test]
async fn test_operations_json_reverse_supports_edit_edge_reference_analysis() {
    let mut method_calls = load_fixture_method_calls("reverse");
    assert_eq!(
        method_calls.len(),
        1,
        "reverse should have one trace in fixture"
    );
    let base_call = method_calls.pop().expect("reverse call should exist");
    let duplicate = duplicate_method_call(&base_call, "dup");

    let request = SynthesisRequest {
        method_calls: vec![base_call, duplicate],
        vis_graph: fixture_vis_graph_for_operations_json(),
    };
    let (status, response) = run_synthesis_and_decode(request).await;

    assert_eq!(status, StatusCode::OK);
    let analysis = response
        .operation_analysis
        .expect("operation analysis should exist for two traces");
    assert_eq!(analysis.differences_found, 0);
    let common_pattern = response
        .common_pattern
        .expect("common pattern should be generated");
    assert!(common_pattern.contains("editEdgeReference"));
}

#[tokio::test]
async fn test_set_with_existing_kanon_id_and_mismatched_receiver_still_composes() {
    let target_id = "main-call2-FunctionExpression2-new1";
    let target_val_id = "main-call2-FunctionExpression2-new1-val";
    let vis_graph = VisGraph {
        nodes: vec![
            Node {
                id: "main-new2".to_string(),
                is_literal: false,
                label: json!("Node"),
            },
            Node {
                id: target_id.to_string(),
                is_literal: false,
                label: json!("Node"),
            },
            Node {
                id: target_val_id.to_string(),
                is_literal: true,
                label: json!("25"),
            },
            Node {
                id: "__Variable-lst".to_string(),
                is_literal: false,
                label: json!("lst"),
            },
        ],
        edges: vec![
            Edge {
                from: "main-new2".to_string(),
                to: target_id.to_string(),
                label: "next".to_string(),
            },
            Edge {
                from: target_id.to_string(),
                to: target_val_id.to_string(),
                label: "val".to_string(),
            },
            Edge {
                from: "__Variable-lst".to_string(),
                to: "main-new2".to_string(),
                label: "lst".to_string(),
            },
        ],
    };

    let operations_a = vec![
        json!({
            "editType": "addNode",
            "id": "__temp1",
            "label": "10",
            "isLiteral": true,
            "type": "string"
        }),
        json!({
            "editType": "editEdgeReference",
            "from": target_id,
            "oldTo": target_val_id,
            "newTo": "__temp1",
            "label": "val"
        }),
    ];
    let operations_b = vec![
        json!({
            "editType": "addNode",
            "id": "__temp2",
            "label": "82",
            "isLiteral": true,
            "type": "string"
        }),
        json!({
            "editType": "editEdgeReference",
            "from": target_id,
            "oldTo": target_val_id,
            "newTo": "__temp2",
            "label": "val"
        }),
    ];

    let call_a = MethodCallOperation {
        call_label: "call5".to_string(),
        context_sensitive_id: "main".to_string(),
        receiver_object: "main-new1".to_string(), // intentionally stale/mismatched
        method_name: "set".to_string(),
        arguments: vec![json!(10)],
        argument_types: Some(vec!["Int".to_string()]),
        argument_names: Some(vec!["arg0".to_string()]),
        method_param_names: Some(vec!["arg".to_string()]),
        operations: operations_a,
        precond_graph: None,
        actual_graph: Some(vis_graph.clone()),
        id_mapping: Some(HashMap::from([
            ("__temp_target".to_string(), target_id.to_string()),
            ("__temp_target_val".to_string(), target_val_id.to_string()),
        ])),
        field_tables: None,
    };
    let call_b = MethodCallOperation {
        call_label: "call6".to_string(),
        context_sensitive_id: "main".to_string(),
        receiver_object: "main-new1".to_string(), // intentionally stale/mismatched
        method_name: "set".to_string(),
        arguments: vec![json!(82)],
        argument_types: Some(vec!["Int".to_string()]),
        argument_names: Some(vec!["arg0".to_string()]),
        method_param_names: Some(vec!["arg".to_string()]),
        operations: operations_b,
        precond_graph: None,
        actual_graph: Some(vis_graph.clone()),
        id_mapping: Some(HashMap::from([
            ("__temp_target".to_string(), target_id.to_string()),
            ("__temp_target_val".to_string(), target_val_id.to_string()),
        ])),
        field_tables: None,
    };

    let request = SynthesisRequest {
        method_calls: vec![call_a, call_b],
        vis_graph,
    };
    let (status, response) = run_synthesis_and_decode(request).await;

    assert_eq!(status, StatusCode::OK);
    let composed = response
        .composed_method_code
        .expect("composed method should be generated");
    assert!(composed.contains("set(arg) {"));
    assert!(composed.contains("this.next.val = h_int_0;"));
}

#[tokio::test]
async fn test_integrated_synthesis_three_append_like_specs_group_holes() {
    require_escher_ts_backend();

    let graph_call1 = VisGraph {
        nodes: vec![
            Node {
                id: "main-new1".to_string(),
                is_literal: false,
                label: json!("Node"),
            },
            Node {
                id: "main-new1-val".to_string(),
                is_literal: true,
                label: json!(2),
            },
            Node {
                id: "__Variable-lst".to_string(),
                is_literal: false,
                label: json!("lst"),
            },
        ],
        edges: vec![
            Edge {
                from: "main-new1".to_string(),
                to: "main-new1-val".to_string(),
                label: "val".to_string(),
            },
            Edge {
                from: "__Variable-lst".to_string(),
                to: "main-new1".to_string(),
                label: "lst".to_string(),
            },
        ],
    };

    let graph_call2 = VisGraph {
        nodes: vec![
            Node {
                id: "main-new1".to_string(),
                is_literal: false,
                label: json!("Node"),
            },
            Node {
                id: "main-new1-val".to_string(),
                is_literal: true,
                label: json!(2),
            },
            Node {
                id: "__temp1".to_string(),
                is_literal: false,
                label: json!("Node"),
            },
            Node {
                id: "__temp1-val".to_string(),
                is_literal: true,
                label: json!("25"),
            },
            Node {
                id: "__Variable-lst".to_string(),
                is_literal: false,
                label: json!("lst"),
            },
        ],
        edges: vec![
            Edge {
                from: "main-new1".to_string(),
                to: "main-new1-val".to_string(),
                label: "val".to_string(),
            },
            Edge {
                from: "__temp1".to_string(),
                to: "__temp1-val".to_string(),
                label: "val".to_string(),
            },
            Edge {
                from: "main-new1".to_string(),
                to: "__temp1".to_string(),
                label: "next".to_string(),
            },
            Edge {
                from: "__Variable-lst".to_string(),
                to: "main-new1".to_string(),
                label: "lst".to_string(),
            },
        ],
    };

    let graph_call3 = VisGraph {
        nodes: vec![
            Node {
                id: "main-new1".to_string(),
                is_literal: false,
                label: json!("Node"),
            },
            Node {
                id: "main-new1-val".to_string(),
                is_literal: true,
                label: json!(2),
            },
            Node {
                id: "__temp1".to_string(),
                is_literal: false,
                label: json!("Node"),
            },
            Node {
                id: "__temp1-val".to_string(),
                is_literal: true,
                label: json!("25"),
            },
            Node {
                id: "__temp4".to_string(),
                is_literal: false,
                label: json!("Node"),
            },
            Node {
                id: "__temp4-val".to_string(),
                is_literal: true,
                label: json!("93"),
            },
            Node {
                id: "__Variable-lst".to_string(),
                is_literal: false,
                label: json!("lst"),
            },
        ],
        edges: vec![
            Edge {
                from: "main-new1".to_string(),
                to: "main-new1-val".to_string(),
                label: "val".to_string(),
            },
            Edge {
                from: "__temp1".to_string(),
                to: "__temp1-val".to_string(),
                label: "val".to_string(),
            },
            Edge {
                from: "__temp4".to_string(),
                to: "__temp4-val".to_string(),
                label: "val".to_string(),
            },
            Edge {
                from: "__temp1".to_string(),
                to: "__temp4".to_string(),
                label: "next".to_string(),
            },
            Edge {
                from: "main-new1".to_string(),
                to: "__temp1".to_string(),
                label: "next".to_string(),
            },
            Edge {
                from: "__Variable-lst".to_string(),
                to: "main-new1".to_string(),
                label: "lst".to_string(),
            },
        ],
    };

    let call1 = MethodCallOperation {
        call_label: "call1".to_string(),
        context_sensitive_id: "main".to_string(),
        receiver_object: "main-new1".to_string(),
        method_name: "append".to_string(),
        arguments: vec![json!(25)],
        argument_types: Some(vec!["Int".to_string()]),
        argument_names: Some(vec!["arg0".to_string()]),
        method_param_names: Some(vec!["arg".to_string()]),
        operations: vec![
            json!({"editType":"addNode","id":"__temp1","label":"Node","isLiteral":false}),
            json!({"editType":"addNode","id":"__temp2","label":"25","isLiteral":true,"type":"string"}),
            json!({"editType":"addEdge","from":"__temp1","to":"__temp2","label":"val"}),
            json!({"editType":"addEdge","from":"main-new1","to":"__temp1","label":"next"}),
        ],
        precond_graph: None,
        actual_graph: Some(graph_call1.clone()),
        id_mapping: None,
        field_tables: None,
    };

    let call2 = MethodCallOperation {
        call_label: "call2".to_string(),
        context_sensitive_id: "main".to_string(),
        receiver_object: "main-new1".to_string(),
        method_name: "append".to_string(),
        arguments: vec![json!(93)],
        argument_types: Some(vec!["Int".to_string()]),
        argument_names: Some(vec!["arg0".to_string()]),
        method_param_names: Some(vec!["arg".to_string()]),
        operations: vec![
            json!({"editType":"addNode","id":"__temp3","label":"93","isLiteral":true,"type":"string"}),
            json!({"editType":"addNode","id":"__temp4","label":"Node","isLiteral":false}),
            json!({"editType":"addEdge","from":"__temp1","to":"__temp4","label":"next"}),
            json!({"editType":"addEdge","from":"__temp4","to":"__temp3","label":"val"}),
        ],
        precond_graph: None,
        actual_graph: Some(graph_call2),
        id_mapping: None,
        field_tables: None,
    };

    let call3 = MethodCallOperation {
        call_label: "call3".to_string(),
        context_sensitive_id: "main".to_string(),
        receiver_object: "main-new1".to_string(),
        method_name: "append".to_string(),
        arguments: vec![json!(48)],
        argument_types: Some(vec!["Int".to_string()]),
        argument_names: Some(vec!["arg0".to_string()]),
        method_param_names: Some(vec!["arg".to_string()]),
        operations: vec![
            json!({"editType":"addNode","id":"__temp5","label":"Node","isLiteral":false}),
            json!({"editType":"addEdge","from":"__temp4","to":"__temp5","label":"next"}),
            json!({"editType":"addNode","id":"__temp6","label":"48","isLiteral":true,"type":"string"}),
            json!({"editType":"addEdge","from":"__temp5","to":"__temp6","label":"val"}),
        ],
        precond_graph: None,
        actual_graph: Some(graph_call3),
        id_mapping: None,
        field_tables: None,
    };

    let request = SynthesisRequest {
        method_calls: vec![call1, call2, call3],
        vis_graph: graph_call1,
    };
    let (status, response) = run_synthesis_and_decode(request).await;

    assert_eq!(status, StatusCode::OK);
    let analysis = response
        .operation_analysis
        .expect("operation analysis should be generated");
    assert_eq!(analysis.total_operations_counts.len(), 3);
    assert!(analysis
        .total_operations_counts
        .iter()
        .all(|count| *count >= 4));
    assert!(analysis.common_operations_count >= 1);

    let list_info = response
        .list_environment_info
        .expect("list environment info should be present");
    assert!(
        !list_info.contains("recovered value assignment")
            && !list_info.contains("recovered edge assignment"),
        "insert three-trace composition should be driven by materialized operation operands, not metadata recovery:\n{}",
        list_info
    );
    let common_pattern = response
        .common_pattern
        .as_deref()
        .expect("common pattern should be present");
    assert!(
        common_pattern.contains("addEdge(from=__temp1, to=__hole_")
            && common_pattern.contains("label=val"),
        "value assignment must appear as an operation-operand hole, got:\n{}",
        common_pattern
    );
    assert!(
        (common_pattern.contains("editEdgeReference(from=__hole_")
            || common_pattern.contains("addEdge(from=__hole_"))
            && common_pattern.contains("to=__temp1")
            && common_pattern.contains("label=next"),
        "rewire source must appear as an operation-operand hole, got:\n{}",
        common_pattern
    );
    let spec_path = extract_task_json_path(&list_info).expect("task json path should be reported");
    let task_json_text =
        fs::read_to_string(&spec_path).expect("generated task json should be readable");
    let specs: serde_json::Value =
        serde_json::from_str(&task_json_text).expect("generated task json should parse");
    let spec_list = specs
        .as_array()
        .expect("generated task json should be an array of task specs");
    assert!(
        spec_list.len() >= 2,
        "append-like 3 traces should produce at least two grouped hole specs"
    );

    let mut has_ptr = false;
    let mut has_int = false;
    for spec in spec_list {
        let return_type = extract_saved_return_type(spec).expect("spec should contain returnType");
        if is_pointer_return_type(spec) {
            has_ptr = true;
        }
        if return_type == "Int" {
            has_int = true;
        }
    }
    assert!(has_ptr, "grouped specs should include a Ptr-returning hole");
    assert!(
        has_int,
        "grouped specs should include an Int-returning hole"
    );

    for spec in spec_list {
        let examples = spec
            .get("examples")
            .and_then(|v| v.as_array())
            .expect("each spec should contain examples");
        assert_eq!(
            examples.len(),
            3,
            "each grouped spec should carry one example per trace"
        );

        let mut seen_by_input: HashMap<String, String> = HashMap::new();
        for ex in examples {
            let (input_value, output_value) =
                extract_saved_example_io(ex).expect("example should have input/output");
            let input = serde_json::to_string(input_value).expect("input should serialize");
            let output = serde_json::to_string(output_value).expect("output should serialize");
            if let Some(existing) = seen_by_input.insert(input.clone(), output.clone()) {
                assert_eq!(
                    existing, output,
                    "same input must not map to conflicting outputs"
                );
            }
        }
    }

    let ptr_helper = response
        .code
        .iter()
        .find(|code| code.contains("append_h("))
        .expect("Ptr helper JS should be present");
    let ptr_result = response
        .escher_results
        .as_ref()
        .and_then(|results| results.iter().find(|result| result.name == "append-h"))
        .expect("append-h escher result should be present");
    assert!(
        ptr_result
            .rendered
            .as_deref()
            .map(|rendered| rendered.contains("last_ptr("))
            .unwrap_or(false),
        "append-h should synthesize through last_ptr instead of a fixed-hop pattern: {:?}",
        ptr_result.rendered
    );
    assert!(
        !ptr_helper.contains(".next).next"),
        "append_h should not underfit to a two-hop pattern: {}",
        ptr_helper
    );
}

#[tokio::test]
async fn test_integrated_synthesis_insert_groups_value_and_edge_source_holes() {
    let request: SynthesisRequest = serde_json::from_value(json!({
        "method_calls": [
            {
                "callLabel": "call1",
                "contextSensitiveID": "main",
                "receiverObject": "main-new2",
                "methodName": "insert",
                "arguments": [0, 80],
                "argumentTypes": ["Int", "Int"],
                "argumentNames": ["arg0", "arg1"],
                "methodParamNames": ["i", "arg"],
                "operations": [
                    {"editType":"addNode","id":"__temp1","label":"Node","isLiteral":false},
                    {"editType":"addNode","id":"__temp2","label":"80","isLiteral":true,"type":"string"},
                    {"editType":"addEdge","from":"__temp1","to":"__temp2","label":"val"},
                    {"editType":"editEdgeReference","from":"main-new2","oldTo":"__temp1","newTo":"__temp1","label":"next"},
                    {"editType":"addEdge","from":"__temp1","to":"__temp1","label":"next"}
                ],
                "precondGraph": {
                    "nodes": [
                        {"id":"main-new2","isLiteral":false,"label":"Node"},
                        {"id":"main-new2-val","isLiteral":true,"label":"37"},
                        {"id":"main-new3","isLiteral":false,"label":"Node"},
                        {"id":"main-new3-val","isLiteral":true,"label":"25"},
                        {"id":"__Variable-lst","isLiteral":false,"label":"lst"}
                    ],
                    "edges": [
                        {"from":"main-new2","to":"main-new2-val","label":"val"},
                        {"from":"main-new3","to":"main-new3-val","label":"val"},
                        {"from":"main-new2","to":"main-new3","label":"next"},
                        {"from":"__Variable-lst","to":"main-new2","label":"lst"}
                    ]
                },
                "actualGraph": {
                    "nodes": [
                        {"id":"main-new2","isLiteral":false,"label":"Node"},
                        {"id":"main-new2-val","isLiteral":true,"label":"37"},
                        {"id":"main-new3","isLiteral":false,"label":"Node"},
                        {"id":"main-new3-val","isLiteral":true,"label":"25"},
                        {"id":"__Variable-lst","isLiteral":false,"label":"lst"}
                    ],
                    "edges": [
                        {"from":"main-new2","to":"main-new2-val","label":"val"},
                        {"from":"main-new3","to":"main-new3-val","label":"val"},
                        {"from":"main-new2","to":"main-new3","label":"next"},
                        {"from":"__Variable-lst","to":"main-new2","label":"lst"}
                    ]
                },
                "idMapping": {"__temp1":"main-new3"}
            },
            {
                "callLabel": "call2",
                "contextSensitiveID": "main",
                "receiverObject": "main-new1",
                "methodName": "insert",
                "arguments": [1, 71],
                "argumentTypes": ["Int", "Int"],
                "argumentNames": ["arg0", "arg1"],
                "methodParamNames": ["i", "arg"],
                "operations": [
                    {"editType":"addNode","id":"__temp3","label":"71","isLiteral":true,"type":"string"},
                    {"editType":"addNode","id":"__temp4","label":"Node","isLiteral":false},
                    {"editType":"addEdge","from":"__temp4","to":"__temp3","label":"val"},
                    {"editType":"editEdgeReference","from":"__temp1","oldTo":"__temp1","newTo":"__temp4","label":"next"},
                    {"editType":"addEdge","from":"__temp4","to":"__temp1","label":"next"}
                ],
                "precondGraph": {
                    "nodes": [
                        {"id":"main-new2","isLiteral":false,"label":"Node"},
                        {"id":"main-new2-val","isLiteral":true,"label":"37"},
                        {"id":"__temp1","isLiteral":false,"label":"Node"},
                        {"id":"__temp1-val","isLiteral":true,"label":"80"},
                        {"id":"main-new3","isLiteral":false,"label":"Node"},
                        {"id":"main-new3-val","isLiteral":true,"label":"25"},
                        {"id":"__Variable-lst","isLiteral":false,"label":"lst"}
                    ],
                    "edges": [
                        {"from":"main-new2","to":"main-new2-val","label":"val"},
                        {"from":"__temp1","to":"__temp1-val","label":"val"},
                        {"from":"main-new3","to":"main-new3-val","label":"val"},
                        {"from":"__temp1","to":"main-new3","label":"next"},
                        {"from":"main-new2","to":"__temp1","label":"next"},
                        {"from":"__Variable-lst","to":"main-new2","label":"lst"}
                    ]
                },
                "actualGraph": {
                    "nodes": [
                        {"id":"main-new2","isLiteral":false,"label":"Node"},
                        {"id":"main-new2-val","isLiteral":true,"label":"37"},
                        {"id":"__temp1","isLiteral":false,"label":"Node"},
                        {"id":"__temp1-val","isLiteral":true,"label":"80"},
                        {"id":"main-new3","isLiteral":false,"label":"Node"},
                        {"id":"main-new3-val","isLiteral":true,"label":"25"},
                        {"id":"__Variable-lst","isLiteral":false,"label":"lst"}
                    ],
                    "edges": [
                        {"from":"main-new2","to":"main-new2-val","label":"val"},
                        {"from":"__temp1","to":"__temp1-val","label":"val"},
                        {"from":"main-new3","to":"main-new3-val","label":"val"},
                        {"from":"__temp1","to":"main-new3","label":"next"},
                        {"from":"main-new2","to":"__temp1","label":"next"},
                        {"from":"__Variable-lst","to":"main-new2","label":"lst"}
                    ]
                },
                "idMapping": {"__temp1":"__temp1","__temp1-val":"__temp1-val","__temp4":"main-new3"}
            }
        ],
        "vis_graph": {
            "nodes": [
                {"id":"main-new2","isLiteral":false,"label":"Node"},
                {"id":"main-new2-val","isLiteral":true,"label":"37"},
                {"id":"main-new3","isLiteral":false,"label":"Node"},
                {"id":"main-new3-val","isLiteral":true,"label":"25"},
                {"id":"__Variable-lst","isLiteral":false,"label":"lst"},
                {"id":"__temp1","isLiteral":false,"label":"Node"},
                {"id":"__temp2","isLiteral":true,"label":"80"}
            ],
            "edges": [
                {"from":"main-new2","to":"main-new2-val","label":"val"},
                {"from":"main-new3","to":"main-new3-val","label":"val"},
                {"from":"main-new2","to":"__temp1","label":"next"},
                {"from":"__Variable-lst","to":"main-new2","label":"lst"},
                {"from":"__temp1","to":"__temp2","label":"val"},
                {"from":"__temp1","to":"main-new3","label":"next"}
            ]
        }
    }))
    .expect("insert request fixture should deserialize");

    let (status, response) = run_synthesis_and_decode(request).await;
    assert_eq!(status, StatusCode::OK);
    assert!(
        response.common_pattern.is_some(),
        "three-trace insert should keep a grouped common pattern"
    );
    assert!(
        response.hole_information.is_some(),
        "three-trace insert should keep grouped hole information"
    );
    assert!(
        response.composed_method_code.is_some(),
        "three-trace insert should keep composed method code"
    );

    let list_info = response
        .list_environment_info
        .expect("list environment info should be present");
    let spec_path = extract_task_json_path(&list_info).expect("task json path should be reported");
    let task_json_text =
        fs::read_to_string(&spec_path).expect("generated task json should be readable");
    let specs: serde_json::Value =
        serde_json::from_str(&task_json_text).expect("generated task json should parse");
    let spec_list = specs
        .as_array()
        .expect("generated task json should be an array of task specs");

    let mut has_ptr = false;
    let mut has_int = false;
    for spec in spec_list {
        let return_type = extract_saved_return_type(spec).expect("spec should contain returnType");
        if is_pointer_return_type(spec) {
            has_ptr = true;
        }
        if return_type == "Int" {
            has_int = true;
        }
    }
    assert!(has_ptr, "insert decomposition should retain a Ptr hole");
    assert!(has_int, "insert decomposition should retain an Int hole");

    let hole_info = response
        .hole_information
        .expect("hole information should be present");
    let mut has_edge_source = false;
    let mut has_value = false;
    for values in hole_info.values() {
        for value in values {
            if value == "role=edge_source" {
                has_edge_source = true;
            }
            if value == "role=value" {
                has_value = true;
            }
        }
    }
    assert!(
        has_edge_source,
        "insert decomposition should expose an edge_source hole"
    );
    assert!(has_value, "insert decomposition should expose a value hole");
}

#[tokio::test]
async fn test_integrated_synthesis_insert_keeps_edge_source_with_runtime_scoped_precond_ids() {
    let request: SynthesisRequest = serde_json::from_value(json!({
        "method_calls": [
            {
                "callLabel": "call1",
                "contextSensitiveID": "main",
                "receiverObject": "main-new1",
                "methodName": "insert",
                "arguments": [0, 80],
                "argumentTypes": ["Int", "Int"],
                "argumentNames": ["arg0", "arg1"],
                "methodParamNames": ["i", "arg"],
                "operations": [
                    {"editType":"addNode","id":"__temp1","label":"Node","isLiteral":false},
                    {"editType":"addNode","id":"__temp2","label":"80","isLiteral":true,"type":"string"},
                    {"editType":"addEdge","from":"__temp1","to":"__temp2","label":"val"},
                    {"editType":"editEdgeReference","from":"main-new1","oldTo":"main-new2","newTo":"__temp1","label":"next"},
                    {"editType":"addEdge","from":"__temp1","to":"main-new2","label":"next"}
                ],
                "precondGraph": {
                    "nodes": [
                        {"id":"main-new1","isLiteral":false,"label":"Node"},
                        {"id":"main-new2","isLiteral":false,"label":"Node"},
                        {"id":"main-new1-val","isLiteral":true,"label":"37"},
                        {"id":"main-new2-val","isLiteral":true,"label":"25"},
                        {"id":"__Variable-lst","isLiteral":false,"label":"lst"}
                    ],
                    "edges": [
                        {"from":"main-new1","to":"main-new1-val","label":"val"},
                        {"from":"main-new2","to":"main-new2-val","label":"val"},
                        {"from":"main-new1","to":"main-new2","label":"next"},
                        {"from":"__Variable-lst","to":"main-new1","label":"lst"}
                    ]
                },
                "actualGraph": {
                    "nodes": [
                        {"id":"main-new1","isLiteral":false,"label":"Node"},
                        {"id":"main-new2","isLiteral":false,"label":"Node"},
                        {"id":"main-call1-FunctionExpression2-new3","isLiteral":false,"label":"Node"},
                        {"id":"main-new1-val","isLiteral":true,"label":"37"},
                        {"id":"main-new2-val","isLiteral":true,"label":"25"},
                        {"id":"main-call1-FunctionExpression2-new3-val","isLiteral":true,"label":"80"},
                        {"id":"__Variable-lst","isLiteral":false,"label":"lst"}
                    ],
                    "edges": [
                        {"from":"main-new1","to":"main-new1-val","label":"val"},
                        {"from":"main-new2","to":"main-new2-val","label":"val"},
                        {"from":"main-call1-FunctionExpression2-new3","to":"main-call1-FunctionExpression2-new3-val","label":"val"},
                        {"from":"main-new1","to":"main-call1-FunctionExpression2-new3","label":"next"},
                        {"from":"main-call1-FunctionExpression2-new3","to":"main-new2","label":"next"},
                        {"from":"__Variable-lst","to":"main-new1","label":"lst"}
                    ]
                },
                "idMapping": {
                    "__temp1":"main-call1-FunctionExpression2-new3",
                    "__temp2":"main-call1-FunctionExpression2-new3-val"
                }
            },
            {
                "callLabel": "call2",
                "contextSensitiveID": "main",
                "receiverObject": "main-new1",
                "methodName": "insert",
                "arguments": [1, 71],
                "argumentTypes": ["Int", "Int"],
                "argumentNames": ["arg0", "arg1"],
                "methodParamNames": ["i", "arg"],
                "operations": [
                    {"editType":"addNode","id":"__temp3","label":"Node","isLiteral":false},
                    {"editType":"addNode","id":"__temp4","label":"71","isLiteral":true,"type":"string"},
                    {"editType":"addEdge","from":"__temp3","to":"__temp4","label":"val"},
                    {"editType":"editEdgeReference","from":"__temp1","oldTo":"main-new2","newTo":"__temp3","label":"next"},
                    {"editType":"addEdge","from":"__temp3","to":"main-new2","label":"next"}
                ],
                "precondGraph": {
                    "nodes": [
                        {"id":"main-new1","isLiteral":false,"label":"Node"},
                        {"id":"main-new2","isLiteral":false,"label":"Node"},
                        {"id":"main-call1-FunctionExpression2-new3","isLiteral":false,"label":"Node"},
                        {"id":"main-new1-val","isLiteral":true,"label":"37"},
                        {"id":"main-new2-val","isLiteral":true,"label":"25"},
                        {"id":"main-call1-FunctionExpression2-new3-val","isLiteral":true,"label":"80"},
                        {"id":"__Variable-lst","isLiteral":false,"label":"lst"}
                    ],
                    "edges": [
                        {"from":"main-new1","to":"main-new1-val","label":"val"},
                        {"from":"main-new2","to":"main-new2-val","label":"val"},
                        {"from":"main-call1-FunctionExpression2-new3","to":"main-call1-FunctionExpression2-new3-val","label":"val"},
                        {"from":"main-new1","to":"main-call1-FunctionExpression2-new3","label":"next"},
                        {"from":"main-call1-FunctionExpression2-new3","to":"main-new2","label":"next"},
                        {"from":"__Variable-lst","to":"main-new1","label":"lst"}
                    ]
                },
                "actualGraph": {
                    "nodes": [
                        {"id":"main-new1","isLiteral":false,"label":"Node"},
                        {"id":"main-new2","isLiteral":false,"label":"Node"},
                        {"id":"main-call1-FunctionExpression2-new3","isLiteral":false,"label":"Node"},
                        {"id":"main-call2-FunctionExpression2-new3","isLiteral":false,"label":"Node"},
                        {"id":"main-new1-val","isLiteral":true,"label":"37"},
                        {"id":"main-new2-val","isLiteral":true,"label":"25"},
                        {"id":"main-call1-FunctionExpression2-new3-val","isLiteral":true,"label":"80"},
                        {"id":"main-call2-FunctionExpression2-new3-val","isLiteral":true,"label":"71"},
                        {"id":"__Variable-lst","isLiteral":false,"label":"lst"}
                    ],
                    "edges": [
                        {"from":"main-new1","to":"main-new1-val","label":"val"},
                        {"from":"main-new2","to":"main-new2-val","label":"val"},
                        {"from":"main-call1-FunctionExpression2-new3","to":"main-call1-FunctionExpression2-new3-val","label":"val"},
                        {"from":"main-call2-FunctionExpression2-new3","to":"main-call2-FunctionExpression2-new3-val","label":"val"},
                        {"from":"main-new1","to":"main-call2-FunctionExpression2-new3","label":"next"},
                        {"from":"main-call2-FunctionExpression2-new3","to":"main-call1-FunctionExpression2-new3","label":"next"},
                        {"from":"main-call1-FunctionExpression2-new3","to":"main-new2","label":"next"},
                        {"from":"__Variable-lst","to":"main-new1","label":"lst"}
                    ]
                },
                "idMapping": {
                    "__temp3":"main-call1-FunctionExpression2-new3",
                    "__temp4":"main-call1-FunctionExpression2-new3-val"
                }
            }
        ],
        "vis_graph": {
            "nodes": [
                {"id":"main-new1","isLiteral":false,"label":"Node"},
                {"id":"main-new2","isLiteral":false,"label":"Node"},
                {"id":"main-new1-val","isLiteral":true,"label":"37"},
                {"id":"main-new2-val","isLiteral":true,"label":"25"},
                {"id":"__temp1","isLiteral":false,"label":"Node"},
                {"id":"__temp2","isLiteral":true,"label":"80"},
                {"id":"__Variable-lst","isLiteral":false,"label":"lst"}
            ],
            "edges": [
                {"from":"main-new1","to":"main-new1-val","label":"val"},
                {"from":"main-new2","to":"main-new2-val","label":"val"},
                {"from":"main-new1","to":"__temp1","label":"next"},
                {"from":"__temp1","to":"__temp2","label":"val"},
                {"from":"__temp1","to":"main-new2","label":"next"},
                {"from":"__Variable-lst","to":"main-new1","label":"lst"}
            ]
        }
    }))
    .expect("runtime-scoped insert request should deserialize");

    let (status, response) = run_synthesis_and_decode(request).await;
    assert_eq!(status, StatusCode::OK);
    let hole_info = response
        .hole_information
        .expect("hole information should be present");
    let mut has_edge_source = false;
    let mut has_value = false;
    for values in hole_info.values() {
        for value in values {
            if value == "role=edge_source" {
                has_edge_source = true;
            }
            if value == "role=value" {
                has_value = true;
            }
        }
    }
    assert!(
        has_edge_source,
        "runtime-scoped precond ids should not erase edge_source holes"
    );
    assert!(
        has_value,
        "runtime-scoped precond ids should still keep value holes"
    );
}

#[tokio::test]
async fn test_integrated_synthesis_insert_three_traces_task_json_keeps_ts_ptr_components_and_ref_heaps(
) {
    let request_text = fs::read_to_string("examples/current_user_insert_three_traces.json")
        .expect("current insert fixture should be readable");
    let request: SynthesisRequest = serde_json::from_str(&request_text)
        .expect("three-trace insert request fixture should deserialize");

    let (status, response) = run_synthesis_and_decode(request).await;
    assert_eq!(status, StatusCode::OK);

    let list_info = response
        .list_environment_info
        .expect("list environment info should be present");
    let spec_path = extract_task_json_path(&list_info).expect("task json path should be reported");
    let task_json_text =
        fs::read_to_string(&spec_path).expect("generated task json should be readable");
    let specs: Value =
        serde_json::from_str(&task_json_text).expect("generated task json should parse");
    let spec_list = specs
        .as_array()
        .expect("generated task json should be an array of task specs");

    let ptr_spec = spec_list
        .iter()
        .find(|spec| is_pointer_return_type(spec))
        .expect("three-trace insert should keep a pointer-returning grouped task");

    let component_names = extract_component_names(ptr_spec);
    assert!(
        component_names.contains(&"nthNextRef"),
        "Ptr grouped task should retain nthNextRef"
    );
    assert!(
        component_names.contains(&"last_ptr"),
        "Ptr grouped task should retain last_ptr"
    );
    assert!(
        component_names.contains(&"findByValueRef"),
        "Ptr grouped task should retain findByValueRef"
    );

    let examples = ptr_spec
        .get("examples")
        .and_then(Value::as_array)
        .expect("Ptr grouped task should contain examples");
    assert_eq!(
        examples.len(),
        3,
        "Ptr grouped task should keep three traces"
    );
    for example in examples {
        let (input, _) = extract_saved_example_io(example).expect("task example should be valid");
        assert!(
            input_slot_is_ref_heap(input, 2),
            "pointer heap slot should stay a ref-array after task conversion"
        );
    }

    let value_output_sets: Vec<Vec<i64>> = spec_list
        .iter()
        .filter(|spec| extract_saved_return_type(spec) == Some("Int"))
        .filter_map(|spec| {
            spec.get("examples")
                .and_then(Value::as_array)
                .map(|examples| {
                    examples
                        .iter()
                        .filter_map(|example| {
                            let (_, output) = extract_saved_example_io(example)?;
                            output.as_i64()
                        })
                        .collect::<Vec<_>>()
                })
        })
        .collect();
    assert!(
        value_output_sets
            .iter()
            .any(|outputs| outputs == &[24, 50, 73]),
        "insert value hole should learn arg values for all traces, got {:?}",
        value_output_sets
    );

    let pointer_output_sets: Vec<Vec<i64>> = spec_list
        .iter()
        .filter(|spec| is_pointer_return_type(spec))
        .filter_map(|spec| {
            spec.get("examples")
                .and_then(Value::as_array)
                .map(|examples| {
                    examples
                        .iter()
                        .filter_map(|example| {
                            let (_, output) = extract_saved_example_io(example)?;
                            output
                                .get("ref")
                                .and_then(Value::as_i64)
                                .or_else(|| output.as_i64())
                        })
                        .collect::<Vec<_>>()
                })
        })
        .collect();
    assert!(
        pointer_output_sets
            .iter()
            .any(|outputs| outputs == &[0, 1, 2]),
        "insert predecessor hole should learn nthNextRef(this, i), got {:?}",
        pointer_output_sets
    );

    let composed = response
        .composed_method_code
        .as_deref()
        .expect("composed method should be present");
    let has_explicit_old_successor = pointer_output_sets
        .iter()
        .any(|outputs| outputs == &[1, 2, 3]);
    let derives_old_successor_from_pred =
        composed.contains("tmp0.next = (h_ptr_0 === null ? null : h_ptr_0.next);");
    assert!(
        has_explicit_old_successor || derives_old_successor_from_pred,
        "insert must either synthesize oldSucc=[1,2,3] or derive tmp.next from pred.next; pointer outputs={:?}, code:\n{}",
        pointer_output_sets,
        composed
    );
}

#[tokio::test]
async fn test_integrated_synthesis_insert_three_traces_executes_backend_tasks() {
    require_escher_ts_backend();

    let request_text = fs::read_to_string("examples/current_user_insert_three_traces.json")
        .expect("current insert fixture should be readable");
    let request: SynthesisRequest = serde_json::from_str(&request_text)
        .expect("three-trace insert request fixture should deserialize");

    let (status, response) = run_synthesis_and_decode(request).await;
    assert_eq!(status, StatusCode::OK);

    let list_info = response
        .list_environment_info
        .as_deref()
        .expect("list environment info should be present");
    let spec_path = extract_task_json_path(list_info).expect("task json path should be reported");
    let task_json_text =
        fs::read_to_string(&spec_path).expect("generated task json should be readable");
    let specs: Value =
        serde_json::from_str(&task_json_text).expect("generated task json should parse");
    let task_names: Vec<String> = specs
        .as_array()
        .expect("generated task json should be an array of task specs")
        .iter()
        .filter_map(|spec| spec.get("name").and_then(Value::as_str).map(str::to_string))
        .collect();

    assert!(
        !task_names.is_empty(),
        "expected at least one emitted escher-ts task"
    );
    assert!(
        task_names.iter().any(
            |name| response
                .escher_results
                .as_ref()
                .into_iter()
                .flatten()
                .any(|result| result.name == *name
                    && result
                        .rendered
                        .as_deref()
                        .is_some_and(|rendered| rendered.contains("@arg1")))
        ),
        "insert backend tasks should include the value helper, got {:?}",
        task_names
    );
    let composed = response
        .composed_method_code
        .as_deref()
        .expect("composed method should be present");
    assert!(
        composed.contains("tmp0.val = h_int_0;"),
        "pruning unused specs must not drop the value assignment from the composed method: {}",
        composed
    );

    let results = response
        .escher_results
        .as_ref()
        .expect("backend execution should populate escher results");
    let success_count = results.iter().filter(|result| result.success).count();
    assert_eq!(
        response.code.len(),
        success_count,
        "each successful backend result should produce compiled JS"
    );
    assert!(
        success_count > 0,
        "expected at least one successful backend synthesis result, got {:?}",
        results
            .iter()
            .map(|result| (
                result.name.as_str(),
                result.success,
                result.error.as_deref()
            ))
            .collect::<Vec<_>>()
    );

    for task_name in &task_names {
        let result = results
            .iter()
            .find(|result| &result.name == task_name)
            .unwrap_or_else(|| panic!("missing backend result for task {}", task_name));
        assert!(
            result.success,
            "backend result for {} failed: {:?}",
            task_name, result.error
        );
        assert!(
            response
                .individual_codes
                .iter()
                .any(|entry| entry.starts_with(&format!("{}: ", task_name))
                    && !entry.contains(": ERROR ")),
            "expected compiled JS entry for task {}, got {:?}",
            task_name,
            response.individual_codes
        );
    }
}
