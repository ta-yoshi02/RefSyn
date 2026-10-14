use refsyn::alignment::{align, align_all};
use refsyn::demonstration::{self, Value};
use refsyn::operation_graph::{self, Constant, Vertex};
use refsyn::pbe_problem;
use refsyn::template::{Example, HoleType, Statement, Template};
use refsyn::SynthesisRequest;
use serde_json::{json, Value as Json};

fn obj(id: &str, val: i64, next: Option<&str>) -> Json {
    json!({ "id": id, "className": "Node", "fields": {
        "val": val, "next": next.map_or(Json::Null, |n| json!({ "ref": n })) } })
}

fn call(label: &str, args: Json, ops: Json) -> Json {
    json!({ "callLabel": label, "contextSensitiveID": "main", "receiverObject": "A",
            "receiverClassName": "Node", "methodName": "insert", "methodParamNames": ["i", "arg"],
            "arguments": args, "operations": ops })
}

fn case(label: &str, objects: Json, args: Json) -> Json {
    json!({ "callLabel": label, "contextSensitiveID": "main", "objects": objects, "arguments": args })
}

fn lit(id: &str, label: &str) -> Json {
    json!({ "editType": "addNode", "id": id, "label": label, "isLiteral": true, "type": "number" })
}

fn add_node(id: &str) -> Json {
    json!({ "editType": "addNode", "id": id, "label": "Node", "isLiteral": false })
}

fn add_edge(from: &str, label: &str, to: &str) -> Json {
    json!({ "editType": "addEdge", "from": from, "label": label, "to": to })
}

fn edit_edge(from: &str, label: &str, old: &str, new: &str) -> Json {
    json!({ "editType": "editEdgeReference", "from": from, "label": label, "oldTo": old, "newTo": new })
}

/// The three insert demonstrations of the paper (§3.1): insert(0,89), insert(2,46), insert(1,71).
fn insert_request() -> Json {
    json!({
        "method_calls": [
            call("c1", json!([0, 89]), json!([add_node("D"), lit("n89", "89"),
                add_edge("D", "val", "n89"), edit_edge("A", "next", "B", "D"), add_edge("D", "next", "B")])),
            call("c2", json!([2, 46]), json!([add_node("E"), lit("n46", "46"),
                add_edge("E", "val", "n46"), edit_edge("B", "next", "C", "E"), add_edge("E", "next", "C")])),
            call("c3", json!([1, 71]), json!([add_node("F"), add_edge("F", "next", "B"),
                edit_edge("D", "next", "B", "F"), lit("n71", "71"), add_edge("F", "val", "n71")])),
        ],
        "vis_graph": { "nodes": [], "edges": [] },
        "validation": {
            "version": 3,
            "classes": [{ "name": "Node", "source": "class Node {}",
                          "instance": { "val": { "undefined": true }, "next": null } }],
            "cases": [
                case("c1", json!([obj("A", 39, Some("B")), obj("B", 27, Some("C")), obj("C", 53, None)]), json!([0, 89])),
                case("c2", json!([obj("A", 39, Some("D")), obj("D", 89, Some("B")), obj("B", 27, Some("C")),
                                  obj("C", 53, None)]), json!([2, 46])),
                case("c3", json!([obj("A", 39, Some("D")), obj("D", 89, Some("B")), obj("B", 27, Some("E")),
                                  obj("E", 46, Some("C")), obj("C", 53, None)]), json!([1, 71])),
            ],
        },
    })
}

fn request(raw: Json) -> SynthesisRequest {
    serde_json::from_value(raw).unwrap()
}

fn graphs(raw: Json) -> anyhow::Result<Vec<operation_graph::OperationGraph>> {
    let recording = demonstration::read(&request(raw))?;
    recording
        .demonstrations
        .iter()
        .map(|d| operation_graph::build(d, &recording.classes))
        .collect()
}

