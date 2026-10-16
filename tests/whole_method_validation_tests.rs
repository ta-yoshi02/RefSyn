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
        "validation":{"version":3,"classes":[{"name":"Cell","source":"class Cell { constructor() { this.payload = 7; this.link = null; } }","instance":{"payload":7,"link":null}}],"cases":[{
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
/// Own properties right after `new C()`, evaluated the way Kanon captures them.
fn instance_of(class_source: &str) -> Value {
    let script = format!(
        "const C = ({class_source}); const o = new C(); \
         console.log(JSON.stringify(Object.fromEntries(Object.entries(o).map(([k, v]) => \
         [k, v === undefined ? {{ undefined: true }} : v]))));"
    );
    let output = std::process::Command::new("node")
        .arg("-e")
        .arg(script)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}

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
                            // Values typed in earlier demonstrations were recorded untyped; the
                            // runtime value Kanon now captures is the number Kanon now records.
                            let value = match label.parse::<i64>() {
                                Ok(number) if number.to_string() == label => json!(number),
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
    let instance = instance_of(&class_source);
    request["validation"] = json!({"version": 3,
        "classes": [{"name": "Obj", "source": class_source, "instance": instance}], "cases": cases});
    request
}

// These payloads were recorded before Kanon typed literal input, so every literal says "string".
// Re-type them as Kanon's literalInputType now records them.
fn with_typed_literal_input(mut request: Value) -> Value {
    for call in request["method_calls"].as_array_mut().unwrap() {
        for op in call["operations"].as_array_mut().unwrap() {
            let label = op["label"].as_str().unwrap_or_default();
            // The saved literals are integers; canonical integers are numbers under Kanon's rule.
            let canonical = label
                .parse::<i64>()
                .is_ok_and(|number| number.to_string() == label);
            if op["isLiteral"] == true && op["type"] == "string" && canonical {
                op["type"] = json!("number");
            }
        }
    }
    request
}

#[tokio::test]
async fn untyped_numeral_literals_are_not_adopted_as_numbers() {
    let payload: Value = serde_json::from_str(
        &std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/docs/evaluation_cases/append/mold_payload.json"
        ))
        .unwrap(),
    )
    .unwrap();
    let response = run(with_typed_pre_state(payload, obj_class_source("append"))).await;
    let validation = response.validation.unwrap();
    assert!(
        matches!(validation.status, ValidationStatus::Unavailable),
        "{validation:?}"
    );
    assert!(response.composed_method_code.is_none());
    assert!(response
        .list_environment_info
        .unwrap()
        .contains("string-valued holes cannot be encoded"));
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
        let response = run(with_typed_pre_state(
            with_typed_literal_input(payload),
            obj_class_source(case),
        ))
        .await;
        let validation = response.validation.unwrap();
        assert!(
            matches!(validation.status, ValidationStatus::Passed),
            "{case}: {validation:?}\n{:?}",
            response.list_environment_info
        );
        assert_eq!(validation.checked_demonstrations, demonstrations, "{case}");
        assert!(response.composed_method_code.is_some(), "{case}");
    }
}

async fn adopted(case: &str) -> SynthesisResponse {
    let payload: Value = serde_json::from_str(
        &std::fs::read_to_string(format!(
            "{}/docs/evaluation_cases/{case}/mold_payload.json",
            env!("CARGO_MANIFEST_DIR")
        ))
        .unwrap(),
    )
    .unwrap();
    let response = run(with_typed_pre_state(
        with_typed_literal_input(payload),
        obj_class_source(case),
    ))
    .await;
    assert!(
        matches!(
            response.validation.as_ref().unwrap().status,
            ValidationStatus::Passed
        ),
        "{case}: {:?}",
        response.validation
    );
    response
}

/// Runs adopted code on inputs that were never demonstrated. Passing says nothing about
/// inputs beyond these; it only separates "reproduces the demonstrations" from these cases.
fn run_on_unseen_inputs(response: &SynthesisResponse, assertions: &str) {
    let script = format!(
        r#"
const assert = require("node:assert/strict");
class Obj {{
  constructor(f, g = null) {{ this.f = f; this.g = g; }}
  {}
  {}
}}
const list = values => values.reduceRight((next, value) => new Obj(value, next), null);
const snapshot = head => {{
  const values = [];
  const seen = new Set();
  for (let node = head; node !== null; node = node.g) {{
    assert.ok(!seen.has(node), "generated list must not contain a cycle");
    seen.add(node);
    values.push(node.f);
  }}
  return values;
}};
{}
"#,
        response.code.join("\n"),
        response.composed_method_code.as_deref().unwrap(),
        assertions
    );
    let output = std::process::Command::new("node")
        .arg("-e")
        .arg(script)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&output.stderr),
        response.composed_method_code.as_deref().unwrap()
    );
}

