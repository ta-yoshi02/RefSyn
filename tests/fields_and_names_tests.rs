use refsyn::alignment::align_all;
use refsyn::template::Template;
use refsyn::{demonstration, operation_graph, pbe_problem, SynthesisRequest, ValidationStatus};
use serde_json::{json, Value as Json};

/// One class `C` with `source`, `new C()` state `instance`, and demonstrations of `method`
/// on receiver `r`, each given as (objects, operations, return target).
fn request(source: &str, instance: Json, method: &str, demos: &[(Json, Json)]) -> Json {
    let calls: Vec<Json> = demos
        .iter()
        .enumerate()
        .map(|(i, (_, ops))| {
            json!({ "callLabel": format!("c{i}"), "contextSensitiveID": "main", "receiverObject": "r",
                    "receiverClassName": "C", "methodName": method, "methodParamNames": [],
                    "arguments": [], "operations": ops })
        })
        .collect();
    let cases: Vec<Json> = demos
        .iter()
        .enumerate()
        .map(|(i, (objects, _))| {
            json!({ "callLabel": format!("c{i}"), "contextSensitiveID": "main", "objects": objects, "arguments": [] })
        })
        .collect();
    json!({
        "method_calls": calls,
        "vis_graph": { "nodes": [], "edges": [] },
        "validation": { "version": 3,
            "classes": [{ "name": "C", "source": source, "instance": instance }], "cases": cases },
    })
}

fn object(id: &str, fields: Json) -> Json {
    json!({ "id": id, "className": "C", "fields": fields })
}

fn op(kind: &str, extra: Json) -> Json {
    let mut op = extra;
    op["editType"] = json!(kind);
    op
}

/// `f() { const n = new C(); n.next = this; return n; }` for a class without a constructor.
fn prepend_without_constructor() -> Json {
    let ops = json!([
        op(
            "addNode",
            json!({ "id": "n", "label": "C", "isLiteral": false })
        ),
        op(
            "addEdge",
            json!({ "from": "n", "label": "next", "to": "r" })
        ),
        op("addVariable", json!({ "label": "return", "to": "n" })),
    ]);
    request(
        "class C { f() {} }",
        json!({}),
        "f",
        &[
            (json!([object("r", json!({ "next": null }))]), ops.clone()),
            (
                json!([
                    object("r", json!({ "next": { "ref": "a" } })),
                    object("a", json!({ "next": null }))
                ]),
                ops,
            ),
        ],
    )
}

fn problems(raw: Json) -> anyhow::Result<pbe_problem::Problems> {
    let request: SynthesisRequest = serde_json::from_value(raw)?;
    let recording = demonstration::read(&request)?;
    let graphs = recording
        .demonstrations
        .iter()
        .map(|d| operation_graph::build(d, &recording.classes))
        .collect::<anyhow::Result<Vec<_>>>()?;
    let mappings = align_all(&graphs)?;
    let template = Template::build(&graphs, &mappings)?;
    pbe_problem::build(&recording, &graphs, &mappings, &template, &[])
}

async fn synthesize(raw: Json) -> refsyn::SynthesisResponse {
    use warp::Reply;
    let reply = refsyn::handle_synthesis(bytes::Bytes::from(serde_json::to_vec(&raw).unwrap()))
        .await
        .unwrap()
        .into_response();
    serde_json::from_slice(&hyper::body::to_bytes(reply.into_body()).await.unwrap()).unwrap()
}

fn assert_adopted(response: &refsyn::SynthesisResponse) {
    let validation = response.validation.as_ref().unwrap();
    assert!(
        matches!(validation.status, ValidationStatus::Passed),
        "{validation:?}\n{:?}\n{:?}",
        response.list_environment_info,
        response.individual_codes
    );
}

#[test]
fn updating_objects_of_a_class_without_a_constructor_needs_no_absent_fields() {
    let raw = request(
        "class C { f() {} }",
        json!({}),
        "f",
        &[(
            json!([
                object("r", json!({ "next": null })),
                object("a", json!({ "next": null }))
            ]),
            json!([op(
                "addEdge",
                json!({ "from": "r", "label": "next", "to": "a" })
            )]),
        )],
    );
    assert!(problems(raw).is_ok());
}

