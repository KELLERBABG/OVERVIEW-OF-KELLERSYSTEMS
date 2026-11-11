use sybil_core::{NodeId, SybilResult};
use crate::graph::PoPGraph;

/// Cluster analysis utilities for the social graph.
pub struct ClusterAnalyzer<'a> {
    graph: &'a PoPGraph,
}

impl<'a> ClusterAnalyzer<'a> {
    pub fn new(graph: &'a PoPGraph) -> Self {
        Self { graph }
    }

    /// Compute the local clustering coefficient for a single node.
    pub fn local_clustering_coefficient(&self, node_id: &NodeId) -> SybilResult<Option<f64>> {
        let g = self.graph.inner();
        let idx = self.graph.node_index(node_id)?;
        let neighbors: Vec<_> = g.neighbors(idx).collect();
        let deg = neighbors.len();
        if deg < 2 {
            return Ok(None);
        }
        let mut triangles = 0;
        for i in 0..neighbors.len() {
            for j in i + 1..neighbors.len() {
                if g.find_edge(neighbors[i], neighbors[j]).is_some() {
                    triangles += 1;
                }
            }
        }
        let possible = (deg * (deg - 1)) / 2;
        Ok(Some(triangles as f64 / possible as f64))
    }

    /// Compute clustering coefficients for all nodes, returns (min, max, mean, std_dev).
    pub fn clustering_coefficient_statistics(&self) -> SybilResult<(f64, f64, f64, f64)> {
        let g = self.graph.inner();
        let mut ccs = Vec::new();
        for node in g.node_indices() {
            let neighbors: Vec<_> = g.neighbors(node).collect();
            let deg = neighbors.len();
            if deg < 2 {
                continue;
            }
            let mut triangles = 0;
            for i in 0..neighbors.len() {
                for j in i + 1..neighbors.len() {
                    if g.find_edge(neighbors[i], neighbors[j]).is_some() {
                        triangles += 1;
                    }
                }
            }
            let possible = (deg * (deg - 1)) / 2;
            if possible > 0 {
                ccs.push(triangles as f64 / possible as f64);
            }
        }
        if ccs.is_empty() {
            return Ok((0.0, 0.0, 0.0, 0.0));
        }
        let min = ccs.iter().cloned().fold(f64::MAX, |a, b| a.min(b));
        let max = ccs.iter().cloned().fold(f64::MIN, |a, b| a.max(b));
        let mean = ccs.iter().sum::<f64>() / ccs.len() as f64;
        let variance = ccs.iter().map(|c| (c - mean).powi(2)).sum::<f64>() / ccs.len() as f64;
        let std_dev = variance.sqrt();
        Ok((min, max, mean, std_dev))
    }
}