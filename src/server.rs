use warp::Filter;
use crate::{SynthesisRequest, handle_synthesis};

pub async fn run_server() {
    let hello = warp::path("hello").map(|| "Hello from RefSyn!");

    // CORS設定
    let cors = warp::cors()
        .allow_any_origin() // すべてのオリジンを許可 (開発用)
        .allow_headers(vec!["Content-Type", "Authorization"]) // 許可するヘッダー
        .allow_methods(vec!["GET", "POST", "PUT", "DELETE", "OPTIONS"]); // 許可するメソッド

    let synthesize_route = warp::post()
        .and(warp::path("synthesize"))
        .and(warp::body::json::<SynthesisRequest>())
        .and_then(handle_synthesis);

    let routes = hello.or(synthesize_route).with(cors); // CORSフィルターを適用

    println!("Server running on http://127.0.0.1:3030");
    warp::serve(routes).run(([127, 0, 0, 1], 3030)).await;
}