fn compare_with_reference(case: &str, response: &SynthesisResponse) {
    let method = if case == "insert_general" {
        "insert"
    } else {
        case
    };
    let payload = json!({
        "method": method,
        "classSource": obj_class_source(case),
        "helpers": response.code,
        "composedMethod": response.composed_method_code.as_deref().unwrap(),
    });
    let report = run_reference_comparison(&payload);
    assert!(report["cases"].as_u64().unwrap() >= 20, "{report}");
    println!("reference comparison: {report}");
}

fn run_reference_comparison(payload: &Value) -> Value {
    use std::io::Write;
    use std::process::{Command, Stdio};

    let mut child = Command::new("node")
        .arg(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/js/list-method-differential.cjs"
        ))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(&serde_json::to_vec(payload).unwrap())
        .unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "{}\npayload: {payload}",
        String::from_utf8_lossy(&output.stderr),
    );
    serde_json::from_slice(&output.stdout).unwrap()
}

#[tokio::test]
async fn adopted_methods_match_the_reference_after_each_seeded_random_call() {
    let mut candidates = Vec::new();
    for case in ["setAt", "insert_general", "append", "prepend", "popBack"] {
        let response = adopted(case).await;
        candidates.push(json!({
            "method": if case == "insert_general" { "insert" } else { case },
            "classSource": obj_class_source(case),
            "helpers": response.code,
            "composedMethod": response.composed_method_code.unwrap(),
        }));
    }
    let report = run_reference_comparison(&json!({
        "classSource": obj_class_source("setAt"),
        "candidates": candidates,
        "sequences": true,
    }));
    assert_eq!(report["sequences"], 288);
    assert_eq!(report["calls"], 14_400);
    println!("random sequence comparison: {report}");
}

#[test]
fn reference_comparator_accepts_correct_methods_and_rejects_faulty_candidates() {
    let output = std::process::Command::new("node")
        .args([
            "--test",
            concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/js/list-method-differential.test.cjs"
            ),
        ])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[tokio::test]
async fn adopted_set_at_matches_the_reference_on_unseen_inputs() {
    let response = adopted("setAt").await;
    compare_with_reference("setAt", &response);
    run_on_unseen_inputs(
        &response,
        r#"
let head = list([3, 7, 11, 19, 23]);
head.setAt(1, 101);
assert.deepEqual(snapshot(head), [3, 101, 11, 19, 23]);
head.setAt(4, -8);
assert.deepEqual(snapshot(head), [3, 101, 11, 19, -8]);
"#,
    );
}

#[tokio::test]
async fn adopted_append_matches_the_reference_on_unseen_inputs() {
    let response = adopted("append").await;
    compare_with_reference("append", &response);
    run_on_unseen_inputs(
        &response,
        r#"
let one = list([4]);
one.append(91);
assert.deepEqual(snapshot(one), [4, 91]);
let many = list([2, 5, 8, 13, 21]);
many.append(-6);
assert.deepEqual(snapshot(many), [2, 5, 8, 13, 21, -6]);
"#,
    );
}

#[tokio::test]
async fn adopted_prepend_matches_the_reference_on_unseen_inputs() {
    let response = adopted("prepend").await;
    compare_with_reference("prepend", &response);
    run_on_unseen_inputs(
        &response,
        r#"
let head = list([7, 12, 18]);
head = head.prepend(44);
assert.deepEqual(snapshot(head), [44, 7, 12, 18]);
head = head.prepend(-3);
assert.deepEqual(snapshot(head), [-3, 44, 7, 12, 18]);
"#,
    );
}

#[tokio::test]
async fn adopted_insert_matches_the_reference_on_unseen_inputs() {
    let response = adopted("insert_general").await;
    assert_eq!(response.hole_information.as_ref().unwrap().len(), 3);
    compare_with_reference("insert_general", &response);
    run_on_unseen_inputs(
        &response,
        r#"
let head = list([5, 10, 20, 40, 80]);
head.insert(0, 7);
assert.deepEqual(snapshot(head), [5, 7, 10, 20, 40, 80]);
head.insert(4, 55);
assert.deepEqual(snapshot(head), [5, 7, 10, 20, 40, 55, 80]);
"#,
    );
}

#[tokio::test]
async fn adopted_pop_back_matches_the_reference_on_unseen_inputs() {
    let response = adopted("popBack").await;
    compare_with_reference("popBack", &response);
    run_on_unseen_inputs(
        &response,
        r#"
let two = list([6, 9]);
two.popBack();
assert.deepEqual(snapshot(two), [6]);
let many = list([1, 3, 5, 7, 9]);
many.popBack();
assert.deepEqual(snapshot(many), [1, 3, 5, 7]);
"#,
    );
}
