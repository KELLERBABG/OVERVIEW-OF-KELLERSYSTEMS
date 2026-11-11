use std::collections::HashMap;

use mesh_graph_core::graph::SocialGraph;
use mesh_graph_core::types::NodeId;
use serde::{Deserialize, Serialize};

use crate::scoring::{compute_reputation, ReputationResult};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepaymentRecord {
    pub from: NodeId,
    pub to: NodeId,
    pub amount: f64,
    pub timestamp: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaintTracker {
    repayments: Vec<RepaymentRecord>,
    flow_scores: HashMap<NodeId, f64>,
}

impl TaintTracker {
    pub fn new() -> Self {
        TaintTracker {
            repayments: Vec::new(),
            flow_scores: HashMap::new(),
        }
    }

    pub fn record_repayment(&mut self, from: &str, to: &str, amount: f64) {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        self.repayments.push(RepaymentRecord {
            from: from.to_string(),
            to: to.to_string(),
            amount,
            timestamp,
        });

        self.recompute_flow_scores();
    }

    fn recompute_flow_scores(&mut self) {
        let mut total_inflow: HashMap<NodeId, f64> = HashMap::new();
        let mut total_outflow: HashMap<NodeId, f64> = HashMap::new();

        for record in &self.repayments {
            *total_outflow.entry(record.from.clone()).or_insert(0.0) += record.amount;
            *total_inflow.entry(record.to.clone()).or_insert(0.0) += record.amount;
        }

        let all_nodes: Vec<NodeId> = self
            .repayments
            .iter()
            .flat_map(|r| vec![r.from.clone(), r.to.clone()])
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();

        for node in &all_nodes {
            let inflow = total_inflow.get(node).copied().unwrap_or(0.0);
            let outflow = total_outflow.get(node).copied().unwrap_or(0.0);
            let total = inflow + outflow;

            let score = if total > 0.0 {
                (inflow - outflow).abs() / total.max(1.0) * (total / (total + 100.0))
            } else {
                0.0
            };

            self.flow_scores.insert(node.clone(), score);
        }
    }

    pub fn flow_score(&self, node_id: &str) -> f64 {
        self.flow_scores.get(node_id).copied().unwrap_or(0.0)
    }

    pub fn repayment_count(&self) -> usize {
        self.repayments.len()
    }

    pub fn adjusted_reputation(
        &self,
        graph: &SocialGraph,
        node_id: &str,
    ) -> Option<ReputationResult> {
        let base = compute_reputation(graph, node_id)?;
        let taint = self.flow_score(node_id);

        let reputation_boost = taint * 0.2;

        Some(ReputationResult {
            composite_score: base.composite_score + reputation_boost,
            ..base
        })
    }

    pub fn net_flow(&self, node_id: &str) -> f64 {
        let mut inflow = 0.0;
        let mut outflow = 0.0;
        for record in &self.repayments {
            if record.from == node_id {
                outflow += record.amount;
            }
            if record.to == node_id {
                inflow += record.amount;
            }
        }
        inflow - outflow
    }
}

impl Default for TaintTracker {
    fn default() -> Self {
        Self::new()
    }
}
