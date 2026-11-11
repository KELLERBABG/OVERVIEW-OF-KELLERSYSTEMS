//! Automated aerospace specification generation.
//!
//! Per the development plan, formal specifications are generated from
//! system behavior at runtime, serving as an incorruptible, machine-readable
//! mathematical reference for certification and flight recorders.
//!
//! A [`SpecRecord`] captures one control-cycle verdict in canonical form:
//! the hash-chained record envelope, the gate verdict, and the canonical
//! formula texts of every safety case. The [`SpecChain`] maintains the
//! hash chain (each record commits to the previous record's hash), making
//! after-the-fact tampering detectable.

use crate::gate::GateReport;
use std::collections::BTreeMap;

/// Canonical textual form of a safety case verdict.
#[derive(Clone, Debug, PartialEq)]
pub struct CaseSpec {
    pub name: String,
    pub verdict: String,
    pub formula: String,
}

/// One flight-recorder spec record.
#[derive(Clone, Debug, PartialEq)]
pub struct SpecRecord {
    /// Cycle index.
    pub cycle: u64,
    /// Time (simulation seconds) at the record.
    pub t: f64,
    /// Approved/blocked gate decision.
    pub approved: bool,
    /// Wall-time of the verification, microseconds.
    pub check_time_us: u64,
    /// The canonical formula text of the block reason (if blocked).
    pub blocked_case: Option<String>,
    /// Per-case canonical specs.
    pub cases: Vec<CaseSpec>,
    /// Canonical snapshot environment used for the proof (sorted names).
    pub env: BTreeMap<String, f64>,
    /// FNV-1a hash over the canonical serialization of this record.
    pub hash: String,
    /// Hash of the previous record ("" for the genesis record).
    pub prev_hash: String,
}

/// Canonical, byte-stable serialization of a record (no hash fields).
fn canonical_body(record: &SpecRecord) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "PLANE-OS-SPEC v1 cycle={} t={:.6} approved={} check_time_us={}\n",
        record.cycle, record.t, record.approved, record.check_time_us
    ));
    if let Some(blocked) = &record.blocked_case {
        out.push_str(&format!("blocked_case={}\n", blocked));
    } else {
        out.push_str("blocked_case=none\n");
    }
    for case in &record.cases {
        out.push_str(&format!(
            "case name={} verdict={} formula={}\n",
            case.name, case.verdict, case.formula
        ));
    }
    for (k, v) in &record.env {
        out.push_str(&format!("env {}={:.6}\n", k, v));
    }
    out
}

