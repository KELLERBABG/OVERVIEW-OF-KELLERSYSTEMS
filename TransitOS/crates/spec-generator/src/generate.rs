use crate::spec::KineticSpec;
use crate::temporal::LtlSpec;
use transit_state_machine::state::VehicleState;

pub fn infer_spec_from_trajectory(trajectory: &[VehicleState]) -> KineticSpec {
    let velocities: Vec<f64> = trajectory.iter().map(|s| s.dynamics.velocity).collect();
    let accelerations: Vec<f64> = trajectory
        .windows(2)
        .map(|w| {
            let dv = w[1].dynamics.velocity - w[0].dynamics.velocity;
            dv.abs()
        })
        .collect();
    let brakings: Vec<f64> = trajectory
        .iter()
        .filter(|s| s.actuators.brake_pressure > 0.0)
        .map(|s| s.dynamics.acceleration.abs())
        .collect();

    KineticSpec::from_observations(&velocities, &accelerations, &brakings)
}

pub fn infer_ltl_spec(trajectory: &[VehicleState]) -> Vec<LtlSpec> {
    let mut specs = Vec::new();

    let max_speed = trajectory.iter().map(|s| s.speed()).fold(0.0_f64, f64::max);
    specs.push(LtlSpec::AlwaysWithin("speed".into(), 0.0, max_speed * 1.1));

    let max_decel = trajectory
        .iter()
        .map(|s| s.dynamics.acceleration.abs())
        .fold(0.0_f64, f64::max);
    specs.push(LtlSpec::AlwaysWithin(
        "deceleration".into(),
        0.0,
        max_decel * 1.1,
    ));

    let max_yaw = trajectory
        .iter()
        .map(|s| s.dynamics.yaw_rate.abs())
        .fold(0.0_f64, f64::max);
    specs.push(LtlSpec::AlwaysWithin("yaw_rate".into(), 0.0, max_yaw * 1.2));

    specs
}
