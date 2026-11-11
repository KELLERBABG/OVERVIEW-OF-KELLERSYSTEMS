use crate::region::StateRegion;
use transit_state_machine::params::TireCondition;
use transit_state_machine::params::VehicleParams;
use transit_state_machine::state::VehicleState;
use transit_state_machine::transitions::{apply_transition, StateTransition};

pub enum ReachDirection {
    Forward,
    Backward,
}

pub struct ReachabilityAnalysis {
    pub explored_states: Vec<VehicleState>,
    pub unsafe_states: Vec<VehicleState>,
}

pub fn forward_reach(
    initial: &VehicleState,
    params: &VehicleParams,
    input_bounds: &StateRegion,
    horizon: f64,
    dt: f64,
) -> ReachabilityAnalysis {
    let mut explored = Vec::new();
    let mut unsafe_states = Vec::new();
    let mut state = *initial;

    let mut t = 0.0;
    while t < horizon {
        let v_bounds = input_bounds.velocity_range;
        let steer = (v_bounds.0 + v_bounds.1) / 2.0;
        let a_bounds = input_bounds.acceleration_range;
        let throttle = (a_bounds.0 + a_bounds.1) / 2.0;
        let b_bounds = input_bounds.brake_range;
        let brake = (b_bounds.0 + b_bounds.1) / 2.0;

        let input = StateTransition {
            dt,
            steering_input: steer,
            throttle_input: throttle.max(0.0),
            brake_input: brake.clamp(0.0, 1.0),
        };

        state = apply_transition(&state, &input, params, &TireCondition::New);
        explored.push(state);

        if state.dynamics.side_slip.abs() > 0.3 || state.speed() > 80.0 {
            unsafe_states.push(state);
        }

        t += dt;
    }

    ReachabilityAnalysis {
        explored_states: explored,
        unsafe_states,
    }
}
