//! Builds one PBE problem per hole (paper §3.2) and encodes it as an escher-ts task.
//! Hole `k`'s input in demonstration `i` is the pre-call state plus every object created by
//! the template and the outputs of holes `0..k`; writes are never applied.

use std::collections::{BTreeMap, BTreeSet};

use anyhow::{anyhow, bail, ensure, Result};
use serde_json::{json, Value as Json};

use crate::alignment::Mapping;
use crate::demonstration::{Recording, Value};
use crate::escher_bridge::{
    library_components, EscherTaskAutoExpandSignature, EscherTaskClassSpec, EscherTaskMeta,
    EscherTaskMetaArg, EscherTaskSignature, EscherTaskSignatureArg, EscherTaskSpec,
};
use crate::operation_graph::{Constant, OperationGraph, Vertex};
use crate::template::{Example, HoleType, Template};

/// What the whole-method validator compares each helper's actual result against.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct HoleCheck {
    pub helper: String,
    /// Per demonstration: a number, or `{"ref": <pre-call object id>}`.
    pub expected: Vec<Json>,
}

pub struct Problems {
    pub tasks: Vec<EscherTaskSpec>,
    pub checks: Vec<HoleCheck>,
}

#[derive(Clone, Copy, PartialEq)]
enum FieldKind {
    Int,
    Pointer,
}

/// One demonstration's heap as seen by every hole: pre-call objects, then created objects.
struct State {
    order: Vec<String>,
    fields: Vec<BTreeMap<String, Value>>,
    /// IDs of this demonstration's created objects, in the template's creation order.
    created: Vec<String>,
}

impl State {
    fn index(&self, id: &str) -> Result<usize> {
        self.order
            .iter()
            .position(|o| o == id)
            .ok_or_else(|| anyhow!("unknown object {id}"))
    }
}

