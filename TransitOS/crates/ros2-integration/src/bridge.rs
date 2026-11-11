use crate::messages::{ActuatorCommandMsg, ObstacleMsg, SafetyVerdictMsg, VehicleStateMsg};
use std::time::Instant;
use transit_deployment_gate::pipeline::VerificationPipeline;
use transit_deployment_gate::verdict::GateDecision;
use transit_smt_solver::query::{CollisionModel, CollisionQuery, Obstacle};
use transit_state_machine::params::{RoadSurface, VehicleClass, VehicleParams};
use transit_state_machine::state::{ActuatorState, Dynamics, Pose, VehicleState};
use transit_taint_analysis::sensors::SensorSource;
use transit_taint_analysis::taint::{TaintStatus, TaintedValue};

pub struct Ros2Bridge {
    pub pipeline: VerificationPipeline,
}

impl Ros2Bridge {
    pub fn new(pipeline: VerificationPipeline) -> Self {
        Self { pipeline }
    }

    /// Verifies an actuator command against the live ego state, vehicle
    /// parameters, and the current obstacle set.
    ///
    /// The command's steering/throttle/brake values and sensor sources are
    /// wired into the vehicle state and taint value so both the invariant and
    /// trust stages see real runtime data. Collision geometry (swept circle
    /// for cars, SAT rectangle for trucks) is selected from the vehicle class.
    pub fn on_actuator_command(
        &mut self,
        cmd: &ActuatorCommandMsg,
        ego: &VehicleStateMsg,
        obstacles: &[ObstacleMsg],
    ) -> (SafetyVerdictMsg, GateDecision) {
        let tainted_cmd = taint_from_sources(&cmd.sources);

        let state = VehicleState {
            pose: Pose {
                x: ego.x,
                y: ego.y,
                heading: ego.heading,
            },
            dynamics: Dynamics {
                velocity: ego.velocity,
                lateral_velocity: 0.0,
                acceleration: 0.0,
                yaw_rate: ego.yaw_rate,
                slip_angle: 0.0,
                side_slip: 0.0,
            },
            articulation: Default::default(),
            actuators: ActuatorState {
                steering_angle: cmd.steering,
                brake_pressure: cmd.brake,
                throttle: cmd.throttle,
                brake_commanded: cmd.brake,
                brake_applied: cmd.brake,
            },
            tires: Default::default(),
            road: RoadSurface::Dry,
        };

        let query_obstacles: Vec<Obstacle> = obstacles
            .iter()
            .map(|o| Obstacle {
                position: (o.x, o.y),
                velocity: (o.vx, o.vy),
                half_length: o.half_length,
                half_width: o.half_width,
            })
            .collect();

        let (half_length, half_width) = vehicle_footprint(&ego.params);
        let query = CollisionQuery::new(
            (ego.x, ego.y),
            ego.velocity,
            ego.heading,
            query_obstacles,
            3.0,
        )
        .with_geometry(half_length, half_width, collision_model(&ego.params));

        let started = Instant::now();
        let decision = self
            .pipeline
            .verify(&state, &ego.params, &tainted_cmd, &query);
        let latency_us = started.elapsed().as_secs_f64() * 1_000_000.0;

        let verdict_msg = SafetyVerdictMsg {
            allowed: matches!(
                decision.verdict,
                transit_deployment_gate::verdict::GateVerdict::Allow
            ),
            reason: decision.reason.clone(),
            verification_id: decision.check_id,
            latency_us,
        };

        (verdict_msg, decision)
    }
}

fn taint_from_sources(sources: &[SensorSource]) -> TaintedValue<()> {
    if sources.is_empty() {
        return TaintedValue::clean(());
    }
    let taint = if sources.iter().any(|s| s.sensor.spoofable()) {
        TaintStatus::Tainted
    } else {
        TaintStatus::Clean
    };
    TaintedValue {
        value: (),
        taint,
        sources: sources.to_vec(),
    }
}

fn collision_model(params: &VehicleParams) -> CollisionModel {
    match params.class {
        VehicleClass::PassengerCar => CollisionModel::Circle,
        VehicleClass::HeavyTruck | VehicleClass::ArticulatedTruck => CollisionModel::Rectangle,
    }
}

fn vehicle_footprint(params: &VehicleParams) -> (f64, f64) {
    let length = params.wheelbase + params.trailer_wheelbase;
    ((length / 2.0).max(2.0), (params.track_width / 2.0).max(1.0))
}
