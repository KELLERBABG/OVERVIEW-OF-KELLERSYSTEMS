use crate::symbolic::{constant, var, SymbolicExpr};

pub fn braking_distance_expr() -> SymbolicExpr {
    let v = var("velocity");
    let mu = var("friction_coeff");
    let g = constant(9.81);
    v.clone().pow(2) / (constant(2.0) * mu * g)
}

pub fn lateral_accel_expr() -> SymbolicExpr {
    var("velocity") * var("yaw_rate")
}

pub fn centripetal_force_expr() -> SymbolicExpr {
    let m = var("mass");
    let v = var("velocity");
    let r = var("curvature_radius");
    m * v.pow(2) / r
}

pub fn slip_angle_expr(wheelbase_frac: f64) -> SymbolicExpr {
    let vy = var("lateral_velocity");
    let vx = var("velocity");
    let yaw_rate = var("yaw_rate");
    let a = constant(wheelbase_frac);
    let steering = var("steering_angle");
    (vy + a * yaw_rate) / vx - steering
}

pub fn tire_force_expr() -> SymbolicExpr {
    let cf = constant(80000.0);
    let slip = var("slip_angle");
    let mu = var("friction_coeff");
    cf * SymbolicExpr::Tanh(Box::new(slip)) * mu
}

pub fn stopping_distance_condition(obstacle_dist: f64) -> SymbolicExpr {
    SymbolicExpr::Sub(
        Box::new(braking_distance_expr()),
        Box::new(constant(obstacle_dist)),
    )
}