fn fnv1a64_hex(input: &str) -> String {
    // Dependency-free FNV-1a 64-bit is NOT a cryptographic hash; we label
    // the chain as tamper-evident only. In a real DO-178C artifact a real
    // SHA-256 would be used; for the prototype this is deterministic and
    // sufficient for tests.
    let mut hash: u64 = 0xcbf29ce484222325;
    for byte in input.as_bytes() {
        hash ^= *byte as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("fnv1a64:{:016x}", hash)
}

/// A chain of spec records.
#[derive(Clone, Debug, Default)]
pub struct SpecChain {
    pub records: Vec<SpecRecord>,
}

impl SpecChain {
    pub fn new() -> Self {
        SpecChain { records: Vec::new() }
    }

    /// Append a record built from a gate report + snapshot env + time.
    pub fn push(
        &mut self,
        report: &GateReport,
        t: f64,
        env: BTreeMap<String, f64>,
    ) -> &SpecRecord {
        let prev_hash = self
            .records
            .last()
            .map(|r| r.hash.clone())
            .unwrap_or_default();

        let cases = report
            .cases
            .iter()
            .map(|c| CaseSpec {
                name: c.name.to_string(),
                verdict: c.verdict.to_string(),
                formula: c.formula.clone(),
            })
            .collect::<Vec<_>>();

        let mut record = SpecRecord {
            cycle: report.cycle,
            t,
            approved: report.approved,
            check_time_us: report.check_time_us,
            blocked_case: report.blocked_reason.map(|s| s.to_string()),
            cases,
            env,
            hash: String::new(),
            prev_hash,
        };

        let body = canonical_body(&record);
        let hash = fnv1a64_hex(&format!("{}\nprev={}", body, record.prev_hash));
        record.hash = hash;

        self.records.push(record);
        self.records.last().expect("just pushed")
    }

    /// Validate the hash chain from the earliest record forward.
    pub fn validate(&self) -> bool {
        for (i, record) in self.records.iter().enumerate() {
            let mut without_hash = record.clone();
            let expected_prev = if i == 0 {
                String::new()
            } else {
                self.records[i - 1].hash.clone()
            };
            if without_hash.prev_hash != expected_prev {
                return false;
            }
            without_hash.hash.clear();
            let body = canonical_body(&without_hash);
            let expected = fnv1a64_hex(&format!("{}\nprev={}", body, without_hash.prev_hash));
            if record.hash != expected {
                return false;
            }
        }
        true
    }

    /// Render a human-readable spec dump.
    pub fn render(&self) -> String {
        let mut out = String::new();
        for r in &self.records {
            out.push_str(&format!(
                "cycle {} t={:.1} approved={} block={:?} hash={}\n",
                r.cycle,
                r.t,
                r.approved,
                r.blocked_case.as_deref().unwrap_or("none"),
                r.hash
            ));
            for c in &r.cases {
                out.push_str(&format!("  [{}] {}\n", c.verdict, c.name));
            }
        }
        out
    }
}

/// Generate a machine-readable formal spec bundle (for certification /
/// flight recorder export). Returns the canonical text form.
pub fn export_spec(chain: &SpecChain) -> String {
    let mut out = String::new();
    out.push_str("PLANE-OS-FORMAL-SPEC v1\n");
    for r in &chain.records {
        out.push_str(&canonical_body(r));
        out.push_str(&format!("hash={}\nprev_hash={}\n", r.hash, r.prev_hash));
        out.push_str("----\n");
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::formula::{Expr, FormulaBuilder};
    use crate::gate::{CaseReport, CaseVerdict};
    use crate::solver::CheckResult;

    fn sample_report(cycle: u64, approved: bool) -> GateReport {
        GateReport {
            cycle,
            approved,
            blocked_reason: if approved { None } else { Some("bogus_case") },
            cases: vec![CaseReport {
                name: "envelope_escape",
                verdict: if approved {
                    CaseVerdict::ProvenSafe
                } else {
                    CaseVerdict::HazardSat
                },
                result: if approved {
                    CheckResult::Unsat
                } else {
                    CheckResult::Sat
                },
                formula: Expr::state("ias_knots")
                    .gt(Expr::magic("envelope.vne_knots"))
                    .to_canonical(),
            }],
            case_count: 1,
            check_time_us: 42,
        }
    }

    #[test]
    fn chain_validates() {
        let mut chain = SpecChain::new();
        let mut env = BTreeMap::new();
        env.insert("state.ias_knots".to_string(), 250.0);
        chain.push(&sample_report(1, true), 0.05, env.clone());
        chain.push(&sample_report(2, false), 0.10, env.clone());
        chain.push(&sample_report(3, true), 0.15, env);
        assert!(chain.validate());
        assert_eq!(chain.records.len(), 3);
    }

    #[test]
    fn tamper_detected() {
        let mut chain = SpecChain::new();
        let mut env = BTreeMap::new();
        env.insert("state.ias_knots".to_string(), 250.0);
        chain.push(&sample_report(1, true), 0.05, env.clone());
        chain.push(&sample_report(2, false), 0.10, env);
        // Tamper with cycle 2's approved flag.
        chain.records[1].approved = true;
        assert!(!chain.validate());
    }

    #[test]
    fn export_contains_hashes() {
        let mut chain = SpecChain::new();
        chain.push(&sample_report(1, true), 0.05, BTreeMap::new());
        chain.push(&sample_report(2, true), 0.10, BTreeMap::new());
        let text = export_spec(&chain);
        assert!(text.contains("PLANE-OS-FORMAL-SPEC v1"));
        assert!(text.contains("hash="));
    }
}