use std::collections::BTreeMap;

use refsyn::demonstration;
use refsyn::operation_graph::{self, Constant, OperationGraph, Vertex};
use refsyn::SynthesisRequest;
use serde_json::{json, Value};

type State = BTreeMap<String, Value>;

struct Random(u32);

impl Random {
    fn choose(&mut self, bound: usize) -> usize {
        self.0 = self.0.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        ((self.0 >> 8) as usize) % bound
    }
}

fn defaults() -> Value {
    json!({"className": "Cell", "fields": {"defaultValue": 7, "next": null}})
}

fn request(initial: &State, literals: &State, operations: &[Value]) -> SynthesisRequest {
    serde_json::from_value(json!({
        "method_calls": [{"callLabel": "call", "contextSensitiveID": "ctx",
            "receiverObject": "o0", "receiverClassName": "Cell", "methodName": "edit",
            "arguments": [], "operations": operations}],
        "vis_graph": {"nodes": [], "edges": []},
        "validation": {"version": 3,
            "classes": [{"name": "Cell", "instance": defaults()["fields"]}],
            "cases": [{"callLabel": "call", "contextSensitiveID": "ctx", "arguments": [],
                "objects": initial.iter().map(|(id, object)| json!({
                    "id": id, "className": object["className"], "fields": object["fields"]
                })).collect::<Vec<_>>(), "values": literals}]}
    }))
    .unwrap()
}

fn graph(request: &SynthesisRequest) -> anyhow::Result<OperationGraph> {
    let recording = demonstration::read(request)?;
    operation_graph::build(&recording.demonstrations[0], &recording.classes)
}

fn vertex_value(vertex: &Vertex) -> Value {
    match vertex {
        Vertex::Existing { object, .. } | Vertex::Created { object, .. } => json!({"ref": object}),
        Vertex::Constant(Constant::Number(n)) => json!(n),
        Vertex::Constant(Constant::String(s)) => json!(s),
        Vertex::Constant(Constant::Bool(b)) => json!(b),
        Vertex::Null => Value::Null,
    }
}

fn replay(initial: &State, graph: &OperationGraph) -> (State, Value) {
    let mut state = initial.clone();
    for vertex in &graph.vertices {
        match vertex {
            Vertex::Created { class, object } => {
                assert_eq!(class, "Cell");
                assert!(state.insert(object.clone(), defaults()).is_none());
            }
            Vertex::Existing { class, object } => assert_eq!(initial[object]["className"], *class),
            _ => {}
        }
    }
    for write in &graph.writes {
        let source = vertex_value(&graph.vertices[write.from]);
        let object = state.get_mut(source["ref"].as_str().unwrap()).unwrap();
        object["fields"][&write.field] = vertex_value(&graph.vertices[write.to]);
    }
    let returned = graph
        .ret
        .map(|id| vertex_value(&graph.vertices[id]))
        .unwrap_or(json!({"undefined": true}));
    (state, returned)
}

struct Trace {
    initial: State,
    literals: State,
    operations: Vec<Value>,
    expected: Vec<(State, Value)>,
}

