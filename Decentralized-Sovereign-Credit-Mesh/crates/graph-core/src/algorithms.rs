use std::collections::HashMap;

use crate::graph::SocialGraph;
use crate::types::{NodeId, TrustWeight};

pub fn compute_pagerank(
    graph: &SocialGraph,
    damping_factor: f64,
    max_iterations: usize,
    tolerance: f64,
) -> HashMap<NodeId, f64> {
    let nodes = graph.all_nodes();
    let n = nodes.len();
    if n == 0 {
        return HashMap::new();
    }

    let init_score = 1.0 / n as f64;
    let mut scores: HashMap<NodeId, f64> = HashMap::new();
    for node in &nodes {
        scores.insert(node.id.clone(), init_score);
    }

    for _iteration in 0..max_iterations {
        let mut new_scores: HashMap<NodeId, f64> = HashMap::new();
        let mut max_diff: f64 = 0.0;

        let dangling_sum: f64 = nodes
            .iter()
            .filter(|n| graph.neighbors(&n.id).is_empty())
            .map(|n| scores[&n.id])
            .sum();

        for node in &nodes {
            let mut incoming_sum = 0.0;
            for (pred_id, _edge) in graph.predecessors(&node.id) {
                let pred_out_degree = graph.neighbors(&pred_id).len();
                if pred_out_degree > 0 {
                    incoming_sum += scores[&pred_id] / pred_out_degree as f64;
                }
            }

            let new_score = (1.0 - damping_factor) / n as f64
                + damping_factor * (incoming_sum + dangling_sum / n as f64);

            new_scores.insert(node.id.clone(), new_score);
            max_diff = max_diff.max((new_score - scores[&node.id]).abs());
        }

        scores = new_scores;

        if max_diff < tolerance {
            break;
        }
    }

    scores
}

pub fn compute_clustering_coefficient(graph: &SocialGraph, node_id: &str) -> Option<f64> {
    let neighbors: Vec<NodeId> = graph
        .neighbors(node_id)
        .into_iter()
        .map(|(id, _)| id)
        .collect();

    let k = neighbors.len();
    if k < 2 {
        return Some(0.0);
    }

    let mut triangles = 0;
    for i in 0..k {
        for j in (i + 1)..k {
            if let Some(_path) = graph.shortest_trust_path(&neighbors[i], &neighbors[j]) {
                triangles += 1;
            }
        }
    }

    let possible = k * (k - 1) / 2;
    Some(triangles as f64 / possible as f64)
}

pub fn average_clustering(graph: &SocialGraph) -> f64 {
    let nodes = graph.all_nodes();
    if nodes.is_empty() {
        return 0.0;
    }
    let sum: f64 = nodes
        .iter()
        .filter_map(|n| compute_clustering_coefficient(graph, &n.id))
        .sum();
    sum / nodes.len() as f64
}

pub fn compute_degree_centrality(graph: &SocialGraph, node_id: &str) -> Option<f64> {
    let n = graph.node_count();
    if n <= 1 {
        return Some(0.0);
    }
    let degree = graph.neighbors(node_id).len() + graph.predecessors(node_id).len();
    Some(degree as f64 / (n - 1) as f64)
}

pub fn compute_trust_weighted_centrality(graph: &SocialGraph, node_id: &str) -> Option<f64> {
    let neighbors = graph.neighbors(node_id);
    if neighbors.is_empty() {
        return Some(0.0);
    }
    let total_weight: TrustWeight = neighbors.iter().map(|(_, e)| e.trust_weight).sum();
    Some(total_weight / neighbors.len() as f64)
}
