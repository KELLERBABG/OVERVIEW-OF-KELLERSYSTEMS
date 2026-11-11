use transit_state_machine::params::{RoadSurface, VehicleParams};
use transit_state_machine::state::{
    ActuatorState, ArticulationState, Dynamics, Pose, TireState, VehicleState,
};

pub struct Scenario {
    pub initial_state: VehicleState,
    pub params: VehicleParams,
    pub description: &'static str,
}

fn base_truck_state(x: f64, y: f64, heading: f64, speed: f64, road: RoadSurface) -> VehicleState {
    VehicleState {
        pose: Pose { x, y, heading },
        dynamics: Dynamics {
            velocity: speed,
            lateral_velocity: 0.0,
            acceleration: 0.0,
            yaw_rate: 0.0,
            slip_angle: 0.0,
            side_slip: 0.0,
        },
        articulation: ArticulationState::default(),
        actuators: ActuatorState {
            steering_angle: 0.0,
            brake_pressure: 0.0,
            throttle: 0.0,
            brake_commanded: 0.0,
            brake_applied: 0.0,
        },
        tires: TireState::default(),
        road,
    }
}

fn base_car_state(x: f64, y: f64, heading: f64, speed: f64, road: RoadSurface) -> VehicleState {
    VehicleState {
        pose: Pose { x, y, heading },
        dynamics: Dynamics {
            velocity: speed,
            lateral_velocity: 0.0,
            acceleration: 0.0,
            yaw_rate: 0.0,
            slip_angle: 0.0,
            side_slip: 0.0,
        },
        articulation: ArticulationState::default(),
        actuators: ActuatorState {
            steering_angle: 0.0,
            brake_pressure: 0.0,
            throttle: 0.0,
            brake_commanded: 0.0,
            brake_applied: 0.0,
        },
        tires: TireState::default(),
        road,
    }
}

pub fn emergency_brake(speed: f64, road: RoadSurface) -> Scenario {
    Scenario {
        initial_state: base_car_state(0.0, 0.0, 0.0, speed, road),
        params: VehicleParams::passenger_car(),
        description: "Emergency braking from constant speed",
    }
}

pub fn lane_change(x: f64, y: f64, speed: f64, road: RoadSurface) -> Scenario {
    Scenario {
        initial_state: base_car_state(x, y, 0.0, speed, road),
        params: VehicleParams::passenger_car(),
        description: "Lane change maneuver at constant speed",
    }
}

pub fn aquaplaning_test(speed: f64) -> Scenario {
    Scenario {
        initial_state: base_car_state(0.0, 0.0, 0.0, speed, RoadSurface::Wet),
        params: VehicleParams::passenger_car(),
        description: "High-speed driving on wet surface to test aquaplaning",
    }
}

pub fn intersection_crossing() -> Scenario {
    Scenario {
        initial_state: base_car_state(0.0, 0.0, 0.0, 10.0, RoadSurface::Dry),
        params: VehicleParams::passenger_car(),
        description: "Approaching and crossing an intersection",
    }
}

pub fn truck_emergency_brake(speed: f64, road: RoadSurface) -> Scenario {
    Scenario {
        initial_state: base_truck_state(0.0, 0.0, 0.0, speed, road),
        params: VehicleParams::heavy_truck(),
        description: "Heavy truck emergency braking with air brake latency",
    }
}

pub fn articulated_truck_emergency_brake(speed: f64, road: RoadSurface) -> Scenario {
    Scenario {
        initial_state: base_truck_state(0.0, 0.0, 0.0, speed, road),
        params: VehicleParams::articulated_truck(),
        description:
            "Articulated truck emergency braking — tests air brake lag and trailer stability",
    }
}

pub fn articulated_lane_change(speed: f64, road: RoadSurface) -> Scenario {
    Scenario {
        initial_state: base_truck_state(0.0, 0.0, 0.0, speed, road),
        params: VehicleParams::articulated_truck(),
        description:
            "Articulated truck lane change — tests trailer off-tracking and jackknife risk",
    }
}

pub fn jackknife_recovery(speed: f64, initial_trailer_angle: f64, road: RoadSurface) -> Scenario {
    let mut state = base_truck_state(0.0, 0.0, 0.0, speed, road);
    state.articulation.trailer_angle = initial_trailer_angle;
    Scenario {
        initial_state: state,
        params: VehicleParams::articulated_truck(),
        description: "Jackknife recovery maneuver — tests trailer angle stability control",
    }
}
