//! Requests over a real TCP port to the routes `run_server` serves (an ephemeral port, not 3030).
//! `whole_method_validation_tests` calls `handle_synthesis` directly and covers no routing or CORS.

use hyper::{Body, Client, Method, Request, StatusCode};
use serde_json::{json, Value};
use std::net::SocketAddr;

fn start() -> SocketAddr {
    let (address, server) =
        warp::serve(refsyn::server::routes()).bind_ephemeral(([127, 0, 0, 1], 0));
    tokio::spawn(server);
    address
}

// One demonstration that creates a cell and links it from the receiver.
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

struct Reply {
    status: StatusCode,
    allow_origin: Option<String>,
    body: Vec<u8>,
}

impl Reply {
    fn json(&self) -> Value {
        serde_json::from_slice(&self.body).expect("JSON body")
    }
}

async fn send(address: SocketAddr, method: Method, path: &str, origin: bool, body: Body) -> Reply {
    let mut builder = Request::builder()
        .method(method)
        .uri(format!("http://{address}{path}"))
        .header("Content-Type", "application/json");
    if origin {
        builder = builder.header("Origin", "https://ta-yoshi02.github.io");
    }
    let response = Client::new()
        .request(builder.body(body).unwrap())
        .await
        .unwrap();
    let status = response.status();
    let allow_origin = response
        .headers()
        .get("access-control-allow-origin")
        .map(|value| value.to_str().unwrap().to_string());
    let body = hyper::body::to_bytes(response.into_body())
        .await
        .unwrap()
        .to_vec();
    Reply {
        status,
        allow_origin,
        body,
    }
}

async fn synthesize(address: SocketAddr, body: impl Into<Body>) -> Reply {
    send(address, Method::POST, "/synthesize", true, body.into()).await
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_valid_request_from_another_origin_gets_a_validated_method() {
    let address = start();
    let reply = synthesize(address, serde_json::to_vec(&request()).unwrap()).await;
    assert_eq!(reply.status, StatusCode::OK);
    assert_eq!(
        reply.allow_origin.as_deref(),
        Some("https://ta-yoshi02.github.io")
    );
    let body = reply.json();
    assert_eq!(body["validation"]["status"], "passed", "{body}");
    assert_eq!(body["validation"]["checked_demonstrations"], 1);
    assert!(body["composed_method_code"]
        .as_str()
        .unwrap()
        .contains("new Cell()"));
}

#[tokio::test]
async fn a_preflight_allows_the_post_that_kanon_sends() {
    let address = start();
    let response = Client::new()
        .request(
            Request::builder()
                .method(Method::OPTIONS)
                .uri(format!("http://{address}/synthesize"))
                .header("Origin", "http://localhost:8080")
                .header("Access-Control-Request-Method", "POST")
                .header("Access-Control-Request-Headers", "content-type")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let headers = response.headers();
    assert_eq!(
        headers["access-control-allow-origin"],
        "http://localhost:8080"
    );
    assert!(headers["access-control-allow-methods"]
        .to_str()
        .unwrap()
        .contains("POST"));
    assert!(headers["access-control-allow-headers"]
        .to_str()
        .unwrap()
        .to_ascii_lowercase()
        .contains("content-type"));
}

#[tokio::test]
async fn a_malformed_request_is_answered_400_with_an_empty_response_and_no_code() {
    let address = start();
    let bodies: [&[u8]; 4] = [
        b"",
        b"{not json",
        b"{}",
        br#"{"method_calls":"x","vis_graph":{"nodes":[],"edges":[]}}"#,
    ];
    for body in bodies {
        let reply = synthesize(address, body.to_vec()).await;
        let text = String::from_utf8_lossy(body);
        assert_eq!(reply.status, StatusCode::BAD_REQUEST, "{text}");
        // The error reply still carries CORS, so the page can read the status.
        assert!(reply.allow_origin.is_some(), "{text}");
        let json = reply.json();
        assert!(json["composed_method_code"].is_null(), "{text}: {json}");
        assert_eq!(json["code"], json!([]), "{text}: {json}");
        assert!(json["validation"].is_null(), "{text}: {json}");
    }
}

#[tokio::test]
async fn only_the_two_routes_are_served() {
    let address = start();
    let hello = send(address, Method::GET, "/hello", false, Body::empty()).await;
    assert_eq!(hello.status, StatusCode::OK);
    assert_eq!(hello.body, b"Hello from RefSyn!");
    let get = send(address, Method::GET, "/synthesize", false, Body::empty()).await;
    assert_eq!(get.status, StatusCode::METHOD_NOT_ALLOWED);
    let unknown = send(
        address,
        Method::POST,
        "/synthesise",
        false,
        Body::from("{}"),
    )
    .await;
    assert_eq!(unknown.status, StatusCode::NOT_FOUND);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn concurrent_requests_each_get_their_own_answer() {
    let address = start();
    let valid = serde_json::to_vec(&request()).unwrap();
    // The stale pre-state contradicts the demonstrated addEdge, so that request cannot be validated.
    let mut stale = request();
    stale["validation"]["cases"][0]["objects"][0]["fields"]["link"] = json!({"ref":"r"});
    let stale = serde_json::to_vec(&stale).unwrap();
    let replies = futures_join(vec![
        synthesize(address, valid.clone()),
        synthesize(address, stale.clone()),
        synthesize(address, valid),
        synthesize(address, stale),
    ])
    .await;
    let statuses: Vec<String> = replies
        .iter()
        .map(|reply| {
            reply.json()["validation"]["status"]
                .as_str()
                .unwrap()
                .to_string()
        })
        .collect();
    assert_eq!(statuses, ["passed", "unavailable", "passed", "unavailable"]);
    // The stale request is rejected while reading the demonstrations, before any synthesis.
    for (reply, adopted) in replies.iter().zip([true, false, true, false]) {
        let expected = if adopted {
            StatusCode::OK
        } else {
            StatusCode::BAD_REQUEST
        };
        assert_eq!(reply.status, expected);
        assert_eq!(reply.json()["composed_method_code"].is_string(), adopted);
    }
    assert!(replies[1].json()["list_environment_info"]
        .as_str()
        .unwrap()
        .contains("addEdge on r.link, which already has an edge"));
}

async fn futures_join(
    requests: Vec<impl std::future::Future<Output = Reply> + Send + 'static>,
) -> Vec<Reply> {
    let handles: Vec<_> = requests.into_iter().map(tokio::spawn).collect();
    let mut replies = Vec::new();
    for handle in handles {
        replies.push(handle.await.unwrap());
    }
    replies
}
