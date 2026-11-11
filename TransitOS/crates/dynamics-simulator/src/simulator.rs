use transit_state_machine::machine::StateMachine;
use transit_state_machine::params::VehicleParams;
use transit_state_machine::state::VehicleState;
use transit_state_machine::transitions::StateTransition;

pub struct Simulator {
    pub machine: StateMachine,
    pub time: f64,
    pub dt: f64,
}

impl Simulator {
    pub fn new(initial: VehicleState, params: VehicleParams) -> Self {
        Self {
            machine: StateMachine::new(initial, params),
            time: 0.0,
            dt: 0.01,
        }
    }

    pub fn step(&mut self, steering: f64, throttle: f64, brake: f64) {
        let input = StateTransition {
            dt: self.dt,
            steering_input: steering,
            throttle_input: throttle,
            brake_input: brake,
        };
        self.machine.step(&input);
        self.time += self.dt;
    }

    pub fn run_for(
        &mut self,
        duration: f64,
        mut control: impl FnMut(&VehicleState) -> (f64, f64, f64),
    ) {
        let mut t = 0.0;
        while t < duration {
            let (steering, throttle, brake) = control(&self.machine.current);
            self.step(steering, throttle, brake);
            t += self.dt;
        }
    }
}
