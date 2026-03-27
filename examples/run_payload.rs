use refsyn::{handle_synthesis, SynthesisResponse};
use std::env;
use std::fs;
use warp::Reply;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let mut args = env::args().skip(1);
    let Some(path) = args.next() else {
        anyhow::bail!("Usage: cargo run --example run_payload -- <payload.json>");
    };
    if args.next().is_some() {
        anyhow::bail!("Usage: cargo run --example run_payload -- <payload.json>");
    }

    let raw = fs::read_to_string(&path)?;
    let bytes = bytes::Bytes::from(raw);
    let reply = handle_synthesis(bytes).await.map_err(|rejection| {
        anyhow::anyhow!("handle_synthesis rejected request: {:?}", rejection)
    })?;
    let response = reply.into_response();
    let status = response.status();
    let body = hyper::body::to_bytes(response.into_body()).await?;
    let parsed: SynthesisResponse = serde_json::from_slice(&body)?;

    println!("HTTP {}", status);
    println!("{}", serde_json::to_string_pretty(&parsed)?);
    Ok(())
}
