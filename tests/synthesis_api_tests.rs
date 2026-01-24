use refsyn::list_env::{GraphOperation, ListEnvironment};
use refsyn::models::{Edge, Node, VisGraph};
use refsyn::{handle_synthesis, SynthesisResponse};
use serde_json::json;
use warp::http::StatusCode;
use warp::Reply;

#[tokio::test]
#[ignore]
async fn test_synthesis_from_kanon_payload() {
    let json_payload = r#"{
        "method_calls":[
            {"callLabel":"call1","contextSensitiveID":"main","receiverObject":"main-new1","methodName":"call1","operations":[{"editType":"addNode","id":"__temp1","label":"Node","isLiteral":false},{"editType":"addNode","id":"__temp2","label":"0","isLiteral":true,"type":"string"},{"editType":"addEdge","from":"__temp1","to":"__temp2","label":"val"},{"editType":"addEdge","from":"main-new1","to":"__temp1","label":"next"}]},
            {"callLabel":"call2","contextSensitiveID":"main","receiverObject":"main-new1","methodName":"call2","operations":[{"editType":"addNode","id":"__temp3","label":"3","isLiteral":true,"type":"string"},{"editType":"addNode","id":"__temp4","label":"Node","isLiteral":false},{"editType":"addEdge","from":"__temp1","to":"__temp4","label":"next"},{"editType":"addEdge","from":"__temp4","to":"__temp3","label":"val"}]}
        ],
        "vis_graph":{
            "nodes":[{"color":"skyblue","id":"main-new1","label":"Node","shape":"ellipse","fixed":true,"value":15,"scaling":{"label":{"enable":true,"min":15,"max":15}},"x":-72,"y":5},{"color":{"border":"white","background":"white","highlight":{"border":"white","background":"white"},"hover":{"border":"white","background":"white"}},"id":"main-new1-val","label":"2","type":"number","isLiteral":true,"scaling":{"label":{"enable":true,"min":10,"max":10}},"fixed":true,"value":10,"x":-103,"y":95},{"color":"skyblue","id":"__temp1","label":"Node","shape":"ellipse","fixed":true,"value":15,"scaling":{"label":{"enable":true,"min":15,"max":15}},"x":72,"y":5},{"color":{"border":"white","background":"white","highlight":{"border":"white","background":"white"},"hover":{"border":"white","background":"white"}},"id":"__temp1-val","label":"0","type":"string","isLiteral":true,"scaling":{"label":{"enable":true,"min":10,"max":10}},"fixed":true,"value":10,"x":102,"y":95},{"hidden":true,"id":"__Variable-lst","label":"lst"},{"id":"__RectForVariable__","label":"def var","color":{"border":"lightsalmon","background":"lightsalmon","highlight":{"border":"lightsalmon","background":"lightsalmon"},"hover":{"border":"lightsalmon","background":"lightsalmon"}},"shape":"box","physics":false},{"id":"__temp3","x":174.0042724609375,"y":90.50143432617188,"label":"3","isLiteral":true,"fixed":true,"color":{"border":"white","background":"white","highlight":{"border":"white","background":"white"},"hover":{"border":"white","background":"white"}},"type":"string"},{"id":"__temp4","x":172.0042724609375,"y":1.501434326171875,"label":"Node","isLiteral":false,"fixed":true}],
            "edges":[{"arrows":{"to":{"enabled":true,"scaleFactor":1}},"from":"main-new1","to":"main-new1-val","label":"val","width":3,"font":{"size":14},"smooth":{"enabled":true},"id":"5e8539bd-1e6e-4d9a-82be-2513e3fa367f"},{"arrows":{"to":{"enabled":true,"scaleFactor":1}},"from":"__temp1","to":"__temp1-val","label":"val","width":3,"font":{"size":14},"smooth":{"enabled":true},"id":"93ad0177-ab66-4d10-8458-35cf937ef9df"},{"arrows":{"to":{"enabled":true,"scaleFactor":1}},"from":"main-new1","to":"__temp1","label":"next","width":3,"font":{"size":14},"smooth":{"enabled":true},"id":"0672b182-dff4-4075-925a-97973be5711c"},{"arrows":{"to":{"enabled":true,"scaleFactor":1}},"from":"__Variable-lst","to":"main-new1","label":"lst","width":3,"font":{"size":14},"smooth":{"enabled":true},"color":"seagreen","id":"226ccde0-e83a-4b8d-a360-2e099f4098b0"},{"from":"__temp1","to":"__temp4","label":"next","id":"114cf33a-36fb-4e98-9dcd-ad011f6d80e0"},{"from":"__temp4","to":"__temp3","label":"val","id":"33035363-0b66-4703-9cde-821561f5f3b6"}]
        }
    }"#;

    let bytes = bytes::Bytes::from(json_payload);
    let result = handle_synthesis(bytes).await;

    match result {
        Ok(reply) => {
            let response = reply.into_response();
            assert_eq!(response.status(), StatusCode::OK);
            let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
            let synthesis_response: SynthesisResponse = serde_json::from_slice(&body).unwrap();

            // The new pipeline does not perform code generation or hole analysis
            assert!(synthesis_response.individual_codes.is_empty());
            assert!(synthesis_response.code.is_empty());
            assert_eq!(synthesis_response.common_pattern, None);
            assert_eq!(synthesis_response.hole_information, None);

            // ListEnvironment情報が含まれているかを確認
            let list_env_info = synthesis_response.list_environment_info.as_ref().unwrap();
            assert!(list_env_info.contains("List Environment Summary"));
            assert!(list_env_info.contains("objects tracked"));
            assert!(list_env_info.contains("Current state"));

            println!("✅ All assertions passed!");
        }
        Err(_) => {
            panic!("handle_synthesis returned an error");
        }
    }
}

