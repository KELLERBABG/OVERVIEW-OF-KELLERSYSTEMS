use crate::params::{RoadSurface, TireCondition, VehicleClass, VehicleParams};
use crate::state::*;

#[derive(Debug, Clone, Copy)]
pub struct StateTransition {
    pub dt: f64,
    pub steering_input: f64,
    pub throttle_input: f64,
    pub brake_input: f64,
}

fn calc_tire_forces(
    vx: f64,
    vy: f64,
    yaw_rate: f64,
    steering: f64,
    params: &VehicleParams,
    road: &RoadSurface,
    _tire_cond: &TireCondition,
) -> (f64, f64, f64, f64, f64, f64) {
    let mu = road.friction_coeff();
    let a = params.front_wt_frac * params.wheelbase;
    let b = (1.0 - params.front_wt_frac) * params.wheelbase;

    let alpha_f = if vx.abs() > 0.1 {
        (vy + a * yaw_rate) / vx - steering
    } else {
        -steering
    };

    let alpha_r = if vx.abs() > 0.1 {
        (vy - b * yaw_rate) / vx
    } else {
        0.0
    };

    let cf = if matches!(
        params.class,
        VehicleClass::HeavyTruck | VehicleClass::ArticulatedTruck
    ) {
        120000.0
    } else {
        80000.0
    };
    let cr = cf;

    let f_yf = -cf * alpha_f.tanh() * mu;
    let f_yr = -cr * alpha_r.tanh() * mu;

    (f_yf, f_yr, alpha_f, alpha_r, 0.0, 0.0)
}

fn air_brake_lag(
    commanded_brake: f64,
    current_applied: f64,
    dt: f64,
    air_brake: &crate::params::AirBrakeParams,
) -> f64 {
    if commanded_brake > current_applied {
        let rate = 1.0 / air_brake.build_up_time_s;
        (current_applied + rate * dt).min(commanded_brake)
    } else {
        let rate = 1.0 / air_brake.release_time_s;
        (current_applied - rate * dt).max(commanded_brake)
    }
}

fn update_articulation(
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

    let hitch_vel_x = vx - h * yaw_rate * trailer_angle.sin();
    let hitch_vel_y = vy + h * yaw_rate * trailer_angle.cos();

    let trailer_yaw_rate =
        (hitch_vel_y * trailer_angle.cos() - hitch_vel_x * trailer_angle.sin()) / l_t;

    let new_trailer_angle = trailer_angle + (yaw_rate - trailer_yaw_rate) * dt;
    let jackknife_risk = (new_trailer_angle.abs() / 0.7).clamp(0.0, 1.0);

    let k = 50000.0;
    let hitch_lat = -k * new_trailer_angle;
    let hitch_long =
        -k * 0.1 * (vx - (hitch_vel_x * trailer_angle.cos() - hitch_vel_y * trailer_angle.sin()));

    ArticulationState {
        trailer_angle: new_trailer_angle,
        trailer_yaw_rate,
        trailer_velocity: hitch_vel_x * trailer_angle.cos() + hitch_vel_y * trailer_angle.sin(),
        hitch_force_lateral: hitch_lat,
        hitch_force_longitudinal: hitch_long,
        jackknife_risk,
    }
}

pub fn apply_transition(
    current: &VehicleState,
    input: &StateTransition,
    params: &VehicleParams,
    tire_cond: &TireCondition,
) -> VehicleState {
    let dt = input.dt;
    let mass = params.mass;
    let iz = params.yaw_inertia();

    let vx = current.dynamics.velocity;
    let vy = current.dynamics.lateral_velocity;
    let yaw = current.pose.heading;
    let yaw_rate = current.dynamics.yaw_rate;

    let steering_lag = params.actuator_delay.steering_lag_s;
    let steer_alpha = (dt / (steering_lag + dt)).min(1.0);
    let steering_eff = current.actuators.steering_angle
        + (input
            .steering_input
            .clamp(-params.max_steering, params.max_steering)
            - current.actuators.steering_angle)
            * steer_alpha;

    let brake_applied = air_brake_lag(
        input.brake_input,
        current.actuators.brake_applied,
        dt,
        &params.air_brake,
    );

    let (f_yf, f_yr, alpha_f, alpha_r, _, _) = calc_tire_forces(
        vx,
        vy,
        yaw_rate,
        steering_eff,
        params,
        &current.road,
        tire_cond,
    );

    let articulation = update_articulation(
        vx,
        vy,
        yaw_rate,
        current.articulation.trailer_angle,
        params,
        dt,
    );

    let f_throttle = input.throttle_input * params.max_throttle_force;
    let f_brake = -brake_applied * params.max_brake_force;
    let f_long = f_throttle + f_brake - 0.4 * vx;

    let drag = 0.3 * vx * vx * if vx >= 0.0 { 1.0 } else { -1.0 };
    let f_long_total = f_long - drag + articulation.hitch_force_longitudinal;

    let ax = (f_long_total
        + f_yf * steering_eff.sin()
        + articulation.hitch_force_lateral * steering_eff.sin())
        / mass;
    let ay =
        (f_yf * steering_eff.cos() + f_yr + articulation.hitch_force_lateral * steering_eff.cos())
            / mass;
    let yaw_accel = (params.front_wt_frac * params.wheelbase * f_yf * steering_eff.cos()
        - (1.0 - params.front_wt_frac) * params.wheelbase * f_yr
        + articulation.hitch_force_lateral * params.hitch_to_rear_axle)
        / iz;

    let new_vx = vx + (ax - yaw_rate * vy) * dt;
    let new_vy = vy + (ay + yaw_rate * vx) * dt;
    let new_yaw_rate = yaw_rate + yaw_accel * dt;
    let new_heading = yaw + yaw_rate * dt;

    let new_x = current.pose.x + (vx * yaw.cos() - vy * yaw.sin()) * dt;
    let new_y = current.pose.y + (vx * yaw.sin() + vy * yaw.cos()) * dt;

    let slip_angle = if new_vx.abs() > 0.1 {
        (new_vy / new_vx).atan()
    } else {
        0.0
    };

    VehicleState {
        pose: Pose {
            x: new_x,
            y: new_y,
            heading: new_heading,
        },
        dynamics: Dynamics {
            velocity: new_vx,
            lateral_velocity: new_vy,
            acceleration: ax,
            yaw_rate: new_yaw_rate,
            slip_angle,
            side_slip: (new_vy / (new_vx.abs() + 0.01)).atan(),
        },
        articulation,
        actuators: ActuatorState {
            steering_angle: steering_eff,
            brake_pressure: input.brake_input,
            throttle: input.throttle_input,
            brake_commanded: input.brake_input,
            brake_applied,
        },
        tires: TireState {
            front_lateral_force: f_yf,
            rear_lateral_force: f_yr,
            front_longitudinal_force: 0.0,
            rear_longitudinal_force: 0.0,
            front_slip_angle: alpha_f,
            rear_slip_angle: alpha_r,
        },
        road: current.road,
    }
}
