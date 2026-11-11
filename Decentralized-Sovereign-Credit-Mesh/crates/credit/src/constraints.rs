use mesh_graph_core::algorithms;
use mesh_graph_core::graph::SocialGraph;
use mesh_graph_core::types::NodeId;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Constraint {
    TrustPathExists {
        from: NodeId,
        to: NodeId,
        max_depth: usize,
    },
    MinReputation {
        user: NodeId,
        min_score: f64,
    },
    MaxExposure {
        user: NodeId,
        max_amount: f64,
    },
    VouchingLimit {
        user: NodeId,
        max_propagation_depth: usize,
    },
    SybilScoreBelow {
        user: NodeId,
        max_score: f64,
    },
    And(Vec<Constraint>),
    Or(Vec<Constraint>),
    Not(Box<Constraint>),
}

impl Constraint {
    pub fn evaluate(&self, graph: &SocialGraph, exposures: &[Exposure]) -> Result<bool, String> {
        match self {
            Constraint::TrustPathExists {
                from,
                to,
                max_depth,
            } => Ok(graph
                .shortest_trust_path(from, to)
                .is_some_and(|p| p.len() - 1 <= *max_depth)),
            Constraint::MinReputation { user, min_score } => {
                let pr = algorithms::compute_pagerank(graph, 0.85, 100, 1e-6);
                let score = pr.get(user).copied().unwrap_or(0.0);
                Ok(score >= *min_score)
            }
            Constraint::MaxExposure { user, max_amount } => {
                let total: f64 = exposures
                    .iter()
                    .filter(|e| e.borrower == *user && e.active)
                    .map(|e| e.outstanding)
                    .sum();
                Ok(total <= *max_amount)
            }
            Constraint::VouchingLimit {
                user,
                max_propagation_depth,
            } => {
                let distances = algorithms::compute_pagerank(graph, 0.85, 100, 1e-6);
                let depth = distances
                    .get(user)
                    .map(|s| (s * 10.0) as usize)
                    .unwrap_or(0);
                Ok(depth <= *max_propagation_depth)
            }
            Constraint::SybilScoreBelow { user, max_score } => {
                let score = mesh_graph_core::entropy::sybil_score(graph, user).unwrap_or(0.0);
                Ok(score <= *max_score)
            }
            Constraint::And(cons) => {
                for c in cons {
                    if !c.evaluate(graph, exposures)? {
                        return Ok(false);
                    }
                }
                Ok(true)
            }
            Constraint::Or(cons) => {
                for c in cons {
                    if c.evaluate(graph, exposures)? {
                        return Ok(true);
                    }
                }
                Ok(false)
            }
            Constraint::Not(c) => Ok(!c.evaluate(graph, exposures)?),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Exposure {
    pub borrower: NodeId,
    pub outstanding: f64,
    pub active: bool,
}

pub struct ConstraintEngine;

impl ConstraintEngine {
    pub fn generate_credit_constraints(
        _graph: &SocialGraph,
        borrower: &str,
        request_amount: f64,
    ) -> Vec<Constraint> {
        vec![
            Constraint::MinReputation {
                user: borrower.to_string(),
                min_score: 0.01,
            },
            Constraint::MaxExposure {
                user: borrower.to_string(),
                max_amount: request_amount * 3.0,
            },
            Constraint::SybilScoreBelow {
                user: borrower.to_string(),
                max_score: 0.9,
            },
        ]
    }

    pub fn check_borrower(
        graph: &SocialGraph,
        borrower: &str,
        request_amount: f64,
        exposures: &[Exposure],
    ) -> Result<(bool, Vec<Constraint>), String> {
        let constraints = Self::generate_credit_constraints(graph, borrower, request_amount);

        for constraint in &constraints {
            let passed = constraint.evaluate(graph, exposures)?;
            if !passed {
                return Ok((false, constraints));
            }
        }

        Ok((true, constraints))
    }
}
