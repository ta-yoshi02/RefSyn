use warp::Filter;

pub async fn run_server() {
    let hello = warp::path("hello").map(|| "Hello from RefSyn!");
    warp::serve(hello).run(([127, 0, 0, 1], 3030)).await;
}
