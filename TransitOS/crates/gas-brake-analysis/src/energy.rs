use transit_state_machine::params::RoadSurface;
use transit_state_machine::state::VehicleState;

pub struct KineticEnergy;

impl KineticEnergy {
    pub fn translational(mass: f64, velocity: f64) -> f64 {
        0.5 * mass * velocity * velocity
    }

    pub fn rotational(inertia: f64, angular_velocity: f64) -> f64 {
        0.5 * inertia * angular_velocity * angular_velocity
    }

    pub fn total(mass: f64, velocity: f64, inertia: f64, yaw_rate: f64) -> f64 {
        Self::translational(mass, velocity) + Self::rotational(inertia, yaw_rate)
    }
}

pub struct BrakingEnergy;

impl BrakingEnergy {
    pub fn work_at_wheels(brake_force: f64, stopping_distance: f64) -> f64 {
        brake_force * stopping_distance
    }

    pub fn min_brake_force(mass: f64, velocity: f64, stopping_distance: f64) -> f64 {
        0.5 * mass * velocity * velocity / stopping_distance.max(1e-6)
    }

    pub fn stopping_distance(velocity: f64, deceleration: f64) -> f64 {
        if deceleration <= 0.0 {
            return f64::INFINITY;
        }
        (velocity * velocity) / (2.0 * deceleration)
    }

    pub fn regen_available(velocity: f64, brake_pressure: f64) -> f64 {
        let regen_frac = 0.3;
        let max_regen = 3000.0;
        (brake_pressure * max_regen * regen_frac * (velocity / 30.0).min(1.0)).min(max_regen)
    }
}

pub fn verify_energy_budget(
    state: &VehicleState,
    mass: f64,
    brake_force: f64,
    obstacle_distance: f64,
) -> bool {
    let ke = KineticEnergy::translational(mass, state.dynamics.velocity);
    let stop_dist = BrakingEnergy::stopping_distance(state.dynamics.velocity, brake_force / mass);
    if stop_dist > obstacle_distance {
        return false;
    }
    let work = brake_force * stop_dist;
    work >= ke
        - BrakingEnergy::regen_available(state.dynamics.velocity, state.actuators.brake_pressure)
}

pub fn verify_max_brake_feasibility(state: &VehicleState, road: &RoadSurface) -> f64 {
    let mu = road.friction_coeff();
    let v = state.dynamics.velocity;
    let max_dec = mu * 9.81;
    (v * v) / (2.0 * max_dec)
}
