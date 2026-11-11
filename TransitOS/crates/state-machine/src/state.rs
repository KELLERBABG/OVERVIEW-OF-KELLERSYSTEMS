use crate::params::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct VehicleState {
    pub pose: Pose,
    pub dynamics: Dynamics,
    pub articulation: ArticulationState,
    pub actuators: ActuatorState,
    pub tires: TireState,
    pub road: RoadSurface,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct Pose {
    pub x: f64,
    pub y: f64,
    pub heading: f64,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct Dynamics {
    pub velocity: f64,
    pub lateral_velocity: f64,
    pub acceleration: f64,
    pub yaw_rate: f64,
    pub slip_angle: f64,
    pub side_slip: f64,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct ArticulationState {
    pub trailer_angle: f64,
    pub trailer_yaw_rate: f64,
    pub trailer_velocity: f64,
    pub hitch_force_lateral: f64,
    pub hitch_force_longitudinal: f64,
    pub jackknife_risk: f64,
}

impl Default for ArticulationState {
    fn default() -> Self {
        Self {
            trailer_angle: 0.0,
            trailer_yaw_rate: 0.0,
            trailer_velocity: 0.0,
            hitch_force_lateral: 0.0,
            hitch_force_longitudinal: 0.0,
            jackknife_risk: 0.0,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct ActuatorState {
    pub steering_angle: f64,
    pub brake_pressure: f64,
    pub throttle: f64,
    pub brake_commanded: f64,
    pub brake_applied: f64,
}

impl ActuatorState {
    pub fn effective_brake_force(&self) -> f64 {
        self.brake_applied
    }
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq)]
pub struct TireState {
    pub front_lateral_force: f64,
    pub rear_lateral_force: f64,
    pub front_longitudinal_force: f64,
    pub rear_longitudinal_force: f64,
    pub front_slip_angle: f64,
    pub rear_slip_angle: f64,
}

impl VehicleState {
    pub fn speed(&self) -> f64 {
        (self.dynamics.velocity * self.dynamics.velocity
            + self.dynamics.lateral_velocity * self.dynamics.lateral_velocity)
            .sqrt()
    }

    pub fn is_stationary(&self) -> bool {
        self.speed() < 1e-6
    }

    pub fn braking_distance(&self, params: &VehicleParams) -> f64 {
        let v = self.dynamics.velocity;
        if v <= 0.0 {
            return 0.0;
        }
        let max_dec = self.road.max_deceleration();
        let air_lag = if params.actuator_delay.total_brake_lag() > 0.0 {
            v * params.actuator_delay.total_brake_lag()
        } else {
            0.0
        };
        let stopping = (v * v) / (2.0 * max_dec);
        stopping + air_lag
    }

    pub fn curvature_radius(&self) -> f64 {
        if self.dynamics.yaw_rate.abs() < 1e-8 {
            return f64::INFINITY;
        }
        self.dynamics.velocity / self.dynamics.yaw_rate
    }

    pub fn is_jackknifed(&self) -> bool {
        self.articulation.trailer_angle.abs() > 0.45
    }
}

impl Default for VehicleState {
    fn default() -> Self {
        Self {
            pose: Pose {
                x: 0.0,
                y: 0.0,
                heading: 0.0,
            },
            dynamics: Dynamics {
                velocity: 0.0,
                lateral_velocity: 0.0,
                acceleration: 0.0,
                yaw_rate: 0.0,
                slip_angle: 0.0,
                side_slip: 0.0,
            },
            articulation: ArticulationState::default(),
            actuators: ActuatorState {
                steering_angle: 0.0,
                brake_pressure: 0.0,
                throttle: 0.0,
                brake_commanded: 0.0,
                brake_applied: 0.0,
            },
            tires: TireState::default(),
            road: RoadSurface::Dry,
        }
    }
}
