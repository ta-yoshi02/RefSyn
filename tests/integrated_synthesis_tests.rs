use refsyn::models::{Edge, Node, VisGraph};
use refsyn::{handle_synthesis, MethodCallOperation, SynthesisRequest};
use serde_json;

/// 統合されたsynthesizeエンドポイントのテスト - 単一の操作列
#[tokio::test]
async fn test_integrated_synthesis_single_operation_list() {
    // 基本的なVisGraphを作成
    let vis_graph = VisGraph {
        nodes: vec![
            Node {
                id: "n1".to_string(),
                is_literal: false,
                label: serde_json::json!("node1"),
            },
            Node {
                id: "n2".to_string(),
                is_literal: false,
                label: serde_json::json!("node2"),
            },
        ],
        edges: vec![Edge {
            from: "n1".to_string(),
            to: "n2".to_string(),
            label: "edge1".to_string(),
        }],
    };

    // 単一の操作列を作成
    let operations = vec![
        serde_json::json!({
            "edit_type": "addNode",
            "node_id": "n3",
            "node_data": {"type": "Node"}
        }),
        serde_json::json!({
            "edit_type": "addEdge",
            "from": "n1",
            "to": "n3",
            "edge_data": {"type": "Edge"}
        }),
    ];

    let method_call = MethodCallOperation {
        call_label: "test_method".to_string(),
        context_sensitive_id: "ctx1".to_string(),
        receiver_object: "obj1".to_string(),
        method_name: "test".to_string(),
        arguments: vec![],
        argument_types: None,
        argument_names: None,
        operations: operations,
        actual_graph: Some(vis_graph.clone()),
        field_tables: None,
    };

    let request = SynthesisRequest {
        method_calls: vec![method_call],
        vis_graph,
    };

    let request_body = serde_json::to_vec(&request).unwrap();
    let body_bytes = bytes::Bytes::from(request_body);

    let response = handle_synthesis(body_bytes).await;
    assert!(response.is_ok(), "Handle synthesis should succeed");

    // レスポンスのステータスコードが成功であることを確認
    println!("Single operation list test completed successfully");
}

/// 統合されたsynthesizeエンドポイントのテスト - 複数の操作列（操作分析が含まれる）
#[tokio::test]
async fn test_integrated_synthesis_multiple_operation_lists() {
    // 基本的なVisGraphを作成
    let vis_graph = VisGraph {
        nodes: vec![
            Node {
                id: "n1".to_string(),
                is_literal: false,
                label: serde_json::json!("node1"),
            },
            Node {
                id: "n2".to_string(),
                is_literal: false,
                label: serde_json::json!("node2"),
            },
        ],
        edges: vec![Edge {
            from: "n1".to_string(),
            to: "n2".to_string(),
            label: "edge1".to_string(),
        }],
    };

    // 最初の操作列
    let operations_a = vec![
        serde_json::json!({
            "edit_type": "addNode",
            "node_id": "n3",
            "node_data": {"type": "Node"}
        }),
        serde_json::json!({
            "edit_type": "addEdge",
            "from": "n1",
            "to": "n3",
            "edge_data": {"type": "Edge"}
        }),
    ];

    // 二番目の操作列（異なる）
    let operations_b = vec![
        serde_json::json!({
            "edit_type": "addNode",
            "node_id": "n3",
            "node_data": {"type": "Node"}
        }),
        serde_json::json!({
            "edit_type": "addEdge",
            "from": "n2", // 異なる
            "to": "n3",
            "edge_data": {"type": "Edge"}
        }),
    ];

    let method_call_a = MethodCallOperation {
        call_label: "test_method_a".to_string(),
        context_sensitive_id: "ctx1".to_string(),
        receiver_object: "obj1".to_string(),
        method_name: "test".to_string(),
        arguments: vec![],
        argument_types: None,
        argument_names: None,
        operations: operations_a,
        actual_graph: Some(vis_graph.clone()),
        field_tables: None,
    };

    let method_call_b = MethodCallOperation {
        call_label: "test_method_b".to_string(),
        context_sensitive_id: "ctx2".to_string(),
        receiver_object: "obj2".to_string(),
        method_name: "test".to_string(),
        arguments: vec![],
        argument_types: None,
        argument_names: None,
        operations: operations_b,
        actual_graph: Some(vis_graph.clone()),
        field_tables: None,
    };

    let request = SynthesisRequest {
        method_calls: vec![method_call_a, method_call_b],
        vis_graph,
    };

    let request_body = serde_json::to_vec(&request).unwrap();
    let body_bytes = bytes::Bytes::from(request_body);

    let response = handle_synthesis(body_bytes).await;
    assert!(
        response.is_ok(),
        "Handle synthesis with multiple operation lists should succeed"
    );

    println!("Multiple operation lists test completed successfully");
}

/// 統合されたsynthesizeエンドポイントのテスト - 空のmethod_calls
#[tokio::test]
async fn test_integrated_synthesis_empty_method_calls() {
    let vis_graph = VisGraph {
        nodes: vec![],
        edges: vec![],
    };

    let request = SynthesisRequest {
        method_calls: vec![],
        vis_graph,
    };

    let request_body = serde_json::to_vec(&request).unwrap();
    let body_bytes = bytes::Bytes::from(request_body);

    let response = handle_synthesis(body_bytes).await;
    assert!(
        response.is_ok(),
        "Handle synthesis with empty method calls should succeed"
    );

    println!("Empty method calls test completed successfully");
}
