use std::collections::HashMap;

use sybil_core::{constants::*, SybilResult};
use sybil_graph::graph::PoPGraph;
use sybil_graph::clustering::ClusterAnalyzer;
use sybil_graph::topology::TopologyAnalyzer;

/// Graph entropy analysis for Sybil detection.
///
/// Measures how closely a graph region matches expected properties
/// of human social networks (small-world characteristics).
pub struct EntropyAnalyzer<'a> {
    graph: &'a PoPGraph,
}

impl<'a> EntropyAnalyzer<'a> {
    pub fn new(graph: &'a PoPGraph) -> Self {
        Self { graph }
    }

    /// Compute the graph entropy score.
    ///
    /// The entropy score measures how much the graph deviates from
    /// expected small-world properties. Higher scores indicate more
    /// "random" structure (potential Sybil activity).
    pub fn graph_entropy(&self) -> SybilResult<f64> {
        let n = self.graph.node_count();
        if n < MIN_GRAPH_SIZE_FOR_DETECTION {
            return Ok(1.0); // Maximum entropy for small graphs
        }

        let topo = TopologyAnalyzer::new(self.graph);
        let cc = topo.average_clustering_coefficient()?;
        let apl = topo.average_path_length()?;

        // Entropy = deviation from expected human social network properties
        let cc_expected = (EXPECTED_HUMAN_CLUSTERING_MIN + EXPECTED_HUMAN_CLUSTERING_MAX) / 2.0;
        let cc_deviation = if cc > 0.0 {
            ((cc - cc_expected) / cc_expected).abs()
        } else {
            1.0
        };

        let expected_apl = (n as f64).ln() / EXPECTED_SMALL_WORLD_DEGREE.ln();
        let apl_ratio = if expected_apl > 0.0 {
            apl / expected_apl
        } else {
            1.0
        };

        // Combine metrics: entropy = clustering_deviation * path_length_ratio
        let entropy = cc_deviation * apl_ratio;
        Ok(entropy.clamp(0.0, 1.0))
    }

    /// Compute entropy for a specific set of nodes (a region/cluster).
    pub fn region_entropy(&self, nodes: &[sybil_core::NodeId]) -> SybilResult<f64> {
        if nodes.len() < MIN_CLUSTER_SIZE_FOR_ANALYSIS {
            return Ok(0.5); // Neutral score for small clusters
        }

        let cluster_analyzer = ClusterAnalyzer::new(self.graph);
        let mut ccs = Vec::new();
        for node in nodes {
            if let Some(cc) = cluster_analyzer.local_clustering_coefficient(node)? {
                ccs.push(cc);
            }
        }

        if ccs.is_empty() {
            return Ok(1.0);
        }

        let mean_cc = ccs.iter().sum::<f64>() / ccs.len() as f64;

        // High internal clustering suggests Sybil (bots connect densely among themselves)
        // But too low also suggests Sybil (isolated nodes)
        let expected_cc = EXPECTED_HUMAN_CLUSTERING_MIN + 0.25;
        let deviation = (mean_cc - expected_cc).abs() / expected_cc;

        Ok(deviation.clamp(0.0, 1.0))
    }

    /// Compute internal edge density for a set of nodes.
    pub fn internal_edge_density(&self, nodes: &[sybil_core::NodeId]) -> SybilResult<f64> {
        if nodes.len() < 2 {
            return Ok(0.0);
        }
        let node_set: std::collections::HashSet<_> = nodes.iter().collect();
        let mut internal_edges = 0u64;
        let max_possible = (nodes.len() * (nodes.len() - 1)) / 2;

        for node in nodes {
            if let Ok(neighbors) = self.graph.neighbors(node) {
                for neighbor in &neighbors {
                    if node_set.contains(neighbor) {
                        internal_edges += 1;
                    }
                }
            }
        }
        // Divide by 2 because each edge counted twice
        internal_edges /= 2;

        Ok(if max_possible > 0 {
            internal_edges as f64 / max_possible as f64
        } else {
            0.0
        })
    }

    /// Compute external edge density (edges to nodes outside the set).
    pub fn external_edge_density(&self, nodes: &[sybil_core::NodeId]) -> SybilResult<f64> {
        if nodes.is_empty() {
            return Ok(0.0);
        }
        let node_set: std::collections::HashSet<_> = nodes.iter().collect();
        let mut external_edges = 0u64;

        for node in nodes {
            if let Ok(neighbors) = self.graph.neighbors(node) {
                for neighbor in &neighbors {
                    if !node_set.contains(neighbor) {
                        external_edges += 1;
                    }
                }
            }
        }

        let total_possible = nodes.len() * (self.graph.node_count() - nodes.len());
        Ok(if total_possible > 0 {
            external_edges as f64 / total_possible as f64
        } else {
            0.0
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sybil_core::SyntheticGraphConfig;
    use sybil_graph::synthetic::SyntheticGraphGenerator;

    #[test]
    fn test_graph_entropy_small_world() {
        let config = SyntheticGraphConfig {
            num_nodes: 200,
            ..Default::default()
        };
        let generator = SyntheticGraphGenerator::new(config);
        let (graph, _) = generator.generate().unwrap();
        let analyzer = EntropyAnalyzer::new(&graph);
        let entropy = analyzer.graph_entropy().unwrap();
        // Small-world human graphs should have relatively low entropy
        assert!((0.0..=1.0).contains(&entropy));
    }

    #[test]
    fn test_internal_density() {
        let config = SyntheticGraphConfig {
            num_nodes: 100,
            sybil_cluster_size: 10,
            num_sybil_clusters: 1,
            ..Default::default()
        };
        let generator = SyntheticGraphGenerator::new(config);
        let (graph, _) = generator.generate().unwrap();
        let analyzer = EntropyAnalyzer::new(&graph);
        // Get all nodes and compute internal density for the whole graph
        let all_nodes = graph.all_node_ids();
        let density = analyzer.internal_edge_density(&all_nodes).unwrap();
        assert!((0.0..=1.0).contains(&density));
    }
}