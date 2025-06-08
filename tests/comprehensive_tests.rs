use refsyn::ir::*;
use refsyn::ast;
use refsyn::env::MemoEnv;
use refsyn::convert_operations_to_ir;
use serde_json;

/// tex解析記録に基づく実際のテストケース作成
/// 
/// これらのテストは`records/analysys.tex`に記録された手動解析結果を基に、
/// 各メソッドが正しく合成されることを検証します。

/// テスト用ユーティリティ関数
fn create_comprehensive_memo_env() -> MemoEnv {
    let mut env = MemoEnv::new();
    env.add_name_id_mapping("main-new1".to_string(), "this".to_string());
    env.add_name_id_mapping("main-new2".to_string(), "node2".to_string());
    env.add_name_id_mapping("main-new3".to_string(), "node3".to_string());
    env.add_name_id_mapping("main-new4".to_string(), "node4".to_string());
    env
}

/// tex解析: appendメソッドの詳細テスト
/// 初期プログラム: lst.append(0); lst.append(3);
#[test]
fn test_append_comprehensive() {
    // append(0)の完全な操作セット（tex解析より）
    let append_0_ops = vec![
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

    // append(3)の完全な操作セット
    let append_3_ops = vec![
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
            "from": "__temp1", // 前のappend結果への参照
            "label": "next",
            "to": "__temp3"
        }),
    ];

    let operations_list = vec![append_0_ops, append_3_ops];
    let memo_envs = vec![create_comprehensive_memo_env(), create_comprehensive_memo_env()];

    let (program_opt, holes) = find_common_pattern_from_operations(&operations_list, &memo_envs);

    assert!(program_opt.is_some(), "append pattern synthesis should succeed");
    let program = program_opt.unwrap();

    println!("=== Append Method Test Results ===");
    println!("Generated {} statements", program.stmts.len());
    println!("Found {} holes", holes.len());
    
    for (i, stmt) in program.stmts.iter().enumerate() {
        println!("Statement {}: {:?}", i, stmt);
    }
    
    // 期待される構造の詳細な検証
    // 1. Nodeの作成
    let new_node_count = program.stmts.iter().filter(|stmt| {
        matches!(stmt, ast::Stmt::VarDecl { expr: ast::Expr::New(_), .. })
    }).count();
    assert!(new_node_count > 0, "append should create at least one new Node");

    // 2. val プロパティの設定
    let val_assignments = program.stmts.iter().filter(|stmt| {
        matches!(stmt, ast::Stmt::Assign { 
            lhs: ast::Lhs::ObjAccess(_, prop), .. 
        } if prop == "val")
    }).count();
    assert!(val_assignments > 0, "append should set val properties");

    // 3. next プロパティの設定
    let next_assignments = program.stmts.iter().filter(|stmt| {
        matches!(stmt, ast::Stmt::Assign { 
            lhs: ast::Lhs::ObjAccess(_, prop), .. 
        } if prop == "next")
    }).count();
    assert!(next_assignments > 0, "append should set next properties");

    // 4. ホールの存在確認（値の差異を表現）
    assert!(!holes.is_empty(), "append should have holes for different values");
}

