use transit_state_machine::params::VehicleParams;
use transit_state_machine::state::{Dynamics, TireState, VehicleState};

pub struct BicycleModel;

impl BicycleModel {
    pub fn compute_load_transfer(params: &VehicleParams, ax: f64, ay: f64) -> (f64, f64, f64, f64) {
        let g = 9.81;
        let h = params.cog_height;
        let l = params.wheelbase;
        let t = params.track_width;
        let m = params.mass;
        let fz_static_front = m * g * (1.0 - params.front_wt_frac);
        let fz_static_rear = m * g * params.front_wt_frac;

        let fz_front = fz_static_front - m * ax * h / l;
        let fz_rear = fz_static_rear + m * ax * h / l;

        let fz_front_left = fz_front / 2.0 - m * ay * h / t;
        let fz_front_right = fz_front / 2.0 + m * ay * h / t;
        let fz_rear_left = fz_rear / 2.0 - m * ay * h / t;
        let fz_rear_right = fz_rear / 2.0 + m * ay * h / t;

        (
            fz_front_left.max(0.0),
            fz_front_right.max(0.0),
            fz_rear_left.max(0.0),
            fz_rear_right.max(0.0),
        )
    }

    pub fn compute_state_derivative(
        state: &VehicleState,
        params: &VehicleParams,
        tire_forces: &TireState,
    ) -> Dynamics {
        let m = params.mass;
        let iz = m * params.wheelbase * params.wheelbase / 12.0;
        let vx = state.dynamics.velocity;
        let vy = state.dynamics.lateral_velocity;
        let yaw_rate = state.dynamics.yaw_rate;
        let steering = state.actuators.steering_angle;

        let f_yf = tire_forces.front_lateral_force;
        let f_yr = tire_forces.rear_lateral_force;
        let f_long = tire_forces.front_longitudinal_force + tire_forces.rear_longitudinal_force;

        let ax = (f_long + f_yf * steering.sin()) / m + yaw_rate * vy;
        let ay = (f_yf * steering.cos() + f_yr) / m - yaw_rate * vx;
        let yaw_accel = (params.front_wt_frac * params.wheelbase * f_yf * steering.cos()
            - (1.0 - params.front_wt_frac) * params.wheelbase * f_yr)
            / iz;

        Dynamics {
            velocity: ax,
            lateral_velocity: ay,
            acceleration: ax,
            yaw_rate: yaw_accel,
            slip_angle: if vx.abs() > 0.1 {
                ((vy + (1.0 - params.front_wt_frac) * params.wheelbase * yaw_rate) / vx).atan()
            } else {
                0.0
            },
            side_slip: if vx.abs() > 0.1 {
                (vy / vx).atan()
            } else {
                0.0
            },
        }
    }
}
