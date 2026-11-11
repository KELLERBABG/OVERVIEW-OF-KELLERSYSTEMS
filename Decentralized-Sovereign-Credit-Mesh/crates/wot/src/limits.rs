use std::collections::HashMap;

use mesh_graph_core::graph::SocialGraph;
use mesh_graph_core::types::NodeId;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VouchingLimits {
    pub max_depth: usize,
    pub max_breadth: usize,
    pub min_weight: f64,
    pub decay_factor: f64,
    pub max_chain_trust: f64,
}

impl Default for VouchingLimits {
    fn default() -> Self {
        VouchingLimits {
            max_depth: 5,
            max_breadth: 50,
            min_weight: 0.1,
            decay_factor: 0.8,
            max_chain_trust: 0.95,
        }
    }
}

impl VouchingLimits {
    pub fn effective_weight(&self, base_weight: f64, depth: usize) -> f64 {
        if depth as f64 > self.max_depth as f64 {
            return 0.0;
        }
        let decayed = base_weight * self.decay_factor.powi(depth as i32);
        decayed.min(self.max_chain_trust).max(0.0)
    }

    pub fn can_vouch(&self, graph: &SocialGraph, voucher_id: &str) -> Result<(), String> {
        let out_degree = graph.neighbors(voucher_id).len();
        if out_degree >= self.max_breadth {
            return Err(format!(
                "voucher '{}' has reached maximum breadth of {} vouches",
                voucher_id, self.max_breadth
            ));
        }
        Ok(())
    }

    pub fn can_propagate(&self, current_weight: f64, depth: usize) -> bool {
        self.effective_weight(current_weight, depth) >= self.min_weight && depth <= self.max_depth
    }

    pub fn propagate_trust(
        &self,
        graph: &SocialGraph,
        from_id: &str,
    ) -> HashMap<NodeId, (f64, Vec<NodeId>)> {
        use std::collections::VecDeque;

        let mut results: HashMap<NodeId, (f64, Vec<NodeId>)> = HashMap::new();
        let mut queue = VecDeque::new();

        results.insert(from_id.to_string(), (1.0, vec![from_id.to_string()]));
        queue.push_back((from_id.to_string(), 0usize));

        while let Some((current, depth)) = queue.pop_front() {
            if depth >= self.max_depth {
                continue;
            }

            let (current_trust, current_path) = results.get(&current).cloned().unwrap_or_default();

            for (target, edge) in graph.neighbors(&current) {
                let new_trust = current_trust * self.effective_weight(edge.trust_weight, depth + 1);
                if new_trust < self.min_weight {
                    continue;
                }

                let mut new_path = current_path.clone();
                new_path.push(target.clone());

                let is_better = match results.get(&target) {
                    Some((existing_trust, _)) => new_trust > *existing_trust,
                    None => true,
                };

                if is_better {
                    results.insert(target.clone(), (new_trust, new_path));
                    queue.push_back((target, depth + 1));
                }
            }
        }

        results.remove(from_id);
        results
    }
}