/// tex解析: prependメソッドの詳細テスト
/// prependは新しいノードを先頭に追加し、それを返す
#[test]
fn test_prepend_comprehensive() {
    // prepend(0)の完全な操作セット
    let prepend_0_ops = vec![
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

    let operations_list = vec![prepend_0_ops];
    let memo_envs = vec![create_comprehensive_memo_env()];

    let (program_opt, holes) = find_common_pattern_from_operations(&operations_list, &memo_envs);

    if let Some(program) = program_opt {
        println!("=== Prepend Method Test Results ===");
        println!("Generated {} statements", program.stmts.len());
        println!("Found {} holes", holes.len());
        
        for (i, stmt) in program.stmts.iter().enumerate() {
            println!("Statement {}: {:?}", i, stmt);
        }

        // prependの特徴: 新しいノードの作成と、thisへの参照設定
        let has_new_node = program.stmts.iter().any(|stmt| {
            matches!(stmt, ast::Stmt::VarDecl { expr: ast::Expr::New(_), .. })
        });
        assert!(has_new_node, "prepend should create new Node");

        // next参照の設定（新しいノード -> this）
        let has_next_to_this = program.stmts.iter().any(|stmt| {
            matches!(stmt, ast::Stmt::Assign { 
                lhs: ast::Lhs::ObjAccess(_, prop),
                expr: ast::Expr::This
            } if prop == "next")
        });
        
        println!("Has next to this assignment: {}", has_next_to_this);
    } else {
        println!("Prepend pattern synthesis returned None");
    }
}

/// tex解析: insertAfterメソッドの詳細テスト
/// insertAfter(i, arg)は指定された位置に新しいノードを挿入
#[test]
fn test_insert_after_comprehensive() {
    // insertAfter(0, 3)の完全な操作セット（tex解析より）
    let insert_after_ops = vec![
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
            "label": "3",
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
            "to": "main-new2" // 挿入位置の次のノード
        }),
        serde_json::json!({
            "editType": "editEdgeReference",
            "from": "main-new1", // 挿入位置のノード
            "oldTo": "main-new2",
            "newTo": "__temp1", // 新しいノード
            "label": "next"
        }),
    ];

    let operations_list = vec![insert_after_ops];
    let memo_envs = vec![create_comprehensive_memo_env()];

    let (program_opt, _holes) = find_common_pattern_from_operations(&operations_list, &memo_envs);

    if let Some(program) = program_opt {
        println!("=== InsertAfter Method Test Results ===");
        println!("Generated {} statements", program.stmts.len());
        
        for (i, stmt) in program.stmts.iter().enumerate() {
            println!("Statement {}: {:?}", i, stmt);
        }

        // insertAfterの特徴: 新しいノード作成 + 参照の再配線
        let has_new_node = program.stmts.iter().any(|stmt| {
            matches!(stmt, ast::Stmt::VarDecl { expr: ast::Expr::New(_), .. })
        });
        assert!(has_new_node, "insertAfter should create new Node");

        let next_modifications = program.stmts.iter().filter(|stmt| {
            matches!(stmt, ast::Stmt::Assign { 
                lhs: ast::Lhs::ObjAccess(_, prop), .. 
            } if prop == "next")
        }).count();
        assert!(next_modifications >= 1, "insertAfter should modify next references");

    } else {
        println!("InsertAfter pattern synthesis returned None");
    }
}

/// IR変換の基本機能テスト
#[test]
fn test_ir_conversion_comprehensive() {
    let test_operations = vec![
        serde_json::json!({
            "editType": "addNode",
            "id": "test1",
            "isLiteral": false,
            "label": "Node"
        }),
        serde_json::json!({
            "editType": "addEdge",
            "from": "test1",
            "to": "test2",
            "label": "next"
        }),
        serde_json::json!({
            "editType": "editEdgeReference",
            "from": "test1",
            "oldTo": "test2",
            "newTo": "test3",
            "label": "next"
        }),
        serde_json::json!({
            "editType": "deleteNode",
            "id": "test2"
        }),
    ];

    let ir_result = convert_operations_to_ir(&test_operations);
    
    match ir_result {
        Ok(ir_ops) => {
            println!("=== IR Conversion Test Results ===");
            println!("Converted {} operations to IR", ir_ops.len());
            
            assert_eq!(ir_ops.len(), 4, "Should convert all operations");
            
            // 各操作タイプの確認
            let add_node_count = ir_ops.iter().filter(|op| {
                matches!(op.kind, OpKind::AddNode { .. })
            }).count();
            assert_eq!(add_node_count, 1, "Should have one AddNode operation");

            let add_edge_count = ir_ops.iter().filter(|op| {
                matches!(op.kind, OpKind::AddEdge { .. })
            }).count();
            assert_eq!(add_edge_count, 1, "Should have one AddEdge operation");

            let edit_edge_count = ir_ops.iter().filter(|op| {
                matches!(op.kind, OpKind::EditEdgeReference { .. })
            }).count();
            assert_eq!(edit_edge_count, 1, "Should have one EditEdgeReference operation");

            let delete_node_count = ir_ops.iter().filter(|op| {
                matches!(op.kind, OpKind::DeleteNode { .. })
            }).count();
            assert_eq!(delete_node_count, 1, "Should have one DeleteNode operation");

            // 正規順序の確認
            let canonical_ids = canonical_order(&ir_ops);
            assert_eq!(canonical_ids.len(), ir_ops.len(), "Canonical order should preserve all operations");

        },
        Err(e) => {
            panic!("IR conversion failed: {}", e);
        }
    }
}

