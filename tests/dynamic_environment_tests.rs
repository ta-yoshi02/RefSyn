// 動的環境構築によるメソッド操作テスト
// プログラム解析結果を使って実際のメソッド操作をテストする

#[cfg(test)]
mod tests {
    use refsyn::ir::*;
    use refsyn::ast;
    use refsyn::env::MemoEnv;
    use refsyn::program_analyzer::ProgramAnalysis;
    use serde_json;
    use std::collections::HashMap;

    /// Create environment from JavaScript program analysis
    fn create_env_from_js_analysis(js_program: &str) -> MemoEnv {
        let analysis = ProgramAnalysis::analyze_program(js_program)
            .expect("Failed to analyze JavaScript program");
        
        let mut env = MemoEnv::new();
        
        // Map all method call receivers to 'this'
        let env_mapping = analysis.create_environment_mapping();
        for (object_id, target) in env_mapping {
            env.add_name_id_mapping(object_id, target);
        }
        
        // Map other objects to their variable names
        for (var_name, object_id) in &analysis.object_declarations {
            env.add_name_id_mapping(object_id.clone(), var_name.clone());
        }
        
        env
    }

    #[test]
    fn test_dynamic_append_with_actual_operations() {
        // JavaScriptプログラムを定義
        let js_program = r#"
            class Node {
                constructor() {
                    this.val = null;
                    this.next = null;
                }
                append(value) { /* TBD */ }
            }
            
            var lst = new Node();  // main-new1
            lst.append(42);        // レシーバー: main-new1
        "#;

        // プログラム解析を実行
        let analysis = ProgramAnalysis::analyze_program(js_program).unwrap();
        
        // 動的に環境を構築
        let env = create_env_from_js_analysis(js_program);

        // appendメソッドの実際の操作を定義（analysys.texより）
        let append_operations = vec![
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
                "editType": "addEdge",
                "from": "__temp1",
                "label": "val",
                "to": "__temp2"
            }),
            serde_json::json!({
                "editType": "addEdge",
                "from": "main-new1",  // 解析結果から動的に取得されたID
                "label": "next",
                "to": "__temp1"
            })
        ];

        // 操作リストとしてラップ
        let operations_list = vec![append_operations];
        let memo_envs = vec![env];

        // 共通パターン抽出を実行（単一操作の場合は元の操作を復元）
        let (program_opt, holes) = find_common_pattern_from_operations(&operations_list, &memo_envs);

        // 結果を検証
        assert!(program_opt.is_some(), "Should generate program from append operation");
        let program = program_opt.unwrap();

        println!("=== Dynamic Append Test ===");
        println!("JavaScript program analysis:");
        println!("  Objects: {:?}", analysis.object_declarations);
        println!("  Method calls: {:?}", analysis.method_calls);
        
        println!("Generated program:");
        for (i, stmt) in program.stmts.iter().enumerate() {
            println!("  {}: {:?}", i, stmt);
        }
        
        println!("Holes: {:?}", holes);

        // プログラム構造の基本的な検証
        assert!(!program.stmts.is_empty(), "Program should have statements");
        
        // Node作成が含まれていることを確認
        let has_node_creation = program.stmts.iter().any(|stmt| {
            matches!(stmt, ast::Stmt::VarDecl { expr: ast::Expr::New(_), .. })
        });
        assert!(has_node_creation, "Should create new Node");
    }

    #[test]
    fn test_multiple_method_calls_with_different_receivers() {
        // 複数のオブジェクトに対する異なるメソッド呼び出し
        let js_program = r#"
            class Node {
                append(value) { /* TBD */ }
                prepend(value) { /* TBD */ }
            }
            
            var list1 = new Node();   // main-new1
            var list2 = new Node();   // main-new2
            
            list1.append(10);         // main-new1.append(10)
            list2.prepend(20);        // main-new2.prepend(20)
        "#;

        let analysis = ProgramAnalysis::analyze_program(js_program).unwrap();
        
        // 各メソッド呼び出しに対応する操作を定義
        // list1.append(10) の操作
        let list1_append_operations = vec![
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
                "label": "10",
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
            })
        ];

        // list2.prepend(20) の操作  
        let list2_prepend_operations = vec![
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
                "label": "20",
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
                "to": "main-new2"
            }),
            serde_json::json!({
                "editType": "editEdgeReference",
                "from": "main-new2",
                "label": "next",
                "to": "__temp3"
            })
        ];

        // 環境をそれぞれのメソッド呼び出し用に構築
        let mut env1 = MemoEnv::new();
        env1.add_name_id_mapping("main-new1".to_string(), "this".to_string());
        
        let mut env2 = MemoEnv::new();
        env2.add_name_id_mapping("main-new2".to_string(), "this".to_string());

        // 各操作を個別にテスト
        let operations_list1 = vec![list1_append_operations];
        let memo_envs1 = vec![env1];
        let (program1_opt, _holes1) = find_common_pattern_from_operations(&operations_list1, &memo_envs1);
        
        let operations_list2 = vec![list2_prepend_operations];
        let memo_envs2 = vec![env2];
        let (program2_opt, _holes2) = find_common_pattern_from_operations(&operations_list2, &memo_envs2);

        // 結果を検証
        assert!(program1_opt.is_some(), "Should generate program for list1.append");
        assert!(program2_opt.is_some(), "Should generate program for list2.prepend");

        println!("=== Multiple Receivers Test ===");
        println!("Analysis results:");
        for (i, call) in analysis.method_calls.iter().enumerate() {
            println!("  Call {}: {}.{}() -> {}", i, call.receiver_var, call.method_name, call.receiver_id);
        }

        let program1 = program1_opt.unwrap();
        let program2 = program2_opt.unwrap();
        
        println!("Program for list1.append(10):");
        for (i, stmt) in program1.stmts.iter().enumerate() {
            println!("  {}: {:?}", i, stmt);
        }
        
        println!("Program for list2.prepend(20):");
        for (i, stmt) in program2.stmts.iter().enumerate() {
            println!("  {}: {:?}", i, stmt);
        }

        // プログラムが生成されていることを確認
        assert!(!program1.stmts.is_empty(), "Program1 should have statements");
        assert!(!program2.stmts.is_empty(), "Program2 should have statements");
    }

    #[test]
    fn test_dynamic_environment_construction_accuracy() {
        // 環境構築の正確性をテスト
        let js_program = r#"
            var first = new Node();   // main-new1
            var second = new Node();  // main-new2 
            var third = new Node();   // main-new3
            
            second.append(99);        // main-new2.append
        "#;

        let analysis = ProgramAnalysis::analyze_program(js_program).unwrap();
        
        // 解析結果の検証
        assert_eq!(analysis.object_declarations.len(), 3);
        assert_eq!(analysis.method_calls.len(), 1);
        assert_eq!(analysis.method_calls[0].receiver_id, "main-new2");

        // 環境マッピングの検証
        let env_mapping = analysis.create_environment_mapping();
        
        // second.append()呼び出しのため、main-new2がthisにマップされる
        assert_eq!(env_mapping.get("main-new2"), Some(&"this".to_string()));
        
        // 他のオブジェクトはメソッド呼び出しがないためthisにマップされない
        assert!(!env_mapping.contains_key("main-new1"));
        assert!(!env_mapping.contains_key("main-new3"));

        println!("=== Environment Construction Accuracy Test ===");
        println!("Object declarations: {:?}", analysis.object_declarations);
        println!("Method calls: {:?}", analysis.method_calls);
        println!("Environment mapping: {:?}", env_mapping);

        // 動的環境を構築
        let env = create_env_from_js_analysis(js_program);
        
        println!("Dynamic environment created successfully for main-new2 -> this mapping");
    }
}
