use std::collections::{BTreeMap, BTreeSet};

use anyhow::{bail, ensure, Result};
use serde_json::Number;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct VertexId(pub usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct HoleId(pub usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ObjectId(pub usize);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Value {
    Undefined,
    Null,
    Boolean(bool),
    Number(Number),
    String(String),
    Reference(ObjectId),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValueType {
    Undefined,
    Boolean,
    Number,
    String,
    Reference(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeclarationValue {
    Create(String),
    Hole { id: HoleId, value_type: ValueType },
    Null,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Declaration {
    pub vertex: VertexId,
    pub value: DeclarationValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Write {
    pub from: VertexId,
    pub field: String,
    pub to: VertexId,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Statement {
    Create {
        vertex: VertexId,
        class: String,
    },
    BindHole {
        vertex: VertexId,
        hole: HoleId,
        value_type: ValueType,
    },
    Write(Write),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Template {
    statements: Vec<Statement>,
    null_vertices: BTreeSet<VertexId>,
}

impl Template {
    pub fn new(
        declarations: Vec<Declaration>,
        writes: Vec<Write>,
        hole_order: Vec<HoleId>,
    ) -> Result<Self> {
        let mut vertices = BTreeMap::new();
        let mut holes = BTreeMap::new();
        let mut statements = Vec::new();
        let mut null_vertices = BTreeSet::new();
        for declaration in declarations {
            ensure!(
                vertices
                    .insert(declaration.vertex, declaration.value.clone())
                    .is_none(),
                "duplicate vertex: {:?}",
                declaration.vertex
            );
            match declaration.value {
                DeclarationValue::Create(class) => {
                    statements.push(Statement::Create {
                        vertex: declaration.vertex,
                        class,
                    });
                }
                DeclarationValue::Hole { id, value_type } => {
                    ensure!(
                        holes.insert(id, (declaration.vertex, value_type)).is_none(),
                        "a hole must belong to exactly one vertex: {:?}",
                        id
                    );
                }
                DeclarationValue::Null => {
                    null_vertices.insert(declaration.vertex);
                }
            }
        }
        let mut ordered_holes = BTreeSet::new();
        for hole in hole_order {
            ensure!(
                ordered_holes.insert(hole),
                "duplicate hole in evaluation order: {:?}",
                hole
            );
            let (vertex, value_type) = holes
                .remove(&hole)
                .ok_or_else(|| anyhow::anyhow!("unknown hole in evaluation order: {:?}", hole))?;
            statements.push(Statement::BindHole {
                vertex,
                hole,
                value_type,
            });
        }
        ensure!(holes.is_empty(), "evaluation order must include every hole");
        for write in writes {
            let source = vertices
                .get(&write.from)
                .ok_or_else(|| anyhow::anyhow!("undeclared write source: {:?}", write.from))?;
            ensure!(
                matches!(
                    source,
                    DeclarationValue::Create(_)
                        | DeclarationValue::Hole {
                            value_type: ValueType::Reference(_),
                            ..
                        }
                ),
                "write source must be an object vertex"
            );
            ensure!(
                vertices.contains_key(&write.to),
                "undeclared write target: {:?}",
                write.to
            );
            ensure!(!write.field.is_empty(), "write field must not be empty");
            statements.push(Statement::Write(write));
        }
        Ok(Self {
            statements,
            null_vertices,
        })
    }

    pub fn statements(&self) -> &[Statement] {
        &self.statements
    }

    pub fn bind_holes(
        &self,
        call: Environment,
        classes: &ClassDefinitions,
        mut evaluate: impl FnMut(HoleId, &ValueType, &HoleEnvironment) -> Result<Value>,
    ) -> Result<HoleEnvironment> {
        call.validate_references()?;
        let mut environment = HoleEnvironment {
            call,
            vertices: self
                .null_vertices
                .iter()
                .map(|vertex| (*vertex, Value::Null))
                .collect(),
        };
        for statement in &self.statements {
            match statement {
                Statement::Create { vertex, class } => {
                    let fields = classes
                        .get(class)
                        .ok_or_else(|| anyhow::anyhow!("missing class definition: {}", class))?;
                    ensure!(
                        fields
                            .values()
                            .all(|value| !matches!(value, Value::Reference(_))),
                        "class defaults must not contain instance references"
                    );
                    let object = ObjectId(environment.call.objects.len());
                    environment.call.objects.push(Object {
                        class: class.clone(),
                        fields: fields.clone(),
                    });
                    environment
                        .vertices
                        .insert(*vertex, Value::Reference(object));
                }
                Statement::BindHole {
                    vertex,
                    hole,
                    value_type,
                } => {
                    let value = evaluate(*hole, value_type, &environment)?;
                    ensure!(
                        environment.call.has_type(&value, value_type),
                        "hole {:?} returned an invalid value for {:?}",
                        hole,
                        value_type
                    );
                    environment.vertices.insert(*vertex, value);
                }
                Statement::Write(_) => {}
            }
        }
        Ok(environment)
    }

    pub fn execute(
        &self,
        call: Environment,
        classes: &ClassDefinitions,
        evaluate: impl FnMut(HoleId, &ValueType, &HoleEnvironment) -> Result<Value>,
    ) -> Result<HoleEnvironment> {
        let mut environment = self.bind_holes(call, classes, evaluate)?;
        for statement in &self.statements {
            if let Statement::Write(write) = statement {
                let Value::Reference(object) = environment.value(write.from)? else {
                    bail!("write source {:?} is not an object", write.from);
                };
                let object = *object;
                let value = environment.value(write.to)?.clone();
                environment.call.objects[object.0]
                    .fields
                    .insert(write.field.clone(), value);
            }
        }
        Ok(environment)
    }
}

pub type ClassDefinitions = BTreeMap<String, BTreeMap<String, Value>>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Object {
    pub class: String,
    pub fields: BTreeMap<String, Value>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Environment {
    pub objects: Vec<Object>,
    pub bindings: BTreeMap<String, Value>,
}

impl Environment {
    fn validate_references(&self) -> Result<()> {
        for value in self.bindings.values().chain(
            self.objects
                .iter()
                .flat_map(|object| object.fields.values()),
        ) {
            if let Value::Reference(object) = value {
                ensure!(
                    object.0 < self.objects.len(),
                    "reference points outside the heap: {:?}",
                    object
                );
            }
        }
        Ok(())
    }

    fn has_type(&self, value: &Value, value_type: &ValueType) -> bool {
        match (value, value_type) {
            (Value::Undefined, ValueType::Undefined)
            | (Value::Boolean(_), ValueType::Boolean)
            | (Value::Number(_), ValueType::Number)
            | (Value::String(_), ValueType::String)
            | (Value::Null, ValueType::Reference(_)) => true,
            (Value::Reference(id), ValueType::Reference(class)) => self
                .objects
                .get(id.0)
                .is_some_and(|object| &object.class == class),
            _ => false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HoleEnvironment {
    pub call: Environment,
    pub vertices: BTreeMap<VertexId, Value>,
}

impl HoleEnvironment {
    pub fn value(&self, vertex: VertexId) -> Result<&Value> {
        self.vertices
            .get(&vertex)
            .ok_or_else(|| anyhow::anyhow!("unbound vertex: {:?}", vertex))
    }
}
