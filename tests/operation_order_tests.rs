//! Demonstrations whose edits are the same up to order. Every fixture is a small `Cell` heap
//! written out below, and every expected correspondence, hole output and PBE input is derived
//! from what the edits mean, not copied from the implementation's output.
//!
//! The stages treat order differently, and the tests follow that split:
//! - `operation_graph::build` keeps the writes in order, duplicates included.
//! - `alignment::align_consistently` ignores write order: it maps the first graph onto each
//!   other graph by structure, and of several such maps takes the first whose hole examples do
//!   not give one input two outputs.
//! - `Template::build` keeps the first demonstration's write order.
//! - `pbe_problem::build` gives each hole the pre-call state, the arguments, the created objects
//!   in the template's creation order and the earlier holes' outputs, with no write applied.
//! - Whether the template reproduces a demonstration whose order means something else is left to
//!   the whole-method validation.

use std::collections::BTreeMap;

use refsyn::alignment::{align_consistently, Mapping};
use refsyn::demonstration::{self, Recording, Value};
use refsyn::operation_graph::{self, Constant, OperationGraph, Vertex};
use refsyn::pbe_problem::Problems;
use refsyn::template::{Example, HoleType, Template};
use refsyn::SynthesisRequest;
use serde_json::{json, Value as Json};

// ---------------------------------------------------------------------------------------------
// Fixtures: one class `Cell { value: int, next: Cell, other: Cell }`, `new Cell()` = {0, null, null}.

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

/// One call: its receiver, arguments, pre-call objects and Kanon's literal nodes
/// (`<object>-<field>` for values already in the heap), and its edits in order.
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
// The stages under test, and readings of their results by object ID rather than vertex number.

struct Planned {
    recording: Recording,
    graphs: Vec<OperationGraph>,
    mappings: Vec<Mapping>,
    template: Template,
    problems: Problems,
}

fn read_and_build(request: &SynthesisRequest) -> anyhow::Result<(Recording, Vec<OperationGraph>)> {
    let recording = demonstration::read(request)?;
    let graphs = recording
        .demonstrations
        .iter()
        .map(|d| operation_graph::build(d, &recording.classes))
        .collect::<anyhow::Result<Vec<_>>>()?;
    Ok((recording, graphs))
}

fn plan(request: &SynthesisRequest) -> anyhow::Result<Planned> {
    let (recording, graphs) = read_and_build(request)?;
    let params = recording.demonstrations[0].param_names.clone();
    let (mappings, template, problems) = align_consistently(&recording, &graphs, &params)?;
    Ok(Planned {
        recording,
        graphs,
        mappings,
        template,
        problems,
    })
}

/// What a vertex denotes in its own demonstration: an object ID, a number, or null.
fn denotes(vertex: &Vertex) -> Json {
    match vertex {
        Vertex::Existing { object, .. } | Vertex::Created { object, .. } => json!(object),
        Vertex::Constant(Constant::Number(n)) => json!(n),
        Vertex::Constant(other) => panic!("unexpected constant {other:?}"),
        Vertex::Null => Json::Null,
    }
}

/// Writes as (object, field, value) in order.
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

impl Planned {
    /// For each vertex of the first graph, what it denotes there and what its image denotes in
    /// demonstration `i`.
    fn correspondence(&self, i: usize) -> BTreeMap<String, Json> {
        let first = &self.graphs[0];
        (0..first.vertices.len())
            .map(|v| {
                (
                    denotes(&first.vertices[v]).to_string(),
                    denotes(&self.graphs[i].vertices[self.mappings[i][v]]),
                )
            })
            .collect()
    }

    /// Each hole as (what it denotes in the first demonstration, type, output per demonstration).
    fn holes(&self) -> Vec<(Json, HoleType, Vec<Json>)> {
        self.template
            .holes
            .iter()
            .map(|h| {
                let outputs = h
                    .examples
                    .iter()
                    .map(|e| match e {
                        Example::Object(id) => json!(id),
                        Example::Constant(Constant::Number(n)) => json!(n),
                        Example::Constant(other) => panic!("unexpected example {other:?}"),
                    })
                    .collect();
                (
                    denotes(&self.graphs[0].vertices[h.vertex]),
                    h.ty.clone(),
                    outputs,
                )
            })
            .collect()
    }

