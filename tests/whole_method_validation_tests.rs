use refsyn::{handle_synthesis, SynthesisResponse, ValidationStatus};
use serde_json::{json, Value};
use warp::Reply;

fn request() -> Value {
    json!({
        "method_calls": [{"callLabel":"call", "contextSensitiveID":"ctx", "receiverObject":"r", "receiverClassName":"Cell", "methodName":"attach", "operations":[
            {"editType":"addNode", "id":"new", "label":"Cell", "isLiteral":false},
            {"editType":"addEdge", "from":"r", "label":"link", "to":"new"}
        ], "arguments":[], "actualGraph":{"nodes":[{"id":"r", "label":"Cell"}],"edges":[]}}],
        "vis_graph":{"nodes":[{"id":"r", "label":"Cell"}],"edges":[]},
        "validation":{"version":1,"classes":[{"name":"Cell","source":"class Cell { constructor() { this.payload = 7; this.link = null; } }","referenceFields":["link"]}],"cases":[{
            "callLabel":"call","contextSensitiveID":"ctx","objects":[{"id":"r","className":"Cell","fields":{"payload":1,"link":null}}],"arguments":[],"returnValue":{"undefined":true}
        }]}
    })
}
async fn run(request: Value) -> SynthesisResponse {
    let reply = handle_synthesis(bytes::Bytes::from(serde_json::to_vec(&request).unwrap()))
        .await
        .unwrap()
        .into_response();
    let body = hyper::body::to_bytes(reply.into_body()).await.unwrap();
    serde_json::from_slice(&body).unwrap()
}

#[tokio::test]
async fn http_adopts_a_composed_method_only_after_replaying_the_demonstration() {
    let response = run(request()).await;
    let validation = response.validation.unwrap();
    assert!(
        matches!(validation.status, ValidationStatus::Passed),
        "{:?}",
        validation
    );
    assert_eq!(validation.checked_demonstrations, 1);
    assert!(response
        .composed_method_code
        .unwrap()
        .contains("new Cell()"));
}

#[tokio::test]
async fn http_rejects_return_mismatch_and_missing_snapshot_without_exposing_helpers() {
    let mut wrong = request();
    wrong["validation"]["cases"][0]["returnValue"] = json!({"ref":"r"});
    let response = run(wrong).await;
    assert!(matches!(
        response.validation.unwrap().status,
        ValidationStatus::Failed
    ));
    assert!(response.composed_method_code.is_none());
    assert!(response.code.is_empty());
    let mut missing = request();
    missing.as_object_mut().unwrap().remove("validation");
    let response = run(missing).await;
    assert!(matches!(
        response.validation.unwrap().status,
        ValidationStatus::Unavailable
    ));
    assert!(response.composed_method_code.is_none());
    assert!(response.code.is_empty());
}
