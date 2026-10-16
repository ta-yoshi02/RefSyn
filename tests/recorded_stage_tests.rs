//! Runs every request recorded from Kanon in `tests/recordings/` through each stage and checks
//! the values its `expected.json` gives; see `tests/recordings/README.md`.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use refsyn::demonstration::{self, Edit, Value};
use refsyn::operation_graph::{self, Constant, Vertex};
use refsyn::template::{Example, HoleType, Template};
use refsyn::{alignment, escher_bridge, handle_synthesis, SynthesisRequest, SynthesisResponse};
use serde_json::{json, Value as Json};
use warp::Reply;

const ROOT: &str = env!("CARGO_MANIFEST_DIR");

fn value(value: &Value) -> Json {
    match value {
        Value::Undefined => json!({"undefined": true}),
        Value::Null => Json::Null,
        Value::Bool(b) => json!(b),
        Value::Number(n) => json!(n),
        Value::String(s) => json!(s),
        Value::Ref(id) => json!({"ref": id}),
    }
}

fn constant(constant: &Constant) -> Json {
    match constant {
        Constant::Number(n) => json!(n),
        Constant::String(s) => json!(s),
        Constant::Bool(b) => json!(b),
    }
}

fn edit(edit: &Edit) -> Json {
    match edit {
        Edit::AddObject { id, class } => json!({"addObject": id, "class": class}),
        Edit::AddLiteral { id, value: v } => json!({"addLiteral": id, "value": value(v)}),
        Edit::AddEdge { from, field, to } => json!({"addEdge": [from, field, to]}),
        Edit::EditEdge {
            from,
            field,
            old,
            new,
        } => json!({"editEdge": [from, field, new], "old": old}),
        Edit::DeleteEdge { from, field, old } => json!({"deleteEdge": [from, field], "old": old}),
        Edit::SetReturn { to } => json!({"return": to}),
    }
}

fn vertex(vertex: &Vertex) -> Json {
    match vertex {
        Vertex::Existing { class, object } => json!({"existing": object, "class": class}),
        Vertex::Created { class, object } => json!({"created": object, "class": class}),
        Vertex::Constant(c) => json!({"constant": constant(c)}),
        Vertex::Null => json!({"null": true}),
    }
}

fn hole_type(ty: &HoleType) -> String {
    match ty {
        HoleType::Number => "number".into(),
        HoleType::String => "string".into(),
        HoleType::Bool => "boolean".into(),
        HoleType::Object(class) => class.clone(),
    }
}