    /// Runs the template on demonstration `i`'s pre-call state, binding each hole to its output
    /// there; returns every object's fields afterwards.
    fn run_template(&self, i: usize) -> BTreeMap<String, BTreeMap<String, Value>> {
        let demo = &self.recording.demonstrations[i];
        let mut heap: BTreeMap<String, BTreeMap<String, Value>> = demo
            .heap
            .objects
            .iter()
            .map(|(id, o)| (id.clone(), o.fields.clone()))
            .collect();
        let mut bound: BTreeMap<usize, Value> = BTreeMap::new();
        for create in &self.template.creates {
            let Vertex::Created { object, class } =
                &self.graphs[i].vertices[self.mappings[i][create.vertex]]
            else {
                panic!("a creation corresponds to a non-creation");
            };
            let instance = self.recording.classes[class].instance.clone().unwrap();
            heap.insert(object.clone(), instance);
            bound.insert(create.vertex, Value::Ref(object.clone()));
        }
        for hole in &self.template.holes {
            let value = match &hole.examples[i] {
                Example::Object(id) => Value::Ref(id.clone()),
                Example::Constant(Constant::Number(n)) => Value::Number(n.clone()),
                Example::Constant(other) => panic!("unexpected example {other:?}"),
            };
            bound.insert(hole.vertex, value);
        }
        for write in &self.template.writes {
            // A vertex that is neither created nor a hole is the shared null.
            let Value::Ref(target) = &bound[&write.from] else {
                panic!("write to a non-object");
            };
            let value = bound.get(&write.to).cloned().unwrap_or(Value::Null);
            heap.get_mut(target)
                .unwrap()
                .insert(write.field.clone(), value);
        }
        heap
    }

    /// Hole `k`'s PBE input in demonstration `i`, split into its parts.
    fn pbe_input(&self, k: usize, i: usize) -> PbeInput {
        let task = serde_json::to_value(&self.problems.tasks[k]).unwrap();
        let fields: Vec<String> = task["classes"][0]["fields"]
            .as_object()
            .unwrap()
            .keys()
            .cloned()
            .collect();
        let (input, output) = (
            task["examples"][i][0].as_array().unwrap().clone(),
            task["examples"][i][1].clone(),
        );
        let params = self.recording.demonstrations[0].param_names.len();
        let created = self.template.creates.len();
        let heaps = 2 + fields.len();
        let mut field_heaps = BTreeMap::new();
        for (f, field) in fields.iter().enumerate() {
            field_heaps.insert(field.clone(), input[2 + f].clone());
        }
        PbeInput {
            receiver: input[0].clone(),
            objects: input[1].clone(),
            field_heaps,
            args: input[heaps..heaps + params].to_vec(),
            created: input[heaps + params..heaps + params + created].to_vec(),
            earlier_holes: input[heaps + params + created..].to_vec(),
            output,
        }
    }
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

/// The fields of `new Cell()` with given links, as the template's run should leave them.
fn fields(value: i64, next: Option<&str>, other: Option<&str>) -> BTreeMap<String, Value> {
    let link = |l: Option<&str>| l.map_or(Value::Null, |id| Value::Ref(id.to_string()));
    BTreeMap::from([
        ("value".to_string(), Value::Number(value.into())),
        ("next".to_string(), link(next)),
        ("other".to_string(), link(other)),
    ])
}

/// What each vertex of the first demonstration should stand for in demonstration `i`, decided
/// from the meaning of the edits: object IDs and numbers as `denotes` writes them.
type Roles<'a> = &'a dyn Fn(usize, &Json) -> Json;

impl Planned {
    /// The correspondence and every hole's type and outputs agree with `roles`.
    fn assert_roles(&self, roles: Roles, types: &[(Json, HoleType)]) {
        for i in 0..self.graphs.len() {
            for (first, image) in self.correspondence(i) {
                let first: Json = serde_json::from_str(&first).unwrap();
                assert_eq!(
                    image,
                    roles(i, &first),
                    "demonstration {}: image of {first}",
                    i + 1
                );
            }
        }
        let mut holes: Vec<(Json, HoleType)> = self
            .holes()
            .into_iter()
            .map(|(denoted, ty, outputs)| {
                let expected: Vec<Json> =
                    (0..self.graphs.len()).map(|i| roles(i, &denoted)).collect();
                assert_eq!(outputs, expected, "outputs of the hole for {denoted}");
                (denoted, ty)
            })
            .collect();
        holes.sort_by_key(|(d, _)| d.to_string());
        let mut types = types.to_vec();
        types.sort_by_key(|(d, _)| d.to_string());
        assert_eq!(holes, types);
    }

