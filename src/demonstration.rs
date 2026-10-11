//! Reads Kanon's recorded demonstrations: the typed pre-call heap, the call, and the edit
//! sequence. Identifiers are kept exactly as Kanon recorded them.

use std::collections::BTreeMap;

use anyhow::{anyhow, bail, ensure, Context, Result};
use serde_json::{Number, Value as Json};

use crate::SynthesisRequest;

const SNAPSHOT_VERSION: u64 = 3;

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Undefined,
    Null,
    Bool(bool),
    Number(Number),
    String(String),
    Ref(String),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Object {
    pub class: String,
    /// Own data properties; a field missing from the map is absent.
    pub fields: BTreeMap<String, Value>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Heap {
    /// Objects in snapshot order.
    pub objects: Vec<(String, Object)>,
}

impl Heap {
    pub fn get(&self, id: &str) -> Option<&Object> {
        self.objects
            .iter()
            .find_map(|(key, object)| (key == id).then_some(object))
    }

    pub fn get_mut(&mut self, id: &str) -> Option<&mut Object> {
        self.objects
            .iter_mut()
            .find_map(|(key, object)| (key == id).then_some(object))
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Edit {
    AddObject {
        id: String,
        class: String,
    },
    AddLiteral {
        id: String,
        value: Value,
    },
    AddEdge {
        from: String,
        field: String,
        to: String,
    },
    EditEdge {
        from: String,
        field: String,
        old: String,
        new: String,
    },
    DeleteEdge {
        from: String,
        field: String,
        old: String,
    },
    SetReturn {
        to: String,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct Class {
    pub name: String,
    /// Own properties right after `new C()`, or why that state could not be captured.
    pub instance: Result<BTreeMap<String, Value>, String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Demonstration {
    pub call_label: String,
    pub context_id: String,
    pub method: String,
    pub param_names: Vec<String>,
    pub receiver: String,
    pub receiver_class: String,
    pub arguments: Vec<Value>,
    pub heap: Heap,
    /// Values of literal nodes that already exist in the pre-call state, by their Kanon ID.
    pub literals: BTreeMap<String, Value>,
    pub edits: Vec<Edit>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Recording {
    pub classes: BTreeMap<String, Class>,
    pub demonstrations: Vec<Demonstration>,
}

pub fn read(request: &SynthesisRequest) -> Result<Recording> {
    let snapshot = request.validation.as_ref().ok_or_else(|| {
        anyhow!("the request has no typed pre-call state; record the demonstrations again")
    })?;
    if let Some(error) = snapshot.get("error").and_then(Json::as_str) {
        bail!("Kanon could not record the pre-call state: {error}");
    }
    ensure!(
        snapshot.get("version").and_then(Json::as_u64) == Some(SNAPSHOT_VERSION),
        "typed pre-call state must be version {SNAPSHOT_VERSION}"
    );
    let classes = array(snapshot, "classes")?
        .iter()
        .map(read_class)
        .collect::<Result<Vec<_>>>()?;
    let mut class_map = BTreeMap::new();
    for class in classes {
        let name = class.name.clone();
        ensure!(
            class_map.insert(name.clone(), class).is_none(),
            "class {name} is defined twice"
        );
    }
    let cases = array(snapshot, "cases")?;
    ensure!(!request.method_calls.is_empty(), "no demonstrations");
    ensure!(
        cases.len() == request.method_calls.len(),
        "every demonstration needs exactly one pre-call state"
    );
    let demonstrations = request
        .method_calls
        .iter()
        .zip(cases)
        .enumerate()
        .map(|(index, (call, case))| {
            read_demonstration(call, case, &class_map)
                .with_context(|| format!("demonstration {}", index + 1))
        })
        .collect::<Result<Vec<_>>>()?;
    let first = &demonstrations[0];
    ensure!(
        demonstrations
            .iter()
            .all(|d| d.method == first.method && d.receiver_class == first.receiver_class),
        "all demonstrations must target the same method of the same class"
    );
    Ok(Recording {
        classes: class_map,
        demonstrations,
    })
}

fn read_class(raw: &Json) -> Result<Class> {
    let name = string(raw, "name")?.to_string();
    let instance = match (raw.get("instance"), raw.get("instanceError")) {
        (Some(fields), None) => Ok(read_fields(fields, &|_| true)
            .with_context(|| format!("instance state of class {name}"))?),
        (None, Some(error)) => Err(error
            .as_str()
            .ok_or_else(|| anyhow!("instanceError of class {name} must be a string"))?
            .to_string()),
        _ => bail!("class {name} needs exactly one of instance or instanceError"),
    };
    Ok(Class { name, instance })
}

fn read_demonstration(
    call: &crate::MethodCallOperation,
    case: &Json,
    classes: &BTreeMap<String, Class>,
) -> Result<Demonstration> {
    ensure!(
        string(case, "callLabel")? == call.call_label
            && string(case, "contextSensitiveID")? == call.context_sensitive_id,
        "the pre-call state belongs to a different call"
    );
    let mut objects = Vec::new();
    for raw in array(case, "objects")? {
        let id = string(raw, "id")?.to_string();
        let class = string(raw, "className")?.to_string();
        ensure!(
            classes.contains_key(&class),
            "unknown class {class} of object {id}"
        );
        ensure!(
            objects.iter().all(|(other, _)| other != &id),
            "object {id} appears twice"
        );
        objects.push((
            id,
            Object {
                class,
                fields: BTreeMap::new(),
            },
        ));
    }
    let known = |id: &str| objects.iter().any(|(key, _)| key == id);
    let mut fields = Vec::new();
    for raw in array(case, "objects")? {
        let map = raw
            .get("fields")
            .ok_or_else(|| anyhow!("object without fields"))?;
        fields.push(read_fields(map, &known)?);
    }
    for ((_, object), map) in objects.iter_mut().zip(fields) {
        object.fields = map;
    }
    let heap = Heap { objects };
    let literals = match case.get("values") {
        None => BTreeMap::new(),
        Some(map) => read_fields(map, &|_| false).context("literal values")?,
    };
    let arguments = array(case, "arguments")?
        .iter()
        .map(|raw| read_value(raw, &|id| heap.get(id).is_some()))
        .collect::<Result<Vec<_>>>()?;
    let receiver = call.receiver_object.clone();
    let receiver_class = heap
        .get(&receiver)
        .ok_or_else(|| anyhow!("receiver {receiver} is missing from the pre-call state"))?
        .class
        .clone();
    if let Some(declared) = &call.receiver_class_name {
        ensure!(declared == &receiver_class, "receiver class mismatch");
    }
    let param_names = call
        .method_param_names
        .clone()
        .or_else(|| call.argument_names.clone())
        .unwrap_or_default();
    ensure!(
        param_names.len() == arguments.len(),
        "parameter names do not match the {} recorded arguments",
        arguments.len()
    );
    let edits = call
        .operations
        .iter()
        .enumerate()
        .map(|(index, raw)| read_edit(raw).with_context(|| format!("edit {}", index + 1)))
        .collect::<Result<Vec<_>>>()?;
    Ok(Demonstration {
        call_label: call.call_label.clone(),
        context_id: call.context_sensitive_id.clone(),
        method: call.method_name.clone(),
        param_names,
        receiver,
        receiver_class,
        arguments,
        heap,
        literals,
        edits,
    })
}

fn read_edit(raw: &Json) -> Result<Edit> {
    let kind = string(raw, "editType")?;
    let field = || string(raw, "label").map(str::to_string);
    let id = |key| string(raw, key).map(str::to_string);
    Ok(match kind {
        "addNode" if raw.get("isLiteral").and_then(Json::as_bool) == Some(true) => {
            let label = string(raw, "label")?;
            let value = match raw.get("type").and_then(Json::as_str) {
                Some("number") => Value::Number(
                    label
                        .parse::<Number>()
                        .ok()
                        .filter(|n| n.as_f64().is_some_and(f64::is_finite))
                        .ok_or_else(|| anyhow!("literal {label:?} is not a finite number"))?,
                ),
                Some("string") => Value::String(label.to_string()),
                Some("boolean") => match label {
                    "true" => Value::Bool(true),
                    "false" => Value::Bool(false),
                    _ => bail!("literal {label:?} is not a boolean"),
                },
                other => bail!("literal {label:?} has no supported type: {other:?}"),
            };
            Edit::AddLiteral {
                id: id("id")?,
                value,
            }
        }
        "addNode" => Edit::AddObject {
            id: id("id")?,
            class: field()?,
        },
        "addEdge" => Edit::AddEdge {
            from: id("from")?,
            field: field()?,
            to: id("to")?,
        },
        "editEdgeReference" => Edit::EditEdge {
            from: id("from")?,
            field: field()?,
            old: id("oldTo")?,
            new: id("newTo")?,
        },
        "deleteEdge" => Edit::DeleteEdge {
            from: id("from")?,
            field: field()?,
            old: id("to")?,
        },
        "addVariable" | "editVariableReference" if field()? == "return" => Edit::SetReturn {
            to: raw
                .get("newTo")
                .or_else(|| raw.get("to"))
                .and_then(Json::as_str)
                .ok_or_else(|| anyhow!("return edit without a target"))?
                .to_string(),
        },
        other => bail!("unsupported edit {other}"),
    })
}

fn read_fields(raw: &Json, known: &dyn Fn(&str) -> bool) -> Result<BTreeMap<String, Value>> {
    raw.as_object()
        .ok_or_else(|| anyhow!("expected a field map"))?
        .iter()
        .map(|(key, value)| Ok((key.clone(), read_value(value, known)?)))
        .collect()
}

fn read_value(raw: &Json, known: &dyn Fn(&str) -> bool) -> Result<Value> {
    Ok(match raw {
        Json::Null => Value::Null,
        Json::Bool(b) => Value::Bool(*b),
        Json::Number(n) => Value::Number(n.clone()),
        Json::String(s) => Value::String(s.clone()),
        Json::Object(map) if map.len() == 1 && map.get("undefined") == Some(&Json::Bool(true)) => {
            Value::Undefined
        }
        Json::Object(map) if map.len() == 1 => {
            let id = map
                .get("ref")
                .and_then(Json::as_str)
                .ok_or_else(|| anyhow!("unsupported encoded value {raw}"))?;
            ensure!(known(id), "reference to unknown object {id}");
            Value::Ref(id.to_string())
        }
        _ => bail!("unsupported encoded value {raw}"),
    })
}

fn array<'a>(raw: &'a Json, key: &str) -> Result<&'a Vec<Json>> {
    raw.get(key)
        .and_then(Json::as_array)
        .ok_or_else(|| anyhow!("missing array {key}"))
}

fn string<'a>(raw: &'a Json, key: &str) -> Result<&'a str> {
    raw.get(key)
        .and_then(Json::as_str)
        .ok_or_else(|| anyhow!("missing string {key}"))
}