fn trace(seed: u32) -> Trace {
    let mut random = Random(seed);
    let mut literals: State = [
        ("zero", json!(0)),
        ("seven", json!(7)),
        ("negative", json!(-19)),
        ("text", json!("007")),
        ("yes", json!(true)),
        ("no", json!(false)),
    ]
    .into_iter()
    .map(|(id, value)| (id.to_string(), value))
    .collect();
    let initial_literals = literals.clone();
    let mut ids: Vec<String> = (0..5).map(|i| format!("o{i}")).collect();
    let initial: State = ids
        .iter()
        .enumerate()
        .map(|(i, id)| {
            let next = ["o0", "o2", "o1", "o2", "o4"][i];
            (
                id.clone(),
                json!({"className": "Cell", "fields": {
                    "next": {"ref": next}, "value": 0, "empty": null,
                    "optional": {"undefined": true}
                }}),
            )
        })
        .collect();
    let mut state = initial.clone();
    let mut operations = Vec::new();
    let mut expected = Vec::new();
    let mut returned = json!({"undefined": true});
    for step in 0..48 {
        match step % 8 {
            0 => {
                let id = format!("o{}", ids.len());
                operations.push(
                    json!({"editType": "addNode", "id": id, "label": "Cell", "isLiteral": false}),
                );
                state.insert(id.clone(), defaults());
                ids.push(id);
            }
            2 => {
                let id = format!("literal{step}");
                let (kind, value) = match random.choose(3) {
                    0 => ("number", json!(random.choose(2001) as i64 - 1000)),
                    1 => ("string", json!(format!("00{}", random.choose(10)))),
                    _ => ("boolean", json!(random.choose(2) == 0)),
                };
                let label = value
                    .as_str()
                    .map(str::to_string)
                    .unwrap_or_else(|| value.to_string());
                operations.push(json!({"editType": "addNode", "id": id, "label": label,
                    "isLiteral": true, "type": kind}));
                literals.insert(id, value);
            }
            6 => {
                let edges: Vec<_> = state
                    .iter()
                    .flat_map(|(id, object)| {
                        object["fields"].as_object().unwrap().iter().filter_map(
                            move |(field, value)| {
                                value
                                    .get("ref")
                                    .map(|target| (id.clone(), field.clone(), target.clone()))
                            },
                        )
                    })
                    .collect();
                let (id, field, old) = &edges[random.choose(edges.len())];
                operations
                    .push(json!({"editType": "deleteEdge", "from": id, "label": field, "to": old}));
                state.get_mut(id).unwrap()["fields"][field] = Value::Null;
            }
            7 => {
                let (id, value) = if random.choose(2) == 0 {
                    let id = &ids[random.choose(ids.len())];
                    (id.clone(), json!({"ref": id}))
                } else {
                    let (id, value) = literals.iter().nth(random.choose(literals.len())).unwrap();
                    (id.clone(), value.clone())
                };
                operations.push(json!({"editType": "addVariable", "label": "return", "to": id}));
                returned = value;
            }
            _ => {
                let id = if step % 8 == 1 {
                    ids.last().unwrap()
                } else {
                    &ids[random.choose(ids.len())]
                };
                let fields = ["next", "alias", "value", "空 白", "empty", "optional"];
                let field = if step % 8 == 1 {
                    "defaultValue"
                } else {
                    fields[random.choose(fields.len())]
                };
                let (target, value) = if random.choose(2) == 0 {
                    let target = &ids[random.choose(ids.len())];
                    (target.clone(), json!({"ref": target}))
                } else {
                    let (target, value) =
                        literals.iter().nth(random.choose(literals.len())).unwrap();
                    (target.clone(), value.clone())
                };
                let old = state[id]["fields"].get(field);
                let operation = match old {
                    Some(old)
                        if step % 8 != 1
                            && !old.is_null()
                            && old != &json!({"undefined": true}) =>
                    {
                        let old_id = old.get("ref").and_then(Value::as_str).unwrap_or_else(|| {
                            literals.iter().find(|(_, value)| *value == old).unwrap().0
                        });
                        json!({"editType": "editEdgeReference", "from": id, "label": field, "oldTo": old_id, "newTo": target})
                    }
                    _ => json!({"editType": "addEdge", "from": id, "label": field, "to": target}),
                };
                operations.push(operation);
                state.get_mut(id).unwrap()["fields"][field] = value;
            }
        }
        expected.push((state.clone(), returned.clone()));
    }
    Trace {
        initial,
        literals: initial_literals,
        operations,
        expected,
    }
}

#[test]
fn normalization_preserves_every_prefix_of_seeded_random_edits() {
    let mut kinds = BTreeMap::new();
    for seed in 1..=64 {
        let trace = trace(seed);
        let empty = graph(&request(&trace.initial, &trace.literals, &[])).unwrap();
        assert_eq!(
            replay(&trace.initial, &empty),
            (trace.initial.clone(), json!({"undefined": true}))
        );
        for (step, expected) in trace.expected.iter().enumerate() {
            let operations = &trace.operations[..=step];
            let request = request(&trace.initial, &trace.literals, operations);
            let normalized = graph(&request).unwrap_or_else(|error| {
                panic!(
                    "seed={seed} step={step} request={} error={error:#}",
                    serde_json::to_string(&request).unwrap()
                )
            });
            assert_eq!(
                &replay(&trace.initial, &normalized),
                expected,
                "seed={seed} step={step} request={}",
                serde_json::to_string(&request).unwrap()
            );
            *kinds
                .entry(
                    trace.operations[step]["editType"]
                        .as_str()
                        .unwrap()
                        .to_owned(),
                )
                .or_insert(0) += 1;
        }
    }
    for kind in [
        "addNode",
        "addEdge",
        "editEdgeReference",
        "deleteEdge",
        "addVariable",
    ] {
        assert!(kinds[kind] > 0, "missing operation kind {kind}");
    }
    println!("random normalization: 64 seeds, 3072 edit prefixes, 64 empty prefixes; {kinds:?}");
}

#[test]
fn normalization_rejects_stale_old_targets_in_seeded_random_edits() {
    for seed in 1..=64 {
        let trace = trace(seed);
        let step = trace
            .operations
            .iter()
            .position(|op| op["editType"] == "editEdgeReference")
            .unwrap();
        let mut operations = trace.operations[..=step].to_vec();
        let old = operations[step]["oldTo"].as_str().unwrap();
        operations[step]["oldTo"] = json!(if old == "o0" { "o1" } else { "o0" });
        let error = graph(&request(&trace.initial, &trace.literals, &operations)).unwrap_err();
        assert!(
            error.to_string().contains("does not match the state"),
            "seed={seed}: {error:#}"
        );
    }
}
