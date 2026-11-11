use transit_state_machine::params::{RoadSurface, VehicleParams};
use transit_state_machine::state::VehicleState;

pub trait SafetyInvariant: Send {
    fn name(&self) -> &str;
    fn check(&self, state: &VehicleState, params: &VehicleParams) -> bool;
    fn description(&self) -> &str;
}

pub struct BrakingDistanceInvariant {
    pub safety_gap: f64,
    pub max_deceleration: f64,
}

impl SafetyInvariant for BrakingDistanceInvariant {
    fn name(&self) -> &str {
        "braking_distance_invariant"
    }

    fn description(&self) -> &str {
        "Braking distance must not exceed the safety gap to any obstacle"
    }

    fn check(&self, state: &VehicleState, params: &VehicleParams) -> bool {
        let stop_dist = state.braking_distance(params);
        stop_dist <= self.safety_gap
    }
}

pub struct LateralAccelerationInvariant {
    pub rollover_threshold: f64,
}

impl SafetyInvariant for LateralAccelerationInvariant {
    fn name(&self) -> &str {
        "lateral_acceleration_invariant"
    }

    fn description(&self) -> &str {
        "Lateral acceleration must stay below rollover threshold"
    }

    fn check(&self, state: &VehicleState, _params: &VehicleParams) -> bool {
        let lateral = state.dynamics.velocity * state.dynamics.yaw_rate;
        lateral.abs() <= self.rollover_threshold
    }
}

pub struct SpeedLimitInvariant {
    pub max_speed: f64,
}

impl SafetyInvariant for SpeedLimitInvariant {
    fn name(&self) -> &str {
        "speed_limit_invariant"
    }

    fn description(&self) -> &str {
        "Vehicle speed must not exceed the legal limit"
    }

    fn check(&self, state: &VehicleState, _params: &VehicleParams) -> bool {
        state.speed() <= self.max_speed
    }
}

pub struct SlipAngleInvariant {
    pub max_slip: f64,
}

impl SafetyInvariant for SlipAngleInvariant {
    fn name(&self) -> &str {
        "slip_angle_invariant"
    }

    fn description(&self) -> &str {
        "Tire slip angles must stay within stable region"
    }

    fn check(&self, state: &VehicleState, _params: &VehicleParams) -> bool {
        state.dynamics.side_slip.abs() <= self.max_slip
    }
}

pub struct YawRateInvariant {
    pub max_yaw_rate: f64,
}

impl SafetyInvariant for YawRateInvariant {
    fn name(&self) -> &str {
        "yaw_rate_invariant"
    }

    fn description(&self) -> &str {
        "Yaw rate must not exceed vehicle stability threshold"
    }

    fn check(&self, state: &VehicleState, _params: &VehicleParams) -> bool {
        state.dynamics.yaw_rate.abs() <= self.max_yaw_rate
    }
}

pub struct RoadFrictionInvariant {
    pub road: RoadSurface,
}

impl SafetyInvariant for RoadFrictionInvariant {
    fn name(&self) -> &str {
        "road_friction_invariant"
    }

    fn description(&self) -> &str {
        "Maneuver demands must not exceed available road friction"
    }

    fn check(&self, state: &VehicleState, _params: &VehicleParams) -> bool {
        let mu = self.road.friction_coeff();
        let total_accel = (state.dynamics.acceleration * state.dynamics.acceleration
            + (state.dynamics.velocity * state.dynamics.yaw_rate)
                * (state.dynamics.velocity * state.dynamics.yaw_rate))
            .sqrt();
        total_accel <= mu * 9.81 * 1.05
    }
}

pub struct SteeringRateInvariant {
    pub max_steering_rate: f64,
    pub prev_steering: f64,
    pub dt: f64,
}

impl SafetyInvariant for SteeringRateInvariant {
    fn name(&self) -> &str {
        "steering_rate_invariant"
    }

    fn description(&self) -> &str {
        "Steering angle must not change faster than actuator limits"
    }

    fn check(&self, state: &VehicleState, _params: &VehicleParams) -> bool {
        if self.dt <= 0.0 {
            return true;
        }
        let rate = (state.actuators.steering_angle - self.prev_steering).abs() / self.dt;
        rate <= self.max_steering_rate
    }
}

pub struct JackknifeInvariant;

impl SafetyInvariant for JackknifeInvariant {
    fn name(&self) -> &str {
        "jackknife_invariant"
    }

    fn description(&self) -> &str {
        "Trailer articulation angle must stay below the jackknife threshold"
    }

    fn check(&self, state: &VehicleState, _params: &VehicleParams) -> bool {
        !state.is_jackknifed()
    }
}

pub struct CompositeInvariant {
    invariants: Vec<Box<dyn SafetyInvariant>>,
}

impl CompositeInvariant {
    pub fn new(invariants: Vec<Box<dyn SafetyInvariant>>) -> Self {
        Self { invariants }
    }

    pub fn add(&mut self, inv: Box<dyn SafetyInvariant>) {
        self.invariants.push(inv);
    }
}

impl SafetyInvariant for CompositeInvariant {
    fn name(&self) -> &str {
        "composite_invariant"
    }

    fn description(&self) -> &str {
        "Conjunction of all registered safety invariants"
    }

    fn check(&self, state: &VehicleState, params: &VehicleParams) -> bool {
        self.invariants.iter().all(|inv| inv.check(state, params))
    }
}

pub fn default_safety_invariants(
    road: RoadSurface,
    speed_limit: f64,
    safety_gap: f64,
) -> Vec<Box<dyn SafetyInvariant>> {
    vec![
        Box::new(BrakingDistanceInvariant {
            safety_gap,
            max_deceleration: road.max_deceleration(),
        }),
        Box::new(LateralAccelerationInvariant {
            rollover_threshold: 6.0,
        }),
        Box::new(SpeedLimitInvariant {
            max_speed: speed_limit,
        }),
        Box::new(SlipAngleInvariant { max_slip: 0.3 }),
        Box::new(YawRateInvariant { max_yaw_rate: 2.0 }),
        Box::new(RoadFrictionInvariant { road }),
        Box::new(JackknifeInvariant),
    ]
}
