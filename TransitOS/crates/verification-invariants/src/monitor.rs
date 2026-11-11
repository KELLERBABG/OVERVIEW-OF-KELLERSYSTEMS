use crate::invariants::SafetyInvariant;
use crate::proof::verify_all;
use transit_state_machine::params::VehicleParams;
use transit_state_machine::state::VehicleState;

pub struct RuntimeMonitor {
    pub invariants: Vec<Box<dyn SafetyInvariant>>,
    pub violation_history: Vec<MonitorEvent>,
}

pub struct MonitorEvent {
    pub time: f64,
    pub invariant_name: String,
    pub violated: bool,
}

impl RuntimeMonitor {
    pub fn new(invariants: Vec<Box<dyn SafetyInvariant>>) -> Self {
        Self {
            invariants,
            violation_history: Vec::new(),
        }
    }

    pub fn check(&mut self, state: &VehicleState, time: f64, params: &VehicleParams) -> bool {
        let results = verify_all(&self.invariants, state, params);
        let all_pass = results.iter().all(|r| r.passed);
        for r in results {
            self.violation_history.push(MonitorEvent {
                time,
                invariant_name: r.invariant_name.clone(),
                violated: !r.passed,
            });
        }
        all_pass
    }

    pub fn get_violations(&self) -> Vec<&MonitorEvent> {
        self.violation_history
            .iter()
            .filter(|e| e.violated)
            .collect()
    }
}
