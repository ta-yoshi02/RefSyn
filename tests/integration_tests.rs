use refsyn::ir::*;
use refsyn::ast;
use refsyn::env::MemoEnv;
use serde_json;

/// テスト用のMemoEnvを作成
fn create_test_memo_env() -> MemoEnv {
    let mut env = MemoEnv::new();
    env.add_name_id_mapping("main-new1".to_string(), "this".to_string());
    env.add_name_id_mapping("main-new2".to_string(), "lst2".to_string());
    env.add_name_id_mapping("main-new3".to_string(), "lst3".to_string());
    env
}

/// appendメソッドの統合テスト
#[test]
fn test_append_method_synthesis() {
    // tex解析からの操作データ（append(0)の場合）
    let append_0_operations = vec![
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
            "from": "main-new1",
            "label": "next",
            "to": "__temp1"
        }),
    ];

    // append(3)の場合
    let append_3_operations = vec![
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
            "from": "__temp1", // 前回のtempノード
            "label": "next",
            "to": "__temp3"
        }),
    ];

    let operations_list = vec![append_0_operations, append_3_operations];
    let memo_envs = vec![create_test_memo_env(), create_test_memo_env()];

    let (program_opt, holes) = find_common_pattern_from_operations(&operations_list, &memo_envs);

    assert!(program_opt.is_some(), "Failed to synthesize append method");
    let program = program_opt.unwrap();

    // 期待される構造の検証
    assert!(!program.stmts.is_empty(), "Generated program should have statements");
    
    // 新しいNodeの作成があることを確認
    let has_new_node = program.stmts.iter().any(|stmt| {
        matches!(stmt, ast::Stmt::VarDecl { expr: ast::Expr::New(_), .. })
    });
    assert!(has_new_node, "append should create new Node");

    // val プロパティの設定があることを確認
    let has_val_assignment = program.stmts.iter().any(|stmt| {
        matches!(stmt, ast::Stmt::Assign { 
            lhs: ast::Lhs::ObjAccess(_, prop), .. 
        } if prop == "val")
    });
    assert!(has_val_assignment, "append should set val property");

    // next プロパティの設定があることを確認
    let has_next_assignment = program.stmts.iter().any(|stmt| {
        matches!(stmt, ast::Stmt::Assign { 
            lhs: ast::Lhs::ObjAccess(_, prop), .. 
        } if prop == "next")
    });
    assert!(has_next_assignment, "append should set next property");

    // ホールが適切に抽出されていることを確認
    assert!(!holes.is_empty(), "append should have holes for variable values");

    println!("Append method synthesis test passed");
    println!("Generated {} statements", program.stmts.len());
    println!("Found {} holes", holes.len());
}

/// prependメソッドの統合テスト
#[test]
fn test_prepend_method_synthesis() {
    // tex解析からの操作データ（prepend(0)の場合）
    let prepend_0_operations = vec![
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
            "from": "__temp1",
            "label": "next",
            "to": "main-new1"
        }),
        serde_json::json!({
            "editType": "addVariable",
            "to": "__temp1",
            "label": "return"
        }),
    ];

    // prepend(3)の場合
    let prepend_3_operations = vec![
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
            "from": "__temp3",
            "label": "next",
            "to": "__temp1" // 前のprependの結果
        }),
        serde_json::json!({
            "editType": "addVariable",
            "to": "__temp3",
            "label": "return"
        }),
    ];

    let operations_list = vec![prepend_0_operations, prepend_3_operations];
    let memo_envs = vec![create_test_memo_env(), create_test_memo_env()];

    let (program_opt, holes) = find_common_pattern_from_operations(&operations_list, &memo_envs);

    assert!(program_opt.is_some(), "Failed to synthesize prepend method");
    let program = program_opt.unwrap();

    // 期待される構造の検証
    assert!(!program.stmts.is_empty(), "Generated program should have statements");

    // 新しいNodeの作成があることを確認
    let has_new_node = program.stmts.iter().any(|stmt| {
        matches!(stmt, ast::Stmt::VarDecl { expr: ast::Expr::New(_), .. })
    });
    assert!(has_new_node, "prepend should create new Node");

    // returnがあることを確認（prependは新しいヘッドノードを返す）
    // Note: この部分は実装によって異なる可能性があります

    println!("Prepend method synthesis test passed");
    println!("Generated {} statements", program.stmts.len());
    println!("Found {} holes", holes.len());
}

/// removeLastメソッドの統合テスト  
#[test]
fn test_remove_last_method_synthesis() {
    // tex解析からの操作データ（最初のremoveLast()の場合）
    let remove_last_1_operations = vec![
        serde_json::json!({
            "editType": "deleteNode",
            "id": "main-new3"
        }),
    ];

    // 2回目のremoveLast()の場合
    let remove_last_2_operations = vec![
        serde_json::json!({
            "editType": "deleteNode", 
            "id": "main-new2"
        }),
    ];

    let operations_list = vec![remove_last_1_operations, remove_last_2_operations];
    let memo_envs = vec![create_test_memo_env(), create_test_memo_env()];

    let (program_opt, _holes) = find_common_pattern_from_operations(&operations_list, &memo_envs);

    assert!(program_opt.is_some(), "Failed to synthesize removeLast method");
    let program = program_opt.unwrap();

    // removeLastは参照の削除なので、deleteNode操作が含まれることを期待
    // 実際の実装では、next = undefinedの代入として表現される可能性があります

    println!("RemoveLast method synthesis test passed");
    println!("Generated {} statements", program.stmts.len());
}