/// The results of `read`, `graphs`, `template` and `problems`; a stage that fails gives
/// `{"error": ...}` and the later ones are left out.
fn stages_before_synthesis(request: &SynthesisRequest) -> serde_json::Map<String, Json> {
    let mut out = serde_json::Map::new();
    let error = |e: anyhow::Error| json!({"error": format!("{e:#}")});
    let recording = match demonstration::read(request) {
        Ok(recording) => recording,
        Err(e) => {
            out.insert("read".into(), error(e));
            return out;
        }
    };
    let demonstrations = recording
        .demonstrations
        .iter()
        .map(|d| {
            json!({
                "call": d.call_label, "context": d.context_id, "method": d.method,
                "params": d.param_names, "receiver": d.receiver, "receiver_class": d.receiver_class,
                "arguments": d.arguments.iter().map(value).collect::<Vec<_>>(),
                "objects": d.heap.objects.iter().map(|(id, object)| json!({
                    "id": id, "class": object.class,
                    "fields": object.fields.iter().map(|(f, v)| (f.clone(), value(v))).collect::<serde_json::Map<_, _>>(),
                })).collect::<Vec<_>>(),
                "literals": d.literals.iter().map(|(id, v)| (id.clone(), value(v))).collect::<serde_json::Map<_, _>>(),
                "edits": d.edits.iter().map(edit).collect::<Vec<_>>(),
            })
        })
        .collect::<Vec<_>>();
    out.insert("read".into(), json!({"demonstrations": demonstrations}));
    let graphs = match recording
        .demonstrations
        .iter()
        .enumerate()
        .map(|(i, d)| {
            operation_graph::build(d, &recording.classes)
                .map_err(|e| e.context(format!("demonstration {}", i + 1)))
        })
        .collect::<anyhow::Result<Vec<_>>>()
    {
        Ok(graphs) => graphs,
        Err(e) => {
            out.insert("graphs".into(), error(e));
            return out;
        }
    };
    out.insert(
        "graphs".into(),
        graphs
            .iter()
            .map(|g| {
                json!({
                    "vertices": g.vertices.iter().map(vertex).collect::<Vec<_>>(),
                    "writes": g.writes.iter().map(|w| json!([w.from, w.field, w.to])).collect::<Vec<_>>(),
                    "return": g.ret,
                })
            })
            .collect(),
    );
    let first = &recording.demonstrations[0];
    let (mappings, template, problems) =
        match alignment::align_consistently(&recording, &graphs, &first.param_names) {
            Ok(aligned) => aligned,
            Err(e) => {
                out.insert("template".into(), error(e));
                return out;
            }
        };
    let holes = template
        .holes
        .iter()
        .enumerate()
        .map(|(k, hole)| {
            json!({
                "helper": Template::helper_name(&first.method, k),
                "type": hole_type(&hole.ty),
                "outputs": hole.examples.iter().map(|e| match e {
                    Example::Object(id) => json!({"ref": id}),
                    Example::Constant(c) => constant(c),
                }).collect::<Vec<_>>(),
            })
        })
        .collect::<Vec<_>>();
    out.insert(
        "template".into(),
        json!({
            "mappings": mappings,
            "holes": holes,
            "composed_method": template.render_method(&first.method, &first.param_names),
        }),
    );
    let tasks: Json =
        serde_json::from_str(&escher_bridge::tasks_to_json(&problems.tasks).unwrap()).unwrap();
    out.insert(
        "problems".into(),
        json!({"tasks": tasks, "checks": problems.checks}),
    );
    out
}

async fn synthesize(body: &[u8]) -> SynthesisResponse {
    let reply = handle_synthesis(bytes::Bytes::copy_from_slice(body))
        .await
        .unwrap()
        .into_response();
    let body = hyper::body::to_bytes(reply.into_body()).await.unwrap();
    serde_json::from_slice(&body).unwrap()
}

const COMPARISON_TIMEOUT: Duration = Duration::from_secs(120);

