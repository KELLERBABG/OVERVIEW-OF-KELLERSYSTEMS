use transit_state_machine::params::{TireCondition, VehicleParams};

pub struct TireModel;

impl TireModel {
    pub fn compute_lateral_force(slip_angle: f64, normal_load: f64, mu: f64) -> f64 {
        let cf = 80000.0;
        let max_f = mu * normal_load;
        let linear = -cf * slip_angle;
        let saturated = max_f * (slip_angle.abs() * cf / max_f).atanh().signum() * max_f;
        if linear.abs() < max_f {
            linear
        } else {
            saturated.clamp(-max_f, max_f)
        }
    }

    pub fn compute_slip_angles(
        vx: f64,
        vy: f64,
        yaw_rate: f64,
        steering: f64,
        params: &VehicleParams,
    ) -> (f64, f64) {
        let a = params.front_wt_frac * params.wheelbase;
        let b = (1.0 - params.front_wt_frac) * params.wheelbase;

        let alpha_f = if vx.abs() > 0.1 {
            ((vy + a * yaw_rate) / vx).atan() - steering
        } else {
            -steering
        };

        let alpha_r = if vx.abs() > 0.1 {
            ((vy - b * yaw_rate) / vx).atan()
        } else {
            0.0
        };

        (alpha_f, alpha_r)
    }

    pub fn condition_multiplier(cond: &TireCondition) -> f64 {
        match cond {
            TireCondition::New => 1.0,
            TireCondition::Worn => 0.7,
            TireCondition::Blown => 0.2,
        }
    }
}
