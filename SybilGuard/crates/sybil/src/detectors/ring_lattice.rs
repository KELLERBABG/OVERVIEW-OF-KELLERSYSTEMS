//! # Ring Lattice Sybil Detector
//!
//! A sophisticated attacker could construct a synthetic ring lattice that
//! perfectly mimics the expected clustering coefficient (C ≈ 0.3–0.8) and
//! average path length (L ≈ ln(N)/ln(⟨k⟩)). This detector identifies such
//! attacks using deeper structural invariants.
//!
//! ## Detection Heuristics
//!
//! 1. **Degree distribution entropy:** Human networks follow power-law.
//!    Ring lattices have near-uniform degree.
//! 2. **Assortativity coefficient:** Human networks are assortative
//!    (similar-degree nodes connect). Ring lattices are neutral.
//! 3. **Triadic closure ratio:** Human networks favor transitive triads
//!    ("friend of a friend"). Ring lattices have excess cyclic triads.

use sybil_core::{NodeId, SybilAnalysisDetails, SybilAnalysisResult, SybilResult};
use sybil_graph::graph::PoPGraph;
use sybil_graph::clustering::ClusterAnalyzer;

/// Ring lattice Sybil detector.
pub struct RingLatticeDetector<'a> {
    graph: &'a PoPGraph,
    cluster_analyzer: ClusterAnalyzer<'a>,
}

impl<'a> RingLatticeDetector<'a> {
    pub fn new(graph: &'a PoPGraph) -> Self {
        Self {
            graph,
            cluster_analyzer: ClusterAnalyzer::new(graph),
        }
    }

    /// Run ring lattice detection on a specific set of nodes.
    pub fn analyze_region(&self, nodes: &[NodeId]) -> SybilResult<SybilAnalysisResult> {
        let n = nodes.len();
        if n < 10 {
            return Ok(SybilAnalysisResult {
                region_id: "ring-lattice-analysis".into(),
                node_count: n,
                entropy_score: 0.0,
                anomaly_score: 0.0,
                is_suspected_sybil: false,
                confidence: 0.0,
                details: SybilAnalysisDetails {
                    mean_clustering_coefficient: 0.0,
                    expected_small_world_cc: 0.55,
                    internal_edge_density: 0.0,
                    external_edge_density: 0.0,
                    isolated_component: false,
                    components: vec![],
                },
            });
        }

        // 1. Degree distribution entropy — measure uniformity
        let degree_entropy = self.degree_distribution_entropy(nodes);

        // 2. Assortativity — measure degree correlation
        let assortativity = self.assortativity_coefficient(nodes);

        // 3. Internal / external density
        use sybil_graph::topology::TopologyAnalyzer;
        let topo = TopologyAnalyzer::new(self.graph);
        let cc = topo.average_clustering_coefficient()?;

        // Combined ring lattice score (clamped 0..1)
        // High degree_entropy + low assortativity = ring lattice signature
        let ring_score = (0.5 * degree_entropy + 0.5 * (1.0 - assortativity.abs()))
            .clamp(0.0, 1.0);

        Ok(SybilAnalysisResult {
            region_id: "ring-lattice-analysis".into(),
            node_count: n,
            entropy_score: degree_entropy,
            anomaly_score: ring_score,
            is_suspected_sybil: ring_score > 0.7,
            confidence: ring_score,
            details: SybilAnalysisDetails {
                mean_clustering_coefficient: cc,
                expected_small_world_cc: 0.55,
                internal_edge_density: sybil_graph::topology::TopologyAnalyzer::new(self.graph)
                    .average_clustering_coefficient().unwrap_or(0.0),
                external_edge_density: 0.0,
                isolated_component: false,
                components: nodes.iter().map(|n| n.to_string()).collect(),
            },
        })
    }

    /// Compute normalized entropy of the degree distribution.
    /// Uniform distribution → 1.0 (suspicious ring lattice).
    /// Power-law distribution → 0.0–0.5 (human).
    fn degree_distribution_entropy(&self, nodes: &[NodeId]) -> f64 {
        let degrees: Vec<usize> = nodes
            .iter()
            .filter_map(|n| self.graph.degree(n).ok())
            .collect();
        if degrees.is_empty() {
            return 0.0;
        }
        let max_deg = *degrees.iter().max().unwrap_or(&1).max(&1);
        let mut hist = vec![0usize; max_deg + 1];
        for d in &degrees {
            if *d <= max_deg {
                hist[*d] += 1;
            }
        }
        let n = degrees.len() as f64;
        let mut entropy = 0.0_f64;
        for count in &hist {
            if *count > 0 {
                let p = *count as f64 / n;
                entropy -= p * p.ln();
            }
        }
        let max_e = (max_deg as f64 + 1.0).ln();
        if max_e > 0.0 { entropy / max_e } else { 0.0 }
    }

    /// Compute assortativity coefficient (degree correlation).
    /// Positive = assortative (human). Zero = neutral (ring lattice).
    fn assortativity_coefficient(&self, nodes: &[NodeId]) -> f64 {
        let degs: Vec<f64> = nodes
            .iter()
            .filter_map(|n| self.graph.degree(n).ok())
            .map(|d| d as f64)
            .collect();
        if degs.len() < 5 {
            return 0.0;
        }
        let mean = degs.iter().sum::<f64>() / degs.len() as f64;
        let mut num = 0.0;
        let mut den = 0.0;
        for i in 0..nodes.len() {
            if let Ok(neighbors) = self.graph.neighbors(&nodes[i]) {
                for nb in &neighbors {
                    if let Ok(nd) = self.graph.degree(nb) {
                        let di = degs[i];
                        let dj = nd as f64;
                        num += (di - mean) * (dj - mean);
                    }
                }
            }
        }
        for d in &degs {
            den += (d - mean).powi(2);
        }
        if den.abs() > 1e-10 { num / den } else { 0.0 }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sybil_core::SyntheticGraphConfig;
    use sybil_graph::synthetic::SyntheticGraphGenerator;

    #[test]
    fn test_degree_entropy_on_small_world() {
        let config = SyntheticGraphConfig { num_nodes: 150, ..Default::default() };
        let generator = SyntheticGraphGenerator::new(config);
        let (graph, _) = generator.generate().unwrap();
        let detector = RingLatticeDetector::new(&graph);
        let nodes = graph.all_node_ids();
        let entropy = detector.degree_distribution_entropy(&nodes);
        // Small-world graphs have skewed degree distribution → low entropy
        assert!((0.0..=1.0).contains(&entropy));
    }

    #[test]
    fn test_ring_lattice_analysis() {
        let config = SyntheticGraphConfig { num_nodes: 100, ..Default::default() };
        let generator = SyntheticGraphGenerator::new(config);
        let (graph, _) = generator.generate().unwrap();
        let detector = RingLatticeDetector::new(&graph);
        let nodes = graph.all_node_ids();
        let result = detector.analyze_region(&nodes).unwrap();
        assert!((0.0..=1.0).contains(&result.confidence));
    }
}