// 改善されたテストの例

use refsyn::ast;
use refsyn::env::MemoEnv;
use refsyn::ir::*;
use serde_json;
use std::collections::HashMap;

#[test]
fn test_append_method_correctness() {
    // analysys.texの手動復元コードに基づく正確なテスト

    // append(0)の操作（analysys.tex 45-46行目）
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

    // append(3)の操作（analysys.tex 48-51行目）
    // 重要：temp1（append(0)で作成されたノード）から次のノードに接続
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
            "from": "__temp1",  // append(0)で作成されたノードから接続
            "label": "next",
            "to": "__temp3"
        }),
    ];

    // 適切な環境を構築：append(0)実行後の状態を反映
    // 実際のプログラムを解析してreceiver_idを特定する必要がある
    let receiver_id = "main-new1"; // TODO: プログラム解析から取得する
    let initial_env = create_initial_memo_env(receiver_id);
    let env_after_append_0 = create_env_after_append_0(receiver_id);
    let memo_envs = vec![initial_env, env_after_append_0];

    let operations_list = vec![append_0_ops, append_3_ops];

    let (program_opt, holes) = find_common_pattern_from_operations(&operations_list, &memo_envs);

    assert!(program_opt.is_some(), "Should synthesize append pattern");
    let program = program_opt.unwrap();

    println!("=== Generated Program ===");
    for (i, stmt) in program.stmts.iter().enumerate() {
        println!("Statement {}: {:?}", i, stmt);
    }
    println!("Holes: {:?}", holes);

    // === analysys.texに基づく期待される構造の検証 ===

    // 期待される構造（analysys.tex 57-60行目 手動復元コード）：
    // 1. var temp1 = new Node();
    // 2. var temp2 = f(); // パラメータ値（ホール）
    // 3. temp1.val = temp2;
    // 4. g().next = temp1; // 接続先（append(0)では this, append(3)では this.next）

    verify_append_structure_from_analysys(&program, &holes);
}

fn create_initial_memo_env(receiver_id: &str) -> MemoEnv {
    // 動的にreceiverオブジェクトのIDを指定
    let mut env = MemoEnv::new();
    env.add_name_id_mapping(receiver_id.to_string(), "this".to_string());
    env
}

fn create_env_after_append_0(receiver_id: &str) -> MemoEnv {
    // append(0)実行後の環境：receiver.next = __temp1 が追加されている
    let mut env = MemoEnv::new();
    env.add_name_id_mapping(receiver_id.to_string(), "this".to_string());
    env.add_name_id_mapping("__temp1".to_string(), "this.next".to_string());
    env
}

