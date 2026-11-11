use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KineticSpec {
    pub max_velocity: f64,
    pub max_acceleration: f64,
    pub max_deceleration: f64,
    pub max_steering_angle: f64,
    pub max_yaw_rate: f64,
    pub max_side_slip: f64,
    pub p95_velocity: f64,
    pub p95_acceleration: f64,
    pub p95_braking: f64,
}

impl KineticSpec {
    pub fn from_observations(velocities: &[f64], accelerations: &[f64], brakings: &[f64]) -> Self {
        if velocities.is_empty() {
            return Self::default();
        }
        let max_v = velocities.iter().cloned().fold(0.0_f64, f64::max);
        let max_a = accelerations.iter().cloned().fold(0.0_f64, f64::max);
        let max_b = brakings.iter().cloned().fold(0.0_f64, f64::max);

        let mut sorted_v = velocities.to_vec();
        sorted_v.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let p95_v = sorted_v[(sorted_v.len() as f64 * 0.95) as usize].max(0.0);

        let mut sorted_a = accelerations.to_vec();
        sorted_a.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let p95_a = sorted_a[(sorted_a.len() as f64 * 0.95) as usize].max(0.0);

        let mut sorted_b = brakings.to_vec();
        sorted_b.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let p95_b = sorted_b[(sorted_b.len() as f64 * 0.95) as usize].max(0.0);

        Self {
            max_velocity: max_v,
            max_acceleration: max_a,
            max_deceleration: max_b,
            max_steering_angle: 0.5,
            max_yaw_rate: 2.0,
            max_side_slip: 0.3,
            p95_velocity: p95_v,
            p95_acceleration: p95_a,
            p95_braking: p95_b,
        }
    }
}

impl Default for KineticSpec {
    fn default() -> Self {
        Self {
            max_velocity: 0.0,
            max_acceleration: 0.0,
            max_deceleration: 0.0,
            max_steering_angle: 0.5,
            max_yaw_rate: 2.0,
            max_side_slip: 0.3,
            p95_velocity: 0.0,
            p95_acceleration: 0.0,
            p95_braking: 0.0,
        }
    }
}
