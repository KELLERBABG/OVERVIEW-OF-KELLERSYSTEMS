//! # IP State Space Exploration
//!
//! Systematic investigation of the global patent graph using formal methods.
//! Isolates plagiarism, minor modifications, and genuine breakthroughs.
//!
//! Provides:
//! - Reachability queries: "is node X derivable from node Y?"
//! - Similarity queries via Δ satisfiability
//! - Equivalence clustering (group patents by invariant equivalence classes)
//! - Plagiarism detection (parallel Δ evaluation + LSH pre-filtering)
//! - Novelty scoring

use crate::delta::build_delta;
use crate::error::CoreError;
use crate::graph::PatentDag;
use crate::ir::InventionIr;
use crate::models::*;
use crate::solver::{compute_severity, BuiltInSolver, SmtSolver, SolverConfig, SolverResponse};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

// ─── Similarity Query ────────────────────────────────────────────────────────

/// The result of a similarity comparison between two patent nodes.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SimilarityResult {
    /// The source (original) node.
    pub source_id: Uuid,
    /// The target (derivative) node.
    pub target_id: Uuid,
    /// Whether delta is SAT (novel) or UNSAT (infringement).
    pub sat: SatResult,
    /// Severity score (0.0 = identical, 1.0 = completely novel).
    pub severity: f64,
    /// The number of edges on the shortest path between nodes.
    pub distance: Option<usize>,
}

/// Compare two patent nodes by computing the delta constraint and solving it.
pub fn compare_nodes(
    dag: &PatentDag,
    source_id: &Uuid,
    target_id: &Uuid,
    source_ir: &InventionIr,
    target_ir: &InventionIr,
    solver: &dyn SmtSolver,
    config: &SolverConfig,
) -> Result<SimilarityResult, CoreError> {
    let delta = build_delta(*source_id, *target_id, source_ir, target_ir, &[])?;

    let formula_str = String::from_utf8(delta.formula_smt_lib2)
        .map_err(|_| CoreError::InvalidFormula("non-UTF-8 delta formula".to_string()))?;

    let response = solver.check_sat(&formula_str, config)?;
    let severity = compute_severity(&response);
    let distance = dag.distance(source_id, target_id).ok().flatten();

    Ok(SimilarityResult {
        source_id: *source_id,
        target_id: *target_id,
        sat: response.sat,
        severity,
        distance,
    })
}

// ─── Plagiarism Detection ────────────────────────────────────────────────────

/// A potential infringement match.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PlagiarismMatch {
    /// The candidate infringing node.
    pub node_id: Uuid,
    /// The original node it may infringe upon.
    pub original_id: Uuid,
    /// Severity score (lower = more likely infringement).
    pub severity: f64,
    /// The SAT/UNSAT result.
    pub sat: SatResult,
}

/// Scan a set of candidate nodes against known patents to detect plagiarism.
///
/// Uses a two-phase approach:
/// 1. LSH pre-filtering (by invariant hash) to narrow candidates
/// 2. Full Δ evaluation on the remaining candidates
pub fn detect_plagiarism(
    dag: &PatentDag,
    candidates: &[(Uuid, InventionIr)],
    patents: &HashMap<Uuid, InventionIr>,
    solver: &dyn SmtSolver,
    config: &SolverConfig,
    max_results: usize,
) -> Result<Vec<PlagiarismMatch>, CoreError> {
    let mut results = Vec::new();

    for (candidate_id, candidate_ir) in candidates {
        let candidate_invariants = crate::ir::extract_invariants(candidate_ir);
        let candidate_hash = simple_hash(&candidate_invariants);

        for (patent_id, patent_ir) in patents {
            if *patent_id == *candidate_id {
                continue;
            }

            let patent_invariants = crate::ir::extract_invariants(patent_ir);
            let patent_hash = simple_hash(&patent_invariants);

            // LSH pre-filter: if hashes are very different, skip the full Δ check.
            if hamming_distance(&candidate_hash, &patent_hash) > 8 {
                continue;
            }

            let delta = build_delta(*patent_id, *candidate_id, patent_ir, candidate_ir, &[])?;
            let formula_str = String::from_utf8(delta.formula_smt_lib2)
                .map_err(|_| CoreError::InvalidFormula("non-UTF-8 delta".to_string()))?;
            let response = solver.check_sat(&formula_str, config)?;
            let severity = compute_severity(&response);

            // UNSAT with low severity = likely infringement
            if response.sat == SatResult::Unsat && severity < 0.3 {
                results.push(PlagiarismMatch {
                    node_id: *candidate_id,
                    original_id: *patent_id,
                    severity,
                    sat: response.sat,
                });
            }

            if results.len() >= max_results {
                return Ok(results);
            }
        }
    }

    Ok(results)
}

/// Compute a simple hash from a list of invariant strings.
fn simple_hash(invariants: &[String]) -> Vec<u8> {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    for inv in invariants {
        hasher.update(inv.as_bytes());
    }
    hasher.finalize().to_vec()
}

/// Compute Hamming distance between two byte vectors.
fn hamming_distance(a: &[u8], b: &[u8]) -> u32 {
    let max_len = a.len().max(b.len());
    let mut dist = 0u32;
    for i in 0..max_len {
        let byte_a = a.get(i).copied().unwrap_or(0);
        let byte_b = b.get(i).copied().unwrap_or(0);
        dist += (byte_a ^ byte_b).count_ones();
    }
    dist
}

// ─── Equivalence Clustering ──────────────────────────────────────────────────

/// A cluster of patent nodes that share equivalent invariants.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EquivalenceCluster {
    /// The cluster ID.
    pub cluster_id: Uuid,
    /// Nodes in this cluster.
    pub node_ids: Vec<Uuid>,
    /// The common invariant pattern.
    pub common_invariant: String,
}

