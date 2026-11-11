use transit_dynamics_simulator::scenarios::Scenario;
use transit_ros2_integration::messages::{
    ActuatorCommandMsg, ObstacleMsg, SafetyVerdictMsg, VehicleStateMsg,
};
use transit_state_machine::params::{RoadSurface, VehicleParams};
use transit_state_machine::state::VehicleState;
use transit_verification_invariants::invariants::default_safety_invariants;

pub struct CarlaBridgeConfig {
    pub host: String,
    pub port: u16,
    pub timeout_ms: u64,
    pub sync_mode: bool,
}

impl Default for CarlaBridgeConfig {
    fn default() -> Self {
        Self {
            host: "localhost".into(),
            port: 2000,
            timeout_ms: 5000,
            sync_mode: true,
        }
    }
}

pub struct E2eTestRun {
    pub scenario: Scenario,
    pub vehicle_state_msgs: Vec<VehicleStateMsg>,
    pub obstacle_msgs: Vec<ObstacleMsg>,
    pub verdicts: Vec<SafetyVerdictMsg>,
}

impl E2eTestRun {
    pub fn new(scenario: Scenario) -> Self {
        Self {
            scenario,
            vehicle_state_msgs: Vec::new(),
            obstacle_msgs: Vec::new(),
            verdicts: Vec::new(),
        }
    }

    pub fn all_verdicts_allowed(&self) -> bool {
        self.verdicts.iter().all(|v| v.allowed)
    }
}

pub struct CarlaBridge;

impl CarlaBridge {
    pub fn translate_state_to_msg(state: &VehicleState, params: &VehicleParams) -> VehicleStateMsg {
        VehicleStateMsg {
            x: state.pose.x,
            y: state.pose.y,
            heading: state.pose.heading,
            velocity: state.dynamics.velocity,
            yaw_rate: state.dynamics.yaw_rate,
            steering_angle: state.actuators.steering_angle,
            params: *params,
        }
    }

    pub fn translate_command_to_actuator(cmd: &ActuatorCommandMsg) -> (f64, f64, f64) {
        (cmd.steering, cmd.throttle, cmd.brake)
    }

    pub fn build_default_invariants(
        road: RoadSurface,
    ) -> Vec<Box<dyn transit_verification_invariants::invariants::SafetyInvariant>> {
        default_safety_invariants(road, 50.0, 100.0)
    }
}
