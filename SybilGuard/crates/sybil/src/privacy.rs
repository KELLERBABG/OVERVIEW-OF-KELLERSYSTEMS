//! # Differential Privacy for Graph Queries
//!
//! During Phase 1 (before zk-SNARKs), the raw social graph must be
//! visible for Sybil detection. This module reduces de-anonymization
//! risk by applying differential privacy to graph API responses.
//!
//! ## Strategies
//!
//! 1. **Node ID randomization:** Returned Node IDs are replaced with
//!    ephemeral pseudonyms for non-anchor queries.
//! 2. **Degree obfuscation:** Exact degrees are replaced with
//!    (degree ± Laplace(ε)) noise for non-auth queries.
//! 3. **Edge list filtering:** External API calls return only
//!    aggregated statistics, not raw edge lists.
//! 4. **Query rate limiting:** Per-IP/Per-DID rate limits on
//!    topology queries prevent reconstruction attacks.
//! 5. **k-anonymity threshold:** Graph statistics are only returned
//!    for components with ≥ k nodes (default: 10).

use std::collections::HashMap;
use sybil_core::NodeId;

/// Privacy budget — tracks epsilon expenditure per node.
#[derive(Debug, Clone)]
pub struct PrivacyBudget {
    total_epsilon: f64,
    epsilon_spent: f64,
    query_count: u64,
}

impl PrivacyBudget {
    pub fn new(total_epsilon: f64) -> Self {
        Self { total_epsilon, epsilon_spent: 0.0, query_count: 0 }
    }
    pub fn can_afford(&self, query_epsilon: f64) -> bool {
        self.epsilon_spent + query_epsilon <= self.total_epsilon
    }
    pub fn spend(&mut self, query_epsilon: f64) -> bool {
        if self.can_afford(query_epsilon) {
            self.epsilon_spent += query_epsilon;
            self.query_count += 1;
            true
        } else {
            false
        }
    }
    pub fn remaining_fraction(&self) -> f64 {
        (self.total_epsilon - self.epsilon_spent) / self.total_epsilon
    }
}

impl Default for PrivacyBudget {
    fn default() -> Self { Self::new(1.0) }
}

/// Obfuscated degree (degree ± Laplace noise).
#[derive(Debug, Clone)]
pub struct ObfuscatedDegree {
    pub approximate_degree: f64,
    pub epsilon_consumed: f64,
}

/// Specialized degrees that can bypass privacy for anchors.
#[derive(Debug, Clone)]
pub enum QueryPrivilege {
    Anchor,
    Protected(PrivacyBudget),
}

/// Handles differential privacy for graph data queries.
pub struct PrivacyLayer {
    budgets: HashMap<NodeId, PrivacyBudget>,
    k_anonymity_threshold: usize,
}

impl PrivacyLayer {
    pub fn new(k_anonymity_threshold: usize) -> Self {
        Self { budgets: HashMap::new(), k_anonymity_threshold }
    }
    pub fn get_budget(&mut self, node: &NodeId) -> &mut PrivacyBudget {
        self.budgets.entry(node.clone()).or_default()
    }
    pub fn obfuscate_degree(&mut self, node: &NodeId, true_degree: usize) -> ObfuscatedDegree {
        let budget = self.get_budget(node);
        let epsilon = 0.1_f64.min(budget.total_epsilon * 0.1);
        if budget.spend(epsilon) {
            let noise = laplace_noise(1.0 / epsilon);
            ObfuscatedDegree {
                approximate_degree: (true_degree as f64 + noise).max(0.0),
                epsilon_consumed: epsilon,
            }
        } else {
            ObfuscatedDegree { approximate_degree: true_degree as f64, epsilon_consumed: 0.0 }
        }
    }
    pub fn passes_k_anonymity(&self, component_size: usize) -> bool {
        component_size >= self.k_anonymity_threshold
    }
    pub fn k_threshold(&self) -> usize { self.k_anonymity_threshold }
    pub fn reset_epoch(&mut self) { self.budgets.clear(); }
}

impl Default for PrivacyLayer {
    fn default() -> Self { Self::new(10) }
}

/// Sample from Laplace(0, scale) for differential privacy noise.
/// Uses the standard method: U ~ Uniform(0,1), return -scale * sign(U-0.5) * ln(1 - 2|U-0.5|).
/// Numerically stable: clips |U - 0.5| away from 0 to avoid ln(0) = -inf.
pub fn laplace_noise(scale: f64) -> f64 {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    // Uniform(0,1) with clamping to avoid ln(0)
    let u = rng.gen::<f64>().clamp(1e-15, 1.0 - 1e-15);
    let diff = u - 0.5;
    if diff.abs() < 1e-15 {
        return 0.0; // degenerate case, essentially no noise
    }
    let sign = if diff > 0.0 { -1.0 } else { 1.0 };
    let abs = (2.0 * diff.abs()).clamp(1e-15, 1.0 - 1e-15);
    sign * scale * abs.ln()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_privacy_budget() {
        let mut budget = PrivacyBudget::new(1.0);
        assert!(budget.can_afford(0.1));
        assert!(budget.spend(0.1));
        assert!(!budget.can_afford(10.0));
    }

    #[test]
    fn test_laplace_noise_finite() {
        for _ in 0..100 {
            let noise = laplace_noise(1.0);
            assert!(noise.is_finite());
        }
    }

    #[test]
    fn test_obfuscation() {
        let mut layer = PrivacyLayer::default();
        let node = NodeId::new();
        let obf = layer.obfuscate_degree(&node, 42);
        assert!(obf.approximate_degree >= 0.0);
        assert!(obf.epsilon_consumed > 0.0);
    }
}