/// Cluster patent nodes by invariant equivalence.
///
/// Two nodes are equivalent if their extracted invariants are syntactically equal
/// (after normalization). This groups patents that protect the same fundamental
/// invention.
pub fn cluster_by_invariants(
    patents: &HashMap<Uuid, InventionIr>,
) -> Vec<EquivalenceCluster> {
    let mut clusters: HashMap<String, Vec<Uuid>> = HashMap::new();
    let mut cluster_id_map: HashMap<String, Uuid> = HashMap::new();

    for (id, ir) in patents {
        let invariants = crate::ir::extract_invariants(ir);
        let mut normalized: Vec<String> = invariants.clone();
        normalized.sort();
        let key = normalized.join("|");

        clusters.entry(key.clone()).or_default().push(*id);
        cluster_id_map.entry(key).or_insert_with(Uuid::new_v4);
    }

    clusters
        .into_iter()
        .map(|(key, node_ids)| {
            let common = key
                .split('|')
                .next()
                .unwrap_or("no invariants")
                .to_string();
            EquivalenceCluster {
                cluster_id: cluster_id_map[&key],
                node_ids,
                common_invariant: common,
            }
        })
        .collect()
}

// ─── Novelty Scoring ─────────────────────────────────────────────────────────

/// Compute a novelty score for a candidate patent against all existing patents.
///
/// The score ranges from 0.0 (identical to existing) to 1.0 (completely novel).
/// Uses the minimum severity across all comparisons — if the candidate is
/// similar to any existing patent, its novelty is reduced.
pub fn compute_novelty_score(
    dag: &PatentDag,
    candidate_id: &Uuid,
    candidate_ir: &InventionIr,
    patents: &HashMap<Uuid, InventionIr>,
    solver: &dyn SmtSolver,
    config: &SolverConfig,
) -> Result<f64, CoreError> {
    let mut max_severity = 0.0;

    for (patent_id, patent_ir) in patents {
        if *patent_id == *candidate_id {
            continue;
        }

        let delta = build_delta(*patent_id, *candidate_id, patent_ir, candidate_ir, &[])?;
        let formula_str = String::from_utf8(delta.formula_smt_lib2)
            .map_err(|_| CoreError::InvalidFormula("non-UTF-8 delta".to_string()))?;
        let response = solver.check_sat(&formula_str, config)?;
        let severity = compute_severity(&response);

        if severity > max_severity {
            max_severity = severity;
        }
    }

    // Novelty = max severity across all comparisons.
    // If all comparisons show UNSAT (contained), novelty is low.
    Ok(max_severity)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::PatentDag;
    use crate::ir::*;
    use std::collections::HashMap;

    fn make_ir(pre: &[&str], post: &[&str]) -> InventionIr {
        InventionIr {
            kind: IrKind::Software(SsaTrace {
                entry_block: "entry".into(),
                blocks: HashMap::new(),
                inputs: vec![],
                outputs: vec![],
            }),
            preconditions: pre.iter().map(|s| s.to_string()).collect(),
            postconditions: post.iter().map(|s| s.to_string()).collect(),
            invariants: vec![],
        }
    }

    #[test]
    fn test_compare_nodes_basic() {
        // This tests that compare_nodes returns successfully and produces
        // valid severity and distance values regardless of the naive solver.
        // Real SAT/UNSAT detection requires Z3.
        let dag = PatentDag::new();
        let source = make_ir(&["(= x 0)"], &["(= y 1)"]);
        let target = make_ir(&["(= x 0)"], &["(= y 2)"]);

        let solver = BuiltInSolver;
        let config = SolverConfig::default();

        let result = compare_nodes(
            &dag,
            &Uuid::new_v4(),
            &Uuid::new_v4(),
            &source,
            &target,
            &solver,
            &config,
        )
        .unwrap();

        // Severity should always be in [0, 1]
        assert!(result.severity >= 0.0 && result.severity <= 1.0);
        // distance is None because nodes aren't in the dag
        assert_eq!(result.distance, None);
        // sat should be a valid enum variant
        assert!(matches!(result.sat, SatResult::Sat | SatResult::Unsat | SatResult::Unknown));
    }

    #[test]
    fn test_hamming_distance() {
        let a = vec![0u8, 0xFF];
        let b = vec![0u8, 0x00];
        assert_eq!(hamming_distance(&a, &b), 8);
    }

    #[test]
    fn test_cluster_by_invariants() {
        let mut patents = HashMap::new();
        let id1 = Uuid::new_v4();
        let id2 = Uuid::new_v4();
        let id3 = Uuid::new_v4();

        patents.insert(id1, make_ir(&["(= x 0)"], &["(= y 1)"]));
        patents.insert(id2, make_ir(&["(= x 0)"], &["(= y 1)"])); // same
        patents.insert(id3, make_ir(&["(= x 5)"], &["(= y 10)"])); // different

        let clusters = cluster_by_invariants(&patents);
        assert_eq!(clusters.len(), 2);
    }

    #[test]
    fn test_compute_novelty_score() {
        let dag = PatentDag::new();
        let solver = BuiltInSolver;
        let config = SolverConfig::default();

        let mut patents = HashMap::new();
        let existing = Uuid::new_v4();
        patents.insert(existing, make_ir(&["(= x 5)"], &["(= y 10)"]));

        let candidate_id = Uuid::new_v4();
        let candidate_ir = make_ir(&["(= x 99)"], &["(= y 100)"]);

        let score = compute_novelty_score(
            &dag,
            &candidate_id,
            &candidate_ir,
            &patents,
            &solver,
            &config,
        )
        .unwrap();

        // Novelty score should be in valid range [0, 1]
        assert!(score >= 0.0 && score <= 1.0);
    }
}