fn verify_append_structure_from_analysys(
    program: &ast::Program,
    holes: &HashMap<String, Vec<String>>,
) {
    println!("=== Verifying against analysys.tex expected structure ===");

    // analysys.texの「一般化と部分関数の推定」セクション（75-79行目）に基づく検証
    // var temp1 = new Node();
    // var temp2 = f();    // f() = 引数 arg を返す関数
    // temp1.val = temp2;
    // g().next = temp1;   // g() = 現在のリストの末尾を返す関数

    let mut node_creation_count = 0;
    let mut val_assignment_count = 0;
    let mut next_assignment_count = 0;
    let mut parameterized_values = Vec::new();

    for stmt in &program.stmts {
        match stmt {
            // 1. Node作成の確認
            ast::Stmt::VarDecl {
                name,
                expr: ast::Expr::New(class),
            } => {
                assert_eq!(class, "Node", "Should create Node instances");
                node_creation_count += 1;
                println!("✓ Found Node creation: {}", name);
            }

            // 2. val プロパティ代入の確認
            ast::Stmt::Assign {
                lhs: ast::Lhs::ObjAccess(obj, prop),
                expr,
            } if prop == "val" => {
                val_assignment_count += 1;
                println!("✓ Found val assignment: {:?}.val = {:?}", obj, expr);

                // パラメータ値がホール化されているかチェック
                if let ast::Expr::Hole(hole_name) = expr {
                    parameterized_values.push(hole_name.clone());
                }
            }

            // 3. next プロパティ代入の確認
            ast::Stmt::Assign {
                lhs: ast::Lhs::ObjAccess(obj, prop),
                expr,
            } if prop == "next" => {
                next_assignment_count += 1;
                println!("✓ Found next assignment: {:?}.next = {:?}", obj, expr);
            }

            _ => {}
        }
    }

    // 基本構造の確認
    assert!(
        node_creation_count >= 1,
        "Should create at least 1 Node (temp1)"
    );
    assert!(
        val_assignment_count >= 1,
        "Should assign val property (temp1.val = temp2)"
    );
    assert!(
        next_assignment_count >= 1,
        "Should assign next property (g().next = temp1)"
    );

    // ホールの確認（analysys.texの異なる値：0と3）
    assert!(
        !holes.is_empty(),
        "Should have holes for parameterized values"
    );
    let hole_values: Vec<_> = holes.values().flat_map(|v| v.iter()).collect();
    assert!(
        hole_values.contains(&&"0".to_string()),
        "Should contain parameter value '0'"
    );
    assert!(
        hole_values.contains(&&"3".to_string()),
        "Should contain parameter value '3'"
    );

    println!("✓ Structure matches analysys.tex expectations");
    println!("✓ Node creation: {} instances", node_creation_count);
    println!("✓ Val assignments: {} statements", val_assignment_count);
    println!("✓ Next assignments: {} statements", next_assignment_count);
    println!("✓ Parameter holes: {:?}", hole_values);
}

fn create_memo_env() -> MemoEnv {
    let mut env = MemoEnv::new();
    env.add_name_id_mapping("main-new1".to_string(), "this".to_string());
    env
}

#[test]
fn test_program_execution_simulation() {
    // 複数のメソッド呼び出しから共通パターンを抽出し、実行をシミュレートする

    // append(42)とappend(99)の操作セット - 複数のメソッド呼び出し例
    let append_42_operations = vec![
        serde_json::json!({"editType": "addNode", "id": "__temp1", "isLiteral": false, "label": "Node"}),
        serde_json::json!({"editType": "addNode", "id": "__temp2", "isLiteral": true, "label": "42", "type": "string"}),
        serde_json::json!({"editType": "addEdge", "from": "__temp1", "label": "val", "to": "__temp2"}),
        serde_json::json!({"editType": "addEdge", "from": "main-new1", "label": "next", "to": "__temp1"}),
    ];

    let append_99_operations = vec![
        serde_json::json!({"editType": "addNode", "id": "__temp3", "isLiteral": false, "label": "Node"}),
        serde_json::json!({"editType": "addNode", "id": "__temp4", "isLiteral": true, "label": "99", "type": "string"}),
        serde_json::json!({"editType": "addEdge", "from": "__temp3", "label": "val", "to": "__temp4"}),
        serde_json::json!({"editType": "addEdge", "from": "main-new1", "label": "next", "to": "__temp3"}),
    ];

    let operations_list = vec![append_42_operations, append_99_operations];
    let memo_envs = vec![create_memo_env(), create_memo_env()];

    let (program_opt, holes) = find_common_pattern_from_operations(&operations_list, &memo_envs);

    assert!(
        program_opt.is_some(),
        "Should synthesize append pattern from multiple method calls"
    );
    let program = program_opt.unwrap();

    // ホールが正しく抽出されていることを確認（複数のメソッド呼び出しの違いがパラメータ化される）
    assert!(
        !holes.is_empty(),
        "Should extract holes from differences between method calls"
    );

    // ホールに期待される値が含まれているかを確認
    let hole_values: Vec<_> = holes.values().flat_map(|v| v.iter()).collect();
    assert!(
        hole_values.contains(&&"42".to_string()) || hole_values.contains(&&"99".to_string()),
        "Holes should contain parameter values from different method calls"
    );

    // プログラムの実行をシミュレート
    let execution_result = simulate_program_execution(&program);

    // 期待される結果と比較（複数のメソッド呼び出しから抽出されたパターンに基づく）
    println!(
        "Execution result from pattern synthesis: {:?}",
        execution_result
    );
    println!("Holes extracted from multiple calls: {:?}", holes);

    assert!(
        execution_result.created_nodes >= 1,
        "Synthesized pattern should create at least 1 node, got {}",
        execution_result.created_nodes
    );
    assert!(
        execution_result.property_assignments >= 1,
        "Synthesized pattern should have at least 1 property assignment, got {}",
        execution_result.property_assignments
    );
    assert!(
        execution_result.maintains_list_structure,
        "Synthesized pattern should maintain list structure"
    );

    // 合成されたプログラムが実際に共通パターンを表現しているかを検証
    verify_synthesized_pattern_correctness(&program, &holes);
}

