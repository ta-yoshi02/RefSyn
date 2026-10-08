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
        "validation":{"version":2,"classes":[{"name":"Cell","source":"class Cell { constructor() { this.payload = 7; this.link = null; } }"}],"cases":[{
            "callLabel":"call","contextSensitiveID":"ctx","objects":[{"id":"r","className":"Cell","fields":{"payload":1,"link":null}}],"arguments":[]
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
async fn http_never_exposes_code_that_was_not_validated() {
    // The pre-state says link already has an edge, which contradicts the demonstrated addEdge.
    let mut stale = request();
    stale["validation"]["cases"][0]["objects"][0]["fields"]["link"] = json!({"ref":"r"});
    let mut missing = request();
    missing.as_object_mut().unwrap().remove("validation");
    for request in [stale, missing] {
        let response = run(request).await;
        let validation = response.validation.unwrap();
        assert!(
            matches!(validation.status, ValidationStatus::Unavailable),
            "{:?}",
            validation
        );
        assert!(response.composed_method_code.is_none());
        assert!(response.code.is_empty());
    }
}

#[tokio::test]
async fn http_validates_demonstrated_return_values() {
    let mut returning = request();
    returning["method_calls"][0]["operations"]
        .as_array_mut()
        .unwrap()
        .push(json!({"editType":"addVariable", "label":"return", "to":"r"}));
    let response = run(returning).await;
    let validation = response.validation.unwrap();
    assert!(
        matches!(validation.status, ValidationStatus::Passed),
        "{:?}",
        validation
    );
    assert!(response.composed_method_code.unwrap().contains("return"));
}

fn obj_class_source(case: &str) -> String {
    let program = std::fs::read_to_string(format!(
        "{}/docs/evaluation_cases/{case}/target_program.js",
        env!("CARGO_MANIFEST_DIR")
    ))
    .unwrap();
    let start = program.find("class Obj").unwrap();
    let mut depth = 0;
    for (offset, ch) in program[start..].char_indices() {
        match ch {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return program[start..start + offset + 1].to_string();
                }
            }
            _ => {}
        }
    }
    panic!("unterminated class in {case}");
}

// Builds the typed pre-state that Kanon now captures from each call's recorded precondGraph.
// The display graph omits null fields, so f and g start as null, matching Obj's constructor.
// Like Kanon's synthesize(), runtime IDs of objects created in earlier demonstrations become their temp IDs.
fn with_typed_pre_state(mut request: Value, class_source: String) -> Value {
    let mut temp_ids = std::collections::HashMap::new();
    for call in request["method_calls"].as_array().unwrap() {
        for (temp, runtime) in call["idMapping"].as_object().into_iter().flatten() {
            temp_ids.insert(runtime.as_str().unwrap().to_string(), temp.clone());
        }
    }
    let id_of = |id: &str| temp_ids.get(id).cloned().unwrap_or_else(|| id.to_string());
    let cases: Vec<Value> = request["method_calls"]
        .as_array()
        .unwrap()
        .iter()
        .map(|call| {
            let graph = &call["precondGraph"];
            let nodes = graph["nodes"].as_array().unwrap();
            let node = |id: &str| nodes.iter().find(|n| n["id"] == id).unwrap();
            let mut values = serde_json::Map::new();
            let objects: Vec<Value> = nodes
                .iter()
                .filter(|n| n["label"] == "Obj")
                .map(|object| {
                    let id = object["id"].as_str().unwrap();
                    let mut fields = json!({"f": null, "g": null});
                    for edge in graph["edges"].as_array().unwrap() {
                        if edge["from"] != id {
                            continue;
                        }
                        let to = edge["to"].as_str().unwrap();
                        let target = node(to);
                        fields[edge["label"].as_str().unwrap()] = if target["label"] == "Obj" {
                            json!({"ref": id_of(to)})
                        } else {
                            let label = target["label"].as_str().unwrap();
                            let value = match (target["type"].as_str(), label.parse::<i64>()) {
                                (Some("number"), Ok(number)) => json!(number),
                                _ => json!(label),
                            };
                            values.insert(id_of(to), value.clone());
                            value
                        };
                    }
                    json!({"id": id_of(id), "className": "Obj", "fields": fields})
                })
                .collect();
            json!({
                "callLabel": call["callLabel"],
                "contextSensitiveID": call["contextSensitiveID"],
                "objects": objects,
                "values": values,
                "arguments": call["arguments"],
            })
        })
        .collect();
    request["validation"] =
        json!({"version": 2, "classes": [{"name": "Obj", "source": class_source}], "cases": cases});
    request
}

#[tokio::test]
async fn saved_evaluation_cases_are_adopted_after_validation() {
    assert!(
        std::path::Path::new(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/external/escher-ts/dist/index.js"
        ))
        .exists(),
        "requires a built external/escher-ts"
    );
    for case in ["append", "insert_general", "popBack", "prepend", "setAt"] {
        let payload: Value = serde_json::from_str(
            &std::fs::read_to_string(format!(
                "{}/docs/evaluation_cases/{case}/mold_payload.json",
                env!("CARGO_MANIFEST_DIR")
            ))
            .unwrap(),
        )
        .unwrap();
        let demonstrations = payload["method_calls"].as_array().unwrap().len();
        let response = run(with_typed_pre_state(payload, obj_class_source(case))).await;
        let validation = response.validation.unwrap();
        assert!(
            matches!(validation.status, ValidationStatus::Passed),
            "{case}: {validation:?}"
        );
        assert_eq!(validation.checked_demonstrations, demonstrations, "{case}");
        assert!(response.composed_method_code.is_some(), "{case}");
    }
}
