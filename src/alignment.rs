//! Vertex correspondence between operation graphs (paper §4.2): a bijection that respects
//! `~` and preserves edge direction, field names, and multiplicities. Edge order is ignored.

use std::collections::BTreeMap;

use anyhow::{bail, ensure, Result};

use crate::operation_graph::{OperationGraph, VertexId};

/// `mapping[v]` is the vertex of the other graph that vertex `v` of the first graph maps to.
pub type Mapping = Vec<VertexId>;

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
    let edges_a = edge_counts(a);
    let edges_b = edge_counts(b);
    let mut search = Search {
        a,
        b,
        edges_a: &edges_a,
        edges_b: &edges_b,
        mapping: vec![None; a.vertices.len()],
        used: vec![false; b.vertices.len()],
    };
    if !search.extend(0) {
        bail!("the operation graph is not isomorphic to the first one");
    }
    Ok(search.mapping.into_iter().map(Option::unwrap).collect())
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

struct Search<'a> {
    a: &'a OperationGraph,
    b: &'a OperationGraph,
    edges_a: &'a EdgeCounts,
    edges_b: &'a EdgeCounts,
    mapping: Vec<Option<VertexId>>,
    used: Vec<bool>,
}

impl Search<'_> {
    fn extend(&mut self, vertex: VertexId) -> bool {
        if vertex == self.a.vertices.len() {
            return self.a.ret.map(|r| self.mapping[r]) == self.b.ret.map(Some);
        }
        for candidate in 0..self.b.vertices.len() {
            if self.used[candidate]
                || !self.a.vertices[vertex].corresponds(&self.b.vertices[candidate])
            {
                continue;
            }
            self.mapping[vertex] = Some(candidate);
            self.used[candidate] = true;
            if self.consistent(vertex) && self.extend(vertex + 1) {
                return true;
            }
            self.mapping[vertex] = None;
            self.used[candidate] = false;
        }
        false
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
        agrees(self.edges_a, self.edges_b, vertex, &|v| self.mapping[v])
            && agrees(self.edges_b, self.edges_a, image, &|v| {
                reverse.get(&v).copied()
            })
    }
}
