use petgraph::visit::NodeIndexable;
use sybil_core::{NodeId, SybilResult};
use std::collections::{HashMap, VecDeque};

use crate::graph::PoPGraph;

/// Community detection algorithms for the social graph.
pub struct CommunityDetector<'a> {
    graph: &'a PoPGraph,
}

impl<'a> CommunityDetector<'a> {
    pub fn new(graph: &'a PoPGraph) -> Self {
        Self { graph }
    }

    /// Label Propagation community detection.
    pub fn label_propagation(&self, max_iterations: usize) -> SybilResult<Vec<Vec<NodeId>>> {
        let g = self.graph.inner();
        let n = g.node_count();
        if n == 0 {
            return Ok(Vec::new());
        }
        let mut labels: Vec<usize> = (0..n).collect();
        let node_ids: Vec<NodeId> = (0..n)
            .map(|i| {
                let ni = g.from_index(i);
                g[ni].node_id.clone()
            })
            .collect();
        for _iteration in 0..max_iterations {
            let mut changed = false;
            let mut order: Vec<usize> = (0..n).collect();
            {
                use rand::seq::SliceRandom;
                let mut rng = rand::thread_rng();
                order.shuffle(&mut rng);
            }
            for &i in &order {
                let ni = g.from_index(i);
                let neighbor_labels: Vec<usize> = g.neighbors(ni).map(|nb| labels[g.to_index(nb)]).collect();
                if neighbor_labels.is_empty() {
                    continue;
                }
                let mut freq: HashMap<usize, usize> = HashMap::new();
                for &label in &neighbor_labels {
                    *freq.entry(label).or_insert(0) += 1;
                }
                let best_label = freq
                    .into_iter()
                    .max_by_key(|&(label, count)| (count, std::cmp::Reverse(label)))
                    .map(|(label, _)| label)
                    .unwrap();
                if labels[i] != best_label {
                    labels[i] = best_label;
                    changed = true;
                }
            }
            if !changed {
                break;
            }
        }
        let mut communities: HashMap<usize, Vec<NodeId>> = HashMap::new();
        for (i, &label) in labels.iter().enumerate() {
            communities.entry(label).or_default().push(node_ids[i].clone());
        }
        Ok(communities.into_values().collect())
    }

    /// BFS-based connected components detection.
    pub fn connected_components(&self) -> SybilResult<Vec<Vec<NodeId>>> {
        let g = self.graph.inner();
        let n = g.node_count();
        if n == 0 {
            return Ok(Vec::new());
        }
        let node_ids: Vec<NodeId> = (0..n)
            .map(|i| {
                let ni = g.from_index(i);
                g[ni].node_id.clone()
            })
            .collect();
        let mut visited = vec![false; n];
        let mut components = Vec::new();
        for start in 0..n {
            if visited[start] {
                continue;
            }
            let mut component = Vec::new();
            let mut queue = VecDeque::new();
            queue.push_back(start);
            visited[start] = true;
            while let Some(idx) = queue.pop_front() {
                component.push(node_ids[idx].clone());
                let ni = g.from_index(idx);
                for neighbor in g.neighbors(ni) {
                    let nb_idx = g.to_index(neighbor);
                    if !visited[nb_idx] {
                        visited[nb_idx] = true;
                        queue.push_back(nb_idx);
                    }
                }
            }
            components.push(component);
        }
        Ok(components)
    }

    /// Get all nodes that belong to components smaller than a threshold.
    pub fn small_components(&self, max_size: usize) -> SybilResult<Vec<Vec<NodeId>>> {
        let components = self.connected_components()?;
        Ok(components.into_iter().filter(|c| c.len() <= max_size).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::synthetic::SyntheticGraphGenerator;
    use sybil_core::SyntheticGraphConfig;

    #[test]
    fn test_connected_components() {
        let config = SyntheticGraphConfig { num_nodes: 100, ..Default::default() };
        let generator = SyntheticGraphGenerator::new(config);
        let (graph, _) = generator.generate().unwrap();
        let detector = CommunityDetector::new(&graph);
        let components = detector.connected_components().unwrap();
        let total: usize = components.iter().map(|c| c.len()).sum();
        assert_eq!(total, graph.node_count());
    }

    #[test]
    fn test_label_propagation() {
        let config = SyntheticGraphConfig { num_nodes: 100, ..Default::default() };
        let generator = SyntheticGraphGenerator::new(config);
        let (graph, _) = generator.generate().unwrap();
        let detector = CommunityDetector::new(&graph);
        let communities = detector.label_propagation(100).unwrap();
        let total: usize = communities.iter().map(|c| c.len()).sum();
        assert_eq!(total, graph.node_count());
        assert!(!communities.is_empty());
    }

    #[test]
    fn test_small_components() {
        let config = SyntheticGraphConfig {
            num_nodes: 100, sybil_cluster_size: 5, num_sybil_clusters: 2,
            sybil_external_connectivity: 0.0, // force isolated clusters
            ..Default::default()
        };
        let generator = SyntheticGraphGenerator::new(config);
        let (graph, _) = generator.generate().unwrap();
        let detector = CommunityDetector::new(&graph);
        let small = detector.small_components(10).unwrap();
        assert!(!small.is_empty(), "should find sybil clusters as isolated components");
    }
}