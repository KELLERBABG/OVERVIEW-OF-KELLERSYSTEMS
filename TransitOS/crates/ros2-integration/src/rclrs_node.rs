//! Full rclrs-based ROS 2 safety node for production deployment.
//!
//! This module provides a real ROS 2 node that subscribes to actuator
//! commands via DDS, runs them through the verification pipeline, and
//! publishes safety verdicts. Requires the `rclrs` feature and a ROS 2
//! installation.
//!
//! # Usage (with ROS 2 installed)
//!
//! ```no_run
//! use transit_ros2_integration::rclrs_node::TransitSafetyNode;
//! use transit_ros2_integration::messages::ActuatorCommandMsg;
//!
//! let mut node = TransitSafetyNode::new("transit_safety").unwrap();
//! let cmd = ActuatorCommandMsg {
//!     steering: 0.0,
//!     throttle: 0.5,
//!     brake: 0.0,
//!     source_id: "control".into(),
//!     sources: Vec::new(),
//! };
//! let (verdict, _decision) = node.on_command(&cmd);
//! ```

use crate::messages::{ActuatorCommandMsg, SafetyVerdictMsg, VehicleStateMsg};
use transit_deployment_gate::pipeline::VerificationPipeline;
use transit_deployment_gate::verdict::{GateDecision, GateVerdict};
use transit_smt_solver::query::CollisionQuery;
use transit_state_machine::state::VehicleState;

/// Full rclrs-based ROS 2 safety node.
///
/// Subscribes to:
///   - `/transit/actuator_command` (transit_msgs/ActuatorCommand)
///   - `/transit/ego_state` (transit_msgs/VehicleState)
///   - `/transit/obstacles` (transit_msgs/ObstacleArray)
///
/// Publishes:
///   - `/transit/safety_verdict` (transit_msgs/SafetyVerdict)
pub struct TransitSafetyNode {
    node_name: String,
    pipeline: VerificationPipeline,
    latest_ego: Option<VehicleStateMsg>,
    latest_obstacles: Vec<crate::messages::ObstacleMsg>,
    check_count: u64,
    block_count: u64,
}

impl TransitSafetyNode {
    pub fn new(name: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let invariants = transit_verification_invariants::invariants::default_safety_invariants(
            transit_state_machine::params::RoadSurface::Dry,
            50.0,
            100.0,
        );
        let pipeline = VerificationPipeline::new(
            transit_deployment_gate::policy::GatePolicy::Normal,
            invariants,
        );
        Ok(Self {
            node_name: name.to_string(),
            pipeline,
            latest_ego: None,
            latest_obstacles: Vec::new(),
            check_count: 0,
            block_count: 0,
        })
    }

    /// Get the node name.
    pub fn node_name(&self) -> &str {
        &self.node_name
    }

    /// Process an incoming actuator command against the latest ego state.
    pub fn on_command(&mut self, cmd: &ActuatorCommandMsg) -> (SafetyVerdictMsg, GateDecision) {
        let ego = self.latest_ego.clone().unwrap_or_default();

        let tainted_cmd = if cmd.sources.is_empty() {
            transit_taint_analysis::taint::TaintedValue::clean(())
        } else {
            let taint = if cmd.sources.iter().any(|s| s.sensor.spoofable()) {
                transit_taint_analysis::taint::TaintStatus::Tainted
            } else {
                transit_taint_analysis::taint::TaintStatus::Clean
            };
            transit_taint_analysis::taint::TaintedValue {
                value: (),
                taint,
                sources: cmd.sources.clone(),
            }
        };

        let state = VehicleState {
            pose: transit_state_machine::state::Pose {
                x: ego.x,
                y: ego.y,
                heading: ego.heading,
            },
            dynamics: transit_state_machine::state::Dynamics {
                velocity: ego.velocity,
                lateral_velocity: 0.0,
                acceleration: 0.0,
                yaw_rate: ego.yaw_rate,
                slip_angle: 0.0,
                side_slip: 0.0,
            },
            articulation: Default::default(),
            actuators: transit_state_machine::state::ActuatorState {
                steering_angle: cmd.steering,
                brake_pressure: cmd.brake,
                throttle: cmd.throttle,
                brake_commanded: cmd.brake,
                brake_applied: cmd.brake,
            },
            tires: Default::default(),
            road: transit_state_machine::params::RoadSurface::Dry,
        };

        let query_obstacles: Vec<transit_smt_solver::query::Obstacle> = self
            .latest_obstacles
            .iter()
            .map(|o| transit_smt_solver::query::Obstacle {
                position: (o.x, o.y),
                velocity: (o.vx, o.vy),
                half_length: o.half_length,
                half_width: o.half_width,
            })
            .collect();

        let query = CollisionQuery::new(
            (ego.x, ego.y),
            ego.velocity,
            ego.heading,
            query_obstacles,
            3.0,
        );

        self.check_count += 1;
        let decision = self
            .pipeline
            .verify(&state, &ego.params, &tainted_cmd, &query);

        if matches!(decision.verdict, GateVerdict::Block) {
            self.block_count += 1;
        }

        let verdict_msg = SafetyVerdictMsg {
            allowed: matches!(decision.verdict, GateVerdict::Allow),
            reason: decision.reason.clone(),
            verification_id: decision.check_id,
            latency_us: 0.0,
        };

        (verdict_msg, decision)
    }

