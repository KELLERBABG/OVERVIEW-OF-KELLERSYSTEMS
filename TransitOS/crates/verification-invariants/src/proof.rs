use crate::invariants::SafetyInvariant;
use transit_state_machine::params::VehicleParams;
use transit_state_machine::state::VehicleState;

pub struct ProofObligation {
    pub invariant: Box<dyn SafetyInvariant>,
    pub state: VehicleState,
    pub params: VehicleParams,
}

impl ProofObligation {
    pub fn new(
        invariant: Box<dyn SafetyInvariant>,
        state: VehicleState,
        params: VehicleParams,
    ) -> Self {
        Self {
            invariant,
            state,
            params,
        }
    }

    pub fn verify(&self) -> bool {
        self.invariant.check(&self.state, &self.params)
    }
}

pub struct ProofResult {
    pub invariant_name: String,
    pub passed: bool,
    pub description: String,
}

pub fn verify_all(
    invariants: &[Box<dyn SafetyInvariant>],
    state: &VehicleState,
    params: &VehicleParams,
) -> Vec<ProofResult> {
    invariants
        .iter()
        .map(|inv| ProofResult {
            invariant_name: inv.name().to_string(),
            passed: inv.check(state, params),
            description: inv.description().to_string(),
        })
        .collect()
}

pub fn all_invariants_pass(
    invariants: &[Box<dyn SafetyInvariant>],
    state: &VehicleState,
    params: &VehicleParams,
) -> bool {
    invariants.iter().all(|inv| inv.check(state, params))
}