fn single(ops: Json, objects: Json) -> Json {
    let mut raw = insert_request();
    raw["method_calls"] = json!([call("c1", json!([0, 89]), ops)]);
    raw["validation"]["cases"] = json!([case("c1", objects, json!([0, 89]))]);
    raw
}

#[test]
fn the_web_tests_use_the_paper_insert_request() {
    let saved: Json = serde_json::from_str(include_str!("data/paper_insert.json")).unwrap();
    assert_eq!(saved, insert_request());
}

#[test]
fn reading_keeps_null_undefined_and_absent_fields_apart() {
    let mut raw = insert_request();
    raw["validation"]["cases"][0]["objects"][2]["fields"] =
        json!({ "val": { "undefined": true }, "extra": null });
    let recording = demonstration::read(&request(raw)).unwrap();
    let c = recording.demonstrations[0].heap.get("C").unwrap();
    assert_eq!(c.fields.get("val"), Some(&Value::Undefined));
    assert_eq!(c.fields.get("extra"), Some(&Value::Null));
    assert_eq!(c.fields.get("next"), None);
}

#[test]
fn reading_rejects_unknown_references_and_untyped_state() {
    let mut raw = insert_request();
    raw["validation"]["cases"][0]["objects"][2]["fields"]["next"] = json!({ "ref": "Z" });
    assert!(demonstration::read(&request(raw)).is_err());
    let mut raw = insert_request();
    raw.as_object_mut().unwrap().remove("validation");
    assert!(demonstration::read(&request(raw)).is_err());
    let mut raw = insert_request();
    raw["validation"]["version"] = json!(2);
    assert!(demonstration::read(&request(raw)).is_err());
}

#[test]
fn edits_become_writes_in_order_with_duplicates() {
    let raw = single(
        json!([
            add_edge("C", "next", "A"),
            edit_edge("C", "next", "A", "B"),
            edit_edge("C", "next", "B", "A")
        ]),
        json!([
            obj("A", 39, Some("B")),
            obj("B", 27, Some("C")),
            obj("C", 53, None)
        ]),
    );
    let graph = &graphs(raw).unwrap()[0];
    let rendered: Vec<(Vertex, String, Vertex)> = graph
        .writes
        .iter()
        .map(|w| {
            (
                graph.vertices[w.from].clone(),
                w.field.clone(),
                graph.vertices[w.to].clone(),
            )
        })
        .collect();
    let node = |id: &str| Vertex::Existing {
        class: "Node".into(),
        object: id.into(),
    };
    assert_eq!(
        rendered,
        vec![
            (node("C"), "next".into(), node("A")),
            (node("C"), "next".into(), node("B")),
            (node("C"), "next".into(), node("A")),
        ]
    );
}

#[test]
fn delete_edges_write_one_shared_null_vertex() {
    let raw = single(
        json!([{ "editType": "deleteEdge", "from": "A", "label": "next", "to": "B" },
               { "editType": "deleteEdge", "from": "B", "label": "next", "to": "C" }]),
        json!([
            obj("A", 39, Some("B")),
            obj("B", 27, Some("C")),
            obj("C", 53, None)
        ]),
    );
    let graph = &graphs(raw).unwrap()[0];
    assert_eq!(graph.writes.len(), 2);
    assert_eq!(graph.writes[0].to, graph.writes[1].to);
    assert_eq!(graph.vertices[graph.writes[0].to], Vertex::Null);
    assert_eq!(
        graph
            .vertices
            .iter()
            .filter(|v| **v == Vertex::Null)
            .count(),
        1
    );
}

