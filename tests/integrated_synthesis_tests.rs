use refsyn::models::{Edge, Node, VisGraph};
use refsyn::{handle_synthesis, MethodCallOperation, SynthesisRequest, SynthesisResponse};
use serde_json::json;
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
        actual_graph: Some(actual_graph.clone()),
        field_tables: None,
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
    assert_eq!(response.common_pattern, None);
    assert_eq!(response.hole_information, None);
    assert_eq!(response.composed_method_code, None);
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