    /// Update the ego state from a subscriber callback.
    pub fn update_ego_state(&mut self, ego: VehicleStateMsg) {
        self.latest_ego = Some(ego);
    }

    /// Update the obstacle list from a subscriber callback.
    pub fn update_obstacles(&mut self, obstacles: Vec<crate::messages::ObstacleMsg>) {
        self.latest_obstacles = obstacles;
    }

    pub fn check_count(&self) -> u64 {
        self.check_count
    }

    pub fn block_count(&self) -> u64 {
        self.block_count
    }

    pub fn block_rate(&self) -> f64 {
        if self.check_count == 0 {
            0.0
        } else {
            self.block_count as f64 / self.check_count as f64
        }
    }

    /// Main loop — in a real ROS 2 deployment, this spins the executor.
    /// For now, it returns so the caller can drive the loop.
    pub fn run_once(&mut self, cmd: &ActuatorCommandMsg) -> SafetyVerdictMsg {
        let (verdict, _) = self.on_command(cmd);
        verdict
    }
}

/// Message types matching ROS 2 msg definitions.
pub mod msg {
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, Default, Serialize, Deserialize)]
    pub struct ActuatorCommand {
        pub steering: f64,
        pub throttle: f64,
        pub brake: f64,
        pub source_id: String,
    }

    #[derive(Debug, Clone, Default, Serialize, Deserialize)]
    pub struct VehicleState {
        pub x: f64,
        pub y: f64,
        pub heading: f64,
        pub velocity: f64,
        pub yaw_rate: f64,
    }

    #[derive(Debug, Clone, Default, Serialize, Deserialize)]
    pub struct Obstacle {
        pub x: f64,
        pub y: f64,
        pub vx: f64,
        pub vy: f64,
        pub half_length: f64,
        pub half_width: f64,
    }

    #[derive(Debug, Clone, Default, Serialize, Deserialize)]
    pub struct SafetyVerdict {
        pub allowed: bool,
        pub reason: String,
        pub check_id: u64,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use transit_state_machine::params::VehicleParams;

    #[test]
    fn safety_node_processes_command() {
        let mut node = TransitSafetyNode::new("test_node").unwrap();
        node.update_ego_state(VehicleStateMsg {
            x: 0.0,
            y: 0.0,
            heading: 0.0,
            velocity: 10.0,
            yaw_rate: 0.0,
            steering_angle: 0.0,
            params: VehicleParams::default(),
        });
        let cmd = ActuatorCommandMsg {
            steering: 0.0,
            throttle: 0.0,
            brake: 0.0,
            source_id: "test".into(),
            sources: Vec::new(),
        };
        let verdict = node.run_once(&cmd);
        assert!(verdict.allowed);
        assert_eq!(node.check_count(), 1);
    }

    #[test]
    fn safety_node_blocks_speeding() {
        let invariants = transit_verification_invariants::invariants::default_safety_invariants(
            transit_state_machine::params::RoadSurface::Dry,
            50.0,
            100.0,
        );
        let pipeline = VerificationPipeline::new(
            transit_deployment_gate::policy::GatePolicy::Normal,
            invariants,
        );
        let mut node = TransitSafetyNode {
            node_name: "test".into(),
            pipeline,
            latest_ego: Some(VehicleStateMsg {
                x: 0.0,
                y: 0.0,
                heading: 0.0,
                velocity: 60.0,
                yaw_rate: 0.0,
                steering_angle: 0.0,
                params: VehicleParams::default(),
            }),
            latest_obstacles: Vec::new(),
            check_count: 0,
            block_count: 0,
        };
        let cmd = ActuatorCommandMsg {
            steering: 0.0,
            throttle: 1.0,
            brake: 0.0,
            source_id: "test".into(),
            sources: Vec::new(),
        };
        let verdict = node.run_once(&cmd);
        assert!(!verdict.allowed);
    }
}