#[test]
fn inconsistent_or_unsupported_edits_fail() {
    let heap = json!([
        obj("A", 39, Some("B")),
        obj("B", 27, Some("C")),
        obj("C", 53, None)
    ]);
    for ops in [
        json!([{ "editType": "deleteEdge", "from": "A", "label": "next", "to": "C" }]),
        json!([{ "editType": "deleteEdge", "from": "Z", "label": "next", "to": "B" }]),
        json!([{ "editType": "deleteEdge", "from": "A", "label": "val", "to": "A-val" }]),
        json!([add_edge("A", "next", "C")]),
        json!([edit_edge("A", "next", "C", "B")]),
        json!([{ "editType": "deleteNode", "id": "A" }]),
        json!([{ "editType": "addVariable", "label": "tmp", "to": "A" }]),
    ] {
        assert!(graphs(single(ops.clone(), heap.clone())).is_err(), "{ops}");
    }
}

#[test]
fn creating_a_class_outside_the_supported_constructors_fails() {
    let mut raw = single(json!([add_node("D")]), json!([obj("A", 39, None)]));
    raw["validation"]["classes"][0] = json!({ "name": "Node", "source": "class Node {}",
        "instanceError": "constructor threw" });
    let err = graphs(raw).unwrap_err();
    assert!(format!("{err:#}").contains("constructor threw"));
}

#[test]
fn insert_demonstrations_correspond_as_in_the_paper() {
    let graphs = graphs(insert_request()).unwrap();
    let mapping = align(&graphs[0], &graphs[1]).unwrap();
    let image = |id: &str| {
        let v = graphs[0]
            .vertices
            .iter()
            .position(|v| matches!(v, Vertex::Existing { object, .. } | Vertex::Created { object, .. } if object == id))
            .unwrap();
        graphs[1].vertices[mapping[v]].clone()
    };
    assert_eq!(
        image("A"),
        Vertex::Existing {
            class: "Node".into(),
            object: "B".into()
        }
    );
    assert_eq!(
        image("B"),
        Vertex::Existing {
            class: "Node".into(),
            object: "C".into()
        }
    );
    assert_eq!(
        image("D"),
        Vertex::Created {
            class: "Node".into(),
            object: "E".into()
        }
    );
    // Demonstration 3 records its writes in a different order; edge order is not compared.
    assert!(align(&graphs[0], &graphs[2]).is_ok());
}

#[test]
fn correspondence_preserves_kind_class_direction_and_multiplicity() {
    let heap = json!([
        obj("A", 39, Some("B")),
        obj("B", 27, Some("C")),
        obj("C", 53, None)
    ]);
    let base = &graphs(single(
        json!([edit_edge("A", "next", "B", "C")]),
        heap.clone(),
    ))
    .unwrap()[0];
    let delete = json!({ "editType": "deleteEdge", "from": "A", "label": "next", "to": "B" });
    let field = json!([{ "editType": "editEdgeReference", "from": "A", "label": "other", "oldTo": "B", "newTo": "C" }]);
    let twice = json!([
        edit_edge("A", "next", "B", "C"),
        edit_edge("A", "next", "C", "C")
    ]);
    let created = json!([add_node("D"), edit_edge("A", "next", "B", "D")]);
    let mut heap_other = heap.clone();
    heap_other[0]["fields"]["other"] = json!({ "ref": "B" });
    for ops in [json!([delete]), field.clone(), created] {
        let h = if ops == field {
            heap_other.clone()
        } else {
            heap.clone()
        };
        let other = &graphs(single(ops.clone(), h)).unwrap_or_else(|e| panic!("{ops}: {e:#}"))[0];
        assert!(align(base, other).is_err(), "{ops}");
    }
    let doubled = &graphs(single(twice, heap.clone())).unwrap()[0];
    assert!(align(doubled, base).is_err());
    let into_created = &graphs(single(
        json!([add_node("D"), edit_edge("A", "next", "B", "D")]),
        heap.clone(),
    ))
    .unwrap()[0];
    let out_of_created = &graphs(single(
        json!([add_node("D"), add_edge("D", "next", "A")]),
        heap,
    ))
    .unwrap()[0];
    assert!(
        align(into_created, out_of_created).is_err(),
        "edge direction is preserved"
    );
}

