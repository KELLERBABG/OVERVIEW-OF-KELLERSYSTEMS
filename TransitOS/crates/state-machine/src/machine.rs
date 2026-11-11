use crate::params::{TireCondition, VehicleParams};
use crate::state::VehicleState;
use crate::transitions::{apply_transition, StateTransition};

pub struct StateMachine {
    pub current: VehicleState,
    pub history: Vec<VehicleState>,
    pub params: VehicleParams,
    pub tire_cond: TireCondition,
}

impl StateMachine {
    pub fn new(initial: VehicleState, params: VehicleParams) -> Self {
        Self {
            current: initial,
            history: vec![initial],
            params,
            tire_cond: TireCondition::New,
        }
    }

    pub fn step(&mut self, input: &StateTransition) {
        let next = apply_transition(&self.current, input, &self.params, &self.tire_cond);
        self.history.push(next);
        self.current = next;
    }

    pub fn replay(&self) -> &[VehicleState] {
        &self.history
    }

    pub fn set_tire_condition(&mut self, cond: TireCondition) {
        self.tire_cond = cond;
    }
}
