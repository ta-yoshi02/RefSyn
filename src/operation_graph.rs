//! Normalizes one demonstration's edits into a MOLD operation graph (paper §4.1).

use std::collections::{BTreeMap, BTreeSet};

use anyhow::{anyhow, bail, ensure, Result};

use crate::demonstration::{Class, Demonstration, Edit, Value};

pub type VertexId = usize;

#[derive(Debug, Clone, PartialEq)]
pub enum Constant {
    Number(serde_json::Number),
    String(String),
    Bool(bool),
}

impl Constant {
    fn same_type(&self, other: &Constant) -> bool {
        std::mem::discriminant(self) == std::mem::discriminant(other)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Vertex {
    /// `Node(C)`: an object of the pre-call state.
    Existing {
        class: String,
        object: String,
    },
    /// `addNode(C)`: an object created by the demonstration.
    Created {
        class: String,
        object: String,
    },
    Constant(Constant),
    Null,
}

impl Vertex {
    /// The vertex correspondence `~` of the paper.
    pub fn corresponds(&self, other: &Vertex) -> bool {
        match (self, other) {
            (Vertex::Existing { class: a, .. }, Vertex::Existing { class: b, .. })
            | (Vertex::Created { class: a, .. }, Vertex::Created { class: b, .. }) => a == b,
            (Vertex::Constant(a), Vertex::Constant(b)) => a.same_type(b),
            (Vertex::Null, Vertex::Null) => true,
            _ => false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Write {
    pub from: VertexId,
    pub field: String,
    pub to: VertexId,
}

#[derive(Debug, Clone, PartialEq)]
pub struct OperationGraph {
    /// Declarations in order of first appearance; `VertexId` indexes this list.
    pub vertices: Vec<Vertex>,
    pub writes: Vec<Write>,
    /// Extension beyond the paper: the value bound to `return`, if the demonstration set it.
    pub ret: Option<VertexId>,
}

#[derive(Debug, Clone, PartialEq)]
enum Slot {
    Absent,
    Value(Value),
}

pub fn build(demo: &Demonstration, classes: &BTreeMap<String, Class>) -> Result<OperationGraph> {
    let mut builder = Builder {
        demo,
        vertices: Vec::new(),
        by_id: BTreeMap::new(),
        null: None,
        literals: demo.literals.clone(),
        state: demo
            .heap
            .objects
            .iter()
            .map(|(id, object)| (id.clone(), object.fields.clone()))
            .collect(),
        created: BTreeSet::new(),
        written: BTreeSet::new(),
    };
    let mut writes = Vec::new();
    let mut ret = None;
    for edit in &demo.edits {
        match edit {
            Edit::AddObject { id, class } => {
                ensure!(
                    builder.fresh(id),
                    "created object {id} reuses an existing ID"
                );
                let class_def = classes
                    .get(class)
                    .ok_or_else(|| anyhow!("unknown class {class} for created object {id}"))?;
                let instance = class_def.instance.as_ref().map_err(|reason| {
                    anyhow!("cannot create {class}: new {class}() is outside the supported constructors ({reason})")
                })?;
                builder.state.insert(id.clone(), instance.clone());
                builder.created.insert(id.clone());
                builder.declare(
                    id,
                    Vertex::Created {
                        class: class.clone(),
                        object: id.clone(),
                    },
                );
            }
            Edit::AddLiteral { id, value } => {
                ensure!(builder.fresh(id), "literal {id} reuses an existing ID");
                builder.literals.insert(id.clone(), value.clone());
            }
            Edit::AddEdge { from, field, to } => {
                let current = builder.slot(from, field)?;
                // Kanon draws no edge for an empty field, nor any field of a created node, so an
                // unwritten field of a created object may still hold its `new C()` default.
                ensure!(
                    matches!(
                        current,
                        Slot::Absent | Slot::Value(Value::Null | Value::Undefined)
                    ) || builder.unwritten_created(from, field),
                    "addEdge on {from}.{field}, which already has an edge"
                );
                writes.push(builder.write(from, field, to)?);
            }
            Edit::EditEdge {
                from,
                field,
                old,
                new,
            } => {
                builder.check_old(from, field, old)?;
                writes.push(builder.write(from, field, new)?);
            }
            Edit::DeleteEdge { from, field, old } => {
                ensure!(
                    matches!(builder.value(old)?, Value::Ref(_)),
                    "deleteEdge on {from}.{field} is supported only for object references"
                );
                builder.check_old(from, field, old)?;
                let source = builder.vertex(from)?;
                let null = builder.null_vertex();
                builder.store(from, field, Value::Null)?;
                writes.push(Write {
                    from: source,
                    field: field.clone(),
                    to: null,
                });
            }
            Edit::SetReturn { to } => ret = Some(builder.vertex(to)?),
        }
    }
    Ok(OperationGraph {
        vertices: builder.vertices,
        writes,
        ret,
    })
}

struct Builder<'a> {
    demo: &'a Demonstration,
    vertices: Vec<Vertex>,
    by_id: BTreeMap<String, VertexId>,
    null: Option<VertexId>,
    literals: BTreeMap<String, Value>,
    /// Fields of every object while replaying the edits, used only to check each edit's
    /// recorded old target. Created objects start from their class's `new C()` state.
    state: BTreeMap<String, BTreeMap<String, Value>>,
    /// Objects created by this demonstration.
    created: BTreeSet<String>,
    /// Fields (object, field) written by an edit of this demonstration.
    written: BTreeSet<(String, String)>,
}

impl Builder<'_> {
    fn fresh(&self, id: &str) -> bool {
        !self.state.contains_key(id) && !self.literals.contains_key(id)
    }

    fn declare(&mut self, id: &str, vertex: Vertex) -> VertexId {
        let index = self.vertices.len();
        self.vertices.push(vertex);
        self.by_id.insert(id.to_string(), index);
        index
    }

    fn null_vertex(&mut self) -> VertexId {
        if let Some(index) = self.null {
            return index;
        }
        let index = self.vertices.len();
        self.vertices.push(Vertex::Null);
        self.null = Some(index);
        index
    }

    fn vertex(&mut self, id: &str) -> Result<VertexId> {
        if let Some(index) = self.by_id.get(id) {
            return Ok(*index);
        }
        let vertex = if let Some(object) = self.demo.heap.get(id) {
            Vertex::Existing {
                class: object.class.clone(),
                object: id.to_string(),
            }
        } else if let Some(value) = self.literals.get(id) {
            Vertex::Constant(match value {
                Value::Number(n) => Constant::Number(n.clone()),
                Value::String(s) => Constant::String(s.clone()),
                Value::Bool(b) => Constant::Bool(*b),
                other => bail!("literal {id} holds {other:?}, which is not a constant"),
            })
        } else {
            bail!("unknown node {id}");
        };
        Ok(self.declare(id, vertex))
    }

    /// The value a node denotes when written into a field.
    fn value(&self, id: &str) -> Result<Value> {
        if self.state.contains_key(id) {
            Ok(Value::Ref(id.to_string()))
        } else {
            self.literals
                .get(id)
                .cloned()
                .ok_or_else(|| anyhow!("unknown node {id}"))
        }
    }

    fn slot(&self, object: &str, field: &str) -> Result<Slot> {
        Ok(self
            .state
            .get(object)
            .ok_or_else(|| anyhow!("unknown write source {object}"))?
            .get(field)
            .map_or(Slot::Absent, |value| Slot::Value(value.clone())))
    }

    fn unwritten_created(&self, object: &str, field: &str) -> bool {
        self.created.contains(object)
            && !self
                .written
                .contains(&(object.to_string(), field.to_string()))
    }

    fn check_old(&self, object: &str, field: &str, old: &str) -> Result<()> {
        ensure!(
            self.slot(object, field)? == Slot::Value(self.value(old)?),
            "the recorded old target {old} of {object}.{field} does not match the state"
        );
        Ok(())
    }

    fn store(&mut self, object: &str, field: &str, value: Value) -> Result<()> {
        self.state
            .get_mut(object)
            .ok_or_else(|| anyhow!("unknown write source {object}"))?
            .insert(field.to_string(), value);
        self.written.insert((object.to_string(), field.to_string()));
        Ok(())
    }

    fn write(&mut self, from: &str, field: &str, to: &str) -> Result<Write> {
        let source = self.vertex(from)?;
        ensure!(
            matches!(
                self.vertices[source],
                Vertex::Existing { .. } | Vertex::Created { .. }
            ),
            "write source {from} is not an object"
        );
        let target = self.vertex(to)?;
        let value = self.value(to)?;
        self.store(from, field, value)?;
        Ok(Write {
            from: source,
            field: field.to_string(),
            to: target,
        })
    }
}
