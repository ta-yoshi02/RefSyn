// 動的プログラム解析を使用した統合テスト
// JavaScriptプログラムを解析してオブジェクトIDを動的に決定し、テストを実行する

#[cfg(test)]
mod tests {
    use refsyn::env::MemoEnv;
    use refsyn::program_analyzer::ProgramAnalysis;

    /// JavaScript プログラムから動的に環境を構築してテスト実行
    fn create_env_from_js_program(js_program: &str, method_call_index: usize) -> MemoEnv {
        let analysis = ProgramAnalysis::analyze_program(js_program)
            .expect("Failed to analyze JavaScript program");

        let mut env = MemoEnv::new();

        // メソッド呼び出しのレシーバーIDを動的に取得
        if let Some(receiver_id) = analysis.get_receiver_id_for_call(method_call_index) {
            env.add_name_id_mapping(receiver_id.to_string(), "this".to_string());
        }

        // その他のオブジェクトIDもマッピング
        for (var_name, object_id) in &analysis.object_declarations {
            if var_name != "this" {
                env.add_name_id_mapping(object_id.clone(), var_name.clone());
            }
        }

        env
    }

    #[test]
    fn test_dynamic_append_operation_analysis() {
        // JavaScriptプログラムの例
        let js_program = r#"
            class Node {
                constructor() {
                    this.val = null;
                    this.next = null;
                }
                append(value) { /* TBD */ }
            }
            
            var list = new Node();  // main-new1
            var lst = new Node();   // main-new2
            lst.append(42);         // レシーバー: main-new2
        "#;

        // プログラム解析を実行
        let analysis = ProgramAnalysis::analyze_program(js_program).unwrap();

        // 結果を検証
        assert_eq!(analysis.object_declarations.len(), 2);
        assert_eq!(
            analysis.object_declarations.get("list"),
            Some(&"main-new1".to_string())
        );
        assert_eq!(
            analysis.object_declarations.get("lst"),
            Some(&"main-new2".to_string())
        );

        assert_eq!(analysis.method_calls.len(), 1);
        assert_eq!(analysis.method_calls[0].receiver_id, "main-new2");
        assert_eq!(analysis.method_calls[0].method_name, "append");

        println!("=== Dynamic Program Analysis Test ===");
        println!("Object declarations: {:?}", analysis.object_declarations);
        println!("Method calls: {:?}", analysis.method_calls);

        // 動的に環境を構築
        let _env = create_env_from_js_program(js_program, 0);

        println!("Environment created successfully");
    }

    #[test]
    fn test_multiple_receivers_analysis() {
        let js_program = r#"
            class Node {
                append(value) { /* TBD */ }
                prepend(value) { /* TBD */ }
            }
            
            var list1 = new Node();    // main-new1
            var list2 = new Node();    // main-new2
            var temp = new Node();     // main-new3
            
            list1.append(10);          // レシーバー: main-new1
            list2.prepend(20);         // レシーバー: main-new2
            temp.append(30);           // レシーバー: main-new3
        "#;

        let analysis = ProgramAnalysis::analyze_program(js_program).unwrap();

        // オブジェクト宣言の検証
        assert_eq!(analysis.object_declarations.len(), 3);
        assert_eq!(
            analysis.object_declarations.get("list1"),
            Some(&"main-new1".to_string())
        );
        assert_eq!(
            analysis.object_declarations.get("list2"),
            Some(&"main-new2".to_string())
        );
        assert_eq!(
            analysis.object_declarations.get("temp"),
            Some(&"main-new3".to_string())
        );

        // メソッド呼び出しの検証
        assert_eq!(analysis.method_calls.len(), 3);

        // 1番目の呼び出し: list1.append(10)
        assert_eq!(analysis.method_calls[0].receiver_var, "list1");
        assert_eq!(analysis.method_calls[0].receiver_id, "main-new1");
        assert_eq!(analysis.method_calls[0].method_name, "append");

        // 2番目の呼び出し: list2.prepend(20)
        assert_eq!(analysis.method_calls[1].receiver_var, "list2");
        assert_eq!(analysis.method_calls[1].receiver_id, "main-new2");
        assert_eq!(analysis.method_calls[1].method_name, "prepend");

        // 3番目の呼び出し: temp.append(30)
        assert_eq!(analysis.method_calls[2].receiver_var, "temp");
        assert_eq!(analysis.method_calls[2].receiver_id, "main-new3");
        assert_eq!(analysis.method_calls[2].method_name, "append");

        println!("=== Multiple Receivers Test ===");
        for (i, call) in analysis.method_calls.iter().enumerate() {
            println!(
                "Call {}: {}.{}() -> {}",
                i, call.receiver_var, call.method_name, call.receiver_id
            );
        }

        // 環境マッピングの検証
        let env_mapping = analysis.create_environment_mapping();
        assert_eq!(env_mapping.len(), 3);
        assert_eq!(env_mapping.get("main-new1"), Some(&"this".to_string()));
        assert_eq!(env_mapping.get("main-new2"), Some(&"this".to_string()));
        assert_eq!(env_mapping.get("main-new3"), Some(&"this".to_string()));
    }