pub fn build(
    recording: &Recording,
    graphs: &[OperationGraph],
    mappings: &[Mapping],
    template: &Template,
    params: &[String],
) -> Result<Problems> {
    let first = &recording.demonstrations[0];
    let class = first.receiver_class.clone();
    let states = recording
        .demonstrations
        .iter()
        .zip(graphs.iter().zip(mappings))
        .map(|(demo, (graph, mapping))| {
            let mut order: Vec<String> = Vec::new();
            let mut fields = Vec::new();
            let mut created_ids = Vec::new();
            for (id, object) in &demo.heap.objects {
                ensure!(
                    object.class == class,
                    "object {id} is a {}; PBE problems support a single class ({class})",
                    object.class
                );
                order.push(id.clone());
                fields.push(object.fields.clone());
            }
            for create in &template.creates {
                let Vertex::Created {
                    object,
                    class: created,
                } = &graph.vertices[mapping[create.vertex]]
                else {
                    unreachable!("the correspondence maps a creation to a non-creation");
                };
                ensure!(
                    created == &class,
                    "created {created}; PBE problems support a single class ({class})"
                );
                let instance = recording.classes[created]
                    .instance
                    .as_ref()
                    .map_err(|e| anyhow!("{e}"))?;
                order.push(object.clone());
                fields.push(instance.clone());
                created_ids.push(object.clone());
            }
            Ok(State {
                order,
                fields,
                created: created_ids,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    let kinds = field_kinds(&states)?;
    // escher-ts finds a field's heap by the name `<field>Heap`, so the name is not converted.
    let field_heap_names: BTreeMap<String, String> = kinds
        .keys()
        .map(|f| (f.clone(), format!("{f}Heap")))
        .collect();
    let names = template.local_names(params);
    let mut inputs = BTreeSet::new();
    for name in ["thisRef", "nodeHeap"]
        .into_iter()
        .chain(field_heap_names.values().map(String::as_str))
        .chain(params.iter().map(String::as_str))
        .chain(names.iter().flatten().map(String::as_str))
    {
        ensure!(
            inputs.insert(name),
            "the PBE input name {name} is used twice; rename the field or parameter"
        );
    }
    let created_args: Vec<(String, String)> = template
        .creates
        .iter()
        .map(|c| {
            (
                names[c.vertex].clone().unwrap(),
                format!("Ref[Object[{class}]]"),
            )
        })
        .collect();
    let param_types = (0..params.len())
        .map(|p| {
            let values: Vec<&Value> = recording
                .demonstrations
                .iter()
                .map(|d| &d.arguments[p])
                .collect();
            if values
                .iter()
                .all(|v| matches!(v, Value::Number(n) if n.is_i64()))
            {
                Ok("Int".to_string())
            } else if values
                .iter()
                .all(|v| matches!(v, Value::Ref(_) | Value::Null))
            {
                Ok(format!("Ref[Object[{class}]]"))
            } else {
                bail!(
                    "argument {} has values PBE problems cannot encode",
                    params[p]
                )
            }
        })
        .collect::<Result<Vec<_>>>()?;

    let mut tasks = Vec::new();
    let mut checks = Vec::new();
    for (k, hole) in template.holes.iter().enumerate() {
        let helper = Template::helper_name(&first.method, k);
        let return_type = task_type(&hole.ty, &class)?;
        let prior: Vec<(String, String)> = template.holes[..k]
            .iter()
            .map(|h| Ok((names[h.vertex].clone().unwrap(), task_type(&h.ty, &class)?)))
            .collect::<Result<_>>()?;
        let args: Vec<(String, String)> = params
            .iter()
            .cloned()
            .zip(param_types.iter().cloned())
            .chain(created_args.iter().cloned())
            .chain(prior)
            .collect();
        let mut examples = Vec::new();
        let mut expected = Vec::new();
        for (i, (demo, state)) in recording.demonstrations.iter().zip(&states).enumerate() {
            let mut input = vec![json!({ "ref": state.index(&demo.receiver)? })];
            input.extend(encode_heap(state, &kinds, &class)?);
            for value in &demo.arguments {
                input.push(encode_value(value, state)?);
            }
            for id in &state.created {
                input.push(json!({ "ref": state.index(id)? }));
            }
            for earlier in &template.holes[..k] {
                input.push(encode_example(&earlier.examples[i], state)?);
            }
            let output = encode_example(&hole.examples[i], state)?;
            examples.push((input, output));
            expected.push(match &hole.examples[i] {
                Example::Object(id) => json!({ "ref": id }),
                Example::Constant(c) => constant_json(c),
            });
        }
        let value_fields: Vec<String> = kinds
            .iter()
            .filter(|(_, k)| **k == FieldKind::Int)
            .map(|(f, _)| f.clone())
            .collect();
        let pointer_fields: Vec<String> = kinds
            .iter()
            .filter(|(_, k)| **k == FieldKind::Pointer)
            .map(|(f, _)| f.clone())
            .collect();
        let uses_ints = !value_fields.is_empty()
            || args.iter().any(|(_, t)| t == "Int")
            || return_type == "Int";
        tasks.push(EscherTaskSpec {
            name: helper.clone(),
            category: "refsyn".to_string(),
            classes: vec![EscherTaskClassSpec {
                name: class.clone(),
                fields: kinds
                    .iter()
                    .map(|(f, k)| {
                        let ty = match k {
                            FieldKind::Int => "Ref[Int]".to_string(),
                            FieldKind::Pointer => format!("Ref[Object[{class}]]"),
                        };
                        (f.clone(), ty)
                    })
                    .collect(),
            }],
            expose_class_components: false,
            auto_class_field_components: true,
            strict_field_access: true,
            signature: EscherTaskSignature {
                return_type: return_type.clone(),
                args: args
                    .iter()
                    .map(|(name, ty)| EscherTaskSignatureArg {
                        name: name.clone(),
                        arg_type: ty.clone(),
                    })
                    .collect(),
                auto_expand_class_signature: EscherTaskAutoExpandSignature {
                    class_name: class.clone(),
                    this_ref_name: "thisRef".to_string(),
                    class_heap_name: "nodeHeap".to_string(),
                    field_heap_names: field_heap_names.clone(),
                },
            },
            components: library_components(uses_ints, !pointer_fields.is_empty()),
            examples,
            refsyn_meta: EscherTaskMeta {
                js_method_name: helper.clone(),
                class_name: class.clone(),
                this_ref_name: "thisRef".to_string(),
                class_heap_name: "nodeHeap".to_string(),
                value_fields,
                pointer_fields,
                field_heap_names: field_heap_names.clone(),
                explicit_args: args
                    .iter()
                    .map(|(name, ty)| EscherTaskMetaArg {
                        name: name.clone(),
                        legacy_type: if ty == "Int" { "Int" } else { "Ptr" }.to_string(),
                        task_type: ty.clone(),
                    })
                    .collect(),
            },
        });
        checks.push(HoleCheck { helper, expected });
    }
    Ok(Problems { tasks, checks })
}

fn task_type(ty: &HoleType, class: &str) -> Result<String> {
    match ty {
        HoleType::Number => Ok("Int".to_string()),
        HoleType::Object(c) if c == class => Ok(format!("Ref[Object[{class}]]")),
        HoleType::Object(c) => bail!("a hole of class {c} cannot be encoded in a {class} problem"),
        HoleType::String => bail!("string-valued holes cannot be encoded for escher-ts"),
        HoleType::Bool => bail!("boolean-valued holes are not supported yet"),
    }
}

/// escher-ts represents null only as a pointer, so a field that is only ever null is a
/// pointer field. Undefined and absent are the error value of either kind, never null.
fn field_kinds(states: &[State]) -> Result<BTreeMap<String, FieldKind>> {
    let names: BTreeSet<&String> = states
        .iter()
        .flat_map(|s| s.fields.iter().flat_map(BTreeMap::keys))
        .collect();
    let mut kinds = BTreeMap::new();
    for name in names {
        let values: Vec<Option<&Value>> = states
            .iter()
            .flat_map(|s| s.fields.iter().map(|f| f.get(name)))
            .collect();
        let ints = values.iter().any(|v| matches!(v, Some(Value::Number(_))));
        let refs = values
            .iter()
            .any(|v| matches!(v, Some(Value::Ref(_) | Value::Null)));
        let undefined = values
            .iter()
            .any(|v| matches!(v, None | Some(Value::Undefined)));
        if let Some(Some(other)) = values.iter().find(|v| {
            matches!(v, Some(Value::String(_) | Value::Bool(_)))
                || matches!(v, Some(Value::Number(n)) if !n.is_i64())
        }) {
            bail!("field {name} holds {other:?}, which escher-ts cannot represent");
        }
        let kind = match (ints, refs, undefined) {
            (true, true, _) => bail!("field {name} holds both integers and references or null"),
            (false, true, _) => FieldKind::Pointer,
            (_, false, _) => FieldKind::Int,
        };
        kinds.insert(name.clone(), kind);
    }
    Ok(kinds)
}

fn encode_heap(
    state: &State,
    kinds: &BTreeMap<String, FieldKind>,
    class: &str,
) -> Result<Vec<Json>> {
    let mut objects = Vec::new();
    for (index, fields) in state.fields.iter().enumerate() {
        let mut slots = serde_json::Map::new();
        for (name, kind) in kinds {
            let slot = match kind {
                FieldKind::Int => json!({ "ref": index }),
                FieldKind::Pointer => encode_pointer(fields.get(name), state)?,
            };
            slots.insert(name.clone(), slot);
        }
        objects.push(json!({ "object": { "className": class, "fields": slots } }));
    }
    let mut heaps = vec![Json::Array(objects)];
    for (name, kind) in kinds {
        let heap = state
            .fields
            .iter()
            .map(|fields| {
                Ok(match (kind, fields.get(name)) {
                    (FieldKind::Int, Some(Value::Number(n))) => json!(n),
                    (FieldKind::Int, _) => json!({ "error": true }),
                    (FieldKind::Pointer, value) => encode_pointer(value, state)?,
                })
            })
            .collect::<Result<Vec<_>>>()?;
        heaps.push(Json::Array(heap));
    }
    Ok(heaps)
}

fn encode_pointer(value: Option<&Value>, state: &State) -> Result<Json> {
    Ok(match value {
        Some(Value::Ref(id)) => json!({ "ref": state.index(id)? }),
        Some(Value::Null) => json!({ "ref": -1 }),
        None | Some(Value::Undefined) => json!({ "error": true }),
        Some(other) => bail!("{other:?} in a pointer field"),
    })
}

fn encode_value(value: &Value, state: &State) -> Result<Json> {
    match value {
        Value::Number(n) if n.is_i64() => Ok(json!(n)),
        Value::Ref(id) => Ok(json!({ "ref": state.index(id)? })),
        Value::Null => Ok(json!({ "ref": -1 })),
        other => bail!("{other:?} cannot be encoded for escher-ts"),
    }
}

fn encode_example(example: &Example, state: &State) -> Result<Json> {
    match example {
        Example::Object(id) => Ok(json!({ "ref": state.index(id)? })),
        Example::Constant(Constant::Number(n)) if n.is_i64() => Ok(json!(n)),
        Example::Constant(c) => bail!("{c:?} cannot be encoded for escher-ts"),
    }
}

fn constant_json(constant: &Constant) -> Json {
    match constant {
        Constant::Number(n) => json!(n),
        Constant::String(s) => json!(s),
        Constant::Bool(b) => json!(b),
    }
}