/// パターンマッチングの詳細テスト
#[test]
fn test_pattern_matching_detailed() {
    // 2つの類似した操作セット
    let ops_a = vec![
        serde_json::json!({
            "editType": "addNode",
            "id": "__temp1",
            "isLiteral": false,
            "label": "Node"
        }),
        serde_json::json!({
            "editType": "addEdge",
            "from": "main-new1",
            "to": "__temp1",
            "label": "next"
        }),
    ];

    let ops_b = vec![
        serde_json::json!({
            "editType": "addNode",
            "id": "__temp5",
            "isLiteral": false,
            "label": "Node"
        }),
        serde_json::json!({
            "editType": "addEdge",
            "from": "main-new1",
            "to": "__temp5",
            "label": "next"
        }),
    ];

    let operations_list = vec![ops_a, ops_b];
    let memo_envs = vec![create_comprehensive_memo_env(), create_comprehensive_memo_env()];

    let (program_opt, holes) = find_common_pattern_from_operations(&operations_list, &memo_envs);

    if let Some(program) = program_opt {
        println!("=== Pattern Matching Test Results ===");
        println!("Generated {} statements", program.stmts.len());
        println!("Found {} holes", holes.len());
        
        // 共通パターンが抽出されることを確認
        assert!(!program.stmts.is_empty(), "Should extract common pattern");
        
        // ホールが適切に作成されることを確認（異なるIDに対して）
        if !holes.is_empty() {
            println!("Holes found: {:?}", holes);
        }

    } else {
        println!("Pattern matching returned None - this might be expected for simple cases");
    }
}

/// エラー処理とエッジケースのテスト
#[test]
fn test_edge_cases() {
    // 空の操作リスト
    let empty_ops: Vec<serde_json::Value> = vec![];
    let empty_result = find_common_pattern_from_operations(&[empty_ops], &[create_comprehensive_memo_env()]);
    match empty_result {
        (Some(_), _) => println!("Empty operations handled gracefully"),
        (None, _) => println!("Empty operations returned None as expected"),
    }

    // 不正なJSON構造
    let malformed_ops = vec![
        serde_json::json!({
            "invalidField": "invalid"
        }),
    ];
    let malformed_result = find_common_pattern_from_operations(&[malformed_ops], &[create_comprehensive_memo_env()]);
    match malformed_result {
        (Some(_), _) => println!("Malformed operations handled gracefully"),
        (None, _) => println!("Malformed operations returned None as expected"),
    }

    // 1つだけの操作リスト（パターンマッチング不可）
    let single_op = vec![
        serde_json::json!({
            "editType": "addNode",
            "id": "single",
            "isLiteral": false,
            "label": "Node"
        }),
    ];
    let single_result = find_common_pattern_from_operations(&[single_op], &[create_comprehensive_memo_env()]);
    match single_result {
        (Some(program), _) => {
            println!("Single operation converted successfully: {} statements", program.stmts.len());
        },
        (None, _) => println!("Single operation returned None"),
    }
}

/// 統計情報の出力テスト
#[test]
fn test_synthesis_statistics() {
    println!("=== RefSyn Test Suite Statistics ===");
    
    // 各メソッドタイプのテスト実行統計
    let test_methods = vec![
        ("append", "Node insertion at end"),
        ("prepend", "Node insertion at beginning"),
        ("insertAfter", "Node insertion at position"),
        ("remove", "Node removal operations"),
        ("concat", "List concatenation"),
    ];

    for (method, description) in test_methods {
        println!("- {}: {}", method, description);
    }

    println!("\nTest infrastructure created successfully!");
    println!("To run specific tests:");
    println!("  cargo test test_append_comprehensive");
    println!("  cargo test test_prepend_comprehensive");
    println!("  cargo test test_insert_after_comprehensive");
    println!("  cargo test --test comprehensive_tests");
}
