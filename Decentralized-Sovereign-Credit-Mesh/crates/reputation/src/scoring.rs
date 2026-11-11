use mesh_graph_core::algorithms;
use mesh_graph_core::graph::SocialGraph;
use mesh_graph_core::types::NodeId;

#[derive(Debug, Clone)]
pub struct ReputationResult {
    pub node_id: NodeId,
    pub pagerank_score: f64,
    pub trust_centrality: f64,
    pub degree_centrality: f64,
    pub clustering_coefficient: f64,
    pub composite_score: f64,
}

pub fn compute_reputation(graph: &SocialGraph, node_id: &str) -> Option<ReputationResult> {
    if !graph.contains_node(node_id) {
        return None;
    }

    let pr_scores = algorithms::compute_pagerank(graph, 0.85, 100, 1e-6);
    let pagerank_score = *pr_scores.get(node_id).unwrap_or(&0.0);

    let trust_centrality =
        algorithms::compute_trust_weighted_centrality(graph, node_id).unwrap_or(0.0);
    let degree_centrality = algorithms::compute_degree_centrality(graph, node_id).unwrap_or(0.0);
    let clustering_coeff =
        algorithms::compute_clustering_coefficient(graph, node_id).unwrap_or(0.0);

    let composite_score = pagerank_score * 0.4
        + trust_centrality * 0.3
        + degree_centrality * 0.2
        + clustering_coeff * 0.1;

    Some(ReputationResult {
        node_id: node_id.to_string(),
        pagerank_score,
        trust_centrality,
        degree_centrality,
        clustering_coefficient: clustering_coeff,
        composite_score,
    })
}

pub fn compute_all_reputations(graph: &SocialGraph) -> Vec<ReputationResult> {
    let mut results: Vec<ReputationResult> = graph
        .all_nodes()
        .iter()
        .filter_map(|n| compute_reputation(graph, &n.id))
        .collect();
    results.sort_by(|a, b| {
        b.composite_score
            .partial_cmp(&a.composite_score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    results
}
