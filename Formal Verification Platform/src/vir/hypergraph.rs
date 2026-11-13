//! SSA Hypergraph representation for the Keller-VIR foundation.
//!
//! A hypergraph generalizes a graph: edges (hyperedges) can connect
//! arbitrary subsets of vertices, enabling multi-operand operations
//! and loop-carried dependencies to be represented naturally.

use std::collections::HashMap;
use std::fmt;

/// A vertex in the SSA hypergraph.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Vertex {
    pub id: usize,
    pub label: String,
}

impl fmt::Display for Vertex {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "v{}({})", self.id, self.label)
    }
}

/// A hyperedge connecting a set of vertices.
#[derive(Debug, Clone)]
pub struct HyperEdge {
    pub id: usize,
    pub source_vertices: Vec<usize>,
    pub target_vertex: usize,
    pub op: String,
}

/// The SSA Hypergraph.
#[derive(Debug, Clone)]
pub struct SsaHypergraph {
    pub vertices: Vec<Vertex>,
    pub hyperedges: Vec<HyperEdge>,
    pub entry: Option<usize>,
    pub exit: Option<usize>,
}

impl SsaHypergraph {
    pub fn new() -> Self {
        Self {
            vertices: Vec::new(),
            hyperedges: Vec::new(),
            entry: None,
            exit: None,
        }
    }

    pub fn add_vertex(&mut self, label: &str) -> usize {
        let id = self.vertices.len();
        self.vertices.push(Vertex { id, label: label.to_string() });
        id
    }

    pub fn add_hyperedge(&mut self, sources: Vec<usize>, target: usize, op: &str) -> usize {
        let id = self.hyperedges.len();
        self.hyperedges.push(HyperEdge {
            id,
            source_vertices: sources,
            target_vertex: target,
            op: op.to_string(),
        });
        id
    }

    /// Check if the hypergraph is in SSA form (each vertex has at most one defining edge).
    pub fn is_ssa(&self) -> bool {
        let mut defs: HashMap<usize, usize> = HashMap::new();
        for he in &self.hyperedges {
            match defs.get(&he.target_vertex) {
                Some(_) => return false,
                None => { defs.insert(he.target_vertex, he.id); }
            }
        }
        true
    }

    /// Number of vertices.
    pub fn num_vertices(&self) -> usize {
        self.vertices.len()
    }

    /// Number of hyperedges.
    pub fn num_edges(&self) -> usize {
        self.hyperedges.len()
    }
}

impl Default for SsaHypergraph {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ssa_hypergraph_basic() {
        let mut hg = SsaHypergraph::new();
        let a = hg.add_vertex("a");
        let b = hg.add_vertex("b");
        let c = hg.add_vertex("c");
        hg.add_hyperedge(vec![a, b], c, "add");
        assert_eq!(hg.num_vertices(), 3);
        assert_eq!(hg.num_edges(), 1);
        assert!(hg.is_ssa());
    }

    #[test]
    fn test_non_ssa_detection() {
        let mut hg = SsaHypergraph::new();
        let a = hg.add_vertex("a");
        let b = hg.add_vertex("b");
        let c = hg.add_vertex("c");
        hg.add_hyperedge(vec![a], c, "def1");
        hg.add_hyperedge(vec![b], c, "def2"); // second def of c -> not SSA
        assert!(!hg.is_ssa());
    }
}