#[test]
fn insert_template_has_seven_statements_in_the_paper_order() {
    let graphs = graphs(insert_request()).unwrap();
    let mappings = align_all(&graphs).unwrap();
    let template = Template::build(&graphs, &mappings).unwrap();
    let kinds: Vec<&str> = template
        .statements()
        .iter()
        .map(|s| match s {
            Statement::Create(_) => "create",
            Statement::BindHole(..) => "hole",
            Statement::Write(_) => "write",
            Statement::Return(_) => "return",
        })
        .collect();
    assert_eq!(
        kinds,
        ["create", "hole", "hole", "hole", "write", "write", "write"]
    );
    let outputs: Vec<(HoleType, Vec<Example>)> = template
        .holes
        .iter()
        .map(|h| (h.ty.clone(), h.examples.clone()))
        .collect();
    let num = |n: i64| Example::Constant(Constant::Number(n.into()));
    let o = |id: &str| Example::Object(id.into());
    assert_eq!(
        outputs,
        vec![
            (HoleType::Number, vec![num(89), num(46), num(71)]),
            (
                HoleType::Object("Node".into()),
                vec![o("A"), o("B"), o("D")]
            ),
            (
                HoleType::Object("Node".into()),
                vec![o("B"), o("C"), o("B")]
            ),
        ]
    );
    assert_eq!(
        template.render_method("insert", &["i".into(), "arg".into()]),
        "insert(i, arg) {
    const created0 = new Node();
    const hole0 = this.insert_h0(i, arg, created0);
    const hole1 = this.insert_h1(i, arg, created0, hole0);
    const hole2 = this.insert_h2(i, arg, created0, hole0, hole1);
    created0.val = hole0;
    hole1.next = created0;
    created0.next = hole2;
}"
    );
}

#[test]
fn hole_inputs_add_created_objects_and_earlier_holes_but_no_writes() {
    let raw = insert_request();
    let recording = demonstration::read(&request(raw.clone())).unwrap();
    let graphs = graphs(raw).unwrap();
    let mappings = align_all(&graphs).unwrap();
    let template = Template::build(&graphs, &mappings).unwrap();
    let problems = pbe_problem::build(
        &recording,
        &graphs,
        &mappings,
        &template,
        &["i".into(), "arg".into()],
    )
    .unwrap();
    let tasks = serde_json::to_value(&problems.tasks).unwrap();
    // The emitted `receiver.field` throws on null, so field components must fail there too.
    assert!(tasks
        .as_array()
        .unwrap()
        .iter()
        .all(|t| t["strictFieldAccess"] == json!(true)));
    let prev = &tasks[1];
    assert_eq!(
        prev["refsynMeta"]["explicitArgs"]
            .as_array()
            .unwrap()
            .iter()
            .map(|a| a["name"].as_str().unwrap())
            .collect::<Vec<_>>(),
        ["i", "arg", "created0", "hole0"]
    );
    let (input, output) = (&prev["examples"][0][0], &prev["examples"][0][1]);
    // [this, nodeHeap, nextHeap, valHeap, i, arg, created0, hole0]
    assert_eq!(input[0], json!({ "ref": 0 }));
    let next_heap = &input[2];
    assert_eq!(
        next_heap[0],
        json!({ "ref": 1 }),
        "A.next still points to B: writes are not applied"
    );
    assert_eq!(
        next_heap[3],
        json!({ "ref": -1 }),
        "the created node's next is null after new Node()"
    );
    assert_eq!(
        input[3][3],
        json!({ "error": true }),
        "the created node's val is undefined"
    );
    assert_eq!(input[6], json!({ "ref": 3 }));
    assert_eq!(input[7], json!(89));
    assert_eq!(output, &json!({ "ref": 0 }));
    // The first hole sees no earlier hole; the last sees both earlier holes but not itself.
    assert_eq!(tasks[0]["examples"][0][0].as_array().unwrap().len(), 7);
    assert_eq!(tasks[2]["examples"][0][0].as_array().unwrap().len(), 9);
    assert_eq!(
        problems.checks[1].expected,
        vec![
            json!({ "ref": "A" }),
            json!({ "ref": "B" }),
            json!({ "ref": "D" })
        ]
    );
}

