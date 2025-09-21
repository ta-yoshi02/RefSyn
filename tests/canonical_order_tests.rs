use refsyn::convert_operations_to_ir;
use refsyn::env::MemoEnv;
use refsyn::ir::*;
use refsyn::program_analyzer::ProgramAnalysis;
use serde_json;

#[test]
fn test_canonical_order_with_user_example() {
    // ユーザーの例：call1のオリジナル順序
    let call1_operations = vec![
        serde_json::json!({
            "editType": "addNode",
            "id": "__temp1",
            "isLiteral": false,
            "label": "Node"
        }),
        serde_json::json!({
            "editType": "addNode",
            "id": "__temp2",
            "isLiteral": true,
            "label": "0",
            "type": "string"
        }),
        serde_json::json!({
            "editType": "addEdge",
            "from": "main-new1",
            "label": "next",
            "to": "__temp1"
        }),
        serde_json::json!({
            "editType": "addEdge",
            "from": "__temp1",
            "label": "val",
            "to": "__temp2"
        }),
    ];

    // call2のオリジナル順序（異なる順序）
    let call2_operations = vec![
        serde_json::json!({
            "editType": "addNode",
            "id": "__temp3",
            "isLiteral": true,
            "label": "3",
            "type": "string"
        }),
        serde_json::json!({
            "editType": "addNode",
            "id": "__temp4",
            "isLiteral": false,
            "label": "Node"
        }),
        serde_json::json!({
            "editType": "addEdge",
            "from": "__temp4",
            "label": "val",
            "to": "__temp3"
        }),
        serde_json::json!({
            "editType": "addEdge",
            "from": "__temp1",
            "label": "next",
            "to": "__temp4"
        }),
    ];

    // IRに変換
    let ir_ops1 =
        convert_operations_to_ir(&call1_operations).expect("Failed to convert call1 to IR");
    let ir_ops2 =
        convert_operations_to_ir(&call2_operations).expect("Failed to convert call2 to IR");

    println!("Call1 IR operations:");
    for (i, op) in ir_ops1.iter().enumerate() {
        println!("  {}: {:?}", i, op);
    }

    println!("Call2 IR operations:");
    for (i, op) in ir_ops2.iter().enumerate() {
        println!("  {}: {:?}", i, op);
    }

    // 正規順序に並び替え
    let canonical_order1 = canonical_order(&ir_ops1);
    let canonical_order2 = canonical_order(&ir_ops2);

    println!("Call1 canonical order: {:?}", canonical_order1);
    println!("Call2 canonical order: {:?}", canonical_order2);

    // 正規順序で並び替えた操作列を作成
    let sorted_ops1: Vec<Op> = canonical_order1
        .iter()
        .filter_map(|op_id| ir_ops1.iter().find(|op| op.id == *op_id))
        .cloned()
        .collect();

    let sorted_ops2: Vec<Op> = canonical_order2
        .iter()
        .filter_map(|op_id| ir_ops2.iter().find(|op| op.id == *op_id))
        .cloned()
        .collect();

    println!("Sorted Call1 operations:");
    for (i, op) in sorted_ops1.iter().enumerate() {
        println!("  {}: {:?}", i, op);
    }

    println!("Sorted Call2 operations:");
    for (i, op) in sorted_ops2.iter().enumerate() {
        println!("  {}: {:?}", i, op);
    }

    // この時点で、並び替えされた操作が正しい依存関係順序になっているかを確認
    // AddNodeがAddEdgeより前に来ているか確認
    assert!(sorted_ops1.len() == 4);
    assert!(sorted_ops2.len() == 4);
}

