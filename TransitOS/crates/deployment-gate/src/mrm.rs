use serde::{Deserialize, Serialize};
use transit_state_machine::params::RoadSurface;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MrmCommand {
    pub steering: f64,
    pub throttle: f64,
    pub brake: f64,
    pub brake_left_bias: f64,
    pub brake_right_bias: f64,
    pub reason: String,
}

impl MrmCommand {
    pub fn symmetric(steering: f64, throttle: f64, brake: f64, reason: &str) -> Self {
        Self {
            steering,
            throttle,
            brake,
            brake_left_bias: 1.0,
            brake_right_bias: 1.0,
            reason: reason.into(),
        }
    }

    pub fn asymmetric(
        steering: f64,
        throttle: f64,
        brake: f64,
        brake_left_bias: f64,
        brake_right_bias: f64,
        reason: &str,
    ) -> Self {
        Self {
            steering,
            throttle,
            brake,
            brake_left_bias: brake_left_bias.clamp(0.0, 1.0),
            brake_right_bias: brake_right_bias.clamp(0.0, 1.0),
            reason: reason.into(),
        }
    }
}

pub struct MrmController;

impl MrmController {
    pub fn compute_split_mu_biases(mu_left: f64, mu_right: f64) -> (f64, f64) {
        let min_mu = mu_left.min(mu_right);
        if min_mu <= 0.0 {
            return (1.0, 1.0);
        }
        let ratio = min_mu / mu_left.max(mu_right);
        if mu_left < mu_right {
            (ratio, 1.0)
        } else {
            (1.0, ratio)
        }
    }

    pub fn straight_line_brake(velocity: f64, road: &RoadSurface) -> MrmCommand {
        let max_dec = road.max_deceleration();
        let brake_target = (velocity.abs() / (2.0 * max_dec * 0.5)).clamp(0.3, 1.0);
        MrmCommand::symmetric(
            0.0,
            0.0,
            brake_target,
            "MRM: straight-line braking within friction limits",
        )
    }

    pub fn straight_line_brake_split_mu(velocity: f64, mu_left: f64, mu_right: f64) -> MrmCommand {
        let max_dec = mu_left.max(mu_right) * 9.81;
        let brake_target = (velocity.abs() / (2.0 * max_dec * 0.5)).clamp(0.3, 1.0);
        let (bias_left, bias_right) = Self::compute_split_mu_biases(mu_left, mu_right);

        let steer_correction = if mu_left < mu_right {
            (mu_right - mu_left).clamp(0.0, 0.05)
        } else if mu_right < mu_left {
            -(mu_left - mu_right).clamp(0.0, 0.05)
        } else {
            0.0
        };

        MrmCommand::asymmetric(
            steer_correction,
            0.0,
            brake_target,
            bias_left,
            bias_right,
            "MRM: split-mu braking with per-wheel bias",
        )
    }

    pub fn pull_over(velocity: f64, _road: &RoadSurface) -> MrmCommand {
        let brake = (velocity.abs() / 30.0).clamp(0.2, 0.6);
        let steer = if velocity > 1.0 { 0.08 } else { 0.0 };
        MrmCommand::symmetric(
            steer,
            0.0,
            brake,
            "MRM: pulling over to shoulder with gentle braking",
        )
    }

    pub fn gradual_decel(velocity: f64, road: &RoadSurface) -> MrmCommand {
        let max_dec = road.max_deceleration();
        let comfortable_dec = max_dec * 0.3;
        let brake = (velocity.abs() / (2.0 * comfortable_dec * 0.5)).clamp(0.1, 0.4);
        MrmCommand::symmetric(0.0, 0.0, brake, "MRM: gradual deceleration to safe stop")
    }

    pub fn select(velocity: f64, road: &RoadSurface, obstacle_distance: Option<f64>) -> MrmCommand {
        match obstacle_distance {
            Some(dist) if dist < 20.0 => Self::straight_line_brake(velocity, road),
            Some(dist) if dist < 50.0 => {
                if velocity > 10.0 {
                    Self::straight_line_brake(velocity, road)
                } else {
                    Self::pull_over(velocity, road)
                }
            }
            _ => Self::gradual_decel(velocity, road),
        }
    }
}
