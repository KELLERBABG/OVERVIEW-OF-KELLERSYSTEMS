use crate::mrm::{MrmCommand, MrmController};
use crate::verdict::GateDecision;
use transit_smt_solver::query::{CollisionQuery, SolverResult};
use transit_smt_solver::SmtSolver;
use transit_state_machine::params::VehicleParams;
use transit_state_machine::state::VehicleState;
use transit_taint_analysis::taint::TaintedValue;
use transit_verification_invariants::invariants::SafetyInvariant;

pub struct Interceptor {
    check_counter: u64,
}

impl Interceptor {
    pub fn new() -> Self {
        Self { check_counter: 0 }
    }

    fn mrm(&self, state: &VehicleState, collision_query: &CollisionQuery) -> MrmCommand {
        MrmController::select(
            state.dynamics.velocity,
            &state.road,
            nearest_obstacle_distance(collision_query),
        )
    }

    pub fn intercept(
        &mut self,
        state: &VehicleState,
        params: &VehicleParams,
        actuator_command: &TaintedValue<()>,
        collision_query: &CollisionQuery,
        invariants: &[Box<dyn SafetyInvariant>],
    ) -> GateDecision {
        self.check_counter += 1;
        let id = self.check_counter;

        if !actuator_command.is_trusted() {
            return GateDecision::block_with_mrm(
                id,
                "Untrusted sensor data in actuator path",
                self.mrm(state, collision_query),
            );
        }

        let violations: Vec<&str> = invariants
            .iter()
            .filter(|inv| !inv.check(state, params))
            .map(|inv| inv.name())
            .collect();
        if !violations.is_empty() {
            return GateDecision::block_with_mrm(
                id,
                &format!("Invariants violated: {}", violations.join(", ")),
                self.mrm(state, collision_query),
            );
        }

        let solver = SmtSolver::new();
        let collision_result = solver.check_collision(collision_query);
        if matches!(collision_result, SolverResult::Sat) {
            return GateDecision::block_with_mrm(
                id,
                "Collision detected by SMT solver",
                MrmController::straight_line_brake(state.dynamics.velocity, &state.road),
            );
        }

        GateDecision::allow(id)
    }
}

impl Default for Interceptor {
    fn default() -> Self {
        Self::new()
    }
}

pub struct DeploymentGate;

impl DeploymentGate {
    pub fn evaluate(
        state: &VehicleState,
        params: &VehicleParams,
        invariants: &[Box<dyn SafetyInvariant>],
        actuator_command: &TaintedValue<()>,
        collision_query: &CollisionQuery,
    ) -> GateDecision {
        let mut interceptor = Interceptor::new();
        interceptor.intercept(state, params, actuator_command, collision_query, invariants)
    }
}

pub(crate) fn nearest_obstacle_distance(collision_query: &CollisionQuery) -> Option<f64> {
    let mut nearest: Option<f64> = None;
    for obstacle in &collision_query.obstacles {
        let dx = obstacle.position.0 - collision_query.ego_position.0;
        let dy = obstacle.position.1 - collision_query.ego_position.1;
        let distance = (dx * dx + dy * dy).sqrt();
        nearest = Some(match nearest {
            Some(current) if current <= distance => current,
            _ => distance,
        });
    }
    nearest
}

#[cfg(test)]
mod tests {
    use super::Interceptor;
    use crate::verdict::GateVerdict;
    use transit_smt_solver::query::CollisionQuery;
    use transit_state_machine::params::VehicleParams;
    use transit_state_machine::state::VehicleState;
    use transit_taint_analysis::sensors::{SensorSource, SensorType};
    use transit_taint_analysis::taint::TaintedValue;
    use transit_verification_invariants::invariants::{
        JackknifeInvariant, SafetyInvariant, SpeedLimitInvariant,
    };

    #[test]
    fn invariants_are_checked_against_live_state() {
        let mut interceptor = Interceptor::new();
        let mut speeding = VehicleState::default();
        speeding.dynamics.velocity = 60.0;

        let invariants: Vec<Box<dyn SafetyInvariant>> =
            vec![Box::new(SpeedLimitInvariant { max_speed: 50.0 })];
        let query = CollisionQuery::new((0.0, 0.0), 60.0, 0.0, vec![], 3.0);

        let decision = interceptor.intercept(
            &speeding,
            &VehicleParams::default(),
            &TaintedValue::clean(()),
            &query,
            &invariants,
        );
        assert!(matches!(decision.verdict, GateVerdict::Block));
    }

    #[test]
    fn safe_state_passes_when_invariants_hold() {
        let mut interceptor = Interceptor::new();
        let mut cruising = VehicleState::default();
        cruising.dynamics.velocity = 10.0;

        let invariants: Vec<Box<dyn SafetyInvariant>> =
            vec![Box::new(SpeedLimitInvariant { max_speed: 50.0 })];
        let query = CollisionQuery::new((0.0, 0.0), 10.0, 0.0, vec![], 3.0);

        let decision = interceptor.intercept(
            &cruising,
            &VehicleParams::default(),
            &TaintedValue::clean(()),
            &query,
            &invariants,
        );
        assert!(matches!(decision.verdict, GateVerdict::Allow));
    }

    #[test]
    fn jackknife_invariant_blocks_articulated_state() {
        let mut interceptor = Interceptor::new();
        let mut jackknifing = VehicleState::default();
        jackknifing.articulation.trailer_angle = 0.5;
        let invariants: Vec<Box<dyn SafetyInvariant>> = vec![Box::new(JackknifeInvariant)];
        let query = CollisionQuery::new((0.0, 0.0), 5.0, 0.0, vec![], 3.0);

        let decision = interceptor.intercept(
            &jackknifing,
            &VehicleParams::articulated_truck(),
            &TaintedValue::clean(()),
            &query,
            &invariants,
        );
        assert!(matches!(decision.verdict, GateVerdict::Block));
    }

    #[test]
    fn low_trust_source_is_blocked() {
        let mut interceptor = Interceptor::new();
        let state = VehicleState::default();
        let command = TaintedValue::from_source(
            (),
            SensorSource {
                sensor: SensorType::Radar,
                id: "radar_0".into(),
            },
        );
        let invariants: Vec<Box<dyn SafetyInvariant>> = Vec::new();
        let query = CollisionQuery::new((0.0, 0.0), 0.0, 0.0, vec![], 3.0);

        let decision = interceptor.intercept(
            &state,
            &VehicleParams::default(),
            &command,
            &query,
            &invariants,
        );
        assert!(matches!(decision.verdict, GateVerdict::Block));
    }
}
