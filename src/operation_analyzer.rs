//! 操作の解析とList環境の生成を行うモジュール
//!
//! 複数の操作列を受け取り、同型解析で共通部分と差異部分を特定し、
//! 各差異部分の直前でのList環境を計算する。

use crate::list_env::{GraphOperation, ListEnvironment};
use crate::models::VisGraph;
use serde_json::Value;

#[derive(Debug, Clone)]
pub struct DifferencePoint {
    pub position: usize,
    pub operation_a: Option<GraphOperation>,
    pub operation_b: Option<GraphOperation>,
    pub environment_before: ListEnvironment,
}

#[derive(Debug)]
pub struct OperationAnalysisResult {
    pub common_operations_count: usize,
    pub difference_points: Vec<DifferencePoint>,
    pub total_operations_a: usize,
    pub total_operations_b: usize,
}

/// 操作列を解析して差異点とその直前の環境を特定する
pub fn analyze_operations_with_environments(
    vis_graph: &VisGraph,
    operations_a: &[Value],
    operations_b: &[Value],
) -> Result<OperationAnalysisResult, String> {
    // GraphOperationに変換
    let graph_ops_a: Result<Vec<GraphOperation>, _> = operations_a
        .iter()
        .map(|op| serde_json::from_value(op.clone()))
        .collect();

    let graph_ops_b: Result<Vec<GraphOperation>, _> = operations_b
        .iter()
        .map(|op| serde_json::from_value(op.clone()))
        .collect();

    let ops_a = graph_ops_a.map_err(|e| format!("Failed to parse operations A: {}", e))?;
    let ops_b = graph_ops_b.map_err(|e| format!("Failed to parse operations B: {}", e))?;

    let mut difference_points = Vec::new();
    let mut current_env = ListEnvironment::from_vis_graph(vis_graph);
    let mut position = 0;
    let max_len = ops_a.len().max(ops_b.len());

    // 操作を一つずつ比較しながら環境を更新
    while position < max_len {
        let op_a = ops_a.get(position);
        let op_b = ops_b.get(position);

        // 操作が同じかどうかチェック
        let operations_match = match (op_a, op_b) {
            (Some(a), Some(b)) => operations_equivalent(a, b),
            (None, None) => true,
            _ => false, // 片方だけある場合は差異
        };

        if !operations_match {
            // 差異発見 - 現在の環境をコピーして記録
            difference_points.push(DifferencePoint {
                position,
                operation_a: op_a.cloned(),
                operation_b: op_b.cloned(),
                environment_before: current_env.clone(),
            });
        }

        // 両方の操作が存在し、かつ同じ場合のみ環境を更新
        if let (Some(a), Some(b)) = (op_a, op_b) {
            if operations_equivalent(a, b) {
                // 共通操作として適用（どちらでも同じなのでAを使用）
                current_env.apply_operation(a).map_err(|e| {
                    format!("Failed to apply operation at position {}: {}", position, e)
                })?;
            }
        }

        position += 1;
    }

    Ok(OperationAnalysisResult {
        common_operations_count: position - difference_points.len(),
        difference_points,
        total_operations_a: ops_a.len(),
        total_operations_b: ops_b.len(),
    })
}

/// 2つの操作が等価かどうかを判定
fn operations_equivalent(op_a: &GraphOperation, op_b: &GraphOperation) -> bool {
    op_a.edit_type == op_b.edit_type
        && op_a.id == op_b.id
        && op_a.label == op_b.label
        && op_a.is_literal == op_b.is_literal
        && op_a.from == op_b.from
        && op_a.to == op_b.to
        && op_a.old_to == op_b.old_to
        && op_a.new_to == op_b.new_to
}