    #[test]
    fn test_realistic_linked_list_program() {
        // より現実的なリンクリストプログラムの解析
        let js_program = r#"
            class Node {
                constructor(value) {
                    this.val = value;
                    this.next = null;
                }
                
                append(value) { /* TBD */ }
                prepend(value) { /* TBD */ }
                removeFirst() { /* TBD */ }
            }
            
            // メインプログラム
            var head = new Node(0);       // main-new1
            var list = new Node(1);       // main-new2
            var backup = new Node(2);     // main-new3
            
            head.append(42);              // main-new1.append
            list.prepend(99);             // main-new2.prepend
            backup.removeFirst();         // main-new3.removeFirst
            head.append(100);             // main-new1.append (再度)
        "#;

        let analysis = ProgramAnalysis::analyze_program(js_program).unwrap();

        println!("=== Realistic LinkedList Program Analysis ===");

        // オブジェクトの確認
        assert_eq!(analysis.object_declarations.len(), 3);
        println!("Objects:");
        for (var, id) in &analysis.object_declarations {
            println!("  {} -> {}", var, id);
        }

        // メソッド呼び出しの確認
        assert_eq!(analysis.method_calls.len(), 4);
        println!("Method calls:");
        for (i, call) in analysis.method_calls.iter().enumerate() {
            println!(
                "  {}: {}.{}() -> receiver_id: {}",
                i, call.receiver_var, call.method_name, call.receiver_id
            );
        }

        // 特定の呼び出しを検証
        assert_eq!(analysis.method_calls[0].receiver_var, "head");
        assert_eq!(analysis.method_calls[0].method_name, "append");
        assert_eq!(analysis.method_calls[0].receiver_id, "main-new1");

        assert_eq!(analysis.method_calls[1].receiver_var, "list");
        assert_eq!(analysis.method_calls[1].method_name, "prepend");
        assert_eq!(analysis.method_calls[1].receiver_id, "main-new2");

        assert_eq!(analysis.method_calls[2].receiver_var, "backup");
        assert_eq!(analysis.method_calls[2].method_name, "removeFirst");
        assert_eq!(analysis.method_calls[2].receiver_id, "main-new3");

        assert_eq!(analysis.method_calls[3].receiver_var, "head");
        assert_eq!(analysis.method_calls[3].method_name, "append");
        assert_eq!(analysis.method_calls[3].receiver_id, "main-new1"); // 同じオブジェクトへの再呼び出し
    }

    #[test]
    fn test_edge_cases_and_error_handling() {
        // エッジケースのテスト

        // 空のプログラム
        let empty_program = "";
        let analysis = ProgramAnalysis::analyze_program(empty_program).unwrap();
        assert_eq!(analysis.object_declarations.len(), 0);
        assert_eq!(analysis.method_calls.len(), 0);

        // オブジェクト宣言のみ（メソッド呼び出しなし）
        let only_declarations = r#"
            var obj1 = new Node();
            var obj2 = new Node();
        "#;
        let analysis = ProgramAnalysis::analyze_program(only_declarations).unwrap();
        assert_eq!(analysis.object_declarations.len(), 2);
        assert_eq!(analysis.method_calls.len(), 0);

        // メソッド呼び出しのみ（対応するオブジェクト宣言なし）
        let only_method_calls = r#"
            unknown_obj.method1();
            another_obj.method2();
        "#;
        let analysis = ProgramAnalysis::analyze_program(only_method_calls).unwrap();
        assert_eq!(analysis.object_declarations.len(), 0);
        assert_eq!(analysis.method_calls.len(), 0); // 宣言されていないオブジェクトの呼び出しは無視される

        println!("Edge cases handled successfully");
    }
}
