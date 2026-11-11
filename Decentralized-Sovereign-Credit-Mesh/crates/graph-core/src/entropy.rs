use std::collections::{HashMap, HashSet, VecDeque};

use crate::graph::SocialGraph;
use crate::types::NodeId;

pub fn graph_entropy(graph: &SocialGraph) -> f64 {
    let nodes = graph.all_nodes();
    let n = nodes.len();
    if n == 0 {
        return 0.0;
    }

    let mut degree_counts: HashMap<usize, usize> = HashMap::new();
    for node in &nodes {
        let deg = graph.neighbors(&node.id).len() + graph.predecessors(&node.id).len();
        *degree_counts.entry(deg).or_insert(0) += 1;
    }

    let mut entropy = 0.0;
    for &count in degree_counts.values() {
        let p = count as f64 / n as f64;
        if p > 0.0 {
            entropy -= p * p.log2();
        }
    }
    entropy
}

pub fn local_entropy(graph: &SocialGraph, node_id: &str, radius: usize) -> f64 {
    let mut seen = HashSet::new();
    let mut queue = VecDeque::new();
    let mut degree_counts: HashMap<usize, usize> = HashMap::new();

    if !graph.contains_node(node_id) {
        return 0.0;
    }

    seen.insert(node_id.to_string());
    queue.push_back((node_id.to_string(), 0));

    while let Some((current, depth)) = queue.pop_front() {
        if depth > radius {
            continue;
        }
        let deg = graph.neighbors(&current).len() + graph.predecessors(&current).len();
        *degree_counts.entry(deg).or_insert(0) += 1;

        for (neighbor, _) in graph.neighbors(&current) {
            if seen.insert(neighbor.clone()) {
                queue.push_back((neighbor, depth + 1));
            }
        }
        for (predecessor, _) in graph.predecessors(&current) {
            if seen.insert(predecessor.clone()) {
                queue.push_back((predecessor, depth + 1));
            }
        }
    }

    let total: usize = degree_counts.values().sum();
    if total == 0 {
        return 0.0;
    }

    let mut entropy = 0.0;
    for &count in degree_counts.values() {
        let p = count as f64 / total as f64;
        if p > 0.0 {
            entropy -= p * p.log2();
        }
    }
    entropy
}

pub fn sybil_score(graph: &SocialGraph, node_id: &str) -> Option<f64> {
    if !graph.contains_node(node_id) {
        return None;
    }

    let global_entropy = graph_entropy(graph);
    let local_ent = local_entropy(graph, node_id, 2);

    if global_entropy == 0.0 {
        return Some(0.0);
    }

    let entropy_ratio = local_ent / global_entropy;

    let out_edges = graph.neighbors(node_id).len() as f64;
    let in_edges = graph.predecessors(node_id).len() as f64;
    let total_edges = out_edges + in_edges;
    let reciprocity = if total_edges > 0.0 {
        1.0 - (out_edges - in_edges).abs() / total_edges
    } else {
        1.0
    };

    let mut cluster_density = 0.0;
    let neighbors: Vec<NodeId> = graph
        .neighbors(node_id)
        .into_iter()
        .map(|(id, _)| id)
        .collect();
    if neighbors.len() >= 2 {
        let mut edges_between = 0;
        let possible = neighbors.len() * (neighbors.len() - 1) / 2;
        for i in 0..neighbors.len() {
            for j in (i + 1)..neighbors.len() {
                if graph
                    .shortest_trust_path(&neighbors[i], &neighbors[j])
                    .is_some()
                {
                    edges_between += 1;
                }
            }
        }
        cluster_density = edges_between as f64 / possible as f64;
    }

    let score = entropy_ratio * 0.4 + (1.0 - reciprocity) * 0.3 + (1.0 - cluster_density) * 0.3;
    Some(score.clamp(0.0, 1.0))
}

pub fn detect_sybil_clusters(graph: &SocialGraph, threshold: f64) -> Vec<Vec<NodeId>> {
    let nodes = graph.all_nodes();
    let mut visited: HashSet<NodeId> = HashSet::new();
    let mut clusters: Vec<Vec<NodeId>> = Vec::new();

    for node in &nodes {
        if visited.contains(&node.id) {
            continue;
        }
        let score = sybil_score(graph, &node.id).unwrap_or(0.0);
        if score > threshold {
            let mut cluster = Vec::new();
            let mut queue = VecDeque::new();
            visited.insert(node.id.clone());
            queue.push_back(node.id.clone());

            while let Some(current) = queue.pop_front() {
                let current_score = sybil_score(graph, &current).unwrap_or(0.0);
                if current_score > threshold {
                    cluster.push(current.clone());
                    for (neighbor, _) in graph.neighbors(&current) {
                        if visited.insert(neighbor.clone()) {
                            queue.push_back(neighbor);
                        }
                    }
                    for (predecessor, _) in graph.predecessors(&current) {
                        if visited.insert(predecessor.clone()) {
                            queue.push_back(predecessor);
                        }
                    }
                }
            }

            if cluster.len() > 1 {
                clusters.push(cluster);
            }
        }
    }
    clusters
}
