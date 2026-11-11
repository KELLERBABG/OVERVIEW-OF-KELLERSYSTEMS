use petgraph::algo::dijkstra;
use petgraph::visit::{EdgeRef, IntoNodeReferences, NodeIndexable};
use sybil_core::{NodeId, SybilResult};

use crate::graph::PoPGraph;

/// Centrality analysis for the social graph.
pub struct CentralityAnalyzer<'a> {
    graph: &'a PoPGraph,
}

impl<'a> CentralityAnalyzer<'a> {
    pub fn new(graph: &'a PoPGraph) -> Self {
        Self { graph }
    }

    /// Compute degree centrality for a node (normalized by max possible degree).
    pub fn degree_centrality(&self, node_id: &NodeId) -> SybilResult<f64> {
        let g = self.graph.inner();
        let idx = self.graph.node_index(node_id)?;
        let deg = g.neighbors(idx).count() as f64;
        let max_deg = (g.node_count() - 1) as f64;
        Ok(if max_deg > 0.0 { deg / max_deg } else { 0.0 })
    }

    /// Compute betweenness centrality for a node (sampled approximation).
    pub fn betweenness_centrality(&self, node_id: &NodeId) -> SybilResult<Option<f64>> {
        let g = self.graph.inner();
        let idx = self.graph.node_index(node_id)?;
        let nodes: Vec<_> = g.node_indices().collect();
        let n = nodes.len();
        if n < 3 {
            return Ok(None);
        }
        let sample_size = n.min(50);
        let sampled: Vec<_> = nodes[..sample_size].to_vec();
        let mut total_shortest_paths = 0u64;
        let mut paths_through_node = 0u64;
        for source in &sampled {
            if *source == idx {
                continue;
            }
            let dists = dijkstra(g, *source, None, |e| *e.weight());
            for (target, dist) in &dists {
                if *target == idx || *target == *source || *dist >= f64::MAX {
                    continue;
                }
                total_shortest_paths += 1;
                for neighbor in g.neighbors(*target) {
                    if neighbor == idx {
                        paths_through_node += 1;
                        break;
                    }
                }
            }
        }
        Ok(Some(if total_shortest_paths > 0 {
            paths_through_node as f64 / total_shortest_paths as f64
        } else {
            0.0
        }))
    }

    /// Compute eigenvector centrality using power iteration.
    pub fn eigenvector_centrality(
        &self,
        max_iterations: usize,
        tolerance: f64,
    ) -> SybilResult<Vec<(NodeId, f64)>> {
        let g = self.graph.inner();
        let n = g.node_count();
        if n == 0 {
            return Ok(Vec::new());
        }
        let mut scores = vec![1.0 / n as f64; n];
        let node_ids: Vec<NodeId> = g.node_references()
            .map(|(_, meta)| meta.node_id.clone())
            .collect();
        for _iteration in 0..max_iterations {
            let mut new_scores = vec![0.0; n];
            for i in 0..n {
                let ni = g.from_index(i);
                for neighbor in g.neighbors(ni) {
                    let j = g.to_index(neighbor);
                    new_scores[i] += scores[j];
                }
            }
            let norm: f64 = new_scores.iter().map(|s| s.powi(2)).sum::<f64>().sqrt();
            if norm < f64::EPSILON {
                break;
            }
            for s in new_scores.iter_mut() {
                *s /= norm;
            }
            let diff: f64 = scores.iter().zip(new_scores.iter())
                .map(|(a, b)| (a - b).abs()).sum();
            scores = new_scores;
            if diff < tolerance {
                break;
            }
        }
        Ok(node_ids.into_iter().zip(scores).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::synthetic::SyntheticGraphGenerator;
    use sybil_core::SyntheticGraphConfig;

    #[test]
    fn test_degree_centrality() {
        let config = SyntheticGraphConfig { num_nodes: 50, ..Default::default() };
        let generator = SyntheticGraphGenerator::new(config);
        let (graph, _) = generator.generate().unwrap();
        let analyzer = CentralityAnalyzer::new(&graph);
        let nodes = graph.all_node_ids();
        if !nodes.is_empty() {
            let dc = analyzer.degree_centrality(&nodes[0]).unwrap();
            assert!((0.0..=1.0).contains(&dc));
        }
    }

    #[test]
    fn test_eigenvector_centrality() {
        let config = SyntheticGraphConfig { num_nodes: 50, ..Default::default() };
        let generator = SyntheticGraphGenerator::new(config);
        let (graph, _) = generator.generate().unwrap();
        let analyzer = CentralityAnalyzer::new(&graph);
        let centralities = analyzer.eigenvector_centrality(100, 1e-6).unwrap();
        assert_eq!(centralities.len(), graph.node_count());
    }

    #[test]
    fn test_betweenness_centrality() {
        let config = SyntheticGraphConfig { num_nodes: 30, ..Default::default() };
        let generator = SyntheticGraphGenerator::new(config);
        let (graph, _) = generator.generate().unwrap();
        let analyzer = CentralityAnalyzer::new(&graph);
        let nodes = graph.all_node_ids();
        if nodes.len() >= 3 {
            let bc = analyzer.betweenness_centrality(&nodes[0]).unwrap();
            assert!(bc.is_some());
            assert!((0.0..=1.0).contains(&bc.unwrap()));
        }
    }
}