/// 操作の詳細情報をフォーマットする
pub fn format_operation_details(op: &GraphOperation) -> String {
    let mut details = Vec::new();

    if let Some(id) = &op.id {
        details.push(format!("id: {}", id));
    }

    if let Some(label) = &op.label {
        details.push(format!("label: {}", label));
    }

    if let Some(from) = &op.from {
        details.push(format!("from: {}", from));
    }

    if let Some(to) = &op.to {
        details.push(format!("to: {}", to));
    }

    if let Some(old_to) = &op.old_to {
        details.push(format!("oldTo: {}", old_to));
    }

    if let Some(new_to) = &op.new_to {
        details.push(format!("newTo: {}", new_to));
    }

    if details.is_empty() {
        "詳細なし".to_string()
    } else {
        details.join(", ")
    }
}

/// 解析結果をターミナルで可視化
pub fn display_analysis_result(result: &OperationAnalysisResult) {
    println!("=== 操作解析結果 ===");
    println!("共通操作数: {}", result.common_operations_count);
    println!("操作列A の総数: {}", result.total_operations_a);
    println!("操作列B の総数: {}", result.total_operations_b);
    println!("差異点の数: {}", result.difference_points.len());
    println!();

    if result.difference_points.is_empty() {
        println!("✅ 両方の操作列は完全に一致しています。");
        return;
    }

    for (index, diff_point) in result.difference_points.iter().enumerate() {
        println!(
            "--- 差異点 {} (位置: {}) ---",
            index + 1,
            diff_point.position
        );

        // 操作の差異を表示
        match (&diff_point.operation_a, &diff_point.operation_b) {
            (Some(op_a), Some(op_b)) => {
                println!("🔄 操作の違い:");
                println!(
                    "  操作A: {} ({})",
                    op_a.edit_type,
                    format_operation_details(op_a)
                );
                println!(
                    "  操作B: {} ({})",
                    op_b.edit_type,
                    format_operation_details(op_b)
                );
            }
            (Some(op_a), None) => {
                println!(
                    "➕ 操作A のみ: {} ({})",
                    op_a.edit_type,
                    format_operation_details(op_a)
                );
                println!("  操作B: なし");
            }
            (None, Some(op_b)) => {
                println!("  操作A: なし");
                println!(
                    "➕ 操作B のみ: {} ({})",
                    op_b.edit_type,
                    format_operation_details(op_b)
                );
            }
            (None, None) => {
                println!("⚠️  両方とも操作なし（予期しない状態）");
            }
        }

        // この差異点直前のList環境を表示
        println!("📋 この差異点直前のList環境:");
        let env_display = diff_point.environment_before.to_debug_string();
        for line in env_display.lines() {
            println!("    {}", line);
        }
        println!();
    }
}

/// 簡単なテスト用ヘルパー関数
pub fn test_operation_analysis() -> Result<(), String> {
    use crate::models::{Edge, Node};
    use serde_json::json;

    // テスト用のグラフ
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

    // テスト用の操作列
    let operations_a = vec![
        json!({"editType": "addNode", "id": "__temp1", "label": "Node", "isLiteral": false}),
        json!({"editType": "addNode", "id": "__temp2", "label": "0", "isLiteral": true}),
        json!({"editType": "addEdge", "from": "__temp1", "to": "__temp2", "label": "val"}),
        json!({"editType": "addEdge", "from": "main-new1", "to": "__temp1", "label": "next"}),
    ];

    let operations_b = vec![
        json!({"editType": "addNode", "id": "__temp1", "label": "Node", "isLiteral": false}),
        json!({"editType": "addNode", "id": "__temp3", "label": "3", "isLiteral": true}), // 差異: ID と label
        json!({"editType": "addEdge", "from": "__temp1", "to": "__temp3", "label": "val"}), // 差異: to
        json!({"editType": "addEdge", "from": "main-new1", "to": "__temp1", "label": "next"}),
    ];

    let result = analyze_operations_with_environments(&vis_graph, &operations_a, &operations_b)?;
    display_analysis_result(&result);

    Ok(())
}
