use refsyn::alignment::align_all;
use refsyn::template::Template;
use refsyn::{
    demonstration, handle_synthesis, operation_graph, pbe_problem, SynthesisRequest,
    SynthesisResponse, ValidationStatus,
};
use serde_json::{json, Value as Json};
use warp::Reply;

/// `class Flag { on; next; set(b) {} copy() {} }` demonstrations as Kanon records them.
fn fixture(name: &str) -> Json {
    serde_json::from_str(
        &std::fs::read_to_string(format!(
            "{}/tests/data/booleans/{name}.json",
            env!("CARGO_MANIFEST_DIR")
        ))
        .unwrap(),
    )
    .unwrap()
}

/// Sets `r.on` before demonstration `case`, in the objects and in the literal values Kanon keeps.
fn with_pre_state_on(mut raw: Json, case: usize, value: Json) -> Json {
    raw["validation"]["cases"][case]["objects"][0]["fields"]["on"] = value.clone();
    raw["validation"]["cases"][case]["values"]["r-on"] = value;
    raw
}

fn problems(raw: Json) -> anyhow::Result<pbe_problem::Problems> {
    let params: Vec<String> = raw["method_calls"][0]["methodParamNames"]
        .as_array()
        .map(|names| {
            names
                .iter()
                .map(|n| n.as_str().unwrap().to_string())
                .collect()
        })
        .unwrap_or_default();
    let request: SynthesisRequest = serde_json::from_value(raw)?;
    let recording = demonstration::read(&request)?;
    let graphs = recording
        .demonstrations
        .iter()
        .map(|d| operation_graph::build(d, &recording.classes))
        .collect::<anyhow::Result<Vec<_>>>()?;
    let mappings = align_all(&graphs)?;
    let template = Template::build(&graphs, &mappings)?;
    pbe_problem::build(&recording, &graphs, &mappings, &template, &params)
}

fn tasks(raw: Json) -> Json {
    serde_json::to_value(problems(raw).unwrap().tasks).unwrap()
}

fn error(raw: Json) -> String {
    match problems(raw) {
        Ok(_) => panic!("the request was encoded"),
        Err(e) => e.to_string(),
    }
}

#[test]
fn a_boolean_field_is_typed_bool_and_kept_in_its_slot() {
    let tasks = tasks(fixture("set_flag"));
    let task = tasks.as_array().unwrap().last().unwrap();
    assert_eq!(task["classes"][0]["fields"]["on"], json!("Bool"));
    // escher-ts gives heaps to reference fields only, so the inputs and the names must agree.
    assert!(task["refsynMeta"]["fieldHeapNames"].get("on").is_none());
    assert_eq!(task["refsynMeta"]["boolFields"], json!(["on"]));
    let input = &task["examples"][0][0];
    // [this, nodeHeap, nextHeap, b, ...]
    assert_eq!(
        input[1][0]["object"]["fields"]["on"],
        json!(false),
        "the slot holds the boolean itself"
    );
    assert_eq!(input[3], json!(true), "the argument stays a boolean");
}

#[test]
fn boolean_arguments_and_holes_are_typed_bool() {
    let problems = problems(fixture("set_flag")).unwrap();
    let tasks = serde_json::to_value(&problems.tasks).unwrap();
    let task = tasks.as_array().unwrap().last().unwrap();
    assert_eq!(task["signature"]["returnType"], json!("Bool"));
    assert_eq!(
        task["signature"]["args"][0],
        json!({ "name": "b", "type": "Bool" })
    );
    let outputs: Vec<&Json> = task["examples"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| &e[1])
        .collect();
    assert_eq!(outputs, [&json!(true), &json!(false)]);
    let checks = serde_json::to_value(&problems.checks).unwrap();
    assert_eq!(
        checks.as_array().unwrap().last().unwrap()["expected"],
        json!([true, false])
    );
}

