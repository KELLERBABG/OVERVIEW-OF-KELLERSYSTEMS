use crate::audit::AuditLog;
use crate::gate::{nearest_obstacle_distance, DeploymentGate};
use crate::mrm::MrmController;
use crate::policy::GatePolicy;
use crate::verdict::{GateDecision, GateVerdict};
use std::sync::atomic::{AtomicU64, Ordering};
use transit_smt_solver::query::CollisionQuery;
use transit_state_machine::params::VehicleParams;
use transit_state_machine::state::VehicleState;
use transit_state_space::edge_case::detect_state_edge_cases;
use transit_taint_analysis::taint::TaintedValue;
use transit_verification_invariants::invariants::SafetyInvariant;
use transit_verification_invariants::proof::verify_all;

/// Global monotonic check ID for correlation across log entries.
static GLOBAL_CHECK_ID: AtomicU64 = AtomicU64::new(1);

pub struct VerificationPipeline {
    pub policy: GatePolicy,
    pub invariants: Vec<Box<dyn SafetyInvariant>>,
    pub audit: AuditLog,
    pub max_latency_us: f64,
}

impl VerificationPipeline {
    pub fn new(policy: GatePolicy, invariants: Vec<Box<dyn SafetyInvariant>>) -> Self {
        Self {
            policy,
            invariants,
            audit: AuditLog::new(),
            max_latency_us: 1000.0,
        }
    }

    pub fn verify(
        &mut self,
        state: &VehicleState,
        params: &VehicleParams,
        actuator_command: &TaintedValue<()>,
        collision_query: &CollisionQuery,
    ) -> GateDecision {
        let check_id = GLOBAL_CHECK_ID.fetch_add(1, Ordering::Relaxed);
        let _span =
            tracing::info_span!("verify", check_id, velocity = state.dynamics.velocity).entered();
        let start = std::time::Instant::now();
        let mut checks = Vec::new();

        if self.policy.require_taint_check() {
            checks.push("taint_check".to_string());
            if !actuator_command.is_trusted() {
                let decision =
                    GateDecision::block(self.audit.next_id, "Untrusted actuator command");
                let elapsed = start.elapsed().as_secs_f64() * 1_000_000.0;
                self.audit.record(decision.clone(), elapsed, checks);
                return decision;
            }
        }

        if self.policy.require_invariant_check() {
            checks.push("invariant_check".to_string());
            let results = verify_all(&self.invariants, state, params);
            let any_violated = results.iter().any(|r| !r.passed);
            if any_violated {
                let reasons: Vec<String> = results
                    .iter()
                    .filter(|r| !r.passed)
                    .map(|r| r.invariant_name.clone())
                    .collect();
                let decision = GateDecision::block(
                    self.audit.next_id,
                    &format!("Invariant violation: {}", reasons.join(", ")),
                );
                let elapsed = start.elapsed().as_secs_f64() * 1_000_000.0;
                self.audit.record(decision.clone(), elapsed, checks);
                return decision;
            }
        }

        if self.policy.require_edge_case_check() {
            checks.push("edge_case_check".to_string());
            let edge_cases = detect_state_edge_cases(state);
            if !edge_cases.is_empty() {
                let names = edge_cases
                    .iter()
                    .map(|c| c.description)
                    .collect::<Vec<_>>()
                    .join(", ");
                let mrm = MrmController::select(
                    state.dynamics.velocity,
                    &state.road,
                    nearest_obstacle_distance(collision_query),
                );
                let decision = GateDecision::block_with_mrm(
                    self.audit.next_id,
                    &format!("Edge case detected: {names}"),
                    mrm,
                );
                let elapsed = start.elapsed().as_secs_f64() * 1_000_000.0;
                self.audit.record(decision.clone(), elapsed, checks);
                return decision;
            }
        }

        if self.policy.require_collision_check() {
            checks.push("collision_check".to_string());
            let decision = DeploymentGate::evaluate(
                state,
                params,
                &self.invariants,
                actuator_command,
                collision_query,
            );
            let elapsed = start.elapsed().as_secs_f64() * 1_000_000.0;
            self.audit.record(decision.clone(), elapsed, checks);
            return decision;
        }

        let decision = GateDecision::allow(self.audit.next_id);
        let elapsed = start.elapsed().as_secs_f64() * 1_000_000.0;
        self.audit.record(decision.clone(), elapsed, checks);
        decision
    }

    pub fn block_rate(&self) -> f64 {
        let entries = self.audit.entries();
        if entries.is_empty() {
            return 0.0;
        }
        let blocks = entries
            .iter()
            .filter(|e| matches!(e.decision.verdict, GateVerdict::Block))
            .count();
        blocks as f64 / entries.len() as f64
    }
}
