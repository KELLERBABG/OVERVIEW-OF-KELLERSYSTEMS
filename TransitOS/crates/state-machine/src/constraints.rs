use crate::params::VehicleParams;
use crate::state::VehicleState;

pub enum StateViolation {
    VelocityExceeded { max: f64, actual: f64 },
    AccelerationExceeded { max: f64, actual: f64 },
    LateralAccelExceeded { threshold: f64, actual: f64 },
    YawRateExceeded { max: f64, actual: f64 },
    SlipAngleExceeded { max: f64, actual: f64 },
    RolloverRisk { threshold: f64, actual: f64 },
    JackknifeRisk { trailer_angle: f64 },
    ActuatorLagExceeded { max_lag_s: f64, brake_lag_s: f64 },
    BrakeCommandMismatch { commanded: f64, applied: f64 },
}

pub fn check_state_bounds(state: &VehicleState, params: &VehicleParams) -> Vec<StateViolation> {
    let mut violations = Vec::new();

    let speed_limit = if params.is_articulated() { 50.0 } else { 70.0 };
    if state.dynamics.velocity.abs() > speed_limit {
        violations.push(StateViolation::VelocityExceeded {
            max: speed_limit,
            actual: state.dynamics.velocity,
        });
    }

    let max_accel = if params.is_articulated() { 6.0 } else { 10.0 };
    if state.dynamics.acceleration.abs() > max_accel {
        violations.push(StateViolation::AccelerationExceeded {
            max: max_accel,
            actual: state.dynamics.acceleration,
        });
    }

    let lateral_accel = state.dynamics.velocity * state.dynamics.yaw_rate;
    let rollover_threshold = params.rollover_threshold();
    if lateral_accel.abs() > rollover_threshold * 0.8 {
        violations.push(StateViolation::RolloverRisk {
            threshold: rollover_threshold,
            actual: lateral_accel.abs(),
        });
    }

    let max_yaw = if params.is_articulated() { 1.5 } else { 3.0 };
    if state.dynamics.yaw_rate.abs() > max_yaw {
        violations.push(StateViolation::YawRateExceeded {
            max: max_yaw,
            actual: state.dynamics.yaw_rate,
        });
    }

    let max_slip = if params.is_articulated() { 0.25 } else { 0.4 };
    if state.dynamics.side_slip.abs() > max_slip {
        violations.push(StateViolation::SlipAngleExceeded {
            max: max_slip,
            actual: state.dynamics.side_slip,
        });
    }

    if params.is_articulated() && state.articulation.trailer_angle.abs() > 0.4 {
        violations.push(StateViolation::JackknifeRisk {
            trailer_angle: state.articulation.trailer_angle,
        });
    }

    let effective_lag = params.actuator_delay.total_brake_lag();
    if effective_lag > 0.25 {
        violations.push(StateViolation::ActuatorLagExceeded {
            max_lag_s: 0.25,
            brake_lag_s: effective_lag,
        });
    }

    let brake_diff = (state.actuators.brake_commanded - state.actuators.brake_applied).abs();
    if brake_diff > 0.15 {
        violations.push(StateViolation::BrakeCommandMismatch {
            commanded: state.actuators.brake_commanded,
            applied: state.actuators.brake_applied,
        });
    }

    violations
}
