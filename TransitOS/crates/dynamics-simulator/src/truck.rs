use transit_state_machine::params::VehicleParams;
use transit_state_machine::state::ArticulationState;

pub struct ArticulatedDynamics;

impl ArticulatedDynamics {
    pub fn compute_trailer_state(
        vx: f64,
        vy: f64,
        yaw_rate: f64,
        trailer_angle: f64,
        params: &VehicleParams,
        dt: f64,
    ) -> ArticulationState {
        if !params.is_articulated() {
            return ArticulationState::default();
        }

        let h = params.hitch_to_rear_axle;
        let l_t = params.trailer_wheelbase;

        let hitch_vx = vx - h * yaw_rate * trailer_angle.sin();
        let hitch_vy = vy + h * yaw_rate * trailer_angle.cos();

        let t_yaw_rate =
            (hitch_vy * trailer_angle.cos() - hitch_vx * trailer_angle.sin()) / l_t.max(0.1);

        let new_angle = trailer_angle + (yaw_rate - t_yaw_rate) * dt;

        let stiffness = 50000.0;
        let f_lat = -stiffness * new_angle.tanh();
        let f_long =
            -stiffness * 0.05 * (vx - (hitch_vx * new_angle.cos() - hitch_vy * new_angle.sin()));

        let risk = (new_angle.abs() / 0.7).clamp(0.0, 1.0);

        ArticulationState {
            trailer_angle: new_angle,
            trailer_yaw_rate: t_yaw_rate,
            trailer_velocity: hitch_vx * new_angle.cos() + hitch_vy * new_angle.sin(),
            hitch_force_lateral: f_lat,
            hitch_force_longitudinal: f_long,
            jackknife_risk: risk,
        }
    }

    pub fn off_track_amount(trailer_angle: f64, trailer_wheelbase: f64) -> f64 {
        (trailer_wheelbase * trailer_angle.sin()).abs()
    }
}

pub struct AirBrakeSim;

impl AirBrakeSim {
    pub fn applied_brake(
        commanded: f64,
        current_applied: f64,
        dt: f64,
        build_up_time: f64,
        release_time: f64,
    ) -> f64 {
        if commanded > current_applied {
            let rate = 1.0 / build_up_time.max(0.01);
            (current_applied + rate * dt).min(commanded)
        } else {
            let rate = 1.0 / release_time.max(0.01);
            (current_applied - rate * dt).max(commanded)
        }
    }

    pub fn stopping_distance_with_lag(
        initial_speed: f64,
        max_deceleration: f64,
        brake_lag_s: f64,
    ) -> f64 {
        let v = initial_speed;
        if v <= 0.0 {
            return 0.0;
        }
        let lag_distance = v * brake_lag_s;
        let braking_distance = (v * v) / (2.0 * max_deceleration);
        lag_distance + braking_distance
    }
}