#[test]
fn test_list_environment_from_kanon_data() {
    // KanonのAPIテストデータから実際のグラフ構造を抽出
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
            Node {
                id: "__temp1".to_string(),
                is_literal: false,
                label: json!("Node"),
            },
            Node {
                id: "__temp1-val".to_string(),
                is_literal: true,
                label: json!("0"),
            },
            Node {
                id: "__temp3".to_string(),
                is_literal: true,
                label: json!("3"),
            },
            Node {
                id: "__temp4".to_string(),
                is_literal: false,
                label: json!("Node"),
            },
        ],
        edges: vec![
            Edge {
                from: "main-new1".to_string(),
                to: "main-new1-val".to_string(),
                label: "val".to_string(),
            },
            Edge {
                from: "__temp1".to_string(),
                to: "__temp1-val".to_string(),
                label: "val".to_string(),
            },
            Edge {
                from: "main-new1".to_string(),
                to: "__temp1".to_string(),
                label: "next".to_string(),
            },
            Edge {
                from: "__temp1".to_string(),
                to: "__temp4".to_string(),
                label: "next".to_string(),
            },
            Edge {
                from: "__temp4".to_string(),
                to: "__temp3".to_string(),
                label: "val".to_string(),
            },
        ],
    };

    // 初期状態のListEnvironmentを作成
    let env = ListEnvironment::from_vis_graph(&vis_graph);

    println!("Initial environment from Kanon data:");
    println!("{}", env.to_debug_string());

    // オブジェクトのインデックスを確認（VisGraphのノード順を維持）
    assert_eq!(env.obj_id_to_index.get("main-new1"), Some(&0));
    assert_eq!(env.obj_id_to_index.get("__temp1"), Some(&1));
    assert_eq!(env.obj_id_to_index.get("__temp4"), Some(&2));

    // next フィールドをチェック
    let next_list = env.field_lists.get("next").unwrap();
    assert_eq!(next_list[0], json!(1)); // main-new1 -> __temp1 (index 1)
    assert_eq!(next_list[1], json!(2)); // __temp1 -> __temp4 (index 2)
    assert_eq!(next_list[2], serde_json::Value::Null); // __temp4 -> nullPtr

    // val フィールドをチェック
    let val_list = env.field_lists.get("val").unwrap();
    assert_eq!(val_list[0], json!("2")); // main-new1.val = "2"
    assert_eq!(val_list[1], json!("0")); // __temp1.val = "0"
    assert_eq!(val_list[2], json!("3")); // __temp4.val = "3"
}

