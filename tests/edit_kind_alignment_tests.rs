//! Correspondences between demonstrations whose edits are of different Kanon kinds
//! (`addEdge`, `editEdgeReference`, `deleteEdge`, `return`) but mean the same update.
//!
//! Each case below states, before running anything:
//! - the pre-call state and edits of both demonstrations,
//! - the writes each operation graph should hold, as (object, field, value) in order,
//! - every correspondence of the first graph onto the second that preserves vertex kind,
//!   class, constant type, field, direction, multiplicity and the return value,
//! - the hole types and each demonstration's PBE input before the earlier holes.
//!
//! The expected set of correspondences is checked against a brute-force oracle that tries
//! every bijection and uses neither `alignment`'s search nor `Vertex::corresponds`. The chosen
//! correspondence must be in that set, and the template under it must reproduce each
//! demonstration as replayed directly from its edits. Vertex numbers are never compared.

use std::collections::BTreeMap;

use refsyn::alignment::{align, align_consistently, Mapping};
use refsyn::demonstration::{self, Recording};
use refsyn::operation_graph::{self, Constant, OperationGraph, Vertex};
use refsyn::pbe_problem::Problems;
use refsyn::template::{Example, HoleType, Template};
use refsyn::SynthesisRequest;
use serde_json::{json, Value as Json};

// ---------------------------------------------------------------------------------------------
// Fixtures: one class `Cell { value: int, next: Cell, other: Cell }`, `new Cell()` = {0, null, null}.

fn new_cell() -> BTreeMap<String, Json> {
    BTreeMap::from([
        ("value".to_string(), json!(0)),
        ("next".to_string(), Json::Null),
        ("other".to_string(), Json::Null),
    ])
}

/// A pre-call object; `links` sets pointer fields, every other pointer field is null.
fn cell(id: &str, value: i64, links: &[(&str, &str)]) -> Json {
    let mut fields = json!({ "value": value, "next": null, "other": null });
    for (field, target) in links {
        fields[*field] = json!({ "ref": target });
    }
    json!({ "id": id, "className": "Cell", "fields": fields })
}

fn add_node(id: &str) -> Json {
    json!({ "editType": "addNode", "id": id, "label": "Cell", "isLiteral": false })
}

fn number(id: &str, value: i64) -> Json {
    json!({ "editType": "addNode", "id": id, "label": value.to_string(), "isLiteral": true, "type": "number" })
}

fn add_edge(from: &str, field: &str, to: &str) -> Json {
    json!({ "editType": "addEdge", "from": from, "label": field, "to": to })
}

fn edit_edge(from: &str, field: &str, old: &str, new: &str) -> Json {
    json!({ "editType": "editEdgeReference", "from": from, "label": field, "oldTo": old, "newTo": new })
}

fn delete_edge(from: &str, field: &str, old: &str) -> Json {
    json!({ "editType": "deleteEdge", "from": from, "label": field, "to": old })
}

fn set_return(to: &str) -> Json {
    json!({ "editType": "addVariable", "label": "return", "to": to })
}

/// One call: its receiver, arguments, pre-call objects, Kanon's literal nodes for values
/// already in the heap (`<object>-<field>`), and its edits in order.
struct Demo {
    receiver: &'static str,
    args: Json,
    objects: Json,
    values: Json,
    edits: Vec<Json>,
}

fn request(method: &str, params: &[&str], demos: &[Demo]) -> SynthesisRequest {
    let label = |i: usize| format!("c{}", i + 1);
    let calls: Vec<Json> = demos
        .iter()
        .enumerate()
        .map(|(i, d)| {
            json!({ "callLabel": label(i), "contextSensitiveID": "main", "receiverObject": d.receiver,
                "receiverClassName": "Cell", "methodName": method, "methodParamNames": params,
                "arguments": d.args, "operations": d.edits })
        })
        .collect();
    let cases: Vec<Json> = demos
        .iter()
        .enumerate()
        .map(|(i, d)| {
            json!({ "callLabel": label(i), "contextSensitiveID": "main", "objects": d.objects,
                "values": d.values, "arguments": d.args })
        })
        .collect();
    serde_json::from_value(json!({
        "method_calls": calls,
        "vis_graph": { "nodes": [], "edges": [] },
        "validation": { "version": 3, "cases": cases, "classes": [{ "name": "Cell",
            "source": "class Cell { constructor() { this.value = 0; this.next = null; this.other = null; } }",
            "instance": { "value": 0, "next": null, "other": null } }] },
    }))
    .unwrap()
}

// ---------------------------------------------------------------------------------------------
// The independent oracle: every bijection between the vertex sets, kept when it preserves what
// a correspondence must preserve. It reads the graphs only through their public fields.

/// What must agree between corresponding vertices: the kind, plus the class of an object or
/// the type of a constant. The object ID and the constant's value may differ.
fn sort_of(vertex: &Vertex) -> String {
    match vertex {
        Vertex::Existing { class, .. } => format!("existing {class}"),
        Vertex::Created { class, .. } => format!("created {class}"),
        Vertex::Constant(Constant::Number(_)) => "number".into(),
        Vertex::Constant(Constant::String(_)) => "string".into(),
        Vertex::Constant(Constant::Bool(_)) => "bool".into(),
        Vertex::Null => "null".into(),
    }
}

fn bijections(n: usize) -> Vec<Vec<usize>> {
    if n == 0 {
        return vec![vec![]];
    }
    let mut all = Vec::new();
    for shorter in bijections(n - 1) {
        for at in 0..n {
            let mut longer = shorter.clone();
            longer.insert(at, n - 1);
            all.push(longer);
        }
    }
    all
}

fn oracle(a: &OperationGraph, b: &OperationGraph) -> Vec<Mapping> {
    if a.vertices.len() != b.vertices.len() {
        return vec![];
    }
    let mut target: Vec<(usize, &str, usize)> = b
        .writes
        .iter()
        .map(|w| (w.from, w.field.as_str(), w.to))
        .collect();
    target.sort();
    bijections(a.vertices.len())
        .into_iter()
        .filter(|m| {
            let mut image: Vec<(usize, &str, usize)> = a
                .writes
                .iter()
                .map(|w| (m[w.from], w.field.as_str(), m[w.to]))
                .collect();
            image.sort();
            (0..m.len()).all(|v| sort_of(&a.vertices[v]) == sort_of(&b.vertices[m[v]]))
                && image == target
                && a.ret.map(|r| m[r]) == b.ret
        })
        .collect()
}