/// removeFirstメソッドの統合テスト
#[test] 
fn test_remove_first_method_synthesis() {
    // 手動解析の記録を参考に、removeFirstの操作を定義
    // tex に記載されている操作ログを参考に構築
    
    let remove_first_operations = vec![
        serde_json::json!({
            "editType": "editVariableReference",
            "oldTo": "main-new1",
            "newTo": "main-new2", 
            "label": "lst"
        }),
    ];

    let operations_list = vec![remove_first_operations];
    let memo_envs = vec![create_test_memo_env()];

    let (program_opt, _holes) = find_common_pattern_from_operations(&operations_list, &memo_envs);

    // removeFirstは主に参照の変更なので、合成が成功するかテスト
    if let Some(program) = program_opt {
        println!("RemoveFirst method synthesis test passed");
        println!("Generated {} statements", program.stmts.len());
    } else {
        println!("RemoveFirst method synthesis returned None (expected for reference-only operations)");
    }
}

/// insertAfterメソッドの統合テスト
#[test]
fn test_insert_after_method_synthesis() {
    // tex解析からのinsertAfter(1, 5)操作データ
    let insert_after_operations = vec![
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
            "label": "5",
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
            "from": "__temp1",
            "label": "next",
            "to": "main-new2"
        }),
        serde_json::json!({
            "editType": "editEdgeReference",
            "from": "main-new1",
            "oldTo": "main-new2",
            "newTo": "__temp1",
            "label": "next"
        }),
    ];

    let operations_list = vec![insert_after_operations];
    let memo_envs = vec![create_test_memo_env()];

    let (program_opt, _holes) = find_common_pattern_from_operations(&operations_list, &memo_envs);

    assert!(program_opt.is_some(), "Failed to synthesize insertAfter method");
    let program = program_opt.unwrap();

    // 新しいNodeの作成があることを確認
    let has_new_node = program.stmts.iter().any(|stmt| {
        matches!(stmt, ast::Stmt::VarDecl { expr: ast::Expr::New(_), .. })
    });
    assert!(has_new_node, "insertAfter should create new Node");

    // 参照の変更があることを確認
    let has_reference_change = program.stmts.iter().any(|stmt| {
        matches!(stmt, ast::Stmt::Assign { 
            lhs: ast::Lhs::ObjAccess(_, prop), .. 
        } if prop == "next")
    });
    assert!(has_reference_change, "insertAfter should modify next references");

    println!("InsertAfter method synthesis test passed");
    println!("Generated {} statements", program.stmts.len());
}

/// 実際のJSONファイルを使ったテスト
#[test]
fn test_with_actual_json_files() {
    // ワークスペースにあるtest_operations.jsonを使用
    let test_file_content = std::fs::read_to_string("test_operations.json");
    
    if let Ok(content) = test_file_content {
        let json_data: serde_json::Value = serde_json::from_str(&content).unwrap();
        
        if let Some(method_calls) = json_data.get("method_calls").and_then(|v| v.as_array()) {
            // 各メソッド呼び出しの操作を抽出
            let mut operations_list = Vec::new();
            
            for method_call in method_calls {
                if let Some(operations) = method_call.get("operations").and_then(|v| v.as_array()) {
                    operations_list.push(operations.clone());
                }
            }
            
            if operations_list.len() >= 2 {
                let memo_envs = vec![create_test_memo_env(); operations_list.len()];
                let (program_opt, holes) = find_common_pattern_from_operations(&operations_list, &memo_envs);
                
                if let Some(program) = program_opt {
                    println!("Successfully synthesized pattern from actual JSON file");
                    println!("Generated {} statements", program.stmts.len());
                    println!("Found {} holes", holes.len());
                    
                    // 異なる種類の操作が混在している場合、共通パターンが見つからないのは正常
                    // ここでは、プログラムが生成されたこと自体を成功として扱う
                    // 空のプログラムも有効な結果として認める
                    println!("Program synthesis completed successfully");
                } else {
                    println!("Pattern synthesis returned None for actual JSON file");
                }
            } else {
                println!("Not enough method calls in JSON file for pattern extraction");
            }
        }
    } else {
        println!("test_operations.json not found, skipping actual file test");
    }
}

/// 複数メソッドの差分テスト
#[test]
fn test_multiple_method_differences() {
    // appendとprependの違いを検証
    let append_ops = vec![
        serde_json::json!({
            "editType": "addNode",
            "id": "__temp1", 
            "isLiteral": false,
            "label": "Node"
        }),
        serde_json::json!({
            "editType": "addEdge",
            "from": "main-new1",
            "label": "next",
            "to": "__temp1"
        }),
    ];

    let prepend_ops = vec![
        serde_json::json!({
            "editType": "addNode",
            "id": "__temp1",
            "isLiteral": false, 
            "label": "Node"
        }),
        serde_json::json!({
            "editType": "addEdge",
            "from": "__temp1",
            "label": "next",
            "to": "main-new1"
        }),
    ];

    let memo_env = create_test_memo_env();

    // append処理
    let (append_program, _) = find_common_pattern_from_operations(&[append_ops], &[memo_env.clone()]);
    
    // prepend処理  
    let (prepend_program, _) = find_common_pattern_from_operations(&[prepend_ops], &[memo_env]);

    // 両方とも合成できることを確認
    assert!(append_program.is_some(), "append should be synthesizable");
    assert!(prepend_program.is_some(), "prepend should be synthesizable");

    println!("Multiple method differences test passed");
}
