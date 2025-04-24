use warp::Filter;
use serde::{Deserialize, Serialize};
use warp::http::Method;

#[derive(Deserialize, Debug)]
struct SynthesisRequest {
    operations: Vec<serde_json::Value>,
    code_lines: Vec<String>,
}

#[derive(Serialize)]
struct SynthesisResponse {
    code: String,
}

fn generate_code(operations: &Vec<serde_json::Value>, code_lines: &Vec<String>) -> String {
    let mut result = String::new();
    result.push_str("// Generated code:\n\n");

    for op in operations {
        if let Ok(op_str) = serde_json::to_string_pretty(op) {
            result.push_str(&format!("// operation: {}\n", op_str));
        } else {
            result.push_str("// operation: <failed to serialize>\n");
        }
    }

    result.push_str("\n");
    for line in code_lines {
        result.push_str(&format!("{}\n", line));
    }

    result
}

// 独自のサーバー実行関数を実装
async fn run_server() {
    // CORSの設定
    let cors = warp::cors()
        .allow_any_origin()
        .allow_methods(&[Method::POST, Method::GET, Method::OPTIONS])
        .allow_headers(["Content-Type"]);

    // JSONリクエストを処理するルート
    let synthesize = warp::post()
        .and(warp::path("synthesize"))
        .and(warp::body::json())
        .map(|request: SynthesisRequest| {
            // リクエスト内容をログに出力
            println!("Received request with {} operations and {} lines of code", 
                    request.operations.len(), request.code_lines.len());
            
            // サンプルのコード生成 - 実際のロジックはここに実装
            let synthesized_code = generate_code(&request.operations, &request.code_lines);
            
            println!("Sending response with synthesized code");
            
            let response = SynthesisResponse {
                code: synthesized_code,
            };
            warp::reply::json(&response)
        });

    // ルートを結合して、CORSを適用
    let routes = synthesize.with(cors);

    // サーバーを起動
    println!("refsyn サーバー起動中: http://localhost:3030");
    warp::serve(routes).run(([127, 0, 0, 1], 3030)).await;
}

#[tokio::main]
async fn main() {
    run_server().await;
}
