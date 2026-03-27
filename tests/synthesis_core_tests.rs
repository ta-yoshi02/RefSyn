use refsyn::models::{Edge, Node, VisGraph};
use refsyn::{
    handle_synthesis, synthesize_core, MethodCallOperation, SynthesisCoreOptions, SynthesisRequest,
    SynthesisResponse,
};
use serde_json::json;
use warp::Reply;

fn base_vis_graph() -> VisGraph {
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
                id: "main-new2-val".to_string(),
                is_literal: true,
                label: json!("2"),
            },
            Node {
                id: "__Variable-lst".to_string(),
                is_literal: false,
                label: json!("__Variable-lst"),
            },
        ],
        edges: vec![
            Edge {
                from: "main-new1".to_string(),
                to: "main-new2".to_string(),
                label: "next".to_string(),
            },
            Edge {
                from: "main-new2".to_string(),
                to: "main-new2-val".to_string(),
                label: "val".to_string(),
            },
            Edge {
                from: "__Variable-lst".to_string(),
                to: "main-new1".to_string(),
                label: "lst".to_string(),
            },
        ],
    }
}

fn append_call(
    context_sensitive_id: &str,
    operations: Vec<serde_json::Value>,
    argument: i64,
) -> MethodCallOperation {
    MethodCallOperation {
        call_label: "append".to_string(),
        context_sensitive_id: context_sensitive_id.to_string(),
        receiver_object: "main-new1".to_string(),
        method_name: "append".to_string(),
        arguments: vec![json!(argument)],
        argument_types: Some(vec!["Int".to_string()]),
        argument_names: Some(vec!["arg".to_string()]),
        method_param_names: Some(vec!["arg".to_string()]),
        operations,
        precond_graph: None,
        actual_graph: Some(base_vis_graph()),
        id_mapping: None,
        field_tables: None,
    }
}

async fn run_handle(request: &SynthesisRequest) -> SynthesisResponse {
    let body = bytes::Bytes::from(serde_json::to_vec(request).expect("request should serialize"));
    let reply = handle_synthesis(body)
        .await
        .expect("handle_synthesis should succeed");
    let response = reply.into_response();
    let body = hyper::body::to_bytes(response.into_body())
        .await
        .expect("response body should be readable");
    serde_json::from_slice(&body).expect("response body should decode")
}

#[tokio::test]
async fn synthesize_core_matches_handle_for_single_trace_response() {
    let request = SynthesisRequest {
        method_calls: vec![append_call(
            "call1",
            vec![
                json!({"editType": "addNode", "id": "__temp1", "isLiteral": false, "label": "Node"}),
                json!({"editType": "addNode", "id": "__temp2", "isLiteral": true, "label": "0", "type": "string"}),
                json!({"editType": "addEdge", "from": "__temp1", "label": "val", "to": "__temp2"}),
                json!({"editType": "addEdge", "from": "main-new2", "label": "next", "to": "__temp1"}),
            ],
            26,
        )],
        vis_graph: base_vis_graph(),
    };

    let core = synthesize_core(request.clone(), SynthesisCoreOptions::default())
        .expect("core synthesis should succeed");
    let http = run_handle(&request).await;

    assert_eq!(core.response.common_pattern, http.common_pattern);
    assert_eq!(core.response.composed_method_code, http.composed_method_code);
    let core_info = core
        .response
        .list_environment_info
        .clone()
        .expect("core summary should exist");
    let http_info = http
        .list_environment_info
        .clone()
        .expect("http summary should exist");
    assert!(core_info.contains("List Environment Summary"));
    assert!(http_info.contains("List Environment Summary"));
    assert!(core_info.contains("3 objects tracked"));
    assert!(http_info.contains("3 objects tracked"));
    assert_eq!(
        serde_json::to_value(&core.response.operation_analysis).expect("core analysis should serialize"),
        serde_json::to_value(&http.operation_analysis).expect("http analysis should serialize"),
    );
    assert!(core.task_json.is_none());
}

#[test]
fn synthesize_core_produces_browser_artifacts_for_multi_trace_requests() {
    let request = SynthesisRequest {
        method_calls: vec![
            append_call(
                "call1",
                vec![
                    json!({"editType": "addNode", "id": "__temp1", "isLiteral": false, "label": "Node"}),
                    json!({"editType": "addNode", "id": "__temp2", "isLiteral": true, "label": "0", "type": "string"}),
                    json!({"editType": "addEdge", "from": "__temp1", "label": "val", "to": "__temp2"}),
                    json!({"editType": "addEdge", "from": "main-new2", "label": "next", "to": "__temp1"}),
                ],
                26,
            ),
            append_call(
                "call2",
                vec![
                    json!({"editType": "addNode", "id": "__temp3", "isLiteral": false, "label": "Node"}),
                    json!({"editType": "addNode", "id": "__temp4", "isLiteral": true, "label": "3", "type": "string"}),
                    json!({"editType": "addEdge", "from": "__temp3", "label": "val", "to": "__temp4"}),
                    json!({"editType": "addEdge", "from": "__temp1", "label": "next", "to": "__temp3"}),
                ],
                10,
            ),
        ],
        vis_graph: base_vis_graph(),
    };

    let artifacts = synthesize_core(request, SynthesisCoreOptions::default())
        .expect("multi-trace core synthesis should succeed");

    assert!(artifacts.task_json.is_some() || artifacts.spec_json.is_some());
    assert!(artifacts.response.list_environment_info.is_some());
    assert!(artifacts.response.operation_analysis.is_some());
}