#[test]
fn an_undefined_boolean_field_is_the_error_value_not_false() {
    let tasks = tasks(with_pre_state_on(
        fixture("set_flag"),
        0,
        json!({ "undefined": true }),
    ));
    let task = tasks.as_array().unwrap().last().unwrap();
    assert_eq!(
        task["examples"][0][0][1][0]["object"]["fields"]["on"],
        json!({ "error": true })
    );
    assert_eq!(
        task["examples"][1][0][1][0]["object"]["fields"]["on"],
        json!(true)
    );
}

#[test]
fn values_that_are_not_booleans_are_refused_rather_than_converted() {
    for (value, message) in [
        (json!(1), "holds both booleans and integers"),
        (json!(null), "holds both booleans and references or null"),
        (json!("true"), "which escher-ts cannot represent"),
    ] {
        let error = error(with_pre_state_on(fixture("set_flag"), 1, value.clone()));
        assert!(error.contains(message), "{value}: {error}");
    }
    let error = error(fixture("set_flag_string_literal"));
    assert!(error.contains("string-valued holes"), "{error}");
}

async fn run(raw: Json) -> SynthesisResponse {
    let reply = handle_synthesis(bytes::Bytes::from(serde_json::to_vec(&raw).unwrap()))
        .await
        .unwrap()
        .into_response();
    let body = hyper::body::to_bytes(reply.into_body()).await.unwrap();
    serde_json::from_slice(&body).unwrap()
}

/// Runs the adopted method in Node against expectations written here, not derived from the
/// demonstrations, on the demonstrated inputs and on others.
fn run_adopted(response: &SynthesisResponse, assertions: &str) {
    let script = format!(
        r#"
const assert = require("node:assert/strict");
class Flag {{
  constructor() {{ this.on = false; this.next = null; }}
  {}
  {}
}}
const flag = (on, next = null) => {{ const f = new Flag(); f.on = on; f.next = next; return f; }}
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

fn passed(response: &SynthesisResponse) -> bool {
    matches!(
        response.validation.as_ref().map(|v| &v.status),
        Some(ValidationStatus::Passed)
    )
}

#[tokio::test]
async fn a_boolean_argument_is_assigned_to_a_field() {
    let response = run(fixture("set_flag")).await;
    assert!(passed(&response), "{:?}", response.validation);
    run_adopted(
        &response,
        r#"
for (const [before, b] of [[false, true], [true, false], [true, true], [false, false]]) {
  const r = flag(before);
  r.set(b);
  assert.equal(r.on, b);
}
"#,
    );
}

#[tokio::test]
async fn a_boolean_field_is_copied_from_another_object() {
    let response = run(fixture("copy_flag")).await;
    assert!(passed(&response), "{:?}", response.validation);
    run_adopted(
        &response,
        r#"
for (const [mine, theirs] of [[true, false], [false, true], [true, true], [false, false]]) {
  const d = flag(theirs);
  const r = flag(mine, d);
  r.copy();
  assert.equal(d.on, mine);
  assert.equal(r.on, mine);
  assert.equal(r.next, d);
}
// The read does not take undefined as false.
const d = flag(true);
const r = flag(undefined, d);
assert.throws(() => r.copy(), TypeError);
"#,
    );
}

#[tokio::test]
async fn demonstrated_numbers_stay_numbers() {
    let response = run(fixture("set_flag_number_literal")).await;
    assert!(passed(&response), "{:?}", response.validation);
    run_adopted(
        &response,
        r#"
for (const [b, expected] of [[true, 1], [false, 0]]) {
  const r = flag(false);
  r.set(b);
  assert.equal(r.on, expected);
}
"#,
    );
}

#[tokio::test]
async fn demonstrated_strings_are_not_taken_for_booleans() {
    let response = run(fixture("set_flag_string_literal")).await;
    assert!(!passed(&response), "{:?}", response.validation);
    assert!(response.code.is_empty());
}