    /// Every hole's PBE input in demonstration `i` consists of `pre` (receiver, `nodeHeap` and
    /// field heaps of the pre-call state), `args`, `created`, and the earlier holes' outputs
    /// under `roles`, encoded by `encode`; the output is the hole's own role.
    fn assert_pbe_inputs(
        &self,
        i: usize,
        pre: &PbeInput,
        roles: Roles,
        encode: &dyn Fn(&Json) -> Json,
    ) {
        let holes = self.holes();
        for (k, (denoted, _, _)) in holes.iter().enumerate() {
            let input = self.pbe_input(k, i);
            let context = format!("demonstration {}, hole for {denoted}", i + 1);
            assert_eq!(input.receiver, pre.receiver, "{context}");
            assert_eq!(input.objects, pre.objects, "{context}");
            assert_eq!(input.field_heaps, pre.field_heaps, "{context}");
            assert_eq!(input.args, pre.args, "{context}");
            assert_eq!(input.created, pre.created, "{context}");
            let earlier: Vec<Json> = holes[..k]
                .iter()
                .map(|(d, _, _)| encode(&roles(i, d)))
                .collect();
            assert_eq!(input.earlier_holes, earlier, "{context}");
            assert_eq!(input.output, encode(&roles(i, denoted)), "{context}");
        }
    }
}

/// `nodeHeap` for objects with these `next`/`other` slots (-1 for null); an integer slot names
/// the object's own entry of `valueHeap`.
fn node_heap(slots: &[(i64, i64)]) -> Json {
    Json::Array(
        slots
            .iter()
            .enumerate()
            .map(|(i, (next, other))| {
                json!({ "object": { "className": "Cell", "fields": {
                "next": r(*next), "other": r(*other), "value": r(i as i64) } } })
            })
            .collect(),
    )
}

/// The input a hole receives besides the earlier holes and its output.
fn pre(
    receiver: i64,
    slots: &[(i64, i64)],
    values: &[i64],
    args: Json,
    created: &[i64],
) -> PbeInput {
    let column = |f: fn(&(i64, i64)) -> i64| Json::Array(slots.iter().map(|s| r(f(s))).collect());
    PbeInput {
        receiver: r(receiver),
        objects: node_heap(slots),
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

/// An object ID becomes its index in `order`; a number stays a number.
fn encoder(order: &'static [&'static str]) -> impl Fn(&Json) -> Json {
    move |v: &Json| match v.as_str() {
        Some(id) => r(order
            .iter()
            .position(|o| *o == id)
            .unwrap_or_else(|| panic!("{id}")) as i64),
        None => v.clone(),
    }
}

fn permutations<T: Clone>(items: &[T]) -> Vec<Vec<T>> {
    if items.len() <= 1 {
        return vec![items.to_vec()];
    }
    let mut all = Vec::new();
    for i in 0..items.len() {
        let mut rest = items.to_vec();
        let head = rest.remove(i);
        for mut tail in permutations(&rest) {
            tail.insert(0, head.clone());
            all.push(tail);
        }
    }
    all
}

// ---------------------------------------------------------------------------------------------
// 1. Independent writes in another order.

/// `A.other = C` and `B.other = C` on A, B, C, all `other` null; the second demonstration
/// makes the same two writes in `second` order.
fn symmetric_writes(second: [&str; 2]) -> SynthesisRequest {
    let objects = json!([cell("A", 1, &[]), cell("B", 2, &[]), cell("C", 3, &[])]);
    let edits = |order: [&str; 2]| order.iter().map(|o| add_edge(o, "other", "C")).collect();
    let demo = |order| Demo {
        receiver: "A",
        args: json!([]),
        objects: objects.clone(),
        values: json!({}),
        edits: edits(order),
    };
    request("point", &[], &[demo(["A", "B"]), demo(second)])
}

#[test]
fn reordered_symmetric_writes_correspond_object_to_object() {
    let planned = plan(&symmetric_writes(["B", "A"])).unwrap();

    // The graphs keep each demonstration's order.
    let other = |o: &str| (json!(o), "other".to_string(), json!("C"));
    assert_eq!(writes(&planned.graphs[0]), [other("A"), other("B")]);
    assert_eq!(writes(&planned.graphs[1]), [other("B"), other("A")]);

    // A and B are symmetric in the graph, but both demonstrations start from one state with no
    // arguments, so mapping A to B would ask one hole for A and B on the same input. Every
    // vertex must stand for the same object in both demonstrations.
    let same = |_: usize, v: &Json| v.clone();
    let cell_type = HoleType::Object("Cell".into());
    planned.assert_roles(
        &same,
        &[("A", &cell_type), ("B", &cell_type), ("C", &cell_type)]
            .map(|(o, t)| (json!(o), t.clone())),
    );

    // A, B, C are 0, 1, 2; no `other` is written yet when a hole runs.
    let before = pre(
        0,
        &[(-1, -1), (-1, -1), (-1, -1)],
        &[1, 2, 3],
        json!([]),
        &[],
    );
    for i in 0..2 {
        planned.assert_pbe_inputs(i, &before, &same, &encoder(&["A", "B", "C"]));
    }

    let after = BTreeMap::from([
        ("A".to_string(), fields(1, None, Some("C"))),
        ("B".to_string(), fields(2, None, Some("C"))),
        ("C".to_string(), fields(3, None, None)),
    ]);
    assert_eq!(planned.run_template(0), after);
    assert_eq!(planned.run_template(1), after);
}

/// Three writes to different objects and fields, each in every order in both demonstrations
/// (6 x 6 pairs): `A.other = C`, `B.next = A`, `C.value = 7` (C.value was 3).
#[test]
fn every_order_of_three_independent_writes_gives_the_same_correspondence_holes_and_result() {
    let objects = json!([cell("A", 1, &[]), cell("B", 2, &[]), cell("C", 3, &[])]);
    let write = |w: &str| match w {
        "A.other" => add_edge("A", "other", "C"),
        "B.next" => add_edge("B", "next", "A"),
        _ => edit_edge("C", "value", "C-value", "n7"),
    };
    let meaning = |w: &str| match w {
        "A.other" => (json!("A"), "other".to_string(), json!("C")),
        "B.next" => (json!("B"), "next".to_string(), json!("A")),
        _ => (json!("C"), "value".to_string(), json!(7)),
    };
    let orders = permutations(&["A.other", "B.next", "C.value"]);
    let demo = |order: &[&str]| Demo {
        receiver: "A",
        args: json!([]),
        objects: objects.clone(),
        values: json!({ "C-value": 3 }),
        edits: std::iter::once(number("n7", 7))
            .chain(order.iter().map(|w| write(w)))
            .collect(),
    };
    let same = |_: usize, v: &Json| v.clone();
    let cell_type = HoleType::Object("Cell".into());
    let types = [
        (json!("A"), cell_type.clone()),
        (json!("B"), cell_type.clone()),
        (json!("C"), cell_type),
        (json!(7), HoleType::Number),
    ];
    // C's value is still 3 when a hole runs: the write of 7 is never applied.
    let before = pre(
        0,
        &[(-1, -1), (-1, -1), (-1, -1)],
        &[1, 2, 3],
        json!([]),
        &[],
    );
    let after = BTreeMap::from([
        ("A".to_string(), fields(1, None, Some("C"))),
        ("B".to_string(), fields(2, Some("A"), None)),
        ("C".to_string(), fields(7, None, None)),
    ]);
    let mut checked = 0;
    for first in &orders {
        for second in &orders {
            let planned = plan(&request("wire", &[], &[demo(first), demo(second)])).unwrap();
            for (graph, order) in planned.graphs.iter().zip([first, second]) {
                assert_eq!(
                    writes(graph),
                    order.iter().map(|w| meaning(w)).collect::<Vec<_>>()
                );
            }
            planned.assert_roles(&same, &types);
            for i in 0..2 {
                planned.assert_pbe_inputs(i, &before, &same, &encoder(&["A", "B", "C"]));
                assert_eq!(planned.run_template(i), after, "{first:?} then {second:?}");
            }
            checked += 1;
        }
    }
    assert_eq!(checked, 36);
}

/// The same two writes as `reordered_symmetric_writes_correspond_object_to_object`, but each
/// demonstration has its own list A -> B -> C and P -> Q -> R, so the inputs differ and the hole
/// examples do not decide whether A and B go to P and Q or to Q and P. Both choices describe the
/// same update: the two cells that are written each get `other` = the target.
fn distinct_lists(second: [&'static str; 2]) -> SynthesisRequest {
    let demo = |ids: [&'static str; 3], values: [i64; 3], order: [&'static str; 2]| Demo {
        receiver: ids[0],
        args: json!([]),
        objects: json!([
            cell(ids[0], values[0], &[("next", ids[1])]),
            cell(ids[1], values[1], &[("next", ids[2])]),
            cell(ids[2], values[2], &[]),
        ]),
        values: json!({}),
        edits: order.iter().map(|o| add_edge(o, "other", ids[2])).collect(),
    };
    request(
        "point",
        &[],
        &[
            demo(["A", "B", "C"], [1, 2, 3], ["A", "B"]),
            demo(["P", "Q", "R"], [4, 5, 6], second),
        ],
    )
}

/// Which of A and B goes to P is not fixed here: the correspondence is allowed either way as
/// long as the target C goes to R, {A, B} goes to {P, Q}, and the template run gives each
/// demonstration its own result. (The current search takes A -> P for the same order and
/// A -> Q for the reversed one; the receiver is not part of the correspondence, and in the
/// paper's insert the receiver A of the first demonstration goes to D in the third.)
#[test]
fn reordered_writes_in_different_states_reproduce_each_demonstration_whichever_cells_pair_up() {
    for second in [["P", "Q"], ["Q", "P"]] {
        let planned = plan(&distinct_lists(second)).unwrap();
        let image = planned.correspondence(1);
        assert_eq!(image[&json!("C").to_string()], json!("R"), "{second:?}");
        let mut written: Vec<Json> = ["A", "B"]
            .map(|o| image[&json!(o).to_string()].clone())
            .to_vec();
        written.sort_by_key(Json::to_string);
        assert_eq!(written, [json!("P"), json!("Q")], "{second:?}");
        for (i, [x, y, target], values) in [
            (0, ["A", "B", "C"], [1, 2, 3]),
            (1, ["P", "Q", "R"], [4, 5, 6]),
        ] {
            let after = BTreeMap::from([
                (x.to_string(), fields(values[0], Some(y), Some(target))),
                (y.to_string(), fields(values[1], Some(target), Some(target))),
                (target.to_string(), fields(values[2], None, None)),
            ]);
            assert_eq!(
                planned.run_template(i),
                after,
                "{second:?}, demonstration {}",
                i + 1
            );
        }
    }
}

// ---------------------------------------------------------------------------------------------
// 2. Created objects in another order.

/// `pushTwo(x, y)` inserts two new cells after the receiver: receiver -> first -> second ->
/// old successor, with first.value = x and second.value = y. The roles of the two created cells
/// show only in the links, never in their IDs or creation order.
const PUSH_WRITES: [&str; 5] = [
    "first.value",
    "second.value",
    "first.next",
    "second.next",
    "receiver.next",
];

fn push_two(
    ids: [&'static str; 4],
    values: [i64; 2],
    args: [i64; 2],
    creation: [&str; 2],
    writes: &[&str],
) -> Demo {
    let [receiver, successor, first, second] = ids;
    let (x, y) = (format!("n{}", args[0]), format!("n{}", args[1]));
    let write = |w: &str| match w {
        "first.value" => add_edge(first, "value", &x),
        "second.value" => add_edge(second, "value", &y),
        "first.next" => add_edge(first, "next", second),
        "second.next" => add_edge(second, "next", successor),
        _ => edit_edge(receiver, "next", successor, first),
    };
    let created = |role: &str| if role == "first" { first } else { second };
    Demo {
        receiver,
        args: json!(args),
        objects: json!([
            cell(receiver, values[0], &[("next", successor)]),
            cell(successor, values[1], &[])
        ]),
        values: json!({}),
        edits: creation
            .iter()
            .map(|role| add_node(created(role)))
            .chain([number(&x, args[0]), number(&y, args[1])])
            .chain(writes.iter().map(|w| write(w)))
            .collect(),
    }
}

/// First demonstration: A(1) -> B(2), pushTwo(10, 20) creating X then Y.
/// Second: P(5) -> Q(6), pushTwo(30, 40) creating U (first) and V (second) in `creation` order.
fn push_two_request(creation: [&str; 2], writes: &[&str]) -> SynthesisRequest {
    request(
        "pushTwo",
        &["x", "y"],
        &[
            push_two(
                ["A", "B", "X", "Y"],
                [1, 2],
                [10, 20],
                ["first", "second"],
                &PUSH_WRITES,
            ),
            push_two(["P", "Q", "U", "V"], [5, 6], [30, 40], creation, writes),
        ],
    )
}

fn push_roles(i: usize, v: &Json) -> Json {
    let roles = [("A", "P"), ("B", "Q"), ("X", "U"), ("Y", "V")];
    match (v.as_str(), i) {
        (_, 0) => v.clone(),
        (Some(id), _) => json!(roles.iter().find(|(a, _)| *a == id).unwrap().1),
        (None, _) => json!(v.as_i64().unwrap() + 20),
    }
}

fn assert_push_two(planned: &Planned) {
    let cell_type = HoleType::Object("Cell".into());
    // The created cells are not holes; the values and the two existing cells are.
    planned.assert_roles(
        &push_roles,
        &[
            (json!("A"), cell_type.clone()),
            (json!("B"), cell_type),
            (json!(10), HoleType::Number),
            (json!(20), HoleType::Number),
        ],
    );
    // Pre-call objects first, then the created cells in the template's creation order (first,
    // second), each still `new Cell()`; created0/created1 are those two entries in both
    // demonstrations whatever order the second one created them in.
    let slots = [(1, -1), (-1, -1), (-1, -1), (-1, -1)];
    let demonstrations = [
        (
            pre(0, &slots, &[1, 2, 0, 0], json!([10, 20]), &[2, 3]),
            &["A", "B", "X", "Y"],
        ),
        (
            pre(0, &slots, &[5, 6, 0, 0], json!([30, 40]), &[2, 3]),
            &["P", "Q", "U", "V"],
        ),
    ];
    for (i, (before, order)) in demonstrations.into_iter().enumerate() {
        planned.assert_pbe_inputs(i, &before, &push_roles, &encoder(order));
    }
    let list = |ids: [&str; 4], values: [i64; 4]| {
        let [receiver, successor, first, second] = ids;
        BTreeMap::from([
            (receiver.to_string(), fields(values[0], Some(first), None)),
            (first.to_string(), fields(values[1], Some(second), None)),
            (second.to_string(), fields(values[2], Some(successor), None)),
            (successor.to_string(), fields(values[3], None, None)),
        ])
    };
    assert_eq!(
        planned.run_template(0),
        list(["A", "B", "X", "Y"], [1, 10, 20, 2])
    );
    assert_eq!(
        planned.run_template(1),
        list(["P", "Q", "U", "V"], [5, 30, 40, 6])
    );
}

#[test]
fn created_cells_correspond_by_their_links_not_their_creation_order() {
    let planned = plan(&push_two_request(["second", "first"], &PUSH_WRITES)).unwrap();
    // The second demonstration declares V before U ...
    let created = |g: &OperationGraph| {
        g.vertices
            .iter()
            .filter(|v| matches!(v, Vertex::Created { .. }))
            .map(denotes)
            .collect::<Vec<_>>()
    };
    assert_eq!(created(&planned.graphs[0]), [json!("X"), json!("Y")]);
    assert_eq!(created(&planned.graphs[1]), [json!("V"), json!("U")]);
    // ... and the template still creates the first cell, then the second, in both.
    assert_push_two(&planned);
}

/// Both creation orders times all 120 orders of the five writes in the second demonstration.
#[test]
fn every_creation_and_write_order_of_push_two_gives_the_same_roles_inputs_and_result() {
    let mut checked = 0;
    for creation in [["first", "second"], ["second", "first"]] {
        for order in permutations(&PUSH_WRITES) {
            let planned = plan(&push_two_request(creation, &order))
                .unwrap_or_else(|e| panic!("{creation:?} {order:?}: {e:#}"));
            assert_push_two(&planned);
            checked += 1;
        }
    }
    assert_eq!(checked, 240);
}

/// Orders that are not the same edits rearranged: they use a node before it exists, or name an
/// old target the field no longer holds. They are rejected while building the graph.
#[test]
fn reorderings_that_break_the_recording_are_rejected_not_aligned() {
    let mut used_before_created = push_two(
        ["P", "Q", "U", "V"],
        [5, 6],
        [30, 40],
        ["first", "second"],
        &PUSH_WRITES,
    );
    let position = used_before_created
        .edits
        .iter()
        .position(|e| e["id"] == "U")
        .unwrap();
    let created = used_before_created.edits.remove(position);
    used_before_created.edits.push(created);
    let mut literal_after_use = push_two(
        ["P", "Q", "U", "V"],
        [5, 6],
        [30, 40],
        ["first", "second"],
        &PUSH_WRITES,
    );
    let position = literal_after_use
        .edits
        .iter()
        .position(|e| e["id"] == "n30")
        .unwrap();
    let literal = literal_after_use.edits.remove(position);
    literal_after_use.edits.push(literal);
    // A.next: B -> C, then C -> B. Swapped, the first edit says A.next was C while it is B.
    let relink = |edits: Vec<Json>| Demo {
        receiver: "A",
        args: json!([]),
        objects: json!([
            cell("A", 1, &[("next", "B")]),
            cell("B", 2, &[]),
            cell("C", 3, &[])
        ]),
        values: json!({}),
        edits,
    };
    assert!(read_and_build(&request(
        "relink",
        &[],
        &[relink(vec![
            edit_edge("A", "next", "B", "C"),
            edit_edge("A", "next", "C", "B"),
        ])]
    ))
    .is_ok());
    for (demo, reason) in [
        (used_before_created, "unknown write source U"),
        (literal_after_use, "unknown node n30"),
        (
            relink(vec![
                edit_edge("A", "next", "C", "B"),
                edit_edge("A", "next", "B", "C"),
            ]),
            "does not match the state",
        ),
    ] {
        let error = read_and_build(&request(
            "m",
            &["x", "y"][..demo.args.as_array().unwrap().len()],
            &[demo],
        ))
        .err()
        .unwrap();
        assert!(format!("{error:#}").contains(reason), "{error:#}");
    }
}

// ---------------------------------------------------------------------------------------------
// 3. Writes whose order is their meaning.

/// A(value 0) receives `value` writes of the given numbers in order, each literal a node of
/// its own value (`n<value>`), so writing a number again reuses its node.
fn overwrite(args: Json, values: &[i64]) -> Demo {
    let mut literals: Vec<i64> = values.to_vec();
    literals.sort();
    literals.dedup();
    let mut previous = "A-value".to_string();
    let mut edits: Vec<Json> = literals
        .iter()
        .map(|v| number(&format!("n{v}"), *v))
        .collect();
    for v in values {
        let next = format!("n{v}");
        edits.push(edit_edge("A", "value", &previous, &next));
        previous = next;
    }
    Demo {
        receiver: "A",
        args,
        objects: json!([cell("A", 0, &[])]),
        values: json!({ "A-value": 0 }),
        edits,
    }
}

fn value_of(heap: &BTreeMap<String, BTreeMap<String, Value>>) -> Value {
    heap["A"]["value"].clone()
}

#[test]
fn the_graph_keeps_the_order_and_repetition_of_writes_to_one_field() {
    let (_, graphs) = read_and_build(&request(
        "set",
        &[],
        &[
            overwrite(json!([]), &[1, 2, 1]),
            overwrite(json!([]), &[2, 1]),
        ],
    ))
    .unwrap();
    let value = |v: i64| (json!("A"), "value".to_string(), json!(v));
    assert_eq!(writes(&graphs[0]), [value(1), value(2), value(1)]);
    assert_eq!(writes(&graphs[1]), [value(2), value(1)]);
    // Writing 1 again is the same write as the first, not a new vertex.
    assert_eq!(graphs[0].writes[0], graphs[0].writes[2]);
    assert_eq!(graphs[0].vertices.len(), 3);
}

/// `set(a, b)` writes a, then b: set(1, 2) leaves 2 and set(2, 1) leaves 1. The two
/// demonstrations write the same numbers in opposite orders and must not end the same.
fn set_request() -> SynthesisRequest {
    request(
        "set",
        &["a", "b"],
        &[
            overwrite(json!([1, 2]), &[1, 2]),
            overwrite(json!([2, 1]), &[2, 1]),
        ],
    )
}

#[test]
fn the_template_keeps_the_last_write_last_so_each_demonstration_ends_with_its_own_value() {
    let planned = plan(&set_request()).unwrap();
    // Alignment ignores write order, so both ways of pairing the constants are isomorphisms;
    // what has to hold is that the template, which keeps the first demonstration's order,
    // ends each demonstration with that demonstration's last value.
    let last = planned.template.writes.last().unwrap();
    let hole = planned
        .template
        .holes
        .iter()
        .find(|h| h.vertex == last.to)
        .unwrap();
    assert_eq!(
        hole.examples,
        [2, 1].map(|n| Example::Constant(Constant::Number(n.into())))
    );
    assert_eq!(value_of(&planned.run_template(0)), Value::Number(2.into()));
    assert_eq!(value_of(&planned.run_template(1)), Value::Number(1.into()));
    assert_ne!(
        value_of(&planned.run_template(0)),
        value_of(&planned.run_template(1))
    );
}

/// The same two orders with no arguments: one input, two different results. No method can do
/// both, and alignment, which only compares structure, does not object. The template gives one
/// result for both demonstrations, and the whole-method validation refuses it.
fn contradicting_request() -> SynthesisRequest {
    request(
        "set",
        &[],
        &[overwrite(json!([]), &[1, 2]), overwrite(json!([]), &[2, 1])],
    )
}

#[test]
fn opposite_orders_on_one_input_are_aligned_but_cannot_both_be_reproduced() {
    let planned = plan(&contradicting_request()).unwrap();
    let (first, second) = (planned.run_template(0), planned.run_template(1));
    assert_eq!(value_of(&first), value_of(&second));
    assert!(
        value_of(&first) != Value::Number(2.into()) || value_of(&second) != Value::Number(1.into()),
        "the template cannot leave 2 after the first demonstration and 1 after the second"
    );
}

async fn synthesize(request: &SynthesisRequest) -> Json {
    use warp::Reply;
    let body = bytes::Bytes::from(serde_json::to_vec(request).unwrap());
    let reply = refsyn::handle_synthesis(body)
        .await
        .unwrap()
        .into_response();
    serde_json::from_slice(&hyper::body::to_bytes(reply.into_body()).await.unwrap()).unwrap()
}

#[tokio::test]
async fn the_synthesized_set_ends_with_its_last_argument_and_the_contradiction_is_refused() {
    assert!(
        std::path::Path::new(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/external/escher-ts/dist/index.js"
        ))
        .exists(),
        "requires a built external/escher-ts"
    );
    let adopted = synthesize(&set_request()).await;
    assert_eq!(adopted["validation"]["status"], "passed", "{adopted:#}");
    let script = format!(
        r#"const assert = require("node:assert/strict");
class Cell {{
  constructor() {{ this.value = 0; this.next = null; this.other = null; }}
  {}
  {}
}}
for (const [a, b] of [[1, 2], [2, 1], [5, 9], [9, 5], [3, 3]]) {{
  const cell = new Cell();
  cell.set(a, b);
  assert.equal(cell.value, b, `set(${{a}}, ${{b}})`);
}}"#,
        adopted["code"]
            .as_array()
            .unwrap()
            .iter()
            .map(|c| c.as_str().unwrap())
            .collect::<Vec<_>>()
            .join("\n"),
        adopted["composed_method_code"].as_str().unwrap()
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

    assert_refused_by_validation(&synthesize(&contradicting_request()).await);
}

/// Every hole was synthesized, and the whole-method validation then reproduced the first
/// demonstration and failed on the second at A.value, so no code is adopted.
fn assert_refused_by_validation(body: &Json) {
    assert_eq!(
        body["validation"],
        json!({ "status": "failed", "checked_demonstrations": 1, "error": "Value or return mismatch at A.value" }),
        "{body:#}"
    );
    let helpers = body["individual_codes"].as_array().unwrap();
    assert_eq!(helpers.len(), 3, "{body:#}");
    assert!(
        helpers
            .iter()
            .all(|h| !h.as_str().unwrap().contains("ERROR")),
        "{body:#}"
    );
    assert!(body["composed_method_code"].is_null(), "{body:#}");
    assert_eq!(body["code"], json!([]), "{body:#}");
}

#[test]
fn a_refusal_before_validation_is_not_taken_for_a_refusal_by_validation() {
    for body in [
        json!({ "error": "synthesis stopped before validation" }),
        json!({ "validation": { "status": "unavailable", "checked_demonstrations": 0, "error": "x" },
                "individual_codes": [], "composed_method_code": null, "code": [] }),
        json!({ "validation": { "status": "failed", "checked_demonstrations": 0, "error": "A hole failed synthesis or translation" },
                "individual_codes": ["set_h0: ERROR escher-ts could not synthesize"], "composed_method_code": null, "code": [] }),
    ] {
        let caught = std::panic::catch_unwind(|| assert_refused_by_validation(&body));
        assert!(caught.is_err(), "{body}");
    }
}

/// The two correspondences above give different helpers, so the adopted methods are compared
/// with each other, as wholes, at every receiver that has a successor in lists of 2 to 6 cells;
/// two of these are the demonstrated states. Agreement here is between the two methods, not
/// with a reference: a mistake both share passes. Receivers without a successor, which the
/// demonstrations did not have, are left out of this comparison.
#[tokio::test]
async fn both_write_orders_adopt_methods_that_agree_where_the_receiver_has_a_successor() {
    let mut methods = Vec::new();
    for second in [["P", "Q"], ["Q", "P"]] {
        let body = synthesize(&distinct_lists(second)).await;
        assert_eq!(body["validation"]["status"], "passed", "{body:#}");
        methods.push(json!({ "helpers": body["code"], "method": body["composed_method_code"] }));
    }
    let script = format!(
        r#"const assert = require("node:assert/strict");
const make = (m) => new Function(`return class Cell {{
  constructor() {{ this.value = 0; this.next = null; this.other = null; }}
  ${{m.helpers.join("\n")}}
  ${{m.method}}
}}`)();
const run = (Cell, values, at) => {{
  const cells = values.map((v) => {{ const c = new Cell(); c.value = v; return c; }});
  cells.forEach((c, i) => {{ c.next = cells[i + 1] ?? null; }});
  cells[at].point();
  return cells.map((c) => [c.value, cells.indexOf(c.next), cells.indexOf(c.other)]);
}};
const [same, reversed] = {}.map(make);
let compared = 0;
for (const values of [[1, 2], [4, 5], [1, 2, 3], [4, 5, 6], [2, 1, 3], [1, 1, 1], [7, 8, 9, 10],
                      [1, 8, 9, 10], [9, 8, 1, 10], [3, 4, 5, 6, 7], [1, 2, 3, 4, 5, 6], [0, 0], [5, 1]]) {{
  for (let at = 0; at + 1 < values.length; at++) {{
    assert.deepEqual(run(reversed, values, at), run(same, values, at), JSON.stringify({{ values, at }}));
    compared++;
  }}
}}
console.log(compared);"#,
        json!(methods)
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
    // One comparison per receiver with a successor: the lists' lengths minus one, summed.
    assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "30");
}

/// `set3(a, b, c)`: A.value = a, B.value = c, A.value = b. The second demonstration makes the
/// independent write to B first; both leave A.value = b and B.value = c.
#[test]
fn an_independent_write_moved_between_repeated_writes_keeps_their_pairing() {
    let demo = |args: [i64; 3], order: &[(&'static str, i64)]| {
        let mut literals = args.to_vec();
        literals.sort();
        let mut edits: Vec<Json> = literals
            .iter()
            .map(|v| number(&format!("n{v}"), *v))
            .collect();
        let mut previous =
            BTreeMap::from([("A", "A-value".to_string()), ("B", "B-value".to_string())]);
        for (object, v) in order {
            let next = format!("n{v}");
            edits.push(edit_edge(object, "value", &previous[object], &next));
            previous.insert(object, next);
        }
        Demo {
            receiver: "A",
            args: json!(args),
            objects: json!([cell("A", 0, &[("next", "B")]), cell("B", 0, &[])]),
            values: json!({ "A-value": 0, "B-value": 0 }),
            edits,
        }
    };
    let planned = plan(&request(
        "set3",
        &["a", "b", "c"],
        &[
            demo([1, 2, 5], &[("A", 1), ("B", 5), ("A", 2)]),
            demo([3, 4, 7], &[("B", 7), ("A", 3), ("A", 4)]),
        ],
    ))
    .unwrap();
    // a, b, c of the first demonstration stand for a, b, c of the second.
    let roles = |i: usize, v: &Json| match (i, v.as_i64()) {
        (1, Some(n)) => json!(
            [(1, 3), (2, 4), (5, 7)]
                .iter()
                .find(|(a, _)| *a == n)
                .unwrap()
                .1
        ),
        _ => v.clone(),
    };
    let cell_type = HoleType::Object("Cell".into());
    planned.assert_roles(
        &roles,
        &[
            (json!("A"), cell_type.clone()),
            (json!("B"), cell_type),
            (json!(1), HoleType::Number),
            (json!(2), HoleType::Number),
            (json!(5), HoleType::Number),
        ],
    );
    let after = |a: i64, b: i64| {
        BTreeMap::from([
            ("A".to_string(), fields(a, Some("B"), None)),
            ("B".to_string(), fields(b, None, None)),
        ])
    };
    assert_eq!(planned.run_template(0), after(2, 5));
    assert_eq!(planned.run_template(1), after(4, 7));
}