/// Runs the adopted method and `reference` on the reference's inputs. Each call is bounded by
/// the script; `timeout` bounds the whole process, including the reference's own code.
fn compare(
    request: &SynthesisRequest,
    response: &SynthesisResponse,
    reference: &Path,
    timeout: Duration,
) -> Json {
    use std::io::{Read, Write};
    use std::process::{Command, Stdio};

    let call = &request.method_calls[0];
    let mut child = Command::new("node")
        .arg(format!("{ROOT}/scripts/compare_with_reference.mjs"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(
            &serde_json::to_vec(&json!({
                "reference": reference,
                "className": call.receiver_class_name,
                "method": call.method_name,
                "helpers": response.code,
                "composedMethod": response.composed_method_code,
            }))
            .unwrap(),
        )
        .unwrap();
    let read_all = |mut pipe: Box<dyn Read + Send>| {
        std::thread::spawn(move || {
            let mut buffer = Vec::new();
            pipe.read_to_end(&mut buffer).map(|_| buffer)
        })
    };
    let stdout = read_all(Box::new(child.stdout.take().unwrap()));
    let stderr = read_all(Box::new(child.stderr.take().unwrap()));
    let deadline = Instant::now() + timeout;
    let status = loop {
        if let Some(status) = child.try_wait().unwrap() {
            break status;
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            return json!({"error": format!("the comparison did not finish within {timeout:?}")});
        }
        std::thread::sleep(Duration::from_millis(10));
    };
    let stdout = stdout.join().unwrap().unwrap();
    let stderr = stderr.join().unwrap().unwrap();
    if !status.success() {
        return json!({"error": String::from_utf8_lossy(&stderr)});
    }
    serde_json::from_slice(&stdout).unwrap()
}

/// Paths at which `expected` is not contained in `actual`.
fn unmet(expected: &Json, actual: &Json, path: &str, out: &mut Vec<String>) {
    match (expected, actual) {
        (Json::Object(e), Json::Object(a)) => {
            for (key, value) in e {
                match a.get(key) {
                    Some(actual) => unmet(value, actual, &format!("{path}/{key}"), out),
                    None => out.push(format!("{path}/{key}: missing")),
                }
            }
        }
        (Json::Array(e), Json::Array(a)) if e.len() == a.len() => {
            for (i, (e, a)) in e.iter().zip(a).enumerate() {
                unmet(e, a, &format!("{path}/{i}"), out);
            }
        }
        _ if expected == actual => {}
        _ => out.push(format!("{path}: expected {expected}, got {actual}")),
    }
}

/// Every stage's result for one recording directory. Synthesis runs only when it is expected
/// and the stages before it succeeded.
async fn results(dir: &Path, expected: &Json) -> Json {
    let body = std::fs::read(dir.join("request.json")).unwrap();
    let request: SynthesisRequest = serde_json::from_slice(&body).unwrap();
    let mut results = stages_before_synthesis(&request);
    let wanted = expected.get("validation").is_some() || expected.get("comparison").is_some();
    if wanted && results.contains_key("problems") {
        let response = synthesize(&body).await;
        results.insert("validation".into(), json!(response.validation));
        if expected.get("comparison").is_some() {
            let reference = dir.join("reference.mjs");
            let comparison = if response.composed_method_code.is_some() {
                compare(&request, &response, &reference, COMPARISON_TIMEOUT)
            } else {
                json!({"error": "no adopted method to compare"})
            };
            results.insert("comparison".into(), comparison);
        }
    }
    Json::Object(results)
}

/// Every subdirectory of `root`, or the files each incomplete one lacks: `request.json`,
/// `expected.json`, and `reference.mjs` when a comparison is expected.
fn recording_dirs(root: &Path) -> Result<Vec<PathBuf>, Vec<String>> {
    let mut dirs: Vec<_> = std::fs::read_dir(root)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.is_dir())
        .collect();
    dirs.sort();
    let mut missing = Vec::new();
    for dir in &dirs {
        let name = dir.file_name().unwrap().to_string_lossy();
        let mut required = vec!["request.json", "expected.json"];
        let compares = std::fs::read_to_string(dir.join("expected.json"))
            .ok()
            .and_then(|text| serde_json::from_str::<Json>(&text).ok())
            .is_some_and(|expected| expected.get("comparison").is_some());
        if compares {
            required.push("reference.mjs");
        }
        for file in required {
            if !dir.join(file).is_file() {
                missing.push(format!("{name}: no {file}"));
            }
        }
    }
    if missing.is_empty() {
        Ok(dirs)
    } else {
        Err(missing)
    }
}

#[tokio::test]
async fn every_recording_gives_the_expected_results_at_each_stage() {
    assert!(
        Path::new(&format!("{ROOT}/external/escher-ts/dist/index.js")).exists(),
        "requires a built external/escher-ts"
    );
    let dirs = recording_dirs(Path::new(&format!("{ROOT}/tests/recordings")))
        .unwrap_or_else(|missing| panic!("incomplete recordings: {missing:#?}"));
    assert!(dirs.len() >= 6, "{dirs:?}");
    let mut failures = Vec::new();
    for dir in &dirs {
        let expected: Json =
            serde_json::from_str(&std::fs::read_to_string(dir.join("expected.json")).unwrap())
                .unwrap();
        let actual = results(dir, &expected).await;
        let mut differences = Vec::new();
        unmet(&expected, &actual, "", &mut differences);
        let name = dir.file_name().unwrap().to_string_lossy();
        println!("recording {name}: {} unmet", differences.len());
        failures.extend(differences.into_iter().map(|d| format!("{name}{d}")));
    }
    assert!(failures.is_empty(), "{failures:#?}");
}

#[test]
fn an_unmet_expectation_names_its_path() {
    let mut differences = Vec::new();
    unmet(
        &json!({"holes": [{"outputs": [89, 46, 72]}, {}], "absent": 1}),
        &json!({"holes": [{"outputs": [89, 46, 71], "type": "number"}, {"x": 0}]}),
        "",
        &mut differences,
    );
    assert_eq!(
        differences,
        [
            "/absent: missing",
            "/holes/0/outputs/2: expected 72, got 71"
        ]
    );
    differences.clear();
    unmet(&json!([1]), &json!([1, 2]), "", &mut differences);
    assert_eq!(differences, [": expected [1], got [1,2]"]);
}

#[test]
fn a_stage_that_rejects_the_recording_reports_why_and_stops() {
    let mut request: Json = serde_json::from_str(
        &std::fs::read_to_string(format!("{ROOT}/tests/recordings/insert/request.json")).unwrap(),
    )
    .unwrap();
    // The first call's editEdgeReference names a previous target that main-new1.next is not.
    let edit = request["method_calls"][0]["operations"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|op| op["editType"] == "editEdgeReference")
        .unwrap();
    edit["oldTo"] = json!("main-new3");
    let results = stages_before_synthesis(&serde_json::from_value(request).unwrap());
    assert!(results["read"].get("demonstrations").is_some());
    let error = results["graphs"]["error"].as_str().unwrap();
    assert!(error.contains("demonstration 1"), "{error}");
    assert!(!results.contains_key("template"));
}

#[test]
fn the_comparison_accepts_equivalent_methods_and_rejects_faulty_ones() {
    let output = std::process::Command::new("node")
        .args([
            "--test",
            &format!("{ROOT}/tests/js/compare-with-reference.test.mjs"),
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

#[test]
fn an_incomplete_recording_is_reported_rather_than_skipped() {
    let root = Path::new(env!("CARGO_TARGET_TMPDIR")).join("incomplete-recordings");
    let _ = std::fs::remove_dir_all(&root);
    for (dir, files) in [
        ("complete", &["request.json", "expected.json"][..]),
        ("only_expected", &["expected.json"][..]),
        (
            "compares_without_reference",
            &["request.json", "expected.json"][..],
        ),
    ] {
        std::fs::create_dir_all(root.join(dir)).unwrap();
        for file in files {
            std::fs::write(root.join(dir).join(file), "{}").unwrap();
        }
    }
    std::fs::write(
        root.join("compares_without_reference/expected.json"),
        r#"{"comparison": {"mismatches": []}}"#,
    )
    .unwrap();
    std::fs::write(root.join("README.md"), "").unwrap();
    assert_eq!(
        recording_dirs(&root).unwrap_err(),
        [
            "compares_without_reference: no reference.mjs",
            "only_expected: no request.json"
        ]
    );
}

#[tokio::test]
async fn synthesis_is_not_run_after_an_earlier_stage_fails() {
    let mut request: Json = serde_json::from_str(
        &std::fs::read_to_string(format!("{ROOT}/tests/recordings/insert/request.json")).unwrap(),
    )
    .unwrap();
    request["method_calls"][0]["operations"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|op| op["editType"] == "editEdgeReference")
        .unwrap()["oldTo"] = json!("main-new3");
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join("stale-recording");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("request.json"), request.to_string()).unwrap();
    let results = results(&dir, &json!({"validation": {}, "comparison": {}})).await;
    assert!(results["graphs"].get("error").is_some(), "{results:#}");
    assert!(results.get("validation").is_none(), "{results:#}");
    assert!(results.get("comparison").is_none(), "{results:#}");
}

#[test]
fn a_comparison_that_does_not_finish_is_stopped_and_reported() {
    let request: SynthesisRequest = serde_json::from_str(
        &std::fs::read_to_string(format!("{ROOT}/tests/recordings/insert/request.json")).unwrap(),
    )
    .unwrap();
    let response: SynthesisResponse = serde_json::from_value(json!({
        "code": [], "composed_method_code": "insert(i, arg) {}", "individual_codes": [],
        "common_pattern": null, "hole_information": null, "list_environment_info": null,
    }))
    .unwrap();
    // The per-call limit does not apply while the reference module itself is loading.
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join("endless-reference");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("reference.mjs"), "while (true) {}\n").unwrap();
    let started = Instant::now();
    let result = compare(
        &request,
        &response,
        &dir.join("reference.mjs"),
        Duration::from_secs(1),
    );
    assert!(started.elapsed() < Duration::from_secs(10));
    assert_eq!(
        result,
        json!({"error": "the comparison did not finish within 1s"})
    );
}
