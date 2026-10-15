//! Vertex correspondence between operation graphs (paper §4.2): a bijection that respects
//! `~` and preserves edge direction, field names, and multiplicities. Edge order is ignored.

use std::cell::Cell;
use std::collections::BTreeMap;
use std::ops::ControlFlow;

use anyhow::{bail, ensure, Result};

use crate::demonstration::Recording;
use crate::operation_graph::{OperationGraph, VertexId};
use crate::pbe_problem::{self, Problems};
use crate::template::Template;

/// `mapping[v]` is the vertex of the other graph that vertex `v` of the first graph maps to.
pub type Mapping = Vec<VertexId>;

/// Steps `align_consistently` may take. One step is one candidate tried for one vertex in any
/// demonstration's isomorphism search, whether or not it is accepted, or one complete
/// combination of correspondences checked against the hole examples.
pub const SEARCH_STEP_LIMIT: usize = 10_000;

/// Maps the first graph onto every graph; the first mapping is the identity.
pub fn align_all(graphs: &[OperationGraph]) -> Result<Vec<Mapping>> {
    let Some(first) = graphs.first() else {
        bail!("no operation graphs to align");
    };
    graphs
        .iter()
        .enumerate()
        .map(|(index, graph)| {
            align(first, graph).map_err(|err| err.context(format!("demonstration {}", index + 1)))
        })
        .collect()
}

/// Returns the first isomorphism found when trying the first graph's vertices in declaration
/// order against candidates in the other graph's declaration order.
pub fn align(a: &OperationGraph, b: &OperationGraph) -> Result<Mapping> {
    let mut first = None;
    let steps = Cell::new(usize::MAX);
    let _ = Search::new(a, b, &steps)?.run(&mut |mapping| {
        first = Some(mapping.clone());
        Ok(ControlFlow::Break(()))
    })?;
    first.ok_or_else(|| anyhow::anyhow!("the operation graph is not isomorphic to the first one"))
}

pub type Aligned = (Vec<Mapping>, Template, Problems);

/// The paper picks one of several isomorphisms without saying which. This generates each
/// demonstration's isomorphisms one at a time, in `align` order, and stops at the first
/// combination under which no hole receives the same encoded input in two demonstrations with
/// different outputs; such a PBE problem has no solution. mu_1 is the identity.
pub fn align_consistently(
    recording: &Recording,
    graphs: &[OperationGraph],
    params: &[String],
) -> Result<Aligned> {
    let Some(first) = graphs.first() else {
        bail!("no operation graphs to align");
    };
    let mut chooser = Chooser {
        recording,
        graphs,
        params,
        mappings: vec![(0..first.vertices.len()).collect()],
        chosen: None,
    };
    let _ = chooser.extend(&Cell::new(SEARCH_STEP_LIMIT))?;
    chooser.chosen.ok_or_else(|| {
        anyhow::anyhow!(
            "every correspondence between the demonstrations gives some hole the same input with different outputs"
        )
    })
}

struct Chooser<'a> {
    recording: &'a Recording,
    graphs: &'a [OperationGraph],
    params: &'a [String],
    mappings: Vec<Mapping>,
    chosen: Option<Aligned>,
}

impl Chooser<'_> {
    fn extend(&mut self, steps: &Cell<usize>) -> Result<ControlFlow<()>> {
        let index = self.mappings.len();
        if index == self.graphs.len() {
            return self.check(steps);
        }
        let graphs = self.graphs;
        let mut any = false;
        let flow = Search::new(&graphs[0], &graphs[index], steps)
            .map_err(|err| err.context(format!("demonstration {}", index + 1)))?
            .run(&mut |mapping| {
                any = true;
                self.mappings.push(mapping.clone());
                let flow = self.extend(steps);
                self.mappings.pop();
                flow
            })?;
        if !any {
            bail!(
                "demonstration {}: the operation graph is not isomorphic to the first one",
                index + 1
            );
        }
        Ok(flow)
    }

    /// Template and PBE construction errors do not depend on the mappings: they concern the
    /// first graph, the pre-call states, the arguments, or the created objects and constants,
    /// whose classes and types `~` preserves. So an error ends the search.
    fn check(&mut self, steps: &Cell<usize>) -> Result<ControlFlow<()>> {
        take_step(steps)?;
        let template = Template::build(self.graphs, &self.mappings)?;
        let problems = pbe_problem::build(
            self.recording,
            self.graphs,
            &self.mappings,
            &template,
            self.params,
        )?;
        if contradictory(&problems) {
            return Ok(ControlFlow::Continue(()));
        }
        self.chosen = Some((self.mappings.clone(), template, problems));
        Ok(ControlFlow::Break(()))
    }
}

