use crate::graph::PoPGraph;
use rand::Rng;
use sybil_core::{
    constants::*, Edge, EdgeMetadata, EdgeType, NodeId, NodeMetadata,
    SybilResult, SyntheticGraphConfig,
};
use tracing::info;

/// Generates synthetic social graphs for testing and validation.
pub struct SyntheticGraphGenerator {
    config: SyntheticGraphConfig,
}

impl SyntheticGraphGenerator {
    pub fn new(config: SyntheticGraphConfig) -> Self {
        Self { config }
    }

    /// Generate a synthetic social graph with optional Sybil clusters.
    pub fn generate(&self) -> SybilResult<(PoPGraph, Vec<(NodeId, bool)>)> {
        let config = &self.config;
        let mut rng = rand::thread_rng();
        let mut graph = PoPGraph::new();
        let mut ground_truth: Vec<(NodeId, bool)> = Vec::new();
        let now = chrono::Utc::now();

        info!("Generating synthetic graph: {} nodes, avg_degree {}, rewiring_prob {}",
            config.num_nodes, config.avg_degree, config.rewiring_probability);

        // 1. Create all honest nodes
        let all_nodes: Vec<NodeId> = (0..config.num_nodes)
            .map(|_| {
                let nid = NodeId::new();
                graph.add_node(NodeMetadata {
                    node_id: nid.clone(),
                    did: None,
                    reputation_score: 0.0,
                    clustering_coefficient: None,
                    centrality_score: None,
                    embedding_depth: 0,
                    joined_at: chrono::Utc::now(),
                    last_active: chrono::Utc::now(),
                    is_sybil: Some(false),
                    metadata: serde_json::Value::Null,
                }).unwrap();
                nid
            })
            .collect();

        // 2. Build Watts-Strogatz ring lattice
        let k = (config.avg_degree / 2.0).round() as usize;
        let k = k.max(1);
        for i in 0..config.num_nodes {
            for j in 1..=k {
                let target = (i + j) % config.num_nodes;
                let source = all_nodes[i].clone();
                let target_id = all_nodes[target].clone();
                let edge = Edge {
                    source, target: target_id, weight: 1.0,
                    metadata: EdgeMetadata { created_at: now, edge_type: EdgeType::Social, is_active: true },
                };
                graph.add_edge(&edge).ok();
            }
        }

        // Rewiring: add extra random edges
        let num_rewire = (config.num_nodes as f64 * config.avg_degree * config.rewiring_probability) as usize;
        for _ in 0..num_rewire {
            let a = all_nodes[rng.gen_range(0..config.num_nodes)].clone();
            let b = all_nodes[rng.gen_range(0..config.num_nodes)].clone();
            if a != b {
                let edge = Edge {
                    source: a, target: b, weight: 1.0,
                    metadata: EdgeMetadata { created_at: now, edge_type: EdgeType::Social, is_active: true },
                };
                graph.add_edge(&edge).ok();
            }
        }

        // 3. Inject Sybil clusters
        for cluster_idx in 0..config.num_sybil_clusters {
            let mut cluster: Vec<NodeId> = Vec::new();
            for _ in 0..config.sybil_cluster_size {
                let nid = NodeId::new();
                graph.add_node(NodeMetadata {
                    node_id: nid.clone(),
                    did: None, reputation_score: 0.0,
                    clustering_coefficient: None, centrality_score: None,
                    embedding_depth: 0,
                    joined_at: chrono::Utc::now(),
                    last_active: chrono::Utc::now(),
                    is_sybil: Some(true),
                    metadata: serde_json::Value::Null,
                }).unwrap();
                cluster.push(nid);
                ground_truth.push((cluster[cluster.len() - 1].clone(), true));
            }

            // Dense internal connections within the Sybil cluster
            for i in 0..cluster.len() {
                for j in i + 1..cluster.len() {
                    if rng.gen_bool(config.sybil_internal_density) {
                        let edge = Edge {
                            source: cluster[i].clone(),
                            target: cluster[j].clone(),
                            weight: 1.0,
                            metadata: EdgeMetadata { created_at: now, edge_type: EdgeType::Social, is_active: true },
                        };
                        graph.add_edge(&edge).ok();
                    }
                }
            }

            // Minimal external connections to the main graph
            let num_external = (config.sybil_cluster_size as f64 * config.sybil_external_connectivity).round() as usize;
            let external = if config.sybil_external_connectivity > 0.0 { num_external.max(1) } else { 0 };
            for i in 0..external.min(cluster.len()) {
                let sybil_node = cluster[i].clone();
                let target = all_nodes[rng.gen_range(0..all_nodes.len())].clone();
                let edge = Edge {
                    source: sybil_node, target, weight: 1.0,
                    metadata: EdgeMetadata { created_at: now, edge_type: EdgeType::Social, is_active: true },
                };
                graph.add_edge(&edge).ok();
            }
            info!("Injected Sybil cluster {}: {} nodes", cluster_idx, config.sybil_cluster_size);
        }

        // Mark all honest ground truth
        for nid in &all_nodes {
            ground_truth.push((nid.clone(), false));
        }

        info!("Generated graph: {} nodes, {} edges", graph.node_count(), graph.edge_count());
        Ok((graph, ground_truth))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_small_world() {
        let config = SyntheticGraphConfig { num_nodes: 100, ..Default::default() };
        let generator = SyntheticGraphGenerator::new(config);
        let (graph, ground_truth) = generator.generate().unwrap();
        assert!(graph.node_count() >= 100);
        assert!(graph.edge_count() > 0);
        assert_eq!(ground_truth.len(), graph.node_count());
    }

    #[test]
    fn test_minimal_config() {
        let config = SyntheticGraphConfig { num_nodes: 10, sybil_cluster_size: 2, num_sybil_clusters: 1, ..Default::default() };
        let generator = SyntheticGraphGenerator::new(config);
        let (graph, _) = generator.generate().unwrap();
        assert_eq!(graph.node_count(), 12);
    }
}