// ---------------------------------------------------------------------------------------------
// Readings of the stages' results by what each vertex stands for.

/// What a vertex denotes in its own demonstration: an object ID, a number, or null.
fn denotes(vertex: &Vertex) -> Json {
    match vertex {
        Vertex::Existing { object, .. } | Vertex::Created { object, .. } => json!(object),
        Vertex::Constant(Constant::Number(n)) => json!(n),
        Vertex::Constant(other) => panic!("unexpected constant {other:?}"),
        Vertex::Null => Json::Null,
    }
}

fn writes(graph: &OperationGraph) -> Vec<(Json, String, Json)> {
    graph
        .writes
        .iter()
        .map(|w| {
            (
                denotes(&graph.vertices[w.from]),
                w.field.clone(),
                denotes(&graph.vertices[w.to]),
            )
        })
        .collect()
}

/// A correspondence as "what a vertex of the first graph denotes" -> "what its image denotes".
type Roles = BTreeMap<String, Json>;

fn roles_of(a: &OperationGraph, b: &OperationGraph, mapping: &Mapping) -> Roles {
    (0..a.vertices.len())
        .map(|v| {
            (
                denotes(&a.vertices[v]).to_string(),
                denotes(&b.vertices[mapping[v]]),
            )
        })
        .collect()
}

fn roles(pairs: &[(Json, Json)]) -> Roles {
    pairs
        .iter()
        .map(|(a, b)| (a.to_string(), b.clone()))
        .collect()
}

type Heap = BTreeMap<String, BTreeMap<String, Json>>;

fn pre_heap(demo: &Demo) -> Heap {
    demo.objects
        .as_array()
        .unwrap()
        .iter()
        .map(|o| {
            let fields = o["fields"]
                .as_object()
                .unwrap()
                .iter()
                .map(|(f, v)| (f.clone(), v.clone()))
                .collect();
            (o["id"].as_str().unwrap().to_string(), fields)
        })
        .collect()
}

/// The demonstration's own result, by applying its edits to its pre-call state.
fn replay(demo: &Demo) -> (Heap, Option<Json>) {
    let mut heap = pre_heap(demo);
    let mut literals: BTreeMap<String, Json> = demo
        .values
        .as_object()
        .unwrap()
        .iter()
        .map(|(id, v)| (id.clone(), v.clone()))
        .collect();
    let mut ret = None;
    for edit in &demo.edits {
        let id = |key: &str| edit[key].as_str().unwrap().to_string();
        let value = |id: String, literals: &BTreeMap<String, Json>| {
            literals
                .get(&id)
                .cloned()
                .unwrap_or_else(|| json!({ "ref": id }))
        };
        let (target, field, written) = match edit["editType"].as_str().unwrap() {
            "addNode" if edit["isLiteral"] == json!(true) => {
                let n: i64 = edit["label"].as_str().unwrap().parse().unwrap();
                literals.insert(id("id"), json!(n));
                continue;
            }
            "addNode" => {
                heap.insert(id("id"), new_cell());
                continue;
            }
            "addVariable" => {
                ret = Some(value(id("to"), &literals));
                continue;
            }
            "addEdge" => (id("from"), id("label"), value(id("to"), &literals)),
            "editEdgeReference" => (id("from"), id("label"), value(id("newTo"), &literals)),
            "deleteEdge" => (id("from"), id("label"), Json::Null),
            other => panic!("unexpected edit {other}"),
        };
        heap.get_mut(&target).unwrap().insert(field, written);
    }
    (heap, ret)
}

/// Runs `template` on demonstration `i`'s pre-call state, creating each object the template
/// creates as its image under `mapping` and binding each hole to its output there.
fn run_template(
    demo: &Demo,
    graph: &OperationGraph,
    template: &Template,
    mapping: &Mapping,
    i: usize,
) -> (Heap, Option<Json>) {
    let mut heap = pre_heap(demo);
    let mut bound: BTreeMap<usize, Json> = BTreeMap::new();
    for create in &template.creates {
        let Vertex::Created { object, .. } = &graph.vertices[mapping[create.vertex]] else {
            panic!("a creation corresponds to a non-creation");
        };
        heap.insert(object.clone(), new_cell());
        bound.insert(create.vertex, json!({ "ref": object }));
    }
    for hole in &template.holes {
        let value = match &hole.examples[i] {
            Example::Object(id) => json!({ "ref": id }),
            Example::Constant(Constant::Number(n)) => json!(n),
            Example::Constant(other) => panic!("unexpected example {other:?}"),
        };
        bound.insert(hole.vertex, value);
    }
    // A vertex that is neither created nor a hole is the shared null.
    let get = |v: usize| bound.get(&v).cloned().unwrap_or(Json::Null);
    for write in &template.writes {
        let target = get(write.from)["ref"].as_str().unwrap().to_string();
        heap.get_mut(&target)
            .unwrap()
            .insert(write.field.clone(), get(write.to));
    }
    (heap, template.ret.map(get))
}

#[derive(Debug, PartialEq)]
struct PbeInput {
    receiver: Json,
    /// `nodeHeap`: every object's slots; an integer slot names the object's own entry.
    objects: Json,
    /// `<field>Heap` for every integer and pointer field.
    field_heaps: BTreeMap<String, Json>,
    args: Vec<Json>,
    created: Vec<Json>,
    earlier_holes: Vec<Json>,
    output: Json,
}

fn r(index: i64) -> Json {
    json!({ "ref": index })
}

