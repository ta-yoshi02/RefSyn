use refsyn::server;

#[tokio::main]
async fn main() {
    println!("Starting RefSyn server...");
    server::run_server().await;
}