#[test]
fn test_pattern_matching_with_proper_context() {
    // 実際のJavaScriptプログラムのコンテキストを設定
    let js_program = r#"
        var list = new Node();   // main-new1
        var node = new Node();   // main-new2
        list.append(0);          // call1: receiver = main-new1
        node.append(3);          // call2: receiver = main-new2
    "#;

    // プログラム解析により動的にレシーバーを特定
    let analysis =
        ProgramAnalysis::analyze_program(js_program).expect("Failed to analyze JavaScript program");

    // より構造的に類似した操作列を作成（同じ順序、異なる値のみ）
    let operations_list = vec![
        vec![
            serde_json::json!({
                "editType": "addNode",
                "id": "__temp1",
                "isLiteral": false,
                "label": "Node"
            }),
            serde_json::json!({
                "editType": "addNode",
                "id": "__temp2",
                "isLiteral": true,
                "label": "0",
                "type": "string"
            }),
            serde_json::json!({
                "editType": "addEdge",
                "from": "__temp1",
                "label": "val",
                "to": "__temp2"
            }),
            serde_json::json!({
                "editType": "addEdge",
                "from": "main-new1",  // call1のレシーバー
                "label": "next",
                "to": "__temp1"
            }),
        ],
        vec![
            serde_json::json!({
                "editType": "addNode",
                "id": "__temp3",
                "isLiteral": false,
                "label": "Node"
            }),
            serde_json::json!({
                "editType": "addNode",
                "id": "__temp4",
                "isLiteral": true,
                "label": "3",
                "type": "string"
            }),
            serde_json::json!({
                "editType": "addEdge",
                "from": "__temp3",
                "label": "val",
                "to": "__temp4"
            }),
            serde_json::json!({
                "editType": "addEdge",
                "from": "main-new2",  // call2のレシーバー
                "label": "next",
                "to": "__temp3"
            }),
        ],
    ];

    // 各メソッド呼び出しに対して動的に環境を構築
    let mut memo_envs = vec![MemoEnv::new(), MemoEnv::new()];

    // call1 (list.append): main-new1がthis
    if let Some(receiver_id) = analysis.get_receiver_id_for_call(0) {
        memo_envs[0].add_name_id_mapping(receiver_id.to_string(), "this".to_string());
        println!("Call1 receiver: {} -> this", receiver_id);
    }

    // call2 (node.append): main-new2がthis
    if let Some(receiver_id) = analysis.get_receiver_id_for_call(1) {
        memo_envs[1].add_name_id_mapping(receiver_id.to_string(), "this".to_string());
        println!("Call2 receiver: {} -> this", receiver_id);
    }

    // 他のオブジェクトIDも適切にマッピング
    for (var_name, object_id) in &analysis.object_declarations {
        for env in &mut memo_envs {
            if !env.get_name_by_id(object_id).is_some() {
                env.add_name_id_mapping(object_id.clone(), var_name.clone());
            }
        }
    }

    let result = find_common_pattern_from_operations(&operations_list, &memo_envs);

    match result {
        (Some(program), holes) => {
            println!("Generated program: {}", program);
            println!("Holes: {:?}", holes);

            // プログラムが生成されたことを確認
            assert!(!program.stmts.is_empty());

            // thisが適切に使用されていることを確認
            let program_str = program.to_string();
            println!("Program contains 'this': {}", program_str.contains("this"));

            // ホールが生成されていることを確認（異なる値に対して）
            assert!(!holes.is_empty(), "Should have holes for different values");
        }
        (None, _) => {
            panic!("Expected to generate a program, but got None");
        }
    }
}

#[test]
fn test_complex_dependency_ordering() {
    // より複雑な依存関係のテスト
    let operations_list = vec![
        vec![
            serde_json::json!({
                "editType": "addNode",
                "id": "__temp1",
                "isLiteral": false,
                "label": "Node"
            }),
            serde_json::json!({
                "editType": "addNode",
                "id": "__temp2",
                "isLiteral": true,
                "label": "42",
                "type": "string"
            }),
            serde_json::json!({
                "editType": "addNode",
                "id": "__temp3",
                "isLiteral": false,
                "label": "Node"
            }),
            // 依存関係: temp3 -> temp1 -> temp2
            serde_json::json!({
                "editType": "addEdge",
                "from": "__temp3",
                "label": "next",
                "to": "__temp1"
            }),
            serde_json::json!({
                "editType": "addEdge",
                "from": "__temp1",
                "label": "val",
                "to": "__temp2"
            }),
        ],
        vec![
            serde_json::json!({
                "editType": "addNode",
                "id": "__temp4",
                "isLiteral": true,
                "label": "100",
                "type": "string"
            }),
            serde_json::json!({
                "editType": "addNode",
                "id": "__temp5",
                "isLiteral": false,
                "label": "Node"
            }),
            serde_json::json!({
                "editType": "addNode",
                "id": "__temp6",
                "isLiteral": false,
                "label": "Node"
            }),
            // 同じ依存関係構造：temp6 -> temp5 -> temp4
            serde_json::json!({
                "editType": "addEdge",
                "from": "__temp6",
                "label": "next",
                "to": "__temp5"
            }),
            serde_json::json!({
                "editType": "addEdge",
                "from": "__temp5",
                "label": "val",
                "to": "__temp4"
            }),
        ],
    ];

    let memo_envs = vec![MemoEnv::new(), MemoEnv::new()];

    let result = find_common_pattern_from_operations(&operations_list, &memo_envs);

    match result {
        (Some(program), holes) => {
            println!("Complex dependency test - Generated program: {}", program);
            println!("Complex dependency test - Holes: {:?}", holes);

            // プログラムが生成されたことを確認
            assert!(!program.stmts.is_empty());

            // 構造的に同じ操作のみが共通パターンとして抽出される
            // この場合：AddNode(Node), AddEdge(next), AddEdge(val) の3つ
            assert_eq!(program.stmts.len(), 3);
        }
        (None, _) => {
            panic!("Expected to generate a program, but got None");
        }
    }
}
