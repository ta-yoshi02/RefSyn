use refsyn::operation_analyzer::{analyze_operations_with_environments, display_analysis_result};
use refsyn::models::{VisGraph, Node, Edge};
use serde_json::json;

fn main() {
    println!("🔍 RefSyn 操作解析デモ");
    println!("======================");

    // デモ用のグラフ設定
    let vis_graph = VisGraph {
        nodes: vec![
            Node { 
                id: "main-new1".to_string(), 
                is_literal: false, 
                label: json!("Node") 
            },
            Node { 
                id: "main-new1-val".to_string(), 
                is_literal: true, 
                label: json!("2") 
            },
        ],
        edges: vec![
            Edge { 
                from: "main-new1".to_string(), 
                to: "main-new1-val".to_string(), 
                label: "val".to_string() 
            },
        ],
    };

    // 2つの異なる操作列を定義
    let operations_a = vec![
        json!({"editType": "addNode", "id": "__temp1", "label": "Node", "isLiteral": false}),
        json!({"editType": "addNode", "id": "__temp2", "label": "0", "isLiteral": true}),
        json!({"editType": "addEdge", "from": "__temp1", "to": "__temp2", "label": "val"}),
        json!({"editType": "addEdge", "from": "main-new1", "to": "__temp1", "label": "next"}),
        json!({"editType": "addNode", "id": "__temp4", "label": "ExtraNode", "isLiteral": false}),
    ];

    let operations_b = vec![
        json!({"editType": "addNode", "id": "__temp1", "label": "Node", "isLiteral": false}),
        json!({"editType": "addNode", "id": "__temp3", "label": "3", "isLiteral": true}),
        json!({"editType": "addEdge", "from": "__temp1", "to": "__temp3", "label": "val"}),
        json!({"editType": "addEdge", "from": "main-new1", "to": "__temp1", "label": "next"}),
        json!({"editType": "addNode", "id": "__temp5", "label": "AnotherNode", "isLiteral": false}),
    ];

    println!("📊 操作列Aと操作列Bを解析中...");
    
    match analyze_operations_with_environments(&vis_graph, &operations_a, &operations_b) {
        Ok(result) => {
            display_analysis_result(&result);
            
            println!("\n💡 解析のポイント:");
            println!("- 差異点ごとにその直前のList環境が表示されています");
            println!("- List環境を見ることで、どの状態で差異が発生したかが分かります");
            println!("- これにより、操作の違いが環境に与える影響を分析できます");
        },
        Err(e) => {
            eprintln!("❌ 解析エラー: {}", e);
        }
    }
}