async fn synthesize(raw: Json) -> refsyn::SynthesisResponse {
    use warp::Reply;
    let reply = refsyn::handle_synthesis(bytes::Bytes::from(serde_json::to_vec(&raw).unwrap()))
        .await
        .unwrap()
        .into_response();
    serde_json::from_slice(&hyper::body::to_bytes(reply.into_body()).await.unwrap()).unwrap()
}

#[tokio::test]
async fn paper_insert_is_synthesized_validated_and_adopted() {
    let mut raw = insert_request();
    raw["validation"]["classes"][0]["source"] =
        json!("class Node { constructor() { this.val = undefined; this.next = null; } }");
    let response = synthesize(raw).await;
    let validation = response.validation.clone().unwrap();
    assert!(
        matches!(validation.status, refsyn::ValidationStatus::Passed),
        "{validation:?}\n{:#?}",
        response.individual_codes
    );
    assert_eq!(validation.checked_demonstrations, 3);
    println!(
        "{}\n{}",
        response.composed_method_code.unwrap(),
        response.code.join("\n")
    );
}

#[tokio::test]
async fn an_empty_request_produces_no_code() {
    let response =
        synthesize(json!({ "method_calls": [], "vis_graph": { "nodes": [], "edges": [] } })).await;
    assert!(response.composed_method_code.is_none());
    assert!(response.code.is_empty());
}

#[tokio::test]
async fn demonstrations_of_different_methods_are_rejected_before_synthesis() {
    let mut raw = insert_request();
    raw["method_calls"][1]["methodName"] = json!("append");
    let response = synthesize(raw).await;
    assert!(response.composed_method_code.is_none());
    assert!(response
        .list_environment_info
        .unwrap()
        .contains("same method of the same class"));
}

#[test]
fn the_core_hands_the_browser_one_task_per_hole_and_the_hole_checks() {
    let artifacts = refsyn::synthesize_core(
        request(insert_request()),
        refsyn::SynthesisCoreOptions::default(),
    )
    .unwrap();
    let tasks: Vec<Json> = serde_json::from_str(artifacts.task_json.as_deref().unwrap()).unwrap();
    assert_eq!(
        tasks
            .iter()
            .map(|t| t["name"].as_str().unwrap())
            .collect::<Vec<_>>(),
        ["insert_h0", "insert_h1", "insert_h2"]
    );
    let checks = artifacts.response.hole_checks.unwrap();
    assert_eq!(
        checks.iter().map(|c| c.helper.as_str()).collect::<Vec<_>>(),
        ["insert_h0", "insert_h1", "insert_h2"]
    );
    assert!(artifacts
        .response
        .composed_method_code
        .unwrap()
        .contains("this.insert_h2("));
}

#[tokio::test]
async fn non_isomorphic_demonstrations_are_not_synthesized() {
    let mut raw = insert_request();
    // The third demonstration writes next twice more instead of setting val.
    raw["method_calls"][2]["operations"] = json!([
        add_node("F"),
        add_edge("F", "next", "B"),
        edit_edge("D", "next", "B", "F"),
        edit_edge("F", "next", "B", "F")
    ]);
    let response = synthesize(raw).await;
    assert!(response.composed_method_code.is_none());
    assert!(response
        .list_environment_info
        .unwrap()
        .contains("not isomorphic"));
}

