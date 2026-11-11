use crate::bridge::E2eTestRun;
use transit_dynamics_simulator::scenarios::Scenario;
use transit_ros2_integration::bridge::Ros2Bridge;
use transit_ros2_integration::messages::{ActuatorCommandMsg, ObstacleMsg};

pub struct ScenarioRunner {
    pub bridge: Ros2Bridge,
}

impl ScenarioRunner {
    pub fn new(bridge: Ros2Bridge) -> Self {
        Self { bridge }
    }

    pub fn run_scenario(&mut self, scenario: &Scenario) -> E2eTestRun {
        let mut run = E2eTestRun::new(Scenario {
            initial_state: scenario.initial_state,
            params: scenario.params,
            description: scenario.description,
        });

        let state = &scenario.initial_state;
        let state_msg = crate::bridge::CarlaBridge::translate_state_to_msg(state, &scenario.params);

        let cmd = ActuatorCommandMsg {
            steering: 0.0,
            throttle: 0.0,
            brake: 0.0,
            source_id: "scenario_runner".into(),
            sources: Vec::new(),
        };

        let obstacles: Vec<ObstacleMsg> = Vec::new();

        let (verdict, _) = self
            .bridge
            .on_actuator_command(&cmd, &state_msg, &obstacles);

        run.vehicle_state_msgs.push(state_msg);
        run.verdicts.push(verdict);
        run
    }
}