fn take_step(steps: &Cell<usize>) -> Result<()> {
    let Some(left) = steps.get().checked_sub(1) else {
        bail!(
            "the correspondence search took {SEARCH_STEP_LIMIT} steps (vertex candidates tried plus combinations checked) without finding one whose hole examples agree"
        );
    };
    steps.set(left);
    Ok(())
}

fn contradictory(problems: &Problems) -> bool {
    problems.tasks.iter().any(|task| {
        task.examples
            .iter()
            .enumerate()
            .any(|(i, (input, output))| {
                task.examples[i + 1..]
                    .iter()
                    .any(|(other_input, other_output)| {
                        input == other_input && output != other_output
                    })
            })
    })
}

type EdgeCounts = BTreeMap<(VertexId, String, VertexId), usize>;

fn edge_counts(graph: &OperationGraph) -> EdgeCounts {
    let mut counts = EdgeCounts::new();
    for write in &graph.writes {
        *counts
            .entry((write.from, write.field.clone(), write.to))
            .or_default() += 1;
    }
    counts
}

type Visit<'v> = dyn FnMut(&Mapping) -> Result<ControlFlow<()>> + 'v;

struct Search<'a> {
    a: &'a OperationGraph,
    b: &'a OperationGraph,
    edges_a: EdgeCounts,
    edges_b: EdgeCounts,
    mapping: Vec<Option<VertexId>>,
    used: Vec<bool>,
    steps: &'a Cell<usize>,
}

impl<'a> Search<'a> {
    fn new(a: &'a OperationGraph, b: &'a OperationGraph, steps: &'a Cell<usize>) -> Result<Self> {
        ensure!(
            a.vertices.len() == b.vertices.len() && a.writes.len() == b.writes.len(),
            "the operation graph is not isomorphic to the first one ({} vertices and {} writes vs {} and {})",
            b.vertices.len(),
            b.writes.len(),
            a.vertices.len(),
            a.writes.len()
        );
        ensure!(
            a.ret.is_some() == b.ret.is_some(),
            "only one of the demonstrations sets the return value"
        );
        Ok(Search {
            a,
            b,
            edges_a: edge_counts(a),
            edges_b: edge_counts(b),
            mapping: vec![None; a.vertices.len()],
            used: vec![false; b.vertices.len()],
            steps,
        })
    }

    /// Hands each isomorphism to `visit` as soon as it is found, until `visit` breaks.
    fn run(mut self, visit: &mut Visit<'_>) -> Result<ControlFlow<()>> {
        self.extend(0, visit)
    }

    fn extend(&mut self, vertex: VertexId, visit: &mut Visit<'_>) -> Result<ControlFlow<()>> {
        if vertex == self.a.vertices.len() {
            if self.a.ret.map(|r| self.mapping[r]) != self.b.ret.map(Some) {
                return Ok(ControlFlow::Continue(()));
            }
            return visit(&self.mapping.iter().map(|v| v.unwrap()).collect());
        }
        for candidate in 0..self.b.vertices.len() {
            take_step(self.steps)?;
            if self.used[candidate]
                || !self.a.vertices[vertex].corresponds(&self.b.vertices[candidate])
            {
                continue;
            }
            self.mapping[vertex] = Some(candidate);
            self.used[candidate] = true;
            if self.consistent(vertex) && self.extend(vertex + 1, visit)?.is_break() {
                return Ok(ControlFlow::Break(()));
            }
            self.mapping[vertex] = None;
            self.used[candidate] = false;
        }
        Ok(ControlFlow::Continue(()))
    }

    /// Checks every edge between `vertex` and already mapped vertices, in both directions,
    /// so that multiplicities agree once all vertices are mapped.
    fn consistent(&self, vertex: VertexId) -> bool {
        let reverse: BTreeMap<VertexId, VertexId> = self
            .mapping
            .iter()
            .enumerate()
            .filter_map(|(v, image)| image.map(|image| (image, v)))
            .collect();
        let image = self.mapping[vertex].expect("vertex was just mapped");
        let agrees = |edges: &EdgeCounts,
                      other: &EdgeCounts,
                      pivot: VertexId,
                      translate: &dyn Fn(VertexId) -> Option<VertexId>| {
            edges.iter().all(|((from, field, to), n)| {
                if *from != pivot && *to != pivot {
                    return true;
                }
                match (translate(*from), translate(*to)) {
                    (Some(f), Some(t)) => {
                        other.get(&(f, field.clone(), t)).copied().unwrap_or(0) == *n
                    }
                    _ => true,
                }
            })
        };
        agrees(&self.edges_a, &self.edges_b, vertex, &|v| self.mapping[v])
            && agrees(&self.edges_b, &self.edges_a, image, &|v| {
                reverse.get(&v).copied()
            })
    }
}