#[test]
fn test_kanon_operations_applied() {
    // 初期状態のグラフ（操作前）
    let initial_graph = VisGraph {
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

    let mut env = ListEnvironment::from_vis_graph(&initial_graph);

    println!("Initial state:");
    println!("{}", env.to_debug_string());

    // Kanonの操作をシミュレート: call1の操作
    let call1_operations = vec![
        GraphOperation {
            edit_type: "addNode".to_string(),
            id: Some("__temp1".to_string()),
            label: Some(json!("Node")),
            is_literal: Some(false),
            node_type: None,
            from: None,
            to: None,
        },
        GraphOperation {
            edit_type: "addNode".to_string(),
            id: Some("__temp2".to_string()),
            label: Some(json!("0")),
            is_literal: Some(true),
            node_type: Some("string".to_string()),
            from: None,
            to: None,
        },
        GraphOperation {
            edit_type: "addEdge".to_string(),
            id: None,
            label: Some(json!("val")),
            is_literal: None,
            node_type: None,
            from: Some("__temp1".to_string()),
            to: Some("__temp2".to_string()),
        },
        GraphOperation {
            edit_type: "addEdge".to_string(),
            id: None,
            label: Some(json!("next")),
            is_literal: None,
            node_type: None,
            from: Some("main-new1".to_string()),
            to: Some("__temp1".to_string()),
        },
    ];

    env.apply_operations(&call1_operations).unwrap();

    println!("After call1 operations:");
    println!("{}", env.to_debug_string());

    // call1後の状態を確認
    // ソート順: "__temp1", "main-new1" -> インデックス 0, 1
    assert_eq!(env.obj_id_to_index.get("__temp1"), Some(&1));
    assert_eq!(env.obj_id_to_index.get("main-new1"), Some(&0));

    let val_list = env.field_lists.get("val").unwrap();
    assert_eq!(val_list[0], json!("2")); // main-new1.val = "2"
    assert_eq!(val_list[1], json!("0")); // __temp1.val = "0"

    let next_list = env.field_lists.get("next").unwrap();
    assert_eq!(next_list[0], json!(1)); // main-new1.next = __temp1 (index 1)
    assert_eq!(next_list[1], serde_json::Value::Null); // __temp1.next = nullPtr

    // call2の操作を追加
    let call2_operations = vec![
        GraphOperation {
            edit_type: "addNode".to_string(),
            id: Some("__temp3".to_string()),
            label: Some(json!("3")),
            is_literal: Some(true),
            node_type: Some("string".to_string()),
            from: None,
            to: None,
        },
        GraphOperation {
            edit_type: "addNode".to_string(),
            id: Some("__temp4".to_string()),
            label: Some(json!("Node")),
            is_literal: Some(false),
            node_type: None,
            from: None,
            to: None,
        },
        GraphOperation {
            edit_type: "addEdge".to_string(),
            id: None,
            label: Some(json!("next")),
            is_literal: None,
            node_type: None,
            from: Some("__temp1".to_string()),
            to: Some("__temp4".to_string()),
        },
        GraphOperation {
            edit_type: "addEdge".to_string(),
            id: None,
            label: Some(json!("val")),
            is_literal: None,
            node_type: None,
            from: Some("__temp4".to_string()),
            to: Some("__temp3".to_string()),
        },
    ];

    env.apply_operations(&call2_operations).unwrap();

    println!("After call2 operations:");
    println!("{}", env.to_debug_string());

    // 最終状態を確認
    // ソート順: "__temp1", "__temp4", "main-new1" -> インデックス 0, 1, 2
    assert_eq!(env.obj_id_to_index.get("__temp1"), Some(&1));
    assert_eq!(env.obj_id_to_index.get("__temp4"), Some(&2));
    assert_eq!(env.obj_id_to_index.get("main-new1"), Some(&0));

    let final_val_list = env.field_lists.get("val").unwrap();
    assert_eq!(final_val_list[0], json!("2")); // main-new1.val = "2"
    assert_eq!(final_val_list[1], json!("0")); // __temp1.val = "0"
    assert_eq!(final_val_list[2], json!("3")); // __temp4.val = "3"

    let final_next_list = env.field_lists.get("next").unwrap();
    assert_eq!(final_next_list[0], json!(1)); // main-new1.next = __temp1
    assert_eq!(final_next_list[1], json!(2)); // __temp1.next = __temp4
    assert_eq!(final_next_list[2], serde_json::Value::Null); // __temp4.next = nullPtr
}