/// The input a hole receives besides the earlier holes and its output. Objects are indexed in
/// the order pre-call objects (by ID), then created objects in the template's creation order;
/// `slots` gives each one's `next` and `other` (-1 for null).
fn pre(
    receiver: i64,
    slots: &[(i64, i64)],
    values: &[i64],
    args: Json,
    created: &[i64],
) -> PbeInput {
    let column = |f: fn(&(i64, i64)) -> i64| Json::Array(slots.iter().map(|s| r(f(s))).collect());
    let objects = slots
        .iter()
        .enumerate()
        .map(|(i, (next, other))| {
            json!({ "object": { "className": "Cell", "fields": {
            "next": r(*next), "other": r(*other), "value": r(i as i64) } } })
        })
        .collect();
    PbeInput {
        receiver: r(receiver),
        objects: Json::Array(objects),
        field_heaps: BTreeMap::from([
            ("next".to_string(), column(|s| s.0)),
            ("other".to_string(), column(|s| s.1)),
            ("value".to_string(), json!(values)),
        ]),
        args: args.as_array().unwrap().clone(),
        created: created.iter().map(|c| r(*c)).collect(),
        earlier_holes: vec![],
        output: Json::Null,
    }
}

fn pbe_input(problems: &Problems, params: usize, created: usize, k: usize, i: usize) -> PbeInput {
    let task = serde_json::to_value(&problems.tasks[k]).unwrap();
    let fields: Vec<String> = task["classes"][0]["fields"]
        .as_object()
        .unwrap()
        .keys()
        .cloned()
        .collect();
    let input = task["examples"][i][0].as_array().unwrap().clone();
    let heaps = 2 + fields.len();
    PbeInput {
        receiver: input[0].clone(),
        objects: input[1].clone(),
        field_heaps: fields
            .iter()
            .enumerate()
            .map(|(f, field)| (field.clone(), input[2 + f].clone()))
            .collect(),
        args: input[heaps..heaps + params].to_vec(),
        created: input[heaps + params..heaps + params + created].to_vec(),
        earlier_holes: input[heaps + params + created..].to_vec(),
        output: task["examples"][i][1].clone(),
    }
}

// ---------------------------------------------------------------------------------------------
// The check every accepted case goes through.

fn read_and_build(request: &SynthesisRequest) -> anyhow::Result<(Recording, Vec<OperationGraph>)> {
    let recording = demonstration::read(request)?;
    let graphs = recording
        .demonstrations
        .iter()
        .map(|d| operation_graph::build(d, &recording.classes))
        .collect::<anyhow::Result<Vec<_>>>()?;
    Ok((recording, graphs))
}

/// One correspondence the first graph may map onto the second by, and whether the template
/// under it reproduces the second demonstration.
struct Allowed {
    roles: Roles,
    reproduces: bool,
}

fn allowed(pairs: &[(Json, Json)]) -> Allowed {
    Allowed {
        roles: roles(pairs),
        reproduces: true,
    }
}

struct Case {
    method: &'static str,
    params: &'static [&'static str],
    demos: [Demo; 2],
    writes: [Vec<(Json, &'static str, Json)>; 2],
    allowed: Vec<Allowed>,
    holes: Vec<(Json, HoleType)>,
    /// Per demonstration: the hole input before the earlier holes, and the object order that
    /// encodes an object ID as an index.
    pre: [(PbeInput, &'static [&'static str]); 2],
}

fn check(case: Case) {
    let request = request(case.method, case.params, &case.demos);
    let (recording, graphs) = read_and_build(&request).unwrap();
    for (i, expected) in case.writes.iter().enumerate() {
        let expected: Vec<(Json, String, Json)> = expected
            .iter()
            .map(|(o, f, v)| (o.clone(), f.to_string(), v.clone()))
            .collect();
        assert_eq!(
            writes(&graphs[i]),
            expected,
            "writes of demonstration {}",
            i + 1
        );
    }

    // The oracle finds exactly the correspondences derived from the edits.
    let found = oracle(&graphs[0], &graphs[1]);
    let mut found_roles: Vec<String> = found
        .iter()
        .map(|m| format!("{:?}", roles_of(&graphs[0], &graphs[1], m)))
        .collect();
    let mut expected_roles: Vec<String> = case
        .allowed
        .iter()
        .map(|a| format!("{:?}", a.roles))
        .collect();
    found_roles.sort();
    expected_roles.sort();
    assert_eq!(found_roles, expected_roles, "the correspondences");

    // The first isomorphism `align` finds, and the one `align_consistently` picks, are among them.
    let first_found = align(&graphs[0], &graphs[1]).unwrap();
    assert!(found.contains(&first_found), "align: {first_found:?}");
    let params = recording.demonstrations[0].param_names.clone();
    let (mappings, template, problems) = align_consistently(&recording, &graphs, &params).unwrap();
    let identity: Mapping = (0..graphs[0].vertices.len()).collect();
    assert_eq!(mappings[0], identity);
    assert!(found.contains(&mappings[1]), "chosen: {:?}", mappings[1]);
    let chosen = roles_of(&graphs[0], &graphs[1], &mappings[1]);
    let image = |i: usize, v: &Json| {
        if i == 0 {
            v.clone()
        } else {
            chosen[&v.to_string()].clone()
        }
    };

    // Holes: the first graph's existing objects and constants, each with its own type and its
    // image as the output in each demonstration.
    let mut holes: Vec<(Json, HoleType)> = template
        .holes
        .iter()
        .map(|h| {
            let denoted = denotes(&graphs[0].vertices[h.vertex]);
            for (i, example) in h.examples.iter().enumerate() {
                let output = match example {
                    Example::Object(id) => json!(id),
                    Example::Constant(Constant::Number(n)) => json!(n),
                    Example::Constant(other) => panic!("unexpected example {other:?}"),
                };
                assert_eq!(
                    output,
                    image(i, &denoted),
                    "hole for {denoted}, demonstration {}",
                    i + 1
                );
            }
            (denoted, h.ty.clone())
        })
        .collect();
    let mut types = case.holes.clone();
    holes.sort_by_key(|(d, _)| d.to_string());
    types.sort_by_key(|(d, _)| d.to_string());
    assert_eq!(holes, types, "hole types");

    // Each hole's PBE input holds the demonstration's own pre-call state, its arguments, the
    // created objects and the earlier holes' outputs; its output is the hole's image.
    for (i, (before, order)) in case.pre.iter().enumerate() {
        let encode = |v: &Json| match v.as_str() {
            Some(id) => r(order.iter().position(|o| *o == id).unwrap() as i64),
            None => v.clone(),
        };
        let denoted: Vec<Json> = template
            .holes
            .iter()
            .map(|h| denotes(&graphs[0].vertices[h.vertex]))
            .collect();
        for (k, d) in denoted.iter().enumerate() {
            let input = pbe_input(&problems, params.len(), template.creates.len(), k, i);
            let context = format!("demonstration {}, hole for {d}", i + 1);
            assert_eq!(input.receiver, before.receiver, "{context}");
            assert_eq!(input.objects, before.objects, "{context}");
            assert_eq!(input.field_heaps, before.field_heaps, "{context}");
            assert_eq!(input.args, before.args, "{context}");
            assert_eq!(input.created, before.created, "{context}");
            let earlier: Vec<Json> = denoted[..k].iter().map(|e| encode(&image(i, e))).collect();
            assert_eq!(input.earlier_holes, earlier, "{context}");
            assert_eq!(input.output, encode(&image(i, d)), "{context}");
        }
    }

    // The template under the chosen correspondence reproduces both demonstrations; under every
    // other allowed one it reproduces the second exactly when the case says so.
    for (i, demo) in case.demos.iter().enumerate() {
        assert_eq!(
            run_template(demo, &graphs[i], &template, &mappings[i], i),
            replay(demo),
            "the chosen correspondence, demonstration {}",
            i + 1
        );
    }
    for m in &found {
        let roles = roles_of(&graphs[0], &graphs[1], m);
        let a = case.allowed.iter().find(|a| a.roles == roles).unwrap();
        let alternative = Template::build(&graphs, &[identity.clone(), m.clone()]).unwrap();
        let reproduced =
            run_template(&case.demos[1], &graphs[1], &alternative, m, 1) == replay(&case.demos[1]);
        assert_eq!(reproduced, a.reproduces, "under {roles:?}");
    }
}

