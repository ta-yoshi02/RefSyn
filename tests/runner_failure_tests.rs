//! The escher-ts runner is replaced through `ESCHER_JS_RUNNER`. Environment variables are shared by
//! the whole test binary, so this file holds a single test that runs its cases in sequence.

use refsyn::{handle_synthesis, SynthesisResponse, ValidationStatus};
use serde_json::{json, Value};
use std::time::{Duration, Instant};
use warp::Reply;

// One demonstration whose method creates a cell and links it, so synthesis needs one hole.
fn request() -> Value {
    json!({
        "method_calls": [{"callLabel":"call", "contextSensitiveID":"ctx", "receiverObject":"r", "receiverClassName":"Cell", "methodName":"attach", "operations":[
            {"editType":"addNode", "id":"new", "label":"Cell", "isLiteral":false},
            {"editType":"addEdge", "from":"r", "label":"link", "to":"new"}
        ], "arguments":[], "actualGraph":{"nodes":[{"id":"r", "label":"Cell"}],"edges":[]}}],
        "vis_graph":{"nodes":[{"id":"r", "label":"Cell"}],"edges":[]},
        "validation":{"version":3,"classes":[{"name":"Cell","source":"class Cell { constructor() { this.payload = 7; this.link = null; } }","instance":{"payload":7,"link":null}}],"cases":[{
            "callLabel":"call","contextSensitiveID":"ctx","objects":[{"id":"r","className":"Cell","fields":{"payload":1,"link":null}}],"arguments":[]
        }]}
    })
}

async fn run() -> (u16, SynthesisResponse) {
    let reply = handle_synthesis(bytes::Bytes::from(serde_json::to_vec(&request()).unwrap()))
        .await
        .unwrap()
        .into_response();
    let status = reply.status().as_u16();
    let body = hyper::body::to_bytes(reply.into_body()).await.unwrap();
    (status, serde_json::from_slice(&body).unwrap())
}

fn runner(name: &str, body: &str) -> String {
    let dir = std::env::temp_dir().join(format!(
        "refsyn-failing-runner-{}-{}",
        std::process::id(),
        name
    ));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("runner.js");
    std::fs::write(&path, body).unwrap();
    path.display().to_string()
}

#[tokio::test]
async fn a_failing_runner_is_reported_and_no_code_is_adopted() {
    std::env::set_var("ESCHER_JS_PROCESS_TIMEOUT_MS", "500");
    let cases = [
        (
            runner("hang", "setInterval(() => {}, 1000);"),
            "Escher JS runner did not finish within 500 ms",
        ),
        (
            runner("exit", "process.exit(3);"),
            "Escher JS runner exited with status",
        ),
        (
            runner("invalid", "process.stdout.write('not json');"),
            "Escher JS runner returned invalid JSON",
        ),
        (
            std::env::temp_dir()
                .join("refsyn-no-such-runner/runner.js")
                .display()
                .to_string(),
            "Escher JS runner not found at",
        ),
    ];
    for (path, expected) in cases {
        std::env::set_var("ESCHER_JS_RUNNER", &path);
        let started = Instant::now();
        let (status, response) = run().await;
        assert!(started.elapsed() < Duration::from_secs(15), "{expected}");
        let info = response.list_environment_info.clone().unwrap_or_default();
        assert!(
            info.lines().any(
                |line| line.starts_with("escher-ts runner failed: ") && line.contains(expected)
            ),
            "{expected}: {info}"
        );
        // The request itself was well formed; the reply reports the failure and carries no method.
        assert_eq!(status, 200, "{expected}");
        // The reason is the runner, not a method that failed to reproduce a demonstration.
        let validation = response.validation.expect("validation result");
        assert!(
            matches!(validation.status, ValidationStatus::Unavailable),
            "{expected}: {validation:?}"
        );
        assert_eq!(validation.checked_demonstrations, 0, "{expected}");
        let error = validation.error.unwrap_or_default();
        assert!(
            error.starts_with("escher-ts runner failed: ") && error.contains(expected),
            "{expected}: {error}"
        );
        assert!(response.composed_method_code.is_none(), "{expected}");
        assert!(response.code.is_empty(), "{expected}");
        assert!(
            response
                .escher_results
                .iter()
                .flatten()
                .all(|result| !result.success),
            "{expected}"
        );
    }
    std::env::remove_var("ESCHER_JS_RUNNER");
    std::env::remove_var("ESCHER_JS_PROCESS_TIMEOUT_MS");
}
