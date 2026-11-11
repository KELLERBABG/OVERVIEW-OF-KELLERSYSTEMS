use petgraph::algo::dijkstra;
use petgraph::visit::EdgeRef;
use sybil_core::{constants::*, SybilResult};

use crate::graph::PoPGraph;

/// Small-world network analysis for the social graph.
pub struct TopologyAnalyzer<'a> {
    graph: &'a PoPGraph,
}

impl<'a> TopologyAnalyzer<'a> {
    pub fn new(graph: &'a PoPGraph) -> Self {
        Self { graph }
    }

    /// Compute the average clustering coefficient for the entire graph.
    pub fn average_clustering_coefficient(&self) -> SybilResult<f64> {
        let g = self.graph.inner();
        let n = g.node_count();
        if n < 3 {
            return Ok(0.0);
        }
        let mut total_cc = 0.0;
        let mut count = 0;
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
                total_cc += triangles as f64 / possible as f64;
                count += 1;
            }
        }
        Ok(if count > 0 { total_cc / count as f64 } else { 0.0 })
    }

    /// Check if the graph exhibits small-world properties.
    pub fn is_small_world(&self) -> SybilResult<bool> {
        let n = self.graph.node_count();
        if n < 10 {
            return Ok(false);
        }
        let cc = self.average_clustering_coefficient()?;
        let apl = self.average_path_length()?;
        let random_cc = self.graph.edge_count() as f64 / (n as f64 * (n as f64 - 1.0) / 2.0);
        let random_apl = (n as f64).ln() / EXPECTED_SMALL_WORLD_DEGREE.ln();
        Ok(cc > random_cc * 10.0 && apl <= random_apl * 1.5)
    }

    /// Compute average path length using Dijkstra from a sample of nodes.
    pub fn average_path_length(&self) -> SybilResult<f64> {
        let g = self.graph.inner();
        let nodes: Vec<_> = g.node_indices().collect();
        let n = nodes.len();
        if n < 2 {
            return Ok(0.0);
        }
        let sample_size = n.min(100);
        let sampled: Vec<_> = nodes[..sample_size].to_vec();
        let mut total_dist = 0.0;
        let mut count = 0;
        for source in sampled {
            let dists = dijkstra(g, source, None, |e| *e.weight());
            for (target, d) in dists.iter() {
                if *target != source && *d < f64::MAX {
                    total_dist += d;
                    count += 1;
                }
            }
        }
        Ok(if count > 0 { total_dist / count as f64 } else { 0.0 })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::synthetic::SyntheticGraphGenerator;
    use sybil_core::SyntheticGraphConfig;

    #[test]
    fn test_clustering_coefficient_small_world() {
        let config = SyntheticGraphConfig { num_nodes: 200, ..Default::default() };
        let generator = SyntheticGraphGenerator::new(config);
        let (graph, _) = generator.generate().unwrap();
        let analyzer = TopologyAnalyzer::new(&graph);
        let cc = analyzer.average_clustering_coefficient().unwrap();
        assert!(cc > 0.1, "clustering coefficient too low: {}", cc);
    }

    #[test]
    fn test_is_small_world() {
        let config = SyntheticGraphConfig { num_nodes: 200, ..Default::default() };
        let generator = SyntheticGraphGenerator::new(config);
        let (graph, _) = generator.generate().unwrap();
        let analyzer = TopologyAnalyzer::new(&graph);
        assert!(analyzer.is_small_world().unwrap());
    }

    #[test]
    fn test_empty_graph_not_small_world() {
        let graph = PoPGraph::new();
        let analyzer = TopologyAnalyzer::new(&graph);
        assert!(!analyzer.is_small_world().unwrap());
    }
}