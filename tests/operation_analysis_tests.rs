use refsyn::operation_analyzer::{
    analyze_operations_with_environments, display_analysis_result, test_operation_analysis,
};

#[test]
fn test_operation_analysis_basic() {
    // 基本的なテスト関数を実行
    match test_operation_analysis() {
        Ok(()) => {
            println!("✅ 操作解析テストが正常に完了しました");
        }
        Err(e) => {
            panic!("❌ 操作解析テストが失敗しました: {}", e);
        }
    }
}

#[test]
fn test_operation_analysis_detailed() {
    use refsyn::models::{Edge, Node, VisGraph};
    use serde_json::json;

    // より詳細なテストケース
    let vis_graph = VisGraph {
        nodes: vec![
            Node {
                id: "main-new1".to_string(),
                is_literal: false,
                label: json!("Node"),
            },
            Node {
                id: "main-new1-val".to_string(),
                is_literal: true,
                label: json!("2"),
            },
        ],
        edges: vec![Edge {
            from: "main-new1".to_string(),
            to: "main-new1-val".to_string(),
            label: "val".to_string(),
        }],
    };

    // 複数の差異点があるテストケース
    let operations_a = vec![
        json!({"editType": "addNode", "id": "__temp1", "label": "Node", "isLiteral": false}), // 共通
        json!({"editType": "addNode", "id": "__temp2", "label": "0", "isLiteral": true}), // 差異1: label
        json!({"editType": "addEdge", "from": "__temp1", "to": "__temp2", "label": "val"}), // 差異1: to
        json!({"editType": "addNode", "id": "__temp4", "label": "Node", "isLiteral": false}), // 差異2: id
        json!({"editType": "addEdge", "from": "main-new1", "to": "__temp1", "label": "next"}), // 共通
    ];

    let operations_b = vec![
        json!({"editType": "addNode", "id": "__temp1", "label": "Node", "isLiteral": false}), // 共通
        json!({"editType": "addNode", "id": "__temp3", "label": "3", "isLiteral": true}), // 差異1: id, label
        json!({"editType": "addEdge", "from": "__temp1", "to": "__temp3", "label": "val"}), // 差異1: to
        json!({"editType": "addNode", "id": "__temp5", "label": "Node", "isLiteral": false}), // 差異2: id
        json!({"editType": "addEdge", "from": "main-new1", "to": "__temp1", "label": "next"}), // 共通
    ];

    let result =
        analyze_operations_with_environments(&vis_graph, &operations_a, &operations_b).unwrap();

    println!("\n=== 詳細テストケースの結果 ===");
    display_analysis_result(&result);

    // 期待される結果の確認
    assert!(
        result.difference_points.len() >= 2,
        "少なくとも2つの差異点があるはずです"
    );
    assert_eq!(result.total_operations_a, 5);
    assert_eq!(result.total_operations_b, 5);
}

#[test]
fn test_operation_analysis_identical() {
    use refsyn::models::{Node, VisGraph};
    use serde_json::json;

    // 同一の操作列のテスト
    let vis_graph = VisGraph {
        nodes: vec![Node {
            id: "main-new1".to_string(),
            is_literal: false,
            label: json!("Node"),
        }],
        edges: vec![],
    };

    let operations = vec![
        json!({"editType": "addNode", "id": "__temp1", "label": "Node", "isLiteral": false}),
        json!({"editType": "addNode", "id": "__temp2", "label": "0", "isLiteral": true}),
    ];

    let result =
        analyze_operations_with_environments(&vis_graph, &operations, &operations).unwrap();

    println!("\n=== 同一操作列のテスト結果 ===");
    display_analysis_result(&result);

    // 差異がないことを確認
    assert_eq!(
        result.difference_points.len(),
        0,
        "同一の操作列なので差異はないはずです"
    );
    assert_eq!(result.common_operations_count, 2);
}
