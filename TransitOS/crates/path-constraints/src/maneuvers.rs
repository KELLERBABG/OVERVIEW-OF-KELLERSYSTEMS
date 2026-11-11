use crate::constraint::{Constraint, ConstraintSet};

pub fn lane_change_constraint(lane_width: f64, safe_gap: f64) -> ConstraintSet {
    let mut set = ConstraintSet::new();
    set.add(Constraint::Within(
        "lateral_position".into(),
        -lane_width,
        lane_width,
    ));
    set.add(Constraint::Ge("lateral_accel".into(), 0.0));
    set.add(Constraint::Le("lateral_accel".into(), 4.0));
    set.add(Constraint::Ge("long_gap".into(), safe_gap));
    set.add(Constraint::Ge("velocity".into(), 0.0));
    set
}

pub fn emergency_brake_constraint(obstacle_distance: f64) -> ConstraintSet {
    let mut set = ConstraintSet::new();
    set.add(Constraint::Le(
        "stopping_distance".into(),
        obstacle_distance,
    ));
    set.add(Constraint::Ge("deceleration".into(), 0.0));
    set.add(Constraint::Le("deceleration".into(), 9.81));
    set.add(Constraint::Within("slip_angle".into(), -0.1, 0.1));
    set
}

pub fn evasive_swerve_constraint(lateral_offset: f64, max_steering: f64) -> ConstraintSet {
    let mut set = ConstraintSet::new();
    set.add(Constraint::Within(
        "steering_angle".into(),
        -max_steering,
        max_steering,
    ));
    set.add(Constraint::Le("lateral_accel".into(), 6.0));
    set.add(Constraint::Ge(
        "lateral_displacement".into(),
        lateral_offset,
    ));
    set.add(Constraint::Ge("velocity".into(), 0.0));
    set
}

pub fn intersection_safe_constraint() -> ConstraintSet {
    let mut set = ConstraintSet::new();
    set.add(Constraint::Le("velocity".into(), 15.0));
    set.add(Constraint::Ge("stopping_distance".into(), 0.0));
    set.add(Constraint::Le("stopping_distance".into(), 20.0));
    set
}

pub fn curve_safe_constraint(curve_radius: f64, mu: f64) -> ConstraintSet {
    let max_speed = (mu * 9.81 * curve_radius).sqrt();
    let mut set = ConstraintSet::new();
    set.add(Constraint::Le("velocity".into(), max_speed));
    set.add(Constraint::Le("lateral_accel".into(), mu * 9.81));
    set
}

pub fn merge_safe_constraint(target_speed: f64, merge_gap: f64) -> ConstraintSet {
    let mut set = ConstraintSet::new();
    set.add(Constraint::Ge("velocity".into(), target_speed * 0.9));
    set.add(Constraint::Le("velocity".into(), target_speed * 1.1));
    set.add(Constraint::Ge("rear_gap".into(), merge_gap));
    set
}