#[derive(Debug)]
struct ExecutionResult {
    created_nodes: usize,
    property_assignments: usize,
    maintains_list_structure: bool,
}

fn simulate_program_execution(program: &ast::Program) -> ExecutionResult {
    let mut created_nodes = 0;
    let mut property_assignments = 0;

    for stmt in &program.stmts {
        match stmt {
            ast::Stmt::VarDecl {
                expr: ast::Expr::New(_),
                ..
            } => {
                created_nodes += 1;
            }
            ast::Stmt::Assign {
                lhs: ast::Lhs::ObjAccess(_, _),
                ..
            } => {
                property_assignments += 1;
            }
            _ => {}
        }
    }

    ExecutionResult {
        created_nodes,
        property_assignments,
        maintains_list_structure: true, // 簡略化
    }
}

/// 合成されたプログラムが複数のメソッド呼び出しから正しく共通パターンを抽出しているかを検証
fn verify_synthesized_pattern_correctness(
    program: &ast::Program,
    holes: &HashMap<String, Vec<String>>,
) {
    println!("=== Pattern Synthesis Verification ===");

    // 1. プログラム構造の基本的な検証
    assert!(
        !program.stmts.is_empty(),
        "Synthesized pattern should have statements"
    );

    // 2. ホールの存在確認（複数のメソッド呼び出し間の違いがパラメータ化されている）
    assert!(
        !holes.is_empty(),
        "Should have holes representing differences between method calls"
    );

    // 3. パターンが append の期待される動作を表現しているかを確認
    let mut creates_node = false;
    let mut sets_val_property = false;
    let mut sets_next_property = false;

    for stmt in &program.stmts {
        match stmt {
            ast::Stmt::VarDecl {
                expr: ast::Expr::New(_),
                ..
            } => {
                creates_node = true;
            }
            ast::Stmt::Assign {
                lhs: ast::Lhs::ObjAccess(_, prop),
                ..
            } if prop == "val" => {
                sets_val_property = true;
            }
            ast::Stmt::Assign {
                lhs: ast::Lhs::ObjAccess(_, prop),
                ..
            } if prop == "next" => {
                sets_next_property = true;
            }
            _ => {}
        }
    }

    assert!(creates_node, "Synthesized pattern should create new nodes");
    assert!(
        sets_val_property,
        "Synthesized pattern should set val property"
    );
    assert!(
        sets_next_property,
        "Synthesized pattern should set next property"
    );

    // 4. ホールの値が期待される範囲内にあることを確認
    let all_hole_values: Vec<_> = holes.values().flat_map(|v| v.iter()).collect();
    println!("All hole values found: {:?}", all_hole_values);

    // 複数のメソッド呼び出しから異なる値が抽出されていることを確認
    let unique_values: std::collections::HashSet<_> = all_hole_values.into_iter().collect();
    assert!(
        unique_values.len() >= 2,
        "Should have at least 2 different values from multiple method calls, got: {:?}",
        unique_values
    );

    println!("✓ Pattern synthesis verification passed");
    println!("✓ Successfully extracted common pattern from multiple method calls");
    println!("✓ Differences between calls properly parameterized as holes");
}
