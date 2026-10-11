//! Holed template built from the first operation graph and the correspondences
//! (paper §4.2), and its lowering to statements (paper §4.3).

use std::collections::BTreeSet;

use anyhow::{bail, ensure, Result};

use crate::alignment::Mapping;
use crate::operation_graph::{Constant, OperationGraph, Vertex, VertexId, Write};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HoleType {
    Number,
    String,
    Bool,
    Object(String),
}

/// A hole's output in one demonstration: the corresponding vertex `mu_i(n)`.
#[derive(Debug, Clone, PartialEq)]
pub enum Example {
    Object(String),
    Constant(Constant),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Hole {
    pub vertex: VertexId,
    pub ty: HoleType,
    /// One output per demonstration, in demonstration order.
    pub examples: Vec<Example>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Create {
    pub vertex: VertexId,
    pub class: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Template {
    vertices: Vec<Vertex>,
    pub creates: Vec<Create>,
    /// Holes in their fixed binding order.
    pub holes: Vec<Hole>,
    pub writes: Vec<Write>,
    pub ret: Option<VertexId>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Statement<'a> {
    Create(&'a Create),
    BindHole(usize, &'a Hole),
    Write(&'a Write),
    Return(VertexId),
}

fn is_hole(vertex: &Vertex) -> bool {
    matches!(vertex, Vertex::Existing { .. } | Vertex::Constant(_))
}

impl Template {
    pub fn build(graphs: &[OperationGraph], mappings: &[Mapping]) -> Result<Template> {
        let Some(first) = graphs.first() else {
            bail!("no operation graphs");
        };
        ensure!(
            graphs.len() == mappings.len(),
            "one mapping per demonstration is required"
        );
        // The paper fixes the binding order before synthesis without naming it; first
        // appearance in E_1 reproduces the order of its insert example.
        let mut order = Vec::new();
        let mut seen = BTreeSet::new();
        let appearances = first
            .writes
            .iter()
            .flat_map(|w| [w.from, w.to])
            .chain(first.ret);
        for vertex in appearances {
            if is_hole(&first.vertices[vertex]) && seen.insert(vertex) {
                order.push(vertex);
            }
        }
        let holes = order
            .into_iter()
            .map(|vertex| {
                let ty = match &first.vertices[vertex] {
                    Vertex::Existing { class, .. } => HoleType::Object(class.clone()),
                    Vertex::Constant(Constant::Number(_)) => HoleType::Number,
                    Vertex::Constant(Constant::String(_)) => HoleType::String,
                    Vertex::Constant(Constant::Bool(_)) => HoleType::Bool,
                    other => unreachable!("not a hole: {other:?}"),
                };
                let examples = graphs
                    .iter()
                    .zip(mappings)
                    .map(|(graph, mapping)| match &graph.vertices[mapping[vertex]] {
                        Vertex::Existing { object, .. } => Example::Object(object.clone()),
                        Vertex::Constant(c) => Example::Constant(c.clone()),
                        other => unreachable!("the correspondence maps a hole to {other:?}"),
                    })
                    .collect();
                Hole {
                    vertex,
                    ty,
                    examples,
                }
            })
            .collect();
        let creates = first
            .vertices
            .iter()
            .enumerate()
            .filter_map(|(vertex, v)| match v {
                Vertex::Created { class, .. } => Some(Create {
                    vertex,
                    class: class.clone(),
                }),
                _ => None,
            })
            .collect();
        Ok(Template {
            vertices: first.vertices.clone(),
            creates,
            holes,
            writes: first.writes.clone(),
            ret: first.ret,
        })
    }

    /// Creations in declaration order, then hole bindings, then writes in the order of E_1
    /// (duplicates kept), then the return value.
    pub fn statements(&self) -> Vec<Statement<'_>> {
        self.creates
            .iter()
            .map(Statement::Create)
            .chain(
                self.holes
                    .iter()
                    .enumerate()
                    .map(|(k, h)| Statement::BindHole(k, h)),
            )
            .chain(self.writes.iter().map(Statement::Write))
            .chain(self.ret.map(Statement::Return))
            .collect()
    }

    /// Local variables visible to hole `k`: every created object, then the holes before `k`.
    pub fn hole_inputs(&self, k: usize) -> Vec<VertexId> {
        self.creates
            .iter()
            .map(|c| c.vertex)
            .chain(self.holes[..k].iter().map(|h| h.vertex))
            .collect()
    }

    pub fn helper_name(method: &str, k: usize) -> String {
        format!("{method}_h{k}")
    }

    /// Renders the method; hole `k` calls its helper with the method parameters followed by
    /// `hole_inputs(k)`.
    pub fn render_method(&self, method: &str, params: &[String]) -> String {
        let names = self.local_names(params);
        let rho = |vertex: VertexId| match self.vertices[vertex] {
            Vertex::Null => "null".to_string(),
            _ => names[vertex]
                .clone()
                .expect("every non-null vertex is bound"),
        };
        let mut body = Vec::new();
        for statement in self.statements() {
            body.push(match statement {
                Statement::Create(c) => format!("const {} = new {}();", rho(c.vertex), c.class),
                Statement::BindHole(k, h) => {
                    let args = params
                        .iter()
                        .cloned()
                        .chain(self.hole_inputs(k).into_iter().map(rho))
                        .collect::<Vec<_>>()
                        .join(", ");
                    format!(
                        "const {} = this.{}({args});",
                        rho(h.vertex),
                        Self::helper_name(method, k)
                    )
                }
                Statement::Write(w) => format!("{}.{} = {};", rho(w.from), w.field, rho(w.to)),
                Statement::Return(v) => format!("return {};", rho(v)),
            });
        }
        format!(
            "{method}({}) {{\n{}\n}}",
            params.join(", "),
            body.iter()
                .map(|line| format!("    {line}"))
                .collect::<Vec<_>>()
                .join("\n")
        )
    }

    pub(crate) fn local_names(&self, params: &[String]) -> Vec<Option<String>> {
        let mut taken: BTreeSet<String> = params.iter().cloned().collect();
        let mut fresh = |base: String| {
            let mut name = base;
            while !taken.insert(name.clone()) {
                name.push('_');
            }
            name
        };
        let mut names = vec![None; self.vertices.len()];
        for (k, c) in self.creates.iter().enumerate() {
            names[c.vertex] = Some(fresh(format!("created{k}")));
        }
        for (k, h) in self.holes.iter().enumerate() {
            names[h.vertex] = Some(fresh(format!("hole{k}")));
        }
        names
    }
}
