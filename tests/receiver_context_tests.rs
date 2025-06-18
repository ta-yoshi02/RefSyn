// 新しいレシーバーコンテキスト管理システムのテスト

use refsyn::env::MemoEnv;
use refsyn::parser::parse_operations;
use serde_json;

/// レシーバーコンテキストの基本テスト
#[test]
fn test_receiver_context_management() {
    let mut env = MemoEnv::new();
    
    // メソッド呼び出しのスコープを開始し、レシーバーを設定
    env.start_method_call_scope_with_receiver("test_call", "main-new1");
    
    // レシーバーが正しく設定されているかテスト
    assert!(env.is_current_receiver("main-new1"));
    assert!(!env.is_current_receiver("main-new2"));
    
    // レシーバーIDからthisマッピングを取得
    assert_eq!(env.get_name_by_id("main-new1"), Some(&"this".to_string()));
    
    // スコープを終了
    env.end_method_call_scope();
    
    // スコープ終了後はレシーバーコンテキストがクリアされる
    assert!(!env.is_current_receiver("main-new1"));
}

/// 動的thisマッピングのテスト
#[test]
fn test_dynamic_this_mapping() {
    let mut env = MemoEnv::new();
    
    // main-new1をレシーバーとして設定
    env.set_current_receiver("main-new1");
    
    // resolve_or_create_var_name_for_idがレシーバーに対してthisを返すかテスト
    let name1 = env.resolve_or_create_var_name_for_id("main-new1", true);
    assert_eq!(name1, "this");
    
    // 非レシーバーIDに対しては通常の変数名を生成
    let name2 = env.resolve_or_create_var_name_for_id("main-new2", false);
    assert!(name2.starts_with("obj_"));
    
    // 異なるレシーバーに変更
    env.set_current_receiver("main-new2");
    
    // 新しいレシーバーがthisとして認識される
    assert!(env.is_current_receiver("main-new2"));
    assert!(!env.is_current_receiver("main-new1"));
}

/// アクセスパス構築のテスト
#[test]
fn test_access_path_with_receiver_context() {
    let mut env = MemoEnv::new();
    env.set_current_receiver("main-new1");
    
    // レシーバー自体のアクセスパスはthis
    let path1 = env.get_access_path("main-new1");
    assert_eq!(path1, Some("this".to_string()));
    
    // プロパティアクセスの登録
    env.register_property_assignment("main-new1".to_string(), "value".to_string(), "prop1".to_string());
    
    // プロパティのアクセスパスはthis.property
    let path2 = env.get_access_path("prop1");
    assert_eq!(path2, Some("this.value".to_string()));
}

/// 複数のメソッド呼び出し間でのコンテキスト分離のテスト
#[test]
fn test_multiple_method_call_contexts() {
    let mut env1 = MemoEnv::new();
    let mut env2 = MemoEnv::new();
    
    // 1つ目のメソッド呼び出し: obj1.method()
    env1.start_method_call_scope_with_receiver("call1", "obj1");
    assert!(env1.is_current_receiver("obj1"));
    assert_eq!(env1.get_name_by_id("obj1"), Some(&"this".to_string()));
    
    // 2つ目のメソッド呼び出し: obj2.method()
    env2.start_method_call_scope_with_receiver("call2", "obj2");
    assert!(env2.is_current_receiver("obj2"));
    assert_eq!(env2.get_name_by_id("obj2"), Some(&"this".to_string()));
    
    // コンテキストは独立している
    assert!(!env1.is_current_receiver("obj2"));
    assert!(!env2.is_current_receiver("obj1"));
    
    env1.end_method_call_scope();
    env2.end_method_call_scope();
}

/// パーサーでのレシーバーコンテキスト使用のテスト
#[test] 
fn test_parser_with_receiver_context() {
    let operations = vec![
        serde_json::json!({
            "editType": "addNode",
            "id": "main-new1", 
            "label": "Node",
            "isLiteral": false
        }),
        serde_json::json!({
            "editType": "addEdge",
            "from": "main-new1",
            "to": "main-new2", 
            "label": "val"
        })
    ];
    
    let mut env = MemoEnv::new();
    
    // main-new1をレシーバーとして指定してパース
    let result = parse_operations(&operations, Some(&"main-new1".to_string()), &mut env);
    
    assert!(result.is_ok(), "Parsing should succeed");
    let program = result.unwrap();
    
    // 生成されたASTでthisが正しく使用されているかチェック
    let ast_string = format!("{}", program);
    println!("Generated AST: {}", ast_string);
    
    // thisキーワードが含まれていることを確認
    assert!(ast_string.contains("this"), "Generated code should contain 'this' keyword");
}

/// エラーハンドリングのテスト
#[test]
fn test_receiver_context_error_handling() {
    let env = MemoEnv::new();
    
    // レシーバー未設定の状態でのアクセス
    assert!(!env.is_current_receiver("any-id"));
    assert_eq!(env.get_current_receiver(), None);
    
    // 存在しないメソッド呼び出しIDでのレシーバー取得
    assert_eq!(env.get_method_receiver("nonexistent"), None);
}