#[test]
fn an_absent_pointer_field_of_a_created_object_is_the_error_value_not_null() {
    let tasks =
        serde_json::to_value(problems(prepend_without_constructor()).unwrap().tasks).unwrap();
    // [this, nodeHeap, nextHeap, created0] for the second demonstration: r, a, then the created n.
    let input = &tasks[0]["examples"][1][0];
    assert_eq!(
        input[1][2]["object"]["fields"]["next"],
        json!({ "error": true })
    );
    assert_eq!(
        input[2],
        json!([{ "ref": 1 }, { "ref": -1 }, { "error": true }])
    );
    assert_eq!(
        tasks[0]["classes"][0]["fields"]["next"],
        json!("Ref[Object[C]]")
    );
}

#[tokio::test]
async fn creating_objects_of_a_class_without_a_constructor_is_synthesized_and_validated() {
    let response = synthesize(prepend_without_constructor()).await;
    assert_adopted(&response);
    assert_eq!(
        response.composed_method_code.as_deref().unwrap(),
        "f() {\n    const created0 = new C();\n    const hole0 = this.f_h0(created0);\n    created0.next = hole0;\n    return created0;\n}"
    );
}

#[tokio::test]
async fn a_method_with_a_non_ascii_name_calls_the_helpers_it_defines() {
    let raw = request(
        "class C { 追加() {} }",
        json!({}),
        "追加",
        &[(
            json!([
                object("r", json!({ "next": { "ref": "a" } })),
                object("a", json!({ "next": null }))
            ]),
            json!([op(
                "editEdgeReference",
                json!({ "from": "r", "label": "next", "oldTo": "a", "newTo": "r" })
            )]),
        )],
    );
    let response = synthesize(raw).await;
    assert_adopted(&response);
    assert!(response.code[0].starts_with("追加_h0("));
    assert_eq!(response.hole_checks.unwrap()[0].helper, "追加_h0");
}

#[tokio::test]
async fn fields_that_are_not_identifiers_use_bracket_notation_in_writes_and_reads() {
    let ops = json!([op(
        "editEdgeReference",
        json!({ "from": "r", "label": "data-value",
        "oldTo": "r-data-value", "newTo": "x-data-value" })
    )]);
    let demo = |own: i64, other: i64| {
        (
            json!([
                object("r", json!({ "data-value": own, "link": { "ref": "x" } })),
                object("x", json!({ "data-value": other, "link": null }))
            ]),
            ops.clone(),
        )
    };
    let mut raw = request(
        "class C { f() {} }",
        json!({}),
        "f",
        &[demo(1, 7), demo(4, 9)],
    );
    for (case, (own, other)) in raw["validation"]["cases"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .zip([(1, 7), (4, 9)])
    {
        case["values"] = json!({ "r-data-value": own, "x-data-value": other });
    }
    let response = synthesize(raw).await;
    assert_adopted(&response);
    let method = response.composed_method_code.unwrap();
    assert!(
        method.contains(r#"this["data-value"] = hole1;"#)
            || method.contains(r#"hole0["data-value"] = hole1;"#),
        "{method}"
    );
    assert!(
        response
            .code
            .iter()
            .any(|helper| helper.contains(r#"["data-value"]"#)),
        "{:?}",
        response.code
    );
}

#[test]
fn field_heaps_that_collide_with_other_inputs_are_refused() {
    let raw = request(
        "class C { f() {} }",
        json!({}),
        "f",
        &[(
            json!([
                object("r", json!({ "node": null })),
                object("a", json!({ "node": null }))
            ]),
            json!([op(
                "addEdge",
                json!({ "from": "r", "label": "node", "to": "a" })
            )]),
        )],
    );
    let error = problems(raw)
        .err()
        .expect("nodeHeap is already the class heap");
    assert!(format!("{error:#}").contains("nodeHeap"), "{error:#}");
}

#[tokio::test]
async fn fields_whose_accessors_share_a_name_produce_no_code() {
    let raw = request(
        "class C { f() {} }",
        json!({}),
        "f",
        &[(
            json!([
                object("r", json!({ "next": null, "Next": null })),
                object("a", json!({ "next": null, "Next": null }))
            ]),
            json!([op(
                "addEdge",
                json!({ "from": "r", "label": "next", "to": "a" })
            )]),
        )],
    );
    let response = synthesize(raw).await;
    assert!(response.composed_method_code.is_none() && response.code.is_empty());
    assert!(
        response
            .individual_codes
            .iter()
            .any(|c| c.contains("hasNext")),
        "{:?}",
        response.individual_codes
    );
}
