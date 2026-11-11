use transit_state_machine::params::RoadSurface;
use transit_state_machine::state::VehicleState;

pub struct ManeuverFeasibility {
    pub feasible: bool,
    pub required_deceleration: f64,
    pub stopping_distance: f64,
    pub safety_margin: f64,
    pub max_available_deceleration: f64,
}

pub fn analyze_emergency_brake(
    state: &VehicleState,
    obstacle_distance: f64,
    road: &RoadSurface,
) -> ManeuverFeasibility {
    let mu = road.friction_coeff();
    let max_dec = mu * 9.81;
    let v = state.dynamics.velocity;

    let stop_dist = (v * v) / (2.0 * max_dec);

    let required_dec = if obstacle_distance > 0.0 {
        (v * v) / (2.0 * obstacle_distance)
    } else {
        f64::INFINITY
    };

    ManeuverFeasibility {
        feasible: stop_dist <= obstacle_distance,
        required_deceleration: required_dec,
        stopping_distance: stop_dist,
        safety_margin: obstacle_distance - stop_dist,
        max_available_deceleration: max_dec,
    }
}

pub fn analyze_evasive_swerve(
    state: &VehicleState,
    lateral_offset: f64,
    available_distance: f64,
    road: &RoadSurface,
) -> ManeuverFeasibility {
    let mu = road.friction_coeff();
    let v = state.dynamics.velocity;
    let max_lat_accel = mu * 9.81;

    let time_available = available_distance / v.max(1.0);
    let required_lat_accel = 2.0 * lateral_offset / (time_available * time_available);

    ManeuverFeasibility {
        feasible: required_lat_accel <= max_lat_accel,
        required_deceleration: required_lat_accel,
        stopping_distance: available_distance,
        safety_margin: max_lat_accel - required_lat_accel,
        max_available_deceleration: max_lat_accel,
    }
}
