use crate::handle_synthesis;
use warp::Filter;

pub async fn run_server() {
    let hello = warp::path("hello").map(|| "Hello from RefSyn!");

    // CORS設定
    let cors = warp::cors()
        .allow_any_origin() // すべてのオリジンを許可 (開発用)
        .allow_headers(vec!["Content-Type", "Authorization"]) // 許可するヘッダー
        .allow_methods(vec!["GET", "POST", "PUT", "DELETE", "OPTIONS"]); // 許可するメソッド

    let synthesize_route = warp::post()
        .and(warp::path("synthesize"))
        .and(warp::body::bytes())
        .and_then(handle_synthesis);

    let routes = hello.or(synthesize_route).with(cors); // CORSフィルターを適用

    println!("Server running on http://127.0.0.1:3030");
    println!("Available endpoints:");
    println!("  GET  /hello       - Hello message");
    println!("  POST /synthesize  - Code synthesis with integrated operation analysis");
    warp::serve(routes).run(([127, 0, 0, 1], 3030)).await;
}