/// Two demonstrations whose graphs are not isomorphic: they normalize, the oracle finds no
/// correspondence, `align` agrees, and the planning stops with `message`.
fn assert_not_isomorphic(demos: [Demo; 2], message: &str) {
    let request = request("m", &[], &demos);
    let (recording, graphs) = read_and_build(&request).unwrap();
    assert!(oracle(&graphs[0], &graphs[1]).is_empty());
    assert!(align(&graphs[0], &graphs[1]).is_err());
    let params = recording.demonstrations[0].param_names.clone();
    let err = align_consistently(&recording, &graphs, &params)
        .err()
        .expect("non-isomorphic demonstrations were aligned");
    assert!(format!("{err:#}").contains(message), "{err:#}");
}

fn demo(receiver: &'static str, objects: Json, values: Json, edits: Vec<Json>) -> Demo {
    Demo {
        receiver,
        args: json!([]),
        objects,
        values,
        edits,
    }
}

fn cell_type() -> HoleType {
    HoleType::Object("Cell".into())
}

// ---------------------------------------------------------------------------------------------
// The oracle itself, on graphs small enough to count by hand.

#[test]
fn the_oracle_tries_every_bijection_and_keeps_only_structure_preserving_ones() {
    assert_eq!(bijections(0).len(), 1);
    assert_eq!(bijections(4).len(), 24);
    let mut five = bijections(5);
    five.sort();
    five.dedup();
    assert_eq!(five.len(), 120);

    // A.next = B and the self-loop C.other = C, onto the same writes in the other order: of the
    // six bijections only A -> X, B -> Y, C -> Z keeps both writes.
    let objects = |ids: [&str; 3]| json!(ids.map(|id| cell(id, 0, &[])));
    let touch =
        |ids: [&'static str; 3], edits: Vec<Json>| demo(ids[0], objects(ids), json!({}), edits);
    let (_, graphs) = read_and_build(&request(
        "m",
        &[],
        &[
            touch(
                ["A", "B", "C"],
                vec![add_edge("A", "next", "B"), add_edge("C", "other", "C")],
            ),
            touch(
                ["X", "Y", "Z"],
                vec![add_edge("Z", "other", "Z"), add_edge("X", "next", "Y")],
            ),
        ],
    ))
    .unwrap();
    let found = oracle(&graphs[0], &graphs[1]);
    assert_eq!(
        found
            .iter()
            .map(|m| roles_of(&graphs[0], &graphs[1], m))
            .collect::<Vec<_>>(),
        [roles(&[
            (json!("A"), json!("X")),
            (json!("B"), json!("Y")),
            (json!("C"), json!("Z"))
        ])]
    );
}

// ---------------------------------------------------------------------------------------------
// 1. One write: addEdge onto editEdgeReference, and deleteEdge onto deleteEdge.

/// A(1) and B(2), A.next null: `addEdge(A, next, B)`. X(5) -> Y(6), and Z(7):
/// `editEdgeReference(X, next, Y, Z)`. Both set the receiver's `next` to another cell, so
/// A -> X and B -> Z; Y is not in the graph. Each PBE input holds its own pre-call state, so
/// the second still has X.next = Y.
fn add_then_edit() -> [Demo; 2] {
    [
        demo(
            "A",
            json!([cell("A", 1, &[]), cell("B", 2, &[])]),
            json!({}),
            vec![add_edge("A", "next", "B")],
        ),
        demo(
            "X",
            json!([
                cell("X", 5, &[("next", "Y")]),
                cell("Y", 6, &[]),
                cell("Z", 7, &[])
            ]),
            json!({}),
            vec![edit_edge("X", "next", "Y", "Z")],
        ),
    ]
}

#[test]
fn add_edge_and_edit_edge_reference_on_one_field_correspond() {
    check(Case {
        method: "link",
        params: &[],
        demos: add_then_edit(),
        writes: [
            vec![(json!("A"), "next", json!("B"))],
            vec![(json!("X"), "next", json!("Z"))],
        ],
        allowed: vec![allowed(&[
            (json!("A"), json!("X")),
            (json!("B"), json!("Z")),
        ])],
        holes: vec![(json!("A"), cell_type()), (json!("B"), cell_type())],
        pre: [
            (
                pre(0, &[(-1, -1), (-1, -1)], &[1, 2], json!([]), &[]),
                &["A", "B"],
            ),
            (
                pre(
                    0,
                    &[(1, -1), (-1, -1), (-1, -1)],
                    &[5, 6, 7],
                    json!([]),
                    &[],
                ),
                &["X", "Y", "Z"],
            ),
        ],
    });
}

#[test]
fn edit_edge_reference_with_a_wrong_old_target_is_rejected_while_normalizing() {
    let [first, mut second] = add_then_edit();
    second.edits = vec![edit_edge("X", "next", "Z", "Z")];
    let err = read_and_build(&request("link", &[], &[first, second]))
        .expect_err("a stale old target was normalized");
    assert!(
        format!("{err:#}").contains("the recorded old target Z of X.next does not match the state"),
        "{err:#}"
    );
}

/// A(1) -> B(2): `deleteEdge(A, next, B)`. X(5) -> Y(6), Z(7) -> X: `deleteEdge(X, next, Y)`.
/// The written value is the shared null in both; the deleted targets are not vertices.
#[test]
fn delete_edges_correspond_through_the_shared_null() {
    check(Case {
        method: "cut",
        params: &[],
        demos: [
            demo(
                "A",
                json!([cell("A", 1, &[("next", "B")]), cell("B", 2, &[])]),
                json!({}),
                vec![delete_edge("A", "next", "B")],
            ),
            demo(
                "X",
                json!([
                    cell("X", 5, &[("next", "Y")]),
                    cell("Y", 6, &[]),
                    cell("Z", 7, &[("next", "X")])
                ]),
                json!({}),
                vec![delete_edge("X", "next", "Y")],
            ),
        ],
        writes: [
            vec![(json!("A"), "next", Json::Null)],
            vec![(json!("X"), "next", Json::Null)],
        ],
        allowed: vec![allowed(&[
            (json!("A"), json!("X")),
            (Json::Null, Json::Null),
        ])],
        holes: vec![(json!("A"), cell_type())],
        pre: [
            (
                pre(0, &[(1, -1), (-1, -1)], &[1, 2], json!([]), &[]),
                &["A", "B"],
            ),
            (
                pre(0, &[(1, -1), (-1, -1), (0, -1)], &[5, 6, 7], json!([]), &[]),
                &["X", "Y", "Z"],
            ),
        ],
    });
}

// ---------------------------------------------------------------------------------------------
// 2. Further shapes, each mixing edit kinds between the demonstrations.

/// Independent writes to different fields. First: A(1), B(2), C(3), all links null;
/// `addEdge(A, other, C)`, `addEdge(B, next, A)`. Second: P(4).other = Q, Q(5).next = R, R(6);
/// the same update with editEdgeReference, in the other order. B -next-> A -other-> C is a
/// chain of distinct fields, so the correspondence is unique.
#[test]
fn independent_writes_of_different_kinds_correspond_by_their_fields() {
    check(Case {
        method: "pair",
        params: &[],
        demos: [
            demo(
                "A",
                json!([cell("A", 1, &[]), cell("B", 2, &[]), cell("C", 3, &[])]),
                json!({}),
                vec![add_edge("A", "other", "C"), add_edge("B", "next", "A")],
            ),
            demo(
                "P",
                json!([
                    cell("P", 4, &[("other", "Q")]),
                    cell("Q", 5, &[("next", "R")]),
                    cell("R", 6, &[])
                ]),
                json!({}),
                vec![
                    edit_edge("Q", "next", "R", "P"),
                    edit_edge("P", "other", "Q", "R"),
                ],
            ),
        ],
        writes: [
            vec![
                (json!("A"), "other", json!("C")),
                (json!("B"), "next", json!("A")),
            ],
            vec![
                (json!("Q"), "next", json!("P")),
                (json!("P"), "other", json!("R")),
            ],
        ],
        allowed: vec![allowed(&[
            (json!("A"), json!("P")),
            (json!("B"), json!("Q")),
            (json!("C"), json!("R")),
        ])],
        holes: vec![
            (json!("A"), cell_type()),
            (json!("B"), cell_type()),
            (json!("C"), cell_type()),
        ],
        pre: [
            (
                pre(
                    0,
                    &[(-1, -1), (-1, -1), (-1, -1)],
                    &[1, 2, 3],
                    json!([]),
                    &[],
                ),
                &["A", "B", "C"],
            ),
            (
                pre(0, &[(-1, 1), (2, -1), (-1, -1)], &[4, 5, 6], json!([]), &[]),
                &["P", "Q", "R"],
            ),
        ],
    });
}

/// Two writes to one field. First: A(1), B(2), C(3); `addEdge(A, next, B)` then
/// `editEdgeReference(A, next, B, C)`, so A ends at C. Second: X(4) -> Y(5), W(7), Z(6);
/// `editEdgeReference(X, next, Y, W)` then `(X, next, W, Z)`, so X ends at Z.
///
/// The graphs ignore write order, so B -> W, C -> Z and B -> Z, C -> W are both isomorphisms.
/// Only the first keeps the last write last: the template writes A.next = B then = C, which
/// under the second leaves X.next = W. Isomorphism alone does not make the choice right; the
/// search takes the right one here because it tries W, declared first, before Z.
#[test]
fn repeated_writes_to_one_field_admit_two_isomorphisms_of_which_one_reproduces() {
    check(Case {
        method: "relink",
        params: &[],
        demos: [
            demo(
                "A",
                json!([cell("A", 1, &[]), cell("B", 2, &[]), cell("C", 3, &[])]),
                json!({}),
                vec![add_edge("A", "next", "B"), edit_edge("A", "next", "B", "C")],
            ),
            demo(
                "X",
                json!([
                    cell("W", 7, &[]),
                    cell("X", 4, &[("next", "Y")]),
                    cell("Y", 5, &[]),
                    cell("Z", 6, &[])
                ]),
                json!({}),
                vec![
                    edit_edge("X", "next", "Y", "W"),
                    edit_edge("X", "next", "W", "Z"),
                ],
            ),
        ],
        writes: [
            vec![
                (json!("A"), "next", json!("B")),
                (json!("A"), "next", json!("C")),
            ],
            vec![
                (json!("X"), "next", json!("W")),
                (json!("X"), "next", json!("Z")),
            ],
        ],
        allowed: vec![
            allowed(&[
                (json!("A"), json!("X")),
                (json!("B"), json!("W")),
                (json!("C"), json!("Z")),
            ]),
            Allowed {
                roles: roles(&[
                    (json!("A"), json!("X")),
                    (json!("B"), json!("Z")),
                    (json!("C"), json!("W")),
                ]),
                reproduces: false,
            },
        ],
        holes: vec![
            (json!("A"), cell_type()),
            (json!("B"), cell_type()),
            (json!("C"), cell_type()),
        ],
        pre: [
            (
                pre(
                    0,
                    &[(-1, -1), (-1, -1), (-1, -1)],
                    &[1, 2, 3],
                    json!([]),
                    &[],
                ),
                &["A", "B", "C"],
            ),
            // W, X, Y, Z by ID; the receiver X is 1.
            (
                pre(
                    1,
                    &[(-1, -1), (2, -1), (-1, -1), (-1, -1)],
                    &[7, 4, 5, 6],
                    json!([]),
                    &[],
                ),
                &["W", "X", "Y", "Z"],
            ),
        ],
    });
}

/// Two cells come to share a target. First: A(1) -> B(2) -> C(3); `addEdge(A, other, C)`,
/// `addEdge(B, other, C)`. Second: P(4) -> Q(5) -> R(6) with P.other = Q;
/// `editEdgeReference(P, other, Q, R)`, `addEdge(Q, other, R)`. A and B are symmetric in the
/// graph and the states differ, so both pairings are allowed and both reproduce.
#[test]
fn shared_targets_admit_both_pairings_of_the_symmetric_sources() {
    check(Case {
        method: "share",
        params: &[],
        demos: [
            demo(
                "A",
                json!([
                    cell("A", 1, &[("next", "B")]),
                    cell("B", 2, &[("next", "C")]),
                    cell("C", 3, &[])
                ]),
                json!({}),
                vec![add_edge("A", "other", "C"), add_edge("B", "other", "C")],
            ),
            demo(
                "P",
                json!([
                    cell("P", 4, &[("next", "Q"), ("other", "Q")]),
                    cell("Q", 5, &[("next", "R")]),
                    cell("R", 6, &[])
                ]),
                json!({}),
                vec![
                    edit_edge("P", "other", "Q", "R"),
                    add_edge("Q", "other", "R"),
                ],
            ),
        ],
        writes: [
            vec![
                (json!("A"), "other", json!("C")),
                (json!("B"), "other", json!("C")),
            ],
            vec![
                (json!("P"), "other", json!("R")),
                (json!("Q"), "other", json!("R")),
            ],
        ],
        allowed: vec![
            allowed(&[
                (json!("A"), json!("P")),
                (json!("B"), json!("Q")),
                (json!("C"), json!("R")),
            ]),
            allowed(&[
                (json!("A"), json!("Q")),
                (json!("B"), json!("P")),
                (json!("C"), json!("R")),
            ]),
        ],
        holes: vec![
            (json!("A"), cell_type()),
            (json!("B"), cell_type()),
            (json!("C"), cell_type()),
        ],
        pre: [
            (
                pre(0, &[(1, -1), (2, -1), (-1, -1)], &[1, 2, 3], json!([]), &[]),
                &["A", "B", "C"],
            ),
            (
                pre(0, &[(1, 1), (2, -1), (-1, -1)], &[4, 5, 6], json!([]), &[]),
                &["P", "Q", "R"],
            ),
        ],
    });
}

/// A two-cell cycle that returns one of its cells. First: A(1), B(2) -> C(3);
/// `addEdge(A, next, B)`, `editEdgeReference(B, next, C, A)`, return A. Second: X(4) -> Z(6),
/// Y(5); `editEdgeReference(X, next, Z, Y)`, `addEdge(Y, next, X)`, return `returned`.
/// The cycle alone is symmetric; the return value decides which cell is A's image.
fn cycle(returned: &str) -> [Demo; 2] {
    [
        demo(
            "A",
            json!([
                cell("A", 1, &[]),
                cell("B", 2, &[("next", "C")]),
                cell("C", 3, &[])
            ]),
            json!({}),
            vec![
                add_edge("A", "next", "B"),
                edit_edge("B", "next", "C", "A"),
                set_return("A"),
            ],
        ),
        demo(
            "X",
            json!([
                cell("X", 4, &[("next", "Z")]),
                cell("Y", 5, &[]),
                cell("Z", 6, &[])
            ]),
            json!({}),
            vec![
                edit_edge("X", "next", "Z", "Y"),
                add_edge("Y", "next", "X"),
                set_return(returned),
            ],
        ),
    ]
}

#[test]
fn a_cycle_of_different_kinds_corresponds_by_the_returned_cell() {
    for (returned, other) in [("X", "Y"), ("Y", "X")] {
        check(Case {
            method: "loop2",
            params: &[],
            demos: cycle(returned),
            writes: [
                vec![
                    (json!("A"), "next", json!("B")),
                    (json!("B"), "next", json!("A")),
                ],
                vec![
                    (json!("X"), "next", json!("Y")),
                    (json!("Y"), "next", json!("X")),
                ],
            ],
            allowed: vec![allowed(&[
                (json!("A"), json!(returned)),
                (json!("B"), json!(other)),
            ])],
            holes: vec![(json!("A"), cell_type()), (json!("B"), cell_type())],
            pre: [
                (
                    pre(
                        0,
                        &[(-1, -1), (2, -1), (-1, -1)],
                        &[1, 2, 3],
                        json!([]),
                        &[],
                    ),
                    &["A", "B", "C"],
                ),
                (
                    pre(
                        0,
                        &[(2, -1), (-1, -1), (-1, -1)],
                        &[4, 5, 6],
                        json!([]),
                        &[],
                    ),
                    &["X", "Y", "Z"],
                ),
            ],
        });
    }
}

/// Two created cells closing a loop through the receiver. First: A(1), all null; create D, E;
/// `addEdge(A, other, D)`, `addEdge(D, next, E)`, `addEdge(E, next, A)`. Second: X(4).other =
/// Y(5); create Q then P; `editEdgeReference(X, other, Y, P)`, `addEdge(P, next, Q)`,
/// `addEdge(Q, next, X)`. The created cells correspond by their links (D -> P, E -> Q), and
/// the PBE inputs list them after the pre-call objects in the template's order D, E.
#[test]
fn two_created_cells_correspond_by_their_links_across_edit_kinds() {
    check(Case {
        method: "hang2",
        params: &[],
        demos: [
            demo(
                "A",
                json!([cell("A", 1, &[])]),
                json!({}),
                vec![
                    add_node("D"),
                    add_node("E"),
                    add_edge("A", "other", "D"),
                    add_edge("D", "next", "E"),
                    add_edge("E", "next", "A"),
                ],
            ),
            demo(
                "X",
                json!([cell("X", 4, &[("other", "Y")]), cell("Y", 5, &[])]),
                json!({}),
                vec![
                    add_node("Q"),
                    add_node("P"),
                    edit_edge("X", "other", "Y", "P"),
                    add_edge("P", "next", "Q"),
                    add_edge("Q", "next", "X"),
                ],
            ),
        ],
        writes: [
            vec![
                (json!("A"), "other", json!("D")),
                (json!("D"), "next", json!("E")),
                (json!("E"), "next", json!("A")),
            ],
            vec![
                (json!("X"), "other", json!("P")),
                (json!("P"), "next", json!("Q")),
                (json!("Q"), "next", json!("X")),
            ],
        ],
        allowed: vec![allowed(&[
            (json!("A"), json!("X")),
            (json!("D"), json!("P")),
            (json!("E"), json!("Q")),
        ])],
        holes: vec![(json!("A"), cell_type())],
        pre: [
            (
                pre(
                    0,
                    &[(-1, -1), (-1, -1), (-1, -1)],
                    &[1, 0, 0],
                    json!([]),
                    &[1, 2],
                ),
                &["A", "D", "E"],
            ),
            (
                pre(
                    0,
                    &[(-1, 1), (-1, -1), (-1, -1), (-1, -1)],
                    &[4, 5, 0, 0],
                    json!([]),
                    &[2, 3],
                ),
                &["X", "Y", "P", "Q"],
            ),
        ],
    });
}

/// Three writes mixing a value and two references. First: A(1), B(2) -> C(3);
/// `addEdge(A, other, C)`, `editEdgeReference(B, next, C, A)`, C.value 3 -> 7. Second:
/// X(4).other = Y(5), Z(6); Z.value 6 -> 8, `addEdge(Y, next, X)`,
/// `editEdgeReference(X, other, Y, Z)`. B -next-> A -other-> C -value-> 7 is unique, and the
/// constant 7 corresponds to 8.
#[test]
fn a_value_write_and_two_reference_writes_of_different_kinds_correspond() {
    check(Case {
        method: "mix3",
        params: &[],
        demos: [
            demo(
                "A",
                json!([
                    cell("A", 1, &[]),
                    cell("B", 2, &[("next", "C")]),
                    cell("C", 3, &[])
                ]),
                json!({ "C-value": 3 }),
                vec![
                    number("n7", 7),
                    add_edge("A", "other", "C"),
                    edit_edge("B", "next", "C", "A"),
                    edit_edge("C", "value", "C-value", "n7"),
                ],
            ),
            demo(
                "X",
                json!([
                    cell("X", 4, &[("other", "Y")]),
                    cell("Y", 5, &[]),
                    cell("Z", 6, &[])
                ]),
                json!({ "Z-value": 6 }),
                vec![
                    number("n8", 8),
                    edit_edge("Z", "value", "Z-value", "n8"),
                    add_edge("Y", "next", "X"),
                    edit_edge("X", "other", "Y", "Z"),
                ],
            ),
        ],
        writes: [
            vec![
                (json!("A"), "other", json!("C")),
                (json!("B"), "next", json!("A")),
                (json!("C"), "value", json!(7)),
            ],
            vec![
                (json!("Z"), "value", json!(8)),
                (json!("Y"), "next", json!("X")),
                (json!("X"), "other", json!("Z")),
            ],
        ],
        allowed: vec![allowed(&[
            (json!("A"), json!("X")),
            (json!("B"), json!("Y")),
            (json!("C"), json!("Z")),
            (json!(7), json!(8)),
        ])],
        holes: vec![
            (json!("A"), cell_type()),
            (json!("B"), cell_type()),
            (json!("C"), cell_type()),
            (json!(7), HoleType::Number),
        ],
        pre: [
            (
                pre(
                    0,
                    &[(-1, -1), (2, -1), (-1, -1)],
                    &[1, 2, 3],
                    json!([]),
                    &[],
                ),
                &["A", "B", "C"],
            ),
            (
                pre(
                    0,
                    &[(-1, 1), (-1, -1), (-1, -1)],
                    &[4, 5, 6],
                    json!([]),
                    &[],
                ),
                &["X", "Y", "Z"],
            ),
        ],
    });
}

/// Five writes: `insertAfter(x)` on a doubly linked list, `other` as `prev`. First: A(1) <-> B(2);
/// create D, D.value = 9, D.next = B, D.prev = A, A.next = D, B.prev = D. Second: X(4) <-> Y(5)
/// <-> Z(6); create N, same update after X with 3, the five writes in another order.
#[test]
fn five_writes_of_a_doubly_linked_insertion_correspond() {
    check(Case {
        method: "insertAfter",
        params: &["x"],
        demos: [
            Demo {
                receiver: "A",
                args: json!([9]),
                objects: json!([
                    cell("A", 1, &[("next", "B")]),
                    cell("B", 2, &[("other", "A")])
                ]),
                values: json!({}),
                edits: vec![
                    add_node("D"),
                    number("n9", 9),
                    add_edge("D", "value", "n9"),
                    add_edge("D", "next", "B"),
                    add_edge("D", "other", "A"),
                    edit_edge("A", "next", "B", "D"),
                    edit_edge("B", "other", "A", "D"),
                ],
            },
            Demo {
                receiver: "X",
                args: json!([3]),
                objects: json!([
                    cell("X", 4, &[("next", "Y")]),
                    cell("Y", 5, &[("next", "Z"), ("other", "X")]),
                    cell("Z", 6, &[("other", "Y")])
                ]),
                values: json!({}),
                edits: vec![
                    add_node("N"),
                    number("n3", 3),
                    edit_edge("Y", "other", "X", "N"),
                    edit_edge("X", "next", "Y", "N"),
                    add_edge("N", "other", "X"),
                    add_edge("N", "next", "Y"),
                    add_edge("N", "value", "n3"),
                ],
            },
        ],
        writes: [
            vec![
                (json!("D"), "value", json!(9)),
                (json!("D"), "next", json!("B")),
                (json!("D"), "other", json!("A")),
                (json!("A"), "next", json!("D")),
                (json!("B"), "other", json!("D")),
            ],
            vec![
                (json!("Y"), "other", json!("N")),
                (json!("X"), "next", json!("N")),
                (json!("N"), "other", json!("X")),
                (json!("N"), "next", json!("Y")),
                (json!("N"), "value", json!(3)),
            ],
        ],
        allowed: vec![allowed(&[
            (json!("A"), json!("X")),
            (json!("B"), json!("Y")),
            (json!("D"), json!("N")),
            (json!(9), json!(3)),
        ])],
        holes: vec![
            (json!("A"), cell_type()),
            (json!("B"), cell_type()),
            (json!(9), HoleType::Number),
        ],
        pre: [
            (
                pre(
                    0,
                    &[(1, -1), (-1, 0), (-1, -1)],
                    &[1, 2, 0],
                    json!([9]),
                    &[2],
                ),
                &["A", "B", "D"],
            ),
            (
                pre(
                    0,
                    &[(1, -1), (2, 0), (-1, 1), (-1, -1)],
                    &[4, 5, 6, 0],
                    json!([3]),
                    &[3],
                ),
                &["X", "Y", "Z", "N"],
            ),
        ],
    });
}

// ---------------------------------------------------------------------------------------------
// 3. Pairs that normalize but must not correspond.

#[test]
fn deleting_a_reference_does_not_correspond_to_replacing_it() {
    // A.next = null against X.next = Z: the written vertex is null in one and a cell in the other.
    assert_not_isomorphic(
        [
            demo(
                "A",
                json!([cell("A", 1, &[("next", "B")]), cell("B", 2, &[])]),
                json!({}),
                vec![delete_edge("A", "next", "B")],
            ),
            demo(
                "X",
                json!([
                    cell("X", 5, &[("next", "Y")]),
                    cell("Y", 6, &[]),
                    cell("Z", 7, &[])
                ]),
                json!({}),
                vec![edit_edge("X", "next", "Y", "Z")],
            ),
        ],
        "not isomorphic",
    );
}

#[test]
fn a_created_target_does_not_correspond_to_an_existing_one() {
    assert_not_isomorphic(
        [
            demo(
                "A",
                json!([cell("A", 1, &[])]),
                json!({}),
                vec![add_node("D"), add_edge("A", "next", "D")],
            ),
            demo(
                "X",
                json!([
                    cell("X", 5, &[("next", "Y")]),
                    cell("Y", 6, &[]),
                    cell("Z", 7, &[])
                ]),
                json!({}),
                vec![edit_edge("X", "next", "Y", "Z")],
            ),
        ],
        "not isomorphic",
    );
}

#[test]
fn writes_to_another_field_or_in_another_shape_do_not_correspond() {
    let three = |ids: [&str; 3]| json!(ids.map(|id| cell(id, 0, &[])));
    // A.next = B against X.other = Y.
    assert_not_isomorphic(
        [
            demo(
                "A",
                three(["A", "B", "C"]),
                json!({}),
                vec![add_edge("A", "next", "B")],
            ),
            demo(
                "X",
                three(["X", "Y", "Z"]),
                json!({}),
                vec![add_edge("X", "other", "Y")],
            ),
        ],
        "not isomorphic",
    );
    // A and B both point to C, against the chain X -> Y -> Z: three vertices and two writes each.
    assert_not_isomorphic(
        [
            demo(
                "A",
                three(["A", "B", "C"]),
                json!({}),
                vec![add_edge("A", "next", "C"), add_edge("B", "next", "C")],
            ),
            demo(
                "X",
                three(["X", "Y", "Z"]),
                json!({}),
                vec![add_edge("X", "next", "Y"), add_edge("Y", "next", "Z")],
            ),
        ],
        "not isomorphic",
    );
}

#[test]
fn returning_in_only_one_demonstration_does_not_correspond() {
    let [first, mut second] = cycle("X");
    second.edits.pop();
    assert_not_isomorphic(
        [first, second],
        "only one of the demonstrations sets the return value",
    );
}

/// Union by rank with `next` as the parent and `value` as the rank. Joining roots of different
/// ranks writes one parent; joining equal ranks also raises the new root's rank. The current
/// specification maps writes one to one, so the two demonstrations are rejected rather than
/// generalized; this records that limit.
#[test]
fn union_by_rank_with_one_and_two_writes_is_rejected_as_a_limit_of_the_specification() {
    assert_not_isomorphic(
        [
            demo(
                "A",
                json!([
                    cell("A", 0, &[("next", "A")]),
                    cell("B", 1, &[("next", "B")])
                ]),
                json!({}),
                vec![edit_edge("A", "next", "A", "B")],
            ),
            demo(
                "X",
                json!([
                    cell("X", 1, &[("next", "X")]),
                    cell("Y", 1, &[("next", "Y")])
                ]),
                json!({ "Y-value": 1 }),
                vec![
                    number("n2", 2),
                    edit_edge("X", "next", "X", "Y"),
                    edit_edge("Y", "value", "Y-value", "n2"),
                ],
            ),
        ],
        "not isomorphic",
    );
}