fn plan_of(raw: Json) -> (Template, pbe_problem::Problems) {
    let recording = demonstration::read(&request(raw.clone())).unwrap();
    let graphs = graphs(raw).unwrap();
    let mappings = align_all(&graphs).unwrap();
    let template = Template::build(&graphs, &mappings).unwrap();
    let params = recording.demonstrations[0].param_names.clone();
    let problems = pbe_problem::build(&recording, &graphs, &mappings, &template, &params).unwrap();
    (template, problems)
}

#[test]
fn creations_precede_holes_shared_vertices_bind_once_and_writes_keep_order_and_multiplicity() {
    let raw = single(
        json!([add_node("X"), add_node("Y"), add_edge("X", "next", "A"), add_edge("Y", "next", "A"),
               { "editType": "deleteEdge", "from": "A", "label": "next", "to": "B" },
               edit_edge("X", "next", "A", "B"), edit_edge("X", "next", "B", "A")]),
        json!([obj("A", 39, Some("B")), obj("B", 27, None)]),
    );
    let (template, problems) = plan_of(raw);
    assert_eq!(
        template.render_method("insert", &["i".into(), "arg".into()]),
        "insert(i, arg) {
    const created0 = new Node();
    const created1 = new Node();
    const hole0 = this.insert_h0(i, arg, created0, created1);
    const hole1 = this.insert_h1(i, arg, created0, created1, hole0);
    created0.next = hole0;
    created1.next = hole0;
    hole0.next = null;
    created0.next = hole1;
    created0.next = hole0;
}"
    );
    // The null written by deleteEdge is fixed, not a hole; both created objects reach every hole.
    assert_eq!(problems.tasks.len(), 2);
}

#[test]
fn holes_read_the_state_before_any_write_with_user_names_and_detached_objects() {
    let pair = |id: &str, weight: i64| json!({ "id": id, "className": "Item", "fields": { "weight": weight, "link": null } });
    let mut first = pair("A", 1);
    first["fields"]["link"] = json!({ "ref": "B" });
    let call = json!({ "callLabel": "c1", "contextSensitiveID": "main", "receiverObject": "A",
        "receiverClassName": "Item", "methodName": "swap", "methodParamNames": [], "arguments": [],
        "operations": [
            edit_edge("A", "weight", "A-weight", "B-weight"),
            edit_edge("B", "weight", "B-weight", "A-weight"),
        ] });
    let raw = json!({
        "method_calls": [call],
        "vis_graph": { "nodes": [], "edges": [] },
        "validation": { "version": 3,
            "classes": [{ "name": "Item", "source": "class Item {}", "instance": { "weight": 0, "link": null } }],
            "cases": [{ "callLabel": "c1", "contextSensitiveID": "main",
                "objects": [first, pair("B", 2), pair("D", 9)],
                "values": { "A-weight": 1, "B-weight": 2, "D-weight": 9 }, "arguments": [] }] },
    });
    let (template, problems) = plan_of(raw);
    assert_eq!(
        template.render_method("swap", &[]),
        "swap() {
    const hole0 = this.swap_h0();
    const hole1 = this.swap_h1(hole0);
    const hole2 = this.swap_h2(hole0, hole1);
    const hole3 = this.swap_h3(hole0, hole1, hole2);
    hole0.weight = hole1;
    hole2.weight = hole3;
}"
    );
    let tasks = serde_json::to_value(&problems.tasks).unwrap();
    let last = &tasks[3]["examples"][0];
    // [this, nodeHeap, linkHeap, weightHeap, hole0, hole1, hole2]: A's weight is still 1.
    assert_eq!(last[0][3], json!([1, 2, 9]));
    assert_eq!(
        last[0][1].as_array().unwrap().len(),
        3,
        "the detached object D stays in the heap"
    );
    assert_eq!(last[1], json!(1), "B.weight receives A's original weight");
    assert_eq!(
        tasks[0]["classes"][0]["fields"],
        json!({ "link": "Ref[Object[Item]]", "weight": "Ref[Int]" })
    );
}
